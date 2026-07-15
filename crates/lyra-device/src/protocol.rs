use std::collections::BTreeMap;
use std::fmt;

use semver::Version;
use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};
use serde_json::Value;
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, PartialEq)]
#[error("{code}: {message}")]
pub struct DeviceDiagnostic {
    pub code: String,
    pub message: String,
}

impl DeviceDiagnostic {
    fn invalid(message: impl Into<String>) -> Self {
        Self {
            code: "device.protocol.invalid".into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct ProtocolVersion(Version);

impl fmt::Display for ProtocolVersion {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Serialize for ProtocolVersion {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ProtocolVersion {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let source = String::deserialize(deserializer)?;
        Version::parse(&source).map(Self).map_err(D::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub struct Capability(String);

impl fmt::Display for Capability {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceHello {
    #[serde(rename = "type")]
    pub message_type: String,
    pub protocol_version: ProtocolVersion,
    pub runtime_version: ProtocolVersion,
    pub device_profile_id: String,
    pub capabilities: Vec<Capability>,
    #[serde(flatten)]
    pub additional: BTreeMap<String, Value>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RawDeviceHello {
    #[serde(rename = "type")]
    message_type: String,
    protocol_version: ProtocolVersion,
    runtime_version: ProtocolVersion,
    device_profile_id: String,
    capabilities: Vec<Capability>,
    #[serde(flatten)]
    additional: BTreeMap<String, Value>,
}

impl DeviceHello {
    /// Decodes and validates a Dev Bridge hello message.
    ///
    /// # Errors
    ///
    /// Returns a stable `device.protocol.invalid` diagnostic for malformed input.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, DeviceDiagnostic> {
        let mut raw: RawDeviceHello = serde_json::from_slice(bytes)
            .map_err(|error| DeviceDiagnostic::invalid(error.to_string()))?;
        if raw.message_type != "hello" {
            return Err(DeviceDiagnostic::invalid("message type must be hello"));
        }
        if raw.device_profile_id.trim().is_empty() {
            return Err(DeviceDiagnostic::invalid("deviceProfileId cannot be empty"));
        }
        if raw.capabilities.iter().any(|item| item.0.is_empty()) {
            return Err(DeviceDiagnostic::invalid("capabilities cannot be empty"));
        }
        raw.capabilities.sort();
        raw.capabilities.dedup();
        Ok(Self {
            message_type: raw.message_type,
            protocol_version: raw.protocol_version,
            runtime_version: raw.runtime_version,
            device_profile_id: raw.device_profile_id,
            capabilities: raw.capabilities,
            additional: raw.additional,
        })
    }
}
