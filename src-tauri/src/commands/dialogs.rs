//! File dialogs, opened by Rust (plan, "Decisiones"): the window asks "pick a vault" and gets a path or nothing. It
//! never gets a `dialog:*` or `fs:*` permission; `tauri-plugin-dialog` is used only from this module.
//!
//! The commands talk to [`FileDialogs`], so the tests answer for the person without opening a system window.

use std::{
    path::{Path, PathBuf},
    sync::mpsc::sync_channel,
};

use tauri::{AppHandle, Manager, Runtime, State, ipc::Request};
use tauri_plugin_dialog::{Dialog, FileDialogBuilder, FilePath};

use super::{args::expect_no_args, blocking, error::AppError};
use crate::state::AppState;

/// Label of the window the dialogs belong to.
const MAIN_WINDOW: &str = "main";

/// The four dialogs of the contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FilePick {
    /// `pick_vault_to_open`: an existing `.kdbx` (RF-10).
    VaultToOpen,
    /// `pick_vault_to_create`: where the new `.kdbx` will go (RF-1).
    VaultToCreate,
    /// `pick_key_file`: an existing key file, of any kind (RF-87).
    KeyFile,
    /// `pick_new_key_file`: where the new `.keyx` will go (RF-5).
    NewKeyFile,
}

/// Whether the dialog picks a file that exists or a place for a new one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DialogMode {
    /// Pick an existing file.
    Open,
    /// Pick the name and folder of a file to be written.
    Save,
}

/// A row of the "file type" list of a dialog.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FileFilter {
    /// What the person reads.
    pub name: &'static str,
    /// Extensions without the dot; `*` is any file.
    pub extensions: &'static [&'static str],
}

/// How a dialog looks: title, kind and file types. The first extension of the first filter is the one a save
/// dialog adds when the person types a name without one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogPlan {
    /// Title of the dialog, in Spanish.
    pub title: &'static str,
    /// Open or save.
    pub mode: DialogMode,
    /// File types offered, the first one selected.
    pub filters: &'static [FileFilter],
}

const VAULTS: FileFilter = FileFilter {
    name: "Bóvedas KeePass (*.kdbx)",
    extensions: &["kdbx"],
};
const KEY_FILES: FileFilter = FileFilter {
    name: "Archivos llave (*.keyx, *.key)",
    extensions: &["keyx", "key"],
};
const NEW_KEY_FILES: FileFilter = FileFilter {
    name: "Archivo llave (*.keyx)",
    extensions: &["keyx"],
};
const ANY_FILE: FileFilter = FileFilter {
    name: "Todos los archivos",
    extensions: &["*"],
};

impl FilePick {
    /// The dialog for this pick.
    #[must_use]
    pub const fn plan(self) -> DialogPlan {
        match self {
            Self::VaultToOpen => DialogPlan {
                title: "Abrir una bóveda",
                mode: DialogMode::Open,
                filters: &[VAULTS],
            },
            Self::VaultToCreate => DialogPlan {
                title: "Crear una bóveda",
                mode: DialogMode::Save,
                filters: &[VAULTS],
            },
            // RF-87: KeePass takes any file as a key file (a photo, an old `.key`), so any file can be picked.
            Self::KeyFile => DialogPlan {
                title: "Elegir el archivo llave",
                mode: DialogMode::Open,
                filters: &[KEY_FILES, ANY_FILE],
            },
            Self::NewKeyFile => DialogPlan {
                title: "Guardar el archivo llave nuevo",
                mode: DialogMode::Save,
                filters: &[NEW_KEY_FILES],
            },
        }
    }
}

/// The system could not show the dialog (the plugin is missing or the app is closing). Not a cancellation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DialogUnavailable;

/// Asks the person for a file. The call blocks until they answer: it runs inside `spawn_blocking`.
pub trait FileDialogs: Send + Sync {
    /// The chosen path, or `None` if the person closed the dialog.
    ///
    /// A save dialog only returns a path: it creates and overwrites nothing. If the file exists, the core answers
    /// `ArchivoExiste` when asked to create it (RF-4, RF-6).
    ///
    /// # Errors
    /// [`DialogUnavailable`] if the dialog could not be shown.
    fn pick(&self, pick: FilePick) -> Result<Option<PathBuf>, DialogUnavailable>;
}

/// The dialogs of the operating system, through `tauri-plugin-dialog` called from Rust.
pub struct SystemDialogs<R: Runtime> {
    app: AppHandle<R>,
}

impl<R: Runtime> SystemDialogs<R> {
    /// Dialogs that belong to the main window of `app`.
    #[must_use]
    pub const fn new(app: AppHandle<R>) -> Self {
        Self { app }
    }

    /// Whether the dialog plugin is registered in the app.
    #[must_use]
    pub fn is_available(&self) -> bool {
        self.app.try_state::<Dialog<R>>().is_some()
    }
}

impl<R: Runtime> FileDialogs for SystemDialogs<R> {
    fn pick(&self, pick: FilePick) -> Result<Option<PathBuf>, DialogUnavailable> {
        // `try_state`, not `app.dialog()`: the extension trait panics when the plugin is not registered.
        let dialog = self.app.try_state::<Dialog<R>>().ok_or(DialogUnavailable)?;
        let plan = pick.plan();
        let mut builder = FileDialogBuilder::new(dialog.inner().clone()).set_title(plan.title);
        for filter in plan.filters {
            builder = builder.add_filter(filter.name, filter.extensions);
        }
        if let Some(window) = self.app.get_webview_window(MAIN_WINDOW) {
            // Modal over the main window, so it cannot end up hidden behind it.
            builder = builder.set_parent(&window);
        }

        // The plugin shows the dialog from the main thread and calls back from another one. Its own
        // `blocking_*` helpers panic if the app is closing and the callback never comes; this does not.
        let (answer, picked) = sync_channel::<Option<FilePath>>(1);
        let reply = move |path: Option<FilePath>| {
            // The command may be gone if the app is closing: nobody is left to tell.
            drop(answer.send(path));
        };
        match plan.mode {
            DialogMode::Open => builder.pick_file(reply),
            DialogMode::Save => builder.save_file(reply),
        }
        match picked.recv().map_err(|_| DialogUnavailable)? {
            None => Ok(None),
            Some(path) => path.into_path().map(Some).map_err(|_| DialogUnavailable),
        }
    }
}

/// Shared body of the four commands: no arguments in, a path or `null` out.
async fn pick(
    state: State<'_, AppState>,
    request: Request<'_>,
    pick: FilePick,
) -> Result<Option<String>, AppError> {
    expect_no_args(request.body())?;
    let (dialogs, picked_vault) = (state.dialogs(), state.picked_vault());
    let picked_key_file = state.picked_key_file();
    blocking(move || {
        let Some(path) = dialogs.pick(pick).map_err(|_| AppError::internal())? else {
            return Ok(None);
        };
        // A path that is not valid text cannot travel to the window nor be stored in the recent list.
        let path = path
            .into_os_string()
            .into_string()
            .map_err(|_| AppError::internal())?;
        if pick == FilePick::VaultToOpen {
            // RF-110: the vault the person is about to unlock. Only this dialog counts, and only its last answer.
            picked_vault.set(Path::new(&path));
        }
        if pick == FilePick::KeyFile {
            // RF-111: the key file the person chose with their own hand. Only this dialog counts, and only its
            // last answer: a path that did not come from here is never remembered as a vault's key file.
            picked_key_file.set(Path::new(&path));
        }
        Ok(Some(path))
    })
    .await
}

/// Asks for the vault to open. Answers its path, or `null` if the person cancelled.
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call; `ErrorInterno` if the dialog cannot be shown.
#[tauri::command]
pub async fn pick_vault_to_open(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Option<String>, AppError> {
    pick(state, request, FilePick::VaultToOpen).await
}

/// Asks where to create a vault. Answers the path, or `null` if the person cancelled. Writes nothing.
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call; `ErrorInterno` if the dialog cannot be shown.
#[tauri::command]
pub async fn pick_vault_to_create(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Option<String>, AppError> {
    pick(state, request, FilePick::VaultToCreate).await
}

/// Asks for an existing key file, of any kind (RF-87). Answers its path, or `null` if the person cancelled.
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call; `ErrorInterno` if the dialog cannot be shown.
#[tauri::command]
pub async fn pick_key_file(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Option<String>, AppError> {
    pick(state, request, FilePick::KeyFile).await
}

/// Asks where to save a new key file. Answers the path, or `null` if the person cancelled. Writes nothing.
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call; `ErrorInterno` if the dialog cannot be shown.
#[tauri::command]
pub async fn pick_new_key_file(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Option<String>, AppError> {
    pick(state, request, FilePick::NewKeyFile).await
}
