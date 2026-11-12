//! AlmarioKS desktop app. Thin shell around `almarioks-core`: the commands of the window, the clipboard and the
//! plugins. The vault itself lives in the core.

use std::sync::Arc;

use tauri::{AppHandle, Manager, Runtime, plugin::Plugin};

pub mod clipboard;
pub mod commands;
pub mod state;

use clipboard::ClipboardService;
use commands::{dialogs::SystemDialogs, url::SystemUrlOpener};
use state::{AppParts, AppState, SETTINGS_FILE_NAME};

/// What can stop the app from starting.
pub type SetupError = Box<dyn std::error::Error>;

/// The context of `tauri.conf.json`: configuration, capabilities and the list of allowed commands. The tests
/// build their app from this same context, so they exercise the real allow-list (RF-79).
#[must_use]
pub fn context<R: Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

/// The plugins registered on the builder, in the order they are registered.
///
/// `tauri-plugin-dialog` is here only so Rust can open file dialogs (`commands::dialogs`): the capability grants
/// the window none of its commands. `tauri-plugin-opener` is not registered at all: `commands::url` calls its
/// `open_url` function directly.
#[must_use]
pub fn plugins<R: Runtime>() -> Vec<Box<dyn Plugin<R>>> {
    vec![Box::new(tauri_plugin_dialog::init())]
}

/// The parts of the real app: the settings file in the configuration folder Tauri gives (never a fixed path),
/// the Windows clipboard, the system's file dialogs and the default browser.
///
/// # Errors
/// The configuration folder of the app cannot be resolved.
pub fn system_parts<R: Runtime>(app: &AppHandle<R>) -> Result<AppParts, SetupError> {
    let settings_file = app.path().app_config_dir()?.join(SETTINGS_FILE_NAME);
    Ok(AppParts {
        settings_file,
        // If the clipboard cannot start the app starts anyway; copying answers the error.
        clipboard: system_clipboard(),
        dialogs: Arc::new(SystemDialogs::new(app.clone())),
        url_opener: Arc::new(SystemUrlOpener),
    })
}

#[cfg(windows)]
fn system_clipboard() -> Result<ClipboardService, clipboard::ClipboardError> {
    ClipboardService::system()
}

/// Only Windows has a clipboard implementation (`clipboard::ffi`).
#[cfg(not(windows))]
fn system_clipboard() -> Result<ClipboardService, clipboard::ClipboardError> {
    Err(clipboard::ClipboardError::Unavailable)
}

/// Assembles the app on `base`: first `plugins`, in order and before anything else, then the commands. `run`
/// calls it with the real runtime and [`plugins`]; the tests call it with the mock runtime.
#[must_use]
pub fn app_builder<R: Runtime>(
    base: tauri::Builder<R>,
    plugins: Vec<Box<dyn Plugin<R>>>,
) -> tauri::Builder<R> {
    plugins
        .into_iter()
        .fold(base, tauri::Builder::plugin_boxed)
        .invoke_handler(commands::handler())
}

/// What the setup hook does once the app exists: hands the state to Tauri and registers the updater. The tests
/// call it themselves with their own parts, because the mock runtime never runs the setup hook.
///
/// # Errors
/// The updater plugin cannot start.
pub fn install<R: Runtime>(app: &AppHandle<R>, parts: AppParts) -> Result<(), SetupError> {
    app.manage(AppState::new(parts));
    // Updater (ADR-0003): nothing is checked at startup. The window asks only when the user clicks,
    // and the plugin verifies the signature against the `pubkey` in tauri.conf.json before installing.
    #[cfg(desktop)]
    app.plugin(tauri_plugin_updater::Builder::new().build())?;
    Ok(())
}

/// Starts the app. Returns the error instead of panicking so `main` decides how to exit.
///
/// # Errors
/// Returns the Tauri error if the runtime, a plugin or the main window cannot start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    app_builder(tauri::Builder::default(), plugins())
        .setup(|app| install(app.handle(), system_parts(app.handle())?))
        .run(context())
}
