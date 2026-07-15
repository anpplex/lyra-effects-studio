use std::net::SocketAddr;
use std::sync::Arc;

use axum::body::Bytes;
use axum::extract::State;
use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::routing::post;
use axum::{Json, Router};
use lyra_device::{DeviceDiagnostic, DeviceHello, HostPolicy, negotiate};
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tokio::task::JoinHandle;

use crate::token::BridgeToken;
use crate::{DevServerEndpoint, ServerDiagnostic};

const MAX_HELLO_BYTES: usize = 16 * 1024;

/// An authenticated, single-listener loopback Dev Bridge server.
pub struct DevServer {
    shutdown: oneshot::Sender<()>,
    task: JoinHandle<Result<(), std::io::Error>>,
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
        });
        let router = Router::new()
            .route("/v1/hello", post(hello))
            .layer(axum::extract::DefaultBodyLimit::max(MAX_HELLO_BYTES))
            .with_state(state);
        let (shutdown, shutdown_signal) = oneshot::channel();
        let task = tokio::spawn(async move {
            axum::serve(listener, router)
                .with_graceful_shutdown(async move {
                    let _ = shutdown_signal.await;
                })
                .await
        });

        Ok((
            Self { shutdown, task },
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
}

struct ServerState {
    policy: HostPolicy,
    token: BridgeToken,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct HelloAccepted {
    protocol_version: String,
    capabilities: Vec<String>,
}

async fn hello(
    State(state): State<Arc<ServerState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<HelloAccepted>, ApiError> {
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

    let hello = DeviceHello::from_slice(&body)
        .map_err(|error| ApiError::from_device(StatusCode::BAD_REQUEST, error))?;
    let negotiated = negotiate(&hello, &state.policy)
        .map_err(|error| ApiError::from_device(StatusCode::UNPROCESSABLE_ENTITY, error))?;

    Ok(Json(HelloAccepted {
        protocol_version: negotiated.protocol_version.to_string(),
        capabilities: negotiated
            .capabilities
            .iter()
            .map(ToString::to_string)
            .collect(),
    }))
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
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.status, Json(self.diagnostic)).into_response()
    }
}
