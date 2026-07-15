# Fake-first device core design

## Scope

M3 starts with a portable `lyra-device` crate that models Dev Bridge negotiation, revision lifecycle and ADB operations without opening sockets, launching `adb` or changing Lyra Android. Every behavior is exercised with shared JSON fixtures and an in-memory fake.

M3's completed fake-first foundation is composed in seven slices:

1. device protocol, revision state and FakeADB;
2. loopback Dev Server and authenticated sessions;
3. Tauri commands and Studio device UI;
4. portable single-device ADB reverse coordination;
5. explicit-path, fixed-argv ADB process adaptation; and
6. user-gated Tauri ADB readiness preflight; and
7. explicit Tauri-owned reverse mapping and cleanup.

The first slice is complete when success and failure paths can be reproduced on macOS, Windows and Linux without a connected device.

## Alternatives considered

1. **Portable Rust core with injected transport — selected.** Protocol and state remain reusable by the CLI, Tauri and tests. Real process execution is a later adapter, so the first implementation cannot accidentally touch a vehicle.
2. **Implement directly in Tauri commands.** This is faster initially, but couples protocol behavior to the desktop runtime and makes Windows/Linux and failure testing harder.
3. **Run an external device daemon.** This isolates processes well, but adds installation, lifecycle and authentication complexity before one-device development is proven.

## Components

- `protocol`: versioned hello and command/event envelopes, capability intersection and stable `device.*` diagnostics. Unknown JSON fields are retained.
- `revision`: an explicit state machine for `draft → locallyValidated → staged → active`, rejection, failure, rollback, supersession and formal-Pack restore.
- `adb`: typed device identity and an `AdbClient` trait. Callers request fixed operations such as list, reverse, push and remove-reverse; they never pass arbitrary shell strings.
- `reverse`: a stateless Dev Bridge coordinator that selects exactly one ready transport, maps a caller-supplied host port to the fixed Android port `49321`, and returns an explicit retryable cleanup handle.
- `lyra-adb`: a separate process crate that implements `AdbClient` through a configured executable path, fixed `OsString` argument vectors and fake-executor tests. It is not an automatic desktop integration.
- `tauri preflight`: a Rust-owned native executable chooser and an in-memory, safe readiness projection. It invokes only `list_devices` after the user explicitly requests a check; it never serializes the executable path or ADB device details.
- `tauri mapping`: a private controller-owned `SystemAdb` and `ReverseMapping` pair. Only explicit enable, explicit remove, or an explicit bridge stop can invoke the coordinator or typed cleanup; Studio receives only mapping readiness.
- `fake_adb`: an ordered transcript of expected calls and configured results. Unexpected calls fail the test with a structured diagnostic.
- `Fixtures/Device`: shared valid, incompatible and malformed hello messages plus FakeADB transcripts. Android will consume the same protocol fixtures in M4.

## Security and recovery

- No API accepts an arbitrary command or shell fragment.
- Device serial, local port, remote port and destination paths are validated before an adapter call.
- Protocol major-version mismatch is fatal. Minor-version differences negotiate the capability intersection.
- A revision cannot activate before staging. Failure from an active revision rolls back only to a previously ready revision.
- Restore ends the session without changing D7, `activeEffectId`, installed Packs or APK assets.
- Tokens, sockets, process launching and Android storage are outside this slice.

## Testing

Unit tests cover protocol decoding, unknown-field retention, capability negotiation, every allowed/forbidden revision transition and ADB input validation. Transcript tests cover zero, one and multiple devices, offline devices, reverse failure, cleanup failure and push failure. The Tauri controller uses injected ADB clients for both preflight and mapping, so tests never launch ADB. CI runs the portable crates on macOS, Windows and Linux with no real ADB executable.

## Follow-on boundary

The Dev Server slice may depend on `lyra-device`; the crate must not depend on Tauri, HTTP/WebSocket libraries or UI types. `lyra-adb` implements the same trait through an explicitly configured binary without a shell. Tauri now constructs it only after an explicit **Check devices**, **Enable mapping**, **Remove mapping** or bridge-stop action; preflight calls only `list_devices`, while mapping uses the existing coordinator and retained cleanup handle. It has no automatic discovery or mapping, app-exit cleanup, Pack push or Android action. Android runtime provisioning remains separate from this completed fake-first boundary.
