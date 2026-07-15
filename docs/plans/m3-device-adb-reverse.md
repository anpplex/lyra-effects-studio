# M3 Device ADB Reverse Coordinator Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a portable, FakeAdb-proven coordinator that creates and removes one safe Dev Bridge ADB reverse mapping.

**Architecture:** `lyra-device` gains a stateless coordinator and a small request/result model. It selects exactly one `AdbDeviceState::Device` transport, maps the host listener port to the fixed Android Dev Bridge port `49321`, and delegates every operation to injected `AdbClient`. Tauri, real process execution, Android and Studio UI remain outside this plan.

**Tech Stack:** Rust 2024, existing `lyra-device` typed ADB API, `FakeAdb`, Cargo tests, Markdown contract documentation.

## Global Constraints

- Do not start an `adb` process, resolve Android SDK paths, open a socket or change Android/Tauri/UI code.
- Do not add an arbitrary shell, URL, token, device-path or remote-port input to the public API.
- `DEV_BRIDGE_REMOTE_PORT` is exactly `49321`; callers supply only a validated `LocalPort`.
- A reverse mapping is allowed only when exactly one transport has `AdbDeviceState::Device`.
- Return `device.adb.noEligibleDevice` for zero ready transports and `device.adb.multipleEligibleDevices` for more than one; neither path may call `reverse` or `remove_reverse`.
- Preserve adapter diagnostic codes unchanged and use ordered `FakeAdb` transcripts for every behavior.
- Each independently verifiable behavior ends in a Conventional Commit.

## File structure

| File | Responsibility |
|---|---|
| `crates/lyra-device/src/adb.rs` | Exposes the fixed typed Android Dev Bridge port. |
| `crates/lyra-device/src/reverse.rs` | Contains request, selected mapping and stateless coordinator policy. |
| `crates/lyra-device/src/lib.rs` | Re-exports the portable public contract. |
| `crates/lyra-device/tests/reverse_coordinator.rs` | Exercises coordinator policy solely through `FakeAdb`. |
| `Fixtures/Device/adb-reverse-single-device.json` | Shared success/release transcript. |
| `docs/design/*.md`, `docs/protocols/dev-bridge-v1.md`, `README.md`, `docs/architecture/rust-tauri.md` | Record the completed fake-first boundary and deferred real adapter. |

---

### Task 1: Implement and prove the portable reverse coordinator

**Files:**
- Modify: `crates/lyra-device/src/adb.rs`
- Create: `crates/lyra-device/src/reverse.rs`
- Modify: `crates/lyra-device/src/lib.rs`
- Create: `crates/lyra-device/tests/reverse_coordinator.rs`
- Create: `Fixtures/Device/adb-reverse-single-device.json`

**Interfaces:**
- Consumes: `AdbClient`, `AdbDeviceState`, `DeviceDiagnostic`, `DeviceSerial`, `LocalPort`, `RemotePort` and `FakeAdb`.
- Produces:

```rust
pub const DEV_BRIDGE_REMOTE_PORT: RemotePort;

pub struct DevBridgeReverseRequest;
impl DevBridgeReverseRequest {
    pub const fn new(local_port: LocalPort) -> Self;
    pub const fn local_port(self) -> LocalPort;
    pub const fn remote_port(self) -> RemotePort;
}

pub struct ReverseMapping;
impl ReverseMapping {
    pub fn serial(&self) -> &DeviceSerial;
    pub const fn local_port(&self) -> LocalPort;
    pub const fn remote_port(&self) -> RemotePort;
    pub fn remove<C: AdbClient>(&self, adb: &mut C) -> Result<(), DeviceDiagnostic>;
}

pub struct DevBridgeReverseCoordinator;
impl DevBridgeReverseCoordinator {
    pub fn establish<C: AdbClient>(
        adb: &mut C,
        request: DevBridgeReverseRequest,
    ) -> Result<ReverseMapping, DeviceDiagnostic>;
}
```

- [ ] **Step 1: Write the failing coordinator contract tests and success transcript.**

Create `Fixtures/Device/adb-reverse-single-device.json` with this ordered transcript:

```json
{
  "steps": [
    {
      "operation": "listDevices",
      "result": {
        "devices": [{ "serial": "AVATR-CLUSTER-01", "state": "device" }]
      }
    },
    {
      "operation": "reverse",
      "serial": "AVATR-CLUSTER-01",
      "localPort": 49321,
      "remotePort": 49321,
      "result": {}
    },
    {
      "operation": "removeReverse",
      "serial": "AVATR-CLUSTER-01",
      "remotePort": 49321,
      "result": {}
    }
  ]
}
```

Create `crates/lyra-device/tests/reverse_coordinator.rs`. Its success test must establish a mapping from the fixture, assert serial/local/remote values, call `mapping.remove`, then call `FakeAdb::assert_finished`. Add a table-driven selection test with list-only transcripts for `[]`, offline/unauthorized-only, and two `device` transports; assert the two stable selection codes and that the transcript is exhausted. Add transcripts that make `reverse` and `removeReverse` return configured `device.adb.*Failed` diagnostics; assert that the coordinator preserves each code and never issues an unconfigured operation.

Use this typed helper shape:

```rust
fn request(port: u16) -> DevBridgeReverseRequest {
    DevBridgeReverseRequest::new(LocalPort::new(port).expect("valid port"))
}

fn fake(source: &str) -> FakeAdb {
    FakeAdb::from_slice(source.as_bytes()).expect("valid transcript")
}
```

- [ ] **Step 2: Run the focused test to verify the public contract is absent.**

Run: `cargo test -p lyra-device --test reverse_coordinator`

Expected: compilation fails because `DevBridgeReverseCoordinator`, `DevBridgeReverseRequest`, `ReverseMapping` and `DEV_BRIDGE_REMOTE_PORT` are not yet exported.

- [ ] **Step 3: Add the minimal typed coordinator.**

In `adb.rs`, define the fixed port inside the module that owns the private `RemotePort` field:

```rust
pub const DEV_BRIDGE_REMOTE_PORT: RemotePort = RemotePort(49_321);
```

In `reverse.rs`, implement the public types above. `establish` must call `list_devices` once, retain only `AdbDeviceState::Device` serials, and branch as follows:

```rust
let Some(serial) = eligible.next() else {
    return Err(DeviceDiagnostic::new(
        "device.adb.noEligibleDevice",
        "exactly one ready ADB device is required",
    ));
};
if eligible.next().is_some() {
    return Err(DeviceDiagnostic::new(
        "device.adb.multipleEligibleDevices",
        "exactly one ready ADB device is required",
    ));
}
adb.reverse(&serial, request.local_port(), request.remote_port())?;
```

Build `ReverseMapping` only after the adapter returns success. Its `remove` method borrows `self`, calls `remove_reverse(&serial, remote_port)`, and does not rewrite a failure, so callers can retry cleanup. Re-export all four public items from `lib.rs`.

- [ ] **Step 4: Run focused checks and the full portable crate.**

Run:

```bash
cargo fmt --check
cargo test -p lyra-device --test reverse_coordinator
cargo test -p lyra-device
cargo clippy -p lyra-device --all-targets -- -D warnings
```

Expected: every command exits 0; tests consume every `FakeAdb` transcript.

- [ ] **Step 5: Commit the verified implementation.**

```bash
git add crates/lyra-device/src/adb.rs crates/lyra-device/src/reverse.rs crates/lyra-device/src/lib.rs crates/lyra-device/tests/reverse_coordinator.rs Fixtures/Device/adb-reverse-single-device.json
git commit -m "feat(device): coordinate Dev Bridge reverse"
```

### Task 2: Publish the completed portable boundary

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture/rust-tauri.md`
- Modify: `docs/design/fake-first-device-core.md`
- Modify: `docs/design/loopback-dev-server.md`
- Modify: `docs/design/studio-device-bridge.md`
- Modify: `docs/protocols/dev-bridge-v1.md`
- Modify: `docs/plans/m3-device-adb-reverse.md`

**Interfaces:**
- Consumes: Task 1's public coordinator and stable selection diagnostics.
- Produces: accurate scope statements that distinguish fake-first reverse policy from a real `adb` process or Android runtime integration.

- [ ] **Step 1: Update the user-facing and architecture documentation.**

State that Studio still has only three lifecycle commands, while the portable domain now owns a tested coordinator. Document `49321` as the fixed Android-side Dev Bridge port, list both selection diagnostics in the v1 table, and state that a future Tauri integration must derive the local port from the private loopback endpoint and inject a real `AdbClient` separately.

- [ ] **Step 2: Mark the completed Task 1 checkboxes in this plan.**

Replace only Task 1's five `- [ ]` markers with `- [x]` after its commands and commit have succeeded. Keep Task 2 and Task 3 unchecked until their own work is complete.

- [ ] **Step 3: Verify documentation consistency.**

Run:

```bash
rg -n "TODO|TBD|FIXME|XXX" README.md docs/design docs/protocols docs/architecture
git diff --check
```

Expected: no placeholder is introduced by this slice and the diff has no whitespace errors.

- [ ] **Step 4: Commit the documentation.**

```bash
git add README.md docs/architecture/rust-tauri.md docs/design/fake-first-device-core.md docs/design/loopback-dev-server.md docs/design/studio-device-bridge.md docs/protocols/dev-bridge-v1.md docs/plans/m3-device-adb-reverse.md
git commit -m "docs(device): publish reverse coordinator boundary"
```

### Task 3: Verify, publish and merge the slice

**Files:**
- Verify: all Task 1 and Task 2 files

**Interfaces:**
- Consumes: no real process adapter, no Android source, and no new frontend command.
- Produces: a reviewable GitHub pull request with cross-platform CI evidence.

- [ ] **Step 1: Run the full local release gate.**

Run:

```bash
npm run studio:lint
npm run studio:test
npm run studio:build
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
npx tauri build --debug --no-bundle
git diff --check
```

Expected: all commands exit 0. The Tauri build validates the existing shell without adding an ADB runtime adapter.

- [ ] **Step 2: Mark Task 2 and Task 3 verification checkboxes only after evidence exists.**

Mark the relevant completed checkboxes in this plan, then run `git diff --check` again before committing the plan status.

- [ ] **Step 3: Push, create a pull request and merge after every CI job passes.**

Run:

```bash
git push -u origin feature/device-adb-reverse
gh pr create --base main --head feature/device-adb-reverse --title "feat: coordinate Dev Bridge ADB reverse"
gh pr checks <pr-number> --watch
gh pr merge <pr-number> --squash --delete-branch
```

Expected: Linux, Windows and macOS release jobs pass before merge. After the merge, fast-forward `main`, rerun `npm run studio:test` and `cargo test -p lyra-device --test reverse_coordinator`, and verify a clean working tree.
