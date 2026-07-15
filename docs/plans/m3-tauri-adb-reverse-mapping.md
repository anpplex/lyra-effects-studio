# M3 User-Gated Tauri Dev Bridge Reverse Mapping Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use `executing-plans` to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let a Studio user explicitly create and remove one safe ADB reverse mapping from the private loopback Dev Bridge to exactly one ready Lyra device.

**Architecture:** `DeviceBridgeController` replaces its list-only probe with a private ADB client factory. It uses that factory inside `spawn_blocking` to run the existing `DevBridgeReverseCoordinator`, retains the resulting typed mapping and client privately for cleanup, and serializes map/remove/stop transitions with one operation gate. Tauri and React expose only no-argument actions plus safe mapping readiness; no renderer receives an executable path, serial, port, endpoint or bearer.

**Tech Stack:** Rust 1.97, Tokio 1, Tauri 2, `lyra-device` reverse coordinator, `lyra-adb` fixed-argv adapter, React 19, TypeScript 6 and Vitest.

## Global Constraints

- Do not discover an SDK, read `ANDROID_SERIAL`, accept a raw executable path, serial, port, endpoint, command string or shell fragment from the renderer.
- Only explicit **Enable mapping**, **Remove mapping**, and explicit **Stop bridge** actions may call a mapping-capable ADB operation. App launch, status reads, preflight selection/check, bridge start, browser fixtures and tests must not map or remove a device route.
- Production mapping must use only `DevBridgeReverseCoordinator::establish` and `ReverseMapping::remove` through `SystemAdb`; it must not call `push` or a raw ADB command.
- A mapping is allowed only while a private Dev Bridge listener exists and only when the coordinator freshly selects exactly one `AdbDeviceState::Device` transport.
- Public mapping status contains exactly `readiness`; it must not contain a path, serial, raw stdout/stderr, local/remote port, listener endpoint/URL, session ID or bearer.
- The current selected executable cannot change while mapping state is enabling, active, removing or cleanup-failed. Preserve `device.adb.mappingActive` for that controller-owned rejection.
- On an explicit stop, remove an owned mapping before listener shutdown. If removal fails, keep the listener and mapping state, return the adapter diagnostic unchanged, and allow only an explicit retry.
- Do not add automatic mapping, automatic cleanup on app exit, persistence, background retry, Pack transfer, revision command or Android source change.
- Preserve the existing preflight generation guard and all Dev Bridge lifecycle behavior not explicitly changed here.
- Tests use fake `AdbClient` implementations only. No test, local gate or CI job may execute a real ADB binary.
- Each independently verifiable behavior ends in a Conventional Commit.

## File structure

| File | Responsibility |
|---|---|
| `crates/lyra-device/src/reverse.rs` | Keeps the typed reverse coordinator usable through the controller's private dynamic `AdbClient` ownership. |
| `src-tauri/src/device_bridge.rs` | Private ADB client factory, mapping state machine, stop-time cleanup and fake-first controller tests. |
| `src-tauri/src/lib.rs` | Three no-argument mapping command wrappers and command-registration compile test. |
| `apps/studio/src/lib/backend.ts` | Safe mapping types, typed invokes and browser fixture transitions. |
| `apps/studio/src/lib/backend.test.ts` | Proves mapping invokes have no renderer-supplied arguments. |
| `apps/studio/src/App.tsx` | Independent mapping status, explicit handlers and safe header controls. |
| `apps/studio/src/App.css` | Compact mapping status styles that retain existing editor geometry. |
| `apps/studio/src/App.test.tsx` | Browser-fixture interaction and secret-free rendering coverage. |
| `README.md` and `docs/` | Current boundary, stable diagnostics, non-goals and verification evidence. |
| `docs/plans/m3-tauri-adb-reverse-mapping.md` | This execution record and release evidence. |

---

### Task 1: Add a fake-first private reverse-mapping controller

**Files:**
- Modify: `crates/lyra-device/src/reverse.rs`
- Modify: `src-tauri/src/device_bridge.rs`

**Interfaces:**
- Consumes: `lyra_adb::SystemAdb`, `lyra_device::{AdbClient, DevBridgeReverseCoordinator, DevBridgeReverseRequest, LocalPort, ReverseMapping, DeviceDiagnostic}` and the existing private `DevServerEndpoint`.
- Produces: `DevBridgeMappingReadiness`, `DevBridgeMappingStatus`, `DeviceBridgeController::{mapping_status,enable_mapping,disable_mapping}`, and a private `AdbClientFactory` seam.
- Invariants: no mapping when stopped/unconfigured; no public status leaks an identifier; active or failed cleanup blocks executable replacement; exactly one controller operation owns a mapping transition.

- [x] **Step 1: Write failing controller tests before adding mapping types.**

  Replace the current test-only `FakeAdbProbe` setup with a queue-backed fake
  client factory. It records each canonical executable path and returns a
  scripted `AdbClient` whose `list_devices`, `reverse` and `remove_reverse`
  methods record typed calls. Add these tests in the existing
  `device_bridge::tests` module:

  ```rust
  #[tokio::test]
  async fn mapping_requires_a_running_bridge_without_creating_an_adb_client() {
      let factory = Arc::new(FakeAdbClientFactory::default());
      let controller = DeviceBridgeController::with_factory(Arc::clone(&factory));

      let error = controller.enable_mapping().await.unwrap_err();

      assert_eq!(error.code, "device.bridge.notRunning");
      assert_eq!(controller.mapping_status().await.readiness, DevBridgeMappingReadiness::Inactive);
      factory.assert_no_clients_created();
  }

  #[tokio::test]
  async fn mapping_uses_the_private_listener_port_and_serializes_only_active_state() {
      let executable = tempfile::NamedTempFile::new().unwrap();
      let factory = Arc::new(FakeAdbClientFactory::from_scripts([[
          FakeAdbCall::List(Ok(vec![device("AVATR-01", AdbDeviceState::Device)])),
          FakeAdbCall::Reverse(Ok(())),
      ]]));
      let controller = DeviceBridgeController::with_factory(Arc::clone(&factory));
      controller.configure_adb_executable(executable.path().into()).await.unwrap();
      controller.start().await.unwrap();
      let listener_port = {
          let running = controller.running.lock().await;
          running.as_ref().unwrap().endpoint.address().port()
      };

      assert_eq!(controller.enable_mapping().await.unwrap().readiness, DevBridgeMappingReadiness::Active);
      factory.assert_established_once_with(executable.path(), listener_port);
      assert_eq!(serde_json::to_value(controller.mapping_status().await).unwrap(), serde_json::json!({ "readiness": "active" }));
  }

  #[tokio::test]
  async fn stop_retries_nothing_after_cleanup_failure_and_keeps_the_bridge_running() {
      let executable = tempfile::NamedTempFile::new().unwrap();
      let factory = Arc::new(FakeAdbClientFactory::from_scripts([[
          FakeAdbCall::List(Ok(vec![device("AVATR-01", AdbDeviceState::Device)])),
          FakeAdbCall::Reverse(Ok(())),
          FakeAdbCall::Remove(Err(DeviceDiagnostic::new("device.adb.commandFailed", "remove failed"))),
          FakeAdbCall::Remove(Ok(())),
      ]]));
      let controller = ready_mapping_controller(executable.path(), factory).await;

      let error = controller.stop().await.unwrap_err();
      assert_eq!(error.code, "device.adb.commandFailed");
      assert_eq!(controller.mapping_status().await.readiness, DevBridgeMappingReadiness::CleanupFailed);
      assert_eq!(controller.status().await.state, DeviceBridgeState::Waiting);

      assert_eq!(controller.disable_mapping().await.unwrap().readiness, DevBridgeMappingReadiness::Inactive);
      assert_eq!(controller.stop().await.unwrap().state, DeviceBridgeState::Stopped);
  }
  ```

  Also add focused coverage that an unconfigured running bridge returns
  `device.adb.notConfigured` without creating a client; duplicate enable is
  idempotent and does not list again; selecting a different executable while
  `Active` returns `device.adb.mappingActive`; explicit disable while inactive
  returns `inactive` without creating a client; and a malformed/multiple/zero
  ready-device result preserves the existing coordinator diagnostic and leaves
  mapping `inactive`. Add one panicking fake-client case for establish and one
  for remove: their worker join failures must use `device.adb.probeFailed`,
  leave establish `inactive`, and retain removal as `cleanupFailed`.

  Use these test-only building blocks so every path remains typed and no fake
  reaches a process:

  ```rust
  enum FakeAdbCall {
      List(Result<Vec<AdbDevice>, DeviceDiagnostic>),
      Reverse(Result<(), DeviceDiagnostic>),
      Remove(Result<(), DeviceDiagnostic>),
  }

  struct FakeAdbClientFactory {
      scripts: StdMutex<VecDeque<VecDeque<FakeAdbCall>>>,
      created_paths: StdMutex<Vec<PathBuf>>,
      reverse_ports: Arc<StdMutex<Vec<u16>>>,
  }

  struct FakeAdbClient {
      calls: VecDeque<FakeAdbCall>,
      reverse_ports: Arc<StdMutex<Vec<u16>>>,
  }

  impl FakeAdbClientFactory {
      fn from_scripts<I, S>(scripts: I) -> Self
      where
          I: IntoIterator<Item = S>,
          S: IntoIterator<Item = FakeAdbCall>,
      {
          Self {
              scripts: StdMutex::new(
                  scripts.into_iter().map(|script| script.into_iter().collect()).collect(),
              ),
              created_paths: StdMutex::new(Vec::new()),
              reverse_ports: Arc::new(StdMutex::new(Vec::new())),
          }
      }

      fn assert_no_clients_created(&self) {
          assert!(self.created_paths.lock().unwrap().is_empty());
      }

      fn assert_established_once_with(&self, executable: &Path, local_port: u16) {
          assert_eq!(self.created_paths.lock().unwrap().as_slice(), [executable.to_path_buf()]);
          assert_eq!(self.reverse_ports.lock().unwrap().as_slice(), [local_port]);
      }
  }

  impl AdbClientFactory for FakeAdbClientFactory {
      fn create(&self, executable: &Path) -> Box<dyn AdbClient + Send> {
          self.created_paths.lock().unwrap().push(executable.into());
          Box::new(FakeAdbClient {
              calls: self.scripts.lock().unwrap().pop_front().expect("unexpected ADB client"),
              reverse_ports: Arc::clone(&self.reverse_ports),
          })
      }
  }

  async fn ready_mapping_controller(
      executable: &Path,
      factory: Arc<FakeAdbClientFactory>,
  ) -> DeviceBridgeController {
      let controller = DeviceBridgeController::with_factory(factory);
      controller.configure_adb_executable(executable.into()).await.unwrap();
      controller.start().await.unwrap();
      controller.enable_mapping().await.unwrap();
      controller
  }
  ```

  Implement `AdbClient` for `FakeAdbClient` by popping the expected enum
  variant, asserting the typed serial/local/remote arguments in `reverse` and
  `remove_reverse`, and failing the test on a wrong or exhausted call. Have the
  factory's shared `reverse_ports` record the local port from that asserted
  `reverse` call, then make
  `assert_established_once_with` compare it to the private listener port.

- [x] **Step 2: Run the focused controller tests to verify the red state.**

  Run:

  ```sh
  cargo test -p lyra-effects-studio-app device_bridge::tests::mapping --lib
  ```

  Expected: compilation fails because mapping status, factory injection and
  controller methods do not exist.

- [x] **Step 3: Replace the list-only probe with one private client factory.**

  First relax only the generic sizing bound in the existing portable reverse
  API so the controller can retain a private dynamic client without duplicating
  selection policy:

  ```rust
  impl ReverseMapping {
      pub fn remove<C: AdbClient + ?Sized>(
          &self,
          adb: &mut C,
      ) -> Result<(), DeviceDiagnostic> {
          adb.remove_reverse(&self.serial, self.remote_port)
      }
  }

  impl DevBridgeReverseCoordinator {
      pub fn establish<C: AdbClient + ?Sized>(
          adb: &mut C,
          request: DevBridgeReverseRequest,
      ) -> Result<ReverseMapping, DeviceDiagnostic> {
          let serial = select_one_ready_device(adb.list_devices()?)?;
          let local_port = request.local_port();
          let remote_port = request.remote_port();

          adb.reverse(&serial, local_port, remote_port)?;

          Ok(ReverseMapping {
              serial,
              local_port,
              remote_port,
          })
      }
  }
  ```

  The body remains the current typed list/select/reverse sequence; only
  `?Sized` changes. Existing `FakeAdb` tests remain the regression proof that
  the portable contract has not changed.

  At the top of `device_bridge.rs`, replace `AdbDeviceProbe` and
  `SystemAdbDeviceProbe` with this private construction boundary:

  ```rust
  trait AdbClientFactory: Send + Sync {
      fn create(&self, executable: &Path) -> Box<dyn AdbClient + Send>;
  }

  struct SystemAdbClientFactory;

  impl AdbClientFactory for SystemAdbClientFactory {
      fn create(&self, executable: &Path) -> Box<dyn AdbClient + Send> {
          Box::new(SystemAdb::from_path(executable))
      }
  }
  ```

  Make `DeviceBridgeController::new()` call a private
  `from_factory(Arc<dyn AdbClientFactory>)`. Rename the test-only constructor
  to `with_factory`. Update `check_adb` to clone the canonical path and factory
  into `spawn_blocking`, create a fresh client there, and call only
  `list_devices`. Retain the existing generation comparison after the worker
  returns.

  Add the safe public projection and private mapping storage:

  ```rust
  #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub(crate) enum DevBridgeMappingReadiness {
      Inactive,
      Enabling,
      Active,
      Removing,
      CleanupFailed,
  }

  #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub(crate) struct DevBridgeMappingStatus {
      readiness: DevBridgeMappingReadiness,
  }

  struct ActiveMapping {
      mapping: ReverseMapping,
      adb: Box<dyn AdbClient + Send>,
  }

  enum MappingState {
      Inactive,
      Enabling,
      Active(ActiveMapping),
      Removing(ActiveMapping),
      CleanupFailed(ActiveMapping),
  }
  ```

  Add `mapping: MappingState` to `AdbPreflightState` with `Inactive` as its
  initial value, and add `mapping_operation: Mutex<()>` to the controller. Map
  private state to `DevBridgeMappingStatus` with a pure helper; do not derive
  `Serialize`, `Debug` or `Clone` for `ActiveMapping`.

- [x] **Step 4: Implement explicit establish, remove and stop-time cleanup.**

  Add these controller methods and helpers. `enable_mapping` and
  `disable_mapping` must acquire `mapping_operation`, but neither may hold
  `running` or `adb_preflight` while awaiting a worker.

  ```rust
  pub(crate) async fn mapping_status(&self) -> DevBridgeMappingStatus;

  pub(crate) async fn enable_mapping(
      &self,
  ) -> Result<DevBridgeMappingStatus, DeviceDiagnostic>;

  pub(crate) async fn disable_mapping(
      &self,
  ) -> Result<DevBridgeMappingStatus, DeviceDiagnostic>;
  ```

  `enable_mapping` must:

  1. obtain the listener port from `running.endpoint.address().port()` or
     return `DeviceDiagnostic::new("device.bridge.notRunning", "start the Dev Bridge before enabling its ADB mapping")` without creating a client;
  2. convert that non-zero port with `LocalPort::new`, treating failure as an
     internal `device.bridge.notRunning` diagnostic because the server always
     owns a non-zero listener;
  3. clone the configured canonical executable or preserve
     `not_configured()` without creating a client;
  4. return current `active` status without a worker if already active; reject
     `CleanupFailed` with `mapping_active()` and no second establish;
  5. set `Enabling`, then use `spawn_blocking` to create the factory client and
     call:

     ```rust
     let mapping = DevBridgeReverseCoordinator::establish(
         adb.as_mut(),
         DevBridgeReverseRequest::new(local_port),
     )?;
     ```

  6. retain both `mapping` and `adb` as `ActiveMapping` on success; on adapter
     error or worker join failure return the diagnostic, restore `Inactive`,
     and never run speculative removal.

  `disable_mapping` must return `inactive` without creating a client when no
  mapping exists. Otherwise move `ActiveMapping` into `Removing`, call only
  `mapping.remove(adb.as_mut())` within `spawn_blocking`, and either drop it
  into `Inactive` on success or restore it as `CleanupFailed` while returning
  the unchanged adapter diagnostic. Map worker join failure to
  `device.adb.probeFailed` and `CleanupFailed`.

  Refactor `stop()` to acquire the same operation guard, call the private
  remove helper when mapping is active or cleanup-failed, and only then take
  and gracefully shut down `RunningBridge`. If removal fails, return early and
  keep `running` intact. Preserve the existing `stop` return type by converting
  that `DeviceDiagnostic` to `ServerDiagnostic::new(error.code, error.message)`;
  this retains the stable code and message seen by the Tauri command. Update
  `configure_adb_executable` to reject every
  non-inactive mapping state with:

  ```rust
  DeviceDiagnostic::new(
      "device.adb.mappingActive",
      "remove the active ADB mapping before selecting another executable",
  )
  ```

  Factor this diagnostic into a `mapping_active()` helper and use the same
  helper for an enable request while cleanup is still pending. Do not create a
  second mapping to recover from cleanup failure.

- [x] **Step 5: Verify the fake-first Rust boundary.**

  Run:

  ```sh
  cargo fmt --check
  cargo test -p lyra-effects-studio-app device_bridge::tests --lib
  cargo clippy -p lyra-effects-studio-app --all-targets -- -D warnings
  rg -n "Command::new|\.args\(|shell|ANDROID_SERIAL" src-tauri/src/device_bridge.rs crates/lyra-adb --glob '*.rs'
  ```

  Expected: all controller tests pass. The final search finds process creation
  only in `crates/lyra-adb/src/adapter.rs`; no controller test reaches a real
  executable.

- [x] **Step 6: Commit the mapping controller.**

  ```sh
  git add src-tauri/src/device_bridge.rs
  git commit -m "feat(adb): add explicit mapping controller"
  ```

### Task 2: Expose only no-argument mapping commands

**Files:**
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `DeviceBridgeController::{mapping_status,enable_mapping,disable_mapping}`.
- Produces: `get_device_bridge_mapping_status`, `enable_device_bridge_mapping` and `disable_device_bridge_mapping` Tauri commands.
- Invariants: no command accepts renderer-controlled device data; no native dialog is opened; command errors contain diagnostics but UI will not render their raw messages.

- [x] **Step 1: Write the failing command-surface test.**

  Extend `src-tauri/src/lib.rs` tests:

  ```rust
  #[test]
  fn device_bridge_mapping_commands_are_available() {
      let _ = super::get_device_bridge_mapping_status;
      let _ = super::enable_device_bridge_mapping;
      let _ = super::disable_device_bridge_mapping;
  }
  ```

- [x] **Step 2: Run the focused library test to verify the red state.**

  Run:

  ```sh
  cargo test -p lyra-effects-studio-app device_bridge_mapping_commands --lib
  ```

  Expected: compilation fails because the three wrappers do not exist.

- [x] **Step 3: Add the narrow command wrappers and registration.**

  Add these wrappers next to the current bridge and preflight commands:

  ```rust
  #[tauri::command]
  async fn get_device_bridge_mapping_status(
      controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
  ) -> Result<device_bridge::DevBridgeMappingStatus, String> {
      Ok(controller.mapping_status().await)
  }

  #[tauri::command]
  async fn enable_device_bridge_mapping(
      controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
  ) -> Result<device_bridge::DevBridgeMappingStatus, String> {
      controller.enable_mapping().await.map_err(|error| error.to_string())
  }

  #[tauri::command]
  async fn disable_device_bridge_mapping(
      controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
  ) -> Result<device_bridge::DevBridgeMappingStatus, String> {
      controller.disable_mapping().await.map_err(|error| error.to_string())
  }
  ```

  Register exactly these three functions in `tauri::generate_handler!`. Do not
  add command arguments, a serializer for mapping internals or a helper that
  invokes ADB outside the controller.

- [x] **Step 4: Verify the Tauri layer without starting ADB.**

  Run:

  ```sh
  cargo fmt --check
  cargo test -p lyra-effects-studio-app --lib
  cargo clippy -p lyra-effects-studio-app --all-targets -- -D warnings
  ```

  Expected: all library tests pass without opening a dialog, starting a
  listener beyond existing fake loopback tests, or launching ADB.

- [x] **Step 5: Commit the command surface.**

  ```sh
  git add src-tauri/src/lib.rs
  git commit -m "feat(adb): expose explicit mapping actions"
  ```

### Task 3: Add the typed Studio mapping contract and browser fixture

**Files:**
- Modify: `apps/studio/src/lib/backend.ts`
- Modify: `apps/studio/src/lib/backend.test.ts`

**Interfaces:**
- Consumes: Task 2's exact snake-case commands.
- Produces: `DevBridgeMappingReadiness`, `DevBridgeMappingStatus` and three `StudioBackend` methods.
- Browser fixture behavior: `inactive → active → inactive`, with bridge and preflight transitions remaining independent.

- [x] **Step 1: Write the failing no-argument facade test.**

  Import `DevBridgeMappingStatus` and append:

  ```ts
  it("uses no-argument commands for explicit Dev Bridge mapping", async () => {
    const inactive: DevBridgeMappingStatus = { readiness: "inactive" };
    const invoke = vi.fn(async () => inactive);
    const backend = createBackend(invoke);

    await expect(backend.deviceBridgeMappingStatus()).resolves.toEqual(inactive);
    await expect(backend.enableDeviceBridgeMapping()).resolves.toEqual(inactive);
    await expect(backend.disableDeviceBridgeMapping()).resolves.toEqual(inactive);
    expect(invoke).toHaveBeenNthCalledWith(1, "get_device_bridge_mapping_status");
    expect(invoke).toHaveBeenNthCalledWith(2, "enable_device_bridge_mapping");
    expect(invoke).toHaveBeenNthCalledWith(3, "disable_device_bridge_mapping");
    expect(invoke).not.toHaveBeenCalledWith(expect.any(String), expect.anything());
  });
  ```

- [x] **Step 2: Run the focused facade test to verify the red state.**

  Run:

  ```sh
  npm --prefix apps/studio run test -- backend.test.ts
  ```

  Expected: a TypeScript or runtime failure because mapping status and methods
  do not exist.

- [x] **Step 3: Implement the safe renderer contract and fixture.**

  Add this public type after `AdbPreflightStatus`:

  ```ts
  export type DevBridgeMappingReadiness =
    | "inactive"
    | "enabling"
    | "active"
    | "removing"
    | "cleanupFailed";

  export interface DevBridgeMappingStatus {
    readiness: DevBridgeMappingReadiness;
  }
  ```

  Add `deviceBridgeMappingStatus`, `enableDeviceBridgeMapping` and
  `disableDeviceBridgeMapping` to `StudioBackend`, and invoke the three Task 2
  command strings with no second argument. In `createFixtureBackend`, keep a
  closure-local `{ readiness: "inactive" }` status. Enable changes it to
  `{ readiness: "active" }`; disable and `stopDeviceBridge` change it back to
  `{ readiness: "inactive" }`. Return `structuredClone` from every mapping
  fixture result. Do not add a path, serial, port, endpoint or raw error field.

- [x] **Step 4: Verify the typed frontend boundary.**

  Run:

  ```sh
  npm --prefix apps/studio run lint
  npm --prefix apps/studio run test -- backend.test.ts
  ```

  Expected: the exact mapping invokes are argument-free and lint reports no
  unsafe renderer type escape.

- [x] **Step 5: Commit the mapping facade.**

  ```sh
  git add apps/studio/src/lib/backend.ts apps/studio/src/lib/backend.test.ts
  git commit -m "feat(studio): add mapping backend facade"
  ```

### Task 4: Render explicit mapping controls in Studio

**Files:**
- Modify: `apps/studio/src/App.tsx`
- Modify: `apps/studio/src/App.css`
- Modify: `apps/studio/src/App.test.tsx`

**Interfaces:**
- Consumes: Task 3's mapping backend methods plus existing bridge and preflight status.
- Produces: `device-mapping-control`, `device-mapping-toggle` and safe mapping labels.
- Copy: `Mapping off`, `Enabling mapping…`, `Mapping active`, `Removing mapping…`, `Retry mapping removal`, `Enable mapping`, `Remove mapping`, `Retry remove` and `Mapping failed`.

- [x] **Step 1: Write the failing fixture interaction test.**

  Add this test after the preflight interaction:

  ```tsx
  it("creates and removes an explicit mapping only after bridge and ADB preflight", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    const control = await screen.findByTestId("device-mapping-control");
    const action = screen.getByTestId("device-mapping-toggle");
    expect(await screen.findByText("Mapping off")).toBeInTheDocument();
    expect(action).toBeDisabled();
    expect(control).not.toHaveTextContent("/Users/");
    expect(control).not.toHaveTextContent("Bearer");

    await user.click(screen.getByTestId("device-bridge-toggle"));
    expect(await screen.findByText("Waiting for Lyra")).toBeInTheDocument();
    await user.click(screen.getByTestId("device-adb-select"));
    expect(await screen.findByText("ADB selected")).toBeInTheDocument();
    await user.click(screen.getByTestId("device-adb-check"));
    expect(await screen.findByText("1 device ready")).toBeInTheDocument();
    expect(action).toBeEnabled();

    await user.click(action);
    expect(await screen.findByText("Mapping active")).toBeInTheDocument();
    expect(action).toHaveTextContent("Remove mapping");

    await user.click(action);
    expect(await screen.findByText("Mapping off")).toBeInTheDocument();
  });
  ```

- [x] **Step 2: Run the focused UI test to verify the red state.**

  Run:

  ```sh
  npm --prefix apps/studio run test -- App.test.tsx
  ```

  Expected: failure because the mapping control and methods are absent.

- [x] **Step 3: Add independent mapping state and safe event handlers.**

  Import `DevBridgeMappingStatus`, add:

  ```ts
  const INACTIVE_DEVICE_MAPPING: DevBridgeMappingStatus = { readiness: "inactive" };

  function mappingLabel(status: DevBridgeMappingStatus, loading: boolean, failed: boolean): string {
    if (status.readiness === "cleanupFailed") return "Retry mapping removal";
    if (failed) return "Mapping failed";
    if (loading || status.readiness === "enabling") return "Enabling mapping…";
    if (status.readiness === "active") return "Mapping active";
    if (status.readiness === "removing") return "Removing mapping…";
    return "Mapping off";
  }
  ```

  Add separate `deviceMapping`, `deviceMappingLoading`, `deviceMappingBusy`
  and `deviceMappingError` state. Refresh mapping once on mount through
  `backend.deviceBridgeMappingStatus()`. After a successful bridge start/stop,
  refresh mapping as well, because native stop may remove it. Implement
  `toggleDeviceMapping` so `active` and `cleanupFailed` call only
  `backend.disableDeviceBridgeMapping()`, all other eligible states call only
  `backend.enableDeviceBridgeMapping()`. On a rejected map/remove action, first
  refresh mapping: if that read returns `cleanupFailed`, clear the local generic
  error so **Retry remove** remains enabled; only a failed refresh leaves the
  generic `Mapping failed` state.

  Enable the action only when all are true: the bridge is not stopped, ADB
  preflight readiness is `oneReadyDevice`, mapping status is not enabling or
  removing, and no bridge/preflight/mapping request is busy. A generic mapping
  read error disables the action except when the last safe status is
  `cleanupFailed`, which must retain its explicit retry. Disable ADB selection
  while mapping is non-inactive or mapping work is in flight.
  Render a compact section inside the existing `.publish-block` before the
  Build pack button:

  ```tsx
  <section className="device-mapping-control" data-testid="device-mapping-control" aria-label="Dev Bridge mapping">
    <span className={`device-mapping-status ${mappingPhase}`} role="status" aria-live="polite">
      <i />
      <span className="device-mapping-label">{mappingLabelText}</span>
    </span>
    <button className="quiet-button device-mapping-action" data-testid="device-mapping-toggle" disabled={!canToggleMapping} onClick={() => void toggleDeviceMapping()}>
      {mappingAction}
    </button>
  </section>
  ```

  Do not show the rejected Tauri error, status title, serial, executable path,
  port, endpoint or bearer.

- [x] **Step 4: Add compact responsive mapping styles.**

  Extend the existing header flex selector with `.device-mapping-control` and
  `.device-mapping-status`. Use neutral dot for inactive, animated neutral for
  enabling/removing, cyan for active and orange for cleanup failure/local
  error. At the existing `1240px` breakpoint hide only
  `.device-mapping-label`; keep the action button visible and accessible. Do
  not alter the three-column workspace grid, preview dimensions or Build pack
  button styles.

- [x] **Step 5: Verify the Studio interaction and production compile.**

  Run:

  ```sh
  npm --prefix apps/studio run lint
  npm --prefix apps/studio run test -- App.test.tsx
  npm --prefix apps/studio run build
  ```

  Expected: fixture mapping remains disabled before explicit prerequisite
  actions, then reaches active and inactive without any private value rendered.

- [x] **Step 6: Commit the visible mapping control.**

  ```sh
  git add apps/studio/src/App.tsx apps/studio/src/App.css apps/studio/src/App.test.tsx
  git commit -m "feat(studio): add explicit mapping controls"
  ```

### Task 5: Publish the M3 mapping boundary and release it

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture/rust-tauri.md`
- Modify: `docs/design/fake-first-device-core.md`
- Modify: `docs/design/device-adb-reverse-coordinator.md`
- Modify: `docs/design/loopback-dev-server.md`
- Modify: `docs/design/studio-device-bridge.md`
- Modify: `docs/design/system-adb-process-adapter.md`
- Modify: `docs/design/tauri-adb-preflight.md`
- Modify: `docs/protocols/dev-bridge-v1.md`
- Modify: `docs/plans/m3-tauri-adb-preflight.md`
- Modify: `docs/plans/m3-tauri-adb-reverse-mapping.md`

**Interfaces:**
- Consumes: the completed explicit mapping controller, Tauri commands and Studio control.
- Produces: current public documentation that distinguishes explicit mapping from deferred runtime provisioning and records release evidence.

- [x] **Step 1: Update documentation and stable diagnostics.**

  Update README and the listed architecture/design documents to state that
  Studio can explicitly establish or remove one mapping only after native ADB
  selection, preflight and bridge start. State everywhere that no action
  exposes private values or enables automatic mapping, app-exit cleanup,
  Pack/revision transfer or Android changes.

  Add these Dev Bridge v1 diagnostic rows:

  | Code | Meaning |
  |---|---|
  | `device.bridge.notRunning` | A mapping was requested without an active loopback Dev Bridge. |
  | `device.adb.mappingActive` | A new ADB executable selection was blocked because an owned mapping still needs its current executable. |

  In `docs/plans/m3-tauri-adb-preflight.md`, mark its already merged Task 5
  push/CI/squash/smoke step complete and append PR #11 plus merge commit
  `2c660ad`. Do not alter its recorded release-gate evidence.

- [x] **Step 2: Record completed tasks and check documentation consistency.**

  After focused tests and documentation edits pass, mark Task 1–4 checkboxes
  in this plan. Run:

  ```sh
  rg -n "TODO|TBD|FIXME|XXX" README.md docs/design docs/protocols docs/architecture
  rg -n "SystemAdb|devices -l|reverse mapping|automatic mapping|app-exit" README.md docs/design docs/protocols docs/architecture
  git diff --check
  ```

  Expected: no placeholder or whitespace issue; every reference distinguishes
  user-gated mapping from runtime provisioning and automatic behavior.

- [x] **Step 3: Run the complete local release gate.**

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

  Expected: all commands pass. Production compile may include the mapping
  adapter, but no local test or build invokes its mapping methods or a real ADB
  executable.

  Verified locally on 2026-07-15: `npm run studio:lint`,
  `npm run studio:test` (7 files / 29 tests), `npm run studio:build`,
  `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace`, `cargo build --workspace --release`,
  `npx tauri build --debug --no-bundle` and `git diff --check` all exited
  successfully. All ADB behavioral tests used injected fake clients; no real
  ADB executable was discovered or invoked.

- [x] **Step 4: Commit documentation and validation evidence.**

  Add a dated, concise list of successful gate commands to this plan, then:

  ```sh
  git add README.md docs
  git commit -m "docs(adb): publish explicit mapping boundary"
  ```

- [ ] **Step 5: Push, review CI, squash-merge and smoke-test main.**

  ```sh
  git push -u origin feature/tauri-adb-reverse
  gh pr create --base main --head feature/tauri-adb-reverse --title "feat: add explicit Dev Bridge mapping"
  gh pr checks <pr-number> --watch
  gh pr merge <pr-number> --squash --delete-branch
  git -C /Users/anpple/Codex/lyra-effects-studio pull --ff-only
  npm run studio:test
  cargo test -p lyra-effects-studio-app --lib
  git status --short
  ```

  Expected: Linux, Windows and macOS CI all succeed before merge. If local
  `gh pr merge` reports that main is already used by the root worktree, query
  the PR state before retrying, then clean up this project-local worktree only
  after main smoke tests succeed.
