use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use lyra_device::{DeviceDiagnostic, DeviceHello, HostPolicy, NegotiatedSession, negotiate};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::{RwLock, oneshot};
use tokio::task::JoinHandle;

use crate::token::{BridgeToken, session_id};
use crate::{DevServerEndpoint, ServerDiagnostic};

const MAX_HELLO_BYTES: usize = 16 * 1024;

/// An authenticated, single-listener loopback Dev Bridge server.
pub struct DevServer {
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), std::io::Error>>,
    state: Arc<ServerState>,
}

impl DevServer {
    /// Starts an authenticated server bound to an ephemeral IPv4 loopback port.
    ///
    /// # Errors
    ///
    /// Returns a `device.bridge.*` diagnostic when random token creation, listener binding or
    /// listener startup fails.
    pub async fn start(policy: HostPolicy) -> Result<(Self, DevServerEndpoint), ServerDiagnostic> {
        let token = BridgeToken::generate()?;
        let listener = TcpListener::bind(SocketAddr::from(([127, 0, 0, 1], 0)))
            .await
            .map_err(|error| {
                ServerDiagnostic::new("device.bridge.listenFailed", error.to_string())
            })?;
        let address = listener.local_addr().map_err(|error| {
            ServerDiagnostic::new("device.bridge.listenFailed", error.to_string())
        })?;
        let state = Arc::new(ServerState {
            policy,
            token: token.clone(),
            session: RwLock::new(None),
        });
        let router = Router::new()
            .route("/v1/hello", post(hello))
            .with_state(Arc::clone(&state));
        let (shutdown, shutdown_signal) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_signal.await;
                })
                .await
        });

        Ok((
            Self {
                shutdown,
                task,
                state,
            },
            DevServerEndpoint::new(address, token),
        ))
    }

    /// Stops the listener and waits for its server task to exit.
    ///
    /// # Errors
    ///
    /// Returns a `device.bridge.*` diagnostic when the server task terminates unexpectedly.
    pub async fn shutdown(self) -> Result<(), ServerDiagnostic> {
        let _ = self.shutdown.send(());
        self.task
            .await
            .map_err(|error| {
                ServerDiagnostic::new("device.bridge.serverTaskFailed", error.to_string())
            })?
            .map_err(|error| {
                ServerDiagnostic::new("device.bridge.serverStopped", error.to_string())
            })
    }

    /// Returns the current non-secret device session, if one has been authenticated.
    #[must_use]
    pub async fn session_snapshot(&self) -> Option<SessionSnapshot> {
        self.state.session.read().await.clone()
    }
}

struct ServerState {
    policy: HostPolicy,
    token: BridgeToken,
    session: RwLock<Option<SessionSnapshot>>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
/// Non-secret state for the one authenticated device session.
pub struct SessionSnapshot {
    pub session_id: String,
    pub device_profile_id: String,
    pub protocol_version: String,
    pub capabilities: Vec<String>,
}

impl SessionSnapshot {
    fn create(
        hello: &DeviceHello,
        negotiated: &NegotiatedSession,
    ) -> Result<Self, ServerDiagnostic> {
        Ok(Self {
            session_id: session_id()?,
            device_profile_id: hello.device_profile_id.clone(),
            protocol_version: negotiated.protocol_version.to_string(),
            capabilities: negotiated
                .capabilities
                .iter()
                .map(ToString::to_string)
                .collect(),
        })
    }
}

async fn hello(
    State(state): State<Arc<ServerState>>,
    request: Request,
) -> Result<Json<SessionSnapshot>, ApiError> {
    let headers = request.headers().clone();
    let body = axum::body::to_bytes(request.into_body(), MAX_HELLO_BYTES)
        .await
        .map_err(|error| {
            ApiError::new(
                StatusCode::PAYLOAD_TOO_LARGE,
                "device.bridge.invalidRequest",
                &format!("hello request exceeds the 16 KiB limit: {error}"),
            )
        })?;
    if !state
        .token
        .matches_authorization(headers.get(AUTHORIZATION))
    {
        return Err(ApiError::new(
            StatusCode::UNAUTHORIZED,
            "device.bridge.unauthorized",
            "valid bearer token required",
        ));
    }
    if !is_json(&headers) {
        return Err(ApiError::new(
            StatusCode::BAD_REQUEST,
            "device.bridge.invalidRequest",
            "content type must be application/json",
        ));
    }

    let hello = DeviceHello::from_slice(&body).map_err(|error| {
        ApiError::new(
            StatusCode::BAD_REQUEST,
            "device.bridge.invalidRequest",
            &error.message,
        )
    })?;
    if let Some(snapshot) = state.snapshot_for_profile(&hello.device_profile_id).await? {
        return Ok(Json(snapshot));
    }
    let negotiated = negotiate(&hello, &state.policy)
        .map_err(|error| ApiError::from_device(StatusCode::UNPROCESSABLE_ENTITY, error))?;
    let snapshot = state.claim_session(&hello, &negotiated).await?;

    Ok(Json(snapshot))
}

impl ServerState {
    async fn snapshot_for_profile(
        &self,
        device_profile_id: &str,
    ) -> Result<Option<SessionSnapshot>, ApiError> {
        let session = self.session.read().await;
        match session.as_ref() {
            Some(snapshot) if snapshot.device_profile_id == device_profile_id => {
                Ok(Some(snapshot.clone()))
            }
            Some(_) => Err(ApiError::new(
                StatusCode::CONFLICT,
                "device.bridge.sessionActive",
                "a different device profile already owns this server",
            )),
            None => Ok(None),
        }
    }

    async fn claim_session(
        &self,
        hello: &DeviceHello,
        negotiated: &NegotiatedSession,
    ) -> Result<SessionSnapshot, ApiError> {
        let mut session = self.session.write().await;
        match session.as_ref() {
            Some(snapshot) if snapshot.device_profile_id == hello.device_profile_id => {
                Ok(snapshot.clone())
            }
            Some(_) => Err(ApiError::new(
                StatusCode::CONFLICT,
                "device.bridge.sessionActive",
                "a different device profile already owns this server",
            )),
            None => {
                let snapshot = SessionSnapshot::create(hello, negotiated).map_err(|error| {
                    ApiError::from_server(StatusCode::INTERNAL_SERVER_ERROR, error)
                })?;
                *session = Some(snapshot.clone());
                Ok(snapshot)
            }
        }
    }
}

fn is_json(headers: &HeaderMap) -> bool {
    headers
        .get(CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.split(';').next())
        .is_some_and(|value| value.trim().eq_ignore_ascii_case("application/json"))
}

struct ApiError {
    status: StatusCode,
    diagnostic: ServerDiagnostic,
}

impl ApiError {
    fn new(status: StatusCode, code: &str, message: &str) -> Self {
        Self {
            status,
            diagnostic: ServerDiagnostic::new(code, message),
        }
    }

    fn from_device(status: StatusCode, diagnostic: DeviceDiagnostic) -> Self {
        Self {
            status,
            diagnostic: ServerDiagnostic::new(diagnostic.code, diagnostic.message),
        }
    }

    fn from_server(status: StatusCode, diagnostic: ServerDiagnostic) -> Self {
        Self { status, diagnostic }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.diagnostic)).into_response()
    }
}
