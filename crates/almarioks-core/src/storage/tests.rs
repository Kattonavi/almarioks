//! Save path against the disk in memory that fails, cuts or hangs at each step (spec, "Cómo se prueba", rows 23,
//! 27, 30 and 33). Vaults are real (`kdbx::create_vault`, 2 iterations) and `kdbx::open` is the verification, as
//! RF-21 asks. The real disk (Windows locks, file dates, example 6 with entries) is in `tests/rf_save.rs`.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this module).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    io::ErrorKind,
    path::{Path, PathBuf},
};

use super::{
    FailureCause, Fingerprint, SaveError, read_vault, save_vault, save_vault_as,
    test_disk::{Fault, MemDisk},
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
    kdbx::create_vault(key, NewVaultKdf::with_iterations(2).unwrap()).unwrap()
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
