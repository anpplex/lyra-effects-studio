# M3 Studio Device Bridge controls implementation plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let Studio start, inspect and stop the authenticated local Dev Bridge while keeping every provisioning secret and all ADB authority inside Rust.

**Architecture:** A Tauri-managed `DeviceBridgeController` owns one optional `DevServer` plus its private provisioning endpoint. Three asynchronous commands return a non-secret status read model. React consumes the same typed contract through its existing backend facade, with an in-memory browser fixture for tests.

**Tech Stack:** Rust 1.97, Tauri 2, Tokio 1, `lyra-dev-server`, React 19, TypeScript 6, Vitest.

## Global Constraints

- The only listener remains `lyra-dev-server` on `127.0.0.1:0`; no command accepts a host, port, URL or token.
- `DeviceBridgeStatus` may expose only `state` and a `DeviceBridgeSession` projection with device profile, protocol version and capabilities; it must never expose a bearer, URL, port or session ID.
- The only status values are `stopped`, `waiting` and `connected`; a session is present only for `connected`.
- The host policy is protocol `1.2.0`, supports `activate` / `stageRevision`, and requires `stageRevision`.
- Start is idempotent; stop is idempotent and performs graceful `DevServer::shutdown` outside the controller mutex.
- Do not execute `adb`, start an Android process, mutate a Pack, add a network route or add arbitrary command input.
- Keep Tauri commands asynchronous and typed through `apps/studio/src/lib/backend.ts`.
- Follow red-green-refactor, run the stated focused tests before each commit, and use Conventional Commits.

---

### Task 1: Add the private Tauri Dev Bridge lifecycle controller

**Files:**
- Modify: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/device_bridge.rs`
- Modify: `src-tauri/src/lib.rs`

**Interfaces:**
- Consumes: `lyra_dev_server::{DevServer, DevServerEndpoint, SessionSnapshot}` and `lyra_device::HostPolicy`.
- Produces: `DeviceBridgeController::{new,status,start,stop}`, `DeviceBridgeSession`, `DeviceBridgeStatus { state, session }`, and Tauri commands `get_device_bridge_status`, `start_device_bridge`, `stop_device_bridge`.
- Invariants: `RunningBridge` remains private and retains its endpoint; all public status types are `Serialize` and contain no endpoint or session-ID fields.

- [ ] **Step 1: Write failing unit tests in `src-tauri/src/device_bridge.rs`.**

  Add a `#[cfg(test)]` module with the following state-transition assertions. Keep the raw TCP helper private to the test module; it reads the private endpoint only to send `Fixtures/Device/hello-valid.json` with the current bearer.

  ```rust
  #[tokio::test]
  async fn controller_transitions_from_stopped_to_waiting_to_connected() {
      let controller = DeviceBridgeController::new();
      assert_eq!(controller.status().await.state, DeviceBridgeState::Stopped);

      let waiting = controller.start().await.unwrap();
      assert_eq!(waiting.state, DeviceBridgeState::Waiting);
      assert!(waiting.session.is_none());

      send_fixture_hello(&controller).await;
      let connected = controller.status().await;
      assert_eq!(connected.state, DeviceBridgeState::Connected);
      assert_eq!(connected.session.unwrap().device_profile_id, "com.avatr.cluster.4032x284");
      assert!(serde_json::to_value(&connected).unwrap()["session"].get("sessionId").is_none());

      controller.stop().await.unwrap();
      assert_eq!(controller.status().await.state, DeviceBridgeState::Stopped);
  }

  #[tokio::test]
  async fn start_and_stop_are_idempotent_and_status_is_secret_free() {
      let controller = DeviceBridgeController::new();
      let first = controller.start().await.unwrap();
      let second = controller.start().await.unwrap();
      assert_eq!(first, second);
      assert_eq!(serde_json::to_value(&first).unwrap(), serde_json::json!({
          "state": "waiting",
          "session": null
      }));
      controller.stop().await.unwrap();
      controller.stop().await.unwrap();
  }
  ```

- [ ] **Step 2: Run the focused tests to verify they fail.**

  Run: `cargo test -p lyra-effects-studio-app device_bridge --lib`

  Expected: compilation failure because `device_bridge` and its controller types do not exist.

- [ ] **Step 3: Add direct dependencies and the minimal controller.**

  In `src-tauri/Cargo.toml`, add `lyra-dev-server = { path = "../crates/lyra-dev-server" }` and direct Tokio support:

  ```toml
  tokio = { version = "1.52.3", features = ["io-util", "macros", "net", "rt-multi-thread", "sync"] }
  ```

  In `device_bridge.rs`, use this public read model:

  ```rust
  #[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "lowercase")]
  pub(crate) enum DeviceBridgeState { Stopped, Waiting, Connected }

  #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub(crate) struct DeviceBridgeSession {
      device_profile_id: String,
      protocol_version: String,
      capabilities: Vec<String>,
  }

  #[derive(Clone, Debug, Eq, PartialEq, Serialize)]
  #[serde(rename_all = "camelCase")]
  pub(crate) struct DeviceBridgeStatus {
      state: DeviceBridgeState,
      session: Option<DeviceBridgeSession>,
  }
  ```

  Implement `DeviceBridgeController` with `Mutex<Option<RunningBridge>>`. `start` creates the fixed `HostPolicy`, starts `DevServer` only when absent, then returns `waiting` or `connected` according to `session_snapshot`. Convert a `SessionSnapshot` into `DeviceBridgeSession` by copying only `device_profile_id`, `protocol_version` and `capabilities`; discard `session_id`. `status` returns `stopped` when the option is absent. `stop` uses `take()` while locked, drops the guard, awaits `server.shutdown()`, then returns `stopped`. The private endpoint is retained in `RunningBridge`; use its loopback address for a `debug_assert!` during status calculation, but never serialize it.

- [ ] **Step 4: Register the narrow commands with Tauri.**

  In `src-tauri/src/lib.rs`, add `mod device_bridge;`, manage one controller, and register only these command signatures next to the existing project commands:

  ```rust
  #[tauri::command]
  pub(crate) async fn get_device_bridge_status(
      controller: tauri::State<'_, DeviceBridgeController>,
  ) -> DeviceBridgeStatus;

  #[tauri::command]
  pub(crate) async fn start_device_bridge(
      controller: tauri::State<'_, DeviceBridgeController>,
  ) -> Result<DeviceBridgeStatus, String>;

  #[tauri::command]
  pub(crate) async fn stop_device_bridge(
      controller: tauri::State<'_, DeviceBridgeController>,
  ) -> Result<DeviceBridgeStatus, String>;
  ```

  Convert only `ServerDiagnostic` to its stable `to_string()` form for rejected Tauri calls. Do not add a command that returns `DevServerEndpoint`.

- [ ] **Step 5: Run focused Rust verification.**

  Run: `cargo fmt --check && cargo test -p lyra-effects-studio-app --lib && cargo clippy -p lyra-effects-studio-app --all-targets -- -D warnings`

  Expected: the controller tests pass, authenticated fixture hello makes the status connected, and no warning or secret-bearing status field exists.

- [ ] **Step 6: Commit the independently testable backend boundary.**

  ```sh
  git add src-tauri/Cargo.toml src-tauri/src/device_bridge.rs src-tauri/src/lib.rs Cargo.lock
  git commit -m "feat(bridge): add Tauri lifecycle controls"
  ```

### Task 2: Extend the typed Studio backend facade and browser fixture

**Files:**
- Modify: `apps/studio/src/lib/backend.ts`
- Modify: `apps/studio/src/lib/backend.test.ts`

**Interfaces:**
- Consumes: command names from Task 1.
- Produces: `DeviceBridgeState`, `DeviceBridgeSession`, `DeviceBridgeStatus`, plus `StudioBackend.deviceBridgeStatus()`, `.startDeviceBridge()` and `.stopDeviceBridge()`.
- Browser fixture behavior: `stopped → waiting → stopped`; no fixture method can construct a `connected` session or carry provisioning data.

- [ ] **Step 1: Write failing facade tests.**

  Add a test that constructs `createBackend(invoke)` and asserts the exact invocation sequence:

  ```ts
  const stopped: DeviceBridgeStatus = { state: "stopped", session: null };
  const invoke = vi.fn(async () => stopped);
  const backend = createBackend(invoke);

  await expect(backend.deviceBridgeStatus()).resolves.toEqual(stopped);
  await expect(backend.startDeviceBridge()).resolves.toEqual(stopped);
  await expect(backend.stopDeviceBridge()).resolves.toEqual(stopped);
  expect(invoke).toHaveBeenNthCalledWith(1, "get_device_bridge_status");
  expect(invoke).toHaveBeenNthCalledWith(2, "start_device_bridge");
  expect(invoke).toHaveBeenNthCalledWith(3, "stop_device_bridge");
  ```

- [ ] **Step 2: Run the focused test to verify it fails.**

  Run: `npm --prefix apps/studio run test -- backend.test.ts`

  Expected: TypeScript or runtime failure because the bridge methods and status type are absent.

- [ ] **Step 3: Implement only the typed facade and fixture state.**

  Add these TypeScript shapes without URL, port, token or session-ID properties:

  ```ts
  export type DeviceBridgeState = "stopped" | "waiting" | "connected";
  export interface DeviceBridgeSession {
    deviceProfileId: string;
    protocolVersion: string;
    capabilities: string[];
  }
  export interface DeviceBridgeStatus {
    state: DeviceBridgeState;
    session: DeviceBridgeSession | null;
  }
  ```

  Map the three methods directly to the exact snake-case Tauri names. Give `createFixtureBackend` a closure-local `DeviceBridgeStatus`; clone it for reads, set `{ state: "waiting", session: null }` on start and `{ state: "stopped", session: null }` on stop.

- [ ] **Step 4: Run focused frontend verification.**

  Run: `npm --prefix apps/studio run lint && npm --prefix apps/studio run test -- backend.test.ts`

  Expected: facade tests prove the wire names and lint finds no unsafe `any` or unused type.

- [ ] **Step 5: Commit the renderer contract.**

  ```sh
  git add apps/studio/src/lib/backend.ts apps/studio/src/lib/backend.test.ts
  git commit -m "feat(studio): add typed bridge backend facade"
  ```

### Task 3: Render and control bridge state in the Studio header

**Files:**
- Modify: `apps/studio/src/App.tsx`
- Modify: `apps/studio/src/App.css`
- Modify: `apps/studio/src/App.test.tsx`

**Interfaces:**
- Consumes: `backend.deviceBridgeStatus`, `.startDeviceBridge`, `.stopDeviceBridge` and `DeviceBridgeStatus` from Task 2.
- Produces: a header `data-testid="device-bridge-control"`, status label, Start/Stop button and inline error message.
- Copy: `Bridge off`, `Waiting for Lyra`, `Lyra connected`, `Start bridge`, `Stop bridge`.

- [ ] **Step 1: Write failing UI tests.**

  Add one browser-fixture interaction test:

  ```tsx
  it("starts and stops the local device bridge without displaying provisioning data", async () => {
    const user = userEvent.setup();
    render(<StrictMode><App /></StrictMode>);

    const control = await screen.findByTestId("device-bridge-control");
    expect(control).toHaveTextContent("Bridge off");
    expect(control).not.toHaveTextContent("Bearer");
    await user.click(screen.getByTestId("device-bridge-toggle"));
    expect(control).toHaveTextContent("Waiting for Lyra");
    await user.click(screen.getByTestId("device-bridge-toggle"));
    expect(control).toHaveTextContent("Bridge off");
  });
  ```

- [ ] **Step 2: Run the UI test to verify it fails.**

  Run: `npm --prefix apps/studio run test -- App.test.tsx`

  Expected: test failure because the header control has not been rendered.

- [ ] **Step 3: Add status refresh and toggle behavior.**

  In `App.tsx`, initialize a `DeviceBridgeStatus` as stopped, load the real/fake status in a mount effect, and keep a `deviceBridgeBusy` plus optional error string. The toggle calls start only from stopped; it calls stop from waiting or connected. Both success paths replace status with the backend response; failures retain status and show a local error. Use a small pure label helper so `connected` can include the non-secret profile and negotiated capabilities in a tooltip or concise secondary line.

  Render the control in the existing `.publish-block`; preserve the Build pack affordance. The state indicator receives a phase class (`stopped`, `waiting` or `connected`) and the action button disables while the request is in flight.

- [ ] **Step 4: Add the compact visual states.**

  In `App.css`, reuse the existing header scale. Give stopped a neutral dot, waiting an amber dot and connected the existing cyan glow. Add only bridge-control, bridge-action and bridge-error selectors; do not change the three-column editor layout or make the token space visible at narrow widths.

- [ ] **Step 5: Run focused Studio verification.**

  Run: `npm --prefix apps/studio run lint && npm --prefix apps/studio run test -- App.test.tsx && npm --prefix apps/studio run build`

  Expected: the browser fixture transition passes, all existing editor tests remain green, and TypeScript production build succeeds.

- [ ] **Step 6: Commit the visible control.**

  ```sh
  git add apps/studio/src/App.tsx apps/studio/src/App.css apps/studio/src/App.test.tsx
  git commit -m "feat(studio): show local bridge status"
  ```

### Task 4: Publish the completed M3 boundary and run release verification

**Files:**
- Modify: `README.md`
- Modify: `docs/architecture/rust-tauri.md`
- Modify: `docs/protocols/dev-bridge-v1.md`
- Modify: `docs/plans/m3-studio-device-bridge.md`

**Interfaces:**
- Documents: exactly three lifecycle commands, non-secret status fields and the deferred FakeADB-only reverse coordinator.

- [ ] **Step 1: Update public boundary documentation.**

  State that Studio can start/stop the local listener and show `stopped`, `waiting`, or `connected`; specify that `SessionSnapshot.sessionId`, endpoint URL/port and bearer are not sent to the UI. Update the architecture boundary to name `src-tauri` as lifecycle owner and preserve `lyra-dev-server` as socket/protocol owner. Keep the README’s real ADB/Android adapter statement unchanged except to list the new local controls.

- [ ] **Step 2: Mark this plan’s completed checkboxes.**

  Replace every completed `- [ ]` in this file with `- [x]`; leave no incomplete item for work that was actually shipped.

- [ ] **Step 3: Run the release verification set.**

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

  Expected: every command exits 0, all status payloads are secret-free, no ADB executable is started, and no Android source changes.

- [ ] **Step 4: Commit the documentation and verification record.**

  ```sh
  git add README.md docs/architecture/rust-tauri.md docs/protocols/dev-bridge-v1.md docs/plans/m3-studio-device-bridge.md
  git commit -m "docs(bridge): publish Studio control boundary"
  ```

- [ ] **Step 5: Push, open a pull request and merge after CI.**

  Run: `git push -u origin feature/device-studio`, then create a pull request to `main`. Wait for macOS, Ubuntu and Windows jobs; squash merge only after every required job passes. Fast-forward the main checkout and rerun the relevant merged smoke tests.
