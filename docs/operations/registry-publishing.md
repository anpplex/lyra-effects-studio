# Registry publishing operations

This runbook is for maintainers publishing the signed static Theme Registry consumed by Lyra. Keep the Ed25519 private key outside the repository and never use a local test key for a production APK.

## 1. Create the production signing material offline

Generate one random 32-byte Ed25519 seed on a trusted maintainer machine:

```sh
openssl rand -base64 32 | tr -d '\n' > lyra-registry-private-key.base64
chmod 600 lyra-registry-private-key.base64
```

The publication script accepts this value through `LYRA_REGISTRY_PRIVATE_KEY_BASE64`. A local build writes the corresponding public key to `public-key.txt`; retain that public key as the Android build input and do not upload the private seed.

## 2. Configure GitHub

Enable Pages with the **GitHub Actions** source once per repository. Then add the private seed as a protected Actions secret and keep the official key ID as the repository variable (or use the workflow default):

```sh
gh api --method POST repos/anpplex/lyra-effects-studio/pages -f build_type=workflow
gh secret set LYRA_REGISTRY_PRIVATE_KEY_BASE64 \
  --repo anpplex/lyra-effects-studio \
  < lyra-registry-private-key.base64
gh variable set LYRA_REGISTRY_KEY_ID \
  --repo anpplex/lyra-effects-studio \
  --body lyra-official-v1
```

The `Publish Theme Registry` workflow runs on a `v*` tag or an explicit manual dispatch. It first checks that the protected secret exists, runs the source/license/reproducibility gates, signs the catalog and Pack checksums, verifies the complete site, and deploys `registry-v1.json`, `registry-v1.sig`, `public-key.txt`, and versioned Packs to:

```text
https://anpplex.github.io/lyra-effects-studio/
```

## 3. Inject the public key into Lyra

After a successful publication, compare the published `public-key.txt` with the reviewed offline public key. Build Lyra with the exact base64 value; the default build intentionally keeps online Registry controls disabled:

```sh
./gradlew \
  -PlyraRegistryPublicKeyBase64="$(tr -d '\n' < public-key.txt)" \
  :app:assembleRelease
```

The Android client pins the origin, verifies the catalog and Pack signatures, and only starts network work after the user presses **刷新目录**. Do not change the origin or key through remote configuration.

## 4. Rotate the key

Key rotation is a coordinated release:

1. Generate a new seed offline and record the new public key through the normal review process.
2. Update the protected GitHub secret and `LYRA_REGISTRY_KEY_ID` variable.
3. Publish a new Registry site and verify `public-key.txt` plus signatures.
4. Ship an Android build containing the new public key before removing the old distribution.

Never commit either private seed, a production `public-key.txt` copied into the Android source tree, or generated Registry archives.
