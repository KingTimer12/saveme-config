use std::path::PathBuf;
use anyhow::{Result, anyhow};
use super::App;

pub struct VSCode;

impl App for VSCode {
    fn id(&self) -> &'static str {
        "vscode"
    }

    fn name(&self) -> &'static str {
        "Visual Studio Code"
    }

    fn is_installed(&self) -> bool {
        self.config_path().map(|p| p.exists()).unwrap_or(false)
    }

    fn config_path(&self) -> Result<PathBuf> {
        let platform = tauri_plugin_os::platform();
        let config_dir = if platform == "windows" {
            std::env::var("APPDATA").map(PathBuf::from)
                .map_err(|e| anyhow!("Failed to get APPDATA: {}", e))?
                .join("Code")
        } else if platform == "darwin" { // macOS
            dirs::home_dir()
                .ok_or_else(|| anyhow!("Could not get home directory"))?
                .join("Library/Application Support/Code")
        } else { // Linux
            std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|_| {
                    std::env::var("HOME").map(|h| PathBuf::from(h).join(".config"))
                })
                .map_err(|e| anyhow!("Failed to get config dir: {}", e))?
                .join("Code")
        };
        Ok(config_dir.join("User").join("settings.json"))
    }

    fn target_hint(&self) -> &'static str {
        "app:vscode:settings"
    }

    fn package_id(&self) -> Option<&'static str> {
        let platform = tauri_plugin_os::platform();
        if platform == "windows" {
            Some("Microsoft.VisualStudioCode")
        } else if platform == "darwin" {
            Some("visual-studio-code")
        } else {
            // This assumes the user has added Microsoft's repo.
            Some("code")
        }
    }
}
