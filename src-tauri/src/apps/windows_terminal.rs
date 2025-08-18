use std::path::PathBuf;
use anyhow::{Result, anyhow};
use super::App;

pub struct WindowsTerminal;

impl App for WindowsTerminal {
    fn id(&self) -> &'static str {
        "windows-terminal"
    }

    fn name(&self) -> &'static str {
        "Windows Terminal"
    }

    fn is_installed(&self) -> bool {
        self.config_path().map(|p| p.exists()).unwrap_or(false)
    }

    fn config_path(&self) -> Result<PathBuf> {
        if !cfg!(target_os = "windows") {
            return Err(anyhow!("Windows Terminal is only available on Windows."));
        }

        let local_appdata = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .map_err(|e| anyhow!("Failed to get LOCALAPPDATA: {}", e))?;

        // This is a known constant path for Windows Terminal
        Ok(local_appdata
            .join("Packages")
            .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
            .join("LocalState")
            .join("settings.json"))
    }

    fn target_hint(&self) -> &'static str {
        "sys:windows-terminal:settings"
    }

    fn package_id(&self) -> Option<&'static str> {
        if cfg!(target_os = "windows") {
            Some("Microsoft.WindowsTerminal")
        } else {
            None
        }
    }
}
