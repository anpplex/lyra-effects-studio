# Rust and Tauri architecture

Lyra Effects Studio uses a portable Rust domain layer and a small Tauri 2 desktop boundary. React and TypeScript own the editor presentation, while ordinary project files remain the only source of truth.

## Boundaries

- `lyra-pack` owns Pack contracts, validation and deterministic archives.
- `lyra-project` owns Device Profiles, scenarios, parameter schemas, project detection and CSS patching.
- `lyra-registry` owns canonical Catalogs and Ed25519 verification.
- `lyra-effects` exposes the portable core as a JSON-speaking CLI.
- `src-tauri` exposes narrowly scoped project commands to the frontend. It does not move validation or Registry trust decisions into JavaScript.
- `apps/studio` owns the cross-platform editor UI and uses a typed fake backend for browser tests.

The Lyra Android application remains Kotlin. Desktop and Android exchange versioned JSON contracts and signed Registry artifacts; no desktop framework code is embedded in the APK.

## Migration status

The original Swift production implementation has been removed after Rust parity tests covered Pack bytes, canonical Catalog behavior, signatures, diagnostics and CLI workflows. Rust is now the sole production implementation for the desktop core and CLI.

## Platform gates

| Gate | macOS | Windows | Linux |
|---|---:|---:|---:|
| Rust core and CLI tests/build | Yes | Yes | Yes |
| Frontend lint/test/build | Yes | Yes | Yes |
| Registry, license and reproducibility audit | Yes | — | — |
| Tauri application bundle | `.app` gate | Planned | Planned |

The cross-platform jobs deliberately compile only the portable crates and CLI. Tauri installers are enabled per platform after their signing, packaging and system-WebView policies are defined. This keeps “core portability” separate from claiming that an unsigned installer is release-ready.
