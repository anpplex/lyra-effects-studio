use base64::Engine as _;
use base64::engine::general_purpose::STANDARD;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use tempfile::TempDir;

#[test]
fn validate_emits_machine_readable_success() {
    let output = run(&[
        "validate",
        path(&repository_root().join("Fixtures/Packs/valid-theme")),
    ]);
    let response = response(&output);

    assert!(output.status.success());
    assert_eq!(response["command"], "validate");
    assert_eq!(response["ok"], true);
    assert_eq!(response["data"]["errorCount"], 0);
}

#[test]
fn pack_creates_deterministic_artifact() {
    let output_root = TempDir::new().expect("output root");
    let archive = output_root.path().join("fixture.lyra-pack.zip");
    let output = run(&[
        "pack",
        path(&repository_root().join("Fixtures/Packs/valid-theme")),
        path(&archive),
    ]);
    let response = response(&output);

    assert!(output.status.success());
    assert!(archive.is_file());
    assert_eq!(response["data"]["sha256"].as_str().expect("sha").len(), 64);
}

#[test]
fn verifies_committed_registry_fixture() {
    let root = repository_root().join("Fixtures/Registry");
    let output = run(&[
        "registry",
        "verify",
        path(&root.join("registry-v1.json")),
        path(&root.join("registry-v1.sig")),
        path(&root.join("public-key.txt")),
    ]);

    assert!(output.status.success());
    assert_eq!(response(&output)["data"]["valid"], true);
}

#[test]
fn builds_and_verifies_registry_artifacts_with_supplied_key() {
    let output_root = TempDir::new().expect("output root");
    let key = output_root.path().join("private-key.txt");
    std::fs::write(&key, format!("{}\n", STANDARD.encode([7_u8; 32]))).expect("private key");
    let fixture = repository_root().join("Fixtures/Registry/registry-v1.json");

    let build = run(&[
        "registry",
        "build",
        path(&fixture),
        path(output_root.path()),
        path(&key),
    ]);
    assert!(
        build.status.success(),
        "{}",
        String::from_utf8_lossy(&build.stdout)
    );
    assert!(output_root.path().join("registry-v1.sig").is_file());
    assert!(output_root.path().join("public-key.txt").is_file());

    let verify = run(&["registry", "verify-site", path(output_root.path())]);
    assert!(
        verify.status.success(),
        "{}",
        String::from_utf8_lossy(&verify.stdout)
    );
}

#[test]
fn registry_build_script_publishes_better_lyrics_theme_ids() {
    let output_root = TempDir::new().expect("output root");
    let site = output_root.path().join("site");
    let output = bash_command()
        .arg(bash_path(
            &repository_root().join("Scripts/build-registry.sh"),
        ))
        .arg(bash_path(&site))
        .env(
            "LYRA_REGISTRY_PRIVATE_KEY_BASE64",
            STANDARD.encode([7_u8; 32]),
        )
        .env("SOURCE_DATE_EPOCH", "1752451200")
        .output()
        .expect("run Registry build script");

    assert!(
        output.status.success(),
        "status={:?}\nstdout={}\nstderr={}",
        output.status,
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let catalog: Value =
        serde_json::from_slice(&std::fs::read(site.join("registry-v1.json")).expect("catalog"))
            .expect("catalog JSON");
    let theme_ids = catalog["packs"]
        .as_array()
        .expect("packs")
        .iter()
        .map(|pack| {
            (
                pack["id"].as_str().expect("pack id"),
                pack["themeId"].as_str().expect("theme id"),
            )
        })
        .collect::<std::collections::BTreeMap<_, _>>();

    assert_eq!(
        theme_ids,
        std::collections::BTreeMap::from([
            ("io.github.better-lyrics.theme-sustain", "sustain",),
            (
                "io.github.chengggit.youtube-music-dynamic-theme",
                "dynamic-background",
            ),
            (
                "io.github.snw-mint.better-lyrics-modern-player",
                "modern-player",
            ),
        ])
    );
}

#[test]
fn registry_build_rejects_better_lyrics_without_a_theme_id() {
    let output_root = TempDir::new().expect("output root");
    let catalog_path = output_root.path().join("registry-v1.unsigned.json");
    let key_path = output_root.path().join("private-key.txt");
    let catalog = serde_json::json!({
        "schemaVersion": 1,
        "registryId": "org.lyra.effects.official",
        "name": "Official",
        "generatedAt": "2026-07-14T00:00:00Z",
        "keyId": "test-key",
        "packs": [{
            "id": "org.lyra.effects.one",
            "name": "One",
            "family": "better-lyrics",
            "version": "1.0.0",
            "manifestUrl": "packs/org.lyra.effects.one/1.0.0/lyra-pack.json",
            "downloadUrl": "packs/org.lyra.effects.one/1.0.0/pack.lyra-pack.zip",
            "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
            "signature": "c2ln",
            "size": 10,
            "minimumRuntimeApi": "1.0.0"
        }]
    });
    std::fs::write(
        &catalog_path,
        serde_json::to_vec(&catalog).expect("catalog JSON"),
    )
    .expect("catalog file");
    std::fs::write(&key_path, format!("{}\n", STANDARD.encode([7_u8; 32]))).expect("private key");

    let output = run(&[
        "registry",
        "build",
        path(&catalog_path),
        path(&output_root.path().join("site")),
        path(&key_path),
    ]);

    assert_eq!(output.status.code(), Some(70));
    assert!(
        response(&output)["message"]
            .as_str()
            .expect("diagnostic")
            .contains("themeId")
    );
}

#[test]
fn usage_errors_are_json_with_exit_code_64() {
    let output = run(&["unknown"]);

    assert_eq!(output.status.code(), Some(64));
    assert_eq!(response(&output)["ok"], false);
}

#[test]
fn audits_registry_licenses() {
    let output = run(&["license-audit", path(&repository_root().join("Registry"))]);
    let response = response(&output);

    assert!(output.status.success());
    assert_eq!(response["data"]["includedCount"], 3);
    assert_eq!(response["data"]["excludedCount"], 15);
}

fn run(arguments: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_lyra-effects"))
        .args(arguments)
        .output()
        .expect("run CLI")
}

fn response(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!(
            "invalid JSON response: {error}; stdout={}",
            String::from_utf8_lossy(&output.stdout)
        )
    })
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn path(path: &Path) -> &str {
    path.to_str().expect("UTF-8 path")
}

#[cfg(windows)]
fn bash_command() -> Command {
    let candidates = [
        Path::new(r"C:\Program Files\Git\bin\bash.exe"),
        Path::new(r"C:\Program Files\Git\usr\bin\bash.exe"),
    ];
    let executable = candidates
        .into_iter()
        .find(|candidate| candidate.is_file())
        .expect("Git for Windows bash.exe");
    Command::new(executable)
}

#[cfg(not(windows))]
fn bash_command() -> Command {
    Command::new("bash")
}

#[cfg(windows)]
fn bash_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(not(windows))]
fn bash_path(path: &Path) -> String {
    path.to_string_lossy().into_owned()
}
