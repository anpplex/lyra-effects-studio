# Lyra Effects Studio

Lyra Effects Studio is a cross-platform editor, previewer, device debugger, and signed Theme Registry toolchain for [Lyra](https://github.com/anpplex/Lyra) lyric effects. macOS is the first desktop release target; the Rust core and CLI are designed for macOS, Windows, and Linux.

The project is in active development. The public Pack and Registry contracts, Rust CLI, Tauri shell, interactive Studio workspace, schema-driven editing, conflict-safe project persistence, authenticated loopback Dev Bridge lifecycle controls, a portable FakeADB-first reverse coordinator, a user-gated native-chooser ADB preflight, and one explicit removable Dev Bridge reverse mapping are available. Android runtime integration remains a later milestone.

## What is included

- Cross-platform Rust workspace shared by the CLI and Tauri 2 desktop application.
- Three-column Studio workspace with theme navigation, an isolated 4032 × 284 scenario preview, Schema-generated controls, minimal CSS patches, undo/redo and diagnostics.
- Manifest-scoped CSS, HTML and JSON source tabs with find/replace, syntax diagnostics and per-document dirty state.
- Open, versioned Pack, parameter, scenario, Device Profile and Registry contracts.
- Deterministic Theme Pack builder and canonical JSON encoder.
- Ed25519-signed static Theme Registry designed for direct consumption by the Lyra APK.
- CLI workflows for validation, packaging, license audits, Registry publication and verification.
- A portable Dev Bridge protocol, capability negotiation, revision lifecycle, typed FakeADB transcript runner, authenticated loopback hello server and safe reverse coordinator. The coordinator permits only one ready transport to map to the fixed Android port `49321`. Studio can select an ADB executable only through a native picker and, after an explicit **Check devices** action, invoke the separate `lyra-adb` adapter's fixed no-shell `devices -l` operation. With a running bridge and exactly one freshly ready device, the user may then explicitly **Enable mapping** or **Remove mapping**; stopping the bridge also performs that owned cleanup only as part of the user's explicit stop action. The canonical path, serial, port, endpoint and bearer stay in Rust; Studio exposes only safe readiness. There is no automatic discovery or mapping, app-exit cleanup, Pack push, Android change or background ADB retry.
- Three license-cleared Lyra-adapted themes; the complete 18-theme audit remains machine-readable.

## Requirements

- Rust 1.97 through `rustup` (the repository pins the exact toolchain).
- Node.js 24 and npm for the Studio frontend and Tauri CLI.
- Platform tools for desktop builds:
  - macOS 14+: Xcode Command Line Tools; full Xcode is only required for mobile targets.
  - Windows 10+: Microsoft C++ Build Tools and Microsoft Edge WebView2.
  - Debian/Ubuntu Linux: `libwebkit2gtk-4.1-dev build-essential curl wget file libxdo-dev libssl-dev libayatana-appindicator3-dev librsvg2-dev`.

The package list follows the [official Tauri 2 prerequisites](https://v2.tauri.app/start/prerequisites/). Rust core, CLI and frontend checks run on macOS, Windows and Linux in CI. The signed desktop release remains macOS-first; CI currently produces an unsigned macOS debug `.app` as the bundle gate.

## Build

```sh
cargo test --workspace
cargo build --workspace --release
cargo run -p lyra-effects -- --version
```

Build and test the React workspace and Tauri application:

```sh
npm ci
npm ci --prefix apps/studio
npm run studio:lint
npm run studio:test
npm run studio:build
npx tauri build --debug --no-bundle
```

To build the same macOS application bundle used by CI:

```sh
npx tauri build --debug --bundles app
```

For browser-based UI development, run `npm run studio:dev`. Browser mode uses an in-memory fake backend; the Tauri build opens real standalone Packs or repo-bound `lyric-effects` projects through a native directory picker.

Real saves are limited to editable entries declared by the Pack manifest, written through a temporary file and protected by a SHA-256 conflict check. CSS, HTML, parameter JSON and scenario JSON are loaded as separate documents; JSON contracts are validated again before persistence. Theme scripts are never exposed by the source workspace. When a Pack declares `parameters`, Studio validates that Schema and generates color, length/number, select, toggle and text controls without theme-specific application code. See [project filesystem security](docs/security/project-filesystem.md).

Preview content runs in an opaque-origin iframe with a deny-by-default Content Security Policy. Studio injects a nonce-bound read-only Mock Bridge, scales a logical 4032 × 284 canvas to the available viewport, and reports bridge, runtime and policy events back through a token-checked message channel. Editing declared CSS, HTML or scenario JSON refreshes this preview without a save/reopen cycle. See [preview sandbox security](docs/security/preview-sandbox.md).

## CLI

```sh
cargo run -p lyra-effects -- validate Fixtures/Packs/valid-theme
cargo run -p lyra-effects -- pack Fixtures/Packs/valid-theme /tmp/sample.lyra-pack.zip
cargo run -p lyra-effects -- registry verify \
  Fixtures/Registry/registry-v1.json \
  Fixtures/Registry/registry-v1.sig \
  Fixtures/Registry/public-key.txt
cargo run -p lyra-effects -- license-audit Registry
```

Every workflow command emits canonical JSON and returns a non-zero exit code for usage, validation, or trust failures.

## Registry publication

The official static endpoint is designed to be:

```text
https://anpplex.github.io/lyra-effects-studio/registry-v1.json
```

Pull requests run all source, license, security and reproducibility gates without access to production signing material. Tag or manual publication runs receive `LYRA_REGISTRY_PRIVATE_KEY_BASE64` from a protected GitHub Actions secret, create versioned Pack URLs and deploy the signed output to GitHub Pages.

For a local non-production build:

```sh
export LYRA_REGISTRY_PRIVATE_KEY_BASE64="$(openssl rand -base64 32 | tr -d '\n')"
export SOURCE_DATE_EPOCH="$(git show -s --format=%ct HEAD)"
bash Scripts/build-registry.sh /tmp/lyra-registry
cargo run -p lyra-effects -- registry verify-site /tmp/lyra-registry
```

See [Registry/README.md](Registry/README.md) for contribution evidence and [docs/security/reproducibility.md](docs/security/reproducibility.md) for the signed reproducibility model.

See [Rust/Tauri architecture](docs/architecture/rust-tauri.md) for platform boundaries, migration status and the CI support matrix.
See [Dev Bridge v1](docs/protocols/dev-bridge-v1.md) for the fake-first device contract and current integration boundary.

## License

The application and SDK are licensed under Apache-2.0. Theme Packs retain their own licenses and notices; inclusion never relicenses an upstream Theme as Apache-2.0.
