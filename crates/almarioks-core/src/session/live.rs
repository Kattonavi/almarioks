//! What an unlocked session is made of, in two halves:
//!
//! - [`Live`] owns the secrets: the decrypted vault and its key. Only the command that has the turn holds it, and
//!   never across the disk.
//! - [`Shared`] owns everything else (turn, facts for the status, save queue, save state). It can be read at any
//!   time without touching a secret.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, mpsc},
};

use super::{
    SaveState, SessionEvents,
    gate::{ReleaseOnDrop, Released, Turn, hold},
};
use crate::{
    error::{CoreError, CoreResult},
    kdbx::{OpenedVault, VaultKey},
    storage::{FailureCause, Fingerprint, SaveError, SaveQueue, SaveTicket, Saved},
};

/// The key of the open vault. The save queue's verifier holds a clone of the `Arc` until the queue is closed.
pub(super) struct Secrets {
    key: VaultKey,
    /// Last field on purpose: dropped after the key.
    _released: ReleaseOnDrop,
}

impl Secrets {
    pub(super) fn new(key: VaultKey, released: &Arc<Released>) -> Self {
        Self {
            key,
            _released: ReleaseOnDrop(Arc::clone(released)),
        }
    }

    pub(super) fn key(&self) -> &VaultKey {
        &self.key
    }
}

/// The secrets of an unlocked session. Fields drop in this order: the vault, the channel, the key.
pub(super) struct Live {
    pub(super) vault: Mutex<OpenedVault>,
    watch: mpsc::Sender<Watch>,
    pub(super) secrets: Arc<Secrets>,
}

impl Live {
    pub(super) fn new(
        vault: OpenedVault,
        watch: mpsc::Sender<Watch>,
        secrets: Arc<Secrets>,
    ) -> Self {
        Self {
            vault: Mutex::new(vault),
            watch,
            secrets,
        }
    }

    /// Hands a message to the thread that tells the save state.
    pub(super) fn tell(&self, message: Watch) {
        // That thread only ends with this value; if it died, the state is still right in the status.
        _ = self.watch.send(message);
    }
}

/// What the status says of an open vault.
pub(super) struct Facts {
    pub(super) path: PathBuf,
    pub(super) name: String,
    pub(super) read_only: bool,
    pub(super) weak_kdf: bool,
}

impl Facts {
    pub(super) fn of(path: &Path, vault: &OpenedVault) -> Self {
        Self {
            path: path.to_path_buf(),
            name: vault.name(),
            read_only: vault.is_read_only(),
            weak_kdf: vault.is_kdf_weak(),
        }
    }
}

/// The save queue of the session and what it could not put in one.
pub(super) struct Saves {
    /// `None` after a lock, and while a reload or a "save as" changes queues.
    queue: Option<SaveQueue>,
    /// Encrypted bytes not confirmed on disk that no queue holds (the queue they were in was stopped).
    carried: Option<Arc<Vec<u8>>>,
    /// The vault in memory has a change that could not be encrypted.
    pub(super) dirty: bool,
}

impl Saves {
    /// The newest encrypted bytes not confirmed on disk.
    pub(super) fn unconfirmed(&self) -> Option<Arc<Vec<u8>>> {
        self.queue
            .as_ref()
            .and_then(SaveQueue::unconfirmed)
            .or_else(|| self.carried.clone())
    }
}

/// The save state the window sees (plan, "Precisiones del contrato" 13) and which save it belongs to.
pub(super) struct Monitor {
    inner: Mutex<(SaveState, u64)>,
}

impl Monitor {
    pub(super) fn state(&self) -> SaveState {
        hold(&self.inner).0
    }

    /// A save was handed to the queue: returns its number.
    fn begin(&self) -> u64 {
        let mut inner = hold(&self.inner);
        inner.0 = SaveState::Saving;
        inner.1 += 1;
        inner.1
    }

    /// Sets the state by hand (reload, "save as", a failure before the queue); results of older saves are ignored.
    pub(super) fn set(&self, state: SaveState) {
        let mut inner = hold(&self.inner);
        inner.0 = state;
        inner.1 += 1;
    }

    /// The result of save number `seq`. Only the newest save decides the state.
    fn record(&self, seq: u64, outcome: Result<Saved, SaveError>) {
        let mut inner = hold(&self.inner);
        if inner.1 != seq {
            return;
        }
        inner.0 = match outcome {
            Ok(_) => SaveState::Saved,
            Err(SaveError::ExternalChange) => SaveState::ExternalChange,
            Err(SaveError::Failed(cause)) => SaveState::Failed(cause),
            Err(SaveError::AlreadyExists) => SaveState::Failed(FailureCause::Io),
            // A lock closed the queue: nobody is told any more.
            Err(SaveError::Closed) => return,
        };
    }
}

/// Messages to the thread that tells the save state.
pub(super) enum Watch {
    /// Wait for this save and record its result.
    Ticket(u64, SaveTicket),
    /// The state was set by hand: tell it.
    Poke,
}

/// Everything of an unlocked session that is not a secret.
pub(super) struct Shared {
    pub(super) turn: Turn,
    pub(super) facts: Mutex<Facts>,
    pub(super) saves: Mutex<Saves>,
    pub(super) monitor: Monitor,
    pub(super) released: Arc<Released>,
}

impl Shared {
    pub(super) fn new(facts: Facts, queue: SaveQueue, released: Arc<Released>) -> Self {
        Self {
            turn: Turn::default(),
            facts: Mutex::new(facts),
            saves: Mutex::new(Saves {
                queue: Some(queue),
                carried: None,
                dirty: false,
            }),
            monitor: Monitor {
                inner: Mutex::new((SaveState::Saved, 0)),
            },
            released,
        }
    }

    /// The code when there is no queue: a lock took it, or its thread could not be started.
    fn without_queue(&self) -> CoreError {
        if self.turn.ended() {
            CoreError::Bloqueada
        } else {
            CoreError::GuardadoFallido
        }
    }

    /// Hands the whole vault, already encrypted, to the queue. Never waits for the disk.
    ///
    /// # Errors
    /// [`CoreError::Bloqueada`] if a lock already took the queue: these bytes are not saved anywhere, and the
    /// command that made them answers `Bloqueada` (RF-30).
    pub(super) fn submit(&self, live: &Live, encrypted: Vec<u8>) -> CoreResult<()> {
        let mut saves = hold(&self.saves);
        let Some(queue) = saves.queue.as_ref() else {
            if self.turn.ended() {
                return Err(CoreError::Bloqueada);
            }
            // No thread to write them: they stay here for a retry or for the recovery file of a lock.
            saves.carried = Some(Arc::new(encrypted));
            saves.dirty = false;
            drop(saves);
            self.monitor.set(SaveState::Failed(FailureCause::Io));
            live.tell(Watch::Poke);
            return Ok(());
        };
        let ticket = queue.submit(encrypted);
        saves.carried = None;
        saves.dirty = false;
        drop(saves);
        let seq = self.monitor.begin();
        live.tell(Watch::Ticket(seq, ticket));
        Ok(())
    }

    /// Stops the queue and waits for its thread, so another one can take over the file (reload, "save as"). What
    /// was not confirmed stays in `carried`, where a lock finds it. Returns the fingerprint of the file as that
    /// thread left it.
    ///
    /// # Errors
    /// - [`CoreError::Ocupada`] while a save is being written: its thread may still replace the file, and waiting
    ///   for it would tie this command to the disk.
    /// - [`CoreError::Bloqueada`] after a lock; [`CoreError::GuardadoFallido`] if there is no queue or its thread
    ///   panicked.
    pub(super) fn retire(&self) -> CoreResult<Fingerprint> {
        let queue = {
            let mut saves = hold(&self.saves);
            let Some(queue) = saves.queue.take() else {
                return Err(self.without_queue());
            };
            if queue.is_writing() {
                saves.queue = Some(queue);
                // PENDIENTE: the spec has no code for "a save is still being written"; `Ocupada` is the closest.
                return Err(CoreError::Ocupada);
            }
            if let Some(bytes) = queue.close() {
                saves.carried = Some(bytes);
            }
            queue
        };
        // Its thread is idle: this returns at once.
        queue.shutdown_confirmed()
    }

    /// Puts a queue in place.
    ///
    /// # Errors
    /// [`CoreError::Bloqueada`] if a lock was asked for meanwhile: the queue is closed and dropped here, so its
    /// verifier (and the key) does not outlive the lock.
    pub(super) fn install(&self, queue: SaveQueue) -> CoreResult<()> {
        let mut saves = hold(&self.saves);
        if self.turn.ended() {
            return Err(CoreError::Bloqueada);
        }
        saves.queue = Some(queue);
        Ok(())
    }

    /// The saved file is the vault in memory: nothing is pending.
    pub(super) fn settled(&self, live: &Live) {
        {
            let mut saves = hold(&self.saves);
            saves.carried = None;
            saves.dirty = false;
        }
        self.monitor.set(SaveState::Saved);
        live.tell(Watch::Poke);
    }

    /// For a lock: nobody else gets a turn, the queue is closed (its verifier, and with it the key, is dropped) and
    /// taken out. Returns the newest encrypted bytes not confirmed on disk, the closed queue (its thread may still
    /// be writing) and whether a change in memory was never encrypted. Never waits for the disk nor for a command.
    pub(super) fn dismantle(&self) -> (Option<Arc<Vec<u8>>>, Option<SaveQueue>, bool) {
        self.turn.end();
        let mut saves = hold(&self.saves);
        let queue = saves.queue.take();
        let carried = saves.carried.take();
        let pending = queue.as_ref().and_then(SaveQueue::close).or(carried);
        (pending, queue, saves.dirty)
    }
}

/// The thread that tells the save state: one teller, so the states arrive in order, and no lock held while the
/// listener runs. It never holds the key; it ends when the [`Live`] it belongs to is dropped and its saves answered.
pub(super) fn watch(shared: &Shared, events: &dyn SessionEvents, inbox: &mpsc::Receiver<Watch>) {
    let mut told = SaveState::Saved;
    let mut tell = || {
        let now = shared.monitor.state();
        if now != told && !shared.turn.ended() {
            told = now;
            events.save_state(now);
        }
    };
    for message in inbox {
        tell();
        if let Watch::Ticket(seq, ticket) = message {
            shared.monitor.record(seq, ticket.wait());
            tell();
        }
    }
}
