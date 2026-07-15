use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex as StdMutex},
};

use lyra_adb::SystemAdb;
use lyra_dev_server::{DevServer, DevServerEndpoint, ServerDiagnostic, SessionSnapshot};
use lyra_device::{
    AdbClient, AdbDevice, AdbDeviceState, DevBridgeReverseCoordinator, DevBridgeReverseRequest,
    DeviceDiagnostic, HostPolicy, LocalPort, ReverseMapping,
};
use serde::Serialize;
use tokio::sync::Mutex;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum DeviceBridgeState {
    Stopped,
    Waiting,
    Connected,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeviceBridgeSession {
    device_profile_id: String,
    protocol_version: String,
    capabilities: Vec<String>,
}

impl From<SessionSnapshot> for DeviceBridgeSession {
    fn from(snapshot: SessionSnapshot) -> Self {
        Self {
            device_profile_id: snapshot.device_profile_id,
            protocol_version: snapshot.protocol_version,
            capabilities: snapshot.capabilities,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeviceBridgeStatus {
    state: DeviceBridgeState,
    session: Option<DeviceBridgeSession>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum AdbPreflightReadiness {
    Unconfigured,
    NotChecked,
    NoReadyDevice,
    OneReadyDevice,
    MultipleReadyDevices,
    Error,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AdbPreflightStatus {
    configured: bool,
    readiness: AdbPreflightReadiness,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum DevBridgeMappingReadiness {
    Inactive,
    Enabling,
    Active,
    Removing,
    CleanupFailed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DevBridgeMappingStatus {
    readiness: DevBridgeMappingReadiness,
}

pub(crate) struct DeviceBridgeController {
    running: Mutex<Option<RunningBridge>>,
    adb_preflight: Mutex<AdbPreflightState>,
    adb_client_factory: Arc<dyn AdbClientFactory>,
    mapping_operation: Mutex<()>,
}

struct RunningBridge {
    server: DevServer,
    endpoint: DevServerEndpoint,
}

struct AdbPreflightState {
    executable: Option<PathBuf>,
    generation: u64,
    status: AdbPreflightStatus,
    mapping: MappingState,
}

struct ActiveMapping {
    mapping: ReverseMapping,
    adb: Box<dyn AdbClient + Send>,
}

type SharedActiveMapping = Arc<StdMutex<ActiveMapping>>;

enum MappingState {
    Inactive,
    Enabling,
    Active(SharedActiveMapping),
    Removing(SharedActiveMapping),
    CleanupFailed(SharedActiveMapping),
}

trait AdbClientFactory: Send + Sync {
    fn create(&self, executable: &Path) -> Box<dyn AdbClient + Send>;
}

struct SystemAdbClientFactory;

impl AdbClientFactory for SystemAdbClientFactory {
    fn create(&self, executable: &Path) -> Box<dyn AdbClient + Send> {
        Box::new(SystemAdb::from_path(executable))
    }
}

impl DeviceBridgeController {
    pub(crate) fn new() -> Self {
        Self::from_factory(Arc::new(SystemAdbClientFactory))
    }

    fn from_factory(adb_client_factory: Arc<dyn AdbClientFactory>) -> Self {
        Self {
            running: Mutex::new(None),
            adb_preflight: Mutex::new(AdbPreflightState {
                executable: None,
                generation: 0,
                status: AdbPreflightStatus {
                    configured: false,
                    readiness: AdbPreflightReadiness::Unconfigured,
                },
                mapping: MappingState::Inactive,
            }),
            adb_client_factory,
            mapping_operation: Mutex::new(()),
        }
    }

    #[cfg(test)]
    fn with_factory<F>(adb_client_factory: Arc<F>) -> Self
    where
        F: AdbClientFactory + 'static,
    {
        Self::from_factory(adb_client_factory)
    }

    pub(crate) async fn adb_status(&self) -> AdbPreflightStatus {
        self.adb_preflight.lock().await.status.clone()
    }

    pub(crate) async fn configure_adb_executable(
        &self,
        executable: PathBuf,
    ) -> Result<AdbPreflightStatus, DeviceDiagnostic> {
        let executable = executable
            .canonicalize()
            .map_err(|_| invalid_executable())?;
        if !executable.is_file() {
            return Err(invalid_executable());
        }

        let status = AdbPreflightStatus {
            configured: true,
            readiness: AdbPreflightReadiness::NotChecked,
        };
        let _operation = self.mapping_operation.lock().await;
        let mut preflight = self.adb_preflight.lock().await;
        if !matches!(preflight.mapping, MappingState::Inactive) {
            return Err(mapping_active());
        }
        preflight.executable = Some(executable);
        preflight.generation = preflight.generation.wrapping_add(1);
        preflight.status = status.clone();
        Ok(status)
    }

    pub(crate) async fn check_adb(&self) -> Result<AdbPreflightStatus, DeviceDiagnostic> {
        let (executable, generation) = {
            let preflight = self.adb_preflight.lock().await;
            (
                preflight.executable.clone().ok_or_else(not_configured)?,
                preflight.generation,
            )
        };
        let factory = Arc::clone(&self.adb_client_factory);
        let result = tokio::task::spawn_blocking(move || {
            let mut adb = factory.create(&executable);
            adb.list_devices()
        })
        .await;
        let mut preflight = self.adb_preflight.lock().await;
        if preflight.generation != generation {
            return Ok(preflight.status.clone());
        }
        match result {
            Ok(Ok(devices)) => {
                let status = readiness_from_devices(&devices);
                preflight.status = status.clone();
                Ok(status)
            }
            Ok(Err(error)) => {
                preflight.status = adb_error_status();
                Err(error)
            }
            Err(_) => {
                preflight.status = adb_error_status();
                Err(probe_failed())
            }
        }
    }

    pub(crate) async fn status(&self) -> DeviceBridgeStatus {
        let running = self.running.lock().await;
        match running.as_ref() {
            Some(running) => status_from_running(running).await,
            None => stopped_status(),
        }
    }

    pub(crate) async fn mapping_status(&self) -> DevBridgeMappingStatus {
        let preflight = self.adb_preflight.lock().await;
        mapping_status_from_state(&preflight.mapping)
    }

    pub(crate) async fn enable_mapping(&self) -> Result<DevBridgeMappingStatus, DeviceDiagnostic> {
        let _operation = self.mapping_operation.lock().await;
        let local_port = {
            let running = self.running.lock().await;
            let running = running.as_ref().ok_or_else(bridge_not_running)?;
            LocalPort::new(running.endpoint.address().port()).map_err(|_| bridge_not_running())?
        };
        let executable = {
            let mut preflight = self.adb_preflight.lock().await;
            match &preflight.mapping {
                MappingState::Inactive => {}
                MappingState::CleanupFailed(_) => return Err(mapping_active()),
                MappingState::Active(_) | MappingState::Enabling | MappingState::Removing(_) => {
                    return Ok(mapping_status_from_state(&preflight.mapping));
                }
            }
            let executable = preflight.executable.clone().ok_or_else(not_configured)?;
            preflight.mapping = MappingState::Enabling;
            executable
        };
        let factory = Arc::clone(&self.adb_client_factory);
        let result = tokio::task::spawn_blocking(move || {
            let mut adb = factory.create(&executable);
            let mapping = DevBridgeReverseCoordinator::establish(
                adb.as_mut(),
                DevBridgeReverseRequest::new(local_port),
            )?;
            Ok::<_, DeviceDiagnostic>(ActiveMapping { mapping, adb })
        })
        .await;
        let mut preflight = self.adb_preflight.lock().await;
        match result {
            Ok(Ok(active)) => {
                preflight.mapping = MappingState::Active(Arc::new(StdMutex::new(active)));
                Ok(mapping_status_from_state(&preflight.mapping))
            }
            Ok(Err(error)) => {
                preflight.mapping = MappingState::Inactive;
                Err(error)
            }
            Err(_) => {
                preflight.mapping = MappingState::Inactive;
                Err(probe_failed())
            }
        }
    }

    pub(crate) async fn disable_mapping(&self) -> Result<DevBridgeMappingStatus, DeviceDiagnostic> {
        let _operation = self.mapping_operation.lock().await;
        self.remove_mapping_with_operation().await
    }

    pub(crate) async fn start(&self) -> Result<DeviceBridgeStatus, ServerDiagnostic> {
        let mut running = self.running.lock().await;
        if running.is_none() {
            let (server, endpoint) = DevServer::start(host_policy()?).await?;
            *running = Some(RunningBridge { server, endpoint });
        }
        Ok(status_from_running(
            running
                .as_ref()
                .expect("running bridge must exist after start"),
        )
        .await)
    }

    pub(crate) async fn stop(&self) -> Result<DeviceBridgeStatus, ServerDiagnostic> {
        let _operation = self.mapping_operation.lock().await;
        self.remove_mapping_with_operation()
            .await
            .map_err(server_diagnostic_from_device)?;
        let running = self.running.lock().await.take();
        if let Some(running) = running {
            running.server.shutdown().await?;
        }
        Ok(stopped_status())
    }

    async fn remove_mapping_with_operation(
        &self,
    ) -> Result<DevBridgeMappingStatus, DeviceDiagnostic> {
        let active = {
            let mut preflight = self.adb_preflight.lock().await;
            let active = match &preflight.mapping {
                MappingState::Inactive => return Ok(mapping_status_from_state(&preflight.mapping)),
                MappingState::Enabling | MappingState::Removing(_) => {
                    return Ok(mapping_status_from_state(&preflight.mapping));
                }
                MappingState::Active(active) | MappingState::CleanupFailed(active) => {
                    Arc::clone(active)
                }
            };
            preflight.mapping = MappingState::Removing(Arc::clone(&active));
            active
        };
        let worker_mapping = Arc::clone(&active);
        let removal = tokio::task::spawn_blocking(move || {
            let mut active = worker_mapping
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner);
            let ActiveMapping { mapping, adb } = &mut *active;
            mapping.remove(adb.as_mut())
        })
        .await;
        let mut preflight = self.adb_preflight.lock().await;
        match removal {
            Ok(Ok(())) => {
                preflight.mapping = MappingState::Inactive;
                Ok(mapping_status_from_state(&preflight.mapping))
            }
            Ok(Err(error)) => {
                preflight.mapping = MappingState::CleanupFailed(active);
                Err(error)
            }
            Err(_) => {
                preflight.mapping = MappingState::CleanupFailed(active);
                Err(probe_failed())
            }
        }
    }
}

fn invalid_executable() -> DeviceDiagnostic {
    DeviceDiagnostic::new(
        "device.adb.invalidExecutable",
        "ADB executable must resolve to a regular file",
    )
}

fn bridge_not_running() -> DeviceDiagnostic {
    DeviceDiagnostic::new(
        "device.bridge.notRunning",
        "start the Dev Bridge before enabling its ADB mapping",
    )
}

fn mapping_active() -> DeviceDiagnostic {
    DeviceDiagnostic::new(
        "device.adb.mappingActive",
        "remove the active ADB mapping before selecting another executable",
    )
}

fn not_configured() -> DeviceDiagnostic {
    DeviceDiagnostic::new(
        "device.adb.notConfigured",
        "select an ADB executable before checking devices",
    )
}

fn probe_failed() -> DeviceDiagnostic {
    DeviceDiagnostic::new("device.adb.probeFailed", "ADB preflight worker failed")
}

fn server_diagnostic_from_device(error: DeviceDiagnostic) -> ServerDiagnostic {
    ServerDiagnostic::new(error.code, error.message)
}

fn mapping_status_from_state(mapping: &MappingState) -> DevBridgeMappingStatus {
    let readiness = match mapping {
        MappingState::Inactive => DevBridgeMappingReadiness::Inactive,
        MappingState::Enabling => DevBridgeMappingReadiness::Enabling,
        MappingState::Active(_) => DevBridgeMappingReadiness::Active,
        MappingState::Removing(active) => {
            debug_assert!(Arc::strong_count(active) > 0);
            DevBridgeMappingReadiness::Removing
        }
        MappingState::CleanupFailed(_) => DevBridgeMappingReadiness::CleanupFailed,
    };
    DevBridgeMappingStatus { readiness }
}

const fn adb_error_status() -> AdbPreflightStatus {
    AdbPreflightStatus {
        configured: true,
        readiness: AdbPreflightReadiness::Error,
    }
}

fn readiness_from_devices(devices: &[AdbDevice]) -> AdbPreflightStatus {
    let ready_count = devices
        .iter()
        .filter(|device| device.state == AdbDeviceState::Device)
        .count();
    let readiness = match ready_count {
        0 => AdbPreflightReadiness::NoReadyDevice,
        1 => AdbPreflightReadiness::OneReadyDevice,
        _ => AdbPreflightReadiness::MultipleReadyDevices,
    };
    AdbPreflightStatus {
        configured: true,
        readiness,
    }
}

fn host_policy() -> Result<HostPolicy, ServerDiagnostic> {
    HostPolicy::new("1.2.0", ["activate", "stageRevision"], ["stageRevision"])
        .map_err(|error| ServerDiagnostic::new(error.code, error.message))
}

async fn status_from_running(running: &RunningBridge) -> DeviceBridgeStatus {
    debug_assert!(running.endpoint.address().ip().is_loopback());
    let session = running
        .server
        .session_snapshot()
        .await
        .map(DeviceBridgeSession::from);
    DeviceBridgeStatus {
        state: if session.is_some() {
            DeviceBridgeState::Connected
        } else {
            DeviceBridgeState::Waiting
        },
        session,
    }
}

const fn stopped_status() -> DeviceBridgeStatus {
    DeviceBridgeStatus {
        state: DeviceBridgeState::Stopped,
        session: None,
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::VecDeque,
        path::{Path, PathBuf},
        sync::{Arc, Mutex as StdMutex, mpsc},
        time::Duration,
    };

    use lyra_device::{
        AdbClient, AdbDevice, AdbDeviceState, DEV_BRIDGE_REMOTE_PORT, DeviceDiagnostic, DevicePath,
        DeviceSerial, LocalPort, RemotePort,
    };

    use super::{
        AdbClientFactory, AdbPreflightReadiness, AdbPreflightStatus, DevBridgeMappingReadiness,
        DeviceBridgeController, DeviceBridgeState,
    };
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpStream;

    #[test]
    fn controller_starts_stopped() {
        tauri::async_runtime::block_on(async {
            let controller = DeviceBridgeController::new();

            let status = controller.status().await;

            assert_eq!(status.state, DeviceBridgeState::Stopped);
            assert!(status.session.is_none());
        });
    }

    #[test]
    fn controller_starts_with_an_unconfigured_adb_preflight() {
        tauri::async_runtime::block_on(async {
            let controller = DeviceBridgeController::new();

            assert_eq!(
                controller.adb_status().await,
                AdbPreflightStatus {
                    configured: false,
                    readiness: AdbPreflightReadiness::Unconfigured,
                }
            );
        });
    }

    #[test]
    fn configuring_a_regular_file_marks_adb_as_not_checked() {
        tauri::async_runtime::block_on(async {
            let controller = DeviceBridgeController::new();
            let executable = tempfile::NamedTempFile::new().unwrap();

            let status = controller
                .configure_adb_executable(executable.path().to_path_buf())
                .await
                .unwrap();

            assert_eq!(
                status,
                AdbPreflightStatus {
                    configured: true,
                    readiness: AdbPreflightReadiness::NotChecked,
                }
            );
            assert_eq!(controller.adb_status().await, status);
        });
    }

    #[test]
    fn configuring_a_missing_file_is_rejected_without_replacing_status() {
        tauri::async_runtime::block_on(async {
            let controller = DeviceBridgeController::new();
            let directory = tempfile::tempdir().unwrap();
            let missing = directory.path().join("adb");

            let error = controller
                .configure_adb_executable(missing)
                .await
                .unwrap_err();

            assert_eq!(error.code, "device.adb.invalidExecutable");
            assert_eq!(
                controller.adb_status().await,
                AdbPreflightStatus {
                    configured: false,
                    readiness: AdbPreflightReadiness::Unconfigured,
                }
            );
        });
    }

    #[test]
    fn checking_without_an_executable_returns_a_stable_error_without_probing() {
        tauri::async_runtime::block_on(async {
            let factory = Arc::new(FakeAdbClientFactory::default());
            let controller = DeviceBridgeController::with_factory(Arc::clone(&factory));

            let error = controller.check_adb().await.unwrap_err();

            assert_eq!(error.code, "device.adb.notConfigured");
            factory.assert_no_clients_created();
        });
    }

    #[test]
    fn explicit_check_maps_only_ready_device_counts() {
        tauri::async_runtime::block_on(async {
            let executable = tempfile::NamedTempFile::new().unwrap();
            let factory = Arc::new(FakeAdbClientFactory::from_scripts([
                vec![FakeAdbCall::List(Ok(vec![]))],
                vec![FakeAdbCall::List(Ok(vec![device(
                    "AVATR-01",
                    AdbDeviceState::Device,
                )]))],
                vec![FakeAdbCall::List(Ok(vec![
                    device("AVATR-01", AdbDeviceState::Device),
                    device("AVATR-02", AdbDeviceState::Device),
                    device("OFFLINE", AdbDeviceState::Offline),
                    device("UNAUTHORIZED", AdbDeviceState::Unauthorized),
                ]))],
            ]));
            let controller = DeviceBridgeController::with_factory(Arc::clone(&factory));
            controller
                .configure_adb_executable(executable.path().to_path_buf())
                .await
                .unwrap();

            assert_eq!(
                controller.check_adb().await.unwrap().readiness,
                AdbPreflightReadiness::NoReadyDevice
            );
            assert_eq!(
                controller.check_adb().await.unwrap().readiness,
                AdbPreflightReadiness::OneReadyDevice
            );
            assert_eq!(
                controller.check_adb().await.unwrap().readiness,
                AdbPreflightReadiness::MultipleReadyDevices
            );
            factory.assert_finished(3);
        });
    }

    #[test]
    fn failed_explicit_check_preserves_the_stable_diagnostic_and_safe_error_status() {
        tauri::async_runtime::block_on(async {
            let executable = tempfile::NamedTempFile::new().unwrap();
            let factory = Arc::new(FakeAdbClientFactory::from_scripts([vec![
                FakeAdbCall::List(Err(DeviceDiagnostic::new(
                    "device.adb.commandFailed",
                    "adb devices failed",
                ))),
            ]]));
            let controller = DeviceBridgeController::with_factory(Arc::clone(&factory));
            controller
                .configure_adb_executable(executable.path().to_path_buf())
                .await
                .unwrap();

            let error = controller.check_adb().await.unwrap_err();

            assert_eq!(error.code, "device.adb.commandFailed");
            assert_eq!(
                controller.adb_status().await,
                AdbPreflightStatus {
                    configured: true,
                    readiness: AdbPreflightReadiness::Error,
                }
            );
            assert_eq!(
                serde_json::to_value(controller.adb_status().await).unwrap(),
                serde_json::json!({ "configured": true, "readiness": "error" })
            );
            factory.assert_finished(1);
        });
    }

    #[test]
    fn a_probe_worker_failure_maps_to_a_stable_error_status() {
        tauri::async_runtime::block_on(async {
            let executable = tempfile::NamedTempFile::new().unwrap();
            let controller = DeviceBridgeController::with_factory(Arc::new(
                FakeAdbClientFactory::from_scripts([vec![FakeAdbCall::PanicList]]),
            ));
            controller
                .configure_adb_executable(executable.path().to_path_buf())
                .await
                .unwrap();

            let error = controller.check_adb().await.unwrap_err();

            assert_eq!(error.code, "device.adb.probeFailed");
            assert_eq!(
                controller.adb_status().await,
                AdbPreflightStatus {
                    configured: true,
                    readiness: AdbPreflightReadiness::Error,
                }
            );
        });
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn a_stale_probe_result_cannot_replace_newer_adb_configuration() {
        let first = tempfile::NamedTempFile::new().unwrap();
        let second = tempfile::NamedTempFile::new().unwrap();
        let (started_sender, started_receiver) = mpsc::sync_channel(1);
        let (release_sender, release_receiver) = mpsc::sync_channel(1);
        let controller = Arc::new(DeviceBridgeController::with_factory(Arc::new(
            BlockingAdbClientFactory {
                started_sender,
                release_receiver: Arc::new(StdMutex::new(release_receiver)),
            },
        )));
        controller
            .configure_adb_executable(first.path().to_path_buf())
            .await
            .unwrap();

        let checking_controller = Arc::clone(&controller);
        let checking = tokio::spawn(async move { checking_controller.check_adb().await });
        started_receiver
            .recv_timeout(Duration::from_secs(1))
            .unwrap();
        controller
            .configure_adb_executable(second.path().to_path_buf())
            .await
            .unwrap();
        release_sender.send(()).unwrap();

        assert_eq!(
            checking.await.unwrap().unwrap().readiness,
            AdbPreflightReadiness::NotChecked
        );
        assert_eq!(
            controller.adb_status().await.readiness,
            AdbPreflightReadiness::NotChecked
        );
    }

    #[tokio::test]
    async fn mapping_requires_a_running_bridge_without_creating_an_adb_client() {
        let factory = Arc::new(FakeAdbClientFactory::default());
        let controller = DeviceBridgeController::with_factory(Arc::clone(&factory));

        let error = controller.enable_mapping().await.unwrap_err();

        assert_eq!(error.code, "device.bridge.notRunning");
        assert_eq!(
            controller.mapping_status().await.readiness,
            DevBridgeMappingReadiness::Inactive
        );
        factory.assert_no_clients_created();
    }

    #[tokio::test]
    async fn mapping_requires_a_configured_adb_without_creating_an_adb_client() {
        let factory = Arc::new(FakeAdbClientFactory::default());
        let controller = DeviceBridgeController::with_factory(Arc::clone(&factory));
        controller.start().await.unwrap();

        let error = controller.enable_mapping().await.unwrap_err();

        assert_eq!(error.code, "device.adb.notConfigured");
        factory.assert_no_clients_created();
        controller.stop().await.unwrap();
    }

    #[tokio::test]
    async fn mapping_uses_the_private_listener_port_and_serializes_only_active_state() {
        let executable = tempfile::NamedTempFile::new().unwrap();
        let factory = Arc::new(FakeAdbClientFactory::from_scripts([vec![
            FakeAdbCall::List(Ok(vec![device("AVATR-01", AdbDeviceState::Device)])),
            FakeAdbCall::Reverse(Ok(())),
            FakeAdbCall::Remove(Ok(())),
        ]]));
        let controller = DeviceBridgeController::with_factory(Arc::clone(&factory));
        controller
            .configure_adb_executable(executable.path().to_path_buf())
            .await
            .unwrap();
        controller.start().await.unwrap();
        let listener_port = {
            let running = controller.running.lock().await;
            running.as_ref().unwrap().endpoint.address().port()
        };

        assert_eq!(
            controller.enable_mapping().await.unwrap().readiness,
            DevBridgeMappingReadiness::Active
        );
        factory.assert_established_once_with(executable.path(), listener_port);
        assert_eq!(
            serde_json::to_value(controller.mapping_status().await).unwrap(),
            serde_json::json!({ "readiness": "active" })
        );

        controller.stop().await.unwrap();
        factory.assert_finished(1);
    }

    #[tokio::test]
    async fn mapping_is_idempotent_after_an_active_mapping() {
        let executable = tempfile::NamedTempFile::new().unwrap();
        let factory = Arc::new(FakeAdbClientFactory::from_scripts([vec![
            FakeAdbCall::List(Ok(vec![device("AVATR-01", AdbDeviceState::Device)])),
            FakeAdbCall::Reverse(Ok(())),
            FakeAdbCall::Remove(Ok(())),
        ]]));
        let controller = ready_mapping_controller(executable.path(), Arc::clone(&factory)).await;

        assert_eq!(
            controller.enable_mapping().await.unwrap().readiness,
            DevBridgeMappingReadiness::Active
        );
        factory.assert_established_once_with(executable.path(), {
            let running = controller.running.lock().await;
            running.as_ref().unwrap().endpoint.address().port()
        });

        controller.stop().await.unwrap();
        factory.assert_finished(1);
    }

    #[tokio::test]
    async fn mapping_preserves_zero_multiple_and_adapter_diagnostics_without_becoming_active() {
        let executable = tempfile::NamedTempFile::new().unwrap();
        let factory = Arc::new(FakeAdbClientFactory::from_scripts([
            vec![FakeAdbCall::List(Ok(vec![]))],
            vec![FakeAdbCall::List(Ok(vec![
                device("AVATR-01", AdbDeviceState::Device),
                device("AVATR-02", AdbDeviceState::Device),
            ]))],
            vec![FakeAdbCall::List(Err(DeviceDiagnostic::new(
                "device.adb.malformedResponse",
                "malformed devices response",
            )))],
        ]));
        let controller = DeviceBridgeController::with_factory(Arc::clone(&factory));
        controller
            .configure_adb_executable(executable.path().to_path_buf())
            .await
            .unwrap();
        controller.start().await.unwrap();

        for expected_code in [
            "device.adb.noEligibleDevice",
            "device.adb.multipleEligibleDevices",
            "device.adb.malformedResponse",
        ] {
            let error = controller.enable_mapping().await.unwrap_err();
            assert_eq!(error.code, expected_code);
            assert_eq!(
                controller.mapping_status().await.readiness,
                DevBridgeMappingReadiness::Inactive
            );
        }

        controller.stop().await.unwrap();
        factory.assert_finished(3);
    }

    #[tokio::test]
    async fn mapping_blocks_replacing_its_adb_executable_until_removed() {
        let executable = tempfile::NamedTempFile::new().unwrap();
        let replacement = tempfile::NamedTempFile::new().unwrap();
        let factory = Arc::new(FakeAdbClientFactory::from_scripts([vec![
            FakeAdbCall::List(Ok(vec![device("AVATR-01", AdbDeviceState::Device)])),
            FakeAdbCall::Reverse(Ok(())),
            FakeAdbCall::Remove(Ok(())),
        ]]));
        let controller = ready_mapping_controller(executable.path(), Arc::clone(&factory)).await;

        let error = controller
            .configure_adb_executable(replacement.path().to_path_buf())
            .await
            .unwrap_err();

        assert_eq!(error.code, "device.adb.mappingActive");
        assert_eq!(
            controller.disable_mapping().await.unwrap().readiness,
            DevBridgeMappingReadiness::Inactive
        );
        controller.stop().await.unwrap();
        factory.assert_finished(1);
    }

    #[tokio::test]
    async fn mapping_disable_is_idempotent_while_inactive_without_creating_an_adb_client() {
        let factory = Arc::new(FakeAdbClientFactory::default());
        let controller = DeviceBridgeController::with_factory(Arc::clone(&factory));

        assert_eq!(
            controller.disable_mapping().await.unwrap().readiness,
            DevBridgeMappingReadiness::Inactive
        );
        factory.assert_no_clients_created();
    }

    #[tokio::test]
    async fn mapping_stop_keeps_the_bridge_running_after_cleanup_failure_until_an_explicit_retry() {
        let executable = tempfile::NamedTempFile::new().unwrap();
        let factory = Arc::new(FakeAdbClientFactory::from_scripts([vec![
            FakeAdbCall::List(Ok(vec![device("AVATR-01", AdbDeviceState::Device)])),
            FakeAdbCall::Reverse(Ok(())),
            FakeAdbCall::Remove(Err(DeviceDiagnostic::new(
                "device.adb.commandFailed",
                "remove failed",
            ))),
            FakeAdbCall::Remove(Ok(())),
        ]]));
        let controller = ready_mapping_controller(executable.path(), Arc::clone(&factory)).await;

        let error = controller.stop().await.unwrap_err();
        assert_eq!(error.code, "device.adb.commandFailed");
        assert_eq!(
            controller.mapping_status().await.readiness,
            DevBridgeMappingReadiness::CleanupFailed
        );
        assert_eq!(controller.status().await.state, DeviceBridgeState::Waiting);

        assert_eq!(
            controller.disable_mapping().await.unwrap().readiness,
            DevBridgeMappingReadiness::Inactive
        );
        assert_eq!(
            controller.stop().await.unwrap().state,
            DeviceBridgeState::Stopped
        );
        factory.assert_finished(1);
    }

    #[tokio::test]
    async fn mapping_worker_panics_leave_establish_inactive_and_cleanup_retryable() {
        let establish_executable = tempfile::NamedTempFile::new().unwrap();
        let establish_factory = Arc::new(FakeAdbClientFactory::from_scripts([vec![
            FakeAdbCall::List(Ok(vec![device("AVATR-01", AdbDeviceState::Device)])),
            FakeAdbCall::PanicReverse,
        ]]));
        let establish = DeviceBridgeController::with_factory(Arc::clone(&establish_factory));
        establish
            .configure_adb_executable(establish_executable.path().to_path_buf())
            .await
            .unwrap();
        establish.start().await.unwrap();

        let establish_error = establish.enable_mapping().await.unwrap_err();
        assert_eq!(establish_error.code, "device.adb.probeFailed");
        assert_eq!(
            establish.mapping_status().await.readiness,
            DevBridgeMappingReadiness::Inactive
        );
        establish.stop().await.unwrap();

        let remove_executable = tempfile::NamedTempFile::new().unwrap();
        let remove_factory = Arc::new(FakeAdbClientFactory::from_scripts([vec![
            FakeAdbCall::List(Ok(vec![device("AVATR-01", AdbDeviceState::Device)])),
            FakeAdbCall::Reverse(Ok(())),
            FakeAdbCall::PanicRemove,
            FakeAdbCall::Remove(Ok(())),
        ]]));
        let remove =
            ready_mapping_controller(remove_executable.path(), Arc::clone(&remove_factory)).await;

        let remove_error = remove.disable_mapping().await.unwrap_err();
        assert_eq!(remove_error.code, "device.adb.probeFailed");
        assert_eq!(
            remove.mapping_status().await.readiness,
            DevBridgeMappingReadiness::CleanupFailed
        );
        assert_eq!(
            remove.disable_mapping().await.unwrap().readiness,
            DevBridgeMappingReadiness::Inactive
        );
        remove.stop().await.unwrap();
        remove_factory.assert_finished(1);
    }

    enum FakeAdbCall {
        List(Result<Vec<AdbDevice>, DeviceDiagnostic>),
        Reverse(Result<(), DeviceDiagnostic>),
        Remove(Result<(), DeviceDiagnostic>),
        PanicList,
        PanicReverse,
        PanicRemove,
    }

    #[derive(Default)]
    struct FakeAdbClientFactory {
        scripts: StdMutex<VecDeque<VecDeque<FakeAdbCall>>>,
        created_paths: StdMutex<Vec<PathBuf>>,
        reverse_ports: Arc<StdMutex<Vec<u16>>>,
    }

    struct FakeAdbClient {
        calls: VecDeque<FakeAdbCall>,
        reverse_ports: Arc<StdMutex<Vec<u16>>>,
    }

    impl FakeAdbClientFactory {
        fn from_scripts<I, S>(scripts: I) -> Self
        where
            I: IntoIterator<Item = S>,
            S: IntoIterator<Item = FakeAdbCall>,
        {
            Self {
                scripts: StdMutex::new(
                    scripts
                        .into_iter()
                        .map(|script| script.into_iter().collect())
                        .collect(),
                ),
                created_paths: StdMutex::new(Vec::new()),
                reverse_ports: Arc::new(StdMutex::new(Vec::new())),
            }
        }

        fn assert_no_clients_created(&self) {
            assert!(self.created_paths.lock().unwrap().is_empty());
        }

        fn assert_established_once_with(&self, executable: &Path, local_port: u16) {
            assert_eq!(
                self.created_paths.lock().unwrap().as_slice(),
                [executable.canonicalize().unwrap()]
            );
            assert_eq!(self.reverse_ports.lock().unwrap().as_slice(), [local_port]);
        }

        fn assert_finished(&self, expected_clients: usize) {
            assert!(self.scripts.lock().unwrap().is_empty());
            assert_eq!(self.created_paths.lock().unwrap().len(), expected_clients);
        }
    }

    impl AdbClientFactory for FakeAdbClientFactory {
        fn create(&self, executable: &Path) -> Box<dyn AdbClient + Send> {
            self.created_paths.lock().unwrap().push(executable.into());
            Box::new(FakeAdbClient {
                calls: self
                    .scripts
                    .lock()
                    .unwrap()
                    .pop_front()
                    .expect("unexpected ADB client"),
                reverse_ports: Arc::clone(&self.reverse_ports),
            })
        }
    }

    impl FakeAdbClient {
        fn next(&mut self, operation: &str) -> FakeAdbCall {
            self.calls
                .pop_front()
                .unwrap_or_else(|| panic!("unexpected ADB {operation} call"))
        }
    }

    impl AdbClient for FakeAdbClient {
        fn list_devices(&mut self) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
            match self.next("list_devices") {
                FakeAdbCall::List(result) => result,
                FakeAdbCall::PanicList => panic!("simulated ADB list worker failure"),
                _ => panic!("expected ADB list_devices call"),
            }
        }

        fn reverse(
            &mut self,
            serial: &DeviceSerial,
            local: LocalPort,
            remote: RemotePort,
        ) -> Result<(), DeviceDiagnostic> {
            assert_eq!(serial.to_string(), "AVATR-01");
            assert_eq!(remote, DEV_BRIDGE_REMOTE_PORT);
            self.reverse_ports.lock().unwrap().push(local.get());
            match self.next("reverse") {
                FakeAdbCall::Reverse(result) => result,
                FakeAdbCall::PanicReverse => panic!("simulated ADB reverse worker failure"),
                _ => panic!("expected ADB reverse call"),
            }
        }

        fn remove_reverse(
            &mut self,
            serial: &DeviceSerial,
            remote: RemotePort,
        ) -> Result<(), DeviceDiagnostic> {
            assert_eq!(serial.to_string(), "AVATR-01");
            assert_eq!(remote, DEV_BRIDGE_REMOTE_PORT);
            match self.next("remove_reverse") {
                FakeAdbCall::Remove(result) => result,
                FakeAdbCall::PanicRemove => panic!("simulated ADB remove worker failure"),
                _ => panic!("expected ADB remove_reverse call"),
            }
        }

        fn push(
            &mut self,
            _: &DeviceSerial,
            _: &Path,
            _: &DevicePath,
        ) -> Result<(), DeviceDiagnostic> {
            panic!("ADB push is forbidden in device bridge tests")
        }
    }

    struct BlockingAdbClientFactory {
        started_sender: mpsc::SyncSender<()>,
        release_receiver: Arc<StdMutex<mpsc::Receiver<()>>>,
    }

    impl AdbClientFactory for BlockingAdbClientFactory {
        fn create(&self, _: &Path) -> Box<dyn AdbClient + Send> {
            Box::new(BlockingAdbClient {
                started_sender: self.started_sender.clone(),
                release_receiver: Arc::clone(&self.release_receiver),
            })
        }
    }

    struct BlockingAdbClient {
        started_sender: mpsc::SyncSender<()>,
        release_receiver: Arc<StdMutex<mpsc::Receiver<()>>>,
    }

    impl AdbClient for BlockingAdbClient {
        fn list_devices(&mut self) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
            self.started_sender.send(()).unwrap();
            self.release_receiver.lock().unwrap().recv().unwrap();
            Ok(vec![device("AVATR-01", AdbDeviceState::Device)])
        }

        fn reverse(
            &mut self,
            _: &DeviceSerial,
            _: LocalPort,
            _: RemotePort,
        ) -> Result<(), DeviceDiagnostic> {
            panic!("unexpected ADB reverse from preflight")
        }

        fn remove_reverse(
            &mut self,
            _: &DeviceSerial,
            _: RemotePort,
        ) -> Result<(), DeviceDiagnostic> {
            panic!("unexpected ADB remove from preflight")
        }

        fn push(
            &mut self,
            _: &DeviceSerial,
            _: &Path,
            _: &DevicePath,
        ) -> Result<(), DeviceDiagnostic> {
            panic!("unexpected ADB push from preflight")
        }
    }

    async fn ready_mapping_controller(
        executable: &Path,
        factory: Arc<FakeAdbClientFactory>,
    ) -> DeviceBridgeController {
        let controller = DeviceBridgeController::with_factory(factory);
        controller
            .configure_adb_executable(executable.to_path_buf())
            .await
            .unwrap();
        controller.start().await.unwrap();
        controller.enable_mapping().await.unwrap();
        controller
    }

    fn device(serial: &str, state: AdbDeviceState) -> AdbDevice {
        AdbDevice {
            serial: DeviceSerial::new(serial).unwrap(),
            state,
        }
    }

    #[test]
    fn controller_start_is_idempotent_while_waiting_for_hello() {
        tauri::async_runtime::block_on(async {
            let controller = DeviceBridgeController::new();

            let first = controller.start().await.unwrap();
            let second = controller.start().await.unwrap();

            assert_eq!(first.state, DeviceBridgeState::Waiting);
            assert_eq!(first, second);
            assert!(first.session.is_none());
        });
    }

    #[tokio::test]
    async fn controller_reports_a_connected_session_after_authenticated_hello() {
        let controller = DeviceBridgeController::new();
        controller.start().await.unwrap();

        send_fixture_hello(&controller).await;

        let status = controller.status().await;
        assert_eq!(status.state, DeviceBridgeState::Connected);
        assert_eq!(
            status.session.as_ref().unwrap().device_profile_id,
            "com.avatr.cluster.4032x284"
        );
        assert!(
            serde_json::to_value(&status).unwrap()["session"]
                .get("sessionId")
                .is_none()
        );

        controller.stop().await.unwrap();
        assert_eq!(controller.status().await.state, DeviceBridgeState::Stopped);
    }

    async fn send_fixture_hello(controller: &DeviceBridgeController) {
        let (address, authorization) = {
            let running = controller.running.lock().await;
            let running = running.as_ref().expect("running bridge");
            (
                running.endpoint.address(),
                running.endpoint.authorization_value(),
            )
        };
        let body = include_bytes!("../../Fixtures/Device/hello-valid.json");
        let mut stream = TcpStream::connect(address).await.unwrap();
        let request = format!(
            "POST /v1/hello HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nAuthorization: {authorization}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n",
            body.len()
        );
        stream.write_all(request.as_bytes()).await.unwrap();
        stream.write_all(body).await.unwrap();
        stream.flush().await.unwrap();

        let mut response = Vec::new();
        stream.read_to_end(&mut response).await.unwrap();
        assert!(String::from_utf8_lossy(&response).starts_with("HTTP/1.1 200"));
    }
}
