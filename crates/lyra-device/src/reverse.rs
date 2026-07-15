use crate::{
    AdbClient, AdbDevice, AdbDeviceState, DEV_BRIDGE_REMOTE_PORT, DeviceDiagnostic, DeviceSerial,
    LocalPort, RemotePort,
};

/// Typed request to map a host Dev Bridge listener into one Android runtime.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DevBridgeReverseRequest {
    local_port: LocalPort,
}

impl DevBridgeReverseRequest {
    #[must_use]
    pub const fn new(local_port: LocalPort) -> Self {
        Self { local_port }
    }

    #[must_use]
    pub const fn local_port(self) -> LocalPort {
        self.local_port
    }

    #[must_use]
    pub const fn remote_port(self) -> RemotePort {
        DEV_BRIDGE_REMOTE_PORT
    }
}

/// A successfully created typed ADB reverse mapping.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReverseMapping {
    serial: DeviceSerial,
    local_port: LocalPort,
    remote_port: RemotePort,
}

impl ReverseMapping {
    #[must_use]
    pub fn serial(&self) -> &DeviceSerial {
        &self.serial
    }

    #[must_use]
    pub const fn local_port(&self) -> LocalPort {
        self.local_port
    }

    #[must_use]
    pub const fn remote_port(&self) -> RemotePort {
        self.remote_port
    }

    /// Removes this mapping through the injected ADB adapter.
    ///
    /// # Errors
    ///
    /// Preserves the adapter's stable diagnostic when cleanup fails.
    pub fn remove<C: AdbClient + ?Sized>(&self, adb: &mut C) -> Result<(), DeviceDiagnostic> {
        adb.remove_reverse(&self.serial, self.remote_port)
    }
}

/// Stateless safety policy for one Dev Bridge reverse mapping.
pub struct DevBridgeReverseCoordinator;

impl DevBridgeReverseCoordinator {
    /// Selects exactly one ready ADB transport and creates its reverse mapping.
    ///
    /// # Errors
    ///
    /// Returns a stable selection diagnostic, or preserves an adapter diagnostic.
    pub fn establish<C: AdbClient + ?Sized>(
        adb: &mut C,
        request: DevBridgeReverseRequest,
    ) -> Result<ReverseMapping, DeviceDiagnostic> {
        let serial = select_one_ready_device(adb.list_devices()?)?;
        let local_port = request.local_port();
        let remote_port = request.remote_port();

        adb.reverse(&serial, local_port, remote_port)?;

        Ok(ReverseMapping {
            serial,
            local_port,
            remote_port,
        })
    }
}

fn select_one_ready_device(devices: Vec<AdbDevice>) -> Result<DeviceSerial, DeviceDiagnostic> {
    let mut eligible = devices
        .into_iter()
        .filter(|device| device.state == AdbDeviceState::Device)
        .map(|device| device.serial);
    let Some(serial) = eligible.next() else {
        return Err(DeviceDiagnostic::new(
            "device.adb.noEligibleDevice",
            "exactly one ready ADB device is required",
        ));
    };
    if eligible.next().is_some() {
        return Err(DeviceDiagnostic::new(
            "device.adb.multipleEligibleDevices",
            "exactly one ready ADB device is required",
        ));
    }
    Ok(serial)
}
