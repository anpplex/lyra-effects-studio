# Rust and Tauri architecture

Lyra Effects Studio uses a portable Rust domain layer and a small Tauri 2 desktop boundary. React and TypeScript own the editor presentation, while ordinary project files remain the only source of truth.

## Boundaries

- `lyra-pack` owns Pack contracts, validation and deterministic archives.
- `lyra-project` owns Device Profiles, scenarios, parameter schemas, project detection and CSS patching.
- `lyra-registry` owns canonical Catalogs and Ed25519 verification.
- `lyra-device` owns Dev Bridge hello/negotiation, revision lifecycle semantics and typed ADB boundaries.
- `lyra-dev-server` owns the authenticated IPv4-loopback hello listener and one-profile session ownership. It depends on `lyra-device`; `lyra-device` remains HTTP- and runtime-free.
- `lyra-effects` exposes the portable core as a JSON-speaking CLI.
- `src-tauri` exposes narrowly scoped project commands plus local Dev Bridge lifecycle commands to the frontend. It owns desktop lifetime, not HTTP protocol, validation or Registry trust decisions.
- `apps/studio` owns the cross-platform editor UI and uses a typed fake backend for browser tests.

The Lyra Android application remains Kotlin. Desktop and Android exchange versioned JSON contracts and signed Registry artifacts; no desktop framework code is embedded in the APK.

## Migration status

The original Swift production implementation has been removed after Rust parity tests covered Pack bytes, canonical Catalog behavior, signatures, diagnostics and CLI workflows. Rust is now the sole production implementation for the desktop core and CLI.

M3 adds the portable device domain, an authenticated IPv4-loopback hello server and a narrow Tauri lifecycle boundary. `lyra-dev-server` owns real TCP protocol behavior; `src-tauri` starts or gracefully stops one instance and retains provisioning material privately. The Studio frontend receives only `stopped`, `waiting` or `connected` plus a projection containing device profile, negotiated protocol version and capabilities. It never receives the bearer, endpoint URL/port or server session ID. JSON fixtures continue to drive hello parsing and strict FakeADB transcripts. A later adapter may compose the same typed `AdbClient` operations with the retained endpoint, but arbitrary shell commands remain absent from the domain API.

## Platform gates

| Gate | macOS | Windows | Linux |
|---|---:|---:|---:|
| Rust core and CLI tests/build | Yes | Yes | Yes |
| Frontend lint/test/build | Yes | Yes | Yes |
| Registry, license and reproducibility audit | Yes | — | — |
| Tauri application bundle | `.app` gate | Planned | Planned |

The cross-platform jobs deliberately compile only the portable crates and CLI. Tauri installers are enabled per platform after their signing, packaging and system-WebView policies are defined. This keeps “core portability” separate from claiming that an unsigned installer is release-ready.

Repository text is checked out with LF endings on every operating system. This is part of the reproducibility boundary because audited Theme CSS and packaged source files use byte-level SHA-256 values; platform checkout settings must not rewrite those bytes.
