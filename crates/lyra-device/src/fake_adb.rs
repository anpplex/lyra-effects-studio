use std::collections::VecDeque;
use std::path::Path;

use serde::Deserialize;

use crate::{
    AdbClient, AdbDevice, DeviceDiagnostic, DevicePath, DeviceSerial, LocalPort, RemotePort,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Transcript {
    steps: VecDeque<TranscriptStep>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TranscriptStep {
    operation: String,
    serial: Option<String>,
    local_port: Option<u16>,
    remote_port: Option<u16>,
    local_path: Option<String>,
    device_path: Option<String>,
    result: TranscriptResult,
}

#[derive(Clone, Debug, Default, Deserialize)]
struct TranscriptResult {
    #[serde(default)]
    devices: Vec<AdbDevice>,
    error: Option<TranscriptError>,
}

#[derive(Clone, Debug, Deserialize)]
struct TranscriptError {
    code: String,
    message: String,
}

#[derive(Debug)]
pub struct FakeAdb {
    steps: VecDeque<TranscriptStep>,
}

impl FakeAdb {
    /// Loads an ordered fake ADB call transcript.
    ///
    /// # Errors
    ///
    /// Returns `device.fakeAdb.invalidTranscript` for malformed JSON.
    pub fn from_slice(bytes: &[u8]) -> Result<Self, DeviceDiagnostic> {
        let transcript: Transcript = serde_json::from_slice(bytes).map_err(|error| {
            DeviceDiagnostic::new("device.fakeAdb.invalidTranscript", error.to_string())
        })?;
        Ok(Self {
            steps: transcript.steps,
        })
    }

    /// Verifies that every configured call was consumed.
    ///
    /// # Errors
    ///
    /// Returns `device.fakeAdb.pendingCalls` when the transcript is incomplete.
    pub fn assert_finished(&self) -> Result<(), DeviceDiagnostic> {
        if self.steps.is_empty() {
            Ok(())
        } else {
            Err(DeviceDiagnostic::new(
                "device.fakeAdb.pendingCalls",
                format!("{} transcript calls remain", self.steps.len()),
            ))
        }
    }

    fn take(&mut self, operation: &str) -> Result<TranscriptStep, DeviceDiagnostic> {
        let Some(step) = self.steps.front() else {
            return Err(unexpected(operation, "transcript is exhausted"));
        };
        if step.operation != operation {
            return Err(unexpected(
                operation,
                format!("next operation is {}", step.operation),
            ));
        }
        self.steps
            .pop_front()
            .ok_or_else(|| unexpected(operation, "transcript is exhausted"))
    }
}

impl AdbClient for FakeAdb {
    fn list_devices(&mut self) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
        let step = self.take("listDevices")?;
        result(&step.result)?;
        Ok(step.result.devices)
    }

    fn reverse(
        &mut self,
        serial: &DeviceSerial,
        local: LocalPort,
        remote: RemotePort,
    ) -> Result<(), DeviceDiagnostic> {
        let step = self.take("reverse")?;
        let serial = serial.to_string();
        expect_field(
            "reverse",
            "serial",
            &step.serial.as_deref(),
            &Some(serial.as_str()),
        )?;
        expect_field("reverse", "localPort", &step.local_port, &Some(local.get()))?;
        expect_field(
            "reverse",
            "remotePort",
            &step.remote_port,
            &Some(remote.get()),
        )?;
        result(&step.result)
    }

    fn remove_reverse(
        &mut self,
        serial: &DeviceSerial,
        remote: RemotePort,
    ) -> Result<(), DeviceDiagnostic> {
        let step = self.take("removeReverse")?;
        let serial = serial.to_string();
        expect_field(
            "removeReverse",
            "serial",
            &step.serial.as_deref(),
            &Some(serial.as_str()),
        )?;
        expect_field(
            "removeReverse",
            "remotePort",
            &step.remote_port,
            &Some(remote.get()),
        )?;
        result(&step.result)
    }

    fn push(
        &mut self,
        serial: &DeviceSerial,
        local: &Path,
        destination: &DevicePath,
    ) -> Result<(), DeviceDiagnostic> {
        let step = self.take("push")?;
        let local = local
            .to_str()
            .ok_or_else(|| unexpected("push", "local path is not UTF-8"))?;
        let serial = serial.to_string();
        let destination = destination.to_string();
        expect_field(
            "push",
            "serial",
            &step.serial.as_deref(),
            &Some(serial.as_str()),
        )?;
        expect_field(
            "push",
            "localPath",
            &step.local_path.as_deref(),
            &Some(local),
        )?;
        expect_field(
            "push",
            "devicePath",
            &step.device_path.as_deref(),
            &Some(destination.as_str()),
        )?;
        result(&step.result)
    }
}

fn result(value: &TranscriptResult) -> Result<(), DeviceDiagnostic> {
    value.error.as_ref().map_or(Ok(()), |error| {
        Err(DeviceDiagnostic::new(&error.code, &error.message))
    })
}

fn expect_field<T: PartialEq + std::fmt::Debug>(
    operation: &str,
    field: &str,
    actual: &T,
    expected: &T,
) -> Result<(), DeviceDiagnostic> {
    if actual == expected {
        Ok(())
    } else {
        Err(unexpected(
            operation,
            format!("{field} expected {expected:?}, got {actual:?}"),
        ))
    }
}

fn unexpected(operation: &str, reason: impl std::fmt::Display) -> DeviceDiagnostic {
    DeviceDiagnostic::new(
        "device.fakeAdb.unexpectedCall",
        format!("unexpected {operation}: {reason}"),
    )
}
