//! AlmarioKS desktop app. Thin shell around `almarioks-core`: no vault commands until release 0.1.

pub mod clipboard;

/// Starts the app. Returns the error instead of panicking so `main` decides how to exit.
///
/// # Errors
/// Returns the Tauri error if the runtime, a plugin or the main window cannot start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .setup(|app| {
            // Updater (ADR-0003): nothing is checked at startup. The window asks only when the user clicks,
            // and the plugin verifies the signature against the `pubkey` in tauri.conf.json before installing.
            #[cfg(desktop)]
            app.handle()
                .plugin(tauri_plugin_updater::Builder::new().build())?;
            #[cfg(not(desktop))]
            let _ = app;
            Ok(())
        })
        .run(tauri::generate_context!())
}
