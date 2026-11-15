//! The two small pieces of synchronisation of the session: whose turn it is ([`Turn`]) and the signal that the
//! vault and its key are gone from memory ([`Released`]). Neither holds a secret.

use std::sync::{
    Arc, Condvar, Mutex, MutexGuard, PoisonError,
    atomic::{AtomicBool, Ordering},
};

use crate::error::{CoreError, CoreResult};

/// Takes a mutex whatever happened to the thread that held it before: the data of the session stays usable (and
/// lockable) after a panic in a command.
pub(super) fn hold<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Debug, Default)]
struct TurnState {
    busy: bool,
    ended: bool,
}

/// One command of the vault at a time (plan, "Concurrencia y bloqueo"), with a way out for a lock: [`Turn::end`]
/// wakes every command that waits its turn and they all answer `Bloqueada` at once (RF-30). A plain mutex would
/// keep them waiting for the command that is encrypting.
#[derive(Debug, Default)]
pub(super) struct Turn {
    state: Mutex<TurnState>,
    freed: Condvar,
    /// Copy of `ended` that is read without any lock.
    ended: AtomicBool,
}

impl Turn {
    /// Waits for the turn.
    ///
    /// # Errors
    /// [`CoreError::Bloqueada`] if a lock was asked for, before or while waiting.
    pub(super) fn take(&self) -> CoreResult<TurnGuard<'_>> {
        let mut state = hold(&self.state);
        loop {
            if state.ended {
                return Err(CoreError::Bloqueada);
            }
            if !state.busy {
                state.busy = true;
                return Ok(TurnGuard { turn: self });
            }
            state = self
                .freed
                .wait(state)
                .unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// A lock was asked for: nobody else gets a turn. Never waits for the command that has it.
    pub(super) fn end(&self) {
        let mut state = hold(&self.state);
        state.ended = true;
        self.ended.store(true, Ordering::SeqCst);
        drop(state);
        self.freed.notify_all();
    }

    /// Whether a lock was asked for.
    pub(super) fn ended(&self) -> bool {
        self.ended.load(Ordering::SeqCst)
    }
}

/// The turn, until dropped.
#[derive(Debug)]
pub(super) struct TurnGuard<'a> {
    turn: &'a Turn,
}

impl Drop for TurnGuard<'_> {
    fn drop(&mut self) {
        hold(&self.turn.state).busy = false;
        self.turn.freed.notify_one();
    }
}

/// Tells that the key (and, before it, the vault) of one unlocked session was dropped.
#[derive(Debug, Default)]
pub(super) struct Released {
    done: Mutex<bool>,
    told: Condvar,
}

impl Released {
    /// Whether it already happened.
    pub(super) fn is_done(&self) -> bool {
        *hold(&self.done)
    }

    /// Blocks until it happens.
    pub(super) fn wait(&self) {
        let mut done = hold(&self.done);
        while !*done {
            done = self.told.wait(done).unwrap_or_else(PoisonError::into_inner);
        }
    }
}

/// Marks a [`Released`] when dropped. It is the LAST field of the struct that owns the key, so the mark comes after
/// the key is gone.
#[derive(Debug)]
pub(super) struct ReleaseOnDrop(pub(super) Arc<Released>);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        *hold(&self.0.done) = true;
        self.0.told.notify_all();
    }
}
