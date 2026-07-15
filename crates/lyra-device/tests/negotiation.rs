use lyra_device::{DeviceHello, HostPolicy, negotiate};

fn policy(required: &[&str]) -> HostPolicy {
    HostPolicy::new(
        "1.2.0",
        ["activate", "fps", "stageRevision"],
        required.iter().copied(),
    )
    .expect("host policy")
}

#[test]
fn negotiates_a_sorted_capability_intersection() {
    let hello =
        DeviceHello::from_slice(include_bytes!("../../../Fixtures/Device/hello-valid.json"))
            .expect("hello");

    let session = negotiate(&hello, &policy(&["stageRevision"])).expect("compatible");

    assert_eq!(session.protocol_version.to_string(), "1.0.0");
    assert_eq!(
        session
            .capabilities
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["activate", "stageRevision"]
    );
}

#[test]
fn rejects_an_incompatible_protocol_major() {
    let hello = DeviceHello::from_slice(include_bytes!(
        "../../../Fixtures/Device/hello-incompatible.json"
    ))
    .expect("well-formed hello");

    let diagnostic = negotiate(&hello, &policy(&[])).expect_err("major mismatch");

    assert_eq!(diagnostic.code, "device.protocol.incompatible");
}

#[test]
fn rejects_a_missing_required_capability() {
    let hello =
        DeviceHello::from_slice(include_bytes!("../../../Fixtures/Device/hello-valid.json"))
            .expect("hello");

    let diagnostic = negotiate(&hello, &policy(&["fps"])).expect_err("missing fps");

    assert_eq!(diagnostic.code, "device.capability.missing");
    assert!(diagnostic.message.contains("fps"));
}
