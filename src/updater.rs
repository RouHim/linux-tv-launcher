use self_update::cargo_crate_version;

/// Checks for updates and returns whether an update was applied
pub fn check_for_updates() -> Result<bool, String> {
    let no_confirm = !cfg!(debug_assertions);

    let updater = self_update::backends::github::Update::configure()
        .repo_owner("RouHim")
        .repo_name("linux-tv-launcher")
        .bin_name("linux-tv-launcher")
        .show_download_progress(false) // Disable progress to avoid console output in GUI
        .no_confirm(no_confirm)
        .current_version(cargo_crate_version!())
        .build()
        .map_err(|e| format!("Failed to configure updater: {}", e))?;

    let status = updater
        .update()
        .map_err(|e| format!("Update check failed: {}", e))?;

    match status {
        self_update::Status::UpToDate(_version) => Ok(false),
        self_update::Status::Updated(_version) => Ok(true),
    }
}
