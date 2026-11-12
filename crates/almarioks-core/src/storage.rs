//! Disk access of the core.
//!
//! What creating a vault needs (RF-4, RF-6): create a file only if it does not exist, write it and read it back.
//! The save path (RF-21 to RF-24, RF-27, RF-33, RF-89) lives in the submodules: [`Disk`] and [`StdDisk`], then
//! [`read_vault`], [`save_vault`] and [`save_vault_as`], the recovery file and the [`SaveQueue`]. They only see
//! bytes that are already encrypted: never a key nor a `keepass` type.

use std::{
    fs::{self, File, OpenOptions},
    io::{ErrorKind, Write},
    path::{Path, PathBuf},
};

use crate::error::{CoreError, CoreResult};

/// `ArchivoExiste` if anything is at `path` (a file, a folder or a link); nothing is touched.
pub(crate) fn ensure_absent(path: &Path) -> CoreResult<()> {
    match fs::symlink_metadata(path) {
        Ok(_) => Err(CoreError::ArchivoExiste),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(()),
        Err(_) => Err(CoreError::GuardadoFallido),
    }
}

/// Writes all bytes and waits until the disk has them.
pub(crate) fn write_and_sync(file: &mut File, bytes: &[u8]) -> CoreResult<()> {
    file.write_all(bytes)
        .map_err(|_| CoreError::GuardadoFallido)?;
    file.sync_all().map_err(|_| CoreError::GuardadoFallido)
}

/// Reads a whole file back.
pub(crate) fn read(path: &Path) -> CoreResult<Vec<u8>> {
    fs::read(path).map_err(|_| CoreError::GuardadoFallido)
}

/// Files created by one operation; removed when dropped unless [`CreatedFiles::keep`] was called. Only files this
/// value created are ever removed.
#[derive(Debug, Default)]
pub(crate) struct CreatedFiles {
    paths: Vec<PathBuf>,
    keep: bool,
}

impl CreatedFiles {
    /// Creates `path` only if it does not exist (`create_new`, atomic in the file system).
    pub(crate) fn create_new(&mut self, path: &Path) -> CoreResult<File> {
        let file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .map_err(|error| {
                if error.kind() == ErrorKind::AlreadyExists {
                    CoreError::ArchivoExiste
                } else {
                    CoreError::GuardadoFallido
                }
            })?;
        self.paths.push(path.to_path_buf());
        Ok(file)
    }

    /// The operation succeeded: the files stay.
    pub(crate) fn keep(mut self) {
        self.keep = true;
    }
}

impl Drop for CreatedFiles {
    fn drop(&mut self) {
        if !self.keep {
            for path in &self.paths {
                // Best effort: the operation already failed and reports its own error.
                drop(fs::remove_file(path));
            }
        }
    }
}

mod disk;
mod queue;
mod recovery;
mod save;
#[cfg(test)]
pub(crate) mod test_disk;
#[cfg(test)]
mod tests;

pub use disk::{Disk, StdDisk};
pub use queue::{SaveQueue, SaveTicket, Verifier};
pub use recovery::{Timestamp, write_recovery};
pub use save::{
    FailureCause, Fingerprint, SaveError, Saved, read_vault, save_vault, save_vault_as,
};
