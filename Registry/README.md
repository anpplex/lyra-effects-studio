# Official Theme Registry sources

This directory contains source inputs for the signed Lyra Theme Registry. A Pack is publishable only when its upstream source, immutable revision, SPDX license, license text, notice, and adapted CSS checksum all pass the same automated checks used for community pull requests.

The initial audit covers all 18 themes in Lyra's Better Lyrics catalog at Lyra revision `6fa5545e0f403ba2e9d4987847ec83753b12e00c`:

- 3 MIT-licensed themes are included as source Packs.
- 15 themes are excluded because no license evidence was present at the audited upstream revision.

An exclusion is not a quality judgment. Maintainers can include a theme in a later Registry version after its author adds a compatible license and the evidence is re-audited.

Run the gates locally:

```sh
bash Scripts/import-lyra-themes.sh --check
swift run lyra-effects license-audit Registry
for pack in Registry/Packs/*; do swift run lyra-effects validate "$pack"; done
```

`license-audit.json` is the machine-readable source of truth. Generated archives and the GitHub Pages site are not committed here.
