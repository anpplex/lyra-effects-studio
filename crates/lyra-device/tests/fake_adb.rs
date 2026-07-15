use std::path::Path;

use lyra_device::{
    AdbClient, AdbDeviceState, DevicePath, DeviceSerial, FakeAdb, LocalPort, RemotePort,
};

#[test]
fn replays_a_single_device_deploy_transcript() {
    let mut adb = FakeAdb::from_slice(include_bytes!(
        "../../../Fixtures/Device/adb-single-device.json"
    ))
    .expect("transcript");
    let serial = DeviceSerial::new("AVATR-CLUSTER-01").expect("serial");
    let port = LocalPort::new(49_321).expect("local port");
    let remote = RemotePort::new(49_321).expect("remote port");
    let destination = DevicePath::new("/data/local/tmp/lyra/revision.zip").expect("device path");

    let devices = adb.list_devices().expect("devices");
    assert_eq!(devices.len(), 1);
    assert_eq!(devices[0].state, AdbDeviceState::Device);
    adb.reverse(&serial, port, remote).expect("reverse");
    adb.push(&serial, Path::new("/tmp/revision.zip"), &destination)
        .expect("push");
    adb.remove_reverse(&serial, remote).expect("remove reverse");
    adb.assert_finished().expect("all calls consumed");
}

#[test]
fn represents_zero_multiple_offline_and_unauthorized_devices() {
    let mut empty =
        FakeAdb::from_slice(br#"{"steps":[{"operation":"listDevices","result":{"devices":[]}}]}"#)
            .expect("empty transcript");
    assert!(empty.list_devices().expect("empty devices").is_empty());

    let mut adb = FakeAdb::from_slice(include_bytes!("../../../Fixtures/Device/adb-failures.json"))
        .expect("failure transcript");
    let devices = adb.list_devices().expect("devices");

    assert_eq!(devices.len(), 3);
    assert_eq!(devices[1].state, AdbDeviceState::Offline);
    assert_eq!(devices[2].state, AdbDeviceState::Unauthorized);
}

#[test]
fn returns_configured_reverse_and_push_failures() {
    let mut adb = FakeAdb::from_slice(include_bytes!("../../../Fixtures/Device/adb-failures.json"))
        .expect("failure transcript");
    let serial = DeviceSerial::new("AVATR-01").expect("serial");
    let local = LocalPort::new(49_321).expect("local port");
    let remote = RemotePort::new(49_321).expect("remote port");
    let destination = DevicePath::new("/data/local/tmp/lyra/revision.zip").expect("device path");
    adb.list_devices().expect("devices");

    assert_eq!(
        adb.reverse(&serial, local, remote)
            .expect_err("reverse")
            .code,
        "device.adb.reverseFailed"
    );
    assert_eq!(
        adb.push(&serial, Path::new("/tmp/revision.zip"), &destination)
            .expect_err("push")
            .code,
        "device.adb.pushFailed"
    );
}

#[test]
fn rejects_unexpected_calls_and_unsafe_arguments() {
    let mut adb = FakeAdb::from_slice(include_bytes!(
        "../../../Fixtures/Device/adb-single-device.json"
    ))
    .expect("transcript");
    let serial = DeviceSerial::new("AVATR-CLUSTER-01").expect("serial");

    assert_eq!(
        adb.remove_reverse(&serial, RemotePort::new(49_321).expect("port"))
            .expect_err("unexpected call")
            .code,
        "device.fakeAdb.unexpectedCall"
    );
    assert!(DeviceSerial::new("-s injected").is_err());
    assert!(LocalPort::new(0).is_err());
    assert!(RemotePort::new(0).is_err());
    assert!(DevicePath::new("../../data/local/tmp").is_err());
    assert!(DevicePath::new("/data/local/../escape").is_err());
}
