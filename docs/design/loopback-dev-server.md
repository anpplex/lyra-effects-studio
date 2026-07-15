# Loopback Dev Server design

## Scope

This is M3 slice 2: a cross-platform host-side Dev Bridge server that gives one Lyra runtime a short-lived, authenticated loopback session. It follows the completed portable `lyra-device` protocol, revision and FakeADB core.

The server slice itself creates no real ADB process, Android source change, Tauri command or Studio UI. M3 slice 3E now consumes its private endpoint only inside an explicit Tauri mapping action; the server still never launches ADB or exposes provisioning material itself.

## Alternatives considered

1. **A standalone loopback server crate — selected.** `lyra-dev-server` depends on `lyra-device`, owns HTTP lifecycle and remains testable on macOS, Windows and Linux without Tauri. It preserves the core rule that `lyra-device` itself has no HTTP or UI dependency.
2. **Put the HTTP server in `src-tauri`.** This would make initial wiring smaller, but hides protocol and security behavior behind a desktop-only runtime and makes cross-platform integration tests less direct.
3. **Start with WebSocket streaming.** Live events will eventually need a duplex channel, but adding WebSocket framing, reconnect semantics and flow control before the first authenticated session makes the first network boundary needlessly broad.

## Architecture

`lyra-dev-server` uses Axum 0.8 and Tokio 1 to bind an ephemeral IPv4 loopback listener at `127.0.0.1:0`. It exposes `DevServer::start(policy)`, which generates a 256-bit random bearer token with `getrandom`, starts the listener and returns a `DevServerEndpoint` containing the loopback URL and an authorization value for trusted provisioning.

The server has one endpoint in this slice:

```text
POST /v1/hello
Authorization: Bearer <256-bit token>
Content-Type: application/json
Body: DeviceHello
```

The request body is decoded by `DeviceHello::from_slice`, then negotiated with the host `HostPolicy`. A successful response is JSON with `sessionId`, `deviceProfileId`, negotiated `protocolVersion` and sorted `capabilities`. The host can query an immutable `SessionSnapshot` from the `DevServer` handle. The Tauri controller projects that snapshot into a smaller `DeviceBridgeSession` before serializing it for Studio, dropping `sessionId` alongside all provisioning material.

The first accepted hello creates the session. Repeating the same `deviceProfileId` is idempotent and returns the existing snapshot, which supports a runtime reconnect. A different profile is rejected until the host stops and starts a new server; this prevents one endpoint from silently switching vehicles or displays.

## Security and failure model

- The listener binds only `127.0.0.1`, never `0.0.0.0`, IPv6 wildcard or a caller-supplied host.
- The 32-byte token is generated from the operating system random source, encoded as lowercase hexadecimal and never included in response bodies, diagnostics or `Debug` output.
- `POST /v1/hello` requires an exact `Authorization: Bearer` token. Missing or invalid credentials return HTTP 401 with `device.bridge.unauthorized`.
- JSON requests are capped at 16 KiB. Malformed, unsupported or oversized requests return a structured `device.bridge.invalidRequest` diagnostic without creating a session.
- Malformed hello request bodies are normalized to `device.bridge.invalidRequest`; negotiation failures retain their portable `device.protocol.*` or `device.capability.*` codes and return HTTP 422.
- A hello for another device profile returns HTTP 409 with `device.bridge.sessionActive`; the existing session is preserved.
- The server exposes no arbitrary command, filesystem, Pack, ADB, network forwarding or theme mutation endpoint.
- `DevServer::shutdown` is explicit and graceful. Dropping the endpoint alone must not be treated as shutdown.

## Public boundary

```rust
pub struct DevServer;
pub struct DevServerEndpoint;
pub struct SessionSnapshot;

impl DevServer {
    pub async fn start(policy: HostPolicy) -> Result<(Self, DevServerEndpoint), ServerDiagnostic>;
    pub async fn session_snapshot(&self) -> Option<SessionSnapshot>;
    pub async fn shutdown(self) -> Result<(), ServerDiagnostic>;
}
```

`DevServerEndpoint` exposes the loopback socket address, `hello_url()` and a provisioning-only `authorization_value()` method. Tauri retains that endpoint for one explicit typed ADB reverse request; it never serializes the endpoint or authorization value. `SessionSnapshot` remains a host-side server read model, while the Studio-facing projection omits its session ID.

## Testing

Tests use real loopback TCP requests, not a mocked router. They verify that the listener uses IPv4 loopback, rejects missing and incorrect bearer tokens, accepts the existing hello fixture, preserves deterministic capability negotiation, maps incompatible/missing-capability failures to stable JSON diagnostics, refuses a second profile and shuts down cleanly. The existing portable crate tests remain the authority for individual hello and negotiation semantics.

CI runs the new crate on macOS, Windows and Linux. No test requires `adb`, Android SDK tools, a device or an open LAN port.

## Follow-on boundary

M3 slice 3A provides narrow Tauri commands and a Studio control that start/stop this server and display non-secret connection state. M3 slice 3B adds the Rust-only `lyra-device` coordinator that requests a typed ADB reverse mapping through injected `FakeAdb` coverage. M3 slice 3C adds the explicit-path, fixed-argv `lyra-adb` process adapter with fake-executor coverage. M3 slice 3D adds a native-chooser, user-gated `devices -l` preflight. M3 slice 3E explicitly derives the endpoint port privately and creates one owned mapping only after a user action; an explicit bridge stop removes that mapping before shutdown and refuses to stop on cleanup failure. Android/runtime provisioning remains separately scoped, and no app-exit cleanup or automatic mapping exists.
