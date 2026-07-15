use lyra_dev_server::{DevServer, DevServerEndpoint, ServerDiagnostic, SessionSnapshot};
use lyra_device::HostPolicy;
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

pub(crate) struct DeviceBridgeController {
    running: Mutex<Option<RunningBridge>>,
}

struct RunningBridge {
    server: DevServer,
    endpoint: DevServerEndpoint,
}

impl DeviceBridgeController {
    pub(crate) fn new() -> Self {
        Self {
            running: Mutex::new(None),
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
    use super::{DeviceBridgeController, DeviceBridgeState};
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
