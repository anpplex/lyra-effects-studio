use lyra_pack::{
    CanonicalJson, PackManifest, SemanticVersion, VersionRange, canonical_json, sha256_hex,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

#[derive(Debug, Deserialize, Serialize)]
struct Example {
    b: String,
    a: u64,
}

#[test]
fn canonical_json_sorts_keys_compacts_and_adds_one_newline() {
    let bytes = canonical_json::to_vec(&Example {
        b: "two".into(),
        a: 1,
    })
    .expect("encode canonical JSON");

    assert_eq!(
        bytes,
        br#"{"a":1,"b":"two"}
"#
    );
}

#[test]
fn canonical_json_round_trips_every_value_kind() {
    let source = br#"{"array":[true,null,2.5,"text"],"nested":{"future":7}}"#;
    let value: CanonicalJson = canonical_json::from_slice(source).expect("decode value");
    let encoded = canonical_json::to_vec(&value).expect("encode value");
    let round_trip: CanonicalJson = canonical_json::from_slice(&encoded).expect("round trip");

    assert_eq!(value, round_trip);
    assert_eq!(value["nested"]["future"], json!(7));
}

#[test]
fn canonical_json_rejects_trailing_non_whitespace() {
    assert!(canonical_json::from_slice::<CanonicalJson>(br#"{"valid":true} garbage"#).is_err());
}

#[test]
fn semantic_versions_follow_semver_precedence() {
    let prerelease = SemanticVersion::parse("1.2.3-alpha.1+build.7").expect("prerelease");
    let release = SemanticVersion::parse("1.2.3").expect("release");

    assert_eq!(prerelease.to_string(), "1.2.3-alpha.1+build.7");
    assert!(prerelease < release);
    assert!(SemanticVersion::parse("2.0.0").expect("major version") > release);
}

#[test]
fn semantic_versions_reject_non_semver_inputs() {
    for source in ["1", "1.2", "01.2.3", "1.02.3", "v1.2.3", "1.2.3-"] {
        assert!(SemanticVersion::parse(source).is_err(), "accepted {source}");
    }
}

#[test]
fn version_ranges_accept_schema_and_runtime_forms() {
    let schema = VersionRange::parse(">=1 <2").expect("schema range");
    let runtime = VersionRange::parse(">=1.0.0 <2.0.0").expect("runtime range");

    assert!(schema.contains(&SemanticVersion::parse("1.9.0").expect("version")));
    assert!(!schema.contains(&SemanticVersion::parse("2.0.0").expect("version")));
    assert!(runtime.contains(&SemanticVersion::parse("1.0.0").expect("version")));
    assert!(!runtime.contains(&SemanticVersion::parse("0.9.9").expect("version")));
}

#[test]
fn manifest_preserves_unknown_top_level_and_nested_fields() {
    let source = br#"{"schemaVersion":1,"id":"io.github.example.refine","name":"Refine","version":"1.0.0","kind":"theme","family":"better-lyrics","author":{"name":"Author","future":"kept"},"license":{"spdx":"MIT","notice":"NOTICE"},"compatibility":{"packSchema":">=1 <2","runtimeApi":">=1.0.0 <2.0.0","bridgeApi":">=1.0.0 <2.0.0"},"entry":{"style":"theme/lyra.css","themeId":"refine"},"capabilities":["styles"],"future":{"nested":true}}"#;

    let manifest = PackManifest::from_slice(source).expect("decode manifest");
    let round_trip: CanonicalJson =
        canonical_json::from_slice(&manifest.to_canonical_vec().expect("encode manifest"))
            .expect("decode round trip");

    assert_eq!(manifest.id, "io.github.example.refine");
    assert_eq!(manifest.version.to_string(), "1.0.0");
    assert_eq!(manifest.author.additional["future"], json!("kept"));
    assert_eq!(round_trip["future"]["nested"], json!(true));
    assert_eq!(round_trip["author"]["future"], json!("kept"));
}

#[test]
fn manifest_rejects_unknown_schema_version() {
    let source = br#"{"schemaVersion":2,"id":"io.example.pack","name":"Pack","version":"1.0.0","kind":"theme","family":"better-lyrics","author":{"name":"A"},"license":{"spdx":"MIT"},"compatibility":{"packSchema":">=1 <2","runtimeApi":">=1.0.0 <2.0.0","bridgeApi":">=1.0.0 <2.0.0"},"entry":{"style":"theme.css"},"capabilities":["styles"]}"#;

    assert!(PackManifest::from_slice(source).is_err());
}

#[test]
fn manifest_rejects_invalid_ids() {
    let source = valid_manifest().replace("io.example.pack", "Invalid Pack ID");

    assert!(PackManifest::from_slice(source.as_bytes()).is_err());
}

#[test]
fn manifest_rejects_duplicate_capabilities() {
    let source = valid_manifest().replace("[\"styles\"]", "[\"styles\",\"styles\"]");

    assert!(PackManifest::from_slice(source.as_bytes()).is_err());
}

#[test]
fn manifest_rejects_empty_required_names() {
    let source = valid_manifest().replace("\"name\":\"Pack\"", "\"name\":\"\"");

    assert!(PackManifest::from_slice(source.as_bytes()).is_err());
}

#[test]
fn better_lyrics_manifest_requires_a_safe_theme_id() {
    let missing = valid_manifest().replace(",\"themeId\":\"sample\"", "");
    assert!(PackManifest::from_slice(missing.as_bytes()).is_err());

    assert!(PackManifest::from_slice(valid_manifest().as_bytes()).is_ok());

    let unsafe_id =
        valid_manifest().replace("\"themeId\":\"sample\"", "\"themeId\":\"../sustain\"");
    assert!(PackManifest::from_slice(unsafe_id.as_bytes()).is_err());

    let oversized_id = valid_manifest().replace(
        "\"themeId\":\"sample\"",
        &format!("\"themeId\":\"{}\"", "a".repeat(65)),
    );
    assert!(PackManifest::from_slice(oversized_id.as_bytes()).is_err());
}

#[test]
fn hashes_bytes_as_lowercase_sha256() {
    assert_eq!(
        sha256_hex(b"abc"),
        "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
    );
}

fn valid_manifest() -> String {
    r#"{"schemaVersion":1,"id":"io.example.pack","name":"Pack","version":"1.0.0","kind":"theme","family":"better-lyrics","author":{"name":"A"},"license":{"spdx":"MIT"},"compatibility":{"packSchema":">=1 <2","runtimeApi":">=1.0.0 <2.0.0","bridgeApi":">=1.0.0 <2.0.0"},"entry":{"style":"theme.css","themeId":"sample"},"capabilities":["styles"]}"#.into()
}
