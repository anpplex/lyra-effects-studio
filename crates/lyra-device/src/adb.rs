use std::fmt;
use std::path::Path;

use serde::{Deserialize, Deserializer, Serialize, Serializer, de::Error as _};

use crate::DeviceDiagnostic;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DeviceSerial(String);

impl DeviceSerial {
    /// Validates an ADB device serial before it reaches a process adapter.
    ///
    /// # Errors
    ///
    /// Returns `device.adb.invalidSerial` for empty or command-like values.
    pub fn new(source: &str) -> Result<Self, DeviceDiagnostic> {
        if source.is_empty()
            || source.starts_with('-')
            || !source.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b':' | b'_' | b'-')
            })
        {
            return Err(DeviceDiagnostic::new(
                "device.adb.invalidSerial",
                "ADB serial contains unsupported characters",
            ));
        }
        Ok(Self(source.into()))
    }
}

impl fmt::Display for DeviceSerial {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl Serialize for DeviceSerial {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0)
    }
}

impl<'de> Deserialize<'de> for DeviceSerial {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let source = String::deserialize(deserializer)?;
        Self::new(&source).map_err(D::Error::custom)
    }
}

macro_rules! port_type {
    ($name:ident, $code:literal) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub struct $name(u16);

        impl $name {
            /// Creates a non-zero TCP port.
            ///
            /// # Errors
            ///
            /// Returns a stable diagnostic when the port is zero.
            pub fn new(value: u16) -> Result<Self, DeviceDiagnostic> {
                if value == 0 {
                    Err(DeviceDiagnostic::new($code, "port must be non-zero"))
                } else {
                    Ok(Self(value))
                }
            }

            #[must_use]
            pub const fn get(self) -> u16 {
                self.0
            }
        }
    };
}

port_type!(LocalPort, "device.adb.invalidLocalPort");
port_type!(RemotePort, "device.adb.invalidRemotePort");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DevicePath(String);

impl DevicePath {
    /// Validates an absolute normalized Android destination path.
    ///
    /// # Errors
    ///
    /// Returns `device.adb.invalidDevicePath` for relative or traversing paths.
    pub fn new(source: &str) -> Result<Self, DeviceDiagnostic> {
        if !source.starts_with('/')
            || source.contains('\0')
            || source.split('/').any(|part| matches!(part, "." | ".."))
        {
            return Err(DeviceDiagnostic::new(
                "device.adb.invalidDevicePath",
                "device path must be absolute and normalized",
            ));
        }
        Ok(Self(source.into()))
    }
}

impl fmt::Display for DevicePath {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AdbDeviceState {
    Device,
    Offline,
    Unauthorized,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AdbDevice {
    pub serial: DeviceSerial,
    pub state: AdbDeviceState,
}

pub trait AdbClient {
    /// Lists every ADB transport, including offline and unauthorized devices.
    ///
    /// # Errors
    ///
    /// Returns a stable diagnostic when the transport cannot be queried.
    fn list_devices(&mut self) -> Result<Vec<AdbDevice>, DeviceDiagnostic>;

    /// Creates a fixed TCP reverse mapping for one validated device.
    ///
    /// # Errors
    ///
    /// Returns a stable diagnostic when the mapping cannot be created.
    fn reverse(
        &mut self,
        serial: &DeviceSerial,
        local: LocalPort,
        remote: RemotePort,
    ) -> Result<(), DeviceDiagnostic>;

    /// Removes a previously created reverse mapping.
    ///
    /// # Errors
    ///
    /// Returns a stable diagnostic when the mapping cannot be removed.
    fn remove_reverse(
        &mut self,
        serial: &DeviceSerial,
        remote: RemotePort,
    ) -> Result<(), DeviceDiagnostic>;

    /// Pushes one trusted host file to a validated Android destination.
    ///
    /// # Errors
    ///
    /// Returns a stable diagnostic when the file cannot be transferred.
    fn push(
        &mut self,
        serial: &DeviceSerial,
        local: &Path,
        destination: &DevicePath,
    ) -> Result<(), DeviceDiagnostic>;
}
