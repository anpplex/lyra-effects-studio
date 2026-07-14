# Lyra Effects Studio

Lyra Effects Studio is a native macOS editor, previewer, device debugger, and signed Theme Registry toolchain for [Lyra](https://github.com/anpplex/Lyra) lyric effects.

The project is in active development. Its first milestone establishes the public Pack and Registry contracts before the visual editor and Android bridge are added.

## What is included

- Native SwiftUI macOS application foundation.
- Open, versioned Pack, parameter, scenario, Device Profile and Registry contracts.
- Deterministic Theme Pack builder and canonical JSON encoder.
- Ed25519-signed static Theme Registry designed for direct consumption by the Lyra APK.
- CLI workflows for validation, packaging, license audits, Registry publication and verification.
- Three license-cleared Lyra-adapted themes; the complete 18-theme audit remains machine-readable.

## Requirements

- macOS 14 or later
- Xcode 26 or a compatible Swift 6.2 toolchain

## Build

```sh
swift test
swift build --product LyraEffectsStudio
swift run lyra-effects --version
```

## CLI

```sh
swift run lyra-effects validate Fixtures/Packs/valid-theme
swift run lyra-effects pack Fixtures/Packs/valid-theme /tmp/sample.lyra-pack.zip
swift run lyra-effects registry verify \
  Fixtures/Registry/registry-v1.json \
  Fixtures/Registry/registry-v1.sig \
  Fixtures/Registry/public-key.txt
swift run lyra-effects license-audit Registry
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
swift run lyra-effects registry verify-site /tmp/lyra-registry
```

See [Registry/README.md](Registry/README.md) for contribution evidence and [docs/security/reproducibility.md](docs/security/reproducibility.md) for the signed reproducibility model.

## License

The application and SDK are licensed under Apache-2.0. Theme Packs retain their own licenses and notices; inclusion never relicenses an upstream Theme as Apache-2.0.
