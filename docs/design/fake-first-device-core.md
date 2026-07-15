# Fake-first device core design

## Scope

M3 starts with a portable `lyra-device` crate that models Dev Bridge negotiation, revision lifecycle and ADB operations without opening sockets, launching `adb` or changing Lyra Android. Every behavior is exercised with shared JSON fixtures and an in-memory fake.

M3's completed fake-first foundation is composed in five slices:

1. device protocol, revision state and FakeADB;
2. loopback Dev Server and authenticated sessions;
3. Tauri commands and Studio device UI;
4. portable single-device ADB reverse coordination;
5. explicit-path, fixed-argv ADB process adaptation.

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

Unit tests cover protocol decoding, unknown-field retention, capability negotiation, every allowed/forbidden revision transition and ADB input validation. Transcript tests cover zero, one and multiple devices, offline devices, reverse failure, cleanup failure and push failure. CI runs the crate on macOS, Windows and Linux with no real ADB executable.

## Follow-on boundary

The Dev Server slice may depend on `lyra-device`; the crate must not depend on Tauri, HTTP/WebSocket libraries or UI types. `lyra-adb` now implements the same trait through an explicitly configured binary without a shell, but Tauri does not construct it yet. A later opt-in Tauri action will derive its local port from the private loopback endpoint. Android remains unchanged until FakeADB, fake process and Fake Bridge coverage are complete.
