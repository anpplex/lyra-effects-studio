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
