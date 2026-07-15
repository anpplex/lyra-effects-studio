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
    #[must_use]
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    fn invalid(message: impl Into<String>) -> Self {
        Self::new("device.protocol.invalid", message)
    }

    pub(crate) fn with_code(code: &str, message: impl Into<String>) -> Self {
        Self::new(code, message)
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

impl Capability {
    fn new(source: &str) -> Result<Self, DeviceDiagnostic> {
        if source.is_empty()
            || !source
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_'))
        {
            return Err(DeviceDiagnostic::invalid(format!(
                "invalid capability: {source}"
            )));
        }
        Ok(Self(source.into()))
    }
}

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

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HostPolicy {
    pub protocol_version: ProtocolVersion,
    pub capabilities: Vec<Capability>,
    pub required_capabilities: Vec<Capability>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NegotiatedSession {
    pub protocol_version: ProtocolVersion,
    pub capabilities: Vec<Capability>,
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

impl HostPolicy {
    /// Creates a validated host negotiation policy.
    ///
    /// # Errors
    ///
    /// Returns `device.protocol.invalid` for malformed versions or capability IDs.
    pub fn new<'a>(
        protocol_version: &str,
        capabilities: impl IntoIterator<Item = &'a str>,
        required_capabilities: impl IntoIterator<Item = &'a str>,
    ) -> Result<Self, DeviceDiagnostic> {
        let protocol_version = ProtocolVersion(
            Version::parse(protocol_version)
                .map_err(|error| DeviceDiagnostic::invalid(error.to_string()))?,
        );
        let capabilities = collect_capabilities(capabilities)?;
        let required_capabilities = collect_capabilities(required_capabilities)?;
        if required_capabilities
            .iter()
            .any(|required| capabilities.binary_search(required).is_err())
        {
            return Err(DeviceDiagnostic::invalid(
                "required capabilities must be advertised by the host",
            ));
        }
        Ok(Self {
            protocol_version,
            capabilities,
            required_capabilities,
        })
    }
}

/// Negotiates a compatible protocol and capability intersection.
///
/// # Errors
///
/// Returns a stable diagnostic for incompatible majors or required capabilities.
pub fn negotiate(
    hello: &DeviceHello,
    policy: &HostPolicy,
) -> Result<NegotiatedSession, DeviceDiagnostic> {
    if hello.protocol_version.0.major != policy.protocol_version.0.major {
        return Err(DeviceDiagnostic::with_code(
            "device.protocol.incompatible",
            format!(
                "device protocol {} is incompatible with host {}",
                hello.protocol_version, policy.protocol_version
            ),
        ));
    }
    if let Some(missing) = policy
        .required_capabilities
        .iter()
        .find(|required| hello.capabilities.binary_search(required).is_err())
    {
        return Err(DeviceDiagnostic::with_code(
            "device.capability.missing",
            format!("device is missing required capability {missing}"),
        ));
    }
    let capabilities = policy
        .capabilities
        .iter()
        .filter(|candidate| hello.capabilities.binary_search(candidate).is_ok())
        .cloned()
        .collect();
    let protocol_version = if hello.protocol_version <= policy.protocol_version {
        hello.protocol_version.clone()
    } else {
        policy.protocol_version.clone()
    };
    Ok(NegotiatedSession {
        protocol_version,
        capabilities,
    })
}

fn collect_capabilities<'a>(
    values: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<Capability>, DeviceDiagnostic> {
    let mut capabilities = values
        .into_iter()
        .map(Capability::new)
        .collect::<Result<Vec<_>, _>>()?;
    capabilities.sort();
    capabilities.dedup();
    Ok(capabilities)
}
