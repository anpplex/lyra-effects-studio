#![doc = "Portable Dev Bridge protocol and device semantics for Lyra Effects Studio."]

mod protocol;
mod revision;

pub use protocol::{
    Capability, DeviceDiagnostic, DeviceHello, HostPolicy, NegotiatedSession, ProtocolVersion,
    negotiate,
};
pub use revision::{RevisionEvent, RevisionId, RevisionMachine, RevisionState};
