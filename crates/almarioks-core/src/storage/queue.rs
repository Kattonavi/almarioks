//! The save queue (RF-89): saves run one at a time on their own thread, and each one writes the whole current
//! state.
//!
//! Locking never waits for the disk (RF-30): the state of the queue is under a `Mutex` that is only held to change
//! memory, never while writing, reading or verifying. So [`SaveQueue::unconfirmed`] and [`SaveQueue::close`] answer
//! at once with the last encrypted bytes not yet confirmed on disk (the copy for the recovery file, RF-33) even when
//! the writing thread is hung.
//!
//! Decision: one pending slot. A save that arrives while another is being written waits for its turn; if a newer
//! one arrives before its turn, the newer one (which already contains it, because every save is the whole state)
//! takes its place and both tickets get the result of that write. The queue is bounded and the file ends with the
//! last state, which is what RF-89 asks.

use std::{
    panic::{self, AssertUnwindSafe},
    path::PathBuf,
    sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, mpsc},
    thread::{self, JoinHandle},
    time::Duration,
};

use super::{
    disk::Disk,
    save::{FailureCause, Fingerprint, SaveError, Saved, save_vault},
};
use crate::error::{CoreError, CoreResult};

/// Checks that bytes read back decrypt with the key (RF-21). It captures the key, so the queue drops it on
/// [`SaveQueue::close`] and only holds a clone while verifying: a hung disk never keeps the key alive (rule 7, D12).
pub type Verifier = Arc<dyn Fn(&[u8]) -> CoreResult<()> + Send + Sync>;

type Outcome = Result<Saved, SaveError>;

/// The answer to one [`SaveQueue::submit`].
#[derive(Debug)]
pub struct SaveTicket {
    answer: mpsc::Receiver<Outcome>,
}

impl SaveTicket {
    /// Waits for the result of the write that included this save.
    ///
    /// # Errors
    /// The [`SaveError`] of that write; [`SaveError::Closed`] if the queue ended before writing it.
    pub fn wait(self) -> Outcome {
        self.answer.recv().unwrap_or(Err(SaveError::Closed))
    }

    /// Like [`SaveTicket::wait`], but gives up after `timeout` and returns `None`.
    #[must_use]
    pub fn wait_timeout(&self, timeout: Duration) -> Option<Outcome> {
        match self.answer.recv_timeout(timeout) {
            Ok(outcome) => Some(outcome),
            Err(mpsc::RecvTimeoutError::Timeout) => None,
            Err(mpsc::RecvTimeoutError::Disconnected) => Some(Err(SaveError::Closed)),
        }
    }
}

struct Job {
    generation: u64,
    bytes: Arc<Vec<u8>>,
    waiters: Vec<mpsc::SyncSender<Outcome>>,
}

struct State {
    /// Fingerprint of what this queue last read or wrote (RF-24).
    confirmed: Fingerprint,
    /// The newest submitted bytes and their generation, until a write of that same generation succeeds.
    unconfirmed: Option<(u64, Arc<Vec<u8>>)>,
    generation: u64,
    pending: Option<Job>,
    verifier: Option<Verifier>,
    /// After an external change the queue stops writing until the session reloads or saves elsewhere (RF-24).
    stopped: Option<SaveError>,
    closed: bool,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Clones the verifier under the lock and calls it without the lock; after a close there is none.
    fn verify(&self, bytes: &[u8]) -> CoreResult<()> {
        let verifier = self.lock().verifier.clone();
        verifier.map_or(Err(CoreError::Bloqueada), |verify| verify(bytes))
    }
}

fn answer(waiters: Vec<mpsc::SyncSender<Outcome>>, outcome: Outcome) {
    for waiter in waiters {
        // Each channel holds one answer and gets one; a ticket that was dropped does not matter.
        _ = waiter.try_send(outcome);
    }
}

/// Saves of one vault file, one at a time on their own thread (RF-89).
pub struct SaveQueue {
    shared: Arc<Shared>,
    worker: Option<JoinHandle<()>>,
}

impl std::fmt::Debug for SaveQueue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SaveQueue").finish_non_exhaustive()
    }
}

impl SaveQueue {
    /// Starts the queue for the vault at `path`, whose content on disk has fingerprint `confirmed` (from
    /// [`super::read_vault`] or the last save).
    ///
    /// # Errors
    /// [`CoreError::GuardadoFallido`] if the system cannot start the thread.
    pub fn start(
        disk: Arc<dyn Disk>,
        path: PathBuf,
        confirmed: Fingerprint,
        verifier: Verifier,
    ) -> CoreResult<Self> {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                confirmed,
                unconfirmed: None,
                generation: 0,
                pending: None,
                verifier: Some(verifier),
                stopped: None,
                closed: false,
            }),
            wake: Condvar::new(),
        });
        let for_worker = Arc::clone(&shared);
        let worker = thread::Builder::new()
            .name("almarioks-save".to_owned())
            .spawn(move || run(&for_worker, disk.as_ref(), &path))
            .map_err(|_| CoreError::GuardadoFallido)?;
        Ok(Self {
            shared,
            worker: Some(worker),
        })
    }

    /// Queues the whole vault, already encrypted. Returns at once: it never waits for the disk.
    pub fn submit(&self, encrypted: Vec<u8>) -> SaveTicket {
        let (sender, answer) = mpsc::sync_channel(1);
        let ticket = SaveTicket { answer };
        let mut state = self.shared.lock();
        if state.closed {
            _ = sender.try_send(Err(SaveError::Closed)); // the channel is empty: it fits
            return ticket;
        }
        state.generation += 1;
        let generation = state.generation;
        let bytes = Arc::new(encrypted);
        state.unconfirmed = Some((generation, Arc::clone(&bytes)));
        if let Some(error) = state.stopped {
            _ = sender.try_send(Err(error)); // the channel is empty: it fits
            return ticket;
        }
        match state.pending.as_mut() {
            Some(job) => {
                job.generation = generation;
                job.bytes = bytes;
                job.waiters.push(sender);
            }
            None => {
                state.pending = Some(Job {
                    generation,
                    bytes,
                    waiters: vec![sender],
                });
            }
        }
        drop(state);
        self.shared.wake.notify_one();
        ticket
    }

    /// The newest encrypted bytes not yet confirmed on disk, if any. Never waits for the disk (RF-30).
    #[must_use]
    pub fn unconfirmed(&self) -> Option<Arc<Vec<u8>>> {
        self.shared
            .lock()
            .unconfirmed
            .as_ref()
            .map(|(_, bytes)| Arc::clone(bytes))
    }

    /// Fingerprint of what this queue last read or wrote.
    #[must_use]
    pub fn confirmed(&self) -> Fingerprint {
        self.shared.lock().confirmed
    }

    /// For a lock (RF-30, RF-33, D12): stops the queue, drops the verifier (and with it the key), answers
    /// [`SaveError::Closed`] to the save waiting its turn and returns the newest bytes not yet confirmed, for the
    /// recovery file. Never waits for the disk. A write already running ends on its own: if it had not verified yet it
    /// fails; if it had, it may still replace the vault with those verified, encrypted bytes after the lock.
    pub fn close(&self) -> Option<Arc<Vec<u8>>> {
        let mut state = self.shared.lock();
        state.closed = true;
        state.verifier = None;
        let pending = state.pending.take();
        let unconfirmed = state
            .unconfirmed
            .as_ref()
            .map(|(_, bytes)| Arc::clone(bytes));
        drop(state);
        self.shared.wake.notify_all();
        if let Some(job) = pending {
            answer(job.waiters, Err(SaveError::Closed));
        }
        unconfirmed
    }

    /// Closes the queue and waits for its thread. Only when the disk is known not to hang (the app is exiting after a
    /// lock); a lock uses [`SaveQueue::close`].
    ///
    /// # Errors
    /// [`CoreError::GuardadoFallido`] if the thread panicked.
    pub fn shutdown(mut self) -> CoreResult<()> {
        self.close();
        match self.worker.take() {
            Some(worker) => worker.join().map_err(|_| CoreError::GuardadoFallido),
            None => Ok(()),
        }
    }
}

impl Drop for SaveQueue {
    /// Closes without joining: a hung disk must not hang whoever drops the queue (RF-30).
    fn drop(&mut self) {
        self.close();
    }
}

/// The writing thread: takes the pending save, writes it without the lock, records the result.
fn run(shared: &Shared, disk: &dyn Disk, path: &std::path::Path) {
    loop {
        let (job, expected) = {
            let mut state = shared.lock();
            loop {
                if state.closed {
                    return;
                }
                if let Some(job) = state.pending.take() {
                    break (job, state.confirmed);
                }
                state = shared
                    .wake
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        let verify = |bytes: &[u8]| shared.verify(bytes);
        let outcome = panic::catch_unwind(AssertUnwindSafe(|| {
            save_vault(disk, path, &job.bytes, expected, &verify)
        }))
        .unwrap_or(Err(SaveError::Failed(FailureCause::Io)));
        {
            let mut state = shared.lock();
            match outcome {
                Ok(saved) => {
                    state.confirmed = saved.fingerprint;
                    if state
                        .unconfirmed
                        .as_ref()
                        .is_some_and(|(generation, _)| *generation == job.generation)
                    {
                        state.unconfirmed = None;
                    }
                }
                Err(SaveError::ExternalChange) => {
                    state.stopped = Some(SaveError::ExternalChange);
                    if let Some(waiting) = state.pending.take() {
                        answer(waiting.waiters, Err(SaveError::ExternalChange));
                    }
                }
                Err(_) => {}
            }
        }
        answer(job.waiters, outcome);
    }
}
