use super::App;
use anyhow::{anyhow, Result};
use std::path::PathBuf;

pub struct WindowsTerminal;

impl App for WindowsTerminal {
    fn id(&self) -> &'static str {
        "windows-terminal"
    }

    fn name(&self) -> &'static str {
        "Windows Terminal"
    }

    fn snap_support(&self) -> bool {
        false
    }

    fn is_installed(&self) -> bool {
        self.app_path().map(|p| p.exists()).unwrap_or(false)
    }

    fn target_hint(&self) -> &'static str {
        "sys:windows-terminal"
    }

    fn package_id(&self) -> Option<&'static str> {
        if cfg!(target_os = "windows") {
            Some("Microsoft.WindowsTerminal")
        } else {
            None
        }
    }

    fn app_path(&self) -> Result<PathBuf> {
        if !cfg!(target_os = "windows") {
            return Err(anyhow!("Windows Terminal is only available on Windows."));
        }

        let local_appdata = std::env::var("LOCALAPPDATA")
            .map(PathBuf::from)
            .map_err(|e| anyhow!("Failed to get LOCALAPPDATA: {}", e))?;

        let base_path = local_appdata
            .join("Packages")
            .join("Microsoft.WindowsTerminal_8wekyb3d8bbwe")
            .join("LocalState");

        Ok(base_path)
    }

    fn config_path(&self) -> Result<Vec<PathBuf>> {
        let base_path = self.app_path()?;
        let mut paths = Vec::new();

        if base_path.exists() {
            match std::fs::read_dir(&base_path) {
                Ok(entries) => {
                    for entry in entries {
                        if let Ok(entry) = entry {
                            paths.push(entry.path());
                        }
                    }
                }
                Err(e) => return Err(anyhow!("Failed to read directory: {}", e)),
            }
        }

        Ok(paths)
    }
}
