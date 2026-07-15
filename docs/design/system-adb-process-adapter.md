# System ADB process adapter design

## Scope

M3 slice 3C adds a separate cross-platform Rust crate, `lyra-adb`, that
implements the portable `lyra_device::AdbClient` trait through fixed Android
Debug Bridge process invocations. It is the first code that can launch an ADB
binary. M3 slice 3D wires `list_devices` into a user-gated Studio preflight,
and 3E wires the coordinator's typed reverse/remove operations into separately
clicked mapping actions. Rust retains a native-picker-selected executable path
in memory and creates `SystemAdb` only after one of those explicit actions.
Constructing the crate itself still does not discover an SDK, start a server or
run a process.

The adapter does not add automatic device selection, bearer provisioning, Pack
transfer workflow, Android runtime source, command shell, environment-variable
discovery or a connection retry loop. Preflight remains limited to native
selection and a one-shot device-list check. Mapping remains explicitly owned by
the Tauri controller, which delegates selection to the completed portable
reverse coordinator and retains typed cleanup for retry.

## Alternatives considered

1. **Dedicated `lyra-adb` crate with a private injected process executor — selected.**
   `lyra-device` stays free of process and platform dependencies, while one
   adapter can be compiled on macOS, Windows and Linux. Its tests use an
   in-memory executor through the same argv-building and parsing path, so CI
   never needs an installed binary or a connected device.
2. **Put `std::process::Command` in `lyra-device`.** This would make the core
   immediately capable of touching host state and break its existing portable,
   FakeAdb-first boundary.
3. **Launch ADB from Tauri commands.** This would combine renderer authority,
   desktop lifecycle, device policy and process parsing in one layer before
   there is a testable adapter boundary.

## Architecture

`lyra-adb` depends only on `lyra-device` and the Rust standard library. Its
public entry point is:

```rust
pub struct SystemAdb;

impl SystemAdb {
    pub fn from_path(executable: impl Into<PathBuf>) -> Self;
}
```

`SystemAdb` wraps a crate-private generic adapter and command executor. The
production executor uses `std::process::Command::output`; unit tests inject an
in-memory executor into the same private adapter. No public API accepts a
shell string or lets a caller select an arbitrary ADB subcommand.

`SystemAdb` implements the existing typed interface exactly:

| `AdbClient` operation | Fixed process arguments |
|---|---|
| `list_devices` | `devices -l` |
| `reverse(serial, local, remote)` | `-s <serial> reverse tcp:<remote> tcp:<local>` |
| `remove_reverse(serial, remote)` | `-s <serial> reverse --remove tcp:<remote>` |
| `push(serial, local, destination)` | `-s <serial> push <local-path> <device-path>` |

Every argument originates from a validated `lyra-device` value. The executable
path is passed directly to `Command::new`, and every argument is a separate
`OsString`; no call uses a shell, concatenated command line or environment
target selection.

The current Tauri integration does not expose this general `AdbClient` surface
to the renderer. Its no-argument preflight commands retain the selected
canonical path privately and call only `SystemAdb::list_devices` after **Check
devices**. Separate no-argument mapping commands derive a private listener
port and can call only `DevBridgeReverseCoordinator::establish` or the retained
`ReverseMapping::remove` after an explicit user action. It never exposes push,
a raw ADB command, an Android SDK path, serial, endpoint or bearer.

`list_devices` scans only after the exact `List of devices attached` header,
which lets it ignore ADB daemon startup prelude lines. It accepts the portable
states `device`, `offline` and `unauthorized`; a malformed UTF-8/header/row or
unsafe serial returns `device.adb.invalidDeviceList`, and any other state
returns `device.adb.unsupportedDeviceState`.

## Diagnostics and recovery

- Failure to spawn the configured executable returns
  `device.adb.launchFailed`.
- A non-zero exit from one of the fixed commands returns
  `device.adb.commandFailed`; unbounded stderr is not copied into the
  diagnostic.
- Parsing errors never partially return a device list.
- `reverse`, cleanup and push preserve the existing typed input validation
  because callers cannot reach the adapter with raw serials, ports or device
  paths.
- The adapter does not cache mappings or choose a device. The Tauri mapping
  controller is its explicit owner and passes it to
  `DevBridgeReverseCoordinator`, whose selection and cleanup semantics remain
  unchanged.

## Testing

A crate-private fake executor records expected executable paths and argument
vectors, then returns configured successful output, command failure or launch
failure. Tests cover:

1. the full four-command argv sequence with a real `AdbClient` surface;
2. `devices -l` parsing with daemon prelude plus `device`, `offline` and
   `unauthorized` rows;
3. malformed header/row/UTF-8 and unsupported-state diagnostics;
4. launch and non-zero command failures without exposing command output;
5. transcript exhaustion, proving no unexpected process invocation occurred.

No test invokes `adb`, reads Android SDK locations, opens a network listener or
requires an Android device. CI adds `lyra-adb` to the existing Linux and
Windows portable-core gate; macOS already validates the whole workspace.

## Follow-on boundary

The completed Tauri integration uses the selected private executable path,
derives the listener port from the private `DevServerEndpoint`, and calls the
reverse coordinator only after a visible mapping action. It keeps the bearer
private and owns mapping cleanup for explicit retry or explicit bridge stop.
Runtime provisioning, Pack/revision transfer and Android integration remain
separate before the adapter can control a vehicle runtime.
