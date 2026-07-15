# Dev Bridge v1

Dev Bridge v1 is the portable contract between Lyra Effects Studio and a future Lyra Android runtime. The current implementation is fake-first for ADB and Android, while its host-side hello server is exercised with real IPv4-loopback TCP.

## Hello and negotiation

The device starts a session with a JSON hello message:

```json
{
  "type": "hello",
  "protocolVersion": "1.0.0",
  "runtimeVersion": "0.11.20-lyricify",
  "deviceProfileId": "com.avatr.cluster.4032x284",
  "capabilities": ["stageRevision", "activate", "console"]
}
```

Protocol major version `1` is mandatory. Different minor or patch versions are compatible, and the negotiated version is the lower of the two semantic versions. Negotiated capabilities are the sorted intersection of host and device advertisements. A host-required capability must also be advertised by the device; host-only capabilities are never silently added.

Unknown hello fields are retained so vendor metadata can pass through newer hosts without weakening validation of required fields.

## Authenticated loopback hello

The host starts an ephemeral listener at `127.0.0.1:0` and provisions its URL plus a fresh 256-bit bearer value through a trusted future adapter. The only route is:

```text
POST /v1/hello
Authorization: Bearer <token>
Content-Type: application/json
```

The body is the hello JSON above and is limited to 16 KiB. Missing or incorrect credentials return HTTP 401. Malformed, unsupported or oversized requests return a JSON `device.bridge.invalidRequest` diagnostic. Negotiation failures return HTTP 422 and preserve their portable diagnostics.

The first accepted hello creates a non-secret session snapshot containing a random 128-bit session ID, device profile, negotiated protocol version and sorted capability intersection. A reconnect from the same profile returns the existing snapshot. A different profile returns HTTP 409 with `device.bridge.sessionActive`; it cannot replace the current session.

## Revision lifecycle

Every theme payload is identified by a 64-character lowercase SHA-256 revision ID. The normal lifecycle is:

```text
Draft -> LocallyValidated -> Staged -> Active -> Ready -> Superseded
                              |          |
                              v          v
                           Rejected    Failed -> RolledBack
```

`RestoreFormalPack` provides an explicit escape path to the APK-bundled formal Pack. Activation records the previous ready revision when one exists, making rollback deterministic. An illegal event returns `device.revision.invalidTransition` and does not mutate the current state.

## Typed ADB boundary

`AdbClient` exposes only four operations:

- list devices, including `device`, `offline` and `unauthorized` states;
- create a TCP reverse mapping;
- remove a TCP reverse mapping;
- push a trusted host file to an Android destination.

Serials, ports and device paths are validated types. There is no raw command-string method. Device paths must be absolute and normalized, ports must be non-zero, and serials reject empty, option-like or shell-shaped input.

`FakeAdb` consumes an ordered JSON transcript and verifies operation names and typed arguments exactly. Its fixtures cover single-device deployment, zero or multiple transports, offline and unauthorized devices, configured transport failures, unsafe inputs and unexpected call order.

## Stable diagnostics

Callers branch on codes rather than English messages. The v1 core currently defines:

| Code | Meaning |
|---|---|
| `device.protocol.invalid` | Core API rejects a malformed hello, version or host policy |
| `device.protocol.incompatible` | Host and device major versions differ |
| `device.capability.missing` | A required capability was not advertised |
| `device.revision.invalidId` | Revision ID is not lowercase SHA-256 |
| `device.revision.invalidTransition` | Revision event is illegal for the current state |
| `device.adb.invalidSerial` | Device serial is unsafe or malformed |
| `device.adb.invalidLocalPort` | Local port is zero |
| `device.adb.invalidRemotePort` | Remote port is zero |
| `device.adb.invalidDevicePath` | Android destination is relative or traversing |
| `device.fakeAdb.invalidTranscript` | Transcript JSON is malformed |
| `device.fakeAdb.unexpectedCall` | Operation order or arguments differ from the transcript |
| `device.fakeAdb.pendingCalls` | A test finished before consuming every configured operation |
| `device.bridge.unauthorized` | Loopback hello lacks the current bearer token |
| `device.bridge.invalidRequest` | Loopback hello is malformed, unsupported or exceeds 16 KiB |
| `device.bridge.sessionActive` | Another device profile already owns the loopback session |
| `device.bridge.tokenGenerationFailed` | The host could not obtain random bearer bytes |
| `device.bridge.sessionGenerationFailed` | The host could not obtain random session-ID bytes |
| `device.bridge.listenFailed` | The IPv4 loopback listener could not start |

## Current boundary

This milestone does not execute `adb`, discover Android SDK paths, expose a Tauri command or modify the Lyra APK. It does listen only on an ephemeral IPv4 loopback port for the authenticated hello route; it has no LAN listener, WebSocket, command, Pack or filesystem endpoint. Future adapters must remain thin consumers of `lyra-device` and `lyra-dev-server`, preserve these stable codes and receive separate process, transport and Android integration tests.
