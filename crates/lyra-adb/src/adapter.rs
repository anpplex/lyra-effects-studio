use std::ffi::OsString;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;

use lyra_device::{
    AdbClient, AdbDevice, AdbDeviceState, DeviceDiagnostic, DevicePath, DeviceSerial, LocalPort,
    RemotePort,
};

/// Explicit host-side ADB adapter backed by a configured executable path.
pub struct SystemAdb {
    inner: Adapter<SystemExecutor>,
}

impl SystemAdb {
    #[must_use]
    pub fn from_path(executable: impl Into<PathBuf>) -> Self {
        Self {
            inner: Adapter::new(executable.into(), SystemExecutor),
        }
    }
}

impl AdbClient for SystemAdb {
    fn list_devices(&mut self) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
        self.inner.list_devices()
    }

    fn reverse(
        &mut self,
        serial: &DeviceSerial,
        local: LocalPort,
        remote: RemotePort,
    ) -> Result<(), DeviceDiagnostic> {
        self.inner.reverse(serial, local, remote)
    }

    fn remove_reverse(
        &mut self,
        serial: &DeviceSerial,
        remote: RemotePort,
    ) -> Result<(), DeviceDiagnostic> {
        self.inner.remove_reverse(serial, remote)
    }

    fn push(
        &mut self,
        serial: &DeviceSerial,
        local: &Path,
        destination: &DevicePath,
    ) -> Result<(), DeviceDiagnostic> {
        self.inner.push(serial, local, destination)
    }
}

pub(crate) struct Adapter<E> {
    executable: PathBuf,
    executor: E,
}

impl<E> Adapter<E> {
    pub(crate) fn new(executable: PathBuf, executor: E) -> Self {
        Self {
            executable,
            executor,
        }
    }
}

pub(crate) trait CommandExecutor {
    fn output(&mut self, executable: &Path, arguments: &[OsString]) -> io::Result<CommandOutput>;
}

pub(crate) struct CommandOutput {
    pub(crate) success: bool,
    pub(crate) stdout: Vec<u8>,
}

struct SystemExecutor;

impl CommandExecutor for SystemExecutor {
    fn output(&mut self, executable: &Path, arguments: &[OsString]) -> io::Result<CommandOutput> {
        Command::new(executable)
            .args(arguments)
            .output()
            .map(|output| CommandOutput {
                success: output.status.success(),
                stdout: output.stdout,
            })
    }
}

impl<E: CommandExecutor> Adapter<E> {
    fn execute(
        &mut self,
        operation: &str,
        arguments: &[OsString],
    ) -> Result<Vec<u8>, DeviceDiagnostic> {
        let output = self
            .executor
            .output(&self.executable, arguments)
            .map_err(|_| {
                DeviceDiagnostic::new("device.adb.launchFailed", "could not launch adb")
            })?;
        if !output.success {
            return Err(DeviceDiagnostic::new(
                "device.adb.commandFailed",
                format!("adb {operation} failed"),
            ));
        }
        Ok(output.stdout)
    }
}

impl<E: CommandExecutor> AdbClient for Adapter<E> {
    fn list_devices(&mut self) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
        let arguments = [OsString::from("devices"), OsString::from("-l")];
        let stdout = self.execute("devices", &arguments)?;
        parse_devices(&stdout)
    }

    fn reverse(
        &mut self,
        serial: &DeviceSerial,
        local: LocalPort,
        remote: RemotePort,
    ) -> Result<(), DeviceDiagnostic> {
        let mut arguments = targeted_arguments(serial);
        arguments.extend([
            OsString::from("reverse"),
            tcp_argument(remote.get()),
            tcp_argument(local.get()),
        ]);
        self.execute("reverse", &arguments)?;
        Ok(())
    }

    fn remove_reverse(
        &mut self,
        serial: &DeviceSerial,
        remote: RemotePort,
    ) -> Result<(), DeviceDiagnostic> {
        let mut arguments = targeted_arguments(serial);
        arguments.extend([
            OsString::from("reverse"),
            OsString::from("--remove"),
            tcp_argument(remote.get()),
        ]);
        self.execute("reverse removal", &arguments)?;
        Ok(())
    }

    fn push(
        &mut self,
        serial: &DeviceSerial,
        local: &Path,
        destination: &DevicePath,
    ) -> Result<(), DeviceDiagnostic> {
        let mut arguments = targeted_arguments(serial);
        arguments.extend([
            OsString::from("push"),
            local.as_os_str().to_os_string(),
            OsString::from(destination.to_string()),
        ]);
        self.execute("push", &arguments)?;
        Ok(())
    }
}

fn targeted_arguments(serial: &DeviceSerial) -> Vec<OsString> {
    vec![OsString::from("-s"), OsString::from(serial.to_string())]
}

fn tcp_argument(port: u16) -> OsString {
    OsString::from(format!("tcp:{port}"))
}

fn parse_devices(stdout: &[u8]) -> Result<Vec<AdbDevice>, DeviceDiagnostic> {
    let output =
        std::str::from_utf8(stdout).map_err(|_| invalid_device_list("device list is not UTF-8"))?;
    let Some(header_index) = output
        .lines()
        .position(|line| line.trim() == "List of devices attached")
    else {
        return Err(invalid_device_list("device list header is missing"));
    };

    output
        .lines()
        .skip(header_index + 1)
        .filter(|line| !line.trim().is_empty())
        .map(parse_device_row)
        .collect()
}

fn parse_device_row(row: &str) -> Result<AdbDevice, DeviceDiagnostic> {
    let mut fields = row.split_whitespace();
    let Some(serial_source) = fields.next() else {
        return Err(invalid_device_list("device row is empty"));
    };
    let Some(state_source) = fields.next() else {
        return Err(invalid_device_list("device row is missing its state"));
    };
    let serial = DeviceSerial::new(serial_source)
        .map_err(|_| invalid_device_list("device list contains an invalid serial"))?;
    let state = match state_source {
        "device" => AdbDeviceState::Device,
        "offline" => AdbDeviceState::Offline,
        "unauthorized" => AdbDeviceState::Unauthorized,
        _ => {
            return Err(DeviceDiagnostic::new(
                "device.adb.unsupportedDeviceState",
                "device list contains an unsupported state",
            ));
        }
    };
    Ok(AdbDevice { serial, state })
}

fn invalid_device_list(message: &str) -> DeviceDiagnostic {
    DeviceDiagnostic::new("device.adb.invalidDeviceList", message)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::ffi::OsString;
    use std::io;
    use std::path::{Path, PathBuf};

    use lyra_device::{AdbClient, AdbDeviceState, DevicePath, DeviceSerial, LocalPort, RemotePort};

    use super::{Adapter, CommandExecutor, CommandOutput};

    struct ExpectedCall {
        arguments: Vec<OsString>,
        result: io::Result<CommandOutput>,
    }

    struct FakeExecutor {
        calls: VecDeque<ExpectedCall>,
    }

    impl FakeExecutor {
        fn new(calls: Vec<ExpectedCall>) -> Self {
            Self {
                calls: calls.into(),
            }
        }

        fn assert_finished(&self) {
            assert!(self.calls.is_empty(), "unexpected pending process calls");
        }
    }

    impl CommandExecutor for FakeExecutor {
        fn output(
            &mut self,
            executable: &Path,
            arguments: &[OsString],
        ) -> io::Result<CommandOutput> {
            assert_eq!(executable, Path::new("/opt/android/adb"));
            let expected = self
                .calls
                .pop_front()
                .ok_or_else(|| io::Error::other("unexpected process call"))?;
            assert_eq!(arguments, expected.arguments);
            expected.result
        }
    }

    fn adapter(calls: Vec<ExpectedCall>) -> Adapter<FakeExecutor> {
        Adapter::new(PathBuf::from("/opt/android/adb"), FakeExecutor::new(calls))
    }

    fn call(arguments: &[&str], result: io::Result<CommandOutput>) -> ExpectedCall {
        ExpectedCall {
            arguments: arguments.iter().map(OsString::from).collect(),
            result,
        }
    }

    fn success(stdout: impl Into<Vec<u8>>) -> CommandOutput {
        CommandOutput {
            success: true,
            stdout: stdout.into(),
        }
    }

    #[test]
    fn list_devices_uses_devices_long_and_parses_supported_states() {
        let mut adb = adapter(vec![call(
            &["devices", "-l"],
            Ok(success(
                b"* daemon not running. starting it now on port 5037 *\n* daemon started successfully *\nList of devices attached\nAVATR-01 device product:cluster\nAVATR-02 offline\nAVATR-03 unauthorized usb:1-1\n",
            )),
        )]);

        let devices = adb.list_devices().expect("device list");

        assert_eq!(devices.len(), 3);
        assert_eq!(devices[0].state, AdbDeviceState::Device);
        assert_eq!(devices[1].state, AdbDeviceState::Offline);
        assert_eq!(devices[2].state, AdbDeviceState::Unauthorized);
        adb.executor.assert_finished();
    }

    #[test]
    fn mutations_use_fixed_separate_arguments() {
        let mut adb = adapter(vec![
            call(
                &["-s", "AVATR-01", "reverse", "tcp:49321", "tcp:42137"],
                Ok(success([])),
            ),
            call(
                &["-s", "AVATR-01", "reverse", "--remove", "tcp:49321"],
                Ok(success([])),
            ),
            call(
                &[
                    "-s",
                    "AVATR-01",
                    "push",
                    "/tmp/revision.zip",
                    "/data/local/tmp/lyra/revision.zip",
                ],
                Ok(success([])),
            ),
        ]);
        let serial = DeviceSerial::new("AVATR-01").expect("serial");
        let local = LocalPort::new(42_137).expect("local port");
        let remote = RemotePort::new(49_321).expect("remote port");
        let destination = DevicePath::new("/data/local/tmp/lyra/revision.zip").expect("path");

        adb.reverse(&serial, local, remote).expect("reverse");
        adb.remove_reverse(&serial, remote).expect("remove reverse");
        adb.push(&serial, Path::new("/tmp/revision.zip"), &destination)
            .expect("push");

        adb.executor.assert_finished();
    }

    #[test]
    fn normalizes_launch_and_unsuccessful_command_failures() {
        let mut launch = adapter(vec![call(
            &["devices", "-l"],
            Err(io::Error::other("configured launch failure")),
        )]);
        let launch_error = launch.list_devices().expect_err("launch failure");
        assert_eq!(launch_error.code, "device.adb.launchFailed");
        launch.executor.assert_finished();

        let mut command = adapter(vec![call(
            &["devices", "-l"],
            Ok(CommandOutput {
                success: false,
                stdout: b"untrusted command output".to_vec(),
            }),
        )]);
        let command_error = command.list_devices().expect_err("command failure");
        assert_eq!(command_error.code, "device.adb.commandFailed");
        assert!(!command_error.message.contains("untrusted"));
        command.executor.assert_finished();
    }

    #[test]
    fn rejects_invalid_or_unsupported_device_lists() {
        for (name, stdout, code) in [
            (
                "missing header",
                b"AVATR-01 device\n".to_vec(),
                "device.adb.invalidDeviceList",
            ),
            (
                "invalid utf8",
                vec![0xff, b'\n'],
                "device.adb.invalidDeviceList",
            ),
            (
                "missing state",
                b"List of devices attached\nAVATR-01\n".to_vec(),
                "device.adb.invalidDeviceList",
            ),
            (
                "unsupported state",
                b"List of devices attached\nAVATR-01 recovery\n".to_vec(),
                "device.adb.unsupportedDeviceState",
            ),
        ] {
            let mut adb = adapter(vec![call(&["devices", "-l"], Ok(success(stdout)))]);

            let error = adb.list_devices().expect_err(name);

            assert_eq!(error.code, code, "{name}");
            adb.executor.assert_finished();
        }
    }
}
