use lyra_device::DeviceHello;

#[test]
fn decodes_v1_hello_and_preserves_unknown_fields() {
    let hello =
        DeviceHello::from_slice(include_bytes!("../../../Fixtures/Device/hello-valid.json"))
            .expect("valid hello");

    assert_eq!(hello.protocol_version.to_string(), "1.0.0");
    assert_eq!(hello.runtime_version.to_string(), "0.11.20-lyricify");
    assert_eq!(hello.device_profile_id, "com.avatr.cluster.4032x284");
    assert_eq!(
        hello
            .capabilities
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["activate", "console", "stageRevision"]
    );
    assert_eq!(hello.additional["vendorBuild"], 709);
}

#[test]
fn rejects_invalid_protocol_versions_with_a_stable_code() {
    let diagnostic = DeviceHello::from_slice(
        br#"{"type":"hello","protocolVersion":"v1","runtimeVersion":"0.11.20","deviceProfileId":"com.avatr.cluster.4032x284","capabilities":[]}"#,
    )
    .expect_err("invalid version");

    assert_eq!(diagnostic.code, "device.protocol.invalid");
}
