use std::path::PathBuf;
use anyhow::Result;
use serde::Serialize;
use once_cell::sync::Lazy;

pub mod zed;
pub mod windows_terminal;
pub mod vscode;

#[derive(Serialize, Clone, Debug)]
pub struct AppInfo {
    pub id: String,
    pub name: String,
    pub is_installed: bool,
}

pub trait App: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn is_installed(&self) -> bool;
    fn config_path(&self) -> Result<PathBuf>;
    fn target_hint(&self) -> &'static str;
    fn package_id(&self) -> Option<&'static str>;
}

pub static REGISTRY: Lazy<Vec<Box<dyn App>>> = Lazy::new(|| {
    vec![
        Box::new(zed::Zed),
        Box::new(windows_terminal::WindowsTerminal),
        Box::new(vscode::VSCode),
    ]
});

pub fn get_app(id: &str) -> Option<&'static dyn App> {
    REGISTRY.iter().find(|app| app.id() == id).map(|app| app.as_ref())
}

pub fn get_all_apps_info() -> Vec<AppInfo> {
    REGISTRY.iter().map(|app| AppInfo {
        id: app.id().to_string(),
        name: app.name().to_string(),
        is_installed: app.is_installed(),
    }).collect()
}
