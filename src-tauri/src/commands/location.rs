//! Where a new vault goes (RF-97 to RF-99): the location the app proposes from the vault's name,
//! `Documents › AlmarioKS › <name>.kdbx`, or the one the person chose with the dialog, and in both cases the key
//! file beside it.
//!
//! The rules (file name, key file, "is it the default folder") are the core's (`almarioks_core::location`); this
//! module asks the system for Documents and looks at the disk. `vault_create_location` writes nothing.

use std::path::PathBuf;

use almarioks_core::location::VaultLocation;
use serde::{Deserialize, Serialize};
use tauri::{State, ipc::Request};

use super::{
    args::{nullable, parse_args},
    blocking,
    error::AppError,
    vault::{path_arg, something_at},
};
use crate::state::AppState;

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
