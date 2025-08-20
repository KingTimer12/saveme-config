use super::App;
use anyhow::{anyhow, Result};
use std::path::PathBuf;

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

    fn target_hint(&self) -> &'static str {
        "app:zed"
    }

    fn package_id(&self) -> Option<&'static str> {
        if cfg!(target_os = "macos") || cfg!(target_os = "linux") {
            Some("zed")
        } else {
            None
        }
    }

    fn app_path(&self) -> Result<PathBuf> {
        let platform = tauri_plugin_os::platform();
        let config_dir = if platform == "windows" {
            std::env::var("APPDATA")
                .map(PathBuf::from)
                .map_err(|e| anyhow!("Failed to get APPDATA: {}", e))?
        } else {
            let config_home = std::env::var("XDG_CONFIG_HOME")
                .map(PathBuf::from)
                .or_else(|_| std::env::var("HOME").map(|h| PathBuf::from(h).join(".config")))
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

        fn collect_files_recursive(dir: &PathBuf, files: &mut Vec<PathBuf>) -> Result<()> {
            for entry in std::fs::read_dir(dir)
                .map_err(|e| anyhow!("Failed to read directory: {}", e))?
            {
                let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
                let path = entry.path();
                if path.is_file() {
                    files.push(path);
                } else if path.is_dir() {
                    collect_files_recursive(&path, files)?;
                }
            }
            Ok(())
        }

        collect_files_recursive(&zed_dir, &mut files)
            .map_err(|e| anyhow!("Failed to read zed config directory recursively: {}", e))?;

        // On Linux, also collect files from .local/share/zed/extensions/installed
        let platform = tauri_plugin_os::platform();
        if platform == "linux" {
            if let Ok(home_dir) = std::env::var("HOME") {
                let extensions_dir = PathBuf::from(home_dir)
                    .join(".local")
                    .join("share")
                    .join("zed")
                    .join("extensions")
                    .join("installed");

                if extensions_dir.exists() {
                    for entry in std::fs::read_dir(&extensions_dir)
                        .map_err(|e| anyhow!("Failed to read extensions directory: {}", e))?
                    {
                        let entry = entry.map_err(|e| anyhow!("Failed to read directory entry: {}", e))?;
                        let path = entry.path();
                        if path.is_dir() {
                            files.push(path);
                        }
                    }
                }
            }
        }

        Ok(files)
    }
}
