# Registry reproducibility and signatures

Theme Pack archives are fully reproducible: source order, timestamps, permissions and ZIP metadata are normalized, so the same source produces the same bytes and SHA-256.

Unsigned Catalog semantics are also reproducible. The reproducibility gate compares canonical Catalogs after removing Pack signature values, then independently verifies every generated Pack and Catalog signature.

The current Rust signer uses `ed25519-dalek` and produces stable standard Ed25519 signatures for a fixed key and message. The verifier also accepts the committed signatures created by Apple CryptoKit, so migration does not change the public key or detached-signature wire format. Reproducibility never relies on signature byte equality alone; all of these must remain true:

- The corresponding unsigned bytes, SHA-256 and public key are unchanged.
- The detached Catalog signature verifies over canonical `registry-v1.json`.
- Every Pack signature verifies over the lowercase SHA-256 string.
- Each Pack ZIP remains byte-identical across repeated builds by the current canonical Rust encoder.

The pre-release Swift encoder and the Rust encoder use different valid ZIP compression/metadata encodings, so their archive bytes are not compared directly. No public Registry version was released with the Swift encoder. File paths, uncompressed contents, validation results, Catalog semantics and signature verification are the migration parity boundary; the Rust encoder is canonical from V0.1 onward.

The publication workflow never exposes the production private key to pull requests. It receives the raw Ed25519 key only from a protected GitHub Actions secret and deletes the temporary key file when the build exits.
