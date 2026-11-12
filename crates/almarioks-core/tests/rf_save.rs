//! Saving with the real disk (RF-21, RF-22, RF-23, RF-24), in a fresh folder under the system temp dir, removed at
//! the end. The fault-by-fault tests (fail, cut and hang at each step) run against the disk in memory, in
//! `src/storage/tests.rs`; here are the cases only the file system can show (lab P3, Windows).
//!
//! The vaults with 3, 4 and 5 entries of example 6 are made with `keepass` directly, like `tests/roundtrip.rs`:
//! adding entries through the core is T6, not this task.

// Tests may panic on purpose: a failed unwrap is a failed test (R-2 exception, scoped to this file).
#![allow(clippy::unwrap_used, clippy::expect_used)]

use std::{
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicUsize, Ordering},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use almarioks_core::{
    CoreError,
    kdbx::{self, MasterPassword, NewVaultKdf, VaultKey},
    storage::{Fingerprint, SaveError, StdDisk, read_vault, save_vault},
};
use keepass::{Database, DatabaseKey};

const PASSWORD: &str = "Respaldo-de-prueba-2026";

struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Self {
        static COUNTER: AtomicUsize = AtomicUsize::new(0);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let name = format!(
            "almarioks-core-save-{}-{nanos}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        );
        let path = std::env::temp_dir().join(name);
        fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    fn join(&self, name: &str) -> PathBuf {
        self.0.join(name)
    }

    fn names(&self) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(&self.0)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        drop(fs::remove_dir_all(&self.0));
    }
}

fn key() -> VaultKey {
    VaultKey::new(Some(MasterPassword::new(PASSWORD.to_owned())), None).unwrap()
}

fn verify(bytes: &[u8]) -> Result<(), CoreError> {
    kdbx::open(bytes, &key()).map(drop)
}

fn new_vault() -> Vec<u8> {
    kdbx::create_vault(&key(), NewVaultKdf::with_iterations(2).unwrap()).unwrap()
}

/// `base` (made by the core) with `count` entries more, saved by the library with the same key and settings.
fn with_entries(base: &[u8], count: usize) -> Vec<u8> {
    let library_key = DatabaseKey::new().with_password(PASSWORD);
    let mut db = Database::parse(base, library_key.clone()).unwrap();
    for n in 0..count {
        db.root_mut()
            .add_entry()
            .set_unprotected("Title", format!("Cuenta {n}"));
    }
    let mut out = Vec::new();
    db.save(&mut out, library_key).unwrap();
    out
}

fn entries(path: &Path) -> usize {
    kdbx::open(&fs::read(path).unwrap(), &key())
        .unwrap()
        .entry_count()
}

fn bak(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.bak", path.display()))
}

#[test]
fn rf_22_example_6_the_backup_has_the_previous_version() {
    let dir = TempDir::new();
    let vault = dir.join("cuentas.kdbx");
    let base = new_vault();
    fs::write(&vault, with_entries(&base, 3)).unwrap();
    let (_, read) = read_vault(&StdDisk, &vault).unwrap();

    let four = save_vault(&StdDisk, &vault, &with_entries(&base, 4), read, &verify).unwrap();
    assert_eq!((entries(&vault), entries(&bak(&vault))), (4, 3));

    save_vault(
        &StdDisk,
        &vault,
        &with_entries(&base, 5),
        four.fingerprint,
        &verify,
    )
    .unwrap();
    assert_eq!((entries(&vault), entries(&bak(&vault))), (5, 4));
    assert_eq!(dir.names(), ["cuentas.kdbx", "cuentas.kdbx.bak"]);
}

#[test]
fn rf_24_only_the_content_counts_not_the_date() {
    let dir = TempDir::new();
    let vault = dir.join("cuentas.kdbx");
    let (v1, v2, other) = (new_vault(), new_vault(), new_vault());
    fs::write(&vault, &v1).unwrap();
    let (_, read) = read_vault(&StdDisk, &vault).unwrap();

    // Another app touches only the date: not a change.
    let a_day_ago = SystemTime::now() - Duration::from_secs(86_400);
    fs::File::options()
        .write(true)
        .open(&vault)
        .unwrap()
        .set_modified(a_day_ago)
        .unwrap();
    let saved = save_vault(&StdDisk, &vault, &v2, read, &verify).unwrap();

    // Another app rewrites the content: not overwritten.
    fs::write(&vault, &other).unwrap();
    let error = save_vault(&StdDisk, &vault, &v1, saved.fingerprint, &verify).unwrap_err();
    assert_eq!(error, SaveError::ExternalChange);
    assert_eq!(fs::read(&vault).unwrap(), other);
}

#[test]
fn rf_27_leftovers_of_a_cut_save_do_not_get_in_the_way() {
    let dir = TempDir::new();
    let vault = dir.join("cuentas.kdbx");
    let (v1, v2) = (new_vault(), new_vault());
    fs::write(&vault, &v1).unwrap();
    fs::write(dir.join("cuentas.kdbx.tmp"), b"half a vault").unwrap();
    fs::write(dir.join("cuentas.kdbx.bak.tmp"), b"half a copy").unwrap();

    save_vault(&StdDisk, &vault, &v2, Fingerprint::of(&v1), &verify).unwrap();

    assert_eq!(fs::read(&vault).unwrap(), v2);
    assert_eq!(fs::read(bak(&vault)).unwrap(), v1);
    assert_eq!(dir.names(), ["cuentas.kdbx", "cuentas.kdbx.bak"]);
}

#[test]
fn rf_95_a_missing_vault_is_archivo_no_encontrado() {
    let dir = TempDir::new();
    assert_eq!(
        read_vault(&StdDisk, &dir.join("no-existe.kdbx")).unwrap_err(),
        CoreError::ArchivoNoEncontrado
    );
}

#[cfg(windows)]
mod windows {
    //! Lab P3: what Windows does when the vault is read-only or another process holds it open.

    use std::{fs, os::windows::fs::OpenOptionsExt};

    use super::{StdDisk, TempDir, bak, new_vault, save_vault, verify};
    use almarioks_core::{CoreError, storage::Fingerprint};

    const FILE_SHARE_READ: u32 = 0x1;
    const FILE_SHARE_WRITE: u32 = 0x2;
    const FILE_SHARE_DELETE: u32 = 0x4;

    fn set_read_only(path: &std::path::Path, read_only: bool) {
        let mut permissions = fs::metadata(path).unwrap().permissions();
        #[allow(clippy::permissions_set_readonly_false)] // Windows attribute, not Unix mode bits
        permissions.set_readonly(read_only);
        fs::set_permissions(path, permissions).unwrap();
    }

    #[test]
    fn rf_23_a_read_only_vault_is_permission_denied_and_stays_intact() {
        let dir = TempDir::new();
        let vault = dir.join("cuentas.kdbx");
        let (v1, v2) = (new_vault(), new_vault());
        fs::write(&vault, &v1).unwrap();
        set_read_only(&vault, true);

        let error = save_vault(&StdDisk, &vault, &v2, Fingerprint::of(&v1), &verify).unwrap_err();
        let names = dir.names();
        set_read_only(&vault, false); // before asserting, so the folder can always be removed

        assert!(error.is_permission_denied());
        assert_eq!(CoreError::from(error), CoreError::GuardadoFallido);
        assert_eq!(fs::read(&vault).unwrap(), v1);
        assert_eq!(names, ["cuentas.kdbx"]);
    }

    #[test]
    fn rf_23_a_vault_held_open_without_share_delete_is_permission_denied() {
        let dir = TempDir::new();
        let vault = dir.join("cuentas.kdbx");
        let (v0, v1, v2) = (new_vault(), new_vault(), new_vault());
        fs::write(&vault, &v0).unwrap();
        let first = save_vault(&StdDisk, &vault, &v1, Fingerprint::of(&v0), &verify).unwrap();

        // Like Notepad or an antivirus: open without FILE_SHARE_DELETE.
        let held = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE)
            .open(&vault)
            .unwrap();
        let error = save_vault(&StdDisk, &vault, &v2, first.fingerprint, &verify).unwrap_err();
        drop(held);

        assert!(error.is_permission_denied());
        assert_eq!(fs::read(&vault).unwrap(), v1);
        assert_eq!(fs::read(bak(&vault)).unwrap(), v0, "the old backup stays");
        assert_eq!(dir.names(), ["cuentas.kdbx", "cuentas.kdbx.bak"]);

        // Like KeePass when reading: with FILE_SHARE_DELETE the save works.
        let shared = fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
            .open(&vault)
            .unwrap();
        save_vault(&StdDisk, &vault, &v2, first.fingerprint, &verify).unwrap();
        drop(shared);
        assert_eq!(fs::read(&vault).unwrap(), v2);
        assert_eq!(fs::read(bak(&vault)).unwrap(), v1);
    }
}
