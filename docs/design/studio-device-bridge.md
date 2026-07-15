# Studio Device Bridge controls design

## Scope

This is M3 slice 3A: Lyra Effects Studio can start, inspect and stop the
already-authenticated loopback Dev Bridge from its Tauri shell. The Studio UI
shows only a small, actionable device-connection state and a connected
runtime's non-secret session summary.

This slice deliberately does not execute `adb`, discover a device, change the
Android application, provision an endpoint to a vehicle, transfer a Pack or
send a revision command. The next M3 slice will compose the existing typed
`AdbClient` / `FakeAdb` boundary with the retained endpoint; a real process
adapter remains later still.

## Alternatives considered

1. **Tauri-owned lifecycle controller with a minimal Studio status control — selected.**
   `lyra-dev-server` continues to own network and protocol behavior, while
   `src-tauri` owns only desktop lifetime and serializes a non-secret read
   model. It gives users an immediate start/stop workflow without widening the
   public protocol or exposing a credential.
2. **Expose the loopback URL and bearer to the Studio frontend.** This makes a
   browser implementation superficially convenient, but puts trusted
   provisioning material in renderer memory and browser devtools. It is
   rejected even though the listener is loopback-only.
3. **Add real ADB reverse setup now.** The portable typed API exists, but no
   process adapter or Android receiver is yet covered by the fake-first test
   suite. Doing so now would turn a safe lifecycle control into an untestable
   vehicle integration.

## Architecture

`src-tauri/src/device_bridge.rs` owns a `DeviceBridgeController` managed as
Tauri application state. Its private `tokio::sync::Mutex<Option<RunningBridge>>`
contains both the `DevServer` and its `DevServerEndpoint`. The endpoint never
crosses the Tauri command boundary: retaining it only makes the future typed
ADB reverse request possible inside Rust.

The controller creates one fixed `HostPolicy` for Dev Bridge v1:

- host protocol version `1.2.0`;
- supported capabilities `activate` and `stageRevision`;
- required capability `stageRevision`.

Calling start while a server already exists is idempotent. Stop removes the
running bridge from the mutex before awaiting graceful shutdown, so a slow
server task never holds the controller lock. A status read queries the
existing `DevServer::session_snapshot()` and returns one of these frontend-safe
states:

| State | Meaning | Session |
|---|---|---|
| `stopped` | No loopback listener is owned by Studio. | Absent |
| `waiting` | A loopback listener is ready, but no runtime has authenticated. | Absent |
| `connected` | One runtime profile completed authenticated hello negotiation. | Non-secret device summary |

The Tauri command surface is intentionally limited to asynchronous
`get_device_bridge_status`, `start_device_bridge` and `stop_device_bridge`.
Each returns `DeviceBridgeStatus`; failures are returned as the existing
stable `device.bridge.*` diagnostic text. No command accepts an address, port,
token, shell fragment, Pack path or arbitrary protocol message.

The controller maps the server's `SessionSnapshot` into a dedicated
`DeviceBridgeSession` before serializing status. That projection keeps only
`deviceProfileId`, `protocolVersion` and sorted `capabilities`; its internal
random `sessionId` never crosses the desktop boundary.

The React backend facade gains equivalent typed methods. Browser mode uses an
in-memory fixture controller whose state changes from `stopped` to `waiting`
and back, so UI tests exercise the same contract without a listener. On mount,
Studio refreshes status once; its header control offers Start or Stop and uses
the same state labels. A connected state shows device profile, negotiated
protocol and capabilities but never a URL, port, session ID or bearer token.

## Security and failure model

- The existing server remains the only component that binds sockets, and it
  binds only ephemeral IPv4 loopback.
- `DevServerEndpoint::authorization_value()` is not called by a Tauri command
  or serialized into `DeviceBridgeStatus`; frontend types do not contain a
  token, URL or port field.
- Starting twice does not create a second listener. Stopping an already
  stopped bridge succeeds and reports `stopped`.
- Server startup and shutdown failures retain their `device.bridge.*` code in
  the rejected command message. The UI preserves the last valid status and
  shows the failure as an inline bridge message.
- A renderer cannot create a session directly. Only an authenticated runtime
  using trusted, future Rust-side provisioning can change `waiting` to
  `connected`.
- The controller does not grant any ADB or filesystem authority. Future ADB
  work must use the typed `AdbClient` operations and FakeADB transcript tests.

## Testing

Rust unit tests exercise controller status transitions with a real
`lyra-dev-server` loopback listener: initial stopped state, idempotent start,
authenticated fixture hello producing connected state, stop returning to
stopped state, and JSON serialization that contains neither bearer nor
session-ID data. Tests may read private endpoint data only inside the Rust
test module to send the trusted fixture hello; the public command type remains
secret-free.

TypeScript tests assert the exact three Tauri command names and the typed
request/response shape. Browser-backend tests cover fixture start/stop
transitions. React tests verify the rendered labels and Start/Stop behavior.
Full workspace, lint, frontend build and debug Tauri compilation remain the
release checks.

## Follow-on boundary

M3 slice 3B now provides a Rust-only deployment coordinator that asks an
injected `AdbClient` to select exactly one ready device and create an ADB
reverse mapping from a future Tauri integration's retained loopback endpoint
to the fixed Android Dev Bridge port. It is proven solely with `FakeAdb`
transcripts and adds no real process execution. A separately scoped Tauri
adapter, process test suite and Android/runtime integration remain required
before a vehicle can consume the endpoint.
