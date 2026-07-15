# Device ADB reverse coordinator design

## Scope

M3 slice 3B adds a portable, FakeAdb-first coordinator that prepares one
typed ADB reverse mapping for the authenticated loopback Dev Bridge. It is a
pure `lyra-device` capability: it neither starts a process nor depends on
Tauri, `lyra-dev-server`, a network client, the Studio frontend or Android.

The coordinator is intentionally the first point that composes existing
`AdbClient` operations. It does not transfer a Pack, start an Android runtime,
provision the bearer, or expose a UI command. Those remain later slices.

## Alternatives considered

1. **Portable coordinator with an injected `AdbClient` — selected.** The
   selection and cleanup rules are independently testable on every CI platform
   with `FakeAdb`. A future Tauri adapter can derive the host port from its
   retained `DevServerEndpoint` without moving any server secret across a
   boundary.
2. **Put selection and reverse calls directly in `DeviceBridgeController`.**
   This would make the initial integration shorter but couples device policy to
   the desktop runtime before a real process adapter exists.
3. **Add a real `adb` process adapter now.** This would require host SDK path,
   process, vehicle and Android failure handling before the portable behavior
   has fake-first coverage. It is explicitly deferred.

## Portable contract

`lyra-device` exports three new types:

- `DevBridgeReverseRequest`, constructed from one validated `LocalPort`.
  Its Android-side port is the fixed `DEV_BRIDGE_REMOTE_PORT` value `49321`;
  callers cannot pass an arbitrary remote port for this protocol.
- `ReverseMapping`, the successful result containing the selected serial and
  both typed ports. Its `remove` method delegates only to
  `AdbClient::remove_reverse` and may be retried after an adapter failure.
- `DevBridgeReverseCoordinator`, a stateless entry point that lists devices,
  selects exactly one ready transport, and calls `AdbClient::reverse`.

The coordinator considers only `AdbDeviceState::Device` eligible. Zero ready
devices, or more than one ready device, return respectively
`device.adb.noEligibleDevice` and `device.adb.multipleEligibleDevices`.
Offline and unauthorized transports are not implicitly selected. In either
selection failure path the coordinator makes no reverse or cleanup call.

On success, the coordinator makes exactly this typed sequence:

```text
list_devices -> reverse(serial, host ephemeral port, device port 49321)
```

Explicit cleanup is a separate operation:

```text
remove_reverse(serial, device port 49321)
```

Adapter diagnostics, including `device.adb.reverseFailed` and a future
remove failure, are preserved without rewriting their stable codes.

## Security and recovery

- The API accepts no raw shell string, host, URL, bearer token, Android path or
  mutable remote-port value.
- Selecting exactly one ready device prevents an automatic reverse mapping
  from silently targeting an arbitrary vehicle when several transports exist.
- The mapping object does not own a process or socket; its successful creation
  is a description of an adapter operation, not proof that a runtime has
  authenticated.
- Cleanup is explicit and retryable. The coordinator does not hide a failed
  `remove_reverse`, and it does not attempt a speculative cleanup after
  `reverse` fails.

## Testing

Tests use only ordered `FakeAdb` transcripts. They cover one-device success
and removal, no eligible device, multiple eligible devices, propagation of a
configured reverse failure, propagation of a removal failure, and transcript
exhaustion. No test starts `adb`, opens a socket, inspects Android SDK paths or
requires a connected device.

## Follow-on boundary

A later, separately scoped Tauri integration may obtain the loopback listener
port from its private `DevServerEndpoint`, construct a
`DevBridgeReverseRequest`, and explicitly create `lyra_adb::SystemAdb` from a
trusted executable path. That integration must keep the endpoint bearer
private, expose no raw ADB command surface, make the action user-visible and
add process-level tests before it can reach an Android runtime.
