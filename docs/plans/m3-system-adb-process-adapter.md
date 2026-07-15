# M3 System ADB Process Adapter Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a cross-platform, fixed-argv `SystemAdb` adapter that implements `lyra-device`'s typed `AdbClient` without invoking a real binary in tests.

**Architecture:** A new `lyra-adb` crate owns the process boundary and depends only on `lyra-device` plus the standard library. A public `SystemAdb` explicitly accepts an executable path, while a crate-private generic adapter accepts an in-memory executor in unit tests. `lyra-device`, Tauri, Studio and Android remain unchanged.

**Tech Stack:** Rust 2024, `std::process::Command`, `std::ffi::OsString`, existing `lyra-device` types, Cargo tests and GitHub Actions.

## Global Constraints

- Add no shell invocation, string command line, environment target selection or automatic Android SDK/ADB discovery.
- `SystemAdb::from_path` only stores an explicitly provided executable path; it must not run a process.
- The only generated ADB operations are `devices -l`, fixed `-s` reverse, fixed `-s` reverse removal and fixed `-s` push.
- Every argument is a separate `OsString` derived from an existing validated typed value.
- ADB stdout is parsed only after the exact `List of devices attached` header.
- Stable diagnostics are `device.adb.launchFailed`, `device.adb.commandFailed`, `device.adb.invalidDeviceList` and `device.adb.unsupportedDeviceState`.
- Tests must use an in-memory executor; no test may execute `adb`, inspect an SDK or require Android hardware.
- Preserve `lyra-device` as process-free and do not add Tauri/UI/Android code.
- Each independently verifiable behavior ends in a Conventional Commit.

## File structure

| File | Responsibility |
|---|---|
| `Cargo.toml` | Adds the new portable process-adapter crate to the workspace. |
| `crates/lyra-adb/Cargo.toml` | Declares only the `lyra-device` dependency and workspace metadata. |
| `crates/lyra-adb/src/lib.rs` | Exports the explicit `SystemAdb` public entry point. |
| `crates/lyra-adb/src/adapter.rs` | Builds fixed argv, parses `devices -l` output, maps process failures and contains private fake-executor tests. |
| `.github/workflows/ci.yml` | Adds `lyra-adb` to Linux/Windows portable testing. |
| `README.md` and device documents | Describe the explicit process boundary and stable diagnostics. |

---

### Task 1: Create and prove the fixed-argv adapter

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/lyra-adb/Cargo.toml`
- Create: `crates/lyra-adb/src/lib.rs`
- Create: `crates/lyra-adb/src/adapter.rs`

**Interfaces:**
- Consumes: `lyra_device::{AdbClient, AdbDevice, AdbDeviceState, DeviceDiagnostic, DevicePath, DeviceSerial, LocalPort, RemotePort}`.
- Produces:

```rust
pub struct SystemAdb;

impl SystemAdb {
    #[must_use]
    pub fn from_path(executable: impl Into<PathBuf>) -> Self;
}

impl AdbClient for SystemAdb {
    fn list_devices(&mut self) -> Result<Vec<AdbDevice>, DeviceDiagnostic>;
    fn reverse(
        &mut self,
        serial: &DeviceSerial,
        local: LocalPort,
        remote: RemotePort,
    ) -> Result<(), DeviceDiagnostic>;
    fn remove_reverse(
        &mut self,
        serial: &DeviceSerial,
        remote: RemotePort,
    ) -> Result<(), DeviceDiagnostic>;
    fn push(
        &mut self,
        serial: &DeviceSerial,
        local: &Path,
        destination: &DevicePath,
    ) -> Result<(), DeviceDiagnostic>;
}
```

- [x] **Step 1: Add the package skeleton and write failing private-executor tests.**

Add `crates/lyra-adb` to the workspace and create a manifest that depends only on `lyra-device = { path = "../lyra-device" }`. In `lib.rs`, declare `mod adapter;` and re-export `SystemAdb`.

In `adapter.rs`, add a `#[cfg(test)]` module that defines a queue-backed fake executor and writes these four tests before any adapter implementation:

1. `list_devices_uses_devices_long_and_parses_supported_states` expects exactly `["devices", "-l"]`, supplies a daemon prelude plus header and `device`/`offline`/`unauthorized` rows, then asserts all three typed states.
2. `mutations_use_fixed_separate_arguments` calls `reverse`, `remove_reverse` and `push` through `AdbClient` and asserts exactly:

```text
-s AVATR-01 reverse tcp:49321 tcp:42137
-s AVATR-01 reverse --remove tcp:49321
-s AVATR-01 push /tmp/revision.zip /data/local/tmp/lyra/revision.zip
```

3. `normalizes_launch_and_unsuccessful_command_failures` configures an I/O launch error and a non-success command result, then asserts `device.adb.launchFailed` and `device.adb.commandFailed` without checking raw stderr text.
4. `rejects_invalid_or_unsupported_device_lists` covers missing header, invalid UTF-8, one-column row and an unknown state, asserting `device.adb.invalidDeviceList` or `device.adb.unsupportedDeviceState`.

Each fake-executor test calls `assert_finished` so any unexpected command fails.

- [x] **Step 2: Run the crate test to verify the adapter symbols are absent.**

Run: `cargo test -p lyra-adb`

Expected: compilation fails because `SystemAdb`, the internal adapter and its command-executor path do not exist yet.

- [x] **Step 3: Implement the smallest safe process boundary.**

Implement a crate-private generic `Adapter<E>` that stores an executable `PathBuf` and an executor. The private executor calls:

```rust
Command::new(executable)
    .args(arguments)
    .output()
```

and returns only success state plus stdout. Convert spawn errors into:

```rust
DeviceDiagnostic::new("device.adb.launchFailed", "could not launch adb")
```

Convert any non-success exit into:

```rust
DeviceDiagnostic::new("device.adb.commandFailed", format!("adb {operation} failed"))
```

Build each typed method's exact `OsString` vector before handing it to the executor. Parse rows only after the exact header, skip blank rows, map accepted states to `AdbDeviceState`, reject invalid serials as `device.adb.invalidDeviceList`, and reject every unknown state as `device.adb.unsupportedDeviceState`. Wrap `Adapter<SystemExecutor>` in the public `SystemAdb` without adding automatic discovery or invocation during `from_path`.

- [x] **Step 4: Verify focused behavior and the new package.**

Run:

```bash
cargo fmt --check
cargo test -p lyra-adb
cargo clippy -p lyra-adb --all-targets -- -D warnings
```

Expected: all test paths use only the fake executor and every command exits 0.

- [x] **Step 5: Commit the verified process adapter.**

```bash
git add Cargo.toml crates/lyra-adb
git commit -m "feat(adb): add fixed process adapter"
```

### Task 2: Publish the process boundary and CI gate

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`
- Modify: `docs/architecture/rust-tauri.md`
- Modify: `docs/design/fake-first-device-core.md`
- Modify: `docs/design/device-adb-reverse-coordinator.md`
- Modify: `docs/design/loopback-dev-server.md`
- Modify: `docs/design/studio-device-bridge.md`
- Modify: `docs/protocols/dev-bridge-v1.md`
- Modify: `docs/plans/m3-system-adb-process-adapter.md`

**Interfaces:**
- Consumes: Task 1's explicit `SystemAdb` and its stable diagnostics.
- Produces: a cross-platform test target and documentation that never claims Studio invokes ADB today.

- [x] **Step 1: Add the package to the portable CI matrix.**

Extend the existing Linux/Windows `cargo test -p ...` command with `-p lyra-adb`. Keep the macOS whole-workspace release gate unchanged.

- [x] **Step 2: Update the public boundary documentation.**

State that `lyra-adb` is explicit-path, fixed-argv and no-shell; it is fake-executor tested but is not yet wired into Tauri/Studio/Android. Add the four stable process/list diagnostics to Dev Bridge v1 and describe the next integration as an explicit user action, not automatic discovery.

- [x] **Step 3: Mark Task 1 steps complete and verify document consistency.**

After Task 1 and documentation updates succeed, mark Task 1's five checkboxes `[x]`. Run:

```bash
rg -n "TODO|TBD|FIXME|XXX" README.md docs/design docs/protocols docs/architecture
git diff --check
```

Expected: no new placeholder or whitespace failure.

- [x] **Step 4: Commit the CI and documentation changes.**

```bash
git add .github/workflows/ci.yml README.md docs
git commit -m "docs(adb): publish process adapter boundary"
```

### Task 3: Run the release gate, publish and merge

**Files:**
- Verify: Task 1 and Task 2 files

**Interfaces:**
- Consumes: an explicit adapter that is not constructed by Tauri and never runs in tests.
- Produces: a GitHub pull request with Linux, Windows and macOS CI evidence.

- [x] **Step 1: Run the full local release gate.**

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

Expected: all commands exit 0 without invoking a real ADB binary.

- [x] **Step 2: Record verification evidence in this plan.**

Mark Task 2 and Task 3 verification checkboxes `[x]` only after their commands have passed, then run `git diff --check` before committing the plan status.

Verified locally on 2026-07-15: Studio lint, 25 Studio tests, Studio production build, Rust formatting, workspace Clippy, workspace tests, workspace release build and the no-bundle Tauri debug build all exited successfully. The Tauri CLI was installed with the README-mandated root `npm ci`; all adapter tests used the in-memory executor and no real ADB binary ran.

- [ ] **Step 3: Push, open a pull request and squash-merge after CI.**

```bash
git push -u origin feature/adb-process-adapter
gh pr create --base main --head feature/adb-process-adapter --title "feat: add fixed ADB process adapter"
gh pr checks <pr-number> --watch
gh pr merge <pr-number> --squash --delete-branch
```

Expected: Linux, Windows and macOS jobs pass before merging. Fast-forward `main` after merge, rerun `cargo test -p lyra-adb` and `npm run studio:test`, then verify a clean working tree.
