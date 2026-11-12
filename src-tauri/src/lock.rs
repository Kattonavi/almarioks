//! The lock, as the app runs it (T14; spec 001, section D): who asks for it (the person, the inactivity timer,
//! the exit), what the window is told and what is cleaned around the session of the core.
//!
//! The session (`almarioks_core::session`) already guarantees that a lock never fails open. This module adds what
//! the core cannot see:
//! - **The clipboard** is cleared when the session says it is locking (RF-31), and its timed clear is ticked here
//!   (RF-73).
//! - **The window** gets `vault-locked` and is reloaded (RF-32, AMENAZAS W2), and gets `vault-save-state`.
//! - **Inactivity** (RF-29) is measured here with an injected clock, so the tests move the time by hand.
//! - **Nobody waits for the disk on the window's thread**: the lock of the core may take up to 5 s (the recovery
//!   file), so it runs on its own thread and whoever asked is answered as soon as the state is locked (RF-30).

use std::{
    sync::{
        Arc, Condvar, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant, SystemTime},
};

use almarioks_core::{
    session::{RECOVERY_LIMIT, SaveState, Session, SessionEvents, Status, SystemClock},
    storage::StdDisk,
};

use crate::{clipboard::ClipboardService, state::SettingsStore};

/// Event the window hears when the vault locks or closes: unmount everything (RF-32). No payload.
pub const VAULT_LOCKED_EVENT: &str = "vault-locked";
/// Event with where the last change is: `saving`, `saved`, `failed` or `external-change` (plan, "Precisiones" 13).
pub const VAULT_SAVE_STATE_EVENT: &str = "vault-save-state";

/// How often the app looks at the inactivity timer and at the clipboard's pending clear.
pub const HEARTBEAT: Duration = Duration::from_secs(1);

/// The longest whoever asked for a lock waits to hear that the state is locked. The core gets there in
/// microseconds; this only bounds the wait if its thread could never run.
const LOCK_ANSWER_LIMIT: Duration = Duration::from_secs(1);

/// The longest the app waits, when it exits, for the vault to close: the recovery file's own limit (RF-33) plus
/// room for a change that was being encrypted.
const EXIT_LIMIT: Duration = Duration::from_secs(RECOVERY_LIMIT.as_secs() + 3);

/// A wall clock that went back by more than this since the last activity is not trusted (see [`idle_between`]).
const BACKWARDS_TOLERANCE: Duration = Duration::from_secs(2);

/// What the window is told. The real one emits Tauri events; nothing here carries content of the vault.
pub trait WindowEvents: Send + Sync {
    /// The vault locked or closed (RF-32).
    fn vault_locked(&self);
    /// The save state changed: `saving`, `saved`, `failed` or `external-change`.
    fn save_state(&self, state: &'static str);
}

/// Reloads the webview after a lock, so no script keeps a value the page had (RF-32, AMENAZAS W2). Injected: the
/// tests' runtime has no page to reload, so they count the calls instead.
pub trait WebviewReloader: Send + Sync {
    /// Reloads the page of the main window. Must not block.
    fn reload(&self);
}

/// One reading of the two clocks the inactivity timer compares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Moment {
    /// A clock that never goes back. Whether it counts the time the machine was suspended depends on the system.
    pub monotonic: Instant,
    /// The time of day: it counts the time the machine was suspended, and the person can change it.
    pub wall: SystemTime,
}

/// The time source of the inactivity timer. Injected so the tests never sleep (spec, "Cómo se prueba", row 29).
pub trait IdleClock: Send + Sync {
    /// Both clocks, read together.
    fn now(&self) -> Moment;
}

/// The clocks of the machine.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemIdleClock;

impl IdleClock for SystemIdleClock {
    fn now(&self) -> Moment {
        Moment {
            monotonic: Instant::now(),
            wall: SystemTime::now(),
        }
    }
}

/// How long ago `last` was, in real time (RF-29: "tiempo real, incluido el que el equipo estuvo suspendido").
///
/// The longest of what the two clocks say: the monotonic one cannot be changed by the person, and the wall clock
/// counts a suspension whatever the system does with the other. `None` means the time cannot be trusted: the wall
/// clock went back by more than a small tolerance, so a suspension can no longer be told apart from a changed
/// clock, and the caller locks.
///
/// Decisión pendiente (the spec does not say how to measure): moving the clock forward locks early and moving it
/// back locks at once. Both err on the locked side.
#[must_use]
pub fn idle_between(last: Moment, now: Moment) -> Option<Duration> {
    let monotonic = now.monotonic.saturating_duration_since(last.monotonic);
    match now.wall.duration_since(last.wall) {
        Ok(wall) => Some(monotonic.max(wall)),
        Err(back) if back.duration() <= BACKWARDS_TOLERANCE => Some(monotonic),
        Err(_) => None,
    }
}

/// The inactivity timer (RF-29): remembers the last activity and says whether N minutes have passed since.
/// Pure logic: the time comes in as a parameter.
#[derive(Debug)]
pub struct IdleTimer {
    last: Mutex<Moment>,
}

impl IdleTimer {
    /// A timer whose last activity is `now`.
    #[must_use]
    pub const fn new(now: Moment) -> Self {
        Self {
            last: Mutex::new(now),
        }
    }

    /// The person used the window (or the vault was just opened): the count starts again.
    pub fn activity(&self, now: Moment) {
        *hold(&self.last) = now;
    }

    /// Time left before the vault must lock; zero once `minutes` have passed or the time cannot be trusted.
    #[must_use]
    pub fn remaining(&self, now: Moment, minutes: u32) -> Duration {
        let limit = Duration::from_secs(u64::from(minutes) * 60);
        idle_between(*hold(&self.last), now)
            .map_or(Duration::ZERO, |idle| limit.saturating_sub(idle))
    }

    /// `minutes` of real time have passed since the last activity (RF-29).
    #[must_use]
    pub fn expired(&self, now: Moment, minutes: u32) -> bool {
        self.remaining(now, minutes).is_zero()
    }
}

/// None of the locks of this module guards an invariant a panic could break (a `Copy` value, a counter, a list of
/// handles), so a poisoned one is taken as it is: the lock must go on.
fn hold<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Counts the times the session said "locked", so whoever asked for a lock can wait for exactly that.
#[derive(Default)]
struct LockSignal {
    seen: Mutex<u64>,
    changed: Condvar,
}

impl LockSignal {
    fn count(&self) -> u64 {
        *hold(&self.seen)
    }

    fn bump(&self) {
        *hold(&self.seen) += 1;
        self.changed.notify_all();
    }

    /// Waits until the count is past `seen`, or `limit` passes.
    fn wait_past(&self, seen: u64, limit: Duration) {
        let guard = hold(&self.seen);
        drop(
            self.changed
                .wait_timeout_while(guard, limit, |count| *count == seen)
                .unwrap_or_else(PoisonError::into_inner),
        );
    }
}

/// What the app does when the session speaks (`SessionEvents`).
struct AppEvents {
    clipboard: Option<Arc<ClipboardService>>,
    window: Arc<dyn WindowEvents>,
    reloader: Arc<dyn WebviewReloader>,
    locked: LockSignal,
    /// The app is closing: there is no window left to tell or to reload.
    exiting: AtomicBool,
    /// A lock could not clear the clipboard (the system refused it): the heartbeat tries again.
    clear_owed: AtomicBool,
    /// An unlock was asked for since the last reload: the page may hold the key that was typed, or what it read
    /// of the vault. A page that was reloaded and has unlocked nothing since holds nothing, and is not reloaded
    /// again: so a page that asked for a lock while it started could never reload itself in a loop.
    page_holds_something: AtomicBool,
}

impl AppEvents {
    /// RF-31: empties the clipboard if its content is ours. If the system refuses, the heartbeat tries again.
    fn clear_clipboard(&self) {
        if let Some(clipboard) = &self.clipboard {
            self.clear_owed
                .store(clipboard.on_lock().is_err(), Ordering::SeqCst);
        }
    }

    /// The heartbeat's part: a clear that a lock still owes, and the timed clear of the last copy (RF-73).
    fn tend_clipboard(&self) {
        let Some(clipboard) = &self.clipboard else {
            return;
        };
        if self.clear_owed.load(Ordering::SeqCst) {
            self.clear_owed
                .store(clipboard.on_lock().is_err(), Ordering::SeqCst);
        }
        // A tick the system refused stays pending inside the service: the next heartbeat tries again.
        _ = clipboard.tick();
    }
}

impl SessionEvents for AppEvents {
    fn locked(&self) {
        // Commands already answer `Bloqueada` (step 1 of the core's lock). Clipboard first: it is the one part
        // that holds a secret (RF-31); it also makes a copy in flight answer `Bloqueada` (RF-90).
        self.clear_clipboard();
        if !self.exiting.load(Ordering::SeqCst) {
            self.window.vault_locked();
            if self.page_holds_something.swap(false, Ordering::SeqCst) {
                self.reloader.reload();
            }
        }
        self.locked.bump();
    }

    fn save_state(&self, state: SaveState) {
        if !self.exiting.load(Ordering::SeqCst) {
            self.window.save_state(state.as_str());
        }
    }
}

/// Lock or close (RF-28, RF-36).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum End {
    /// The vault stays chosen: the window shows the unlock screen.
    Lock,
    /// The session forgets the file: the window goes back to the welcome screen.
    Close,
}

/// What the app is assembled from around the session.
pub struct LockParts {
    /// The clipboard, if it could start.
    pub clipboard: Option<Arc<ClipboardService>>,
    /// The settings: the minutes of inactivity are read at every evaluation.
    pub settings: Arc<SettingsStore>,
    /// Who tells the window.
    pub window: Arc<dyn WindowEvents>,
    /// Who reloads the webview after a lock.
    pub reloader: Arc<dyn WebviewReloader>,
    /// The clocks of the inactivity timer.
    pub clock: Arc<dyn IdleClock>,
}

/// The session of the core plus everything that must happen around its lock. One per app, behind an `Arc`.
pub struct LockCenter {
    session: Arc<Session>,
    events: Arc<AppEvents>,
    settings: Arc<SettingsStore>,
    clock: Arc<dyn IdleClock>,
    idle: IdleTimer,
    /// Threads still finishing a lock (the recovery file): the exit waits for them.
    workers: Mutex<Vec<JoinHandle<()>>>,
}

impl LockCenter {
    /// A closed session over the real disk and the system's clock, wired to the clipboard and the window.
    #[must_use]
    pub fn new(parts: LockParts) -> Self {
        let events = Arc::new(AppEvents {
            clipboard: parts.clipboard,
            window: parts.window,
            reloader: parts.reloader,
            locked: LockSignal::default(),
            exiting: AtomicBool::new(false),
            clear_owed: AtomicBool::new(false),
            page_holds_something: AtomicBool::new(false),
        });
        let for_session: Arc<dyn SessionEvents> = events.clone();
        let session = Arc::new(Session::new(
            Arc::new(StdDisk),
            Arc::new(SystemClock),
            for_session,
        ));
        let idle = IdleTimer::new(parts.clock.now());
        Self {
            session,
            events,
            settings: parts.settings,
            clock: parts.clock,
            idle,
            workers: Mutex::new(Vec::new()),
        }
    }

    /// The session of the core.
    #[must_use]
    pub fn session(&self) -> Arc<Session> {
        Arc::clone(&self.session)
    }

    /// A vault is about to be opened or created with a key the person typed. From here on the page may hold
    /// something worth forgetting, also if the unlock fails: the next lock or close reloads it (RF-32, AMENAZAS
    /// W2). Typing the key is activity too.
    pub fn unlocking(&self) {
        self.events
            .page_holds_something
            .store(true, Ordering::SeqCst);
        self.activity();
    }

    /// The person used the window, or a vault is being opened: the inactivity count starts again (RF-29).
    pub fn activity(&self) {
        self.idle.activity(self.clock.now());
    }

    /// Empties the clipboard if its content is ours, as a lock does (RF-31). For a copy that found the vault
    /// locking when it finished and must leave nothing behind (RF-90).
    pub fn clear_clipboard(&self) {
        self.events.clear_clipboard();
    }

    /// Whole seconds left before the inactivity lock, rounded up; 0 when it is due. Reads the settings (disk on
    /// first use): call it from `spawn_blocking`.
    #[must_use]
    pub fn locks_in_seconds(&self) -> u64 {
        let minutes = self.settings.read().auto_lock_minutes();
        let remaining = self.idle.remaining(self.clock.now(), minutes);
        remaining.as_secs() + u64::from(remaining.subsec_nanos() > 0)
    }

    /// One heartbeat: locks the vault if it is open and the inactivity time has passed (RF-29), and lets the
    /// clipboard clear what is due (RF-73). Returns whether it locked. Production calls it every [`HEARTBEAT`];
    /// the tests call it by hand after moving their clocks.
    pub fn evaluate(&self) -> bool {
        if self.events.exiting.load(Ordering::SeqCst) {
            return false;
        }
        let locked = self.lock_if_idle();
        self.events.tend_clipboard();
        locked
    }

    fn lock_if_idle(&self) -> bool {
        // Only an open vault locks by inactivity: a locked or closed one has nothing to hide, and locking it
        // again would tell and reload the window once per heartbeat.
        if !matches!(self.session.status(), Status::Open(_)) {
            return false;
        }
        // Read at every evaluation, so a change of the setting counts from the next heartbeat.
        let minutes = self.settings.read().auto_lock_minutes();
        if !self.idle.expired(self.clock.now(), minutes) {
            return false;
        }
        self.end(End::Lock);
        true
    }

    /// Locks or closes the vault and returns as soon as the session is locked: every command of the vault
    /// already answers `Bloqueada` (RF-30), the clipboard is clean (RF-31) and the window was told and reloaded
    /// (RF-32). What may still take time (the recovery file, up to 5 s, RF-33) goes on in its own thread.
    ///
    /// Blocking for an instant: call it from `spawn_blocking`, never from the window's thread.
    pub fn end(&self, end: End) {
        let seen = self.events.locked.count();
        let session = Arc::clone(&self.session);
        let spawned = thread::Builder::new()
            .name("almarioks-lock".to_owned())
            .spawn(move || finish(&session, end));
        match spawned {
            Ok(worker) => {
                let mut workers = hold(&self.workers);
                workers.retain(|worker| !worker.is_finished());
                workers.push(worker);
                drop(workers);
                self.events.locked.wait_past(seen, LOCK_ANSWER_LIMIT);
            }
            // No thread to hand it to: lock here. Slower to answer, never open.
            Err(_) => finish(&self.session, end),
        }
    }

    /// Waits for the locks that are still finishing (the recovery file of RF-33). Bounded by the core's own
    /// limit. An unlock asked for right after a lock calls it first: otherwise the core would answer `Ocupada`
    /// for as long as that lock is still writing, and the person would have to type the key again.
    ///
    /// Blocking: call it from `spawn_blocking`.
    pub fn settle(&self) {
        let workers = std::mem::take(&mut *hold(&self.workers));
        for worker in workers {
            // A lock thread that panicked already dropped the vault (the core catches its listener).
            drop(worker.join());
        }
    }

    /// The app is closing (RF-36, RF-75): closes the vault as a lock does, waiting a bounded time for what was
    /// not saved to reach its recovery file, and then empties the clipboard if its content is ours. Only the
    /// first call does anything.
    pub fn shutdown(&self) {
        if self.events.exiting.swap(true, Ordering::SeqCst) {
            return;
        }
        let workers = std::mem::take(&mut *hold(&self.workers));
        let session = Arc::clone(&self.session);
        let (done, finished) = mpsc::channel();
        let spawned = thread::Builder::new()
            .name("almarioks-exit".to_owned())
            .spawn(move || {
                for worker in workers {
                    // A lock thread that panicked already dropped the vault (the core catches its listener).
                    drop(worker.join());
                }
                finish(&session, End::Close);
                // Nobody listens if the limit passed first.
                _ = done.send(());
            });
        match spawned {
            // Past the limit the process ends anyway: the vault is already locked and only encrypted bytes
            // were on their way to the disk.
            Ok(_) => drop(finished.recv_timeout(EXIT_LIMIT)),
            Err(_) => finish(&self.session, End::Close),
        }
        if let Some(clipboard) = &self.events.clipboard {
            // Best effort: the process is ending and there is nobody to tell.
            _ = clipboard.on_exit();
        }
    }
}

/// Runs the lock of the core. What it returns (what happened to the unsaved changes) is not lost: the session
/// keeps it for the next unlock, which is where the person reads it (RF-34).
fn finish(session: &Session, end: End) {
    match end {
        End::Lock => drop(session.lock()),
        End::Close => drop(session.close()),
    }
}

/// Starts the thread that calls [`LockCenter::evaluate`] every [`HEARTBEAT`] for as long as the app lives. The
/// timer does not depend on this thread being punctual: each evaluation compares clocks, so after a suspension
/// the first heartbeat locks (spec, example 10).
///
/// # Errors
/// The system could not start the thread: without it nothing would lock by inactivity, so the app must not start.
pub fn start_heartbeat(center: Arc<LockCenter>) -> std::io::Result<()> {
    thread::Builder::new()
        .name("almarioks-heartbeat".to_owned())
        .spawn(move || {
            while !center.events.exiting.load(Ordering::SeqCst) {
                thread::sleep(HEARTBEAT);
                center.evaluate();
            }
        })
        .map(drop)
}
