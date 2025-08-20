// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/

use chrono::Utc;
use serde::Serialize;
use tauri_plugin_os::platform;

mod apps;
mod installer;
mod storage;

use apps::AppInfo;
use storage::manifest::Manifest;

#[derive(Serialize, Clone)]
struct BackupInfo {
    name: String,
    created_at: String,
}

#[derive(Serialize, Clone)]
struct BackupChainInfo {
    name: String,
    backup_hash: String,
    chain_hash: String,
    previous_backup_hash: Option<String>,
    is_integrity_valid: bool,
}

#[tauri::command]
fn list_applications() -> Vec<AppInfo> {
    apps::get_all_apps_info()
}

#[tauri::command]
fn save_config(name: &str, app_ids: Vec<String>) -> Result<String, String> {
    let mut manifest = Manifest::new(
        name.to_string(),
        Utc::now().to_rfc3339(),
        platform().to_string(),
    );

    for app_id in app_ids {
        if let Some(app) = apps::get_app(&app_id) {
            if app.is_installed() {
                println!("Processing app: {}", app.name());
                if let Ok(paths) = app.config_path() {
                    for path in paths {
                        println!("Processing config file: {}", path.display());
                        if path.exists() {
                            println!("Config file exists");
                            println!("Creating blob from file");
                            manifest
                                .create_blob_from_file(&path, app.target_hint())
                                .map_err(|e| e.to_string())?;
                            println!("Blob created successfully");
                        }
                    }
                }
            }
        }
    }

    // Configurar cadeia blockchain (referenciar backup anterior se existir)
    let existing_backups = Manifest::list_all_backups_sorted().map_err(|e| e.to_string())?;
    if !existing_backups.is_empty() {
        let last_backup = existing_backups.last().unwrap();
        if last_backup != name {
            // Só referenciar se não for o mesmo backup sendo atualizado
            manifest.set_previous_backup(last_backup).map_err(|e| e.to_string())?;
            println!("Linked to previous backup: {}", last_backup);
        }
    }

    manifest.ingest_blobs_dir().map_err(|e| e.to_string())?;
    manifest.save().map_err(|e| e.to_string())?;
    Ok("Config saved successfully".to_string())
}

#[tauri::command]
fn list_backups() -> Result<Vec<BackupInfo>, String> {
    let storage_dir = Manifest::base_storage_dir().map_err(|e| e.to_string())?;
    let mut backups = Vec::new();

    if !storage_dir.exists() {
        return Ok(backups);
    }

    for entry in std::fs::read_dir(storage_dir).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if entry.file_type().map_err(|e| e.to_string())?.is_dir() {
            let manifest_path = entry.path().join("manifest.json");
            if manifest_path.exists() {
                let content = std::fs::read_to_string(manifest_path).map_err(|e| e.to_string())?;
                let manifest: Manifest =
                    serde_json::from_str(&content).map_err(|e| e.to_string())?;
                backups.push(BackupInfo {
                    name: manifest.name,
                    created_at: manifest.created_at,
                });
            }
        }
    }
    Ok(backups)
}

#[tauri::command]
fn restore_config(backup_name: &str, app_ids: Vec<String>) -> Result<String, String> {
    let manifest = Manifest::load_from(backup_name).map_err(|e| e.to_string())?;

    for app_id in app_ids {
        if let Some(app) = apps::get_app(&app_id) {
            // If the app is not installed, try to install it.
            if !app.is_installed() {
                if app.package_id().is_some() {
                    installer::install_app(app)?;
                } else {
                    // Optionally, you could choose to skip or warn the user.
                    // For now, we'll just print a message to the console.
                    println!("Skipping restore for '{}' because it is not installed and no package_id is available.", app.name());
                    continue;
                }
            }

            // Proceed with restoring the configuration.
            if let Some(entry) = manifest
                .entries
                .iter()
                .find(|e| e.target_hint == app.target_hint())
            {
                if let Ok(dest_paths) = app.config_path() {
                    // Ensure parent directory exists
                    for dest_path in dest_paths {
                        if let Some(parent) = dest_path.parent() {
                            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
                        }
                        manifest
                            .restore_blob_to(entry, &dest_path)
                            .map_err(|e| e.to_string())?;
                    }
                }
            }
        }
    }

    Ok("Config restored successfully".to_string())
}

#[tauri::command]
fn verify_backup_integrity(backup_name: &str) -> Result<String, String> {
    let manifest = Manifest::load_from(backup_name).map_err(|e| e.to_string())?;
    
    let is_valid = manifest.verify_backup_integrity().map_err(|e| e.to_string())?;
    
    if is_valid {
        Ok(format!("Backup '{}' integrity verified successfully", backup_name))
    } else {
        Err(format!("Backup '{}' failed integrity verification", backup_name))
    }
}

#[tauri::command]
fn verify_backup_chain(start_backup_name: &str) -> Result<String, String> {
    let manifest = Manifest::load_from(start_backup_name).map_err(|e| e.to_string())?;
    
    let is_valid = manifest.verify_chain_from(start_backup_name).map_err(|e| e.to_string())?;
    
    if is_valid {
        Ok(format!("Backup chain starting from '{}' verified successfully", start_backup_name))
    } else {
        Err(format!("Backup chain starting from '{}' failed verification", start_backup_name))
    }
}

#[tauri::command]
fn get_backup_chain_info(backup_name: &str) -> Result<BackupChainInfo, String> {
    let manifest = Manifest::load_from(backup_name).map_err(|e| e.to_string())?;
    
    Ok(BackupChainInfo {
        name: manifest.name.clone(),
        backup_hash: manifest.calculate_backup_hash().map_err(|e| e.to_string())?,
        chain_hash: manifest.backup_chain_hash.clone().unwrap_or_default(),
        previous_backup_hash: manifest.previous_backup_hash.clone(),
        is_integrity_valid: manifest.verify_backup_integrity().map_err(|e| e.to_string())?,
    })
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
        .invoke_handler(tauri::generate_handler![
            list_applications,
            save_config,
            list_backups,
            restore_config,
            verify_backup_integrity,
            verify_backup_chain,
            get_backup_chain_info
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
