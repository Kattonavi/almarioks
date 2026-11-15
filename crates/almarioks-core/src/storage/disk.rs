//! The disk as a trait (plan, "Regla de dependencia"): the save path runs against [`StdDisk`] in the app and against
//! a disk that fails, cuts or hangs in the tests (spec, "Cómo se prueba", rows 23, 27, 30 and 33).

use std::{
    fs::{self, File, OpenOptions},
    io::{self, ErrorKind, Write},
    path::Path,
};

/// The few disk operations the save path needs. Every method is one step that can fail on its own.
///
/// Implementations must be usable from the save thread (`Send + Sync`).
pub trait Disk: Send + Sync {
    /// Reads a whole file.
    ///
    /// # Errors
    /// The I/O error; [`ErrorKind::NotFound`] when nothing is at `path`.
    fn read(&self, path: &Path) -> io::Result<Vec<u8>>;

    /// Whether anything (a file, a folder or a link) is at `path`.
    ///
    /// # Errors
    /// The I/O error when the answer is unknown (for example, no permission on the folder).
    fn exists(&self, path: &Path) -> io::Result<bool>;

    /// Creates or truncates `path`, writes all of `bytes` and waits until the disk has them (`sync_all`).
    ///
    /// # Errors
    /// The I/O error; the file may be left partly written.
    fn write_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;

    /// Like [`Disk::write_synced`], but only if nothing is at `path` (atomic in the file system). If it fails after
    /// creating the file, it removes it (best effort).
    ///
    /// # Errors
    /// [`ErrorKind::AlreadyExists`] when something is at `path`; otherwise the I/O error.
    fn write_new_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()>;

    /// Copies `from` over `to` (contents and permissions, like [`fs::copy`]).
    ///
    /// # Errors
    /// The I/O error.
    fn copy(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Removes the read-only attribute of `path`.
    ///
    /// # Errors
    /// The I/O error.
    fn clear_read_only(&self, path: &Path) -> io::Result<()>;

    /// Renames `from` to `to` in one step, replacing `to` if it exists ([`fs::rename`]; in Windows it replaces).
    ///
    /// # Errors
    /// The I/O error; [`ErrorKind::PermissionDenied`] when `to` is read-only or another process holds it open
    /// without `FILE_SHARE_DELETE` (`docs/lab/boveda-0.1/EXPERIMENTO.md`, P3).
    fn replace(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// Removes a file.
    ///
    /// # Errors
    /// The I/O error.
    fn remove(&self, path: &Path) -> io::Result<()>;
}

/// The real disk, with `std::fs` and no `unsafe` (lab P3 showed `std` is enough on Windows).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct StdDisk;

fn write_all_synced(mut file: File, bytes: &[u8]) -> io::Result<()> {
    file.write_all(bytes)?;
    file.sync_all()
}

impl Disk for StdDisk {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        fs::read(path)
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        match fs::symlink_metadata(path) {
            Ok(_) => Ok(true),
            Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
            Err(error) => Err(error),
        }
    }

    fn write_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(path)?;
        write_all_synced(file, bytes)
    }

    fn write_new_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        let file = OpenOptions::new().write(true).create_new(true).open(path)?;
        let written = write_all_synced(file, bytes);
        if written.is_err() {
            // Best effort: only the file this call created; the write error is the one reported.
            drop(fs::remove_file(path));
        }
        written
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::copy(from, to).map(drop)
    }

    fn clear_read_only(&self, path: &Path) -> io::Result<()> {
        let mut permissions = fs::metadata(path)?.permissions();
        if !permissions.readonly() {
            return Ok(());
        }
        clear_read_only_flag(&mut permissions);
        fs::set_permissions(path, permissions)
    }

    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        fs::rename(from, to)
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        fs::remove_file(path)
    }
}

#[cfg(windows)]
fn clear_read_only_flag(permissions: &mut fs::Permissions) {
    // In Windows this only clears FILE_ATTRIBUTE_READONLY (Clippy's warning about it is for Unix mode bits).
    permissions.set_readonly(false);
}

#[cfg(unix)]
fn clear_read_only_flag(permissions: &mut fs::Permissions) {
    use std::os::unix::fs::PermissionsExt;
    // Only the owner's write bit; group and others stay as they were.
    permissions.set_mode(permissions.mode() | 0o200);
}

#[cfg(not(any(windows, unix)))]
fn clear_read_only_flag(_permissions: &mut fs::Permissions) {}
