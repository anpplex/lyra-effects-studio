use lyra_pack::SemanticVersion;
use lyra_registry::{
    LicenseAudit, RegistryBuilder, RegistryCatalog, RegistryPackArtifact, RegistrySigner,
    RegistryVerifier,
};
use serde_json::json;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

#[test]
fn catalog_preserves_unknown_catalog_and_pack_fields() {
    let source = br#"{"schemaVersion":1,"registryId":"org.lyra.effects.official","name":"Official","generatedAt":"2026-07-14T00:00:00Z","keyId":"test-key","packs":[{"id":"org.lyra.effects.one","name":"One","family":"better-lyrics","themeId":"sustain","version":"1.0.0","manifestUrl":"packs/org.lyra.effects.one/1.0.0/lyra-pack.json","downloadUrl":"packs/org.lyra.effects.one/1.0.0/pack.lyra-pack.zip","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","signature":"c2ln","size":10,"minimumRuntimeApi":"1.0.0","packFuture":true}],"future":{"kept":true}}"#;

    let catalog = RegistryCatalog::from_slice(source).expect("catalog");
    assert_eq!(catalog.packs[0].theme_id.as_deref(), Some("sustain"));
    let round_trip: serde_json::Value =
        serde_json::from_slice(&catalog.to_canonical_vec().expect("canonical catalog"))
            .expect("round trip");

    assert_eq!(round_trip["future"]["kept"], json!(true));
    assert_eq!(round_trip["packs"][0]["packFuture"], json!(true));
}

#[test]
fn builder_sorts_and_rejects_duplicate_pack_versions() {
    let one = artifact("org.lyra.effects.zed", "1.0.0");
    let two = artifact("org.lyra.effects.alpha", "2.0.0");
    let catalog = RegistryBuilder::build(
        "org.lyra.effects.official",
        "Official",
        "2026-07-14T00:00:00Z",
        "test-key",
        vec![one.clone(), two],
    )
    .expect("build catalog");

    assert_eq!(
        catalog
            .packs
            .iter()
            .map(|pack| pack.id.as_str())
            .collect::<Vec<_>>(),
        vec!["org.lyra.effects.alpha", "org.lyra.effects.zed"]
    );
    assert!(
        RegistryBuilder::build(
            "org.lyra.effects.official",
            "Official",
            "2026-07-14T00:00:00Z",
            "test-key",
            vec![one.clone(), one],
        )
        .is_err()
    );
}

#[test]
fn signs_canonical_catalog_and_rejects_alteration() {
    let signer = RegistrySigner::from_private_key_bytes([7; 32]);
    let catalog = RegistryBuilder::build(
        "org.lyra.effects.official",
        "Official",
        "2026-07-14T00:00:00Z",
        "test-key",
        Vec::new(),
    )
    .expect("catalog");
    let signature = signer.sign_catalog(&catalog).expect("signature");
    let verifier =
        RegistryVerifier::from_public_key_base64(&signer.public_key_base64()).expect("verifier");

    assert!(
        verifier
            .verify_catalog(&catalog, &signature)
            .expect("verify")
    );
    let mut altered = catalog;
    altered.name = "Altered".into();
    assert!(
        !verifier
            .verify_catalog(&altered, &signature)
            .expect("reject alteration")
    );
}

#[test]
fn verifies_pack_checksum_and_detached_signature() {
    let signer = RegistrySigner::from_private_key_bytes([9; 32]);
    let data = b"pack bytes";
    let checksum = lyra_pack::sha256_hex(data);
    let signature = signer.sign_checksum(&checksum);
    let verifier =
        RegistryVerifier::from_public_key_base64(&signer.public_key_base64()).expect("verifier");

    assert!(verifier.verify_pack(data, &checksum, &signature));
    assert!(!verifier.verify_pack(b"altered", &checksum, &signature));
}

#[test]
fn committed_crypto_fixture_verifies() {
    let root = repository_root().join("Fixtures/Registry");
    let catalog = RegistryCatalog::from_slice(
        &fs::read(root.join("registry-v1.json")).expect("catalog fixture"),
    )
    .expect("catalog");
    let signature = read_trimmed(root.join("registry-v1.sig"));
    let public_key = read_trimmed(root.join("public-key.txt"));
    let verifier = RegistryVerifier::from_public_key_base64(&public_key).expect("verifier");

    assert!(
        verifier
            .verify_catalog(&catalog, &signature)
            .expect("fixture verification")
    );
}

#[test]
fn license_audit_accounts_for_all_18_themes_and_verifies_included_sources() {
    let root = repository_root();
    let audit = LicenseAudit::from_slice(
        &fs::read(root.join("Registry/license-audit.json")).expect("license audit"),
    )
    .expect("license audit contract");

    assert_eq!(audit.included.len(), 3);
    assert_eq!(audit.excluded.len(), 15);
    assert!(audit.validate_sources(&root.join("Registry")).is_empty());
}

fn artifact(id: &str, version: &str) -> RegistryPackArtifact {
    RegistryPackArtifact {
        id: id.into(),
        name: id.into(),
        family: "better-lyrics".into(),
        theme_id: None,
        version: SemanticVersion::parse(version).expect("version"),
        manifest_url: format!("packs/{id}/{version}/lyra-pack.json"),
        download_url: format!("packs/{id}/{version}/pack.lyra-pack.zip"),
        sha256: "a".repeat(64),
        signature: "c2ln".into(),
        size: 10,
        minimum_runtime_api: SemanticVersion::parse("1.0.0").expect("runtime"),
        additional: BTreeMap::default(),
    }
}

fn read_trimmed(path: PathBuf) -> String {
    fs::read_to_string(path)
        .expect("text fixture")
        .trim()
        .into()
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
