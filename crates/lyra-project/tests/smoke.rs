use std::path::Path;

#[test]
fn uses_the_repository_scenario_schema_as_the_contract_source() {
    let schema =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Schemas/preview-scenario-v1.schema.json");

    assert!(
        schema.is_file(),
        "missing shared schema: {}",
        schema.display()
    );
    assert_eq!(lyra_project::FORMAT_VERSION, 1);
}
