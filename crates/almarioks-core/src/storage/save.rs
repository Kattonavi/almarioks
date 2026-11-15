//! Saving a vault to disk (RF-21 to RF-24, RF-27, RF-95), in the order verified in Windows by the lab
//! (`docs/lab/boveda-0.1/EXPERIMENTO.md`, P3):
//!
//! 0. the file on disk still has the content last read or written (RF-24);
//! 1. write `<vault>.tmp` in the same folder and `sync_all`;
//! 2. read it back and let the caller check that it decrypts with the key (RF-21);
//! 3. copy the vault to `<vault>.bak.tmp`, without the read-only attribute that `fs::copy` carries over;
//! 4. replace the vault with the temporary file in one step;
//! 5. only if 4 worked, promote `<vault>.bak.tmp` to `<vault>.bak` (one copy, RF-22).
//!
//! The vault never stops existing, and if step 4 fails the previous `.bak` is still there. This module only sees
//! bytes that are already encrypted, paths, fingerprints and functions: never a key nor a `keepass` type (plan,
//! "Concurrencia y bloqueo").

use std::{
    ffi::OsString,
    fmt,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
};

use sha2::{Digest, Sha256};

use super::disk::Disk;
use crate::error::{CoreError, CoreResult};

/// Suffix of the temporary file of a save, next to the vault.
const TMP_SUFFIX: &str = ".tmp";
/// Suffix of the backup copy being prepared.
const BAK_TMP_SUFFIX: &str = ".bak.tmp";
/// Suffix of the one backup copy (RF-22).
const BAK_SUFFIX: &str = ".bak";

/// SHA-256 of a vault file's bytes (RF-24).
///
/// A fingerprint of content to notice that another app rewrote the file, not part of the vault's cryptography: the
/// bytes it hashes are already encrypted and it protects nothing (constitution rule 4 keeps the vault's crypto in
/// `keepass`). The file's date does not count, only its content (spec, "Casos límite").
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct Fingerprint([u8; 32]);

impl Fingerprint {
    /// The fingerprint of `bytes`.
    #[must_use]
    pub fn of(bytes: &[u8]) -> Self {
        Self(Sha256::digest(bytes).into())
    }

    /// The 32 bytes of the SHA-256.
    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

impl fmt::Debug for Fingerprint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Fingerprint(")?;
        for byte in self.0.iter().take(4) {
            write!(f, "{byte:02x}")?;
        }
        f.write_str("…)")
    }
}

/// Why writing failed, so the window can say more than the code (lab P3: `PermissionDenied` means another app has
/// the file open or it is read-only, and the person can close that app and retry).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum FailureCause {
    /// The system refused: the file is read-only or another process has it open without `FILE_SHARE_DELETE`.
    PermissionDenied,
    /// The temporary file did not read back as written or did not decrypt with the key (RF-21).
    Verification,
    /// Any other I/O error (disk full, folder gone, device error).
    Io,
}

/// A save that did not happen. In every case the vault file on disk is as it was (RF-23), except where noted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum SaveError {
    /// The file on disk is not what was last read or written, or it is gone (RF-24). Nothing was touched.
    ExternalChange,
    /// "Save as" found a file at the new path (RF-23). Nothing was touched.
    AlreadyExists,
    /// A step failed; the temporary files of this save were removed (RF-23).
    Failed(FailureCause),
    /// The save queue was closed by a lock before this save was written (RF-30).
    Closed,
}

impl SaveError {
    /// The stable code of the spec for this failure.
    #[must_use]
    pub const fn code(self) -> CoreError {
        match self {
            Self::ExternalChange => CoreError::CambioExterno,
            Self::AlreadyExists => CoreError::ArchivoExiste,
            Self::Failed(_) => CoreError::GuardadoFallido,
            Self::Closed => CoreError::Bloqueada,
        }
    }

    /// Whether the system refused the write: the window can suggest closing the other app that has the file open.
    #[must_use]
    pub const fn is_permission_denied(self) -> bool {
        matches!(self, Self::Failed(FailureCause::PermissionDenied))
    }
}

impl fmt::Display for SaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(&self.code(), f)
    }
}

impl std::error::Error for SaveError {}

impl From<SaveError> for CoreError {
    fn from(error: SaveError) -> Self {
        error.code()
    }
}

/// A save that happened.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Saved {
    /// Fingerprint of what is now on disk: the one the next save must expect (RF-24).
    pub fingerprint: Fingerprint,
    /// `false` if the vault was replaced but promoting `.bak.tmp` to `.bak` failed: the save is done and the previous
    /// `.bak` (if any) stays. Decision pending: the spec does not say what to tell the person in that case.
    pub backup_updated: bool,
}

fn failed(error: &io::Error) -> SaveError {
    if error.kind() == ErrorKind::PermissionDenied {
        SaveError::Failed(FailureCause::PermissionDenied)
    } else {
        SaveError::Failed(FailureCause::Io)
    }
}

/// `path` with `suffix` added to its whole file name: `cuentas.kdbx` + `.bak` → `cuentas.kdbx.bak`.
pub(crate) fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name: OsString = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}

/// Files this save created; removed when dropped unless released. Best effort: the save already failed and reports
/// its own error.
struct Leftovers<'a> {
    disk: &'a dyn Disk,
    paths: Vec<PathBuf>,
}

impl<'a> Leftovers<'a> {
    fn new(disk: &'a dyn Disk) -> Self {
        Self {
            disk,
            paths: Vec::new(),
        }
    }

    fn track(&mut self, path: &Path) {
        self.paths.push(path.to_path_buf());
    }

    fn release(&mut self, path: &Path) {
        self.paths.retain(|tracked| tracked != path);
    }
}

impl Drop for Leftovers<'_> {
    fn drop(&mut self) {
        for path in &self.paths {
            drop(self.disk.remove(path));
        }
    }
}

/// Reads a vault file and its fingerprint, the one the first save must expect (RF-24).
///
/// # Errors
/// [`CoreError::ArchivoNoEncontrado`] if nothing is at `path` (RF-95). Any other read error is
/// [`CoreError::ArchivoDanado`] (decision pending: the spec has no code for "cannot read").
pub fn read_vault(disk: &dyn Disk, path: &Path) -> CoreResult<(Vec<u8>, Fingerprint)> {
    match disk.read(path) {
        Ok(bytes) => {
            let fingerprint = Fingerprint::of(&bytes);
            Ok((bytes, fingerprint))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Err(CoreError::ArchivoNoEncontrado),
        Err(_) => Err(CoreError::ArchivoDanado),
    }
}

/// Writes `<new path>.tmp`, reads it back and verifies it (steps 1 and 2). The temporary file is tracked.
fn write_verified_tmp(
    disk: &dyn Disk,
    leftovers: &mut Leftovers<'_>,
    tmp: &Path,
    encrypted: &[u8],
    verify: &dyn Fn(&[u8]) -> CoreResult<()>,
) -> Result<(), SaveError> {
    // A temporary file left by a cut save is overwritten (lab P3) and, if this save fails, removed.
    leftovers.track(tmp);
    disk.write_synced(tmp, encrypted)
        .map_err(|error| failed(&error))?;
    let reread = disk.read(tmp).map_err(|error| failed(&error))?;
    if reread != encrypted || verify(&reread).is_err() {
        return Err(SaveError::Failed(FailureCause::Verification));
    }
    Ok(())
}

/// Saves `encrypted` (the whole vault, already encrypted) over `path` (RF-21 to RF-24, RF-27).
///
/// `expected` is the fingerprint of what was last read or written; `verify` receives the bytes read back from the
/// temporary file and must check that they decrypt with the key (RF-21).
///
/// # Errors
/// - [`SaveError::ExternalChange`] if the file's content is not `expected` or it is gone; nothing is touched.
/// - [`SaveError::Failed`] if any step fails: the vault is byte for byte as it was, the previous `.bak` stays and
///   the temporary files of this save are removed (RF-23).
pub fn save_vault(
    disk: &dyn Disk,
    path: &Path,
    encrypted: &[u8],
    expected: Fingerprint,
    verify: &dyn Fn(&[u8]) -> CoreResult<()>,
) -> Result<Saved, SaveError> {
    // Step 0 (RF-24). A missing file is an external change too: decision pending, the conservative choice is not to
    // recreate a vault someone moved or deleted; the person can "save as".
    let current = match disk.read(path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == ErrorKind::NotFound => {
            return Err(SaveError::ExternalChange);
        }
        Err(error) => return Err(failed(&error)),
    };
    if Fingerprint::of(&current) != expected {
        return Err(SaveError::ExternalChange);
    }
    drop(current);

    let tmp = with_suffix(path, TMP_SUFFIX);
    let bak_tmp = with_suffix(path, BAK_TMP_SUFFIX);
    let bak = with_suffix(path, BAK_SUFFIX);
    let mut leftovers = Leftovers::new(disk);

    // Steps 1 and 2.
    write_verified_tmp(disk, &mut leftovers, &tmp, encrypted, verify)?;

    // Step 3: a read-only `.bak` would make the next save fail (lab P3).
    leftovers.track(&bak_tmp);
    disk.copy(path, &bak_tmp).map_err(|error| failed(&error))?;
    disk.clear_read_only(&bak_tmp)
        .map_err(|error| failed(&error))?;

    // Step 4: the only step that changes the vault.
    disk.replace(&tmp, path).map_err(|error| failed(&error))?;
    leftovers.release(&tmp);
    let fingerprint = Fingerprint::of(encrypted);

    // Step 5: the vault is saved whatever happens here; if it fails, `.bak.tmp` is removed and the old `.bak` stays.
    let backup_updated = disk.replace(&bak_tmp, &bak).is_ok();
    if backup_updated {
        leftovers.release(&bak_tmp);
    }
    Ok(Saved {
        fingerprint,
        backup_updated,
    })
}

/// Saves `encrypted` at a new path, for "save as" after a failure or an external change (RF-23, RF-24). It never
/// replaces a file that exists.
///
/// # Errors
/// - [`SaveError::AlreadyExists`] if something is at `new_path`; nothing is touched.
/// - [`SaveError::Failed`] if any step fails; the temporary file is removed.
///
/// The check right before the rename leaves a tiny window in which a file created by someone else at `new_path`
/// would be replaced; `std` has no "rename without replacing" in Windows without `unsafe`.
pub fn save_vault_as(
    disk: &dyn Disk,
    new_path: &Path,
    encrypted: &[u8],
    verify: &dyn Fn(&[u8]) -> CoreResult<()>,
) -> Result<Fingerprint, SaveError> {
    let absent = |disk: &dyn Disk| match disk.exists(new_path) {
        Ok(false) => Ok(()),
        Ok(true) => Err(SaveError::AlreadyExists),
        Err(error) => Err(failed(&error)),
    };
    absent(disk)?;
    let tmp = with_suffix(new_path, TMP_SUFFIX);
    let mut leftovers = Leftovers::new(disk);
    write_verified_tmp(disk, &mut leftovers, &tmp, encrypted, verify)?;
    absent(disk)?;
    disk.replace(&tmp, new_path)
        .map_err(|error| failed(&error))?;
    leftovers.release(&tmp);
    Ok(Fingerprint::of(encrypted))
}
