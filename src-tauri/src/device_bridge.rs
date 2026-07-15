use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use lyra_adb::SystemAdb;
use lyra_dev_server::{DevServer, DevServerEndpoint, ServerDiagnostic, SessionSnapshot};
use lyra_device::{AdbClient, AdbDevice, AdbDeviceState, DeviceDiagnostic, HostPolicy};
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

pub(crate) struct DeviceBridgeController {
    running: Mutex<Option<RunningBridge>>,
    adb_preflight: Mutex<AdbPreflightState>,
    adb_probe: Arc<dyn AdbDeviceProbe>,
}

struct RunningBridge {
    server: DevServer,
    endpoint: DevServerEndpoint,
}

struct AdbPreflightState {
    executable: Option<PathBuf>,
    generation: u64,
    status: AdbPreflightStatus,
}

trait AdbDeviceProbe: Send + Sync {
    fn list_devices(&self, executable: &Path) -> Result<Vec<AdbDevice>, DeviceDiagnostic>;
}

struct SystemAdbDeviceProbe;

impl AdbDeviceProbe for SystemAdbDeviceProbe {
    fn list_devices(&self, executable: &Path) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
        let mut adb = SystemAdb::from_path(executable);
        adb.list_devices()
    }
}

impl DeviceBridgeController {
    pub(crate) fn new() -> Self {
        Self::from_probe(Arc::new(SystemAdbDeviceProbe))
    }

    fn from_probe(adb_probe: Arc<dyn AdbDeviceProbe>) -> Self {
        Self {
            running: Mutex::new(None),
            adb_preflight: Mutex::new(AdbPreflightState {
                executable: None,
                generation: 0,
                status: AdbPreflightStatus {
                    configured: false,
                    readiness: AdbPreflightReadiness::Unconfigured,
                },
            }),
            adb_probe,
        }
    }

    #[cfg(test)]
    fn with_probe<P>(adb_probe: Arc<P>) -> Self
    where
        P: AdbDeviceProbe + 'static,
    {
        Self::from_probe(adb_probe)
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
        let mut preflight = self.adb_preflight.lock().await;
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
        let probe = Arc::clone(&self.adb_probe);
        let result = tokio::task::spawn_blocking(move || probe.list_devices(&executable)).await;
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
        let running = self.running.lock().await.take();
        if let Some(running) = running {
            running.server.shutdown().await?;
        }
        Ok(stopped_status())
    }
}

fn invalid_executable() -> DeviceDiagnostic {
    DeviceDiagnostic::new(
        "device.adb.invalidExecutable",
        "ADB executable must resolve to a regular file",
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

    use lyra_device::{AdbDevice, AdbDeviceState, DeviceDiagnostic, DeviceSerial};

    use super::{
        AdbDeviceProbe, AdbPreflightReadiness, AdbPreflightStatus, DeviceBridgeController,
        DeviceBridgeState,
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
            let probe = Arc::new(FakeAdbProbe::default());
            let controller = DeviceBridgeController::with_probe(Arc::clone(&probe));

            let error = controller.check_adb().await.unwrap_err();

            assert_eq!(error.code, "device.adb.notConfigured");
            probe.assert_no_calls();
        });
    }

    #[test]
    fn explicit_check_maps_only_ready_device_counts() {
        tauri::async_runtime::block_on(async {
            let executable = tempfile::NamedTempFile::new().unwrap();
            let probe = Arc::new(FakeAdbProbe::from_results([
                Ok(vec![]),
                Ok(vec![device("AVATR-01", AdbDeviceState::Device)]),
                Ok(vec![
                    device("AVATR-01", AdbDeviceState::Device),
                    device("AVATR-02", AdbDeviceState::Device),
                    device("OFFLINE", AdbDeviceState::Offline),
                    device("UNAUTHORIZED", AdbDeviceState::Unauthorized),
                ]),
            ]));
            let controller = DeviceBridgeController::with_probe(Arc::clone(&probe));
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
            probe.assert_finished(3);
        });
    }

    #[test]
    fn failed_explicit_check_preserves_the_stable_diagnostic_and_safe_error_status() {
        tauri::async_runtime::block_on(async {
            let executable = tempfile::NamedTempFile::new().unwrap();
            let probe = Arc::new(FakeAdbProbe::from_results([Err(DeviceDiagnostic::new(
                "device.adb.commandFailed",
                "adb devices failed",
            ))]));
            let controller = DeviceBridgeController::with_probe(Arc::clone(&probe));
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
            probe.assert_finished(1);
        });
    }

    #[test]
    fn a_probe_worker_failure_maps_to_a_stable_error_status() {
        tauri::async_runtime::block_on(async {
            let executable = tempfile::NamedTempFile::new().unwrap();
            let controller = DeviceBridgeController::with_probe(Arc::new(PanicProbe));
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
        let controller = Arc::new(DeviceBridgeController::with_probe(Arc::new(
            BlockingProbe {
                started_sender,
                release_receiver: StdMutex::new(release_receiver),
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

    #[derive(Default)]
    struct FakeAdbProbe {
        results: StdMutex<VecDeque<Result<Vec<AdbDevice>, DeviceDiagnostic>>>,
        calls: StdMutex<Vec<PathBuf>>,
    }

    impl FakeAdbProbe {
        fn from_results(
            results: impl IntoIterator<Item = Result<Vec<AdbDevice>, DeviceDiagnostic>>,
        ) -> Self {
            Self {
                results: StdMutex::new(results.into_iter().collect()),
                calls: StdMutex::new(Vec::new()),
            }
        }

        fn assert_no_calls(&self) {
            assert!(self.calls.lock().unwrap().is_empty());
        }

        fn assert_finished(&self, expected_calls: usize) {
            assert!(self.results.lock().unwrap().is_empty());
            assert_eq!(self.calls.lock().unwrap().len(), expected_calls);
        }
    }

    impl AdbDeviceProbe for FakeAdbProbe {
        fn list_devices(&self, executable: &Path) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
            self.calls.lock().unwrap().push(executable.into());
            self.results
                .lock()
                .unwrap()
                .pop_front()
                .expect("unexpected ADB probe")
        }
    }

    struct PanicProbe;

    impl AdbDeviceProbe for PanicProbe {
        fn list_devices(&self, _: &Path) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
            panic!("simulated ADB probe worker failure");
        }
    }

    struct BlockingProbe {
        started_sender: mpsc::SyncSender<()>,
        release_receiver: StdMutex<mpsc::Receiver<()>>,
    }

    impl AdbDeviceProbe for BlockingProbe {
        fn list_devices(&self, _: &Path) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
            self.started_sender.send(()).unwrap();
            self.release_receiver.lock().unwrap().recv().unwrap();
            Ok(vec![device("AVATR-01", AdbDeviceState::Device)])
        }
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
