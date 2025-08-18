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

    fn is_installed(&self) -> bool {
        self.config_path().map(|p| p.exists()).unwrap_or(false)
    }

    fn config_path(&self) -> Result<PathBuf> {
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
        Ok(config_dir.join("zed").join("settings.json"))
    }

    fn target_hint(&self) -> &'static str {
        "app:zed:settings"
    }

    fn package_id(&self) -> Option<&'static str> {
        // Zed is not yet in many official repositories.
        // On Linux, it's often installed via a script.
        // On macOS, it can be a cask: "zed"
        // On Windows, it's a winget package: "Zed.Zed"
        // For now, we will return a value for macOS to test, but a more robust
        // solution would check the OS.
        if cfg!(target_os = "macos") {
            Some("zed")
        } else {
            None
        }
    }
}
