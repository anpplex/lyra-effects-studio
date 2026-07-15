# Security Policy

## Reporting

Do not open a public issue for a vulnerability. Use GitHub private vulnerability reporting for this repository.

Reports should include affected versions, reproduction steps, impact, and any proposed mitigation. Maintainers will acknowledge complete reports as soon as practical.

## Registry trust

Production signing keys are never committed. Clients must verify the detached Registry signature, Pack checksum, and Pack signature before installation.

The publication workflow is the only production signing boundary. Pull-request workflows have read-only repository permissions and never receive the signing secret. Third-party Actions are pinned to full commit SHAs.

The Rust signer emits standard Ed25519 signatures that remain compatible with the committed Apple CryptoKit fixtures. Reproducibility checks compare Pack bytes and canonical Catalog semantics, then independently verify every generated signature. See `docs/security/reproducibility.md`.

Downloaded Packs must be treated as untrusted input even after signature verification. Theme Packs cannot contain scripts or executables; the validator rejects both Unix execute bits and executable/script file extensions so the policy is consistent on Windows. Web Effect Packs will use a separate capability and sandbox review before they are enabled.

Studio's CSS/HTML scenario preview is isolated from the application origin and native APIs. Its threat model and message-channel checks are documented in `docs/security/preview-sandbox.md`.
