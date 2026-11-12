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
//! The vault commands (`vault_*`, `group_*`, `entry_*`, `search`, `activity`) and the lock are not here yet: they
//! need the session of the core in `crate::state::AppState` (T13, T14). `entry_open_url` only has to read the URL
//! of the entry and call [`url::open_url`].

pub mod args;
pub mod error;
mod list;
pub mod settings;

pub use args::{SecretArg, expect_no_args, parse_args};
pub use error::{APP_ONLY_CODES, AppError, INTERNAL_CODE};
pub use list::COMMANDS;

/// The handler with every command of [`COMMANDS`], for any runtime (the real one and the tests' mock).
pub fn handler<R: tauri::Runtime>() -> impl Fn(tauri::ipc::Invoke<R>) -> bool + Send + Sync + 'static
{
    tauri::generate_handler![
        settings::recent_list,
        settings::recent_remove,
        settings::settings_get,
        settings::settings_set,
    ]
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
