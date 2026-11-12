//! Independent validation from the spec: saving (RF-21 to RF-24, RF-27, RF-89; example 6).
#![allow(clippy::unwrap_used, clippy::expect_used)]

mod spec_support;

use std::{
    fs,
    path::Path,
    time::{Duration, SystemTime},
};

use almarioks_core::{
    CoreError, CoreResult,
    kdbx::{self, OpenedVault},
    session::{SaveState, Status},
    storage::{Fingerprint, SaveError, save_vault},
};
use keepass::{Database, DatabaseKey};
use spec_support::{
    Act, FaultDisk, PW, Rig, TempDir, WAIT, at, count, create_vault_at, key, new_entry, open_path,
    root_titles, status_open,
};

fn add_and_save(rig: &Rig, title: &str) {
    rig.add(title).expect("add");
    assert_eq!(rig.settle(), SaveState::Saved, "saving {title}");
}

/// What another KeePass app does: opens the file with the same key, adds an entry and writes it back.
fn edit_with_another_app(path: &Path, title: &str) {
    let bytes = fs::read(path).expect("read");
    let mut db =
        Database::parse(&bytes, DatabaseKey::new().with_password(PW)).expect("keepass opens it");
    {
        let mut root = db.root_mut();
        let mut entry = root.add_entry();
        entry.set_unprotected("Title", title);
    }
    let mut out = Vec::new();
    db.save(&mut out, DatabaseKey::new().with_password(PW))
        .expect("keepass saves it");
    fs::write(path, out).expect("write");
}

#[test]
fn spec_rf_22_example_6_the_bak_holds_the_previous_version_and_opens_with_the_same_key() {
    let rig = Rig::open("rf22-bak");
    for title in ["Uno", "Dos", "Tres"] {
        add_and_save(&rig, title);
    }
    assert_eq!(count(&rig.path), 3);
    let bak = rig.dir.join("cuentas.kdbx.bak");

    add_and_save(&rig, "Cuatro");
    assert_eq!(count(&rig.path), 4, "cuentas.kdbx has 4");
    assert_eq!(
        count(&bak),
        3,
        "cuentas.kdbx.bak opens with the same key and has 3"
    );

    add_and_save(&rig, "Cinco");
    assert_eq!(count(&rig.path), 5);
    assert_eq!(count(&bak), 4);
    let backups: Vec<_> = fs::read_dir(rig.dir.path())
        .expect("dir")
        .filter_map(Result::ok)
        .filter(|e| e.file_name().to_string_lossy().contains(".bak"))
        .collect();
    assert_eq!(backups.len(), 1, "RF-22: one single copy");
}

#[test]
fn spec_rf_89_a_change_made_while_a_save_hangs_waits_its_turn_and_both_end_in_the_file() {
    let rig = Rig::open("rf89-queue");
    let hung = rig.disk.watch();
    rig.disk.set_main(Act::Hang);
    rig.add("Primero").expect("first change");
    hung.recv_timeout(WAIT)
        .expect("first save hangs on the disk");
    rig.add("Segundo")
        .expect("second change answers while the first save is still on the disk");
    rig.disk.release();
    let deadline = std::time::Instant::now() + WAIT;
    loop {
        let state = rig.states_rx.recv_timeout(WAIT).expect("save state");
        let open = status_open(rig.session.status());
        if state == SaveState::Saved && !open.save_pending {
            break;
        }
        assert!(
            !matches!(state, SaveState::Failed(_) | SaveState::ExternalChange),
            "{state:?}"
        );
        assert!(std::time::Instant::now() < deadline, "saves never settled");
    }
    let mut titles = root_titles(&rig.path);
    titles.sort();
    assert_eq!(
        titles,
        vec!["Primero".to_owned(), "Segundo".to_owned()],
        "casos límite: the file has both"
    );
}

#[test]
fn spec_rf_24_a_file_rewritten_by_another_app_is_not_overwritten_and_save_as_keeps_the_change() {
    let rig = Rig::open("rf24-external");
    edit_with_another_app(&rig.path, "Ajena");
    let foreign = fs::read(&rig.path).expect("foreign bytes");
    rig.add("Mía").expect("add");
    assert_eq!(
        rig.settle(),
        SaveState::ExternalChange,
        "RF-24: CambioExterno"
    );
    assert_eq!(
        fs::read(&rig.path).expect("file"),
        foreign,
        "RF-24: the other app's file is intact"
    );

    let elsewhere = rig.dir.join("copia.kdbx");
    let status = rig
        .session
        .save_as(&elsewhere)
        .expect("RF-24: save to another path");
    assert!(matches!(status, Status::Open(_)));
    assert_eq!(root_titles(&elsewhere), vec!["Mía".to_owned()]);
    assert_eq!(
        fs::read(&rig.path).expect("file"),
        foreign,
        "still intact after save_as"
    );
}

#[test]
fn spec_rf_24_reload_reads_the_other_apps_file_and_drops_what_was_pending() {
    let rig = Rig::open("rf24-reload");
    edit_with_another_app(&rig.path, "Ajena");
    rig.add("Mía").expect("add");
    assert_eq!(rig.settle(), SaveState::ExternalChange);
    let open = status_open(rig.session.reload().expect("RF-24: reload"));
    assert!(!open.save_pending, "pending changes were dropped");
    assert_eq!(rig.titles(), Ok(vec!["Ajena".to_owned()]));
    // After the reload the next change is saved over the other app's version.
    rig.drain();
    rig.add("Después").expect("add");
    assert_eq!(rig.settle(), SaveState::Saved);
    let mut titles = root_titles(&rig.path);
    titles.sort();
    assert_eq!(titles, vec!["Ajena".to_owned(), "Después".to_owned()]);
}

#[test]
fn spec_rf_24_when_another_app_only_changes_the_date_the_vault_saves_normally() {
    let rig = Rig::open("rf24-date");
    let file = fs::OpenOptions::new()
        .write(true)
        .open(&rig.path)
        .expect("open for date");
    file.set_modified(SystemTime::UNIX_EPOCH + Duration::from_secs(1_000_000_000))
        .expect("set date");
    drop(file);
    rig.add("Con fecha cambiada").expect("add");
    assert_eq!(
        rig.settle(),
        SaveState::Saved,
        "casos límite: a date change is not a change"
    );
    assert_eq!(
        root_titles(&rig.path),
        vec!["Con fecha cambiada".to_owned()]
    );
}

#[test]
fn spec_rf_23_failed_save_keeps_the_change_in_memory_editable_and_retry_saves_it() {
    let rig = Rig::open("rf23-retry");
    rig.disk.set_main(Act::Fail);
    rig.add("Pendiente").expect("add");
    assert!(matches!(rig.settle(), SaveState::Failed(_)));
    rig.add("Otra más")
        .expect("RF-23: the vault stays editable");
    assert!(matches!(rig.settle(), SaveState::Failed(_)));
    assert_eq!(count(&rig.path), 0, "RF-23: the original is intact");
    let mut in_memory = rig.titles().expect("read");
    in_memory.sort();
    assert_eq!(
        in_memory,
        vec!["Otra más".to_owned(), "Pendiente".to_owned()]
    );
    assert!(status_open(rig.session.status()).save_pending);

    rig.disk.set_main(Act::Pass);
    rig.drain();
    rig.session.retry_save().expect("RF-23: retry");
    assert_eq!(rig.settle(), SaveState::Saved);
    assert_eq!(count(&rig.path), 2);
}

#[test]
fn spec_rf_23_save_as_never_replaces_an_existing_file_and_writes_a_new_one() {
    let rig = Rig::open("rf23-save-as");
    rig.disk.set_main(Act::Fail);
    rig.add("Pendiente").expect("add");
    assert!(matches!(rig.settle(), SaveState::Failed(_)));
    let taken = rig.dir.join("ocupado.kdbx");
    fs::write(&taken, b"no tocar").expect("write");
    assert_eq!(
        rig.session.save_as(&taken).map(|_| ()),
        Err(CoreError::ArchivoExiste)
    );
    assert_eq!(fs::read(&taken).expect("taken"), b"no tocar");

    rig.disk.set_main(Act::Pass);
    let fresh = rig.dir.join("nueva.kdbx");
    rig.session.save_as(&fresh).expect("save as");
    assert_eq!(root_titles(&fresh), vec!["Pendiente".to_owned()]);
}

// ----- Each step of the save, straight on `save_vault` (spec "Cómo se prueba": 23, 27). -----

struct Fixture {
    dir: TempDir,
    original: Vec<u8>,
    encrypted: Vec<u8>,
}

fn fixture(tag: &str) -> Fixture {
    let dir = TempDir::new(tag);
    let path = dir.join("cuentas.kdbx");
    let mut vault = create_vault_at(&path, "Cuentas");
    let root = vault.root_id();
    vault
        .create_entry(root, new_entry("Original"), at(1_794_391_200))
        .expect("entry");
    fs::write(&path, vault.to_bytes(&key(PW)).expect("bytes")).expect("write");
    let original = fs::read(&path).expect("read");
    let mut vault = open_path(&path).expect("open");
    let root = vault.root_id();
    vault
        .create_entry(root, new_entry("Nueva"), at(1_794_391_260))
        .expect("entry");
    let encrypted = vault.to_bytes(&key(PW)).expect("bytes");
    Fixture {
        dir,
        original,
        encrypted,
    }
}

fn verifier() -> impl Fn(&[u8]) -> CoreResult<()> {
    let key = key(PW);
    move |bytes: &[u8]| kdbx::open(bytes, &key).map(|_: OpenedVault| ())
}

fn save(fx: &Fixture, disk: &FaultDisk) -> Result<(), SaveError> {
    let path = fx.dir.join("cuentas.kdbx");
    save_vault(
        disk,
        &path,
        &fx.encrypted,
        Fingerprint::of(&fx.original),
        &verifier(),
    )
    .map(|_| ())
}

#[test]
fn spec_rf_23_a_failure_at_any_step_leaves_the_original_byte_for_byte() {
    let fx = fixture("rf23-steps");
    let path = fx.dir.join("cuentas.kdbx");
    let disk = FaultDisk::new();
    let mut failures = 0;
    for k in 1..=60 {
        fs::write(&path, &fx.original).expect("restore");
        disk.arm_step(k, Act::Fail, false);
        let result = save(&fx, &disk);
        if !disk.triggered() {
            assert_eq!(result, Ok(()), "with no fault the save works");
            assert_eq!(count(&path), 2);
            break;
        }
        match result {
            Err(SaveError::Failed(_)) => {
                failures += 1;
                assert_eq!(
                    fs::read(&path).expect("file"),
                    fx.original,
                    "RF-23: step {k} touched the original"
                );
            }
            Err(other) => panic!("RF-23: step {k} answered {other:?} instead of GuardadoFallido"),
            Ok(()) => assert!(
                open_path(&path).is_ok_and(|v| v.entry_count() == 2),
                "step {k}: said saved but the file does not have the change"
            ),
        }
    }
    assert!(
        failures >= 3,
        "the save has several steps that can fail (temp, verify, replace)"
    );
}

#[test]
fn spec_rf_27_if_the_process_dies_at_any_step_the_vault_file_still_opens_with_its_key() {
    let fx = fixture("rf27-crash");
    let path = fx.dir.join("cuentas.kdbx");
    let disk = FaultDisk::new();
    for k in 1..=60 {
        fs::write(&path, &fx.original).expect("restore");
        for leftover in fs::read_dir(fx.dir.path())
            .expect("dir")
            .filter_map(Result::ok)
        {
            if leftover.path() != path {
                let _ = fs::remove_file(leftover.path());
            }
        }
        // From step k on nothing reaches the disk: the process is gone (no cleanup either).
        disk.arm_step(k, Act::Fail, true);
        let _ = save(&fx, &disk);
        if !disk.triggered() {
            break;
        }
        let opened = open_path(&path);
        assert!(
            opened
                .as_ref()
                .is_ok_and(|v| v.entry_count() == 1 || v.entry_count() == 2),
            "RF-27: dying at step {k} left a vault file that does not open: {:?}",
            opened.map(|v| v.entry_count())
        );
    }
}

#[test]
fn spec_rf_21_a_temporary_file_that_does_not_read_back_as_written_is_never_put_in_place() {
    let fx = fixture("rf21-verify");
    let path = fx.dir.join("cuentas.kdbx");
    let disk = FaultDisk::new();
    disk.set_main(Act::Corrupt);
    let result = save(&fx, &disk);
    assert!(
        matches!(result, Err(SaveError::Failed(_))),
        "RF-21: {result:?}"
    );
    assert_eq!(
        fs::read(&path).expect("file"),
        fx.original,
        "RF-23: original intact"
    );
    assert_eq!(
        SaveError::Failed(almarioks_core::storage::FailureCause::Verification).code(),
        CoreError::GuardadoFallido
    );
}

#[test]
fn spec_rf_21_the_temporary_file_is_written_in_the_same_folder_as_the_vault() {
    let fx = fixture("rf21-folder");
    let disk = FaultDisk::new();
    save(&fx, &disk).expect("save");
    let writes: Vec<_> = disk
        .log()
        .into_iter()
        .filter(|(op, _)| matches!(op, spec_support::Op::Write | spec_support::Op::WriteNew))
        .collect();
    assert!(!writes.is_empty());
    for (_, written) in writes {
        assert_eq!(
            written.parent(),
            Some(fx.dir.path()),
            "RF-21: temporary file outside the vault's folder"
        );
        assert_ne!(
            written,
            fx.dir.join("cuentas.kdbx"),
            "RF-21: the vault itself is written in place"
        );
    }
}
