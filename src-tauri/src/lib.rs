// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use std::path::PathBuf;

use chrono::Utc;

mod storage;

#[tauri::command]
fn save_config(name: &str) -> Result<String, String> {
    let platform = tauri_plugin_os::platform();
    let config_dir = if platform == "windows" {
        std::env::var("APPDATA")
            .unwrap_or_else(|_| std::env::var("USERPROFILE").unwrap_or_default())
    } else {
        std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_default();
            format!("{}/.config", home)
        })
    };
    let zed_path = PathBuf::from(config_dir).join("zed").join("settings.json");
    let mut manifest = storage::manifest::Manifest::new(
        name.to_string(),
        Utc::now().to_rfc3339(),
        platform.to_string(),
    );
    manifest
        .create_blob_from_file(&zed_path, "app:zed:settings")
        .map_err(|e| e.to_string())?;
    manifest.ingest_blobs_dir().map_err(|e| e.to_string())?;
    manifest.save().map_err(|e| e.to_string())?;
    Ok("Config saved successfully".to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(
            tauri_plugin_log::Builder::new()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::Webview,
                ))
                .build(),
        )
        .plugin(tauri_plugin_os::init())
        .plugin(tauri_plugin_shell::init())
        .invoke_handler(tauri::generate_handler![save_config])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
