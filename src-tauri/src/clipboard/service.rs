//! The clipboard rules, with no Win32: what to remember after a copy, when to clear and how copy and lock
//! take turns (RF-31, RF-73 to RF-75, RF-90).

use std::{
    sync::{
        Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant},
};

use almarioks_core::CoreError;
use secrecy::SecretString;

use super::{ClipboardError, Clock, SequenceNumber, SystemClipboard};

/// Shortest time a copied value may stay in the clipboard, in seconds (RF-73).
pub const MIN_CLEAR_SECONDS: u32 = 10;
/// Longest time a copied value may stay in the clipboard, in seconds (RF-73).
pub const MAX_CLEAR_SECONDS: u32 = 120;
/// Default time a copied value stays in the clipboard, in seconds (RF-73, decision D2 of the spec).
pub const DEFAULT_CLEAR_SECONDS: u32 = 20;

/// The answer to a copy: in how many seconds the clipboard will be cleared (RF-74).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyReceipt {
    /// Seconds until the clipboard is cleared, if nobody writes to it before.
    pub clears_in_seconds: u32,
}

/// What a lock or an exit did to the clipboard (RF-31, RF-75).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClearOutcome {
    /// The clipboard still had what we copied and is now empty.
    Cleared,
    /// Somebody wrote to the clipboard after us: it was left alone.
    NotOurs,
    /// We had nothing in the clipboard.
    NothingPending,
}

/// What a [`ClipboardService::tick`] did (RF-73).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TickOutcome {
    /// We have nothing in the clipboard.
    Idle,
    /// The time has not passed yet; call again after `remaining`.
    NotDue {
        /// Time left until the clipboard must be cleared.
        remaining: Duration,
    },
    /// The time passed, the clipboard still had what we copied and is now empty.
    Cleared,
    /// The time passed, but somebody wrote to the clipboard after us: it was left alone.
    NotOurs,
}

/// What the service remembers after a copy. Never the value.
#[derive(Clone, Copy)]
struct Pending {
    sequence: SequenceNumber,
    clear_at: Instant,
}

/// Copies secrets to the clipboard and clears them when their time passes, on lock and on exit.
///
/// Every operation takes the same internal lock, so copy, tick, lock and exit are served one at a time (RF-90).
pub struct ClipboardService {
    clipboard: Box<dyn SystemClipboard>,
    clock: Box<dyn Clock>,
    /// Counts lock and exit requests. Bumped BEFORE waiting for `pending`, so a copy that is already running
    /// sees the request when it finishes.
    lock_requests: AtomicU64,
    pending: Mutex<Option<Pending>>,
}

impl ClipboardService {
    /// Builds the service over a system clipboard and a clock.
    #[must_use]
    pub fn new(clipboard: impl SystemClipboard + 'static, clock: impl Clock + 'static) -> Self {
        Self {
            clipboard: Box::new(clipboard),
            clock: Box::new(clock),
            lock_requests: AtomicU64::new(0),
            pending: Mutex::new(None),
        }
    }

    /// Puts `secret` in the clipboard, out of the history and the cloud (RF-71, RF-72), and starts the count
    /// of `clear_after_seconds` (RF-73). A new copy replaces the previous count. The value is dropped, and
    /// with it wiped, before returning.
    ///
    /// # Errors
    /// - `Bloqueada` if a lock or exit was requested while this copy was waiting or running: nothing of ours
    ///   is left in the clipboard (RF-90).
    /// - `ArgumentoInvalido` if `clear_after_seconds` is not between 10 and 120; the clipboard is not touched.
    /// - [`ClipboardError::Unavailable`] if the system refused the clipboard.
    pub fn copy(
        &self,
        secret: SecretString,
        clear_after_seconds: u32,
    ) -> Result<CopyReceipt, ClipboardError> {
        let lock_requests_seen = self.lock_requests.load(Ordering::SeqCst);
        let mut pending = self.pending();
        if self.lock_requested_since(lock_requests_seen) {
            return Err(CoreError::Bloqueada.into());
        }
        if !(MIN_CLEAR_SECONDS..=MAX_CLEAR_SECONDS).contains(&clear_after_seconds) {
            return Err(CoreError::ArgumentoInvalido.into());
        }
        let sequence = self
            .clipboard
            .set_secret(&secret)
            .map_err(|_| ClipboardError::Unavailable)?;
        drop(secret);
        let clear_at = self.clock.now() + Duration::from_secs(u64::from(clear_after_seconds));
        *pending = Some(Pending { sequence, clear_at });
        if self.lock_requested_since(lock_requests_seen) {
            // RF-90: the lock was requested while the value was on its way. Take it back now; if the system
            // refuses, `pending` stays so the lock that is waiting for its turn tries again.
            if self.clipboard.clear_if_sequence_is(sequence).is_ok() {
                *pending = None;
            }
            return Err(CoreError::Bloqueada.into());
        }
        Ok(CopyReceipt {
            clears_in_seconds: clear_after_seconds,
        })
    }

    /// Clears the clipboard if the time of the last copy has passed and nobody wrote to it since (RF-73).
    ///
    /// # Errors
    /// [`ClipboardError::Unavailable`] if the system refused the clipboard: the clear stays pending and the
    /// next call tries again.
    pub fn tick(&self) -> Result<TickOutcome, ClipboardError> {
        let mut pending = self.pending();
        let Some(Pending { sequence, clear_at }) = *pending else {
            return Ok(TickOutcome::Idle);
        };
        let remaining = clear_at.saturating_duration_since(self.clock.now());
        if !remaining.is_zero() {
            return Ok(TickOutcome::NotDue { remaining });
        }
        let cleared = self.clear(&mut pending, sequence)?;
        Ok(if cleared {
            TickOutcome::Cleared
        } else {
            TickOutcome::NotOurs
        })
    }

    /// The vault is locking or closing: clears the clipboard now if it still has what we copied (RF-31), and
    /// makes any copy that is still running answer `Bloqueada` and leave nothing behind (RF-90).
    ///
    /// The caller must lock the vault whatever this returns (constitution rule 7).
    ///
    /// # Errors
    /// [`ClipboardError::Unavailable`] if the system refused the clipboard: the clear stays pending and
    /// [`Self::tick`] or the next lock tries again.
    pub fn on_lock(&self) -> Result<ClearOutcome, ClipboardError> {
        self.clear_now()
    }

    /// The app is closing: clears the clipboard now if it still has what we copied (RF-75).
    ///
    /// # Errors
    /// [`ClipboardError::Unavailable`] if the system refused the clipboard.
    pub fn on_exit(&self) -> Result<ClearOutcome, ClipboardError> {
        self.clear_now()
    }

    /// Time left until the clipboard is cleared, or `None` if we have nothing in it or somebody wrote to it
    /// after us. For scheduling [`Self::tick`] and for the countdown of the window; it never reads the content.
    #[must_use]
    pub fn clears_in(&self) -> Option<Duration> {
        let pending = self.pending();
        let Pending { sequence, clear_at } = (*pending)?;
        (self.clipboard.sequence_number() == sequence)
            .then(|| clear_at.saturating_duration_since(self.clock.now()))
    }

    /// How many times a lock or an exit has been requested, counted when the request arrives and not when it
    /// is served. Lets the tests of RF-90 know a lock is waiting behind a copy.
    #[must_use]
    pub fn lock_requests(&self) -> u64 {
        self.lock_requests.load(Ordering::SeqCst)
    }

    fn clear_now(&self) -> Result<ClearOutcome, ClipboardError> {
        // The request is recorded before waiting for a copy in flight (RF-90).
        self.lock_requests.fetch_add(1, Ordering::SeqCst);
        let mut pending = self.pending();
        let Some(Pending { sequence, .. }) = *pending else {
            return Ok(ClearOutcome::NothingPending);
        };
        let cleared = self.clear(&mut pending, sequence)?;
        Ok(if cleared {
            ClearOutcome::Cleared
        } else {
            ClearOutcome::NotOurs
        })
    }

    /// Clears by sequence number and forgets the pending clear, unless the system refused the clipboard.
    fn clear(
        &self,
        pending: &mut Option<Pending>,
        sequence: SequenceNumber,
    ) -> Result<bool, ClipboardError> {
        let cleared = self
            .clipboard
            .clear_if_sequence_is(sequence)
            .map_err(|_| ClipboardError::Unavailable)?;
        *pending = None;
        Ok(cleared)
    }

    fn lock_requested_since(&self, seen: u64) -> bool {
        self.lock_requests.load(Ordering::SeqCst) != seen
    }

    /// The state holds no invariant a panic could break (an `Option` of two `Copy` values), so a poisoned
    /// lock is taken as is: lock and exit must still be able to clear the clipboard.
    fn pending(&self) -> MutexGuard<'_, Option<Pending>> {
        self.pending.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
