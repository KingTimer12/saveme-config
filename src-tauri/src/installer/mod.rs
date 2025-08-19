use crate::apps::App;
use std::process::Command;

pub fn install_app(app: &dyn App) -> Result<(), String> {
    let package_id = app.package_id().ok_or_else(|| {
        format!(
            "Application '{}' does not have a package ID defined and cannot be installed.",
            app.name()
        )
    })?;

    let platform = tauri_plugin_os::platform();

    println!("Attempting to install '{}' with package ID '{}' on platform '{}'", app.name(), package_id, platform);

    // Check if app supports snap and if we're on Linux
    if platform != "windows" && platform != "darwin" && app.snap_support() {
        // First, install snap if not already installed
        println!("Installing snap package manager...");
        let snap_install_status = Command::new("sudo")
            .arg("apt-get")
            .arg("install")
            .arg("-y")
            .arg("snapd")
            .status()
            .map_err(|e| {
                format!(
                    "Failed to execute snap installation command. Error: {}",
                    e
                )
            })?;

        if !snap_install_status.success() {
            return Err(format!(
                "Failed to install snap. The command finished with a non-zero exit code: {:?}",
                snap_install_status.code()
            ));
        }

        // Install the application using snap
        println!("Installing '{}' using snap...", app.name());
        let mut cmd = Command::new("sudo");
        cmd.arg("snap").arg("install").arg(package_id);

        let status = cmd.status().map_err(|e| {
            format!(
                "Failed to execute snap installation command for '{}'. Error: {}",
                app.name(),
                e
            )
        })?;

        if status.success() {
            println!("Successfully installed '{}' using snap", app.name());
            Ok(())
        } else {
            Err(format!(
                "Failed to install '{}' using snap. The command finished with a non-zero exit code: {:?}",
                app.name(),
                status.code()
            ))
        }
    } else {
        let mut cmd = if platform == "windows" {
            let mut c = Command::new("winget");
            c.arg("install").arg("-e").arg("--id").arg(package_id);
            c
        } else if platform == "darwin" {
            // macOS
            let mut c = Command::new("brew");
            c.arg("install").arg(package_id);
            c
        } else {
            // Assuming Linux. This is dangerous, should be more specific.
            // For this sandbox, we'll assume 'apt'.
            let mut c = Command::new("sudo");
            c.arg("apt-get").arg("install").arg("-y").arg(package_id);
            c
        };

        let status = cmd.status().map_err(|e| {
            format!(
                "Failed to execute installation command for '{}'. Error: {}",
                app.name(),
                e
            )
        })?;

        if status.success() {
            println!("Successfully installed '{}'", app.name());
            Ok(())
        } else {
            Err(format!(
                "Failed to install '{}'. The command finished with a non-zero exit code: {:?}",
                app.name(),
                status.code()
            ))
        }
    }
}
