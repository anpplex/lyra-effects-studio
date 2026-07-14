use std::path::Path;

#[test]
fn uses_the_audited_registry_sources_in_place() {
    let audit = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Registry/license-audit.json");
    let pack_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Registry/Packs");

    assert!(
        audit.is_file(),
        "missing license audit: {}",
        audit.display()
    );
    assert!(
        pack_root.is_dir(),
        "missing Registry packs: {}",
        pack_root.display()
    );
    assert_eq!(lyra_registry::FORMAT_VERSION, 1);
}
