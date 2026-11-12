//! The lock as the app runs it (T14; spec 001, section D): by inactivity with clocks the test moves (RF-29 and the
//! worked example 10), by order of the person (RF-28), on exit (RF-36, RF-75), and what happens around it: the
//! clipboard (RF-31, RF-73), the event and the reload of the webview (RF-32). No test sleeps: the heartbeat that
//! production runs every second is called by hand.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::time::Duration;

use almarioks_lib::lock::{IdleTimer, VAULT_LOCKED_EVENT, idle_between};
use common::{
    error_code,
    vault::{CANARY, FakeIdleClock, Rig},
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
