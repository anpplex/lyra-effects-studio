use lyra_project::{
    ChangeAccumulator, DeviceProfile, FileEvent, ParameterSchema, PreviewScenario, ProjectDetector,
    ProjectMode, patch_css_variable,
};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

#[test]
fn css_patcher_replaces_only_the_requested_value() {
    let source = "/* keep */\n:root {\n  --lyra-size: 42px; /* comment */\n  --other: red;\n}\n";

    let result = patch_css_variable(source, "--lyra-size", "56px").expect("patch variable");

    assert_eq!(
        result,
        "/* keep */\n:root {\n  --lyra-size: 56px; /* comment */\n  --other: red;\n}\n"
    );
}

#[test]
fn css_patcher_inserts_missing_variable_without_reformatting() {
    let source = ":root {\n  color: white;\n}\nbody { margin: 0; }\n";

    let result = patch_css_variable(source, "--lyra-accent", "#fff").expect("insert variable");

    assert_eq!(
        result,
        ":root {\n  --lyra-accent: #fff;\n  color: white;\n}\nbody { margin: 0; }\n"
    );
}

#[test]
fn css_patcher_rejects_unsafe_names_and_values() {
    assert!(patch_css_variable(":root {}", "color", "red").is_err());
    assert!(patch_css_variable(":root {}", "--safe", "red; } body {").is_err());
}

#[test]
fn built_in_avatr_profile_is_valid() {
    let profile = DeviceProfile::built_in_avatr_star_ring();

    assert_eq!(profile.id, "com.avatr.cluster.4032x284");
    assert_eq!(profile.canvas.physical_width, 4032);
    assert_eq!(profile.canvas.physical_height, 284);
    assert_eq!(profile.safe_area.left, 64);
    assert!(profile.capabilities.iter().any(|item| item == "devBridge"));
    assert!(profile.validate().is_empty());
}

#[test]
fn parameter_schema_reports_duplicate_ids_bad_bindings_and_bounds() {
    let source = br#"{"schemaVersion":1,"groups":[{"id":"one","label":"One","parameters":[{"id":"size","label":"Size","control":"number","binding":{"cssVariable":"not-a-variable"},"defaultValue":120,"minimum":20,"maximum":96},{"id":"size","label":"Size 2","control":"number","binding":{"cssVariable":"--size-2"},"defaultValue":42}]}]}"#;
    let schema = ParameterSchema::from_slice(source).expect("parameter schema");
    let codes = diagnostic_codes(schema.validate());

    assert!(codes.contains(&"parameter.idDuplicate".into()));
    assert!(codes.contains(&"binding.cssVariableInvalid".into()));
    assert!(codes.contains(&"parameter.defaultOutOfRange".into()));
}

#[test]
fn default_scenario_has_no_remote_assets_or_timing_errors() {
    let bytes = fs::read(repository_root().join("Fixtures/Scenarios/default-song.json"))
        .expect("scenario fixture");
    let scenario = PreviewScenario::from_slice(&bytes).expect("scenario");

    assert_eq!(scenario.track.title, "Lyra Sample");
    assert!(!scenario.lyrics.is_empty());
    assert!(scenario.validate().is_empty());
}

#[test]
fn scenario_rejects_remote_artwork_and_invalid_timing() {
    let source = br#"{"schemaVersion":1,"id":"invalid","track":{"title":"Invalid","artist":"Test","artwork":"https://example.com/art.jpg"},"lyrics":[{"startMilliseconds":2000,"endMilliseconds":1000,"text":"bad"}],"events":[]}"#;
    let scenario = PreviewScenario::from_slice(source).expect("scenario");
    let codes = diagnostic_codes(scenario.validate());

    assert!(codes.contains(&"scenario.remoteAssetForbidden".into()));
    assert!(codes.contains(&"scenario.lyricTimingInvalid".into()));
}

#[test]
fn detects_repo_bound_project_from_nested_folder() {
    let root = TempDir::new().expect("project root");
    let nested = root.path().join("lyric-effects/packs/better-lyrics");
    fs::create_dir_all(&nested).expect("nested folder");

    let descriptor = ProjectDetector.detect(&nested).expect("detect project");

    assert_eq!(descriptor.mode, ProjectMode::RepoBound);
    let canonical_root = root.path().canonicalize().expect("canonical root");
    assert_eq!(descriptor.root, canonical_root);
    assert_eq!(
        descriptor.effects_root,
        canonical_root.join("lyric-effects")
    );
}

#[test]
fn detects_standalone_pack_and_rejects_unrecognized_root() {
    let root = TempDir::new().expect("project root");
    fs::write(root.path().join("lyra-pack.json"), "{}").expect("manifest marker");
    let descriptor = ProjectDetector
        .detect(root.path())
        .expect("standalone project");
    assert_eq!(descriptor.mode, ProjectMode::Standalone);
    assert_eq!(
        descriptor.effects_root,
        root.path().canonicalize().expect("canonical root")
    );

    assert!(ProjectDetector.detect(Path::new("/")).is_err());
}

#[test]
fn change_accumulator_debounces_coalesces_sorts_and_ignores_generated_paths() {
    let root = PathBuf::from("/tmp/lyra-project");
    let mut changes = ChangeAccumulator::new(root.clone(), 0.25);
    changes.ingest([
        FileEvent::new(root.join("theme/b.css"), 1.0),
        FileEvent::new(root.join("theme/a.css"), 1.1),
        FileEvent::new(root.join("theme/a.css"), 1.2),
        FileEvent::new(root.join(".build/output"), 1.2),
        FileEvent::new(root.join(".git/index"), 1.2),
        FileEvent::new(root.join("Registry/Site/registry-v1.json"), 1.2),
    ]);

    assert!(changes.poll(1.3).is_empty());
    assert_eq!(
        changes.poll(1.5),
        vec![root.join("theme/a.css"), root.join("theme/b.css")]
    );
}

fn diagnostic_codes(diagnostics: Vec<lyra_project::Diagnostic>) -> Vec<String> {
    diagnostics.into_iter().map(|item| item.code).collect()
}

fn repository_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}
