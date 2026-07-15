# M3 Authenticated Loopback Dev Server Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (- [ ]) syntax for tracking.

**Goal:** Provide a cross-platform authenticated loopback Dev Bridge server that accepts one Lyra runtime hello session without real ADB, Android, Tauri or Studio UI integration.

**Architecture:** Add lyra-dev-server beside the portable lyra-device core. The crate binds only 127.0.0.1:0, creates an opaque 256-bit bearer token from the OS random source, decodes existing DeviceHello JSON and negotiates it through lyra-device. Tokio/Axum own HTTP lifecycle; lyra-device remains free of HTTP and UI dependencies.

**Tech Stack:** Rust 1.97, Axum 0.8.9, Tokio 1.52.3, getrandom 0.4.3, serde/serde_json and existing lyra-device.

## Global Constraints

- Bind only IPv4 loopback (127.0.0.1) and an ephemeral port; never accept a configurable host or port.
- Generate a fresh 32-byte OS-random bearer token for every server start; never serialize, log or Debug-print it.
- Accept only POST /v1/hello with JSON under 16 KiB and Authorization: Bearer <token>.
- Preserve device.protocol.* and device.capability.* diagnostics from lyra-device; server-local failures use device.bridge.*.
- Keep exactly one profile session: same-profile hello is idempotent, another profile yields HTTP 409 and preserves the existing session.
- No ADB process, Android source, Tauri command, Studio UI, WebSocket, arbitrary command or filesystem endpoint.
- Every behavior change follows red-green-refactor and ends with one Conventional Commit.

---

### Task 1: Server crate, opaque endpoint and wire diagnostics

**Files:**
- Modify: Cargo.toml
- Create: crates/lyra-dev-server/Cargo.toml
- Create: crates/lyra-dev-server/src/lib.rs
- Create: crates/lyra-dev-server/src/diagnostic.rs
- Create: crates/lyra-dev-server/src/token.rs

**Interfaces:**
- Produces ServerDiagnostic { code, message }, DevServerEndpoint and private BridgeToken.
- DevServerEndpoint::address() -> SocketAddr, hello_url() -> String, authorization_value() -> String.

- [ ] **Step 1: Write the failing endpoint contract test.**

~~~rust
#[test]
fn endpoint_uses_loopback_and_redacts_its_token() {
    let endpoint = DevServerEndpoint::new_for_test();
    assert_eq!(endpoint.address().ip().to_string(), "127.0.0.1");
    assert!(endpoint.address().port() > 0);
    assert_eq!(
        endpoint.hello_url(),
        format!("http://{}/v1/hello", endpoint.address())
    );
    assert!(endpoint.authorization_value().starts_with("Bearer "));
    assert!(!format!("{endpoint:?}").contains(&endpoint.authorization_value()));
}
~~~

- [ ] **Step 2: Run the test before creating the crate.**

Run: cargo test -p lyra-dev-server endpoint_uses_loopback_and_redacts_its_token

Expected: failure because package lyra-dev-server does not exist.

- [ ] **Step 3: Add the workspace member and minimum crate.**

Add crates/lyra-dev-server to workspace members. Declare axum = "0.8.9", tokio = { version = "1.52.3", features = ["macros", "net", "rt-multi-thread", "sync"] }, getrandom = "0.4.3", serde, serde_json and local lyra-device. Implement a private 32-byte token filled by getrandom::fill, encoded with a lowercase-hex helper. Implement DevServerEndpoint with a custom redacted Debug implementation; do not derive Debug for the token. Put the quoted test in a cfg(test) module in src/lib.rs, where its private new_for_test constructor is inaccessible to crate consumers.

- [ ] **Step 4: Run the focused contract test.**

Run: cargo fmt --check && cargo test -p lyra-dev-server endpoint_uses_loopback_and_redacts_its_token && cargo clippy -p lyra-dev-server --all-targets -- -D warnings

Expected: the endpoint contract passes with no lint warnings.

- [ ] **Step 5: Commit the independent foundation.**

~~~sh
git add Cargo.toml Cargo.lock crates/lyra-dev-server
git commit -m "feat(server): add loopback endpoint contracts"
~~~

### Task 2: Authenticated hello route

**Files:**
- Modify: crates/lyra-dev-server/src/lib.rs
- Create: crates/lyra-dev-server/src/server.rs
- Create: crates/lyra-dev-server/tests/hello.rs

**Interfaces:**
- Consumes HostPolicy, DeviceHello::from_slice, negotiate and DevServerEndpoint.
- Produces DevServer::start(policy), DevServer::shutdown(self), POST /v1/hello and JSON error envelopes.

- [ ] **Step 1: Write failing real-loopback authentication tests.**

~~~rust
#[tokio::test]
async fn hello_rejects_missing_and_wrong_bearer_tokens() {
    let (server, endpoint) = start_test_server().await;
    assert_response(
        send_hello(&endpoint, None, hello_fixture()).await,
        401,
        "device.bridge.unauthorized",
    );
    assert_response(
        send_hello(&endpoint, Some("Bearer wrong"), hello_fixture()).await,
        401,
        "device.bridge.unauthorized",
    );
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn hello_preserves_the_portable_incompatible_protocol_code() {
    let (server, endpoint) = start_test_server().await;
    let authorization = endpoint.authorization_value();
    assert_response(
        send_hello(
            &endpoint,
            Some(&authorization),
            incompatible_fixture(),
        ).await,
        422,
        "device.protocol.incompatible",
    );
    server.shutdown().await.unwrap();
}
~~~

The helper opens tokio::net::TcpStream to endpoint.address(), writes a minimal HTTP/1.1 request with Connection: close, reads the response to EOF and decodes its JSON. It does not mock Axum or open a LAN socket.

- [ ] **Step 2: Run the authentication tests.**

Run: cargo test -p lyra-dev-server --test hello

Expected: compilation failure because DevServer and the route do not exist.

- [ ] **Step 3: Implement the minimal server lifecycle and route.**

Bind TcpListener with SocketAddr::from(([127, 0, 0, 1], 0)). Build an Axum router with only post(hello) at /v1/hello and DefaultBodyLimit::max(16 * 1024). Compare the exact bearer value before decoding JSON. Decode with DeviceHello::from_slice, call negotiate, and convert errors to a JSON envelope such as:

~~~json
{"code":"device.bridge.unauthorized","message":"valid bearer token required"}
~~~

Use HTTP 401 for authentication, 400 for malformed JSON, 413 for body-limit rejection and 422 for portable negotiation diagnostics. Start axum::serve on a Tokio task with a one-shot graceful shutdown signal.

- [ ] **Step 4: Run focused and crate tests.**

Run: cargo fmt --check && cargo test -p lyra-dev-server && cargo clippy -p lyra-dev-server --all-targets -- -D warnings

Expected: authentication and incompatible-protocol tests pass; no server process survives test completion.

- [ ] **Step 5: Commit the route.**

~~~sh
git add crates/lyra-dev-server
git commit -m "feat(server): authenticate loopback hello sessions"
~~~

### Task 3: Session snapshot and profile ownership

**Files:**
- Modify: crates/lyra-dev-server/src/server.rs
- Modify: crates/lyra-dev-server/src/lib.rs
- Modify: crates/lyra-dev-server/tests/hello.rs

**Interfaces:**
- Produces SessionSnapshot { session_id, device_profile_id, protocol_version, capabilities } and DevServer::session_snapshot(&self).

- [ ] **Step 1: Write failing ownership tests.**

~~~rust
#[tokio::test]
async fn hello_creates_a_snapshot_with_the_negotiated_intersection() {
    let (server, endpoint) = start_test_server().await;
    let accepted = send_authorized_hello(&endpoint, hello_fixture()).await;
    assert_eq!(accepted.status, 200);
    assert_eq!(accepted.json["capabilities"], json!(["activate", "stageRevision"]));
    assert_eq!(
        server.session_snapshot().await.unwrap().device_profile_id,
        "com.avatr.cluster.4032x284",
    );
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn same_profile_reconnect_is_idempotent_but_another_profile_is_rejected() {
    let (server, endpoint) = start_test_server().await;
    let first = send_authorized_hello(&endpoint, hello_fixture()).await;
    let repeat = send_authorized_hello(&endpoint, hello_fixture()).await;
    assert_eq!(first.json["sessionId"], repeat.json["sessionId"]);
    assert_response(
        send_authorized_hello(&endpoint, hello_with_profile("other.profile")).await,
        409,
        "device.bridge.sessionActive",
    );
    server.shutdown().await.unwrap();
}
~~~

- [ ] **Step 2: Run the ownership tests.**

Run: cargo test -p lyra-dev-server --test hello

Expected: failure because a successful hello does not yet retain or return a snapshot.

- [ ] **Step 3: Add the lock-protected one-session store.**

Store Option<SessionSnapshot> in Arc<tokio::sync::RwLock<_>>. On the first accepted hello, generate a separate random 16-byte lowercase-hex session ID and store the profile, selected protocol version and sorted capabilities. For the same profile return the stored snapshot unchanged. For a different profile produce HTTP 409 / device.bridge.sessionActive before mutating state. Derive Serialize for the non-secret snapshot and make its fields readable to future Tauri commands.

- [ ] **Step 4: Run focused/full tests and strict lint.**

Run: cargo fmt --check && cargo test -p lyra-dev-server && cargo clippy -p lyra-dev-server --all-targets -- -D warnings

Expected: all session tests pass and original lyra-device behavior remains unchanged.

- [ ] **Step 5: Commit session ownership.**

~~~sh
git add crates/lyra-dev-server
git commit -m "feat(server): retain authenticated bridge sessions"
~~~

### Task 4: Boundary tests, documentation and CI gate

**Files:**
- Modify: crates/lyra-dev-server/tests/hello.rs
- Modify: .github/workflows/ci.yml
- Modify: README.md
- Modify: docs/architecture/rust-tauri.md
- Modify: docs/protocols/dev-bridge-v1.md
- Modify: docs/plans/m3-dev-server.md

**Interfaces:**
- Consumes final DevServer, DevServerEndpoint, SessionSnapshot and response diagnostics.

- [ ] **Step 1: Write remaining failure-path tests first.**

~~~rust
#[tokio::test]
async fn malformed_or_oversized_hello_never_creates_a_session() {
    let (server, endpoint) = start_test_server().await;
    assert_response(
        send_raw(&endpoint, malformed_request()).await,
        400,
        "device.bridge.invalidRequest",
    );
    assert_response(
        send_raw(&endpoint, body_larger_than(16 * 1024)).await,
        413,
        "device.bridge.invalidRequest",
    );
    assert!(server.session_snapshot().await.is_none());
    server.shutdown().await.unwrap();
}

#[tokio::test]
async fn shutdown_stops_the_loopback_listener() {
    let (server, endpoint) = start_test_server().await;
    let address = endpoint.address();
    server.shutdown().await.unwrap();
    assert!(TcpStream::connect(address).await.is_err());
}
~~~

- [ ] **Step 2: Run the tests and confirm expected failures.**

Run: cargo test -p lyra-dev-server --test hello

Expected: failures for body-size handling, no-session preservation or shutdown semantics.

- [ ] **Step 3: Implement only the tested boundary behavior.**

Map extractor/body errors to device.bridge.invalidRequest, verify rejection does not write the store and await the server task during shutdown. Do not add routes, WebSockets, ADB calls or UI.

- [ ] **Step 4: Update public contracts and CI.**

Add -p lyra-dev-server to the Windows/Linux portable package test command. Document the loopback-only URL, bearer-token provisioning rule, one-profile ownership and non-goals. Mark every completed M3 server-plan checkbox after verification.

- [ ] **Step 5: Run release verification.**

~~~sh
npm run studio:lint
npm run studio:test
npm run studio:build
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --workspace --release
target/release/lyra-effects license-audit Registry
bash Scripts/verify-reproducible.sh
npx tauri build --debug --bundles app
git diff --check
~~~

Expected: all commands exit 0; no real ADB, Android SDK or LAN listener is required.

- [ ] **Step 6: Commit the public server contract.**

~~~sh
git add .github/workflows/ci.yml README.md docs crates/lyra-dev-server
git commit -m "docs(server): publish loopback bridge boundary"
~~~

- [ ] **Step 7: Push, open a PR and merge only after all three CI jobs pass.**

Run: git push -u origin feature/dev-server, then create a PR to main. Wait for Ubuntu, Windows and macOS gates; squash merge only after all pass.
