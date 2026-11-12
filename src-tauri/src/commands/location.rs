//! Where a new vault goes (RF-97 to RF-99): the location the app proposes from the vault's name,
//! `Documents › AlmarioKS › <name>.kdbx`, or the one the person chose with the dialog, and in both cases the key
//! file beside it.
//!
//! The rules (file name, key file, "is it the default folder") are the core's (`almarioks_core::location`); this
//! module asks the system for Documents and looks at the disk. `vault_create_location` writes nothing: the folder
//! `AlmarioKS` is created by `vault_create`, through [`ensure_default_folder`], and it is the only folder the app
//! ever creates.
//!
//! Also here, because it is the same rule read the other way: `unlock_key_suggestion` proposes, for a vault that
//! is about to be unlocked, the key file beside it (RF-110) or the one it was last opened with (RF-111).

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use almarioks_core::{
    CoreError,
    location::{VaultLocation, default_folder, is_in_default_folder},
    session::{Session, Status},
    settings::Settings,
};
use serde::{Deserialize, Serialize};
use tauri::{State, ipc::Request};

use super::{
    args::{nullable, parse_args},
    blocking,
    error::AppError,
    vault::{path_arg, something_at},
};
use crate::state::{AppState, PickedVault};

/// `vault_create_location{name, chosen_path}`. `chosen_path` is always sent: `null` while the person has not
/// chosen another location, or the path the dialog (`pick_vault_to_create`) answered.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultCreateLocationArgs {
    name: String,
    #[serde(deserialize_with = "nullable")]
    chosen_path: Option<String>,
}

/// Where the vault and its key file would go, and whether something is already there.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CreateLocationView {
    /// The vault file: what `vault_create` takes as `path`.
    vault_path: String,
    /// Its file name with extension, to show it.
    vault_file: String,
    /// The key file beside the vault: what `vault_create` takes as `key_file: { kind: "generate", path }`.
    key_file_path: String,
    /// Its file name with extension, to show it.
    key_file: String,
    /// Whether the vault goes directly inside `Documents › AlmarioKS`, which `vault_create` creates if missing.
    in_default_folder: bool,
    /// Something is already at `vault_path`: `vault_create` would answer `ArchivoExiste` (RF-98).
    vault_exists: bool,
    /// Something is already at `key_file_path`: generating the key file there would answer `ArchivoExiste`.
    key_file_exists: bool,
}

/// A path of ours as the text the window gets: only if it is valid text, never an approximation of it.
fn text(path: PathBuf) -> Result<String, AppError> {
    path.into_os_string()
        .into_string()
        .map_err(|_| AppError::internal())
}

/// Where the vault called `name` would be created (RF-97) and where its new key file would go (RF-99). Reads the
/// disk only to say whether those two paths are taken (RF-98); creates nothing, not even the folder.
///
/// Without `chosen_path` the answer is `Documents\AlmarioKS\<file name from the name>.kdbx`; with it, that same
/// path. The name is not validated here (`vault_create` does): an empty one still gets a proposal.
///
/// # Errors
/// `ArgumentoInvalido` (also a `chosen_path` that is empty or has no file name); `ErrorInterno` if no location
/// can be proposed because the system gives no Documents folder (a `chosen_path` still works then).
#[tauri::command]
pub async fn vault_create_location(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<CreateLocationView, AppError> {
    let args: VaultCreateLocationArgs = parse_args(request.body())?;
    let chosen = args.chosen_path.as_deref().map(path_arg).transpose()?;
    let documents = state.documents_dir();
    blocking(move || {
        let documents = documents();
        let location = match chosen {
            Some(path) => VaultLocation::chosen(documents.as_deref(), &path)
                .ok_or_else(AppError::invalid_argument)?,
            None => VaultLocation::proposed(&documents.ok_or_else(AppError::internal)?, &args.name),
        };
        Ok(CreateLocationView {
            vault_exists: something_at(&location.vault_path),
            key_file_exists: something_at(&location.key_file_path),
            vault_path: text(location.vault_path)?,
            vault_file: location.vault_file,
            key_file_path: text(location.key_file_path)?,
            key_file: location.key_file,
            in_default_folder: location.in_default_folder,
        })
    })
    .await
}

/// `unlock_key_suggestion{path}`: the vault that is about to be unlocked.
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UnlockKeySuggestionArgs {
    path: String,
}

/// Where a proposed key file comes from, when it is not the one beside the vault (RF-111).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeySource {
    /// The key file this vault was last opened with.
    Remembered,
}

/// What `unlock_key_suggestion` answers when there is something to propose or to say (RF-110, RF-111): where a
/// key file is, never what it holds. A field that does not apply is left out of the answer, so the proposal of
/// the key file beside the vault is the two fields RF-110 has always answered:
///
/// | Answer | Meaning |
/// |---|---|
/// | `key_file_path`, `key_file` | the key file beside the vault |
/// | those two and `source: "remembered"` | the key file this vault was last opened with |
/// | those two and `remembered_missing` | that one is not there; the one beside the vault is proposed |
/// | `remembered_missing` alone | that one is not there, and there is none beside the vault |
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
pub struct KeySuggestionView {
    /// The key file: what `vault_open` takes as `key_file_path`.
    #[serde(skip_serializing_if = "Option::is_none")]
    key_file_path: Option<String>,
    /// Its file name with extension, to show it.
    #[serde(skip_serializing_if = "Option::is_none")]
    key_file: Option<String>,
    /// Present only for the remembered key file; the one beside the vault has no `source`.
    #[serde(skip_serializing_if = "Option::is_none")]
    source: Option<KeySource>,
    /// The key file remembered for this vault is not at this path any more (RF-111). It is still remembered: it
    /// may live on a drive that is not plugged in.
    #[serde(skip_serializing_if = "Option::is_none")]
    remembered_missing: Option<String>,
}

/// Whether `path` is a vault the person is about to unlock, as far as the app itself knows (RF-110): the locked
/// vault of the session, the last vault chosen in the "open a vault" dialog during this run, or a recent vault
/// of `settings`. Those are paths the window already has from the app; any other path is not answered about, so
/// the command adds no way to ask the disk what exists to the ones there already are (review 0.1, P-3 and Q-1).
///
/// Paths are compared as the recent list compares them: as written.
fn is_unlock_candidate(
    path: &Path,
    session: &Session,
    picked: &PickedVault,
    settings: &Settings,
) -> bool {
    let locked = matches!(session.status(), Status::Locked { path: locked } if locked == path);
    locked || picked.is(path) || settings.recent_vaults().iter().any(|recent| recent == path)
}

/// Whether `path` can be proposed as a key file. Asks the disk one question, "is a file at that path?", without
/// opening it and without following a link: a folder or a link there is not a key file to propose.
fn is_key_file_there(path: &Path) -> bool {
    path.symlink_metadata().is_ok_and(|found| found.is_file())
}

/// The key file beside `vault_path`, if a file is there: its path and its name. Same folder, same name, `.keyx`
/// (the rule of `almarioks_core::location::key_file_name`).
fn key_file_beside(vault_path: &Path) -> Option<(String, String)> {
    let location = VaultLocation::chosen(None, vault_path)?;
    if !is_key_file_there(&location.key_file_path) {
        return None;
    }
    Some((
        location.key_file_path.to_str()?.to_owned(),
        location.key_file,
    ))
}

/// What to propose for the vault at `vault_path`, a vault the person is about to unlock (RF-110, RF-111).
/// `remembered` is the key file the settings remember for it, which wins over the one beside the vault.
///
/// If the remembered one is not there, the answer says so with its path and falls back to the one beside the
/// vault. It is not forgotten for that: nothing is written here.
fn suggestion_for(vault_path: &Path, remembered: Option<&Path>) -> Option<KeySuggestionView> {
    // The settings only hold paths that are valid text with a file name after the last separator
    // (`is_rememberable_key_file`).
    let remembered = remembered.and_then(|path| {
        let text = path.to_str()?;
        Some((path, text, text.rsplit(['\\', '/']).next()?))
    });
    if let Some((path, text, name)) = remembered
        && is_key_file_there(path)
    {
        return Some(KeySuggestionView {
            key_file_path: Some(text.to_owned()),
            key_file: Some(name.to_owned()),
            source: Some(KeySource::Remembered),
            remembered_missing: None,
        });
    }
    let remembered_missing = remembered.map(|(_, text, _)| text.to_owned());
    match key_file_beside(vault_path) {
        Some((key_file_path, key_file)) => Some(KeySuggestionView {
            key_file_path: Some(key_file_path),
            key_file: Some(key_file),
            source: None,
            remembered_missing,
        }),
        None => remembered_missing.map(|missing| KeySuggestionView {
            remembered_missing: Some(missing),
            ..KeySuggestionView::default()
        }),
    }
}

/// The key file to propose when the person is about to unlock the vault at `path`: the one it was last opened
/// with, if the settings remember one for it (RF-111), or else the `X.keyx` beside `X.kdbx` (RF-110); `null` if
/// there is neither. The shapes of the answer are in [`KeySuggestionView`]. Reads nothing of a key file and writes
/// nothing; the person can still remove what is proposed or choose another one, and `vault_open` is what reads
/// it.
///
/// It only answers for a vault the app knows the person is about to unlock ([`is_unlock_candidate`]). For any
/// other path the answer is `null` without looking at the disk: the same answer as "there is no key file", never
/// an error that would tell the two apart. A key file is remembered only for a recent vault, and a path of the
/// settings file that could not have been remembered was dropped when the file was read: it never gets here.
///
/// # Errors
/// `ArgumentoInvalido` if the arguments are malformed or the path is empty.
#[tauri::command]
pub async fn unlock_key_suggestion(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<Option<KeySuggestionView>, AppError> {
    let args: UnlockKeySuggestionArgs = parse_args(request.body())?;
    let path = path_arg(&args.path)?;
    let (session, picked, settings) = (state.session(), state.picked_vault(), state.settings());
    blocking(move || {
        // One reading of the settings for both questions: is it a recent vault, and what is remembered for it.
        let settings = settings.read();
        if !is_unlock_candidate(&path, &session, &picked, &settings) {
            return Ok(None);
        }
        Ok(suggestion_for(&path, settings.key_file(&path)))
    })
    .await
}

/// The folder `Documents › AlmarioKS` if this call created it. Dropped without [`Self::keep`], it is removed
/// again, and only while it is still empty: a creation that fails leaves nothing behind.
#[derive(Debug)]
pub(super) struct MadeFolder(Option<PathBuf>);

impl MadeFolder {
    /// The vault was created inside: the folder stays.
    pub(super) fn keep(mut self) {
        self.0 = None;
    }
}

impl Drop for MadeFolder {
    fn drop(&mut self) {
        if let Some(folder) = &self.0 {
            // Best effort, and `remove_dir` only removes an empty folder: nothing of the person's can go with it.
            _ = std::fs::remove_dir(folder);
        }
    }
}

/// RF-97: creates the folder `Documents › AlmarioKS` if `vault_path` is a file directly inside it and the folder
/// is missing. For any other path it does nothing: the app creates no folder the window asks for, only this one.
///
/// # Errors
/// `GuardadoFallido` if the folder is missing and cannot be created.
pub(super) fn ensure_default_folder(
    documents: Option<&Path>,
    vault_path: &Path,
) -> Result<MadeFolder, AppError> {
    let Some(documents) = documents else {
        return Ok(MadeFolder(None));
    };
    if !is_in_default_folder(documents, vault_path) {
        return Ok(MadeFolder(None));
    }
    // One level only: Documents itself is the system's to create.
    let folder = default_folder(documents);
    match std::fs::create_dir(&folder) {
        Ok(()) => Ok(MadeFolder(Some(folder))),
        // Already there. If it is a file and not a folder, creating the vault inside fails on its own.
        Err(error) if error.kind() == ErrorKind::AlreadyExists => Ok(MadeFolder(None)),
        Err(_) => Err(CoreError::GuardadoFallido.into()),
    }
}
