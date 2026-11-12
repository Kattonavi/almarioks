//! Commands of the window (spec 001, section J; plan, "Contrato entre la ventana y el núcleo").
//!
//! Rules every command follows:
//! - It answers `Result<T, AppError>`; [`AppError`] is `{ code, message, field?, limit? }`.
//! - It takes the raw [`tauri::ipc::Request`] and reads its arguments with [`parse_args`] (one `args` struct with
//!   `deny_unknown_fields` and `snake_case` fields) or [`expect_no_args`]: a malformed call always answers
//!   `ArgumentoInvalido`, never Tauri's text (RF-80).
//! - It is `async`, and anything that touches the disk, the screen or the CPU for long goes through [`blocking`].
//!
//! # Where a new command is registered (three places, checked by `tests/commands_contract.rs`)
//! 1. Its name in [`COMMANDS`] (`list.rs`, which `build.rs` includes).
//! 2. Its path in [`handler`], below.
//! 3. `allow-<name-with-dashes>` in `capabilities/main-window.json`.
//!
//! # Order of the checks in a command of the vault (spec, "Códigos de error")
//! `Bloqueada` › `ArgumentoInvalido` › `SoloLectura` › `NoEncontrado` › the rule of the domain. So a command that
//! reads or changes the vault calls [`require_open`] BEFORE reading its arguments, and leaves the rest of the order
//! to the session and the vault of the core. `entry_history` (RF-85, recortable) is not in this build.

pub mod args;
pub mod dialogs;
pub mod entries;
pub mod error;
pub mod groups;
mod list;
pub mod location;
pub mod passwords;
pub mod settings;
pub mod url;
pub mod vault;

use almarioks_core::{
    CoreError, FieldError,
    session::{Session, Status},
};

pub use args::{SecretArg, expect_no_args, parse_args};
pub use error::{APP_ONLY_CODES, AppError, INTERNAL_CODE};
pub use list::COMMANDS;

/// The handler with every command of [`COMMANDS`], for any runtime (the real one and the tests' mock).
pub fn handler<R: tauri::Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static
{
    tauri::generate_handler![
        vault::vault_status,
        vault::vault_create,
        vault::vault_open,
        vault::vault_lock,
        vault::vault_close,
        vault::app_prepare_update,
        vault::vault_convert,
        vault::vault_retry_save,
        vault::vault_save_as,
        vault::vault_reload,
        vault::activity,
        location::vault_create_location,
        groups::group_tree,
        groups::group_create,
        groups::group_rename,
        groups::group_move,
        groups::group_delete_preview,
        groups::group_delete,
        groups::recycle_bin_empty,
        entries::entry_list,
        entries::entry_get,
        entries::entry_create,
        entries::entry_update,
        entries::entry_move,
        entries::entry_delete,
        entries::entry_reveal,
        entries::entry_copy,
        entries::entry_open_url,
        entries::search,
        dialogs::pick_vault_to_open,
        dialogs::pick_vault_to_create,
        dialogs::pick_key_file,
        dialogs::pick_new_key_file,
        passwords::password_strength,
        passwords::generate_password,
        settings::recent_list,
        settings::recent_remove,
        settings::settings_get,
        settings::settings_set,
    ]
}

/// `Bloqueada` unless the vault is open (RF-35). Called before the arguments are even read, because `Bloqueada`
/// wins over `ArgumentoInvalido`. The session checks again when the work runs: this is only the order.
///
/// # Errors
/// `Bloqueada` while the vault is closed, locked or being unlocked.
pub(crate) fn require_open(session: &Session) -> Result<(), AppError> {
    match session.status() {
        Status::Open(_) => Ok(()),
        Status::Closed | Status::Locked { .. } | Status::Deriving { .. } => {
            Err(CoreError::Bloqueada.into())
        }
    }
}

/// Adds the field and the limit to a validation error of the core (plan, "Precisiones" 15). The core has already
/// decided WHICH error wins (`SoloLectura` and `NoEncontrado` come first); `check` only runs the same rules again
/// on the same input to learn the data, and is ignored if it does not name the same code.
pub(crate) fn with_field_data(
    error: CoreError,
    check: impl FnOnce() -> Result<(), FieldError>,
) -> AppError {
    match check() {
        Err(field) if field.error() == error => field.into(),
        Err(_) | Ok(()) => error.into(),
    }
}

/// Runs blocking work (disk, system dialogs, CPU) off the async workers and off the main thread.
///
/// # Errors
/// Whatever `work` answers, or [`AppError::internal`] if the worker died.
pub(crate) async fn blocking<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, AppError> + Send + 'static,
) -> Result<T, AppError> {
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| AppError::internal())?
}
