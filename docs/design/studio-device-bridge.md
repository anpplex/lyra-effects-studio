# Studio Device Bridge controls design

## Scope

This is M3 slice 3A: Lyra Effects Studio can start, inspect and stop the
already-authenticated loopback Dev Bridge from its Tauri shell. The Studio UI
shows only a small, actionable device-connection state and a connected
runtime's non-secret session summary.

The Dev Bridge lifecycle itself does not automatically execute `adb`, discover
a device, change the Android application, provision an endpoint to a vehicle,
transfer a Pack or send a revision command. M3 slice 3B composed the typed
`AdbClient` / `FakeAdb` boundary with the retained endpoint, 3C added a
fixed-argv process adapter, and 3D added native-chooser device preflight. M3
slice 3E adds one separately visible mapping action: it consumes the private
endpoint only after **Enable mapping**, and an explicit **Stop bridge** first
removes an owned mapping. It does not add automatic mapping, app-exit cleanup
or runtime provisioning.

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
3. **Add real ADB reverse setup now.** The portable typed API exists. The
   later isolated process adapter is fake-executor tested, but connecting it
   here would still combine device action and lifecycle controls before an
   explicit user flow and Android receiver are covered.

## Architecture

`src-tauri/src/device_bridge.rs` owns a `DeviceBridgeController` managed as
Tauri application state. Its private `tokio::sync::Mutex<Option<RunningBridge>>`
contains both the `DevServer` and its `DevServerEndpoint`. The endpoint never
crosses the Tauri command boundary: retaining it makes the current typed ADB
reverse request possible only inside Rust.

The same controller separately owns in-memory ADB preflight and mapping state
plus an injected private ADB-client factory. A Rust-owned native picker
supplies a canonical regular-file path; that path remains private. An explicit
**Check devices** action starts a blocking `SystemAdb::list_devices` call,
which produces the fixed `devices -l` argv. A separately clicked **Enable
mapping** derives the private listener port, freshly selects exactly one ready
device through `DevBridgeReverseCoordinator`, and retains both the typed
mapping and client only for cleanup. Preflight itself neither maps nor changes
the Dev Bridge lifecycle state.

The controller creates one fixed `HostPolicy` for Dev Bridge v1:

- host protocol version `1.2.0`;
- supported capabilities `activate` and `stageRevision`;
- required capability `stageRevision`.

Calling start while a server already exists is idempotent. Stop first takes the
same private mapping-operation gate used for enable/remove. If a mapping is
active or cleanup-failed, it removes that mapping before taking the running
bridge from the mutex; cleanup failure leaves the listener running and remains
explicitly retryable. A status read queries the existing
`DevServer::session_snapshot()` and returns one of these frontend-safe states:

| State | Meaning | Session |
|---|---|---|
| `stopped` | No loopback listener is owned by Studio. | Absent |
| `waiting` | A loopback listener is ready, but no runtime has authenticated. | Absent |
| `connected` | One runtime profile completed authenticated hello negotiation. | Non-secret device summary |

The lifecycle command surface is intentionally limited to asynchronous
`get_device_bridge_status`, `start_device_bridge` and `stop_device_bridge`.
Each returns `DeviceBridgeStatus`; failures are returned as the existing
stable `device.bridge.*` diagnostic text. The separate preflight surface adds
`get_device_bridge_adb_status`, `choose_device_bridge_adb_executable` and
`check_device_bridge_adb`. The mapping surface adds
`get_device_bridge_mapping_status`, `enable_device_bridge_mapping` and
`disable_device_bridge_mapping`; each takes no renderer argument and returns
only `{ readiness }`. None accepts an address, port, token, executable path,
serial, shell fragment, Pack path or arbitrary protocol message.

The controller maps the server's `SessionSnapshot` into a dedicated
`DeviceBridgeSession` before serializing status. That projection keeps only
`deviceProfileId`, `protocolVersion` and sorted `capabilities`; its internal
random `sessionId` never crosses the desktop boundary.

The React backend facade gains equivalent typed methods. Browser mode uses an
in-memory fixture controller whose bridge state changes from `stopped` to
`waiting` and back, its preflight state changes from `unconfigured` to
`notChecked` to `oneReadyDevice`, and its mapping state changes only from
`inactive` to `active` and back. On mount, Studio refreshes the three safe
status projections independently. The header offers Start or Stop, **Select
ADB**, **Check devices**, and an explicit **Enable mapping** / **Remove
mapping** control. Mapping stays disabled until the bridge is running and
preflight reports exactly one ready device. A connected bridge state shows
device profile, negotiated protocol and capabilities but never a URL, port,
session ID or bearer token. The preflight and mapping controls never display a
path, serial, process output or raw command diagnostic.

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
- The controller grants only narrow user-gated ADB actions. Its private picker
  path is canonicalized to a regular file; **Check devices** calls only
  `SystemAdb::list_devices`, while explicit map/remove actions use only the
  typed coordinator or retained cleanup. There is no automatic discovery or
  mapping, app-exit cleanup, Pack push, Android change or background retry.

## Testing

Rust unit tests exercise controller status transitions with a real
`lyra-dev-server` loopback listener: initial stopped state, idempotent start,
authenticated fixture hello producing connected state, stop returning to
stopped state, and JSON serialization that contains neither bearer nor
session-ID data. Tests may read private endpoint data only inside the Rust
test module to send the trusted fixture hello; the public command type remains
secret-free.

TypeScript tests assert the exact lifecycle, preflight and no-argument mapping
Tauri command names and typed response shapes. Browser-backend tests cover
bridge, preflight and mapping fixture transitions. React tests verify rendered
safe labels, Start/Stop behavior, the disabled-until-selected ADB check and
the explicit mapping lifecycle. Full workspace, lint, frontend build and debug
Tauri compilation remain the release checks.

## Follow-on boundary

M3 slice 3E completes the separately scoped Tauri reverse action and mapping
cleanup policy. Android/runtime provisioning, Pack transfer, revision commands
and remote-theme activation remain required before a vehicle can consume the
endpoint; no renderer command surface is added for those future concerns.
