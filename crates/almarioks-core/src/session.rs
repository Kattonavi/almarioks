//! The session of the vault (T9): closed, locked, deriving or open; one unlock at a time (RF-88); every change
//! saved on its own (RF-21, D7); and a lock that never fails open (constitution rule 7; RF-28, RF-30 to RF-36).
//!
//! It joins what already exists: [`crate::kdbx::OpenedVault`] and [`crate::storage::SaveQueue`]. No Tauri, no
//! `async`: threads and `std` locks. The disk, the clock and whoever listens enter as traits.
//!
//! # Who waits for what
//! - The state ([`Session::status`]) is under a mutex that is only held to change memory: never while a key is
//!   derived, a vault is encrypted or the disk is touched. So it answers at once, also during an unlock (RF-18).
//! - Commands of the vault ([`Session::read`], [`Session::mutate`] and the save commands) run one at a time. The
//!   one that changes the vault encrypts it before answering, and encrypting derives the key again: about as long
//!   as an unlock (measured in `rf_30_what_encrypting_costs_is_measured`).
//! - The disk is only touched by the save thread, or by a command that holds no secret meanwhile.
//!
//! # A lock never waits
//! [`Session::lock`] and [`Session::close`] do, in this order (ADR-0002 as D12 puts it):
//! 1. The state says locked and nobody else gets a turn: every command that arrives or waits answers `Bloqueada`
//!    at once.
//! 2. The listener is told ([`SessionEvents::locked`]): the app unmounts the view and clears the clipboard.
//! 3. The save queue is closed, which drops the copy of the key its verifier had, and the newest encrypted bytes
//!    not confirmed on disk are taken from it.
//! 4. The vault and the key are dropped.
//! 5. Those bytes, if any, go to a recovery file on a thread that only has encrypted bytes, with a 5 s limit.
//! 6. The result is noted for the next unlock (RF-34).
//!
//! **The limit of step 4.** Nothing in the session can interrupt a command that is already running. If one is
//! encrypting when the lock is asked for, it finishes that encryption, finds the lock and answers `Bloqueada`; the
//! vault and the key are dropped right then. The same goes for the save thread if it is checking, at that moment,
//! that what it wrote decrypts. Both are one key derivation, they run side by side, and neither is ever tied to
//! the disk: so the secrets outlive the request to lock by at most ONE derivation (about 1 s in a vault made
//! here, RF-2), and only when a change or a save was in flight. Step 5 waits for that before it writes.
//!
//! That bound is the session's own work. The closures given to [`Session::read`] and [`Session::mutate`] run
//! with the vault in their hands: one that blocks keeps the vault and the key in memory for as long as it blocks
//! (every other command still answers `Bloqueada` at once). They must only touch memory and return.

mod clock;
mod gate;
mod live;
#[cfg(test)]
mod tests;

use std::{
    fmt,
    io::ErrorKind,
    mem,
    panic::{self, AssertUnwindSafe},
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, mpsc},
    thread,
    time::{Duration, UNIX_EPOCH},
};

pub use clock::{Clock, Ring, SystemClock};
use gate::{Released, hold};
use live::{Facts, Live, Secrets, Shared, Watch};

use crate::{
    error::{CoreError, CoreResult},
    kdbx::{self, KeyFile, MasterPassword, NewKeyFile, NewVaultKdf, OpenedVault, VaultKey},
    storage::{
        self, Disk, FailureCause, Fingerprint, SaveQueue, Verifier, read_vault, save_vault_as,
        write_recovery,
    },
    vault::Timestamp,
};

/// How long a lock waits for the recovery file (RF-33).
pub const RECOVERY_LIMIT: Duration = Duration::from_secs(5);

/// What happened to the changes that were not saved when the vault was locked (RF-34; plan, "Precisiones" 3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Recovery {
    /// They are in this recovery file, encrypted with the key of the vault.
    Saved {
        /// `<vault>.recuperacion-<AAAAMMDD-HHMMSS>.kdbx`.
        path: PathBuf,
    },
    /// They are lost.
    Lost,
}

/// Where the last change is (event `vault-save-state`; plan, "Precisiones" 13).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SaveState {
    /// A save is being written or waits its turn.
    Saving,
    /// The file has everything.
    Saved,
    /// The save failed; the file is as it was and the changes are in memory (RF-23).
    Failed(FailureCause),
    /// Another app changed the file; it was not overwritten (RF-24).
    ExternalChange,
}

impl SaveState {
    /// The value of the event: `saving`, `saved`, `failed` or `external-change`.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Saving => "saving",
            Self::Saved => "saved",
            Self::Failed(_) => "failed",
            Self::ExternalChange => "external-change",
        }
    }
}

/// Whoever listens to the session: the app turns these into its events and cleans what the core cannot see.
///
/// Both are called with no lock of the session held, so they may call [`Session::status`]. They must not block:
/// [`SessionEvents::locked`] runs inside the lock, before the keys are dropped.
pub trait SessionEvents: Send + Sync {
    /// A lock or a close was asked for (step 2): unmount the view, clear the clipboard if we wrote it (RF-31,
    /// RF-32). Commands already answer `Bloqueada`.
    fn locked(&self);

    /// The save state changed.
    fn save_state(&self, state: SaveState);
}

/// An open vault, as the status tells it (plan, "Precisiones" 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenStatus {
    /// The vault file.
    pub path: PathBuf,
    /// The vault's name.
    pub name: String,
    /// KDBX 3.x: every change answers `SoloLectura` (RF-14).
    pub read_only: bool,
    /// Its protection against brute force is weak (RF-17).
    pub weak_kdf: bool,
    /// Where the last change is.
    pub save: SaveState,
    /// There are changes the disk has not confirmed.
    pub save_pending: bool,
    /// Only in the answer of the unlock that follows a lock with unsaved changes; `None` every other time (RF-34).
    pub recovery: Option<Recovery>,
}

/// The state of the session (plan, "Precisiones" 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// No vault chosen.
    Closed,
    /// A vault is chosen; neither it nor its key is in memory.
    Locked {
        /// The vault file.
        path: PathBuf,
    },
    /// An unlock is running.
    Deriving {
        /// The vault file.
        path: PathBuf,
    },
    /// The vault is decrypted in memory.
    Open(OpenStatus),
}

/// What a change did, so a change that changed nothing is not saved: a save would replace the `.bak` with a copy of
/// the same content and lose the real previous version (RF-22, RF-45).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Applied<T> {
    /// The vault changed: it is saved.
    Changed(T),
    /// Nothing changed: nothing is saved.
    Unchanged(T),
}

impl<T> Applied<T> {
    /// [`Applied::Changed`] if `changed`.
    pub const fn when(changed: bool, value: T) -> Self {
        if changed {
            Self::Changed(value)
        } else {
            Self::Unchanged(value)
        }
    }
}

enum Phase {
    Closed,
    Locked {
        path: PathBuf,
    },
    Deriving {
        path: PathBuf,
        /// The locked vault to go back to if the unlock fails; `None` is closed.
        back: Option<PathBuf>,
    },
    Open {
        live: Arc<Live>,
        shared: Arc<Shared>,
    },
}

/// What the lock hears while it waits for the recovery file.
enum LockMessage {
    /// The secrets were dropped and, if there was something to write, this is how it went.
    Done(Option<CoreResult<PathBuf>>),
    /// [`RECOVERY_LIMIT`] passed.
    TimeUp,
}

struct State {
    phase: Phase,
    /// Goes up with every lock and close: an unlock that started before does not open the vault.
    generation: u64,
    /// A lock is still at steps 2 to 5.
    ending: bool,
    /// For the next unlock that works (RF-34). It survives a second lock and a close.
    note: Option<Recovery>,
    /// The recovery file that was not written within the limit may still appear.
    late: Option<mpsc::Receiver<LockMessage>>,
    /// Closed queues whose thread may still be writing: no other queue may start until they are done.
    retired: Vec<SaveQueue>,
}

impl State {
    /// The note of RF-34, once.
    fn take_note(&mut self) -> Option<Recovery> {
        let note = self.note.take();
        let late = self.late.take();
        if note != Some(Recovery::Lost) {
            return note;
        }
        let arrived = late.and_then(|inbox| {
            inbox.try_iter().find_map(|message| match message {
                LockMessage::Done(Some(Ok(path))) => Some(path),
                LockMessage::Done(_) | LockMessage::TimeUp => None,
            })
        });
        Some(arrived.map_or(Recovery::Lost, |path| Recovery::Saved { path }))
    }
}

/// The session of one vault. Share it between threads behind an `Arc`.
///
/// Dropping it releases the vault and the key but writes no recovery file: call [`Session::close`] first.
pub struct Session {
    disk: Arc<dyn Disk>,
    clock: Arc<dyn Clock>,
    events: Arc<dyn SessionEvents>,
    state: Mutex<State>,
}

impl fmt::Debug for Session {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never the content.
        f.debug_struct("Session").finish_non_exhaustive()
    }
}

impl Session {
    /// A closed session.
    #[must_use]
    pub fn new(disk: Arc<dyn Disk>, clock: Arc<dyn Clock>, events: Arc<dyn SessionEvents>) -> Self {
        Self {
            disk,
            clock,
            events,
            state: Mutex::new(State {
                phase: Phase::Closed,
                generation: 0,
                ending: false,
                note: None,
                late: None,
                retired: Vec::new(),
            }),
        }
    }

    fn state(&self) -> MutexGuard<'_, State> {
        hold(&self.state)
    }

    /// The state, at once: also while a key is derived, a change is encrypted or the disk hangs (RF-18).
    #[must_use]
    pub fn status(&self) -> Status {
        let state = self.state();
        match &state.phase {
            Phase::Closed => Status::Closed,
            Phase::Locked { path } => Status::Locked { path: path.clone() },
            Phase::Deriving { path, .. } => Status::Deriving { path: path.clone() },
            Phase::Open { shared, .. } => Status::Open(describe(shared, None)),
        }
    }

    /// Opens the vault at `path` with a master password, a key file or both, and starts saving its changes.
    ///
    /// The answer carries, once, what happened to the changes a previous lock could not save (RF-34).
    ///
    /// # Errors
    /// The session stays as it was (locked or closed) in every case.
    /// - [`CoreError::Ocupada`]: another unlock is running (RF-88), a lock is still writing its recovery file, or
    ///   a save of the previous session is still being written by a disk that hung.
    /// - [`CoreError::ArgumentoInvalido`]: a vault is already open (close it first).
    /// - [`CoreError::ArchivoNoEncontrado`], [`CoreError::ArchivoLlaveNoEncontrado`] (RF-95), then those of
    ///   [`VaultKey::new`], [`KeyFile::from_bytes`] and [`kdbx::open`].
    /// - [`CoreError::Bloqueada`]: a lock or a close was asked for meanwhile; the vault was not opened.
    pub fn unlock(
        &self,
        path: &Path,
        password: Option<MasterPassword>,
        key_file: Option<&Path>,
    ) -> CoreResult<Status> {
        self.enter(path, || {
            let (bytes, fingerprint) = read_vault(self.disk.as_ref(), path)?;
            let key_file = key_file.map(|file| self.read_key_file(file)).transpose()?;
            Ok((bytes, fingerprint, VaultKey::new(password, key_file)?))
        })
    }

    /// Creates a vault ([`kdbx::create_named_vault_file`], RF-1 to RF-6) and leaves the session open on it.
    ///
    /// # Errors
    /// Those of [`kdbx::create_named_vault_file`] and those of [`Session::unlock`].
    pub fn create(
        &self,
        path: &Path,
        name: &str,
        password: Option<MasterPassword>,
        key_file: NewKeyFile,
        kdf: NewVaultKdf,
    ) -> CoreResult<Status> {
        self.enter(path, || {
            // Creating consumes the key. The session needs its own, and it is checked below against the file: a
            // key that did not open what was just written would encrypt every later save with the wrong key.
            let password_again = password
                .as_ref()
                .map(|password| MasterPassword::new(password.expose().to_owned()));
            let (file_again, generated) = match &key_file {
                NewKeyFile::None => (None, None),
                NewKeyFile::Existing(file) => {
                    (Some(KeyFile::from_bytes(file.content().to_vec())?), None)
                }
                NewKeyFile::Generate(key_path) => (None, Some(key_path.clone())),
            };
            drop(kdbx::create_named_vault_file(
                path, name, password, key_file, kdf,
            )?);
            let key_file = match generated {
                Some(key_path) => Some(self.read_key_file(&key_path)?),
                None => file_again,
            };
            let key = VaultKey::new(password_again, key_file)?;
            let (bytes, fingerprint) = read_vault(self.disk.as_ref(), path)?;
            Ok((bytes, fingerprint, key))
        })
    }

    /// Reads from the open vault. `look` must only touch memory and return: while it runs, a lock cannot drop
    /// the vault nor the key (module documentation, "The limit of step 4").
    ///
    /// # Errors
    /// [`CoreError::Bloqueada`] if the vault is not open (RF-35), and also if a lock was asked for while `look`
    /// ran: its value is dropped (RF-90: whoever put it somewhere, like the clipboard, takes it back). Otherwise,
    /// what `look` returns.
    pub fn read<T>(&self, look: impl FnOnce(&OpenedVault) -> CoreResult<T>) -> CoreResult<T> {
        let (live, shared) = self.open_parts()?;
        let _turn = shared.turn.take()?;
        let seen = look(&hold(&live.vault));
        if shared.turn.ended() {
            return Err(CoreError::Bloqueada);
        }
        seen
    }

    /// The one way to change the vault: applies `change` and, if it changed something, encrypts the whole vault
    /// and hands it to the save queue (RF-21, RF-89). It answers without waiting for the disk; how the save went
    /// is told by [`SessionEvents::save_state`], and a failed save leaves the change in memory (RF-23).
    ///
    /// `change` gets the time of the injected clock, for the dates of what it creates or edits. Like the closure of
    /// [`Session::read`], it must only touch memory and return.
    ///
    /// # Errors
    /// In this order: [`CoreError::Bloqueada`] (RF-35), [`CoreError::SoloLectura`] before touching anything
    /// (RF-14), what `change` returns (nothing is saved then), and [`CoreError::Bloqueada`] again if a lock was
    /// asked for meanwhile: that change is in no file, and the caller was told so.
    pub fn mutate<T>(
        &self,
        change: impl FnOnce(&mut OpenedVault, Timestamp) -> CoreResult<Applied<T>>,
    ) -> CoreResult<T> {
        let (live, shared) = self.open_parts()?;
        let _turn = shared.turn.take()?;
        if hold(&shared.facts).read_only {
            return Err(CoreError::SoloLectura);
        }
        let now = self.now();
        let mut vault = hold(&live.vault);
        let (value, encrypted) = match change(&mut vault, now) {
            Ok(Applied::Changed(value)) => (value, vault.to_bytes(live.secrets.key())),
            Ok(Applied::Unchanged(value)) => {
                drop(vault);
                return if shared.turn.ended() {
                    Err(CoreError::Bloqueada)
                } else {
                    Ok(value)
                };
            }
            Err(error) => {
                drop(vault);
                return Err(if shared.turn.ended() {
                    CoreError::Bloqueada
                } else {
                    error
                });
            }
        };
        let name = vault.name();
        drop(vault);
        if shared.turn.ended() {
            return Err(CoreError::Bloqueada);
        }
        // Renaming the root group renames the vault.
        hold(&shared.facts).name = name;
        match encrypted {
            Ok(bytes) => shared.submit(&live, bytes)?,
            Err(_) => {
                // The change is in memory and could not be encrypted: same as a save that failed (RF-23).
                hold(&shared.saves).dirty = true;
                shared.monitor.set(SaveState::Failed(FailureCause::Io));
                live.tell(Watch::Poke);
            }
        }
        Ok(value)
    }

    /// Tries again the save that failed (RF-23): the same encrypted bytes, without encrypting again. With nothing
    /// pending it does nothing. The result arrives by [`SessionEvents::save_state`].
    ///
    /// # Errors
    /// [`CoreError::Bloqueada`]; [`CoreError::GuardadoFallido`] if a change that was never encrypted still cannot
    /// be.
    pub fn retry_save(&self) -> CoreResult<Status> {
        let (live, shared) = self.open_parts()?;
        let _turn = shared.turn.take()?;
        let pending = pending_bytes(&live, &shared)?;
        if shared.turn.ended() {
            return Err(CoreError::Bloqueada);
        }
        if let Some(bytes) = pending {
            shared.submit(&live, bytes.to_vec())?;
        }
        Ok(Status::Open(describe(&shared, None)))
    }

    /// Saves the vault at `new_path`, where nothing may exist, and goes on there: later changes are saved to the
    /// new file (RF-23, RF-24). If it fails, the session stays on the file it had, with its changes pending.
    ///
    /// # Errors
    /// [`CoreError::Bloqueada`]; [`CoreError::SoloLectura`]; [`CoreError::Ocupada`] while a save is being written;
    /// [`CoreError::ArchivoExiste`]; [`CoreError::GuardadoFallido`].
    pub fn save_as(&self, new_path: &Path) -> CoreResult<Status> {
        let (live, shared) = self.open_parts()?;
        let _turn = shared.turn.take()?;
        let bytes = match pending_bytes(&live, &shared)? {
            Some(bytes) => bytes,
            None => Arc::new(hold(&live.vault).to_bytes(live.secrets.key())?),
        };
        if shared.turn.ended() {
            return Err(CoreError::Bloqueada);
        }
        let confirmed = shared.retire()?;
        let old_path = hold(&shared.facts).path.clone();

        // The disk may hang from here on: no secret is held meanwhile, only the way to find them if they are still
        // there when the file has to be checked (a lock makes that check fail and nothing is left at `new_path`).
        let secrets = Arc::downgrade(&live.secrets);
        let weak = Arc::downgrade(&live);
        drop(live);
        let verify = move |written: &[u8]| {
            let secrets = secrets.upgrade().ok_or(CoreError::Bloqueada)?;
            kdbx::open(written, secrets.key()).map(drop)
        };
        let written = save_vault_as(self.disk.as_ref(), new_path, &bytes, &verify);

        let live = weak.upgrade().ok_or(CoreError::Bloqueada)?;
        match written {
            Ok(fingerprint) => {
                shared.install(self.start_queue(new_path, fingerprint, &live.secrets)?)?;
                hold(&shared.facts).path = new_path.to_path_buf();
                shared.settled(&live);
                Ok(Status::Open(describe(&shared, None)))
            }
            Err(error) => {
                shared.install(self.start_queue(&old_path, confirmed, &live.secrets)?)?;
                Err(error.code())
            }
        }
    }

    /// Reads the vault file again with the key the session already has, and drops what was pending (RF-24). If it
    /// fails (the file is gone, another app changed its key), the session stays as it was, changes included.
    ///
    /// # Errors
    /// [`CoreError::Bloqueada`]; [`CoreError::Ocupada`] while a save is being written; those of
    /// [`read_vault`] and [`kdbx::open`].
    pub fn reload(&self) -> CoreResult<Status> {
        let (live, shared) = self.open_parts()?;
        let _turn = shared.turn.take()?;
        let confirmed = shared.retire()?;
        let path = hold(&shared.facts).path.clone();

        // As in `save_as`: no secret is held while the disk is read.
        let weak = Arc::downgrade(&live);
        drop(live);
        let read = read_vault(self.disk.as_ref(), &path);

        let live = weak.upgrade().ok_or(CoreError::Bloqueada)?;
        let opened = read.and_then(|(bytes, fingerprint)| {
            kdbx::open(&bytes, live.secrets.key()).map(|vault| (vault, fingerprint))
        });
        if shared.turn.ended() {
            return Err(CoreError::Bloqueada);
        }
        match opened {
            Ok((vault, fingerprint)) => {
                let queue = self.start_queue(&path, fingerprint, &live.secrets)?;
                *hold(&shared.facts) = Facts::of(&path, &vault);
                *hold(&live.vault) = vault;
                shared.install(queue)?;
                shared.settled(&live);
                Ok(Status::Open(describe(&shared, None)))
            }
            Err(error) => {
                shared.install(self.start_queue(&path, confirmed, &live.secrets)?)?;
                Err(error)
            }
        }
    }

    /// Locks the vault (RF-28, RF-30 to RF-35): the six steps of the module documentation. It leaves the session
    /// locked on the same file and returns what happened to the changes that were not saved, which the next
    /// unlock tells again (RF-34).
    ///
    /// It returns at once unless there is something to wait for, and then never more than [`RECOVERY_LIMIT`]: the
    /// recovery file, or a command that was encrypting. The state is locked from the first moment either way.
    pub fn lock(&self) -> Option<Recovery> {
        self.end(false)
    }

    /// Closes the vault: the same as [`Session::lock`], and the session forgets the file (RF-36).
    pub fn close(&self) -> Option<Recovery> {
        self.end(true)
    }

    fn end(&self, close: bool) -> Option<Recovery> {
        // Step 1.
        let open = {
            let mut state = self.state();
            state.generation += 1;
            let (stays, open) = match mem::replace(&mut state.phase, Phase::Closed) {
                Phase::Closed => (Phase::Closed, None),
                Phase::Locked { path } | Phase::Deriving { path, .. } => {
                    (Phase::Locked { path }, None)
                }
                Phase::Open { live, shared } => {
                    shared.turn.end();
                    let path = hold(&shared.facts).path.clone();
                    state.ending = true;
                    (
                        Phase::Locked { path: path.clone() },
                        Some((live, shared, path)),
                    )
                }
            };
            if !close {
                state.phase = stays;
            }
            open
        };
        // Step 2. Whatever the listener does, the lock goes on: a panic there must not leave the vault and the key
        // in memory, nor the unsaved changes without their recovery file (rule 7).
        let told = panic::catch_unwind(AssertUnwindSafe(|| self.events.locked()));
        let outcome = open.and_then(|(live, shared, path)| self.release(live, shared, &path));
        if let Err(listener_panic) = told {
            // The lock is complete; the bug of the listener is still its caller's to see.
            panic::resume_unwind(listener_panic);
        }
        outcome
    }

    /// Steps 3 to 6 of a lock.
    fn release(&self, live: Arc<Live>, shared: Arc<Shared>, path: &Path) -> Option<Recovery> {
        // Step 3.
        let (pending, queue, dirty) = shared.dismantle();
        // Step 4. If a command is running it has its own hold on the secrets, for one derivation at most.
        let released = Arc::clone(&shared.released);
        drop(live);
        drop(shared);
        // Step 5.
        let (written, late) = self.recover(pending, path, released);
        // A change that was never encrypted is in no file.
        let outcome = written.or(dirty.then_some(Recovery::Lost));
        // Step 6.
        let mut state = self.state();
        state.retired.extend(queue);
        state.ending = false;
        if outcome.is_some() {
            state.note.clone_from(&outcome);
            state.late = late;
        }
        outcome
    }

    /// Step 5 of a lock: once the secrets are gone (D12), writes the encrypted bytes to a recovery file, on a
    /// thread that has nothing else, and waits for it up to [`RECOVERY_LIMIT`].
    fn recover(
        &self,
        pending: Option<Arc<Vec<u8>>>,
        vault_path: &Path,
        released: Arc<Released>,
    ) -> (Option<Recovery>, Option<mpsc::Receiver<LockMessage>>) {
        if pending.is_none() && released.is_done() {
            return (None, None);
        }
        let lost = pending.as_ref().map(|_| Recovery::Lost);
        let (sender, inbox) = mpsc::channel();
        let for_alarm = sender.clone();
        self.clock.alarm(
            RECOVERY_LIMIT,
            Box::new(move || {
                // Nobody listens once the lock has its answer.
                _ = for_alarm.send(LockMessage::TimeUp);
            }),
        );
        let stamp = storage::Timestamp::from_system_time_utc(self.clock.now());
        let (disk, vault_path) = (Arc::clone(&self.disk), vault_path.to_path_buf());
        let spawned = thread::Builder::new()
            .name("almarioks-recovery".to_owned())
            .spawn(move || {
                released.wait();
                let written = pending.map(|bytes| {
                    stamp.and_then(|at| write_recovery(disk.as_ref(), &vault_path, &bytes, at))
                });
                // The lock may have stopped waiting; the next unlock still looks here (see `State::late`).
                _ = sender.send(LockMessage::Done(written));
            });
        if spawned.is_err() {
            return (lost, None);
        }
        match inbox.recv() {
            Ok(LockMessage::Done(None)) => (None, None),
            Ok(LockMessage::Done(Some(Ok(path)))) => (Some(Recovery::Saved { path }), None),
            Ok(LockMessage::Done(Some(Err(_)))) => (Some(Recovery::Lost), None),
            Ok(LockMessage::TimeUp) | Err(_) => {
                let late = lost.is_some().then_some(inbox);
                (lost, late)
            }
        }
    }

    /// The open vault, or `Bloqueada` (RF-35). The state lock is released before returning.
    fn open_parts(&self) -> CoreResult<(Arc<Live>, Arc<Shared>)> {
        match &self.state().phase {
            Phase::Open { live, shared } => Ok((Arc::clone(live), Arc::clone(shared))),
            Phase::Closed | Phase::Locked { .. } | Phase::Deriving { .. } => {
                Err(CoreError::Bloqueada)
            }
        }
    }

    /// The injected clock as the time of the vault's dates. A clock out of range gives 1970, never a panic.
    fn now(&self) -> Timestamp {
        let seconds = match self.clock.now().duration_since(UNIX_EPOCH) {
            Ok(since) => i64::try_from(since.as_secs()).ok(),
            Err(before) => i64::try_from(before.duration().as_secs())
                .ok()
                .map(|seconds| -seconds),
        };
        seconds
            .and_then(Timestamp::from_unix_seconds)
            .unwrap_or(Timestamp::EPOCH)
    }

    fn read_key_file(&self, path: &Path) -> CoreResult<KeyFile> {
        match self.disk.read(path) {
            Ok(bytes) => KeyFile::from_bytes(bytes),
            Err(error) if error.kind() == ErrorKind::NotFound => {
                Err(CoreError::ArchivoLlaveNoEncontrado)
            }
            // PENDIENTE: the spec has no code for a key file that is there and cannot be read.
            Err(_) => Err(CoreError::ArchivoLlaveDanado),
        }
    }

    fn start_queue(
        &self,
        path: &Path,
        confirmed: Fingerprint,
        secrets: &Arc<Secrets>,
    ) -> CoreResult<SaveQueue> {
        let secrets = Arc::clone(secrets);
        let verifier: Verifier =
            Arc::new(move |written: &[u8]| kdbx::open(written, secrets.key()).map(drop));
        SaveQueue::start(
            Arc::clone(&self.disk),
            path.to_path_buf(),
            confirmed,
            verifier,
        )
    }

    /// The skeleton of an unlock: one at a time, the slow part outside every lock, and the result thrown away if a
    /// lock or a close was asked for meanwhile.
    fn enter(
        &self,
        path: &Path,
        load: impl FnOnce() -> CoreResult<(Vec<u8>, Fingerprint, VaultKey)>,
    ) -> CoreResult<Status> {
        let generation = self.begin(path)?;
        let opened = load()
            .and_then(|(bytes, fingerprint, key)| self.open_live(path, &bytes, fingerprint, key));

        let mut state = self.state();
        let still_wanted =
            state.generation == generation && matches!(state.phase, Phase::Deriving { .. });
        if !still_wanted {
            if let Ok((live, shared)) = opened {
                let (_, queue, _) = shared.dismantle();
                state.retired.extend(queue);
                drop(state);
                drop(live);
            }
            return Err(CoreError::Bloqueada);
        }
        match opened {
            Ok((live, shared)) => {
                let recovery = state.take_note();
                let status = Status::Open(describe(&shared, recovery));
                state.phase = Phase::Open { live, shared };
                Ok(status)
            }
            Err(error) => {
                state.phase = match mem::replace(&mut state.phase, Phase::Closed) {
                    Phase::Deriving {
                        back: Some(path), ..
                    } => Phase::Locked { path },
                    Phase::Deriving { back: None, .. }
                    | Phase::Closed
                    | Phase::Locked { .. }
                    | Phase::Open { .. } => Phase::Closed,
                };
                Err(error)
            }
        }
    }

    /// Marks the unlock as running (RF-88) and returns the generation it belongs to.
    fn begin(&self, path: &Path) -> CoreResult<u64> {
        let (generation, retired) = {
            let mut state = self.state();
            if state.ending {
                // PENDIENTE: the spec has no code for "the lock is still writing the recovery file" (5 s at most).
                return Err(CoreError::Ocupada);
            }
            let back = match &state.phase {
                Phase::Closed => None,
                Phase::Locked { path } => Some(path.clone()),
                Phase::Deriving { .. } => return Err(CoreError::Ocupada),
                // PENDIENTE: the spec has no code for "a vault is already open" (one vault at a time, "Fuera de
                // alcance"); the window never offers it.
                Phase::Open { .. } => return Err(CoreError::ArgumentoInvalido),
            };
            if state.retired.iter().any(SaveQueue::is_writing) {
                // A save of the previous session is still in the hands of the disk and may replace the file when
                // it comes back: a second writer on that file must not start. PENDIENTE: no code of its own.
                return Err(CoreError::Ocupada);
            }
            state.phase = Phase::Deriving {
                path: path.to_path_buf(),
                back,
            };
            (state.generation, mem::take(&mut state.retired))
        };
        for queue in retired {
            // Idle threads: this returns at once. A thread that panicked wrote nothing more.
            _ = queue.shutdown();
        }
        Ok(generation)
    }

    /// Decrypts the vault (the slow part: one key derivation) and starts its save queue and the thread that tells
    /// the save state.
    fn open_live(
        &self,
        path: &Path,
        bytes: &[u8],
        fingerprint: Fingerprint,
        key: VaultKey,
    ) -> CoreResult<(Arc<Live>, Arc<Shared>)> {
        let vault = kdbx::open(bytes, &key)?;
        let released = Arc::new(Released::default());
        let secrets = Arc::new(Secrets::new(key, &released));
        let queue = self.start_queue(path, fingerprint, &secrets)?;
        let shared = Arc::new(Shared::new(Facts::of(path, &vault), queue, released));
        let (watch, inbox) = mpsc::channel();
        let (for_watcher, events) = (Arc::clone(&shared), Arc::clone(&self.events));
        let spawned = thread::Builder::new()
            .name("almarioks-save-state".to_owned())
            .spawn(move || live::watch(&for_watcher, events.as_ref(), &inbox));
        if spawned.is_err() {
            drop(shared.dismantle());
            return Err(CoreError::GuardadoFallido);
        }
        Ok((Arc::new(Live::new(vault, watch, secrets)), shared))
    }
}

impl Drop for Session {
    /// Releases the vault and the key; no recovery file (the owner calls [`Session::close`] before).
    fn drop(&mut self) {
        let phase = mem::replace(&mut self.state().phase, Phase::Closed);
        if let Phase::Open { live, shared } = phase {
            drop(shared.dismantle());
            drop(live);
        }
    }
}

fn describe(shared: &Shared, recovery: Option<Recovery>) -> OpenStatus {
    let facts = hold(&shared.facts);
    let saves = hold(&shared.saves);
    OpenStatus {
        path: facts.path.clone(),
        name: facts.name.clone(),
        read_only: facts.read_only,
        weak_kdf: facts.weak_kdf,
        save: shared.monitor.state(),
        save_pending: saves.dirty || saves.unconfirmed().is_some(),
        recovery,
    }
}

/// The newest state of the vault, encrypted, if the disk has not confirmed it.
fn pending_bytes(live: &Live, shared: &Shared) -> CoreResult<Option<Arc<Vec<u8>>>> {
    let dirty = hold(&shared.saves).dirty;
    if dirty {
        // Not under the lock of the saves: a lock takes it and must not wait for an encryption.
        let bytes = hold(&live.vault).to_bytes(live.secrets.key())?;
        return Ok(Some(Arc::new(bytes)));
    }
    Ok(hold(&shared.saves).unconfirmed())
}
