//! AlmarioKS desktop app. Thin shell around `almarioks-core`: the commands of the window, the clipboard and the
//! plugins. The vault itself lives in the core.

use std::sync::{Arc, OnceLock};

use almarioks_core::{CoreResult, kdbx::NewVaultKdf};
use tauri::{AppHandle, Emitter, Manager, RunEvent, Runtime, plugin::Plugin};

pub mod clipboard;
pub mod commands;
pub mod lock;
pub mod state;
pub mod webview;

use clipboard::ClipboardService;
use commands::{dialogs::SystemDialogs, url::SystemUrlOpener};
use lock::{
    SystemIdleClock, VAULT_LOCKED_EVENT, VAULT_SAVE_STATE_EVENT, WebviewReloader, WindowEvents,
};
use state::{AppParts, AppState, SETTINGS_FILE_NAME};

/// What can stop the app from starting.
pub type SetupError = Box<dyn std::error::Error>;

/// The context of `tauri.conf.json`: configuration, capabilities and the list of allowed commands. The tests
/// build their app from this same context, so they exercise the real allow-list (RF-79).
#[must_use]
pub fn context<R: Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

/// Label of the only window of the app (`tauri.conf.json`).
pub const MAIN_WINDOW: &str = "main";

/// The plugins registered on the builder, in the order they are registered.
///
/// - `tauri-plugin-single-instance` goes FIRST (RF-81): plugins start in this order, and a second instance must
///   hand over to the running one and exit before anything else of it starts. It has no commands and no
///   JavaScript side, so it needs no permission.
/// - `tauri-plugin-dialog` is here only so Rust can open file dialogs (`commands::dialogs`): the capability
///   grants the window none of its commands.
/// - `tauri-plugin-opener` is not registered at all: `commands::url` calls its `open_url` function directly.
#[must_use]
pub fn plugins<R: Runtime>() -> Vec<Box<dyn Plugin<R>>> {
    let dialog: Box<dyn Plugin<R>> = Box::new(tauri_plugin_dialog::init());
    #[cfg(desktop)]
    let plugins = vec![single_instance(), dialog];
    #[cfg(not(desktop))]
    let plugins = vec![dialog];
    plugins
}

/// The single-instance plugin: when the app is started again, the running instance shows its window. The
/// arguments and the folder of the second instance are ignored on purpose: nothing is opened from them.
#[cfg(desktop)]
fn single_instance<R: Runtime>() -> Box<dyn Plugin<R>> {
    Box::new(tauri_plugin_single_instance::init(|app, _args, _folder| {
        show_main_window(app)
    }))
}

/// Brings the existing window to the front (RF-81): what the running instance does when the person starts the
/// app again. Showing it does not unlock anything: a locked vault stays locked.
pub fn show_main_window<R: Runtime>(app: &AppHandle<R>) {
    let Some(window) = app.get_webview_window(MAIN_WINDOW) else {
        return;
    };
    // Best effort, step by step: a window that is not minimized, or that the system will not let us focus,
    // must not stop the other steps, and there is nobody to report to.
    drop(window.unminimize());
    drop(window.show());
    drop(window.set_focus());
}

/// The parts of the real app: the settings file in the configuration folder Tauri gives (never a fixed path),
/// the Windows clipboard, the system's file dialogs, the default browser and the Documents folder of the system.
///
/// # Errors
/// The configuration folder of the app cannot be resolved.
pub fn system_parts<R: Runtime>(app: &AppHandle<R>) -> Result<AppParts, SetupError> {
    let settings_file = app.path().app_config_dir()?.join(SETTINGS_FILE_NAME);
    let documents = app.clone();
    Ok(AppParts {
        settings_file,
        // If the clipboard cannot start the app starts anyway; copying answers the error.
        clipboard: system_clipboard(),
        dialogs: Arc::new(SystemDialogs::new(app.clone())),
        url_opener: Arc::new(SystemUrlOpener),
        idle_clock: Arc::new(SystemIdleClock),
        reloader: Arc::new(AppWindow(app.clone())),
        new_vault_kdf: Arc::new(calibrated_kdf),
        // The known folder of the system (it may be redirected, or inside a synchronized folder): never a path
        // put together by hand. Without one the app starts all the same and proposes no location (RF-97).
        documents_dir: Arc::new(move || documents.path().document_dir().ok()),
    })
}

/// The KDF of a new vault on this machine (RF-2): calibrated the first time a vault is created or converted
/// (a few seconds) and kept for the rest of the run, since the machine does not change meanwhile.
fn calibrated_kdf() -> CoreResult<NewVaultKdf> {
    static CHOSEN: OnceLock<NewVaultKdf> = OnceLock::new();
    if let Some(kdf) = CHOSEN.get() {
        return Ok(*kdf);
    }
    let kdf = NewVaultKdf::calibrate_on_this_machine()?;
    Ok(*CHOSEN.get_or_init(|| kdf))
}

/// The window, as the lock sees it: the events it hears and its reload. Holds only the app handle.
struct AppWindow<R: Runtime>(AppHandle<R>);

impl<R: Runtime> WindowEvents for AppWindow<R> {
    fn vault_locked(&self) {
        // Best effort: with no window left there is nobody to tell, and the vault is locked either way.
        drop(self.0.emit(VAULT_LOCKED_EVENT, ()));
    }

    fn save_state(&self, state: &'static str) {
        // Best effort, as above: `vault_status` still says whether a save is pending.
        drop(self.0.emit(VAULT_SAVE_STATE_EVENT, state));
    }
}

impl<R: Runtime> WebviewReloader for AppWindow<R> {
    fn reload(&self) {
        reload_main_webview(&self.0);
    }
}

/// Reloads the page of the main window, so nothing a script kept from the vault survives a lock (RF-32,
/// AMENAZAS W2). `WebviewWindow::reload` only posts a message to the event loop: it does not block the lock.
///
/// Verified by hand, once per release: the mock runtime of the tests accepts the call and reloads nothing.
pub fn reload_main_webview<R: Runtime>(app: &AppHandle<R>) {
    if let Some(window) = app.get_webview_window(MAIN_WINDOW) {
        // Before the reload, and through the same queue, so the settings are in force for the navigation the
        // reload starts: no context menu and no keys of the browser (D-9).
        webview::renew_browser_surface(&window);
        // Best effort: a window that is closing has nothing left to reload.
        drop(window.reload());
    }
}

/// The app is exiting: closes the vault as a lock does and empties the clipboard if its content is ours (RF-36,
/// RF-75). Waits a bounded time; safe to call more than once.
pub fn shutdown<R: Runtime>(app: &AppHandle<R>) {
    if let Some(state) = app.try_state::<AppState>() {
        state.lock_center().shutdown();
    }
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

/// What the setup hook does once the app exists: hands the state (with a closed session) to Tauri and registers
/// the updater. The tests
/// call it themselves with their own parts, because the mock runtime never runs the setup hook.
///
/// # Errors
/// The updater plugin cannot start.
pub fn install<R: Runtime>(app: &AppHandle<R>, parts: AppParts) -> Result<(), SetupError> {
    app.manage(AppState::new(parts, Arc::new(AppWindow(app.clone()))));
    // Updater (ADR-0003): nothing is checked at startup. The window asks only when the user clicks,
    // and the plugin verifies the signature against the `pubkey` in tauri.conf.json before installing.
    //
    // On Windows the plugin's `install` starts the installer and ends the process with `std::process::exit`:
    // `RunEvent::Exit`, and with it `shutdown`, never comes. The only hook on that path is the plugin's own
    // `on_before_exit`, which its `Builder` does not expose (the plugin sets it to `cleanup_before_exit`), so
    // there is no second barrier here short of reimplementing check, download and install in Rust. What closes
    // the vault and clears the clipboard before an update is the command `app_prepare_update`, which the window
    // must await BEFORE `install()` (`tests/commands_contract.rs` reads `UpdatePanel.tsx` to hold it to that).
    #[cfg(desktop)]
    app.plugin(tauri_plugin_updater::Builder::new().build())?;
    Ok(())
}

/// Starts the app. Returns the error instead of panicking so `main` decides how to exit.
///
/// # Errors
/// Returns the Tauri error if the runtime, a plugin or the main window cannot start.
///
/// # Panics
/// Tauri 2.12 runs the setup hook when the event loop is ready and panics if it fails ("Failed to setup app"):
/// an error of the hook (the browser surface, the state, the heartbeat) ends the process instead of coming back
/// from here. Either way the app does not start.
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    let app = app_builder(tauri::Builder::default(), plugins())
        .setup(|app| {
            // First of all, with the window just created and before anybody can use it: no context menu and no
            // keys of the browser (D-9). An error here stops the start, as the two below do.
            webview::close_browser_surface(app.handle())?;
            install(app.handle(), system_parts(app.handle())?)?;
            // Only the real app has the heartbeat: the tests evaluate the timer by hand, with their own clocks.
            lock::start_heartbeat(app.state::<AppState>().lock_center())?;
            Ok(())
        })
        .build(context())?;
    app.run(|app, event| {
        // `Exit` comes once, after the last window closed and before the process ends.
        if matches!(event, RunEvent::Exit) {
            shutdown(app);
        }
    });
    Ok(())
}
