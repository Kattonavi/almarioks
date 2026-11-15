//! Save path against the disk in memory that fails, cuts or hangs at each step (spec, "Cómo se prueba", rows 23,
//! 27, 30 and 33). Vaults are real (`kdbx::create_vault`, 2 iterations) and `kdbx::open` is the verification, as
//! RF-21 asks. The real disk (Windows locks, file dates, example 6 with entries) is in `tests/rf_save.rs`.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this module).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
    sync::{Arc, mpsc},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use super::{
    Disk, FailureCause, Fingerprint, SaveError, SaveQueue, Timestamp, Verifier, read_vault,
    save_vault, save_vault_as,
    test_disk::{Fault, MemDisk},
    write_recovery,
};
use crate::{
    error::{CoreError, CoreResult},
    kdbx::{self, MasterPassword, NewVaultKdf, VaultKey},
};

/// Operations of one save: read the vault, write the temporary file, read it back, copy to `.bak.tmp`, clear its
/// read-only attribute, replace the vault, promote `.bak.tmp`.
const SAVE_OPS: usize = 7;
/// Index of the replace of the vault among [`SAVE_OPS`].
const REPLACE_OP: usize = 5;
/// No wait in these tests is a "wait for it to pass": this only bounds a channel so a bug fails instead of hanging.
const GUARD: Duration = Duration::from_secs(30);
/// Master password of the test vaults (fake data).
const PASSWORD: &str = "guardar-Prueba-2026";

fn vault_path() -> PathBuf {
    PathBuf::from("bovedas").join("cuentas.kdbx")
}

fn sibling(suffix: &str) -> PathBuf {
    PathBuf::from(format!("{}{suffix}", vault_path().display()))
}

fn key(password: &str) -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(password.to_owned())), None).unwrap()
}

fn new_vault(key: &VaultKey) -> Vec<u8> {
    kdbx::create_vault(key, "Cuentas", NewVaultKdf::with_iterations(2).unwrap()).unwrap()
}

fn opens(bytes: &[u8], key: &VaultKey) -> CoreResult<()> {
    kdbx::open(bytes, key).map(drop)
}

/// A disk with the vault `v1` and, optionally, an older `.bak`.
fn disk_with(v1: &[u8], bak: Option<&[u8]>) -> MemDisk {
    let disk = MemDisk::default();
    disk.put(&vault_path(), v1);
    if let Some(bak) = bak {
        disk.put(&sibling(".bak"), bak);
    }
    disk
}

fn paths(disk: &MemDisk) -> Vec<PathBuf> {
    disk.paths()
}

fn vault_and_bak() -> Vec<PathBuf> {
    vec![vault_path(), sibling(".bak")]
}

struct Versions {
    key: VaultKey,
    v0: Vec<u8>,
    v1: Vec<u8>,
    v2: Vec<u8>,
}

/// Three different vaults with the same key: every `create_vault` has its own salt and seeds.
fn versions() -> Versions {
    let key = key(PASSWORD);
    let (v0, v1, v2) = (new_vault(&key), new_vault(&key), new_vault(&key));
    assert!(v0 != v1 && v1 != v2);
    Versions { key, v0, v1, v2 }
}

#[test]
fn rf_21_save_writes_a_verified_temporary_file_and_replaces_the_vault() {
    let v = versions();
    let disk = disk_with(&v.v1, None);
    let verify = |bytes: &[u8]| opens(bytes, &v.key);

    let saved = save_vault(&disk, &vault_path(), &v.v2, Fingerprint::of(&v.v1), &verify).unwrap();

    assert_eq!(saved.fingerprint, Fingerprint::of(&v.v2));
    assert!(saved.backup_updated);
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v2);
    assert_eq!(disk.get(&sibling(".bak")).unwrap(), v.v1);
    assert_eq!(paths(&disk), vault_and_bak(), "no temporary file is left");
    assert_eq!(disk.op_count(), SAVE_OPS);
}

#[test]
fn rf_21_bytes_that_do_not_decrypt_with_the_key_are_not_written() {
    let v = versions();
    let disk = disk_with(&v.v1, None);
    let other_key = key("otra-clave");
    let verify = |bytes: &[u8]| opens(bytes, &other_key);

    let error =
        save_vault(&disk, &vault_path(), &v.v2, Fingerprint::of(&v.v1), &verify).unwrap_err();

    assert_eq!(error, SaveError::Failed(FailureCause::Verification));
    assert_eq!(error.code(), CoreError::GuardadoFallido);
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v1);
    assert_eq!(paths(&disk), vec![vault_path()]);
}

#[test]
fn rf_22_one_backup_replaced_on_every_save() {
    let v = versions();
    let disk = disk_with(&v.v0, None);
    let verify = |bytes: &[u8]| opens(bytes, &v.key);

    let first = save_vault(&disk, &vault_path(), &v.v1, Fingerprint::of(&v.v0), &verify).unwrap();
    save_vault(&disk, &vault_path(), &v.v2, first.fingerprint, &verify).unwrap();

    assert_eq!(disk.get(&vault_path()).unwrap(), v.v2);
    assert_eq!(disk.get(&sibling(".bak")).unwrap(), v.v1);
    assert_eq!(paths(&disk), vault_and_bak());
}

#[test]
fn rf_23_a_failure_at_any_step_leaves_the_vault_and_the_old_backup_intact() {
    let v = versions();
    let verify = |bytes: &[u8]| opens(bytes, &v.key);
    for step in 0..SAVE_OPS {
        let disk = disk_with(&v.v1, Some(&v.v0));
        disk.fault_at(step, Fault::Fail(ErrorKind::Other));

        let result = save_vault(&disk, &vault_path(), &v.v2, Fingerprint::of(&v.v1), &verify);

        if step <= REPLACE_OP {
            assert_eq!(
                result,
                Err(SaveError::Failed(FailureCause::Io)),
                "step {step}"
            );
            assert_eq!(disk.get(&vault_path()).unwrap(), v.v1, "step {step}");
            opens(&disk.get(&vault_path()).unwrap(), &v.key).unwrap();
        } else {
            // The vault was already replaced: the save is done and only the backup was not promoted.
            let saved = result.unwrap();
            assert!(!saved.backup_updated, "step {step}");
            assert_eq!(disk.get(&vault_path()).unwrap(), v.v2, "step {step}");
        }
        assert_eq!(disk.get(&sibling(".bak")).unwrap(), v.v0, "step {step}");
        assert_eq!(
            paths(&disk),
            vault_and_bak(),
            "step {step}: temporary files removed"
        );
    }
}

#[test]
fn rf_23_permission_denied_is_told_apart_and_a_retry_works() {
    let v = versions();
    let verify = |bytes: &[u8]| opens(bytes, &v.key);
    let disk = disk_with(&v.v1, Some(&v.v0));
    // Lab P3: a read-only destination makes the replace fail with PermissionDenied.
    disk.set_read_only(&vault_path(), true);

    let error =
        save_vault(&disk, &vault_path(), &v.v2, Fingerprint::of(&v.v1), &verify).unwrap_err();

    assert!(error.is_permission_denied());
    assert_eq!(CoreError::from(error), CoreError::GuardadoFallido);
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v1);
    assert_eq!(disk.get(&sibling(".bak")).unwrap(), v.v0);
    assert_eq!(paths(&disk), vault_and_bak());

    disk.set_read_only(&vault_path(), false);
    save_vault(&disk, &vault_path(), &v.v2, Fingerprint::of(&v.v1), &verify).unwrap();
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v2);
    assert_eq!(disk.get(&sibling(".bak")).unwrap(), v.v1);
}

#[test]
fn rf_27_a_cut_at_any_step_leaves_a_vault_that_opens_and_the_next_save_works() {
    let v = versions();
    let verify = |bytes: &[u8]| opens(bytes, &v.key);
    for step in 0..SAVE_OPS {
        let disk = disk_with(&v.v1, Some(&v.v0));
        disk.fault_at(step, Fault::Cut);

        // The process dies at this step: nobody sees its answer.
        _ = save_vault(&disk, &vault_path(), &v.v2, Fingerprint::of(&v.v1), &verify);

        // A new process: the vault opens with its key and is one of the two versions, whole.
        disk.revive();
        let (on_disk, fingerprint) = read_vault(&disk, &vault_path()).unwrap();
        opens(&on_disk, &v.key).unwrap();
        let expected = if step <= REPLACE_OP { &v.v1 } else { &v.v2 };
        assert_eq!(&on_disk, expected, "step {step}");
        assert_eq!(disk.get(&sibling(".bak")).unwrap(), v.v0, "step {step}");

        // Leftovers of the cut save do not get in the way of the next one.
        save_vault(&disk, &vault_path(), &v.v0, fingerprint, &verify).unwrap();
        assert_eq!(disk.get(&vault_path()).unwrap(), v.v0, "step {step}");
        assert_eq!(
            disk.get(&sibling(".bak")).unwrap(),
            *expected,
            "step {step}"
        );
        assert_eq!(paths(&disk), vault_and_bak(), "step {step}");
    }
}

#[test]
fn rf_24_a_content_change_by_another_app_is_not_overwritten() {
    let v = versions();
    let verify = |bytes: &[u8]| opens(bytes, &v.key);
    let disk = disk_with(&v.v1, Some(&v.v0));
    disk.put(&vault_path(), &v.v2); // the sync app rewrote it

    let error =
        save_vault(&disk, &vault_path(), &v.v0, Fingerprint::of(&v.v1), &verify).unwrap_err();

    assert_eq!(error, SaveError::ExternalChange);
    assert_eq!(error.code(), CoreError::CambioExterno);
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v2);
    assert_eq!(disk.get(&sibling(".bak")).unwrap(), v.v0);
    assert_eq!(disk.op_count(), 1, "only the vault was read");
}

#[test]
fn rf_24_the_same_content_written_again_is_not_a_change() {
    let v = versions();
    let verify = |bytes: &[u8]| opens(bytes, &v.key);
    let disk = disk_with(&v.v1, None);
    disk.put(&vault_path(), &v.v1); // rewritten with the same bytes: only its date would change

    save_vault(&disk, &vault_path(), &v.v2, Fingerprint::of(&v.v1), &verify).unwrap();
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v2);
}

#[test]
fn rf_24_a_vault_deleted_by_another_app_is_not_recreated() {
    let v = versions();
    let verify = |bytes: &[u8]| opens(bytes, &v.key);
    let disk = MemDisk::default();

    let error =
        save_vault(&disk, &vault_path(), &v.v2, Fingerprint::of(&v.v1), &verify).unwrap_err();

    assert_eq!(error, SaveError::ExternalChange);
    assert!(paths(&disk).is_empty());
}

#[test]
fn rf_95_reading_a_missing_vault_is_archivo_no_encontrado() {
    let v = versions();
    let disk = disk_with(&v.v1, None);
    let (bytes, fingerprint) = read_vault(&disk, &vault_path()).unwrap();
    assert_eq!(bytes, v.v1);
    assert_eq!(fingerprint, Fingerprint::of(&v.v1));
    assert_eq!(
        read_vault(&disk, Path::new("otra.kdbx")).unwrap_err(),
        CoreError::ArchivoNoEncontrado
    );
}

#[test]
fn rf_83_the_storage_codes_have_a_spanish_message_without_the_code() {
    for error in [
        CoreError::ArchivoNoEncontrado,
        CoreError::ArchivoLlaveNoEncontrado,
    ] {
        assert_eq!(error.code(), format!("{error:?}"));
        assert!(!error.message().is_empty());
        assert!(!error.message().contains(error.code()));
    }
}

#[test]
fn rf_23_save_as_writes_a_new_file_and_never_replaces_one() {
    let v = versions();
    let verify = |bytes: &[u8]| opens(bytes, &v.key);
    let disk = disk_with(&v.v1, None);
    let elsewhere = PathBuf::from("bovedas").join("cuentas-copia.kdbx");

    let fingerprint = save_vault_as(&disk, &elsewhere, &v.v2, &verify).unwrap();
    assert_eq!(fingerprint, Fingerprint::of(&v.v2));
    assert_eq!(disk.get(&elsewhere).unwrap(), v.v2);
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v1);

    let error = save_vault_as(&disk, &vault_path(), &v.v2, &verify).unwrap_err();
    assert_eq!(error, SaveError::AlreadyExists);
    assert_eq!(error.code(), CoreError::ArchivoExiste);
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v1);
    assert_eq!(paths(&disk), vec![elsewhere, vault_path()]);
}

#[test]
fn rf_23_save_as_that_fails_leaves_nothing() {
    let v = versions();
    let verify = |bytes: &[u8]| opens(bytes, &v.key);
    let disk = MemDisk::default();
    let elsewhere = PathBuf::from("copia.kdbx");
    disk.fault_at(2, Fault::Fail(ErrorKind::StorageFull));

    let error = save_vault_as(&disk, &elsewhere, &v.v2, &verify).unwrap_err();
    assert_eq!(error, SaveError::Failed(FailureCause::Io));
    assert!(paths(&disk).is_empty());
}

#[test]
fn rf_27_a_save_hung_before_the_replace_leaves_the_vault_readable_and_finishes_when_released() {
    let v = versions();
    let disk = disk_with(&v.v1, None);
    let (entered_tx, entered) = std::sync::mpsc::sync_channel(1);
    let (release, release_rx) = std::sync::mpsc::sync_channel(1);
    disk.fault_at(
        REPLACE_OP,
        Fault::Hang {
            entered: entered_tx,
            release: release_rx,
        },
    );
    std::thread::scope(|scope| {
        let saving = scope.spawn(|| {
            let verify = |bytes: &[u8]| opens(bytes, &v.key);
            save_vault(&disk, &vault_path(), &v.v2, Fingerprint::of(&v.v1), &verify)
        });
        entered
            .recv_timeout(std::time::Duration::from_secs(30))
            .unwrap();
        // While the disk hangs, the vault is still the old one and whole.
        assert_eq!(disk.get(&vault_path()).unwrap(), v.v1);
        release.send(()).unwrap();
        assert!(saving.join().unwrap().is_ok());
    });
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v2);
    assert_eq!(paths(&disk), vault_and_bak());
}

// ---- Recovery file (RF-33) ----

#[test]
fn rf_33_recovery_file_is_named_by_the_clock_and_never_replaces_another() {
    let v = versions();
    let disk = disk_with(&v.v0, None);
    let now = Timestamp::new(2026, 11, 11, 8, 30, 5).unwrap();

    let first = write_recovery(&disk, &vault_path(), &v.v1, now).unwrap();
    let second = write_recovery(&disk, &vault_path(), &v.v2, now).unwrap();

    assert_eq!(first, sibling(".recuperacion-20261111-083005.kdbx"));
    assert_eq!(second, sibling(".recuperacion-20261111-083005-2.kdbx"));
    assert_eq!(
        disk.get(&first).unwrap(),
        v.v1,
        "the first one is not replaced"
    );
    opens(&disk.get(&second).unwrap(), &v.key).unwrap();
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v0);
}

#[test]
fn rf_33_recovery_that_cannot_be_written_is_guardado_fallido() {
    let v = versions();
    let disk = MemDisk::default();
    disk.fault_at(0, Fault::Fail(ErrorKind::StorageFull));
    let now = Timestamp::new(2026, 11, 11, 8, 30, 5).unwrap();
    assert_eq!(
        write_recovery(&disk, &vault_path(), &v.v1, now).unwrap_err(),
        CoreError::GuardadoFallido
    );
    assert!(paths(&disk).is_empty());
}

#[test]
fn rf_33_the_clock_gives_aaaammdd_hhmmss() {
    let at = |seconds| Timestamp::from_system_time_utc(UNIX_EPOCH + Duration::from_secs(seconds));
    assert_eq!(at(0).unwrap().to_string(), "19700101-000000");
    assert_eq!(at(1_794_385_805).unwrap().to_string(), "20261111-083005");
    assert_eq!(at(1_709_251_199).unwrap().to_string(), "20240229-235959");
    assert!(Timestamp::new(2025, 2, 29, 0, 0, 0).is_err());
    assert!(Timestamp::new(2026, 13, 1, 0, 0, 0).is_err());
    assert!(Timestamp::new(2026, 1, 1, 24, 0, 0).is_err());
    assert!(Timestamp::from_system_time_utc(SystemTime::now()).is_ok());
}

// ---- Save queue (RF-89, RF-30) ----

/// Its own copy of the key: the queue must be able to drop it (`VaultKey` is not `Clone` on purpose).
fn verifier() -> Verifier {
    let key = key(PASSWORD);
    Arc::new(move |bytes: &[u8]| opens(bytes, &key))
}

fn queue(disk: &Arc<MemDisk>, confirmed: &[u8], verifier: Verifier) -> SaveQueue {
    let disk: Arc<dyn Disk> = Arc::clone(disk) as Arc<dyn Disk>;
    SaveQueue::start(disk, vault_path(), Fingerprint::of(confirmed), verifier).unwrap()
}

/// Arms a hang at the `nth` operation; returns (entered, release).
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

#[test]
fn rf_89_saves_run_one_after_another_and_the_file_ends_with_the_last_state() {
    let v = versions();
    let disk = Arc::new(disk_with(&v.v0, None));
    let queue = queue(&disk, &v.v0, verifier());

    queue.submit(v.v1.clone()).wait().unwrap();
    let last = queue.submit(v.v2.clone()).wait().unwrap();

    assert_eq!(last.fingerprint, Fingerprint::of(&v.v2));
    assert_eq!(queue.confirmed(), Fingerprint::of(&v.v2));
    assert!(queue.unconfirmed().is_none());
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v2);
    assert_eq!(disk.get(&sibling(".bak")).unwrap(), v.v1);
    queue.shutdown().unwrap();
}

#[test]
fn rf_89_saves_that_arrive_during_a_save_wait_their_turn_and_none_is_lost() {
    let v = versions();
    let third = new_vault(&v.key);
    let disk = Arc::new(disk_with(&v.v0, None));
    let (entered, release) = hang_at(&disk, 1); // writing the temporary file of the first save
    let queue = queue(&disk, &v.v0, verifier());

    let first = queue.submit(v.v1.clone());
    entered.recv_timeout(GUARD).unwrap();
    let second = queue.submit(v.v2.clone());
    let newest = queue.submit(third.clone());
    assert_eq!(queue.unconfirmed().unwrap().as_slice(), third.as_slice());
    release.send(()).unwrap();

    assert_eq!(first.wait().unwrap().fingerprint, Fingerprint::of(&v.v1));
    // The newest state already holds the second one: both tickets get the result of that write.
    assert_eq!(second.wait().unwrap().fingerprint, Fingerprint::of(&third));
    assert_eq!(newest.wait().unwrap().fingerprint, Fingerprint::of(&third));
    assert_eq!(disk.get(&vault_path()).unwrap(), third);
    assert_eq!(disk.get(&sibling(".bak")).unwrap(), v.v1);
    assert!(queue.unconfirmed().is_none());
    queue.shutdown().unwrap();
}

#[test]
fn rf_30_with_the_disk_hung_the_unconfirmed_bytes_come_out_at_once_and_the_key_is_released() {
    let v = versions();
    let disk = Arc::new(disk_with(&v.v0, None));
    let (entered, release) = hang_at(&disk, 1);
    let verify = verifier();
    let held = Arc::downgrade(&verify);
    let queue = queue(&disk, &v.v0, verify);

    let hung = queue.submit(v.v1.clone());
    entered.recv_timeout(GUARD).unwrap();
    let waiting = queue.submit(v.v2.clone());

    let started = Instant::now();
    assert_eq!(queue.unconfirmed().unwrap().as_slice(), v.v2.as_slice());
    let for_recovery = queue.close().unwrap();
    assert!(started.elapsed() < Duration::from_secs(1), "RF-30: ≤ 1 s");
    assert_eq!(for_recovery.as_slice(), v.v2.as_slice());
    assert!(
        held.upgrade().is_none(),
        "rule 7 / D12: the key is not held by the hung save"
    );
    assert_eq!(waiting.wait_timeout(GUARD), Some(Err(SaveError::Closed)));
    assert_eq!(SaveError::Closed.code(), CoreError::Bloqueada);

    // The disk comes back: the hung save cannot verify any more and the vault stays as it was.
    release.send(()).unwrap();
    assert_eq!(
        hung.wait(),
        Err(SaveError::Failed(FailureCause::Verification))
    );
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v0);
    queue.shutdown().unwrap();

    // RF-33: the copy already encrypted goes to the recovery file, and it opens with the key.
    let now = Timestamp::new(2026, 11, 11, 8, 30, 5).unwrap();
    let recovery = write_recovery(disk.as_ref(), &vault_path(), &for_recovery, now).unwrap();
    opens(&disk.get(&recovery).unwrap(), &v.key).unwrap();
}

#[test]
fn rf_23_a_failed_save_keeps_its_bytes_unconfirmed_and_a_retry_writes_them() {
    let v = versions();
    let disk = Arc::new(disk_with(&v.v0, None));
    disk.fault_at(1, Fault::Fail(ErrorKind::StorageFull));
    let queue = queue(&disk, &v.v0, verifier());

    assert_eq!(
        queue.submit(v.v1.clone()).wait(),
        Err(SaveError::Failed(FailureCause::Io))
    );
    assert_eq!(queue.unconfirmed().unwrap().as_slice(), v.v1.as_slice());
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v0);

    queue.submit(v.v1.clone()).wait().unwrap();
    assert!(queue.unconfirmed().is_none());
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v1);
    queue.shutdown().unwrap();
}

#[test]
fn rf_24_after_an_external_change_the_queue_stops_writing() {
    let v = versions();
    let disk = Arc::new(disk_with(&v.v0, None));
    let queue = queue(&disk, &v.v0, verifier());
    disk.put(&vault_path(), &v.v2); // another app

    assert_eq!(
        queue.submit(v.v1.clone()).wait(),
        Err(SaveError::ExternalChange)
    );
    let ops = disk.op_count();
    assert_eq!(
        queue.submit(v.v1.clone()).wait(),
        Err(SaveError::ExternalChange)
    );
    assert_eq!(
        disk.op_count(),
        ops,
        "the second one does not touch the disk"
    );
    assert_eq!(queue.unconfirmed().unwrap().as_slice(), v.v1.as_slice());
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v2);
    queue.shutdown().unwrap();
}

#[test]
fn rf_30_a_closed_queue_answers_bloqueada() {
    let v = versions();
    let disk = Arc::new(disk_with(&v.v0, None));
    let queue = queue(&disk, &v.v0, verifier());
    assert!(queue.close().is_none(), "nothing was pending");
    assert_eq!(queue.submit(v.v1.clone()).wait(), Err(SaveError::Closed));
    assert_eq!(disk.get(&vault_path()).unwrap(), v.v0);
    queue.shutdown().unwrap();
}
