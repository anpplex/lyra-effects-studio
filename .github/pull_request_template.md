## Summary

Describe the focused change and the user-visible or protocol impact.

## Verification

- [ ] `cargo fmt --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace`
- [ ] `cargo build --workspace --release`
- [ ] Pack/Registry validation relevant to this change
- [ ] Reproducibility check when publication output changes

## Theme Pack evidence

- [ ] Upstream repository and immutable commit are recorded
- [ ] SPDX license and exact license text are included
- [ ] NOTICE preserves required attribution
- [ ] Adapted CSS checksum matches the audited source
- [ ] No script, executable, traversal, oversized, or undeclared resource is present
