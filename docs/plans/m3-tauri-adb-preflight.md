# M3 User-Gated Tauri ADB Preflight Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a Studio user select one ADB executable through a native Rust dialog and explicitly check device readiness without automatic discovery, reverse mapping or Android changes.

**Architecture:** `DeviceBridgeController` gains a private, in-memory ADB configuration plus an injected `AdbDeviceProbe`. The production probe creates `lyra_adb::SystemAdb` only inside a blocking explicit-check task; unit tests inject a queue-backed fake probe. Tauri exposes three no-argument commands, and the React backend/UI render only a small safe readiness projection.

**Tech Stack:** Rust 1.97, Tauri 2, Tokio 1, `tauri-plugin-dialog` 2.7, `lyra-adb`, `lyra-device`, React 19, TypeScript 6 and Vitest.

## Global Constraints

- Do not discover an Android SDK, read `ANDROID_SERIAL`, accept a raw executable path, serial, port, command string or shell fragment from the renderer.
- ADB is launched only when the user invokes the explicit **Check devices** UI action; application startup, bridge start/stop, status reads, native-picker cancellation and tests must not launch it.
- The production check may issue only `SystemAdb::list_devices`, which is the existing fixed `devices -l` argv; it must not reverse, remove-reverse or push.
- The executable path must come from a Rust-owned native file picker, canonicalize successfully and resolve to a regular file. Retain it only in memory and never serialize it.
- Public ADB preflight status contains exactly `configured` and `readiness`; it must not contain a path, serial, raw stdout/stderr, listener endpoint, URL, port, session ID or bearer.
- Use the stable diagnostics `device.adb.notConfigured`, `device.adb.invalidExecutable` and `device.adb.probeFailed` for controller-owned failures; preserve existing `SystemAdb` diagnostics unchanged.
- A stale async probe result must not overwrite the status after a newly selected executable changes the configuration generation.
- Preserve the current loopback Dev Bridge behavior. This slice creates no reverse mapping, Pack transfer, revision command, Android change, persistence or automatic retry.
- Every independently verifiable behavior ends in a Conventional Commit. Tests use only fake probes and never execute an ADB binary.

## File structure

| File | Responsibility |
|---|---|
| `src-tauri/Cargo.toml` | Adds the existing `lyra-adb` process adapter to the desktop shell only. |
| `src-tauri/src/device_bridge.rs` | Owns private executable state, safe preflight projection, injected probe and controller tests. |
| `src-tauri/src/lib.rs` | Owns native dialog plumbing and the three narrow Tauri command wrappers. |
| `apps/studio/src/lib/backend.ts` | Declares the typed preflight contract, exact invoke names and browser fixture behavior. |
| `apps/studio/src/lib/backend.test.ts` | Proves the renderer invokes only the narrow no-argument commands. |
| `apps/studio/src/App.tsx` | Renders Select ADB and Check devices controls with separate loading/error state. |
| `apps/studio/src/App.css` | Adds compact readiness styles without changing editor layout. |
| `apps/studio/src/App.test.tsx` | Proves fixture configuration/check interactions and secret-free copy. |
| `README.md` and `docs/` | Publish the new user-gated preflight boundary and diagnostics. |

---

### Task 1: Add the fake-first desktop preflight controller

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Modify: `src-tauri/src/device_bridge.rs`
- Modify: `Cargo.lock`

**Interfaces:**
- Consumes: `lyra_adb::SystemAdb`, `lyra_device::{AdbClient, AdbDevice, AdbDeviceState, DeviceDiagnostic}` and the existing `DeviceBridgeController`.
- Produces: `AdbPreflightReadiness`, `AdbPreflightStatus`, private `AdbDeviceProbe`, `DeviceBridgeController::{adb_status,configure_adb_executable,check_adb}`.
- Invariants: an unconfigured check does not call the probe; a configuration call does not call the probe; only `Device` transports count as ready.

- [ ] **Step 1: Write failing controller tests before adding preflight types.**

  In the existing `#[cfg(test)]` module in `src-tauri/src/device_bridge.rs`, add a queue-backed `FakeAdbProbe` that records each path and returns queued `Result<Vec<AdbDevice>, DeviceDiagnostic>` values. Add these tests using `tempfile::NamedTempFile` for real regular-file paths:

  ```rust
  #[tokio::test]
  async fn adb_preflight_starts_unconfigured_and_never_probes_without_selection() {
      let probe = Arc::new(FakeAdbProbe::default());
      let controller = DeviceBridgeController::with_probe(Arc::clone(&probe));

      assert_eq!(controller.adb_status().await, AdbPreflightStatus::unconfigured());
      let error = controller.check_adb().await.unwrap_err();
      assert_eq!(error.code, "device.adb.notConfigured");
      probe.assert_no_calls();
  }

  #[tokio::test]
  async fn configuring_a_regular_file_resets_to_not_checked_without_probing() {
      let executable = tempfile::NamedTempFile::new().unwrap();
      let probe = Arc::new(FakeAdbProbe::default());
      let controller = DeviceBridgeController::with_probe(Arc::clone(&probe));

      let status = controller
          .configure_adb_executable(executable.path().to_path_buf())
          .await
          .unwrap();

      assert_eq!(status, AdbPreflightStatus::not_checked());
      assert_eq!(serde_json::to_value(status).unwrap(), serde_json::json!({
          "configured": true,
          "readiness": "notChecked"
      }));
      probe.assert_no_calls();
  }

  #[tokio::test]
  async fn explicit_check_maps_only_ready_device_counts() {
      let executable = tempfile::NamedTempFile::new().unwrap();
      let probe = Arc::new(FakeAdbProbe::from_results([
          Ok(vec![]),
          Ok(vec![device("AVATR-01", AdbDeviceState::Device)]),
          Ok(vec![
              device("AVATR-01", AdbDeviceState::Device),
              device("AVATR-02", AdbDeviceState::Device),
              device("OFFLINE", AdbDeviceState::Offline),
          ]),
      ]));
      let controller = DeviceBridgeController::with_probe(Arc::clone(&probe));
      controller.configure_adb_executable(executable.path().to_path_buf()).await.unwrap();

      assert_eq!(controller.check_adb().await.unwrap().readiness, AdbPreflightReadiness::NoReadyDevice);
      assert_eq!(controller.check_adb().await.unwrap().readiness, AdbPreflightReadiness::OneReadyDevice);
      assert_eq!(controller.check_adb().await.unwrap().readiness, AdbPreflightReadiness::MultipleReadyDevices);
      probe.assert_finished();
  }

  #[tokio::test]
  async fn probe_error_is_stable_and_never_serializes_process_output() {
      let executable = tempfile::NamedTempFile::new().unwrap();
      let probe = Arc::new(FakeAdbProbe::from_results([Err(DeviceDiagnostic::new(
          "device.adb.commandFailed",
          "adb devices failed",
      ))]));
      let controller = DeviceBridgeController::with_probe(Arc::clone(&probe));
      controller.configure_adb_executable(executable.path().to_path_buf()).await.unwrap();

      let error = controller.check_adb().await.unwrap_err();
      assert_eq!(error.code, "device.adb.commandFailed");
      assert_eq!(controller.adb_status().await.readiness, AdbPreflightReadiness::Error);
      assert!(serde_json::to_string(&controller.adb_status().await).unwrap().contains("error"));
      probe.assert_finished();
  }
  ```

  Add one `BlockingProbe` test with a channel: begin `check_adb`, wait until the fake probe starts, configure a second `NamedTempFile`, release the fake and assert the returned plus stored status remain `notChecked`. Also add a non-file path test that returns `device.adb.invalidExecutable` and never calls the fake.

- [ ] **Step 2: Run the focused tests to verify the red state.**

  Run:

  ```sh
  cargo test -p lyra-effects-studio-app device_bridge::tests::adb_preflight --lib
  ```

  Expected: compilation fails because `AdbPreflightStatus`, probe injection and controller methods do not exist.

- [ ] **Step 3: Add the desktop-only adapter dependency and minimal production probe.**

  Add this direct dependency to `src-tauri/Cargo.toml`:

  ```toml
  lyra-adb = { path = "../crates/lyra-adb" }
  ```

  At the top of `device_bridge.rs`, add the exact public-safe types and private probe boundary:

  ```rust
  #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub(crate) enum AdbPreflightReadiness {
      Unconfigured,
      NotChecked,
      NoReadyDevice,
      OneReadyDevice,
      MultipleReadyDevices,
      Error,
  }

  #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub(crate) struct AdbPreflightStatus {
      configured: bool,
      readiness: AdbPreflightReadiness,
  }

  trait AdbDeviceProbe: Send + Sync {
      fn list_devices(&self, executable: &Path) -> Result<Vec<AdbDevice>, DeviceDiagnostic>;
  }

  struct SystemAdbDeviceProbe;

  impl AdbDeviceProbe for SystemAdbDeviceProbe {
      fn list_devices(&self, executable: &Path) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
          let mut adb = SystemAdb::from_path(executable);
          adb.list_devices()
      }
  }
  ```

  Give `AdbPreflightStatus` private `unconfigured`, `not_checked` and `error`
  constructors. Add an `AdbPreflightState` containing `executable: Option<PathBuf>`,
  `generation: u64` and `status: AdbPreflightStatus`; store it in a separate
  `tokio::sync::Mutex` on `DeviceBridgeController` with an `Arc<dyn AdbDeviceProbe>`.
  Add a private `from_probe(Arc<dyn AdbDeviceProbe>) -> Self` initializer;
  `new()` passes `Arc::new(SystemAdbDeviceProbe)` to it, while the
  `#[cfg(test)]` `with_probe` wrapper delegates to it. This keeps production
  construction fixed while giving tests a fake-only injection seam.

  `configure_adb_executable` must canonicalize the supplied path, reject a
  non-file or canonicalization failure as:

  ```rust
  DeviceDiagnostic::new(
      "device.adb.invalidExecutable",
      "ADB executable must resolve to a regular file",
  )
  ```

  On success it increments `generation`, retains the canonical path and sets
  `not_checked`. `check_adb` returns:

  ```rust
  DeviceDiagnostic::new(
      "device.adb.notConfigured",
      "select an ADB executable before checking devices",
  )
  ```

  without invoking a probe when no path exists. Otherwise clone the path,
  generation and probe, call `probe.list_devices` inside
  `tokio::task::spawn_blocking`, and map a join failure to
  `device.adb.probeFailed` / `ADB preflight worker failed`. Count only
  `AdbDeviceState::Device`; update to one of the three ready-count states. On
  an adapter error first store `error`, then return that unchanged diagnostic.
  Re-lock after the worker completes; if `generation` differs, discard the
  stale result and return the current stored projection.

- [ ] **Step 4: Run focused Rust verification and inspect the process boundary.**

  Run:

  ```sh
  cargo fmt --check
  cargo test -p lyra-effects-studio-app device_bridge::tests::adb_preflight --lib
  cargo clippy -p lyra-effects-studio-app --all-targets -- -D warnings
  rg -n "Command::new|\.args\(|shell|ANDROID_SERIAL" src-tauri/src/device_bridge.rs crates/lyra-adb --glob '*.rs'
  ```

  Expected: all focused tests pass; the search finds process creation only in
  `crates/lyra-adb/src/adapter.rs`, and no controller test starts a binary.

- [ ] **Step 5: Commit the testable controller boundary.**

  ```sh
  git add src-tauri/Cargo.toml src-tauri/src/device_bridge.rs Cargo.lock
  git commit -m "feat(adb): add user-gated preflight controller"
  ```

### Task 2: Expose only native chooser and explicit check commands

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `DeviceBridgeController::{adb_status,configure_adb_executable,check_adb}` from Task 1 and the already initialized `tauri-plugin-dialog`.
- Produces: `get_device_bridge_adb_status`, `choose_device_bridge_adb_executable` and `check_device_bridge_adb` Tauri commands.
- Invariants: no command parameter carries a path; cancellation returns the existing status; no dialog callback exposes its file path to the renderer.

- [ ] **Step 1: Write the failing command-surface assertion.**

  Extend the `src-tauri/src/lib.rs` test module with this compile-time symbol
  check. It requires the three async command functions to exist without
  instantiating a Tauri application or opening a native dialog:

  ```rust
  #[test]
  fn adb_preflight_commands_are_available() {
      let _ = get_device_bridge_adb_status;
      let _ = choose_device_bridge_adb_executable;
      let _ = check_device_bridge_adb;
  }
  ```

- [ ] **Step 2: Run the library test to verify the commands are absent.**

  Run:

  ```sh
  cargo test -p lyra-effects-studio-app adb_preflight_commands --lib
  ```

  Expected: compilation failure because the three new functions are absent.

- [ ] **Step 3: Implement native picker plumbing and command wrappers.**

  Import `tauri_plugin_dialog::DialogExt` plus `tokio::sync::oneshot`. Add a
  private helper that starts the non-blocking native dialog and awaits exactly
  one callback result:

  ```rust
  async fn choose_adb_executable(app: &tauri::AppHandle) -> Result<Option<PathBuf>, String> {
      let (sender, receiver) = oneshot::channel();
      app.dialog()
          .file()
          .set_title("Choose Android Debug Bridge executable")
          .pick_file(move |selection| {
              let result = selection
                  .map(|file| file.into_path().map_err(|_| "device.adb.invalidExecutable: selected file is not a local path".to_owned()))
                  .transpose();
              let _ = sender.send(result);
          });
      receiver
          .await
          .map_err(|_| "device.adb.invalidExecutable: ADB file picker closed unexpectedly".to_owned())?
  }
  ```

  Register these wrappers in `tauri::generate_handler!` next to the existing
  bridge lifecycle commands:

  ```rust
  #[tauri::command]
  async fn get_device_bridge_adb_status(
      controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
  ) -> Result<device_bridge::AdbPreflightStatus, String> {
      Ok(controller.adb_status().await)
  }

  #[tauri::command]
  async fn choose_device_bridge_adb_executable(
      app: tauri::AppHandle,
      controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
  ) -> Result<device_bridge::AdbPreflightStatus, String> {
      let Some(path) = choose_adb_executable(&app).await? else {
          return Ok(controller.adb_status().await);
      };
      controller.configure_adb_executable(path).await.map_err(|error| error.to_string())
  }

  #[tauri::command]
  async fn check_device_bridge_adb(
      controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
  ) -> Result<device_bridge::AdbPreflightStatus, String> {
      controller.check_adb().await.map_err(|error| error.to_string())
  }
  ```

  The chooser must be the only source of a production executable path. Do not
  add an argument to any wrapper, and do not log or serialize a selected path.

- [ ] **Step 4: Verify the desktop command layer without opening a dialog.**

  Run:

  ```sh
  cargo fmt --check
  cargo test -p lyra-effects-studio-app --lib
  cargo clippy -p lyra-effects-studio-app --all-targets -- -D warnings
  ```

  Expected: the application library compiles, the command type test passes and
  no test calls the native picker or starts ADB.

- [ ] **Step 5: Commit the narrow Tauri surface.**

  ```sh
  git add src-tauri/src/lib.rs
  git commit -m "feat(adb): expose explicit Tauri preflight actions"
  ```

### Task 3: Extend the typed Studio facade and browser fixture

**Files:**
- Modify: `apps/studio/src/lib/backend.ts`
- Modify: `apps/studio/src/lib/backend.test.ts`

**Interfaces:**
- Consumes: Task 2's exact snake-case Tauri command names.
- Produces: `AdbPreflightReadiness`, `AdbPreflightStatus` and three `StudioBackend` methods.
- Browser fixture behavior: `unconfigured → notChecked → oneReadyDevice`; it does not contain a path, serial or raw process output.

- [ ] **Step 1: Write failing typed-facade tests.**

  Add types to the test import and append this exact interaction test:

  ```ts
  it("uses no-argument commands for the user-gated ADB preflight", async () => {
    const unconfigured: AdbPreflightStatus = {
      configured: false,
      readiness: "unconfigured",
    };
    const invoke = vi.fn(async () => unconfigured);
    const backend = createBackend(invoke);

    await expect(backend.deviceBridgeAdbStatus()).resolves.toEqual(unconfigured);
    await expect(backend.chooseDeviceBridgeAdbExecutable()).resolves.toEqual(unconfigured);
    await expect(backend.checkDeviceBridgeAdb()).resolves.toEqual(unconfigured);
    expect(invoke).toHaveBeenNthCalledWith(1, "get_device_bridge_adb_status");
    expect(invoke).toHaveBeenNthCalledWith(2, "choose_device_bridge_adb_executable");
    expect(invoke).toHaveBeenNthCalledWith(3, "check_device_bridge_adb");
    expect(invoke).not.toHaveBeenCalledWith(expect.any(String), expect.anything());
  });
  ```

- [ ] **Step 2: Run the focused facade test to verify the red state.**

  Run:

  ```sh
  npm --prefix apps/studio run test -- backend.test.ts
  ```

  Expected: TypeScript or runtime failure because the preflight status and
  methods do not exist.

- [ ] **Step 3: Implement the safe TypeScript contract and fixture.**

  Add these exact public types after `DeviceBridgeStatus`:

  ```ts
  export type AdbPreflightReadiness =
    | "unconfigured"
    | "notChecked"
    | "noReadyDevice"
    | "oneReadyDevice"
    | "multipleReadyDevices"
    | "error";

  export interface AdbPreflightStatus {
    configured: boolean;
    readiness: AdbPreflightReadiness;
  }
  ```

  Add `deviceBridgeAdbStatus`, `chooseDeviceBridgeAdbExecutable` and
  `checkDeviceBridgeAdb` to `StudioBackend` and map them to the exact command
  strings from Step 1 with no argument object. In `createFixtureBackend`, keep
  a closure-local status initialized to `{ configured: false, readiness:
  "unconfigured" }`; chooser changes it to `{ configured: true, readiness:
  "notChecked" }`, and check changes it to `{ configured: true, readiness:
  "oneReadyDevice" }`. Return `structuredClone` from every fixture read or
  transition.

- [ ] **Step 4: Run focused frontend verification.**

  Run:

  ```sh
  npm --prefix apps/studio run lint
  npm --prefix apps/studio run test -- backend.test.ts
  ```

  Expected: the façade proves all three invocations have no renderer-supplied
  arguments, and lint finds no unsafe type escape.

- [ ] **Step 5: Commit the renderer contract.**

  ```sh
  git add apps/studio/src/lib/backend.ts apps/studio/src/lib/backend.test.ts
  git commit -m "feat(studio): add typed ADB preflight facade"
  ```

### Task 4: Render the explicit preflight controls in Studio

**Files:**
- Modify: `apps/studio/src/App.tsx`
- Modify: `apps/studio/src/App.css`
- Modify: `apps/studio/src/App.test.tsx`

**Interfaces:**
- Consumes: Task 3's `backend.deviceBridgeAdbStatus`, `.chooseDeviceBridgeAdbExecutable`, `.checkDeviceBridgeAdb` and `AdbPreflightStatus`.
- Produces: `data-testid="device-adb-control"`, `device-adb-select`, `device-adb-check` and safe visible readiness labels.
- Copy: `ADB not configured`, `ADB selected`, `No ready device`, `1 device ready`, `Multiple devices`, `ADB check failed`, `Select ADB`, `Check devices`.

- [ ] **Step 1: Write the failing browser-fixture interaction test.**

  Add this test to `apps/studio/src/App.test.tsx`:

  ```tsx
  it("requires explicit ADB selection before checking a device", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    const control = await screen.findByTestId("device-adb-control");
    const check = screen.getByTestId("device-adb-check");
    expect(control).toHaveTextContent("ADB not configured");
    expect(check).toBeDisabled();
    expect(control).not.toHaveTextContent("/Users/");
    expect(control).not.toHaveTextContent("Bearer");

    await user.click(screen.getByTestId("device-adb-select"));
    expect(await screen.findByText("ADB selected")).toBeInTheDocument();
    expect(check).toBeEnabled();

    await user.click(check);
    expect(await screen.findByText("1 device ready")).toBeInTheDocument();
  });
  ```

- [ ] **Step 2: Run the focused UI test to verify the red state.**

  Run:

  ```sh
  npm --prefix apps/studio run test -- App.test.tsx
  ```

  Expected: failure because the ADB control and methods are absent.

- [ ] **Step 3: Add independent state, labels and event handlers.**

  In `App.tsx`, define an `UNCONFIGURED_ADB_PREFLIGHT` constant and a pure
  `adbPreflightLabel(status, loading, failed)` helper that maps the six
  readiness values to the Task 4 copy. Add separate `adbPreflight`,
  `adbPreflightLoading`, `adbPreflightBusy` and `adbPreflightError` state;
  refresh it once on mount through `backend.deviceBridgeAdbStatus()` without
  coupling it to the Dev Bridge lifecycle request.

  `chooseDeviceBridgeAdb` must set busy, call only
  `backend.chooseDeviceBridgeAdbExecutable()`, replace the stored status on
  success and preserve it on failure. `checkDeviceBridgeAdb` follows the same
  pattern with `backend.checkDeviceBridgeAdb()`. Disable Check devices while
  loading/busy/error or when `configured` is false. Render a compact sibling
  `<section>` inside the existing `.publish-block`:

  ```tsx
  <section className="device-adb-control" data-testid="device-adb-control" aria-label="ADB device check">
    <span className={`device-adb-status ${adbPhase}`} role="status" aria-live="polite">
      <i />
      <span>{adbLabel}</span>
    </span>
    <button className="quiet-button" data-testid="device-adb-select" disabled={adbPreflightBusy} onClick={() => void chooseDeviceBridgeAdb()}>
      Select ADB
    </button>
    <button className="quiet-button" data-testid="device-adb-check" disabled={adbPreflightLoading || adbPreflightBusy || adbPreflightError || !adbPreflight.configured} onClick={() => void checkDeviceBridgeAdb()}>
      Check devices
    </button>
  </section>
  ```

  On failure, show only `ADB check failed`; do not render the backend error
  string, a path or a serial. A subsequent explicit Select ADB action clears
  the error when it returns a valid status.

- [ ] **Step 4: Add compact, responsive styles.**

  In `App.css`, extend the existing flex selector list with
  `.device-adb-control` and `.device-adb-status`. Reuse neutral/amber/cyan/orange
  dots from the bridge control: unconfigured/notChecked neutral, no-ready or
  multiple amber, one-ready cyan and error orange. At the existing `1240px`
  breakpoint hide only the ADB readiness text while retaining both explicit
  buttons and their accessible labels. Do not alter the three-column workspace
  grid, preview dimensions or Build pack button.

- [ ] **Step 5: Run focused Studio verification.**

  Run:

  ```sh
  npm --prefix apps/studio run lint
  npm --prefix apps/studio run test -- App.test.tsx
  npm --prefix apps/studio run build
  ```

  Expected: fixture flow remains explicit, Check devices starts disabled and
  the production frontend compiles without a path/serial/token field.

- [ ] **Step 6: Commit the visible preflight control.**

  ```sh
  git add apps/studio/src/App.tsx apps/studio/src/App.css apps/studio/src/App.test.tsx
  git commit -m "feat(studio): add explicit ADB preflight controls"
  ```

### Task 5: Publish the boundary, verify and merge

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture/rust-tauri.md`
- Modify: `docs/design/fake-first-device-core.md`
- Modify: `docs/design/loopback-dev-server.md`
- Modify: `docs/design/studio-device-bridge.md`
- Modify: `docs/design/system-adb-process-adapter.md`
- Modify: `docs/protocols/dev-bridge-v1.md`
- Modify: `docs/plans/m3-system-adb-process-adapter.md`
- Modify: `docs/plans/m3-tauri-adb-preflight.md`

**Interfaces:**
- Consumes: completed controller, Tauri commands and Studio control from Tasks 1–4.
- Produces: accurate public documentation stating that real `devices -l` is user-triggered and that reverse mapping remains deferred.

- [ ] **Step 1: Update scope and stable diagnostics.**

  Update README and architecture/design documents to say that Studio now has an
  in-memory, native-chooser ADB preflight that runs `devices -l` only after an
  explicit user action. Keep the following non-goals visible everywhere a real
  adapter is described: no automatic discovery, no reverse mapping, no Pack
  push, no Android change and no background ADB call.

  Add these rows to the Dev Bridge v1 stable-diagnostics table:

  | Code | Meaning |
  |---|---|
  | `device.adb.notConfigured` | A user has not selected an ADB executable in this Studio session. |
  | `device.adb.invalidExecutable` | The native selection did not resolve to a local regular file. |
  | `device.adb.probeFailed` | Studio could not complete the isolated preflight worker. |

  Mark the already merged PR #10 publish/merge checkbox in
  `docs/plans/m3-system-adb-process-adapter.md` as complete and append its
  merge commit `d117a33`; do not alter its recorded validation evidence.

- [ ] **Step 2: Record Task 1–4 completion and check documentation consistency.**

  After focused tests and docs updates pass, mark every completed checkbox in
  Tasks 1–4 of this plan. Run:

  ```sh
  rg -n "TODO|TBD|FIXME|XXX" README.md docs/design docs/protocols docs/architecture
  rg -n "SystemAdb|devices -l|reverse mapping|automatic discovery" README.md docs/design docs/protocols docs/architecture
  git diff --check
  ```

  Expected: no placeholder or whitespace failure; every mention distinguishes
  explicit preflight from the still-deferred reverse mapping.

- [ ] **Step 3: Run the complete local release gate.**

  Run every command separately and require exit code 0:

  ```sh
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

  Expected: all local validation passes; no test or build invokes an ADB
  executable. The no-bundle Tauri build may compile the production probe but
  must not call its `list_devices` method.

- [ ] **Step 4: Commit documentation and validation evidence.**

  Add a dated, concise list of the successful release-gate commands to this
  plan, then commit the documentation and status changes:

  ```sh
  git add README.md docs
  git commit -m "docs(adb): publish user-gated preflight"
  ```

- [ ] **Step 5: Push, review CI, squash-merge and smoke-test main.**

  ```sh
  git push -u origin feature/tauri-adb-preflight
  gh pr create --base main --head feature/tauri-adb-preflight --title "feat: add user-gated ADB preflight"
  gh pr checks <pr-number> --watch
  gh pr merge <pr-number> --squash --delete-branch
  git -C /Users/anpple/Codex/lyra-effects-studio pull --ff-only
  npm run studio:test
  cargo test -p lyra-effects-studio-app --lib
  git status --short
  ```

  Expected: Linux, Windows and macOS CI all pass before merge; the root main
  worktree is clean after smoke tests. If `gh pr merge` reports that `main` is
  already in use by the root worktree, query the PR state before retrying; do
  not assume the remote merge failed.
