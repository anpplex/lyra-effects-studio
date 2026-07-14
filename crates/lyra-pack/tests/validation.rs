use lyra_pack::{PackArchiver, PackValidator};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn accepts_minimal_theme_pack() {
    let fixture = make_pack("theme/lyra.css");

    assert!(
        PackValidator::default()
            .validate(fixture.path())
            .expect("validate")
            .is_empty()
    );
}

#[test]
fn rejects_theme_scripts_traversal_missing_files_and_executables() {
    let fixture = make_pack("../outside.css");
    fs::write(fixture.path().join("effect.js"), "alert(1)").expect("script fixture");
    let executable = fixture.path().join("run.sh");
    fs::write(&executable, "#!/bin/sh").expect("executable fixture");
    set_executable(&executable);

    let codes: BTreeSet<_> = PackValidator::default()
        .validate(fixture.path())
        .expect("validate")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();

    assert!(codes.contains("path.traversal"));
    assert!(codes.contains("theme.scriptForbidden"));
    assert!(codes.contains("file.executableForbidden"));
    assert!(codes.contains("entry.missing"));
}

#[test]
fn rejects_script_entry_even_when_the_script_file_is_missing() {
    let fixture = make_pack("theme/lyra.css");
    let manifest = fs::read_to_string(fixture.path().join("lyra-pack.json")).expect("manifest");
    fs::write(
        fixture.path().join("lyra-pack.json"),
        manifest.replace(
            "\"entry\":{\"style\":\"theme/lyra.css\"}",
            "\"entry\":{\"style\":\"theme/lyra.css\",\"script\":\"missing.js\"}",
        ),
    )
    .expect("script manifest");

    let codes: BTreeSet<_> = PackValidator::default()
        .validate(fixture.path())
        .expect("validate")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();

    assert!(codes.contains("theme.scriptForbidden"));
}

#[test]
fn all_license_cleared_registry_packs_validate() {
    let pack_root = repository_root().join("Registry/Packs");
    for entry in fs::read_dir(pack_root).expect("Registry packs") {
        let path = entry.expect("Pack directory").path();
        if path.is_dir() {
            assert!(
                PackValidator::default()
                    .validate(&path)
                    .expect("validate")
                    .is_empty(),
                "{} did not validate",
                path.display()
            );
        }
    }
}

#[test]
fn enforces_configured_file_and_pack_size_budgets() {
    let fixture = make_pack("theme/lyra.css");
    let codes: BTreeSet<_> = PackValidator::with_budgets(4, 16)
        .validate(fixture.path())
        .expect("validate")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();

    assert!(codes.contains("file.tooLarge"));
    assert!(codes.contains("pack.tooLarge"));
}

#[cfg(unix)]
#[test]
fn rejects_symlinks_that_escape_the_pack_root() {
    use std::os::unix::fs::symlink;

    let fixture = make_pack("theme/lyra.css");
    symlink("/tmp/outside.css", fixture.path().join("escape.css")).expect("symlink fixture");

    let codes: BTreeSet<_> = PackValidator::default()
        .validate(fixture.path())
        .expect("validate")
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect();

    assert!(codes.contains("symlink.escapesRoot"));
}

#[test]
fn builds_byte_identical_archives_with_sorted_normalized_entries() {
    let source = repository_root().join("Fixtures/Packs/valid-theme");
    let output = TempDir::new().expect("output directory");
    let first = PackArchiver::default()
        .build(&source, &output.path().join("first.lyra-pack.zip"))
        .expect("first archive");
    let second = PackArchiver::default()
        .build(&source, &output.path().join("second.lyra-pack.zip"))
        .expect("second archive");

    assert_eq!(first.sha256, second.sha256);
    assert_eq!(
        fs::read(&first.path).expect("first bytes"),
        fs::read(&second.path).expect("second bytes")
    );
    assert!(first.byte_count > 0);
    assert_eq!(first.entries, vec!["lyra-pack.json", "theme/lyra.css"]);
}

fn make_pack(style: &str) -> TempDir {
    let root = TempDir::new().expect("pack directory");
    fs::create_dir(root.path().join("theme")).expect("theme directory");
    fs::write(root.path().join("theme/lyra.css"), "body {}").expect("style fixture");
    let manifest = format!(
        r#"{{"schemaVersion":1,"id":"io.github.example.refine","name":"Refine","version":"1.0.0","kind":"theme","family":"better-lyrics","author":{{"name":"Author"}},"license":{{"spdx":"MIT"}},"compatibility":{{"packSchema":">=1 <2","runtimeApi":">=1.0.0 <2.0.0","bridgeApi":">=1.0.0 <2.0.0"}},"entry":{{"style":"{style}"}},"capabilities":["styles"]}}"#
    );
    fs::write(root.path().join("lyra-pack.json"), manifest).expect("manifest fixture");
    root
}

#[cfg(unix)]
fn set_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o755)).expect("executable mode");
}

#[cfg(not(unix))]
fn set_executable(_path: &Path) {}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
