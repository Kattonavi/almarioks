//! AlmarioKS desktop app. Thin shell around `almarioks-core`: no vault commands until release 0.1.

/// Starts the app. Returns the error instead of panicking so `main` decides how to exit.
///
/// # Errors
/// Returns the Tauri error if the runtime or the main window cannot start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default().run(tauri::generate_context!())
}
