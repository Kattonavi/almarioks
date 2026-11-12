//! Independent validation from the spec: a lock that never fails open (RF-28, RF-30, RF-33 to RF-36, RF-88,
//! RF-90, D12). Only the public API; disk and clock are the test's own (`spec_support`).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod spec_support;

use std::{
    fs,
    sync::mpsc,
    thread,
    time::{Duration, Instant},
};

use almarioks_core::{
    CoreError,
    session::{Applied, Recovery, SaveState, Status},
};
use spec_support::{
    Act, Op, PW, Rig, WAIT, count, is_recovery_name, open_path, recovery_files, root_titles,
    status_open,
};

const ONE_SECOND: Duration = Duration::from_secs(1);

fn assert_every_command_is_locked(rig: &Rig) {
    assert_eq!(
        rig.session.read(|v| Ok(v.entry_count())),
        Err(CoreError::Bloqueada),
        "read"
    );
    assert_eq!(
        rig.add("Después del bloqueo"),
        Err(CoreError::Bloqueada),
        "mutate"
    );
    assert_eq!(
        rig.session.mutate(|_, _| Ok(Applied::Unchanged(()))),
        Err(CoreError::Bloqueada),
        "mutate that changes nothing"
    );
    assert_eq!(
        rig.session.retry_save().map(|_| ()),
        Err(CoreError::Bloqueada),
        "retry_save"
    );
    let elsewhere = rig.dir.join("otra-ruta.kdbx");
    assert_eq!(
        rig.session.save_as(&elsewhere).map(|_| ()),
        Err(CoreError::Bloqueada),
        "save_as"
    );
    assert!(
        !elsewhere.exists(),
        "save_as of a locked vault wrote a file"
    );
    assert_eq!(
        rig.session.reload().map(|_| ()),
        Err(CoreError::Bloqueada),
        "reload"
    );
}

#[test]
fn spec_rf_35_closed_session_answers_bloqueada_to_every_vault_command() {
    let rig = Rig::closed("rf35-closed");
    assert_eq!(rig.session.status(), Status::Closed);
    assert_every_command_is_locked(&rig);
}

#[test]
fn spec_rf_28_rf_35_after_lock_every_command_answers_bloqueada_and_nothing_is_written() {
    let rig = Rig::open("rf35-locked");
    let recovery = rig.session.lock();
    assert_eq!(
        recovery, None,
        "nothing was pending, so there is nothing to recover"
    );
    assert_eq!(
        rig.session.status(),
        Status::Locked {
            path: rig.path.clone()
        }
    );
    assert_eq!(
        rig.locked_rx.recv_timeout(WAIT),
        Ok(()),
        "the listener hears the lock (RF-31, RF-32)"
    );
    assert_every_command_is_locked(&rig);
    assert!(
        recovery_files(rig.dir.path()).is_empty(),
        "a lock with nothing pending wrote a recovery file"
    );
}

#[test]
fn spec_rf_36_close_does_the_same_as_lock_and_goes_back_to_welcome() {
    let rig = Rig::open("rf36-close");
    rig.add("Antes de cerrar").expect("add");
    assert_eq!(rig.settle(), SaveState::Saved);
    assert_eq!(rig.session.close(), None);
    assert_eq!(rig.session.status(), Status::Closed);
    assert_eq!(
        rig.locked_rx.recv_timeout(WAIT),
        Ok(()),
        "close tells the listener like a lock"
    );
    assert_every_command_is_locked(&rig);
    // Back at the welcome screen the same vault can be opened again, with what was saved.
    let open = status_open(rig.unlock().expect("unlock after close"));
    assert_eq!(open.recovery, None);
    assert_eq!(rig.titles(), Ok(vec!["Antes de cerrar".to_owned()]));
}

#[test]
fn spec_rf_30_rf_33_lock_with_a_hung_save_answers_bloqueada_within_1s_and_recovers_the_change() {
    let rig = Rig::open("rf30-hung");
    let hung = rig.disk.watch();
    rig.disk.set_main(Act::Hang);
    rig.add("Entrada colgada")
        .expect("a change answers without waiting for the disk");
    let (op, _) = hung
        .recv_timeout(WAIT)
        .expect("the save reaches the disk and hangs");
    assert_ne!(op, Op::Read, "hung on a write step");

    let started = Instant::now();
    let (tx, rx) = mpsc::channel();
    let session = rig.session.clone();
    thread::spawn(move || {
        let _ = tx.send(session.lock());
    });
    rig.locked_rx.recv_timeout(WAIT).expect("the lock starts");
    assert_eq!(
        rig.session.read(|v| Ok(v.entry_count())),
        Err(CoreError::Bloqueada)
    );
    assert_eq!(rig.add("Otra"), Err(CoreError::Bloqueada));
    assert!(matches!(rig.session.status(), Status::Locked { .. }));
    let elapsed = started.elapsed();
    assert!(
        elapsed < ONE_SECOND,
        "RF-30: commands answered Bloqueada after {elapsed:?}"
    );

    // Casos límite: "Bloqueo mientras un guardado está colgado → lo pendiente va a recuperación".
    let recovery = rx
        .recv_timeout(Duration::from_secs(10))
        .expect("lock returns while the disk still hangs");
    let Some(Recovery::Saved { path }) = recovery else {
        panic!("RF-33: the pending change should be in a recovery file, got {recovery:?}");
    };
    assert!(
        is_recovery_name(&path, &rig.file_name()),
        "RF-33 name: {path:?}"
    );
    assert_eq!(
        root_titles(&path),
        vec!["Entrada colgada".to_owned()],
        "the recovery opens with the key"
    );
    rig.disk.release();
}

#[test]
fn spec_rf_33_rf_34_failed_save_goes_to_a_new_recovery_file_each_time_and_unlock_says_it_once() {
    let rig = Rig::open("rf33-failed");
    rig.disk.set_main(Act::Fail);
    rig.add("Cambio pendiente").expect("add");
    assert!(matches!(rig.settle(), SaveState::Failed(_)), "RF-23");
    let open = status_open(rig.session.status());
    assert!(open.save_pending, "RF-23: the change stays in memory");

    let first = match rig.session.lock() {
        Some(Recovery::Saved { path }) => path,
        other => panic!("RF-33: expected a recovery file, got {other:?}"),
    };
    assert!(
        is_recovery_name(&first, &rig.file_name()),
        "RF-33 name: {first:?}"
    );
    assert_eq!(root_titles(&first), vec!["Cambio pendiente".to_owned()]);
    assert_eq!(count(&rig.path), 0, "RF-23: the vault file is as it was");
    let first_bytes = fs::read(&first).expect("first recovery");

    // RF-34: the next unlock says where the changes went.
    let open = status_open(rig.unlock().expect("unlock"));
    assert_eq!(
        open.recovery,
        Some(Recovery::Saved {
            path: first.clone()
        })
    );

    // Same second on the clock: the second recovery must not replace the first (RF-33).
    rig.add("Segundo pendiente").expect("add");
    assert!(matches!(rig.settle(), SaveState::Failed(_)));
    let second = match rig.session.lock() {
        Some(Recovery::Saved { path }) => path,
        other => panic!("RF-33: expected a second recovery file, got {other:?}"),
    };
    assert_ne!(second, first, "RF-33: a recovery replaced the previous one");
    assert_eq!(
        fs::read(&first).expect("first recovery"),
        first_bytes,
        "the first recovery is untouched"
    );
    assert_eq!(root_titles(&second), vec!["Segundo pendiente".to_owned()]);
    assert_eq!(recovery_files(rig.dir.path()).len(), 2);

    // RF-34 once: told at the next unlock, never again.
    rig.disk.set_main(Act::Pass);
    let open = status_open(rig.unlock().expect("unlock"));
    assert_eq!(open.recovery, Some(Recovery::Saved { path: second }));
    assert_eq!(rig.session.lock(), None, "nothing pending this time");
    let open = status_open(rig.unlock().expect("unlock"));
    assert_eq!(open.recovery, None, "RF-34: the note was already given");
}

#[test]
fn spec_rf_34_when_the_recovery_cannot_be_written_unlock_says_the_changes_were_lost() {
    let rig = Rig::open("rf34-lost");
    rig.disk.set_main(Act::Fail);
    rig.disk.set_recovery(Act::Fail);
    rig.add("Se perderá").expect("add");
    assert!(matches!(rig.settle(), SaveState::Failed(_)));
    assert_eq!(rig.session.lock(), Some(Recovery::Lost));
    assert!(recovery_files(rig.dir.path()).is_empty());
    rig.disk.reset();
    let open = status_open(rig.unlock().expect("unlock"));
    assert_eq!(
        open.recovery,
        Some(Recovery::Lost),
        "RF-34: say they were lost"
    );
    assert_eq!(rig.titles(), Ok(Vec::new()));
}

#[test]
fn spec_rf_33_a_hung_recovery_write_does_not_hold_the_lock_beyond_its_limit() {
    let rig = Rig::open("rf33-limit");
    rig.disk.set_main(Act::Fail);
    rig.add("Pendiente").expect("add");
    assert!(matches!(rig.settle(), SaveState::Failed(_)));
    let hung = rig.disk.watch();
    rig.disk.set_recovery(Act::Hang);

    let started = Instant::now();
    let (tx, rx) = mpsc::channel();
    let session = rig.session.clone();
    thread::spawn(move || {
        let _ = tx.send(session.lock());
    });
    rig.locked_rx.recv_timeout(WAIT).expect("lock starts");
    assert_eq!(
        rig.session.read(|v| Ok(v.entry_count())),
        Err(CoreError::Bloqueada)
    );
    assert!(
        started.elapsed() < ONE_SECOND,
        "RF-30 with the recovery write hanging"
    );
    let (op, path) = hung
        .recv_timeout(WAIT)
        .expect("the recovery write starts and hangs");
    assert!(matches!(op, Op::Write | Op::WriteNew), "{op:?} {path:?}");

    // 5 s limit (RF-33): by the injected clock if it asked for an alarm, otherwise real time. Never a sleep here.
    let deadline = Instant::now() + Duration::from_secs(8);
    let result = loop {
        rig.clock.ring_all();
        match rx.recv_timeout(Duration::from_millis(50)) {
            Ok(result) => break result,
            Err(_) if Instant::now() < deadline => {}
            Err(error) => panic!(
                "RF-33: the lock waited for a hung recovery write beyond the limit ({error:?})"
            ),
        }
    };
    assert!(
        !matches!(result, Some(Recovery::Saved { .. })),
        "the recovery file was never written, yet the lock says it was: {result:?}"
    );
    rig.disk.release();
}

#[test]
fn spec_rf_88_rf_18_a_second_unlock_while_the_first_runs_is_ocupada() {
    let rig = Rig::closed("rf88-busy");
    let hung = rig.disk.watch();
    rig.disk.hang_next_reads(1);
    let (tx, rx) = mpsc::channel();
    let session = rig.session.clone();
    let path = rig.path.clone();
    thread::spawn(move || {
        let _ = tx.send(
            session
                .unlock(&path, spec_support::password(PW), None)
                .map(|_| ()),
        );
    });
    hung.recv_timeout(WAIT)
        .expect("the first unlock reads the vault");

    let asked = Instant::now();
    let status = rig.session.status();
    assert!(
        asked.elapsed() < Duration::from_millis(200),
        "RF-18: status took {:?}",
        asked.elapsed()
    );
    assert!(!matches!(status, Status::Open(_)), "{status:?}");

    let asked = Instant::now();
    assert_eq!(rig.unlock().map(|_| ()), Err(CoreError::Ocupada), "RF-88");
    assert!(
        asked.elapsed() < ONE_SECOND,
        "the second unlock waited for the first"
    );
    rig.disk.release();
    assert_eq!(rx.recv_timeout(WAIT).expect("first unlock ends"), Ok(()));
    assert!(matches!(rig.session.status(), Status::Open(_)));
}

#[test]
fn spec_rf_30_a_lock_asked_during_an_unlock_leaves_the_vault_locked() {
    let rig = Rig::closed("rf30-during-unlock");
    let hung = rig.disk.watch();
    rig.disk.hang_next_reads(1);
    let (tx, rx) = mpsc::channel();
    let session = rig.session.clone();
    let path = rig.path.clone();
    thread::spawn(move || {
        let _ = tx.send(
            session
                .unlock(&path, spec_support::password(PW), None)
                .map(|_| ()),
        );
    });
    hung.recv_timeout(WAIT).expect("the unlock reads the vault");
    let _ = rig.session.lock();
    rig.disk.release();
    let unlocked = rx.recv_timeout(WAIT).expect("unlock ends");
    assert!(
        unlocked.is_err(),
        "the unlock that was running when the lock came must not open the vault"
    );
    assert!(!matches!(rig.session.status(), Status::Open(_)));
    assert_eq!(
        rig.session.read(|v| Ok(v.entry_count())),
        Err(CoreError::Bloqueada)
    );
}

#[test]
fn spec_rf_90_a_value_read_when_the_lock_arrives_is_answered_bloqueada() {
    let rig = Rig::open("rf90-copy");
    rig.add("GitHub").expect("add");
    assert_eq!(rig.settle(), SaveState::Saved);
    let (inside_tx, inside_rx) = mpsc::channel();
    let (go_tx, go_rx) = mpsc::channel::<()>();
    let session = rig.session.clone();
    let reader = thread::spawn(move || {
        session.read(|vault| {
            let _ = inside_tx.send(());
            let _ = go_rx.recv_timeout(Duration::from_secs(5));
            Ok(vault.entry_count())
        })
    });
    inside_rx.recv_timeout(WAIT).expect("the copy is reading");
    let session = rig.session.clone();
    let locker = thread::spawn(move || session.lock());
    let heard = rig.locked_rx.recv_timeout(Duration::from_secs(2));
    let _ = go_tx.send(());
    assert_eq!(
        reader.join().expect("reader"),
        Err(CoreError::Bloqueada),
        "RF-90"
    );
    let _ = locker.join();
    assert!(
        heard.is_ok(),
        "RF-30: the lock was not announced while a read was running"
    );
    assert_eq!(
        rig.session.read(|v| Ok(v.entry_count())),
        Err(CoreError::Bloqueada)
    );
}

#[test]
fn spec_rf_33_the_recovery_holds_only_encrypted_bytes_that_open_with_the_vault_key_and_no_other() {
    let rig = Rig::open("rf33-key");
    rig.disk.set_main(Act::Fail);
    rig.add("Cifrada").expect("add");
    assert!(matches!(rig.settle(), SaveState::Failed(_)));
    let Some(Recovery::Saved { path }) = rig.session.lock() else {
        panic!("expected a recovery file");
    };
    let bytes = fs::read(&path).expect("recovery");
    assert!(
        !bytes.windows(b"Cifrada".len()).any(|w| w == b"Cifrada"),
        "RF-77: the recovery file has the title in clear"
    );
    assert_eq!(
        almarioks_core::kdbx::open(&bytes, &spec_support::key("otra-llave")).map(|_| ()),
        Err(CoreError::LlaveIncorrecta)
    );
    assert!(open_path(&path).is_ok());
}
