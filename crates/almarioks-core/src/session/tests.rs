//! The session against the disk in memory that fails or hangs (spec, "Cómo se prueba", rows 18, 23, 30, 33 and 34)
//! and a clock moved by hand. Vaults are real (2 iterations). No test sleeps: every wait is a channel that a step
//! of the session answers; `GUARD` only bounds it so a bug fails instead of hanging.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this module).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    fs,
    io::ErrorKind,
    mem,
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
        mpsc,
    },
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use super::{
    Applied, Clock, Phase, RECOVERY_LIMIT, Recovery, Ring, SaveState, Session, SessionEvents,
    Status,
    gate::hold,
    live::{Live, Secrets},
};
use crate::{
    error::{CoreError, CoreResult},
    kdbx::{self, MasterPassword, NewKeyFile, NewVaultKdf, OpenedVault, VaultKey},
    storage::{
        Disk, FailureCause, StdDisk,
        test_disk::{Fault, MemDisk},
    },
    vault::{EntryId, EntryPatch, NewEntry, Timestamp},
};

const GUARD: Duration = Duration::from_secs(30);
/// Master password of the test vaults (fake data).
const PASSWORD: &str = "sesion-Prueba-2026";
/// 2026-11-11 08:30:05 UTC: what the clock of the tests says.
const NOW: u64 = 1_794_385_805;
/// A save is: read the vault, write the temporary file, read it back, copy to `.bak.tmp`, clear its read-only
/// attribute, replace the vault, promote `.bak.tmp`.
const WRITE_TMP_OP: usize = 1;
const REPLACE_OP: usize = 5;

fn vault_path() -> PathBuf {
    PathBuf::from("bovedas").join("cuentas.kdbx")
}

fn sibling(suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", vault_path().display()))
}

fn recovery_path() -> PathBuf {
    sibling(".recuperacion-20261111-083005.kdbx")
}

fn password() -> Option<MasterPassword> {
    Some(MasterPassword::new(PASSWORD.to_owned()))
}

fn key() -> VaultKey {
    VaultKey::new(password(), None).unwrap()
}

fn kdf() -> NewVaultKdf {
    NewVaultKdf::with_iterations(2).unwrap()
}

fn new_vault() -> Vec<u8> {
    kdbx::create_vault(&key(), "Cuentas", kdf()).unwrap()
}

fn entry(title: &str) -> NewEntry {
    NewEntry {
        title: title.to_owned(),
        ..NewEntry::default()
    }
}

fn titles_of(vault: &OpenedVault) -> CoreResult<Vec<String>> {
    Ok(vault
        .list_entries(vault.root_id())?
        .into_iter()
        .map(|entry| entry.title.unwrap_or_default())
        .collect())
}

/// The titles in a vault file, which must open with the key of the tests.
fn titles_in(bytes: &[u8]) -> Vec<String> {
    titles_of(&kdbx::open(bytes, &key()).unwrap()).unwrap()
}

/// A vault another app would write: same key, one entry.
fn vault_of_another_app(title: &str) -> Vec<u8> {
    let mut vault = kdbx::open(&new_vault(), &key()).unwrap();
    let root = vault.root_id();
    vault
        .create_entry(root, entry(title), Timestamp::EPOCH)
        .unwrap();
    vault.to_bytes(&key()).unwrap()
}

/// A clock that only moves when the test says so.
struct FakeClock {
    now: Mutex<SystemTime>,
    alarms: Mutex<Vec<(SystemTime, Ring)>>,
}

impl FakeClock {
    fn new() -> Self {
        Self {
            now: Mutex::new(UNIX_EPOCH + Duration::from_secs(NOW)),
            alarms: Mutex::new(Vec::new()),
        }
    }

    /// Moves the time and rings what is due.
    fn advance(&self, by: Duration) {
        let now = {
            let mut now = hold(&self.now);
            *now += by;
            *now
        };
        let (due, later): (Vec<_>, Vec<_>) = mem::take(&mut *hold(&self.alarms))
            .into_iter()
            .partition(|(at, _)| *at <= now);
        hold(&self.alarms).extend(later);
        for (_, ring) in due {
            ring();
        }
    }
}

impl Clock for FakeClock {
    fn now(&self) -> SystemTime {
        *hold(&self.now)
    }

    fn alarm(&self, after: Duration, ring: Ring) {
        let at = self.now() + after;
        hold(&self.alarms).push((at, ring));
    }
}

type Hook = Box<dyn Fn() + Send>;

/// Listens to the session as the app would.
struct Recorder {
    locked: AtomicUsize,
    saves: Mutex<mpsc::Sender<SaveState>>,
    on_locked: Mutex<Option<Hook>>,
}

impl SessionEvents for Recorder {
    fn locked(&self) {
        self.locked.fetch_add(1, Ordering::SeqCst);
        if let Some(hook) = hold(&self.on_locked).as_ref() {
            hook();
        }
    }

    fn save_state(&self, state: SaveState) {
        _ = hold(&self.saves).send(state); // a test that ended does not listen
    }
}

struct Rig {
    disk: Arc<MemDisk>,
    clock: Arc<FakeClock>,
    events: Arc<Recorder>,
    saves: Mutex<mpsc::Receiver<SaveState>>,
    session: Arc<Session>,
}

/// A closed session over a disk in memory that has one empty vault.
fn rig() -> Rig {
    let disk = Arc::new(MemDisk::default());
    disk.put(&vault_path(), &new_vault());
    rig_on(disk.clone(), disk)
}

fn rig_on(disk: Arc<MemDisk>, session_disk: Arc<dyn Disk>) -> Rig {
    let clock = Arc::new(FakeClock::new());
    let (sender, saves) = mpsc::channel();
    let events = Arc::new(Recorder {
        locked: AtomicUsize::new(0),
        saves: Mutex::new(sender),
        on_locked: Mutex::new(None),
    });
    let session = Arc::new(Session::new(session_disk, clock.clone(), events.clone()));
    Rig {
        disk,
        clock,
        events,
        saves: Mutex::new(saves),
        session,
    }
}

impl Rig {
    fn unlock(&self) -> CoreResult<Status> {
        self.session.unlock(&vault_path(), password(), None)
    }

    /// Unlocks and returns the open status.
    fn open(&self) -> super::OpenStatus {
        match self.unlock().unwrap() {
            Status::Open(open) => open,
            other => panic!("not open: {other:?}"),
        }
    }

    fn open_status(&self) -> super::OpenStatus {
        match self.session.status() {
            Status::Open(open) => open,
            other => panic!("not open: {other:?}"),
        }
    }

    fn add(&self, title: &str) -> CoreResult<EntryId> {
        self.session.mutate(|vault, now| {
            let root = vault.root_id();
            vault
                .create_entry(root, entry(title), now)
                .map(Applied::Changed)
        })
    }

    fn titles(&self) -> CoreResult<Vec<String>> {
        self.session.read(titles_of)
    }

    fn next_save(&self) -> SaveState {
        hold(&self.saves).recv_timeout(GUARD).unwrap()
    }

    /// Adds an entry and waits for the two states its save goes through.
    fn add_and_expect(&self, title: &str, end: SaveState) {
        self.add(title).unwrap();
        assert_eq!(self.next_save(), SaveState::Saving);
        assert_eq!(self.next_save(), end);
    }

    /// Adds an entry whose save fails because the disk is full.
    fn add_unsaved(&self, title: &str) {
        self.disk
            .fault_at(WRITE_TMP_OP, Fault::Fail(ErrorKind::StorageFull));
        self.add_and_expect(title, SaveState::Failed(FailureCause::Io));
    }

    fn file(&self, path: &Path) -> Vec<u8> {
        self.disk.get(path).unwrap()
    }

    /// Weak references to the secrets of the open session: dead means dropped.
    fn probes(&self) -> (Weak<Live>, Weak<Secrets>) {
        match &self.session.state().phase {
            Phase::Open { live, .. } => (Arc::downgrade(live), Arc::downgrade(&live.secrets)),
            _ => panic!("not open"),
        }
    }

    /// Waits for the save threads of the sessions that were locked (a join, not a sleep).
    fn settle(&self) {
        let retired = mem::take(&mut self.session.state().retired);
        for queue in retired {
            queue.shutdown().unwrap();
        }
    }

    /// RF-35: every command that reads or changes the vault answers `Bloqueada`.
    fn assert_every_command_is_blocked(&self) {
        assert_eq!(self.titles(), Err(CoreError::Bloqueada));
        assert_eq!(self.add("No entra"), Err(CoreError::Bloqueada));
        assert_eq!(self.session.retry_save(), Err(CoreError::Bloqueada));
        assert_eq!(
            self.session.save_as(&sibling(".otra.kdbx")),
            Err(CoreError::Bloqueada)
        );
        assert_eq!(self.session.reload(), Err(CoreError::Bloqueada));
    }
}

/// Arms a hang at the `nth` operation of the disk from now; returns (entered, release).
fn hang_at(disk: &MemDisk, nth: usize) -> (mpsc::Receiver<()>, mpsc::SyncSender<()>) {
    let (entered_tx, entered) = mpsc::sync_channel(1);
    let (release, release_rx) = mpsc::sync_channel(1);
    disk.fault_at(
        nth,
        Fault::Hang {
            entered: entered_tx,
            release: release_rx,
        },
    );
    (entered, release)
}

fn locked() -> Status {
    Status::Locked { path: vault_path() }
}

// ---- States and unlock ----

#[test]
fn rf_35_closed_and_locked_answer_bloqueada_to_every_command() {
    fn send_and_sync<T: Send + Sync>() {}
    send_and_sync::<Session>();

    let rig = rig();
    assert_eq!(rig.session.status(), Status::Closed);
    rig.assert_every_command_is_blocked();

    let open = rig.open();
    assert_eq!(open.path, vault_path());
    assert_eq!(open.name, "Cuentas");
    assert!(!open.read_only && !open.weak_kdf && !open.save_pending);
    assert_eq!(open.save, SaveState::Saved);
    assert_eq!(open.recovery, None);
    rig.add_and_expect("GitHub", SaveState::Saved);
    assert_eq!(rig.titles().unwrap(), ["GitHub"]);

    assert_eq!(rig.session.lock(), None, "nothing was pending");
    assert_eq!(rig.session.status(), locked());
    rig.assert_every_command_is_blocked();
}

#[test]
fn rf_28_lock_leaves_the_vault_locked_and_tells_the_listener() {
    let rig = rig();
    rig.open();
    rig.add_and_expect("GitHub", SaveState::Saved);
    let (live, secrets) = rig.probes();

    assert_eq!(rig.session.lock(), None);

    assert_eq!(rig.session.status(), locked());
    assert_eq!(rig.events.locked.load(Ordering::SeqCst), 1);
    assert!(
        live.upgrade().is_none() && secrets.upgrade().is_none(),
        "rule 7: neither the vault nor the key is in memory"
    );
    // Unlocking again shows what was saved, and there is nothing to tell (RF-34).
    assert_eq!(rig.open().recovery, None);
    assert_eq!(rig.titles().unwrap(), ["GitHub"]);
    // Locking twice is the same as once.
    rig.session.lock();
    assert_eq!(rig.session.lock(), None);
    assert_eq!(rig.session.status(), locked());
}

#[test]
fn rf_95_a_failed_unlock_leaves_the_session_as_it_was() {
    let rig = rig();
    let missing = PathBuf::from("bovedas").join("no-existe.kdbx");
    let wrong = || Some(MasterPassword::new("otra-clave".to_owned()));

    assert_eq!(
        rig.session.unlock(&missing, password(), None),
        Err(CoreError::ArchivoNoEncontrado)
    );
    assert_eq!(
        rig.session
            .unlock(&vault_path(), password(), Some(&missing)),
        Err(CoreError::ArchivoLlaveNoEncontrado)
    );
    assert_eq!(
        rig.session.unlock(&vault_path(), wrong(), None),
        Err(CoreError::LlaveIncorrecta)
    );
    assert_eq!(
        rig.session.unlock(&vault_path(), None, None),
        Err(CoreError::LlaveVacia)
    );
    assert_eq!(rig.session.status(), Status::Closed);

    rig.open();
    rig.session.lock();
    assert_eq!(
        rig.session.unlock(&vault_path(), wrong(), None),
        Err(CoreError::LlaveIncorrecta)
    );
    assert_eq!(
        rig.session.unlock(&missing, password(), None),
        Err(CoreError::ArchivoNoEncontrado)
    );
    assert_eq!(rig.session.status(), locked(), "RF-11: it stays locked");
    rig.open();
}

#[test]
fn rf_88_a_second_unlock_while_one_is_running_is_ocupada() {
    let rig = rig();
    let (entered, release) = hang_at(&rig.disk, 0); // the first unlock stops while reading the vault file

    thread::scope(|scope| {
        let first = scope.spawn(|| rig.unlock());
        entered.recv_timeout(GUARD).unwrap();

        // RF-18: the state answers while the unlock runs.
        let started = Instant::now();
        assert_eq!(
            rig.session.status(),
            Status::Deriving { path: vault_path() }
        );
        assert!(started.elapsed() < Duration::from_millis(200), "RF-18");
        assert_eq!(rig.unlock(), Err(CoreError::Ocupada));
        assert_eq!(rig.titles(), Err(CoreError::Bloqueada), "RF-35");
        assert_eq!(
            rig.session.status(),
            Status::Deriving { path: vault_path() },
            "the refused unlock changed nothing"
        );

        release.send(()).unwrap();
        assert!(matches!(first.join().unwrap(), Ok(Status::Open(_))));
    });
    assert!(rig.titles().unwrap().is_empty());
    assert_eq!(rig.disk.op_count(), 1, "one unlock read the file once");
}

#[test]
fn rf_88_a_lock_or_a_close_during_the_unlock_discards_it() {
    for close in [false, true] {
        let rig = rig();
        let (entered, release) = hang_at(&rig.disk, 0);

        thread::scope(|scope| {
            let unlock = scope.spawn(|| rig.unlock());
            entered.recv_timeout(GUARD).unwrap();
            let asked = if close {
                rig.session.close()
            } else {
                rig.session.lock()
            };
            assert_eq!(asked, None);
            release.send(()).unwrap();
            // The key was derived and the vault decrypted, and both are thrown away.
            assert_eq!(unlock.join().unwrap(), Err(CoreError::Bloqueada));
        });

        let expected = if close { Status::Closed } else { locked() };
        assert_eq!(rig.session.status(), expected);
        rig.assert_every_command_is_blocked();
        assert_eq!(rig.events.locked.load(Ordering::SeqCst), 1);
        rig.open(); // and the session still works
    }
}

#[test]
fn rf_1_create_leaves_the_session_open_on_the_new_vault() {
    let folder = std::env::temp_dir().join(format!(
        "almarioks-session-test-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&folder).unwrap();
    let (path, key_path) = (folder.join("nueva.kdbx"), folder.join("nueva.keyx"));
    let rig = rig_on(Arc::new(MemDisk::default()), Arc::new(StdDisk));

    let created = rig.session.create(
        &path,
        "Mi bóveda",
        password(),
        NewKeyFile::Generate(key_path.clone()),
        kdf(),
    );
    let Ok(Status::Open(open)) = created else {
        panic!("not created: {created:?}");
    };
    assert_eq!(
        (open.path.as_path(), open.name.as_str()),
        (path.as_path(), "Mi bóveda")
    );
    assert!(!open.read_only && !open.weak_kdf);
    rig.add_and_expect("GitHub", SaveState::Saved);
    assert_eq!(rig.session.close(), None);
    assert_eq!(rig.session.status(), Status::Closed);

    // RF-1: it opens again with that key and with no other.
    assert_eq!(
        rig.session.unlock(&path, password(), None),
        Err(CoreError::LlaveIncorrecta)
    );
    assert!(matches!(
        rig.session.unlock(&path, password(), Some(&key_path)),
        Ok(Status::Open(_))
    ));
    assert_eq!(rig.titles().unwrap(), ["GitHub"]);
    rig.session.close();
    // RF-4: creating over it is refused and the session stays closed.
    assert_eq!(
        rig.session
            .create(&path, "Otra", password(), NewKeyFile::None, kdf()),
        Err(CoreError::ArchivoExiste)
    );
    assert_eq!(rig.session.status(), Status::Closed);
    rig.settle();
    drop(fs::remove_dir_all(&folder));
}

// ---- Changes that save themselves ----

#[test]
fn rf_21_every_change_is_saved_at_once_and_a_change_of_nothing_is_not() {
    let rig = rig();
    rig.open();

    let id = rig.add("GitHub").unwrap();
    assert_eq!(rig.next_save(), SaveState::Saving);
    assert_eq!(rig.next_save(), SaveState::Saved);
    assert_eq!(titles_in(&rig.file(&vault_path())), ["GitHub"]);
    assert!(
        titles_in(&rig.file(&sibling(".bak"))).is_empty(),
        "RF-22: the previous version"
    );
    let open = rig.open_status();
    assert_eq!((open.save, open.save_pending), (SaveState::Saved, false));
    // The dates come from the injected clock.
    let modified = rig.session.read(|vault| vault.entry(id)).unwrap();
    assert_eq!(
        modified.summary.modified,
        Timestamp::from_unix_seconds(i64::try_from(NOW).unwrap()).unwrap()
    );

    // RF-45: an edit that changes nothing is not saved, so the backup keeps the real previous version.
    let ops = rig.disk.op_count();
    let same_title = EntryPatch {
        title: Some("GitHub".to_owned()),
        ..EntryPatch::default()
    };
    let changed = rig
        .session
        .mutate(|vault, now| {
            vault
                .update_entry(id, same_title, now)
                .map(|changed| Applied::when(changed, changed))
        })
        .unwrap();
    assert!(!changed);
    // A change that fails is not saved either.
    assert_eq!(rig.add("   "), Err(CoreError::TituloVacio));
    assert_eq!(rig.disk.op_count(), ops, "the disk was not touched");
    assert!(hold(&rig.saves).try_recv().is_err(), "nothing was told");

    // Example 6: the next change leaves the one before in the backup.
    rig.add_and_expect("Banco", SaveState::Saved);
    assert_eq!(titles_in(&rig.file(&vault_path())), ["Banco", "GitHub"]);
    assert_eq!(titles_in(&rig.file(&sibling(".bak"))), ["GitHub"]);
}

#[test]
fn rf_14_a_kdbx_3_vault_answers_solo_lectura_before_touching_anything() {
    let fixture = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("keepass2")
        .join("v3-aes.kdbx");
    let rig = rig();
    rig.disk.put(&vault_path(), &fs::read(fixture).unwrap());
    let fixture_password = Some(MasterPassword::new("lab-Maestra-2026".to_owned()));

    let status = rig
        .session
        .unlock(&vault_path(), fixture_password, None)
        .unwrap();
    let Status::Open(open) = status else {
        panic!("not open");
    };
    assert!(open.read_only);

    let ops = rig.disk.op_count();
    let called = AtomicBool::new(false);
    let answer = rig.session.mutate(|_, _| {
        called.store(true, Ordering::SeqCst);
        Ok(Applied::Changed(()))
    });
    assert_eq!(answer, Err(CoreError::SoloLectura));
    assert!(!called.load(Ordering::SeqCst), "the change never ran");
    assert_eq!(
        rig.session.save_as(&sibling(".copia.kdbx")),
        Err(CoreError::SoloLectura)
    );
    assert_eq!(rig.disk.op_count(), ops);
    assert!(rig.session.read(|vault| Ok(vault.entry_count())).is_ok());
    assert_eq!(rig.session.lock(), None);
}

#[test]
fn rf_23_a_failed_save_keeps_the_vault_editable_and_a_retry_saves_it() {
    let rig = rig();
    rig.open();
    let original = rig.file(&vault_path());

    rig.add_unsaved("GitHub");

    assert_eq!(rig.file(&vault_path()), original, "the file is intact");
    let open = rig.open_status();
    assert_eq!(open.save, SaveState::Failed(FailureCause::Io));
    assert_eq!(open.save.as_str(), "failed");
    assert!(open.save_pending);
    assert_eq!(rig.titles().unwrap(), ["GitHub"], "the change is in memory");

    // Retry: the same bytes, now with a disk that works.
    assert!(rig.session.retry_save().is_ok());
    assert_eq!(rig.next_save(), SaveState::Saving);
    assert_eq!(rig.next_save(), SaveState::Saved);
    assert_eq!(titles_in(&rig.file(&vault_path())), ["GitHub"]);
    assert!(!rig.open_status().save_pending);

    // And the vault was editable all along: a later change saves both.
    rig.add_unsaved("Banco");
    rig.add_and_expect("Correo", SaveState::Saved);
    assert_eq!(
        titles_in(&rig.file(&vault_path())),
        ["Banco", "Correo", "GitHub"]
    );
    // With nothing pending a retry does nothing.
    let ops = rig.disk.op_count();
    assert!(rig.session.retry_save().is_ok());
    assert_eq!(rig.disk.op_count(), ops);
}

#[test]
fn rf_89_changes_made_while_a_save_is_being_written_wait_their_turn() {
    let rig = rig();
    rig.open();
    let (entered, release) = hang_at(&rig.disk, WRITE_TMP_OP);

    rig.add("GitHub").unwrap();
    assert_eq!(rig.next_save(), SaveState::Saving);
    entered.recv_timeout(GUARD).unwrap();
    // The change does not wait for the disk: the second one is answered while the first is being written.
    rig.add("Banco").unwrap();
    assert_eq!(rig.titles().unwrap(), ["Banco", "GitHub"]);
    let open = rig.open_status();
    assert_eq!((open.save, open.save_pending), (SaveState::Saving, true));
    // Nobody else may take the file while its save thread is writing.
    assert_eq!(rig.session.reload(), Err(CoreError::Ocupada));
    assert_eq!(
        rig.session.save_as(&sibling(".copia.kdbx")),
        Err(CoreError::Ocupada)
    );

    release.send(()).unwrap();
    // One "saved", when the newest change is on disk.
    assert_eq!(rig.next_save(), SaveState::Saved);
    assert_eq!(titles_in(&rig.file(&vault_path())), ["Banco", "GitHub"]);
    assert_eq!(titles_in(&rig.file(&sibling(".bak"))), ["GitHub"]);
    assert!(!rig.open_status().save_pending);
    assert!(hold(&rig.saves).try_recv().is_err());
}

#[test]
fn rf_24_after_an_external_change_save_as_goes_on_in_the_new_file() {
    let rig = rig();
    rig.open();
    let theirs = vault_of_another_app("De otra app");
    rig.disk.put(&vault_path(), &theirs);

    rig.add_and_expect("GitHub", SaveState::ExternalChange);
    assert_eq!(rig.file(&vault_path()), theirs, "it was not overwritten");
    assert_eq!(rig.open_status().save.as_str(), "external-change");
    // Trying again does not overwrite it either.
    assert!(rig.session.retry_save().is_ok());
    assert_eq!(rig.next_save(), SaveState::Saving);
    assert_eq!(rig.next_save(), SaveState::ExternalChange);

    // "Save as" never replaces a file, and failing leaves everything as it was.
    assert_eq!(
        rig.session.save_as(&vault_path()),
        Err(CoreError::ArchivoExiste)
    );
    let open = rig.open_status();
    assert_eq!(
        (open.path, open.save, open.save_pending),
        (vault_path(), SaveState::ExternalChange, true)
    );

    let copy = PathBuf::from("bovedas").join("copia.kdbx");
    let Ok(Status::Open(open)) = rig.session.save_as(&copy) else {
        panic!("save as failed");
    };
    assert_eq!(
        (open.path, open.save, open.save_pending),
        (copy.clone(), SaveState::Saved, false)
    );
    assert_eq!(rig.next_save(), SaveState::Saved);
    assert_eq!(titles_in(&rig.file(&copy)), ["GitHub"]);
    assert_eq!(rig.file(&vault_path()), theirs);

    // The session follows the new file.
    rig.add_and_expect("Banco", SaveState::Saved);
    assert_eq!(titles_in(&rig.file(&copy)), ["Banco", "GitHub"]);
    assert_eq!(rig.file(&vault_path()), theirs);
    rig.session.lock();
    assert_eq!(rig.session.status(), Status::Locked { path: copy });
}

#[test]
fn rf_24_after_an_external_change_reload_drops_what_was_pending() {
    let rig = rig();
    rig.open();
    rig.disk.put(&vault_path(), b"otra app dejo esto a medias");
    rig.add_and_expect("GitHub", SaveState::ExternalChange);

    // A reload that cannot read the file leaves the session as it was, changes included.
    assert_eq!(rig.session.reload(), Err(CoreError::ArchivoDanado));
    assert_eq!(rig.titles().unwrap(), ["GitHub"]);
    let open = rig.open_status();
    assert_eq!(
        (open.save, open.save_pending),
        (SaveState::ExternalChange, true)
    );

    let theirs = vault_of_another_app("De otra app");
    rig.disk.put(&vault_path(), &theirs);
    let Ok(Status::Open(open)) = rig.session.reload() else {
        panic!("reload failed");
    };
    assert_eq!((open.save, open.save_pending), (SaveState::Saved, false));
    assert_eq!(rig.next_save(), SaveState::Saved);
    assert_eq!(
        rig.titles().unwrap(),
        ["De otra app"],
        "the key was not asked again"
    );
    assert_eq!(rig.file(&vault_path()), theirs);

    rig.add_and_expect("Banco", SaveState::Saved);
    assert_eq!(
        titles_in(&rig.file(&vault_path())),
        ["Banco", "De otra app"]
    );
    // Nothing was pending at the lock: the dropped change is not "recovered".
    assert_eq!(rig.session.lock(), None);
}

// ---- Lock ----

#[test]
fn rf_30_lock_with_the_disk_hung_in_a_save_is_locked_in_less_than_1_s() {
    let rig = rig();
    rig.open();
    let original = rig.file(&vault_path());
    let (entered, release) = hang_at(&rig.disk, WRITE_TMP_OP);
    rig.add("GitHub").unwrap();
    assert_eq!(rig.next_save(), SaveState::Saving);
    entered.recv_timeout(GUARD).unwrap();
    let (live, secrets) = rig.probes();

    let started = Instant::now();
    let outcome = rig.session.lock();
    assert_eq!(rig.session.status(), locked());
    rig.assert_every_command_is_blocked();
    assert!(started.elapsed() < Duration::from_secs(1), "RF-30: ≤ 1 s");

    assert!(
        live.upgrade().is_none() && secrets.upgrade().is_none(),
        "rule 7: the hung save holds neither the vault nor the key"
    );
    // RF-33: what was pending is in the recovery file, which opens with the key.
    assert_eq!(
        outcome,
        Some(Recovery::Saved {
            path: recovery_path()
        })
    );
    assert_eq!(titles_in(&rig.file(&recovery_path())), ["GitHub"]);
    // The save of that session is still in the hands of the disk: no second writer on the same file.
    assert_eq!(rig.unlock(), Err(CoreError::Ocupada));
    assert_eq!(rig.session.status(), locked());

    // The disk comes back: the session is still locked and the vault file was not touched.
    release.send(()).unwrap();
    rig.settle();
    assert_eq!(rig.session.status(), locked());
    rig.assert_every_command_is_blocked();
    assert_eq!(rig.file(&vault_path()), original);
    assert_eq!(rig.disk.paths(), [vault_path(), recovery_path()]);
    assert!(
        hold(&rig.saves).try_recv().is_err(),
        "no save state after a lock"
    );

    // RF-34: the next unlock tells where the changes are, once.
    assert_eq!(
        rig.open().recovery,
        Some(Recovery::Saved {
            path: recovery_path()
        })
    );
    assert_eq!(rig.open_status().recovery, None);
    assert_eq!(rig.session.lock(), None);
    assert_eq!(rig.open().recovery, None);
}

#[test]
fn rf_33_lock_with_a_save_that_failed_writes_a_recovery_file_and_replaces_none() {
    let rig = rig();
    rig.open();
    let original = rig.file(&vault_path());
    rig.add_unsaved("GitHub");

    let first = rig.session.lock();

    assert_eq!(rig.session.status(), locked());
    assert_eq!(
        first,
        Some(Recovery::Saved {
            path: recovery_path()
        })
    );
    let recovered = rig.file(&recovery_path());
    assert_eq!(titles_in(&recovered), ["GitHub"], "it opens with the key");
    assert_eq!(rig.file(&vault_path()), original);

    // The same second, another lock with unsaved changes: the first recovery file stays.
    assert_eq!(rig.open().recovery, first);
    rig.add_unsaved("Banco");
    let second = sibling(".recuperacion-20261111-083005-2.kdbx");
    assert_eq!(
        rig.session.lock(),
        Some(Recovery::Saved {
            path: second.clone()
        })
    );
    assert_eq!(rig.file(&recovery_path()), recovered);
    assert_eq!(titles_in(&rig.file(&second)), ["Banco"]);
}

#[test]
fn rf_34_changes_that_could_not_be_recovered_are_told_as_lost_once() {
    let rig = rig();
    rig.open();
    rig.add_unsaved("GitHub");
    rig.disk
        .fault_at(0, Fault::Fail(ErrorKind::PermissionDenied)); // writing the recovery file

    assert_eq!(rig.session.lock(), Some(Recovery::Lost));

    assert_eq!(rig.session.status(), locked());
    assert_eq!(rig.disk.paths(), [vault_path()], "no recovery file");
    // A failed unlock and a close do not use up the note.
    assert_eq!(
        rig.session.unlock(&vault_path(), None, None),
        Err(CoreError::LlaveVacia)
    );
    rig.session.close();
    assert_eq!(rig.open().recovery, Some(Recovery::Lost));
    assert_eq!(rig.open_status().recovery, None);
    assert!(rig.titles().unwrap().is_empty());
}

#[test]
fn rf_33_a_recovery_file_that_takes_more_than_5_s_counts_as_lost() {
    let rig = rig();
    rig.open();
    rig.add_unsaved("GitHub");
    let (live, secrets) = rig.probes();
    let (entered, release) = hang_at(&rig.disk, 0); // writing the recovery file

    thread::scope(|scope| {
        let lock = scope.spawn(|| rig.session.lock());
        entered.recv_timeout(GUARD).unwrap();

        // While the recovery file is being written the vault is already locked and released (D12).
        assert_eq!(rig.session.status(), locked());
        rig.assert_every_command_is_blocked();
        assert!(live.upgrade().is_none() && secrets.upgrade().is_none());
        assert_eq!(
            rig.unlock(),
            Err(CoreError::Ocupada),
            "the lock has not finished"
        );

        rig.clock.advance(RECOVERY_LIMIT - Duration::from_millis(1));
        assert!(!lock.is_finished(), "the limit has not passed");
        rig.clock.advance(Duration::from_millis(1));
        assert_eq!(lock.join().unwrap(), Some(Recovery::Lost));
    });

    assert_eq!(rig.session.status(), locked());
    assert_eq!(rig.open().recovery, Some(Recovery::Lost));
    drop(release);
}

#[test]
fn rf_36_close_does_what_lock_does_and_forgets_the_file() {
    let rig = rig();
    rig.open();
    rig.add_unsaved("GitHub");
    let (live, secrets) = rig.probes();

    let outcome = rig.session.close();

    assert_eq!(rig.session.status(), Status::Closed);
    rig.assert_every_command_is_blocked();
    assert_eq!(rig.events.locked.load(Ordering::SeqCst), 1);
    assert!(live.upgrade().is_none() && secrets.upgrade().is_none());
    assert_eq!(
        outcome,
        Some(Recovery::Saved {
            path: recovery_path()
        })
    );
    assert_eq!(titles_in(&rig.file(&recovery_path())), ["GitHub"]);
    // RF-34 also after a close.
    assert_eq!(rig.open().recovery, outcome);
    assert_eq!(rig.session.close(), None);
    assert_eq!(rig.session.status(), Status::Closed);
}

#[test]
fn rf_30_the_steps_of_a_lock_run_in_order() {
    let rig = rig();
    rig.open();
    rig.add_unsaved("GitHub");
    let (_, secrets) = rig.probes();

    // What the listener sees when it is told (step 2).
    let (seen_tx, seen) = mpsc::channel();
    let (session, disk, key_alive) = (
        Arc::downgrade(&rig.session),
        rig.disk.clone(),
        secrets.clone(),
    );
    *hold(&rig.events.on_locked) = Some(Box::new(move || {
        let session = session.upgrade().unwrap();
        _ = seen_tx.send((
            session.status(),
            session.read(|vault| Ok(vault.entry_count())),
            disk.paths(),
            key_alive.upgrade().is_some(),
        ));
    }));

    let outcome = rig.session.lock();

    let (status, read, paths, key_was_alive) = seen.recv_timeout(GUARD).unwrap();
    assert_eq!(status, locked(), "step 1 comes before the listener");
    assert_eq!(
        read,
        Err(CoreError::Bloqueada),
        "step 1: commands are refused"
    );
    assert_eq!(
        paths,
        [vault_path()],
        "step 5 comes after: no recovery file yet"
    );
    assert!(key_was_alive, "step 4 comes after the listener");
    // Steps 3 to 6 are done when the lock returns.
    assert!(secrets.upgrade().is_none());
    assert_eq!(
        outcome,
        Some(Recovery::Saved {
            path: recovery_path()
        })
    );
    *hold(&rig.events.on_locked) = None;
}

#[test]
fn rf_30_a_command_in_flight_answers_bloqueada_and_then_the_secrets_are_dropped() {
    let rig = rig();
    rig.open();
    let (live, secrets) = rig.probes();
    let (told_tx, told) = mpsc::channel();
    *hold(&rig.events.on_locked) = Some(Box::new(move || {
        _ = told_tx.send(());
    }));
    let (inside_tx, inside) = mpsc::channel();
    let (finish, may_finish) = mpsc::channel::<()>();
    let may_finish = Mutex::new(may_finish);

    thread::scope(|scope| {
        // A command that is slow inside the vault, as one that is encrypting a change.
        let slow = scope.spawn(|| {
            rig.session.read(|vault| {
                inside_tx.send(()).unwrap();
                hold(&may_finish).recv_timeout(GUARD).unwrap();
                Ok(vault.entry_count())
            })
        });
        inside.recv_timeout(GUARD).unwrap();
        // A second command waits its turn behind it.
        let waiting = scope.spawn(|| rig.titles());

        let lock = scope.spawn(|| rig.session.lock());
        told.recv_timeout(GUARD).unwrap();

        // Asked to lock: every command answers at once, without waiting for the slow one.
        let started = Instant::now();
        assert_eq!(rig.session.status(), locked());
        rig.assert_every_command_is_blocked();
        assert_eq!(waiting.join().unwrap(), Err(CoreError::Bloqueada));
        assert!(started.elapsed() < Duration::from_secs(1), "RF-30: ≤ 1 s");
        // The limit: the command in flight still has the vault and the key until it ends.
        assert!(live.upgrade().is_some() && secrets.upgrade().is_some());

        finish.send(()).unwrap();
        // RF-90: what ends after the lock was asked for answers `Bloqueada`, whatever it read.
        assert_eq!(slow.join().unwrap(), Err(CoreError::Bloqueada));
        assert_eq!(lock.join().unwrap(), None);
    });

    assert!(live.upgrade().is_none() && secrets.upgrade().is_none());
    assert_eq!(rig.session.status(), locked());
    *hold(&rig.events.on_locked) = None;
}

#[test]
fn rf_30_a_save_already_verified_that_ends_after_the_lock_writes_only_encrypted_bytes() {
    let rig = rig();
    rig.open();
    let original = rig.file(&vault_path());
    let (entered, release) = hang_at(&rig.disk, REPLACE_OP); // verified, about to replace the vault
    rig.add("GitHub").unwrap();
    assert_eq!(rig.next_save(), SaveState::Saving);
    entered.recv_timeout(GUARD).unwrap();
    let (live, secrets) = rig.probes();

    let started = Instant::now();
    let outcome = rig.session.lock();
    assert_eq!(rig.session.status(), locked());
    assert!(started.elapsed() < Duration::from_secs(1), "RF-30: ≤ 1 s");
    assert!(live.upgrade().is_none() && secrets.upgrade().is_none());
    assert_eq!(rig.file(&vault_path()), original);
    assert_eq!(
        outcome,
        Some(Recovery::Saved {
            path: recovery_path()
        })
    );

    // The disk comes back and the save thread, which has no key, finishes what it had verified. The next
    // operation of the disk is the promotion of the backup: when it is reached, the vault was replaced.
    let (replaced, finish) = hang_at(&rig.disk, 0);
    release.send(()).unwrap();
    replaced.recv_timeout(GUARD).unwrap();
    assert_eq!(rig.session.status(), locked());
    rig.assert_every_command_is_blocked();
    let written = rig.file(&vault_path());
    assert_eq!(titles_in(&written), ["GitHub"], "it opens with the key");
    assert_eq!(
        written,
        rig.file(&recovery_path()),
        "the same encrypted bytes"
    );
    finish.send(()).unwrap();
    rig.settle();
    assert_eq!(
        rig.disk.paths(),
        [vault_path(), sibling(".bak"), recovery_path()]
    );
    assert_eq!(rig.file(&sibling(".bak")), original);

    // The next session starts from that file and tells about the recovery copy.
    assert_eq!(rig.open().recovery, outcome);
    assert_eq!(rig.titles().unwrap(), ["GitHub"]);
    rig.add_and_expect("Banco", SaveState::Saved);
}

#[test]
fn rf_30_what_encrypting_costs_is_measured() {
    // Not pass/fail on time (spec, "Cómo se prueba", row 2): the numbers are printed and go to the task report.
    // `cargo nextest run -p almarioks-core what_encrypting_costs --no-capture`
    let key = key();
    let file = new_vault(); // Argon2id, 64 MiB, 2 iterations
    let started = Instant::now();
    let vault = kdbx::open(&file, &key).unwrap();
    let unlock = started.elapsed();
    let started = Instant::now();
    let encrypted = vault.to_bytes(&key).unwrap();
    let encrypt = started.elapsed();
    println!(
        "64 MiB, 2 iterations: unlock (derive + decrypt) {unlock:?}; to_bytes (derive + encrypt) {encrypt:?}"
    );
    // Why it costs a derivation: every save has its own salt, so the key is derived again.
    assert_ne!(encrypted, file);
    assert!(kdbx::open(&encrypted, &key).is_ok());
}
