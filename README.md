# Lyra Effects Studio

Lyra Effects Studio is a cross-platform editor, previewer, device debugger, and signed Theme Registry toolchain for [Lyra](https://github.com/anpplex/Lyra) lyric effects. macOS is the first desktop release target; the Rust core and CLI are designed for macOS, Windows, and Linux.

The project is in active development. The public Pack and Registry contracts, Rust CLI, Tauri shell, interactive Studio workspace, schema-driven editing and conflict-safe CSS project persistence are available; the Android bridge is the next milestone.

## What is included

- Cross-platform Rust workspace shared by the CLI and Tauri 2 desktop application.
- Three-column Studio workspace with theme navigation, a true-ratio 4032 × 284 preview, Schema-generated controls, minimal CSS patches, undo/redo and diagnostics.
- Open, versioned Pack, parameter, scenario, Device Profile and Registry contracts.
- Deterministic Theme Pack builder and canonical JSON encoder.
- Ed25519-signed static Theme Registry designed for direct consumption by the Lyra APK.
- CLI workflows for validation, packaging, license audits, Registry publication and verification.
- Three license-cleared Lyra-adapted themes; the complete 18-theme audit remains machine-readable.

## Requirements

- Rust 1.97 through `rustup` (the repository pins the exact toolchain)
- Xcode Command Line Tools on macOS; the platform linker/build tools on Windows or Linux

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

For browser-based UI development, run `npm run studio:dev`. Browser mode uses an in-memory fake backend; the Tauri build opens real standalone Packs or repo-bound `lyric-effects` projects through a native directory picker.

Real saves are limited to the CSS entry declared by the Pack manifest, written through a temporary file and protected by a SHA-256 conflict check. When a Pack declares `parameters`, Studio validates that Schema and generates color, length/number, select, toggle and text controls without theme-specific application code. See [project filesystem security](docs/security/project-filesystem.md).

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

## License

The application and SDK are licensed under Apache-2.0. Theme Packs retain their own licenses and notices; inclusion never relicenses an upstream Theme as Apache-2.0.
