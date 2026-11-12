//! The lock as the app runs it (T14; spec 001, section D): by inactivity with clocks the test moves (RF-29 and the
//! worked example 10), by order of the person (RF-28), on exit and before an update (RF-36, RF-75), and around it: the
//! clipboard (RF-31, RF-73), the event and the reload of the webview (RF-32). No test sleeps: the heartbeat that
//! production runs every second is called by hand.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::{
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
        mpsc::channel,
    },
    thread,
    time::Duration,
};

use almarioks_core::{CoreError, kdbx::MasterPassword, session::Status, settings::Settings};
use almarioks_lib::{
    lock::{
        IdleTimer, LockCenter, LockParts, VAULT_LOCKED_EVENT, VAULT_SAVE_STATE_EVENT, WindowEvents,
        idle_between,
    },
    state::SettingsStore,
};
use common::{
    CountingReloader, TempDir, error_code,
    vault::{CANARY, FakeIdleClock, PATIENCE, Rig, canary_vault_bytes},
};
use serde_json::{Value, json};

const HOUR: u64 = 3_600;

// ---------- RF-29: the rule, with no app ----------

#[test]
fn rf_29_at_4_59_it_does_not_lock_and_at_5_00_it_does() {
    let clock = FakeIdleClock::default();
    let timer = IdleTimer::new(clock.moment());

    clock.advance(4 * 60 + 59);
    assert!(!timer.expired(clock.moment(), 5));
    assert_eq!(timer.remaining(clock.moment(), 5), Duration::from_secs(1));
    clock.advance(1);
    assert!(timer.expired(clock.moment(), 5));
    assert_eq!(timer.remaining(clock.moment(), 5), Duration::ZERO);

    // Each choice of the settings (1, 5, 15, 30) is its own limit.
    for minutes in [1, 5, 15, 30] {
        let timer = IdleTimer::new(clock.moment());
        clock.advance(u64::from(minutes) * 60 - 1);
        assert!(
            !timer.expired(clock.moment(), minutes),
            "{minutes} min, one second early"
        );
        clock.advance(1);
        assert!(timer.expired(clock.moment(), minutes), "{minutes} min");
    }
}

#[test]
fn rf_29_activity_starts_the_count_again() {
    let clock = FakeIdleClock::default();
    let timer = IdleTimer::new(clock.moment());

    clock.advance(4 * 60);
    timer.activity(clock.moment());
    clock.advance(4 * 60);
    assert!(!timer.expired(clock.moment(), 5));
    clock.advance(60);
    assert!(timer.expired(clock.moment(), 5));
}

#[test]
fn rf_29_example_10_the_time_the_machine_was_suspended_counts() {
    // 22:00, N = 5: the lid closes. The monotonic clock is taken not to move while suspended (the worst case).
    let clock = FakeIdleClock::default();
    let timer = IdleTimer::new(clock.moment());
    let closed = clock.moment();

    // 06:00: the lid opens.
    clock.suspend(8 * HOUR);
    let opened = clock.moment();
    assert_eq!(opened.monotonic, closed.monotonic);
    assert_eq!(
        idle_between(closed, opened),
        Some(Duration::from_secs(8 * HOUR))
    );
    assert!(
        timer.expired(opened, 5),
        "locked at the first evaluation after waking up"
    );

    // And when the monotonic clock does count the suspension, the answer is the same.
    let awake = FakeIdleClock::default();
    let timer = IdleTimer::new(awake.moment());
    awake.advance(8 * HOUR);
    assert!(timer.expired(awake.moment(), 5));
}

#[test]
fn rf_29_a_clock_that_was_changed_never_keeps_the_vault_open_longer() {
    // Forward: locks early.
    let clock = FakeIdleClock::default();
    let timer = IdleTimer::new(clock.moment());
    clock.suspend(5 * 60);
    assert!(timer.expired(clock.moment(), 5));

    // Back, beyond a small correction: the time cannot be trusted, so it locks.
    let clock = FakeIdleClock::default();
    let timer = IdleTimer::new(clock.moment());
    let before = clock.moment();
    clock.set_wall_back(HOUR);
    assert_eq!(idle_between(before, clock.moment()), None);
    assert!(timer.expired(clock.moment(), 30));

    // Back by a correction of the system's clock: the monotonic clock decides, and five minutes are still five.
    let clock = FakeIdleClock::default();
    let timer = IdleTimer::new(clock.moment());
    clock.set_wall_back(1);
    assert!(!timer.expired(clock.moment(), 5));
    clock.advance(5 * 60);
    assert!(timer.expired(clock.moment(), 5));
}

// ---------- RF-29: the app ----------

#[test]
fn rf_29_the_open_vault_locks_after_five_minutes_without_activity() {
    let rig = Rig::new("rf29-idle");
    let vault = rig.open_canary_vault();

    rig.idle_clock.advance(4 * 60 + 59);
    assert!(!rig.heartbeat());
    assert_eq!(rig.state(), "open");
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 0);

    rig.idle_clock.advance(1);
    assert!(rig.heartbeat());
    assert_eq!(rig.state(), "locked");
    // RF-35, RF-32: nothing of the vault is answered any more, the window was told and the webview reloaded.
    assert_eq!(
        error_code(&rig.args("entry_get", json!({ "id": vault.bank }))),
        "Bloqueada"
    );
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 1);
    assert_eq!(rig.reloader.count(), 1);

    // A vault that is already locked is not locked, told or reloaded again at every heartbeat.
    rig.idle_clock.advance(HOUR);
    assert!(!rig.heartbeat());
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 1);
    assert_eq!(rig.reloader.count(), 1);
}

#[test]
fn rf_29_example_10_the_vault_is_locked_at_the_first_heartbeat_after_a_suspension() {
    let rig = Rig::new("rf29-suspend");
    rig.open_canary_vault();

    rig.idle_clock.suspend(8 * HOUR);
    assert!(rig.heartbeat());
    assert_eq!(rig.state(), "locked");
}

#[test]
fn rf_29_the_activity_command_starts_the_count_again_and_the_status_counts_down() {
    let rig = Rig::new("rf29-activity");
    rig.open_canary_vault();
    let locks_in = || rig.plain("vault_status").unwrap()["locks_in_seconds"].clone();
    assert_eq!(locks_in(), 300);

    rig.idle_clock.advance(200);
    assert_eq!(locks_in(), 100);
    assert_eq!(rig.plain("activity"), Ok(Value::Null));
    assert_eq!(locks_in(), 300);

    rig.idle_clock.advance(200);
    assert!(
        !rig.heartbeat(),
        "400 s since it was opened, 200 s since the last activity"
    );
    rig.idle_clock.advance(99);
    assert_eq!(locks_in(), 1);
    assert!(!rig.heartbeat());
    rig.idle_clock.advance(1);
    assert_eq!(locks_in(), 0);
    assert!(rig.heartbeat());
    assert_eq!(rig.state(), "locked");
}

#[test]
fn rf_29_the_minutes_are_read_from_the_settings_at_every_evaluation() {
    let rig = Rig::new("rf29-setting");
    rig.open_canary_vault();

    rig.idle_clock.advance(90);
    assert!(!rig.heartbeat(), "5 minutes by default");
    rig.args(
        "settings_set",
        json!({ "auto_lock_minutes": 1, "clipboard_seconds": 20 }),
    )
    .unwrap();
    assert!(
        rig.heartbeat(),
        "90 s are more than the 1 minute just chosen"
    );
    assert_eq!(rig.state(), "locked");
}

#[test]
fn rf_29_unlocking_after_a_long_wait_at_the_unlock_screen_does_not_lock_at_once() {
    let rig = Rig::new("rf29-unlock");
    let vault = rig.open_canary_vault();
    rig.plain("vault_lock").unwrap();

    rig.idle_clock.advance(2 * HOUR);
    assert!(!rig.heartbeat(), "a locked vault has nothing to lock");
    rig.args(
        "vault_open",
        json!({ "path": vault.path, "password": common::vault::master() }),
    )
    .unwrap();
    assert!(!rig.heartbeat());
    assert_eq!(rig.state(), "open");
    assert_eq!(rig.plain("vault_status").unwrap()["locks_in_seconds"], 300);
}

// ---------- RF-28, RF-31, RF-32, RF-36: by order of the person ----------

#[test]
fn rf_28_rf_32_vault_lock_locks_tells_the_window_and_reloads_the_webview() {
    let rig = Rig::new("rf28-lock");
    let vault = rig.open_canary_vault();
    rig.args(
        "entry_copy",
        json!({ "id": vault.bank, "field": "Password" }),
    )
    .unwrap();
    assert_eq!(rig.clipboard.text(), Some(format!("{CANARY}-password")));

    // By the time the command answers, all of it is done.
    assert_eq!(rig.plain("vault_lock"), Ok(Value::Null));
    assert_eq!(rig.state(), "locked");
    assert_eq!(rig.clipboard.text(), None, "RF-31");
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 1);
    assert_eq!(rig.reloader.count(), 1);
    let status = rig.plain("vault_status").unwrap();
    assert_eq!(status, json!({ "state": "locked", "path": vault.path }));

    // RF-36: closing does the same and forgets the file.
    assert_eq!(rig.plain("vault_close"), Ok(Value::Null));
    assert_eq!(
        rig.plain("vault_status").unwrap(),
        json!({ "state": "closed" })
    );
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 2);
    // Nothing was unlocked since the reload: the page holds nothing and is not reloaded again.
    assert_eq!(rig.reloader.count(), 1);

    // A key typed for an unlock that failed is in the page too: the next lock or close reloads it (AMENAZAS W2).
    let wrong = rig.args(
        "vault_open",
        json!({ "path": vault.path, "password": "equivocada" }),
    );
    assert_eq!(error_code(&wrong), "LlaveIncorrecta");
    rig.plain("vault_close").unwrap();
    assert_eq!(rig.reloader.count(), 2);
}

#[test]
fn rf_31_the_lock_leaves_alone_what_another_app_copied() {
    let rig = Rig::new("rf31-other");
    let vault = rig.open_canary_vault();
    rig.args(
        "entry_copy",
        json!({ "id": vault.bank, "field": "Password" }),
    )
    .unwrap();
    rig.clipboard.other_app_copies("hola");

    rig.plain("vault_lock").unwrap();
    assert_eq!(rig.clipboard.text().as_deref(), Some("hola"));
}

#[test]
fn rf_31_the_inactivity_lock_clears_the_clipboard_also_of_a_value_copied_without_a_count() {
    let rig = Rig::new("rf31-idle");
    let vault = rig.open_canary_vault();
    let copied = rig
        .args(
            "entry_copy",
            json!({ "id": vault.bank, "field": "UserName" }),
        )
        .unwrap();
    assert_eq!(copied, json!({ "clears_in_seconds": null }));
    assert_eq!(rig.clipboard.text().as_deref(), Some("ana"));

    rig.idle_clock.advance(5 * 60);
    assert!(rig.heartbeat());
    assert_eq!(rig.clipboard.text(), None);
}

// ---------- RF-73: the heartbeat ticks the clipboard ----------

#[test]
fn rf_73_the_heartbeat_clears_a_protected_copy_when_its_time_passes_and_never_a_plain_one() {
    let rig = Rig::new("rf73-tick");
    let vault = rig.open_canary_vault();
    let copy = |field: &str| {
        rig.args("entry_copy", json!({ "id": vault.bank, "field": field }))
            .unwrap()
    };

    assert_eq!(copy("Password"), json!({ "clears_in_seconds": 20 }));
    rig.clipboard_clock.advance(19);
    assert!(!rig.heartbeat());
    assert_eq!(rig.clipboard.text(), Some(format!("{CANARY}-password")));
    rig.clipboard_clock.advance(1);
    assert!(!rig.heartbeat(), "clearing the clipboard is not a lock");
    assert_eq!(rig.clipboard.text(), None);
    assert_eq!(rig.state(), "open");

    // The seconds come from the settings at the moment of the copy.
    rig.args(
        "settings_set",
        json!({ "auto_lock_minutes": 30, "clipboard_seconds": 45 }),
    )
    .unwrap();
    assert_eq!(copy("PIN"), json!({ "clears_in_seconds": 45 }));
    rig.clipboard_clock.advance(44);
    rig.heartbeat();
    assert_eq!(rig.clipboard.text(), Some(format!("{CANARY}-pin")));
    rig.clipboard_clock.advance(1);
    rig.heartbeat();
    assert_eq!(rig.clipboard.text(), None);

    // A field that is not protected has no count (plan, "Precisiones" 16).
    assert_eq!(copy("UserName"), json!({ "clears_in_seconds": null }));
    rig.clipboard_clock.advance(10 * 60);
    rig.heartbeat();
    assert_eq!(rig.clipboard.text().as_deref(), Some("ana"));
}

// ---------- RF-36, RF-75: the app exits ----------

#[test]
fn rf_36_rf_75_exit_closes_the_vault_and_clears_the_clipboard() {
    let rig = Rig::new("rf75-exit");
    let vault = rig.open_canary_vault();
    rig.args(
        "entry_copy",
        json!({ "id": vault.bank, "field": "Password" }),
    )
    .unwrap();

    almarioks_lib::shutdown(rig.app.handle());

    assert_eq!(rig.state(), "closed");
    assert_eq!(rig.clipboard.text(), None);
    assert_eq!(
        error_code(&rig.args("entry_get", json!({ "id": vault.bank }))),
        "Bloqueada"
    );
    // There is no window left to tell or to reload.
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 0);
    assert_eq!(rig.reloader.count(), 0);
    // Asking twice is harmless.
    almarioks_lib::shutdown(rig.app.handle());
    assert_eq!(rig.state(), "closed");
}

#[test]
fn rf_75_exit_leaves_alone_what_another_app_copied() {
    let rig = Rig::new("rf75-other");
    let vault = rig.open_canary_vault();
    rig.args(
        "entry_copy",
        json!({ "id": vault.bank, "field": "Password" }),
    )
    .unwrap();
    rig.clipboard.other_app_copies("hola");

    almarioks_lib::shutdown(rig.app.handle());
    assert_eq!(rig.clipboard.text().as_deref(), Some("hola"));
}

// ---------- RF-36, RF-75 (review 0.1, C-1): the window is about to install an update ----------

/// The recovery files next to the canary vault of `rig`.
fn recovery_files(rig: &Rig) -> Vec<String> {
    std::fs::read_dir(rig.dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .filter(|name| name.starts_with("canario.kdbx.recuperacion-"))
        .collect()
}

#[test]
fn rf_36_rf_75_prepare_update_closes_the_vault_and_clears_the_clipboard_without_touching_the_window()
 {
    let rig = Rig::new("update-prepare");
    let vault = rig.open_canary_vault();
    rig.args(
        "entry_copy",
        json!({ "id": vault.bank, "field": "Password" }),
    )
    .unwrap();
    assert_eq!(rig.clipboard.text(), Some(format!("{CANARY}-password")));

    // On Windows the installer ends the process and the app never sees that exit: this is the exit.
    assert_eq!(rig.plain("app_prepare_update"), Ok(Value::Null));

    assert_eq!(
        rig.plain("vault_status").unwrap(),
        json!({ "state": "closed" })
    );
    assert_eq!(rig.clipboard.text(), None, "RF-75");
    assert_eq!(
        error_code(&rig.args("entry_get", json!({ "id": vault.bank }))),
        "Bloqueada"
    );
    // The window holds the downloaded update and is about to install it: a reload would lose it.
    assert_eq!(rig.reloader.count(), 0);
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 0);
    // Nothing was pending: no recovery file.
    assert_eq!(recovery_files(&rig), Vec::<String>::new());

    // Asking twice is harmless.
    assert_eq!(rig.plain("app_prepare_update"), Ok(Value::Null));
    assert_eq!(rig.state(), "closed");
    assert_eq!(rig.reloader.count(), 0);
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 0);

    // The page was not reloaded, so it still holds what it had read: the next lock or close tells the window
    // and reloads it (RF-32), as after any unlock.
    assert_eq!(rig.plain("vault_close"), Ok(Value::Null));
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 1);
    assert_eq!(rig.reloader.count(), 1);
}

#[test]
fn rf_36_rf_75_prepare_update_leaves_alone_what_another_app_copied() {
    let rig = Rig::new("update-other");
    let vault = rig.open_canary_vault();
    rig.args(
        "entry_copy",
        json!({ "id": vault.bank, "field": "Password" }),
    )
    .unwrap();
    rig.clipboard.other_app_copies("hola");

    rig.plain("app_prepare_update").unwrap();
    assert_eq!(rig.state(), "closed");
    assert_eq!(rig.clipboard.text().as_deref(), Some("hola"));
}

#[test]
fn rf_36_rf_75_prepare_update_writes_the_recovery_file_of_a_save_that_failed() {
    let rig = Rig::new("update-recovery");
    let vault = rig.open_canary_vault();
    // A folder sits where a save writes its temporary file: every save of this vault fails, on the real disk.
    let in_the_way = std::path::PathBuf::from(format!("{}.tmp", vault.path));
    std::fs::create_dir(&in_the_way).unwrap();
    std::fs::write(in_the_way.join("ocupado"), b"x").unwrap();
    rig.args(
        "entry_update",
        json!({ "id": vault.bank, "title": "Banco 2" }),
    )
    .unwrap();
    rig.events.wait_for(VAULT_SAVE_STATE_EVENT, "\"failed\"");
    assert_eq!(rig.plain("vault_status").unwrap()["save_pending"], true);
    assert_eq!(recovery_files(&rig), Vec::<String>::new());

    assert_eq!(rig.plain("app_prepare_update"), Ok(Value::Null));

    // RF-33: by the time the command answers, what was not saved is in its recovery file; the installer may
    // end the process right after.
    assert_eq!(rig.state(), "closed");
    let recovered = recovery_files(&rig);
    assert_eq!(recovered.len(), 1, "one recovery file: {recovered:?}");
    assert_eq!(rig.reloader.count(), 0);
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 0);

    // RF-34: the next unlock says where the changes are.
    let opened = rig
        .args(
            "vault_open",
            json!({ "path": vault.path, "password": common::vault::master() }),
        )
        .unwrap();
    assert_eq!(opened["recovery"]["kind"], "saved");
    assert!(
        opened["recovery"]["path"]
            .as_str()
            .unwrap()
            .ends_with(&recovered[0])
    );
}

#[test]
fn rf_29_rf_36_after_prepare_update_the_vault_opens_again_and_still_locks_by_inactivity() {
    let rig = Rig::new("update-goes-on");
    let vault = rig.open_canary_vault();
    rig.plain("app_prepare_update").unwrap();
    assert_eq!(rig.state(), "closed");

    // The installer did not start and the person goes on: the app is not left "exiting".
    rig.idle_clock.advance(HOUR);
    assert!(!rig.heartbeat(), "a closed vault has nothing to lock");
    let opened = rig
        .args(
            "vault_open",
            json!({ "path": vault.path, "password": common::vault::master() }),
        )
        .unwrap();
    assert_eq!(opened["state"], "open");
    assert_eq!(opened["locks_in_seconds"], 300);
    assert_eq!(
        rig.args("entry_get", json!({ "id": vault.bank })).unwrap()["title"],
        "Banco"
    );

    rig.idle_clock.advance(4 * 60 + 59);
    assert!(!rig.heartbeat());
    assert_eq!(rig.state(), "open");
    rig.idle_clock.advance(1);
    assert!(
        rig.heartbeat(),
        "RF-29: the inactivity lock is still running"
    );
    assert_eq!(rig.state(), "locked");
    // And this lock is an ordinary one: the window is told and reloaded (RF-32).
    assert_eq!(rig.events.count(VAULT_LOCKED_EVENT), 1);
    assert_eq!(rig.reloader.count(), 1);

    // The copy of a value is cleared by a later preparation too.
    rig.args(
        "vault_open",
        json!({ "path": vault.path, "password": common::vault::master() }),
    )
    .unwrap();
    rig.args(
        "entry_copy",
        json!({ "id": vault.bank, "field": "Password" }),
    )
    .unwrap();
    rig.plain("app_prepare_update").unwrap();
    assert_eq!(rig.state(), "closed");
    assert_eq!(rig.clipboard.text(), None);
}

// ---------- RF-29 (review 0.1, M-1): the inactivity lock does not wait for the settings file ----------

/// A window nobody looks at.
struct NoWindow;

impl WindowEvents for NoWindow {
    fn vault_locked(&self) {}
    fn save_state(&self, _state: &'static str) {}
}

/// The lock of the app over `settings`, with a clock the test moves, and the canary vault open in its session.
fn center_with_open_vault(
    dir: &TempDir,
    settings: &Arc<SettingsStore>,
    clock: &FakeIdleClock,
) -> Arc<LockCenter> {
    let center = Arc::new(LockCenter::new(LockParts {
        clipboard: None,
        settings: Arc::clone(settings),
        window: Arc::new(NoWindow),
        reloader: Arc::new(CountingReloader::default()),
        clock: Arc::new(clock.clone()),
    }));
    let path = dir.path().join("canario.kdbx");
    std::fs::write(&path, canary_vault_bytes()).unwrap();
    center.unlocking();
    center
        .session()
        .unlock(
            &path,
            Some(MasterPassword::new(common::vault::master())),
            None,
        )
        .unwrap();
    center.activity();
    center
}

/// The countdown and one heartbeat, on a thread of their own: a heartbeat that waits for the settings file fails
/// the test instead of hanging it.
fn countdown_and_heartbeat(center: &Arc<LockCenter>) -> (u64, bool) {
    let (answer, answered) = channel();
    let center = Arc::clone(center);
    thread::spawn(move || {
        // The test may have given up waiting.
        _ = answer.send((center.locks_in_seconds(), center.evaluate()));
    });
    answered
        .recv_timeout(PATIENCE)
        .expect("the inactivity lock waited for the settings file")
}

#[test]
fn rf_29_the_vault_locks_on_time_while_a_change_of_the_settings_hangs_on_the_disk() {
    let dir = TempDir::new("rf29-hung-settings");
    // A disk that hangs on the write the test arms, until it is released.
    let armed = Arc::new(AtomicBool::new(false));
    let (inside, entered) = channel();
    let (release, released) = channel::<()>();
    let gate = Mutex::new((inside, released));
    let hangs = Arc::clone(&armed);
    let settings = Arc::new(SettingsStore::with_writer(
        dir.settings_file(),
        Box::new(move |settings, file| {
            if hangs.swap(false, Ordering::SeqCst) {
                let gate = gate.lock().unwrap();
                gate.0.send(()).unwrap();
                gate.1.recv_timeout(PATIENCE).unwrap();
            }
            settings.save(file)
        }),
    ));
    settings
        .update(|settings| settings.set_auto_lock_minutes(1))
        .unwrap();
    let clock = FakeIdleClock::default();
    let center = center_with_open_vault(&dir, &settings, &clock);

    // The person chooses 30 minutes and the write hangs (a configuration folder on a network that went away).
    armed.store(true, Ordering::SeqCst);
    let changing = {
        let settings = Arc::clone(&settings);
        thread::spawn(move || settings.update(|settings| settings.set_auto_lock_minutes(30)))
    };
    entered.recv_timeout(PATIENCE).unwrap();

    clock.advance(59);
    assert_eq!(countdown_and_heartbeat(&center), (1, false));
    clock.advance(1);
    assert_eq!(
        countdown_and_heartbeat(&center),
        (0, true),
        "the minute that is on disk, not the 30 that are not there yet"
    );
    assert!(matches!(center.session().status(), Status::Locked { .. }));

    // The write ends: the change is kept, and counts from the next heartbeat.
    release.send(()).unwrap();
    let changed = changing.join().unwrap().unwrap();
    assert_eq!(changed.auto_lock_minutes(), 30);
    assert_eq!(settings.auto_lock_minutes(), 30);
    assert_eq!(Settings::load(&dir.settings_file()).auto_lock_minutes(), 30);
}

#[test]
fn rf_29_a_change_of_the_settings_that_could_not_be_written_does_not_move_the_inactivity_lock() {
    let dir = TempDir::new("rf29-failed-settings");
    let fails = Arc::new(AtomicBool::new(false));
    let failing = Arc::clone(&fails);
    let settings = Arc::new(SettingsStore::with_writer(
        dir.settings_file(),
        Box::new(move |settings, file| {
            if failing.load(Ordering::SeqCst) {
                return Err(CoreError::GuardadoFallido);
            }
            settings.save(file)
        }),
    ));
    let clock = FakeIdleClock::default();
    let center = center_with_open_vault(&dir, &settings, &clock);
    // Read from a file that is not there yet: the default.
    assert_eq!(settings.auto_lock_minutes(), 5);

    fails.store(true, Ordering::SeqCst);
    let refused = settings.update(|settings| settings.set_auto_lock_minutes(30));
    assert_eq!(refused.map(drop), Err(CoreError::GuardadoFallido));
    assert_eq!(settings.auto_lock_minutes(), 5);
    assert_eq!(settings.read().auto_lock_minutes(), 5);

    // And one that was written counts at the next heartbeat.
    fails.store(false, Ordering::SeqCst);
    clock.advance(60);
    assert_eq!(countdown_and_heartbeat(&center), (240, false));
    settings
        .update(|settings| settings.set_auto_lock_minutes(1))
        .unwrap();
    assert_eq!(countdown_and_heartbeat(&center), (0, true));
}
