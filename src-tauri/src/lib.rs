use std::path::PathBuf;

use serde::Serialize;
use tauri_plugin_dialog::DialogExt;
use tokio::sync::oneshot;

mod device_bridge;
mod project;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppInfo {
    app_version: &'static str,
    #[serde(rename = "packContractVersion")]
    pack_contract: u64,
    #[serde(rename = "projectContractVersion")]
    project_contract: u64,
    #[serde(rename = "registryContractVersion")]
    registry_contract: u64,
}

#[tauri::command]
fn app_info() -> AppInfo {
    AppInfo {
        app_version: env!("CARGO_PKG_VERSION"),
        pack_contract: lyra_pack::FORMAT_VERSION,
        project_contract: lyra_project::FORMAT_VERSION,
        registry_contract: lyra_registry::FORMAT_VERSION,
    }
}

#[tauri::command]
async fn get_device_bridge_status(
    controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
) -> Result<device_bridge::DeviceBridgeStatus, String> {
    Ok(controller.status().await)
}

#[tauri::command]
async fn start_device_bridge(
    controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
) -> Result<device_bridge::DeviceBridgeStatus, String> {
    controller.start().await.map_err(|error| error.to_string())
}

#[tauri::command]
async fn stop_device_bridge(
    controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
) -> Result<device_bridge::DeviceBridgeStatus, String> {
    controller.stop().await.map_err(|error| error.to_string())
}

#[tauri::command]
async fn get_device_bridge_adb_status(
    controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
) -> Result<device_bridge::AdbPreflightStatus, String> {
    Ok(controller.adb_status().await)
}

#[tauri::command]
async fn choose_device_bridge_adb_executable(
    app: tauri::AppHandle,
    controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
) -> Result<device_bridge::AdbPreflightStatus, String> {
    let Some(path) = choose_adb_executable(&app).await? else {
        return Ok(controller.adb_status().await);
    };
    controller
        .configure_adb_executable(path)
        .await
        .map_err(|error| error.to_string())
}

#[tauri::command]
async fn check_device_bridge_adb(
    controller: tauri::State<'_, device_bridge::DeviceBridgeController>,
) -> Result<device_bridge::AdbPreflightStatus, String> {
    controller
        .check_adb()
        .await
        .map_err(|error| error.to_string())
}

async fn choose_adb_executable(app: &tauri::AppHandle) -> Result<Option<PathBuf>, String> {
    let (sender, receiver) = oneshot::channel();
    app.dialog()
        .file()
        .set_title("Choose Android Debug Bridge executable")
        .pick_file(move |selection| {
            let result = selection
                .map(|file| {
                    file.into_path().map_err(|_| {
                        "device.adb.invalidExecutable: selected file is not a local path".to_owned()
                    })
                })
                .transpose();
            let _ = sender.send(result);
        });
    receiver.await.map_err(|_| {
        "device.adb.invalidExecutable: ADB file picker closed unexpectedly".to_owned()
    })?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Starts the desktop application runtime.
///
/// # Panics
///
/// Panics when Tauri cannot create or run the application event loop.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(device_bridge::DeviceBridgeController::new())
        .invoke_handler(tauri::generate_handler![
            app_info,
            get_device_bridge_status,
            start_device_bridge,
            stop_device_bridge,
            get_device_bridge_adb_status,
            choose_device_bridge_adb_executable,
            check_device_bridge_adb,
            project::open_project,
            project::save_project_document,
            project::save_project_style
        ])
        .run(tauri::generate_context!())
        .expect("error while running Lyra Effects Studio");
}

#[cfg(test)]
mod tests {
    #[test]
    fn app_info_exposes_all_portable_contract_versions() {
        let info = super::app_info();

        assert_eq!(info.app_version, env!("CARGO_PKG_VERSION"));
        assert_eq!(info.pack_contract, 1);
        assert_eq!(info.project_contract, 1);
        assert_eq!(info.registry_contract, 1);
    }

    #[test]
    fn adb_preflight_commands_are_available() {
        let _ = super::get_device_bridge_adb_status;
        let _ = super::choose_device_bridge_adb_executable;
        let _ = super::check_device_bridge_adb;
    }
}
