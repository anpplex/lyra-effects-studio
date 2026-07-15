use lyra_device::{
    DEV_BRIDGE_REMOTE_PORT, DevBridgeReverseCoordinator, DevBridgeReverseRequest, FakeAdb,
    LocalPort,
};

fn request(port: u16) -> DevBridgeReverseRequest {
    DevBridgeReverseRequest::new(LocalPort::new(port).expect("valid port"))
}

fn fake(source: &str) -> FakeAdb {
    FakeAdb::from_slice(source.as_bytes()).expect("valid transcript")
}

#[test]
fn coordinates_one_ready_device_and_removes_mapping() {
    let mut adb = FakeAdb::from_slice(include_bytes!(
        "../../../Fixtures/Device/adb-reverse-single-device.json"
    ))
    .expect("valid transcript");

    let mapping = DevBridgeReverseCoordinator::establish(&mut adb, request(49_321))
        .expect("one ready device maps successfully");

    assert_eq!(mapping.serial().to_string(), "AVATR-CLUSTER-01");
    assert_eq!(mapping.local_port().get(), 49_321);
    assert_eq!(mapping.remote_port(), DEV_BRIDGE_REMOTE_PORT);

    mapping.remove(&mut adb).expect("mapping cleanup");
    adb.assert_finished().expect("all calls consumed");
}

#[test]
fn refuses_not_ready_or_ambiguous_device_selection_without_reverse() {
    for (name, source, code) in [
        (
            "no transports",
            r#"{"steps":[{"operation":"listDevices","result":{"devices":[]}}]}"#,
            "device.adb.noEligibleDevice",
        ),
        (
            "offline and unauthorized transports",
            r#"{"steps":[{"operation":"listDevices","result":{"devices":[{"serial":"AVATR-OFFLINE","state":"offline"},{"serial":"AVATR-LOCKED","state":"unauthorized"}]}}]}"#,
            "device.adb.noEligibleDevice",
        ),
        (
            "multiple ready transports",
            r#"{"steps":[{"operation":"listDevices","result":{"devices":[{"serial":"AVATR-01","state":"device"},{"serial":"AVATR-02","state":"device"}]}}]}"#,
            "device.adb.multipleEligibleDevices",
        ),
    ] {
        let mut adb = fake(source);

        let error =
            DevBridgeReverseCoordinator::establish(&mut adb, request(49_321)).expect_err(name);

        assert_eq!(error.code, code, "{name}");
        adb.assert_finished().expect(name);
    }
}

#[test]
fn preserves_reverse_failure_without_attempting_cleanup() {
    let mut adb = fake(
        r#"{"steps":[{"operation":"listDevices","result":{"devices":[{"serial":"AVATR-01","state":"device"}]}},{"operation":"reverse","serial":"AVATR-01","localPort":49321,"remotePort":49321,"result":{"error":{"code":"device.adb.reverseFailed","message":"reverse unavailable"}}}]}"#,
    );

    let error = DevBridgeReverseCoordinator::establish(&mut adb, request(49_321))
        .expect_err("configured reverse failure");

    assert_eq!(error.code, "device.adb.reverseFailed");
    adb.assert_finished()
        .expect("no cleanup after failed reverse");
}

#[test]
fn preserves_remove_reverse_failure_for_retry() {
    let mut adb = fake(
        r#"{"steps":[{"operation":"listDevices","result":{"devices":[{"serial":"AVATR-01","state":"device"}]}},{"operation":"reverse","serial":"AVATR-01","localPort":49321,"remotePort":49321,"result":{}},{"operation":"removeReverse","serial":"AVATR-01","remotePort":49321,"result":{"error":{"code":"device.adb.removeReverseFailed","message":"cleanup unavailable"}}}]}"#,
    );
    let mapping = DevBridgeReverseCoordinator::establish(&mut adb, request(49_321))
        .expect("successful mapping");

    let error = mapping
        .remove(&mut adb)
        .expect_err("configured cleanup failure");

    assert_eq!(error.code, "device.adb.removeReverseFailed");
    assert_eq!(mapping.serial().to_string(), "AVATR-01");
    adb.assert_finished().expect("all calls consumed");
}
