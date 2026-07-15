#![doc = "Portable Dev Bridge protocol and device semantics for Lyra Effects Studio."]

mod adb;
mod fake_adb;
mod protocol;
mod revision;

pub use adb::{
    AdbClient, AdbDevice, AdbDeviceState, DevicePath, DeviceSerial, LocalPort, RemotePort,
};
pub use fake_adb::FakeAdb;

pub use protocol::{
    Capability, DeviceDiagnostic, DeviceHello, HostPolicy, NegotiatedSession, ProtocolVersion,
    negotiate,
};
pub use revision::{RevisionEvent, RevisionId, RevisionMachine, RevisionState};
