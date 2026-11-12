//! Clipboard rules (spec 001, section I) against a FAKE clipboard and a FAKE clock: no Win32, no sleep.
//! Covers the worked example 1 (a to d), RF-31, RF-71, RF-73 to RF-75 and RF-90. The real Windows clipboard
//! (RF-72) is tested in `clipboard_win32.rs`.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
    thread,
    time::{Duration, Instant},
};

use almarioks_lib::clipboard::{
    ClearOutcome, ClipboardError, ClipboardService, Clock, CopyReceipt, DEFAULT_CLEAR_SECONDS,
    SequenceNumber, SystemClipboard, SystemClipboardError, TickOutcome,
};
use secrecy::{ExposeSecret, SecretString};

/// Fake data, never a real password.
const GITHUB: &str = "CANARIO-ALMARIOKS-github";
const OTHER_ENTRY: &str = "CANARIO-ALMARIOKS-otra-entrada";
/// How long a test waits for another thread before failing instead of hanging.
const PATIENCE: Duration = Duration::from_secs(10);

#[derive(Default)]
struct FakeState {
    text: Option<String>,
    sequence: u32,
    sets: u32,
    fail_set: bool,
    failing_clears: u32,
    /// When armed, the next `set_secret` reports it is inside and waits to be released.
    hold: Option<(Sender<()>, Receiver<()>)>,
}

/// A clipboard in memory with the semantics of the real one: every write, ours or not, moves the sequence.
#[derive(Clone, Default)]
struct FakeClipboard(Arc<Mutex<FakeState>>);

/// The test's side of a held `set_secret`.
struct Hold {
    inside: Receiver<()>,
    release: Sender<()>,
}

impl FakeClipboard {
    fn text(&self) -> Option<String> {
        self.0.lock().unwrap().text.clone()
    }

    fn sets(&self) -> u32 {
        self.0.lock().unwrap().sets
    }

    /// Another app writes to the clipboard.
    fn other_app_copies(&self, text: &str) {
        let mut state = self.0.lock().unwrap();
        state.text = Some(text.to_owned());
        state.sequence += 1;
    }

    fn fail_next_set(&self) {
        self.0.lock().unwrap().fail_set = true;
    }

    fn fail_next_clears(&self, times: u32) {
        self.0.lock().unwrap().failing_clears = times;
    }

    fn hold_next_set(&self) -> Hold {
        let (inside_tx, inside) = channel();
        let (release, release_rx) = channel();
        self.0.lock().unwrap().hold = Some((inside_tx, release_rx));
        Hold { inside, release }
    }
}

impl SystemClipboard for FakeClipboard {
    fn set_secret(&self, secret: &SecretString) -> Result<SequenceNumber, SystemClipboardError> {
        let (sequence, hold) = {
            let mut state = self.0.lock().unwrap();
            if std::mem::take(&mut state.fail_set) {
                return Err(SystemClipboardError::new(5));
            }
            state.text = Some(secret.expose_secret().to_owned());
            // The real clipboard moves several steps per copy (one per format): the number is read, not guessed.
            state.sequence += 8;
            state.sets += 1;
            (state.sequence, state.hold.take())
        };
        if let Some((inside, release)) = hold {
            inside.send(()).unwrap();
            release.recv_timeout(PATIENCE).unwrap();
        }
        Ok(SequenceNumber(sequence))
    }

    fn sequence_number(&self) -> SequenceNumber {
        SequenceNumber(self.0.lock().unwrap().sequence)
    }

    fn clear_if_sequence_is(&self, expected: SequenceNumber) -> Result<bool, SystemClipboardError> {
        let mut state = self.0.lock().unwrap();
        if state.failing_clears > 0 {
            state.failing_clears -= 1;
            return Err(SystemClipboardError::new(5));
        }
        if state.sequence != expected.0 {
            return Ok(false);
        }
        state.text = None;
        state.sequence += 1;
        Ok(true)
    }
}

/// A clock that only moves when the test says so.
#[derive(Clone)]
struct FakeClock(Arc<Mutex<Instant>>);

impl FakeClock {
    fn advance(&self, seconds: u64) {
        *self.0.lock().unwrap() += Duration::from_secs(seconds);
    }
}

impl Clock for FakeClock {
    fn now(&self) -> Instant {
        *self.0.lock().unwrap()
    }
}

fn setup() -> (FakeClipboard, FakeClock, ClipboardService) {
    let clipboard = FakeClipboard::default();
    let clock = FakeClock(Arc::new(Mutex::new(Instant::now())));
    let service = ClipboardService::new(clipboard.clone(), clock.clone());
    (clipboard, clock, service)
}

fn secret(text: &str) -> SecretString {
    SecretString::from(text)
}

fn code<T>(result: Result<T, ClipboardError>) -> Option<&'static str> {
    result.err().map(ClipboardError::code)
}

#[test]
fn rf_71_copy_puts_the_value_in_the_system_clipboard() {
    let (clipboard, _clock, service) = setup();

    service.copy(secret(GITHUB), 20).unwrap();

    assert_eq!(clipboard.text().as_deref(), Some(GITHUB));
    assert_eq!(clipboard.sets(), 1);
}

#[test]
fn rf_73_example_1a_nobody_else_copies_it_is_empty_at_20_s_and_not_before() {
    let (clipboard, clock, service) = setup();
    service.copy(secret(GITHUB), 20).unwrap();

    clock.advance(19);
    assert_eq!(
        service.tick(),
        Ok(TickOutcome::NotDue {
            remaining: Duration::from_secs(1)
        })
    );
    assert_eq!(clipboard.text().as_deref(), Some(GITHUB), "not before 20 s");

    clock.advance(1);
    assert_eq!(service.tick(), Ok(TickOutcome::Cleared));
    assert_eq!(clipboard.text(), None, "empty at 20 s");
    assert_eq!(service.tick(), Ok(TickOutcome::Idle));
}

#[test]
fn rf_73_example_1b_another_app_copies_hola_at_8_s_and_it_is_still_there_at_20_s() {
    let (clipboard, clock, service) = setup();
    service.copy(secret(GITHUB), 20).unwrap();

    clock.advance(8);
    clipboard.other_app_copies("hola");
    assert_eq!(service.clears_in(), None, "the clipboard is no longer ours");

    clock.advance(12);
    assert_eq!(service.tick(), Ok(TickOutcome::NotOurs));
    assert_eq!(clipboard.text().as_deref(), Some("hola"));
    assert_eq!(service.tick(), Ok(TickOutcome::Idle));
}

#[test]
fn rf_73_example_1c_copying_another_password_at_5_s_restarts_the_count_and_clears_at_25_s() {
    let (clipboard, clock, service) = setup();
    service.copy(secret(GITHUB), 20).unwrap();

    clock.advance(5);
    service.copy(secret(OTHER_ENTRY), 20).unwrap();

    clock.advance(15); // 20 s after the first copy
    assert_eq!(
        service.tick(),
        Ok(TickOutcome::NotDue {
            remaining: Duration::from_secs(5)
        })
    );
    assert_eq!(
        clipboard.text().as_deref(),
        Some(OTHER_ENTRY),
        "not at 20 s"
    );

    clock.advance(5); // 25 s after the first copy
    assert_eq!(service.tick(), Ok(TickOutcome::Cleared));
    assert_eq!(clipboard.text(), None, "empty at 25 s");
}

#[test]
fn rf_73_example_1d_another_app_copies_a_text_equal_to_the_password_and_it_is_not_cleared() {
    let (clipboard, clock, service) = setup();
    service.copy(secret(GITHUB), 20).unwrap();

    clock.advance(8);
    clipboard.other_app_copies(GITHUB);

    clock.advance(12);
    assert_eq!(service.tick(), Ok(TickOutcome::NotOurs));
    assert_eq!(
        clipboard.text().as_deref(),
        Some(GITHUB),
        "decided by sequence number, not by comparing the content"
    );
}

#[test]
fn rf_73_seconds_outside_10_to_120_are_rejected_without_touching_the_clipboard() {
    let (clipboard, _clock, service) = setup();
    clipboard.other_app_copies("hola");

    for seconds in [0, 9, 121, u32::MAX] {
        assert_eq!(
            code(service.copy(secret(GITHUB), seconds)),
            Some("ArgumentoInvalido"),
            "{seconds} s"
        );
    }

    assert_eq!(clipboard.text().as_deref(), Some("hola"));
    assert_eq!(clipboard.sets(), 0);
    assert_eq!(service.tick(), Ok(TickOutcome::Idle));
}

#[test]
fn rf_73_a_clear_the_system_refused_is_tried_again_on_the_next_tick() {
    let (clipboard, clock, service) = setup();
    service.copy(secret(GITHUB), 20).unwrap();
    clock.advance(20);

    clipboard.fail_next_clears(1);
    assert_eq!(code(service.tick()), Some("PortapapelesNoDisponible"));
    assert_eq!(clipboard.text().as_deref(), Some(GITHUB));

    assert_eq!(service.tick(), Ok(TickOutcome::Cleared));
    assert_eq!(clipboard.text(), None);
}

#[test]
fn rf_74_copy_says_in_how_many_seconds_it_will_be_cleared() {
    let (_clipboard, clock, service) = setup();

    for seconds in [10, DEFAULT_CLEAR_SECONDS, 120] {
        assert_eq!(
            service.copy(secret(GITHUB), seconds),
            Ok(CopyReceipt {
                clears_in_seconds: seconds
            })
        );
        assert_eq!(
            service.clears_in(),
            Some(Duration::from_secs(u64::from(seconds)))
        );
    }

    clock.advance(100);
    assert_eq!(service.clears_in(), Some(Duration::from_secs(20)));
}

#[test]
fn rf_71_a_copy_the_system_refused_reports_it_and_leaves_nothing_pending() {
    let (clipboard, _clock, service) = setup();
    clipboard.other_app_copies("hola");
    clipboard.fail_next_set();

    assert_eq!(
        code(service.copy(secret(GITHUB), 20)),
        Some("PortapapelesNoDisponible")
    );

    assert_eq!(clipboard.text().as_deref(), Some("hola"));
    assert_eq!(service.tick(), Ok(TickOutcome::Idle));
}

#[test]
fn rf_31_lock_clears_the_clipboard_if_almarioks_put_its_content() {
    let (clipboard, clock, service) = setup();
    service.copy(secret(GITHUB), 20).unwrap();
    clock.advance(3);

    assert_eq!(service.on_lock(), Ok(ClearOutcome::Cleared));

    assert_eq!(clipboard.text(), None);
    assert_eq!(service.tick(), Ok(TickOutcome::Idle));
}

#[test]
fn rf_31_lock_leaves_alone_what_another_app_copied() {
    let (clipboard, _clock, service) = setup();
    assert_eq!(service.on_lock(), Ok(ClearOutcome::NothingPending));

    service.copy(secret(GITHUB), 20).unwrap();
    clipboard.other_app_copies("hola");

    assert_eq!(service.on_lock(), Ok(ClearOutcome::NotOurs));
    assert_eq!(clipboard.text().as_deref(), Some("hola"));
}

#[test]
fn rf_31_a_lock_clear_the_system_refused_stays_pending() {
    let (clipboard, _clock, service) = setup();
    service.copy(secret(GITHUB), 20).unwrap();

    clipboard.fail_next_clears(1);
    assert_eq!(code(service.on_lock()), Some("PortapapelesNoDisponible"));

    assert_eq!(service.on_lock(), Ok(ClearOutcome::Cleared));
    assert_eq!(clipboard.text(), None);
}

#[test]
fn rf_75_exit_clears_the_clipboard_if_almarioks_put_its_content() {
    let (clipboard, _clock, service) = setup();
    service.copy(secret(GITHUB), 120).unwrap();

    assert_eq!(service.on_exit(), Ok(ClearOutcome::Cleared));

    assert_eq!(clipboard.text(), None);
}

#[test]
fn rf_75_exit_leaves_alone_what_another_app_copied() {
    let (clipboard, _clock, service) = setup();
    assert_eq!(service.on_exit(), Ok(ClearOutcome::NothingPending));

    service.copy(secret(GITHUB), 20).unwrap();
    clipboard.other_app_copies(GITHUB);

    assert_eq!(service.on_exit(), Ok(ClearOutcome::NotOurs));
    assert_eq!(clipboard.text().as_deref(), Some(GITHUB));
}

/// Starts a copy that stops inside the system clipboard, requests a lock behind it and lets the copy finish.
/// Returns what each one answered.
fn copy_in_flight_then_lock(
    clipboard: &FakeClipboard,
    service: ClipboardService,
) -> (
    Result<CopyReceipt, ClipboardError>,
    Result<ClearOutcome, ClipboardError>,
    Arc<ClipboardService>,
) {
    let service = Arc::new(service);
    let hold = clipboard.hold_next_set();

    let copier = {
        let service = Arc::clone(&service);
        thread::spawn(move || service.copy(secret(GITHUB), 20))
    };
    hold.inside.recv_timeout(PATIENCE).unwrap();
    assert_eq!(
        clipboard.text().as_deref(),
        Some(GITHUB),
        "the copy is in flight: the value is already in the clipboard"
    );

    let locker = {
        let service = Arc::clone(&service);
        thread::spawn(move || service.on_lock())
    };
    let waiting_since = Instant::now();
    while service.lock_requests() == 0 {
        assert!(waiting_since.elapsed() < PATIENCE, "the lock never arrived");
        thread::yield_now();
    }
    hold.release.send(()).unwrap();

    let copied = copier.join().unwrap();
    let locked = locker.join().unwrap();
    (copied, locked, service)
}

#[test]
fn rf_90_a_copy_that_finishes_after_the_lock_was_requested_leaves_nothing_and_answers_bloqueada() {
    let (clipboard, _clock, service) = setup();

    let (copied, locked, service) = copy_in_flight_then_lock(&clipboard, service);

    assert_eq!(code(copied), Some("Bloqueada"));
    assert_eq!(clipboard.text(), None, "nothing is left in the clipboard");
    assert_eq!(locked, Ok(ClearOutcome::NothingPending));
    assert_eq!(service.tick(), Ok(TickOutcome::Idle));
}

#[test]
fn rf_90_if_the_late_copy_cannot_take_its_value_back_the_lock_behind_it_does() {
    let (clipboard, _clock, service) = setup();
    clipboard.fail_next_clears(1);

    let (copied, locked, _service) = copy_in_flight_then_lock(&clipboard, service);

    assert_eq!(code(copied), Some("Bloqueada"));
    assert_eq!(locked, Ok(ClearOutcome::Cleared));
    assert_eq!(clipboard.text(), None, "nothing is left in the clipboard");
}

#[test]
fn rf_90_a_copy_served_before_the_lock_is_cleared_by_it() {
    let (clipboard, _clock, service) = setup();

    assert!(service.copy(secret(GITHUB), 20).is_ok());
    assert_eq!(service.on_lock(), Ok(ClearOutcome::Cleared));

    assert_eq!(clipboard.text(), None);
    assert_eq!(service.lock_requests(), 1);
}
