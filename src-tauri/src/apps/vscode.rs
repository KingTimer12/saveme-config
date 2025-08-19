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
    
    fn snap_support(&self) -> bool {
        false
    }

    fn is_installed(&self) -> bool {
        self.app_path().map(|p| p.exists()).unwrap_or(false)
    }
    
    fn app_path(&self) -> Result<PathBuf> {
        let platform = tauri_plugin_os::platform();
        let app_dir = if platform == "windows" {
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
        Ok(app_dir)
    }

    fn config_path(&self) -> Result<Vec<PathBuf>> {
        let app_dir = self.app_path()?;
        
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&app_dir)
            .map_err(|e| anyhow!("Failed to read vscode config directory: {}", e))? {
            let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }

        Ok(files)
    }

    fn target_hint(&self) -> &'static str {
        "app:vscode"
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
