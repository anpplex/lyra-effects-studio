# Project filesystem boundary

Lyra Effects Studio does not grant its WebView general filesystem access. The native directory dialog exposes only an explicit project path, and the frontend passes that path to a narrow Rust command.

The Rust project command layer applies these rules:

- the chosen path must resolve to a standalone Pack or a repository containing `lyric-effects`;
- Pack manifests are discovered without following directory symlinks;
- an editable CSS file must be the `entry.style` declared by its validated Pack manifest;
- canonical Pack and style paths must remain inside the detected project boundary;
- editable source must be UTF-8 and no larger than 2 MiB;
- every save compares the current source SHA-256 with the hash returned by open;
- a mismatch returns `conflict` and never overwrites the external change;
- successful saves use a temporary file in the destination directory and an atomic persist operation.

The Tauri capability grants the main window `dialog:allow-open`. It does not grant the frontend shell, HTTP, or general filesystem permissions.
