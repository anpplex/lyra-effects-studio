# Contributing

Thank you for improving Lyra Effects Studio.

1. Open an issue for protocol or product-level changes.
2. Create a focused branch and include tests with code changes.
3. Run `npm run studio:lint`, `npm run studio:test`, `npm run studio:build`, `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`, `cargo test --workspace`, and `cargo build --workspace --release` before opening a pull request.
4. Preserve upstream licenses and notices for every Theme Pack contribution.

Theme Pack pull requests must also provide:

- An upstream repository URL and immutable 40-character commit SHA.
- A recognized SPDX license and the exact license text from that revision.
- Required attribution in `NOTICE` and machine-readable evidence in `upstream.json`.
- A byte-identical adapted CSS source with its lowercase SHA-256 in the audit record.
- A passing `lyra-effects validate` result with no scripts, executable files, traversal, escaping symlinks, or budget violations.

Do not submit a Theme merely because it is publicly visible on GitHub. A repository with no license remains fully copyrighted and cannot enter the downloadable Registry.

By contributing, you agree that your code contribution is licensed under Apache-2.0. Theme assets may use another compatible license when clearly declared in their manifest and Pack directory.

Pull requests run the portable Rust core, CLI and frontend on macOS, Windows and Linux. The macOS release gate additionally validates Registry reproducibility, every included Pack and a Tauri `.app` bundle. Platform-specific failures must be fixed in the shared implementation or isolated behind an explicit target boundary; do not skip an operating system silently.
