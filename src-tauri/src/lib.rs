use serde::Serialize;

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
/// Starts the desktop application runtime.
///
/// # Panics
///
/// Panics when Tauri cannot create or run the application event loop.
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            app_info,
            project::open_project,
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
