//! The vault as a whole: its state, creating, opening, locking, closing, converting and the save commands
//! (spec 001, sections A to D; plan, "Contrato" and "Precisiones" 2 to 4, 12).
//!
//! Deriving a key and encrypting are slow, so everything here runs through [`blocking`]; `vault_status` only reads
//! memory and answers at once also while another command derives (RF-18).
//!
//! The master password arrives as text inside `args`, is wrapped while it is read ([`SecretArg`]) and handed to
//! the core, which wipes it. It is kept nowhere else: unlocking again needs it typed again.

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use almarioks_core::{
    CoreError,
    kdbx::{KeyFile, MasterPassword, NewKeyFile},
    location::VaultLocation,
    session::{Recovery, Status},
    vault::validate_group_name,
};
use serde::{Deserialize, Deserializer, Serialize, de::Error as _};
use serde_json::Value;
use tauri::{State, ipc::Request};

use super::{
    args::{SecretArg, expect_no_args, parse_args, present},
    blocking,
    error::AppError,
    location::ensure_default_folder,
    require_open, with_field_data,
};
use crate::{
    lock::{End, LockCenter},
    state::{AppState, SettingsStore},
};

/// `recovery` of the answer to an unlock (RF-34; plan, "Precisiones" 3).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RecoveryView {
    /// The unsaved changes are in this recovery file.
    Saved {
        /// The recovery file.
        path: String,
    },
    /// The unsaved changes are lost.
    Lost,
}

/// The state of the vault as the window reads it (plan, "Precisiones" 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub enum StatusView {
    /// No vault chosen.
    Closed,
    /// A vault is chosen and locked.
    Locked {
        /// The vault file.
        path: String,
    },
    /// An unlock is running (RF-18).
    Deriving {
        /// The vault file.
        path: String,
    },
    /// The vault is open.
    Open {
        /// The vault file.
        path: String,
        /// The vault's name.
        name: String,
        /// KDBX 3.x: nothing can be changed until it is converted (RF-14).
        read_only: bool,
        /// Its protection against brute force is weak (RF-17).
        weak_kdf: bool,
        /// There are changes the disk has not confirmed.
        save_pending: bool,
        /// Only in the answer of the unlock that follows a lock with unsaved changes (RF-34).
        recovery: Option<RecoveryView>,
        /// Seconds left before the inactivity lock (RF-29), for the countdown of the window.
        locks_in_seconds: u64,
    },
}

fn text(path: &Path) -> String {
    // Every path of the session came from the window as text, so nothing is lost here.
    path.to_string_lossy().into_owned()
}

/// The status of the core as the window reads it. Reads the settings: blocking.
fn status_view(status: Status, center: &LockCenter) -> StatusView {
    match status {
        Status::Closed => StatusView::Closed,
        Status::Locked { path } => StatusView::Locked { path: text(&path) },
        Status::Deriving { path } => StatusView::Deriving { path: text(&path) },
        Status::Open(open) => StatusView::Open {
            path: text(&open.path),
            name: open.name,
            read_only: open.read_only,
            weak_kdf: open.weak_kdf,
            save_pending: open.save_pending,
            recovery: open.recovery.map(|recovery| match recovery {
                Recovery::Saved { path } => RecoveryView::Saved { path: text(&path) },
                Recovery::Lost => RecoveryView::Lost,
            }),
            locks_in_seconds: center.locks_in_seconds(),
        },
    }
}

/// RF-19: the vault goes first in the recent list. If the settings file cannot be written the vault is open all
/// the same, so the failure is not the answer of the command: the list simply does not change.
///
/// For the commands that put a NEW file at `path` (create, convert, save as): the key file remembered for
/// whatever was at that path before is not the key file of this vault, so it is forgotten (security review 0.1,
/// second round, U-1: a vault recreated there with a key file alone would otherwise open by itself). Only an
/// unlock that worked remembers one (`remember_unlocked`).
fn remember(settings: &SettingsStore, path: &Path) {
    drop(settings.update(|settings| {
        settings.remember_vault(path);
        settings.set_key_file(path, None);
        Ok(())
    }));
}

/// What an unlock that worked leaves to remember (RF-111).
struct Unlocked<'a> {
    /// The vault that opened.
    path: &'a Path,
    /// The key file it opened with, if any.
    key_file: Option<&'a Path>,
    /// The unlock carried a master password that is not empty.
    with_password: bool,
    /// The last path `pick_key_file` answered in this run, if any.
    picked: Option<&'a Path>,
}

impl Unlocked<'_> {
    /// The key file to remember for this vault, given the one `remembered` so far, or `None` to remember none
    /// (security review 0.1, second round). All of these must hold:
    /// - the unlock carried a password (C-10): for a vault that opens with its key file alone, the remembered
    ///   path would be all a compromised window needs;
    /// - it is not the key file beside the vault (C-3), which `unlock_key_suggestion` finds on its own;
    /// - it came from a gesture of the person (C-3): it is the last one the dialog answered, or the one already
    ///   remembered for this vault.
    ///
    /// Whether the path may be written down at all (C-2) is decided by `Settings::set_key_file`.
    fn key_file_to_remember(&self, remembered: Option<&Path>) -> Option<&Path> {
        let key_file = self.key_file.filter(|_| self.with_password)?;
        let beside = VaultLocation::chosen(None, self.path)
            .is_some_and(|location| location.key_file_path == key_file);
        let from_a_gesture = self.picked == Some(key_file) || remembered == Some(key_file);
        (!beside && from_a_gesture).then_some(key_file)
    }
}

/// RF-19 and RF-111, in one write of the settings file: the vault goes first in the recent list, and the key file
/// it opened with is remembered for it, replaced or forgotten ([`Unlocked::key_file_to_remember`]). As in
/// [`remember`], a settings file that cannot be written is not the answer of the command.
///
/// The only place that remembers a key file (C-1): creating, converting, saving as and the suggestion never do.
fn remember_unlock(settings: &SettingsStore, unlocked: &Unlocked<'_>) {
    drop(settings.update(|settings| {
        settings.remember_vault(unlocked.path);
        // Read here, inside the same change: what is remembered cannot move between the question and the answer.
        let keep = unlocked.key_file_to_remember(settings.key_file(unlocked.path));
        settings.set_key_file(unlocked.path, keep);
        Ok(())
    }));
}

/// A path the window sends: any text but the empty one.
pub(super) fn path_arg(text: &str) -> Result<PathBuf, AppError> {
    if text.is_empty() {
        return Err(AppError::invalid_argument());
    }
    Ok(PathBuf::from(text))
}

fn master_password(password: Option<&SecretArg>) -> Option<MasterPassword> {
    // The core takes its own copy and wipes it; ours is wiped when the arguments are dropped.
    password.map(|password| MasterPassword::new(password.expose().to_owned()))
}

/// `vault_open{path, password?, key_file_path?}`. No `Debug`: it holds the master password.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultOpenArgs {
    path: String,
    #[serde(default, deserialize_with = "present")]
    password: Option<SecretArg>,
    #[serde(default, deserialize_with = "present")]
    key_file_path: Option<String>,
}

/// `key_file` of `vault_create` (plan, "Precisiones" 4): `{ kind: "none" }`, `{ kind: "existing", path }` or
/// `{ kind: "generate", path }`. Read by hand so that it is exactly one of those objects: serde's derive would
/// also take a positional list, and would let extra keys through next to `none`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum KeyFileArg {
    /// Password only.
    None,
    /// A key file the person already has.
    Existing(String),
    /// A new `.keyx` at this path.
    Generate(String),
}

impl<'de> Deserialize<'de> for KeyFileArg {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let refused = || D::Error::custom("key_file");
        // Paths, never a secret: it may pass through a `Value`.
        let Value::Object(fields) = Value::deserialize(deserializer)? else {
            return Err(refused());
        };
        let kind = fields.get("kind").and_then(Value::as_str);
        let path = fields.get("path").and_then(Value::as_str);
        match (kind, path, fields.len()) {
            (Some("none"), None, 1) => Ok(Self::None),
            (Some("existing"), Some(path), 2) if !path.is_empty() => {
                Ok(Self::Existing(path.to_owned()))
            }
            (Some("generate"), Some(path), 2) if !path.is_empty() => {
                Ok(Self::Generate(path.to_owned()))
            }
            _ => Err(refused()),
        }
    }
}

/// `vault_create{path, name, password?, key_file}`. No `Debug`: it holds the master password.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultCreateArgs {
    path: String,
    name: String,
    #[serde(default, deserialize_with = "present")]
    password: Option<SecretArg>,
    key_file: KeyFileArg,
}

/// `vault_convert{new_path}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultConvertArgs {
    new_path: String,
}

/// `vault_save_as{path}`.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VaultSaveAsArgs {
    path: String,
}

/// Whether anything (a file, a folder or a link) is at `path`.
pub(super) fn something_at(path: &Path) -> bool {
    path.symlink_metadata().is_ok()
}

/// Reads the key file the person chose for a new vault.
fn existing_key_file(path: &Path) -> Result<KeyFile, CoreError> {
    match std::fs::read(path) {
        Ok(bytes) => KeyFile::from_bytes(bytes),
        Err(error) if error.kind() == ErrorKind::NotFound => {
            Err(CoreError::ArchivoLlaveNoEncontrado)
        }
        // Decisión pendiente, the same the core took: the spec has no code for a key file that is there and
        // cannot be read.
        Err(_) => Err(CoreError::ArchivoLlaveDanado),
    }
}

/// Where the vault is: closed, locked, being unlocked or open (RF-34, RF-35). Answers at once, also while
/// another command derives a key (RF-18).
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call.
#[tauri::command]
pub async fn vault_status(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<StatusView, AppError> {
    expect_no_args(request.body())?;
    let (session, center) = (state.session(), state.lock_center());
    blocking(move || Ok(status_view(session.status(), &center))).await
}

/// Creates a vault and leaves it open (RF-1 to RF-6, RF-9).
///
/// The folder of the vault must exist, with one exception (RF-97): if the vault goes directly inside
/// `Documents › AlmarioKS` and that folder is missing, it is created first (and removed again if the creation
/// fails). No other folder is ever created. Whatever the path, a file that is already there is never replaced
/// (RF-4, RF-98): the answer is `ArchivoExiste`, also for the new key file (RF-6, RF-99).
///
/// # Errors
/// In the order of the plan ("Precisiones" 12): `ArgumentoInvalido` › `LlaveVacia` › `ArchivoExiste` (vault) ›
/// `ArchivoExiste` (new key file); then those of the key file given (`ArchivoLlaveNoEncontrado`,
/// `ArchivoLlaveDanado`), the name's (`NombreVacio`, `CampoMuyLargo`, `CaracterNoPermitido`, with `field` and
/// `limit`), `Ocupada`, `GuardadoFallido`. Nothing that exists is touched.
#[tauri::command]
pub async fn vault_create(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<StatusView, AppError> {
    let args: VaultCreateArgs = parse_args(request.body())?;
    let path = path_arg(&args.path)?;
    let (session, center) = (state.session(), state.lock_center());
    let (settings, choose_kdf) = (state.settings(), state.new_vault_kdf());
    let documents = state.documents_dir();
    blocking(move || {
        // The cheap refusals first, in the order of the plan: choosing the KDF takes seconds and the person
        // should not wait for it to learn that the file exists. The core checks them all again.
        let no_password = args
            .password
            .as_ref()
            .is_none_or(|password| password.expose().is_empty());
        if no_password && args.key_file == KeyFileArg::None {
            return Err(CoreError::LlaveVacia.into());
        }
        if something_at(&path) {
            return Err(CoreError::ArchivoExiste.into());
        }
        let key_file = match &args.key_file {
            KeyFileArg::None => NewKeyFile::None,
            KeyFileArg::Existing(file) => NewKeyFile::Existing(existing_key_file(Path::new(file))?),
            KeyFileArg::Generate(file) => {
                if something_at(Path::new(file)) {
                    return Err(CoreError::ArchivoExiste.into());
                }
                NewKeyFile::Generate(PathBuf::from(file))
            }
        };
        let kdf = choose_kdf()?;
        // A lock that is still writing its recovery file ends first (see `vault_open`).
        center.settle();
        center.unlocking();
        // RF-97: after every cheap refusal and right before writing. Only `Documents › AlmarioKS`, and only when
        // the vault goes directly inside it; if the creation fails, the folder this call made goes away again.
        let made_folder = ensure_default_folder(documents().as_deref(), &path)?;
        let status = session
            .create(
                &path,
                &args.name,
                master_password(args.password.as_ref()),
                key_file,
                kdf,
            )
            .map_err(|error| {
                with_field_data(error, || validate_group_name(&args.name).map(drop))
            })?;
        made_folder.keep();
        center.activity();
        remember(&settings, &path);
        Ok(status_view(status, &center))
    })
    .await
}

/// Opens a vault with a master password, a key file or both (RF-10 to RF-17). The answer carries, once, what
/// happened to the changes a previous lock could not save (`recovery`, RF-34).
///
/// # Errors
/// `ArgumentoInvalido`; `Ocupada` (RF-88); `ArchivoNoEncontrado`, `ArchivoLlaveNoEncontrado` (RF-95);
/// `LlaveVacia`, `ArchivoLlaveDanado`, `LlaveIncorrecta`, `ArchivoDanado`, `FormatoNoSoportado`,
/// `ParametrosKdfExcesivos`; `Bloqueada` if a lock was asked for meanwhile. The vault stays as it was.
#[tauri::command]
pub async fn vault_open(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<StatusView, AppError> {
    let args: VaultOpenArgs = parse_args(request.body())?;
    let path = path_arg(&args.path)?;
    let key_file = args.key_file_path.as_deref().map(path_arg).transpose()?;
    let (session, center, settings) = (state.session(), state.lock_center(), state.settings());
    let picked_key_file = state.picked_key_file();
    blocking(move || {
        // Typing the key is activity: without this, a vault unlocked after a long wait at the unlock screen
        // could be locked by the heartbeat before this command notes the unlock.
        center.unlocking();
        // `vault_lock` answers as soon as the vault is locked, while the lock of the core may still be writing
        // the recovery file (up to 5 s, RF-33). Unlocking waits for it here instead of answering `Ocupada`.
        center.settle();
        let with_password = args
            .password
            .as_ref()
            .is_some_and(|password| !password.expose().is_empty());
        let status = session.unlock(
            &path,
            master_password(args.password.as_ref()),
            key_file.as_deref(),
        )?;
        center.activity();
        // Only after the unlock worked: a failed one leaves the settings file as it was (RF-111).
        let picked = picked_key_file.last();
        remember_unlock(
            &settings,
            &Unlocked {
                path: &path,
                key_file: key_file.as_deref(),
                with_password,
                picked: picked.as_deref(),
            },
        );
        Ok(status_view(status, &center))
    })
    .await
}

/// Locks the vault (RF-28). Answers as soon as it is locked: every command of the vault already answers
/// `Bloqueada` (RF-30), the clipboard is clean (RF-31) and the window was told and reloaded (RF-32). Writing a
/// recovery file, if one is needed, goes on behind (RF-33).
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call.
#[tauri::command]
pub async fn vault_lock(state: State<'_, AppState>, request: Request<'_>) -> Result<(), AppError> {
    expect_no_args(request.body())?;
    let center = state.lock_center();
    blocking(move || {
        center.end(End::Lock);
        Ok(())
    })
    .await
}

/// Closes the vault: the same as a lock, and the app forgets which file it was (RF-36).
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call.
#[tauri::command]
pub async fn vault_close(state: State<'_, AppState>, request: Request<'_>) -> Result<(), AppError> {
    expect_no_args(request.body())?;
    let center = state.lock_center();
    blocking(move || {
        center.end(End::Close);
        Ok(())
    })
    .await
}

/// The window is about to install an update it has downloaded: closes the vault as the exit of the app does
/// (RF-36, with the recovery file of RF-33) and empties the clipboard if its content is ours (RF-75), because on
/// Windows the installer ends the process and the app never sees that exit. The window is not told and not
/// reloaded, and the app goes on working if the installer does not start (`LockCenter::close_for_update`).
///
/// May take as long as a recovery file (5 s). Asking again is harmless.
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call.
#[tauri::command]
pub async fn app_prepare_update(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<(), AppError> {
    expect_no_args(request.body())?;
    let center = state.lock_center();
    blocking(move || {
        center.close_for_update();
        Ok(())
    })
    .await
}

/// Converts the open KDBX 3.x vault into a KDBX 4 file at a new path and goes on there; the original file is
/// not touched (RF-15).
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido` (also if the open vault is already KDBX 4); `Ocupada`; `ArchivoExiste`;
/// `GuardadoFallido`.
#[tauri::command]
pub async fn vault_convert(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<StatusView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: VaultConvertArgs = parse_args(request.body())?;
    let new_path = path_arg(&args.new_path)?;
    let (center, settings, choose_kdf) =
        (state.lock_center(), state.settings(), state.new_vault_kdf());
    blocking(move || {
        if something_at(&new_path) {
            return Err(CoreError::ArchivoExiste.into());
        }
        let kdf = choose_kdf()?;
        center.activity();
        let status = session.convert(&new_path, kdf)?;
        center.activity();
        remember(&settings, &new_path);
        Ok(status_view(status, &center))
    })
    .await
}

/// Tries again the save that failed (RF-23). How it went arrives by the event `vault-save-state`.
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `GuardadoFallido`.
#[tauri::command]
pub async fn vault_retry_save(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<StatusView, AppError> {
    let session = state.session();
    require_open(&session)?;
    expect_no_args(request.body())?;
    let center = state.lock_center();
    blocking(move || Ok(status_view(session.retry_save()?, &center))).await
}

/// Saves the vault at another path, where nothing may exist, and goes on there (RF-23, RF-24).
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `SoloLectura`; `Ocupada`; `ArchivoExiste`; `GuardadoFallido`.
#[tauri::command]
pub async fn vault_save_as(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<StatusView, AppError> {
    let session = state.session();
    require_open(&session)?;
    let args: VaultSaveAsArgs = parse_args(request.body())?;
    let path = path_arg(&args.path)?;
    let (center, settings) = (state.lock_center(), state.settings());
    blocking(move || {
        let status = session.save_as(&path)?;
        remember(&settings, &path);
        Ok(status_view(status, &center))
    })
    .await
}

/// Reads the vault file again, dropping what was pending (RF-24).
///
/// # Errors
/// `Bloqueada`; `ArgumentoInvalido`; `Ocupada`; `ArchivoNoEncontrado`, `LlaveIncorrecta`, `ArchivoDanado` (the
/// vault stays as it was).
#[tauri::command]
pub async fn vault_reload(
    state: State<'_, AppState>,
    request: Request<'_>,
) -> Result<StatusView, AppError> {
    let session = state.session();
    require_open(&session)?;
    expect_no_args(request.body())?;
    let center = state.lock_center();
    blocking(move || Ok(status_view(session.reload()?, &center))).await
}

/// The person used the window: the inactivity count starts again (RF-29). The window sends it at most once
/// every 10 s. It says nothing about the vault and works whatever its state.
///
/// # Errors
/// `ArgumentoInvalido` if anything comes with the call.
#[tauri::command]
pub async fn activity(state: State<'_, AppState>, request: Request<'_>) -> Result<(), AppError> {
    expect_no_args(request.body())?;
    state.lock_center().activity();
    Ok(())
}
