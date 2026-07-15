# M3 Fake-first Device Core Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a portable, fully fake-tested Dev Bridge protocol, revision state machine and typed ADB abstraction without invoking real ADB or changing Android.

**Architecture:** A new `lyra-device` Rust crate owns protocol and device semantics. JSON fixtures are shared black-box inputs; process execution, sockets, Tauri commands and Android code remain outside this plan.

**Tech Stack:** Rust 1.97, serde/serde_json, semver, thiserror, async-free injected traits, Cargo tests.

## Global Constraints

- No real `adb` process, network listener, Tauri command or Android source change.
- No arbitrary shell-command API; only typed operations with validated arguments.
- Stable failures use `device.*` codes and do not require parsing English messages.
- Protocol major `1` is required; minor differences use capability intersection.
- Every task follows red-green-refactor and ends with one Conventional Commit.

---

### Task 1: Crate and protocol contracts

**Files:**
- Modify: `Cargo.toml`
- Create: `crates/lyra-device/Cargo.toml`
- Create: `crates/lyra-device/src/lib.rs`
- Create: `crates/lyra-device/src/protocol.rs`
- Create: `crates/lyra-device/tests/protocol.rs`
- Create: `Fixtures/Device/hello-valid.json`
- Create: `Fixtures/Device/hello-incompatible.json`

**Interfaces:**
- Produces: `DeviceHello::from_slice(&[u8])`, `ProtocolVersion`, `Capability`, `DeviceDiagnostic`.

- [ ] Write tests that decode the accepted hello fixture, retain unknown fields, and reject an invalid protocol version with `device.protocol.invalid`.
- [ ] Run `cargo test -p lyra-device --test protocol`; expect failure because the crate does not exist.
- [ ] Add the workspace member and serde models. `DeviceHello` contains `protocol_version`, `runtime_version`, `device_profile_id`, `capabilities` and flattened additional fields.
- [ ] Run `cargo fmt --check && cargo test -p lyra-device --test protocol && cargo clippy -p lyra-device --all-targets -- -D warnings`; expect success.
- [ ] Commit with `feat(device): add Dev Bridge protocol contracts`.

### Task 2: Handshake negotiation

**Files:**
- Modify: `crates/lyra-device/src/protocol.rs`
- Create: `crates/lyra-device/tests/negotiation.rs`

**Interfaces:**
- Produces: `negotiate(&DeviceHello, &HostPolicy) -> Result<NegotiatedSession, DeviceDiagnostic>`.

- [ ] Write tests for compatible hello, capability intersection, major-version rejection (`device.protocol.incompatible`) and missing required capability (`device.capability.missing`).
- [ ] Run `cargo test -p lyra-device --test negotiation`; expect missing-symbol failure.
- [ ] Implement deterministic capability sorting and exact major-version comparison. Do not silently add host-only capabilities.
- [ ] Run the focused tests, full crate tests and Clippy; expect success.
- [ ] Commit with `feat(device): negotiate bridge capabilities`.

### Task 3: Revision state machine

**Files:**
- Create: `crates/lyra-device/src/revision.rs`
- Modify: `crates/lyra-device/src/lib.rs`
- Create: `crates/lyra-device/tests/revision.rs`

**Interfaces:**
- Produces: `RevisionId::from_sha256`, `RevisionState`, `RevisionEvent`, `RevisionMachine::apply`.

- [ ] Write table-driven tests for validate, stage, reject, activate, ready, supersede, fail, rollback and restore; include forbidden activate-before-stage and rollback-without-ready cases.
- [ ] Run `cargo test -p lyra-device --test revision`; expect missing-module failure.
- [ ] Implement the explicit transition table. Invalid transitions return `device.revision.invalidTransition` and leave state unchanged.
- [ ] Run focused/full tests and Clippy; expect success.
- [ ] Commit with `feat(device): add revision lifecycle state machine`.

### Task 4: Typed ADB abstraction and fake transcript

**Files:**
- Create: `crates/lyra-device/src/adb.rs`
- Create: `crates/lyra-device/src/fake_adb.rs`
- Modify: `crates/lyra-device/src/lib.rs`
- Create: `crates/lyra-device/tests/fake_adb.rs`
- Create: `Fixtures/Device/adb-single-device.json`
- Create: `Fixtures/Device/adb-failures.json`

**Interfaces:**
- Produces: `AdbClient` trait with `list_devices`, `reverse`, `remove_reverse`, and `push`; `FakeAdb::from_transcript`.

- [ ] Write transcript tests for zero/one/multiple/offline devices, reverse failure, push failure, unexpected call order and unsafe serial/path/port rejection.
- [ ] Run `cargo test -p lyra-device --test fake_adb`; expect missing-module failure.
- [ ] Implement typed values (`DeviceSerial`, `LocalPort`, `RemotePort`, `DevicePath`) and ordered fake results. No method accepts raw command text.
- [ ] Run focused/full tests and Clippy; expect success.
- [ ] Commit with `feat(device): add typed FakeADB transport`.

### Task 5: Cross-platform gate and public protocol documentation

**Files:**
- Modify: `.github/workflows/ci.yml`
- Modify: `README.md`
- Create: `docs/protocols/dev-bridge-v1.md`
- Modify: `docs/architecture/rust-tauri.md`

**Interfaces:**
- Consumes: all `lyra-device` public contracts and fixtures.

- [ ] Add `lyra-device` to the Windows/Linux package list and document the hello, negotiation, revision and ADB boundaries.
- [ ] Run `npm run studio:lint && npm run studio:test && npm run studio:build`.
- [ ] Run `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings && cargo test --workspace && cargo build --workspace --release`.
- [ ] Run `target/release/lyra-effects license-audit Registry && bash Scripts/verify-reproducible.sh`; expect zero diagnostics and reproducibility success.
- [ ] Commit with `docs(device): publish fake-first bridge contract` and open a PR; merge only after macOS, Windows and Linux jobs pass.
