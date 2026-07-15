use serde::Serialize;

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
}
