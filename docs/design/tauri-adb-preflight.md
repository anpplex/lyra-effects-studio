# User-gated Tauri ADB preflight design

## Scope

M3 slice 3D connects the completed `lyra-adb` process adapter to Lyra Effects
Studio through a small, user-gated **ADB preflight**. A user explicitly selects
an executable through a native dialog and then explicitly asks Studio to check
ADB devices. Studio reports only a safe readiness summary; preflight itself
does not create a reverse mapping, transfer a Pack, change Android, discover an
SDK or run an ADB process automatically.

This slice was deliberately narrower than creating the Dev Bridge reverse
mapping. It established the trusted executable-selection and testable Tauri
process boundary first; M3 slice 3E now owns explicit mapping, cleanup retries
and stop-versus-cleanup behavior as a separately designed flow.

## Alternatives considered

1. **User-gated preflight with a native executable chooser — selected.** The
   native side retains the selected canonical file path, and a separate button
   performs only the typed `devices -l` operation. The renderer receives a
   readiness projection, never the path, serials, bridge endpoint or bearer.
2. **Create the ADB reverse mapping in this slice.** This gives a vehicle a
   usable route sooner, but requires an owned `SystemAdb` plus `ReverseMapping`,
   deterministic cleanup and concurrency policy when the bridge stops. That is
   a separate follow-on slice.
3. **Discover an SDK or default ADB path.** This is rejected because it makes
   an unrequested host process capability implicit and conflicts with the
   explicit-path contract of `lyra-adb`.

## Architecture

`src-tauri` gains a dependency on `lyra-adb`. `DeviceBridgeController` keeps
two private pieces of state alongside the existing loopback-server owner:

- an optional, canonical executable path selected from a Rust-owned native file
  dialog; and
- the last `AdbPreflightStatus` projection.

The public projection is deliberately small:

| Field | Values | Purpose |
|---|---|---|
| `configured` | boolean | An executable was selected in this app session. |
| `readiness` | `unconfigured`, `notChecked`, `noReadyDevice`, `oneReadyDevice`, `multipleReadyDevices`, `error` | Safe operational summary. |

It contains no executable path, ADB serial, raw ADB output, local listener
port, URL, session ID or bearer. The configuration remains in memory only and
is cleared when Studio exits.

The native command that chooses an executable takes no path argument. It uses
the existing Tauri dialog plugin from Rust, validates that a selected path
canonicalizes to a regular file, stores it privately and resets readiness to
`notChecked`. Cancelling the native dialog preserves the previous state and
does not launch a process.

The native preflight command takes no executable, serial, port or command
argument. It clones the retained path and runs a private injected probe on
`tokio::task::spawn_blocking`. The production probe constructs
`lyra_adb::SystemAdb` and calls only `AdbClient::list_devices`, which produces
the existing fixed `devices -l` argv. Test probes are in-memory fakes and do
not launch a process.

Ready transports are counted using the existing portable
`AdbDeviceState::Device` state. The count maps to `noReadyDevice`,
`oneReadyDevice` or `multipleReadyDevices`; offline and unauthorized transports
never become ready. A typed probe failure marks the status `error` and returns
the existing stable diagnostic without stderr or raw output. If configuration
changes while a probe is in flight, the stale result is discarded rather than
overwriting the newer configuration state.

The existing Dev Bridge listener is independent: preflight can be run before
or after it starts, and starting or stopping the listener never launches ADB
solely because of preflight. M3 slice 3E uses the same private selected path
only after an explicit mapping action to establish a
`DevBridgeReverseCoordinator` mapping; explicit bridge stop may then clean up
that owned mapping before listener shutdown.

## Tauri and Studio surface

Tauri exposes exactly three additional narrow commands:

- `get_device_bridge_adb_status` returns the safe projection;
- `choose_device_bridge_adb_executable` opens the Rust-owned native picker and
  returns the projection; and
- `check_device_bridge_adb` performs one explicit preflight and returns the
  projection.

The browser backend implements the same shape with an in-memory configured,
one-ready-device fixture. The Studio header adds compact **Select ADB** and
**Check devices** controls near the existing bridge control. Controls show
their own busy state; the check button is disabled until an executable is
configured. The UI renders the readiness label or a generic stable error state,
not an executable path or raw process diagnostic.

M3 slice 3E adds a separate mapping projection and no-argument enable/remove
commands. It remains disabled until this preflight shows exactly one ready
device and a loopback bridge is running; it never reuses the preflight command
as an automatic mapping trigger.

## Failure, concurrency and security model

- No command accepts a raw shell fragment, ADB subcommand, serial, port or
  host path.
- The only process-capable input is selected through an OS-native dialog owned
  by Rust; it is canonicalized and must be a regular file before retention.
- The process is never started by application launch, bridge lifecycle,
  polling, status reads or configuration selection. It starts only after the
  user invokes **Check devices**.
- The process runs off Tauri's async executor. It uses the already-tested
  fixed-argv `SystemAdb` boundary and forwards no stderr or raw output.
- A stale asynchronous result cannot replace a newer selected executable's
  status. Cancellation and failed configuration leave the previous valid state
  intact.
- Preflight itself creates no ADB reverse mapping, listener, Android change,
  Pack transfer, persistence or automatic retry loop. The separate mapping
  flow adds only explicit enable/remove and explicit stop-time cleanup; it does
  not add automatic mapping or app-exit cleanup.

## Testing and release gates

Rust controller tests inject a queue-backed ADB client factory. They prove
selection does not probe, unconfigured checks do not probe, zero/one/multiple
ready counts map to the safe projection, typed errors do not expose raw output
and a configuration change rejects a stale result. Mapping tests additionally
cover explicit establish/remove and stop-time cleanup. Existing `lyra-adb` unit
tests continue to prove exact argv without running ADB.

TypeScript backend tests assert the three new command names and browser-fixture
transitions. React tests cover the disabled, configured and ready fixture flow;
Rust controller tests cover safe error projection and stale-result handling.
The full Studio lint/test/build, Rust format/Clippy/test, release build and
no-bundle Tauri debug build remain required. CI must not install, discover or
execute a real ADB binary.

## Follow-on boundary

M3 slice 3E now retains a successful `SystemAdb` and `ReverseMapping` under an
explicit user action, derives the loopback port inside Rust and provides
cleanup semantics. Android runtime provisioning, Pack transfer, revision
commands and remote-theme activation remain out of scope.
