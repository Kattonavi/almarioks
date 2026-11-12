//! Clipboard service (spec 001, section I: RF-71 to RF-75, plus RF-31 and RF-90).
//!
//! Two layers:
//! - [`ClipboardService`]: the rules, with no Win32. It talks to the system through [`SystemClipboard`] and reads
//!   the time through [`Clock`], so the worked example 1 of the spec runs against a fake clipboard and a fake clock.
//! - `ffi` (Windows only): the one module of the project with `unsafe`. It implements [`SystemClipboard`] with a
//!   hidden message-only window, the three history/cloud exclusion formats and the clear-by-sequence-number.
//!
//! The service never keeps the copied value: after a copy it remembers only the clipboard sequence number and
//! the instant the clipboard must be cleared.
//!
//! # Wiring (tasks T13 and T14, not done here)
//! - Read the secret from the vault and call [`ClipboardService::copy`] while holding the app state lock
//!   (plan, "Concurrencia y bloqueo"). The service serializes copy and lock on its own (RF-90), but it cannot see
//!   a secret that was read before the lock and handed over after it.
//! - After a copy, schedule a call to [`ClipboardService::tick`] once `clears_in_seconds` have passed. A stale
//!   tick is harmless: it answers [`TickOutcome::NotDue`] with the time left.
//! - Call [`ClipboardService::on_lock`] when the vault locks or closes and [`ClipboardService::on_exit`] when the
//!   app quits.

use std::time::Instant;

use almarioks_core::CoreError;
use secrecy::SecretString;

mod service;

pub use service::{
    ClearOutcome, ClipboardService, CopyReceipt, DEFAULT_CLEAR_SECONDS, MAX_CLEAR_SECONDS,
    MIN_CLEAR_SECONDS, TickOutcome,
};

/// The system's clipboard sequence number: it changes every time anyone writes to the clipboard, so it tells
/// whether the content is still ours without reading it (RF-73).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SequenceNumber(pub u32);

/// The system refused a clipboard operation. Carries only the operating system's error code: never content.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemClipboardError {
    os_code: i32,
}

impl SystemClipboardError {
    /// Wraps the operating system's error code (an `HRESULT` on Windows).
    #[must_use]
    pub const fn new(os_code: i32) -> Self {
        Self { os_code }
    }

    /// The operating system's error code, for logs and for the tests that decide whether to skip.
    #[must_use]
    pub const fn os_code(self) -> i32 {
        self.os_code
    }
}

/// What the service needs from the operating system's clipboard.
///
/// There is no unconditional "clear" on purpose: the only way to empty the clipboard is to prove, by sequence
/// number, that nobody wrote to it after us.
pub trait SystemClipboard: Send + Sync {
    /// Replaces the clipboard content with `secret`, marked so the system keeps it out of the clipboard history
    /// and of the cloud (RF-72), and returns the sequence number the clipboard has right after.
    ///
    /// # Errors
    /// The system refused the clipboard. Nothing of ours is left in it.
    fn set_secret(&self, secret: &SecretString) -> Result<SequenceNumber, SystemClipboardError>;

    /// The clipboard's current sequence number.
    fn sequence_number(&self) -> SequenceNumber;

    /// Empties the clipboard only if its sequence number is still `expected`. The comparison and the emptying
    /// are one atomic step: nobody can write in between. Returns whether it emptied it.
    ///
    /// # Errors
    /// The system refused the clipboard; nothing changed.
    fn clear_if_sequence_is(&self, expected: SequenceNumber) -> Result<bool, SystemClipboardError>;
}

/// The time source of the service. Injected so the tests never sleep.
pub trait Clock: Send + Sync {
    /// The current instant of a monotonic clock.
    fn now(&self) -> Instant;
}

/// The operating system's monotonic clock.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Instant {
        Instant::now()
    }
}

/// Why a clipboard operation did not happen. Never carries the value being copied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum ClipboardError {
    /// A rule of the spec rejected the request: `Bloqueada` (RF-90) or `ArgumentoInvalido` (RF-73).
    #[error(transparent)]
    Core(#[from] CoreError),
    /// The system refused the clipboard (another app is holding it open, or the session has none).
    #[error("No se pudo usar el portapapeles. Espera un momento y vuelve a intentarlo.")]
    Unavailable,
}

impl ClipboardError {
    /// The stable code, in the same form as [`CoreError::code`]. Never shown to the person.
    #[must_use]
    pub const fn code(self) -> &'static str {
        match self {
            Self::Core(error) => error.code(),
            Self::Unavailable => "PortapapelesNoDisponible",
        }
    }
}
