use std::path::PathBuf;
use anyhow::{Result, anyhow};
use super::App;

pub struct Zed;

impl App for Zed {
    fn id(&self) -> &'static str {
        "zed"
    }

    fn name(&self) -> &'static str {
        "Zed"
    }
    
    fn snap_support(&self) -> bool {
        true
    }

    fn is_installed(&self) -> bool {
        self.app_path().map(|p| p.exists()).unwrap_or(false)
    }
    
    fn app_path(&self) -> Result<PathBuf> {
        let platform = tauri_plugin_os::platform();
        let config_dir = if platform == "windows" {
            std::env::var("APPDATA").map(PathBuf::from)
                .map_err(|e| anyhow!("Failed to get APPDATA: {}", e))?
        } else {
            let config_home = std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|_| {
                    std::env::var("HOME").map(|h| PathBuf::from(h).join(".config"))
                })
                .map_err(|e| anyhow!("Failed to get config dir: {}", e))?;
            config_home
        };

        let zed_dir = config_dir.join("zed");
        if !zed_dir.exists() {
            return Err(anyhow!("Zed is not installed"));
        }
        Ok(zed_dir)
    }

    fn config_path(&self) -> Result<Vec<PathBuf>> {
        let zed_dir = self.app_path()?;
        let mut files = Vec::new();
        for entry in std::fs::read_dir(&zed_dir)
            .map_err(|e| anyhow!("Failed to read zed config directory: {}", e))? {
            let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }

        Ok(files)
    }

    fn target_hint(&self) -> &'static str {
        "app:zed"
    }

    fn package_id(&self) -> Option<&'static str> {
        // Zed is not yet in many official repositories.
        // On Linux, it's often installed via a script.
        // On macOS, it can be a cask: "zed"
        // On Windows, it's a winget package: "Zed.Zed"
        // For now, we will return a value for macOS to test, but a more robust
        // solution would check the OS.
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            Some("zed")
        } else {
            None
        }
    }
}
