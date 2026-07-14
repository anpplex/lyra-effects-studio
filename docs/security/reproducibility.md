# Registry reproducibility and signatures

Theme Pack archives are fully reproducible: source order, timestamps, permissions and ZIP metadata are normalized, so the same source produces the same bytes and SHA-256.

Unsigned Catalog semantics are also reproducible. The reproducibility gate compares canonical Catalogs after removing Pack signature values, then independently verifies every generated Pack and Catalog signature.

Signature bytes themselves are intentionally not reproducible when generated with Apple CryptoKit. CryptoKit randomizes Ed25519 signatures for the same key and message as a side-channel defense, as documented by Apple in [`signature(for:)`](https://developer.apple.com/documentation/cryptokit/curve25519/signing/privatekey/signature%28for%3A%29). Different signature bytes are valid only when all of these remain true:

- The corresponding unsigned bytes, SHA-256 and public key are unchanged.
- The detached Catalog signature verifies over canonical `registry-v1.json`.
- Every Pack signature verifies over the lowercase SHA-256 string.
- The Pack ZIP itself remains byte-identical.

The publication workflow never exposes the production private key to pull requests. It receives the raw Ed25519 key only from a protected GitHub Actions secret and deletes the temporary key file when the build exits.
