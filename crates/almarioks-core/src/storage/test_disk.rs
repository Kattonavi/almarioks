//! A disk in memory for the tests that fails, cuts or hangs at a chosen operation (spec, "Cómo se prueba", rows 23,
//! 27, 30 and 33). It mimics what lab P3 saw in Windows: replacing, writing over or removing a read-only file is
//! `PermissionDenied`, and `copy` carries the read-only attribute over.

use std::{
    collections::BTreeMap,
    io::{self, ErrorKind},
    path::{Path, PathBuf},
    sync::{Mutex, MutexGuard, PoisonError, mpsc},
};

use super::disk::Disk;

/// What happens at the chosen operation.
#[derive(Debug)]
pub(crate) enum Fault {
    /// The operation fails with this error and does nothing; later operations work.
    Fail(ErrorKind),
    /// The process dies: a write leaves half its bytes, any other operation does nothing, and every later operation
    /// (including the cleanup of the save) fails until [`MemDisk::revive`].
    Cut,
    /// The operation tells `entered`, then blocks until `release` gets a message or its sender is dropped (so a failed
    /// assertion never hangs the test run), and then runs normally.
    Hang {
        entered: mpsc::SyncSender<()>,
        release: mpsc::Receiver<()>,
    },
}

#[derive(Debug, Clone)]
struct MemFile {
    bytes: Vec<u8>,
    read_only: bool,
}

#[derive(Debug, Default)]
struct Inner {
    files: BTreeMap<PathBuf, MemFile>,
    count: usize,
    fault: Option<(usize, Fault)>,
    dead: bool,
}

enum Action {
    Run,
    Cut,
}

/// The disk in memory.
#[derive(Debug, Default)]
pub(crate) struct MemDisk {
    inner: Mutex<Inner>,
}

fn denied() -> io::Error {
    io::Error::from(ErrorKind::PermissionDenied)
}

fn not_found() -> io::Error {
    io::Error::from(ErrorKind::NotFound)
}

fn gone() -> io::Error {
    io::Error::other("the process was cut")
}

impl MemDisk {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Puts a file, as if another app wrote it.
    pub(crate) fn put(&self, path: &Path, bytes: &[u8]) {
        self.lock().files.insert(
            path.to_path_buf(),
            MemFile {
                bytes: bytes.to_vec(),
                read_only: false,
            },
        );
    }

    /// The content of a file.
    pub(crate) fn get(&self, path: &Path) -> Option<Vec<u8>> {
        self.lock().files.get(path).map(|file| file.bytes.clone())
    }

    /// Every path on the disk, sorted.
    pub(crate) fn paths(&self) -> Vec<PathBuf> {
        self.lock().files.keys().cloned().collect()
    }

    /// Sets or clears the read-only attribute, as another app would.
    pub(crate) fn set_read_only(&self, path: &Path, read_only: bool) {
        if let Some(file) = self.lock().files.get_mut(path) {
            file.read_only = read_only;
        }
    }

    /// Arms `fault` at the `nth` operation from now (0 is the next one).
    pub(crate) fn fault_at(&self, nth: usize, fault: Fault) {
        let mut inner = self.lock();
        let at = inner.count + nth;
        inner.fault = Some((at, fault));
    }

    /// A new process after a cut: the disk works again and no fault is armed.
    pub(crate) fn revive(&self) {
        let mut inner = self.lock();
        inner.dead = false;
        inner.fault = None;
    }

    /// Operations done since the disk was made.
    pub(crate) fn op_count(&self) -> usize {
        self.lock().count
    }

    /// Runs one operation through the armed fault. The lock is never held while hanging.
    fn op<T>(&self, run: impl FnOnce(&mut Inner, Action) -> io::Result<T>) -> io::Result<T> {
        let hang = {
            let mut inner = self.lock();
            if inner.dead {
                return Err(gone());
            }
            let n = inner.count;
            inner.count += 1;
            let fault = match inner.fault.take() {
                Some((at, fault)) if at == n => Some(fault),
                other => {
                    inner.fault = other;
                    None
                }
            };
            match fault {
                None => None,
                Some(Fault::Fail(kind)) => return Err(io::Error::from(kind)),
                Some(Fault::Cut) => {
                    inner.dead = true;
                    return run(&mut inner, Action::Cut).and(Err(gone()));
                }
                Some(Fault::Hang { entered, release }) => Some((entered, release)),
            }
        };
        if let Some((entered, release)) = hang {
            _ = entered.send(()); // the test may have stopped listening
            _ = release.recv(); // a message or a dropped sender: both release
        }
        let mut inner = self.lock();
        run(&mut inner, Action::Run)
    }
}

/// Writes over `path` as Windows would: a read-only file refuses.
fn write_file(inner: &mut Inner, path: &Path, bytes: &[u8], action: &Action) -> io::Result<()> {
    if inner.files.get(path).is_some_and(|file| file.read_only) {
        return Err(denied());
    }
    let kept = match action {
        Action::Run => bytes,
        Action::Cut => bytes.get(..bytes.len() / 2).unwrap_or_default(),
    };
    inner.files.insert(
        path.to_path_buf(),
        MemFile {
            bytes: kept.to_vec(),
            read_only: false,
        },
    );
    Ok(())
}

impl Disk for MemDisk {
    fn read(&self, path: &Path) -> io::Result<Vec<u8>> {
        self.op(|inner, _| {
            inner
                .files
                .get(path)
                .map(|file| file.bytes.clone())
                .ok_or_else(not_found)
        })
    }

    fn exists(&self, path: &Path) -> io::Result<bool> {
        self.op(|inner, _| Ok(inner.files.contains_key(path)))
    }

    fn write_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.op(|inner, action| write_file(inner, path, bytes, &action))
    }

    fn write_new_synced(&self, path: &Path, bytes: &[u8]) -> io::Result<()> {
        self.op(|inner, action| {
            if inner.files.contains_key(path) {
                return Err(io::Error::from(ErrorKind::AlreadyExists));
            }
            write_file(inner, path, bytes, &action)
        })
    }

    fn copy(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.op(|inner, action| {
            if matches!(action, Action::Cut) {
                return Ok(());
            }
            let source = inner.files.get(from).cloned().ok_or_else(not_found)?;
            if inner.files.get(to).is_some_and(|file| file.read_only) {
                return Err(denied());
            }
            inner.files.insert(to.to_path_buf(), source);
            Ok(())
        })
    }

    fn clear_read_only(&self, path: &Path) -> io::Result<()> {
        self.op(|inner, action| {
            if matches!(action, Action::Cut) {
                return Ok(());
            }
            let file = inner.files.get_mut(path).ok_or_else(not_found)?;
            file.read_only = false;
            Ok(())
        })
    }

    fn replace(&self, from: &Path, to: &Path) -> io::Result<()> {
        self.op(|inner, action| {
            if matches!(action, Action::Cut) {
                return Ok(());
            }
            if !inner.files.contains_key(from) {
                return Err(not_found());
            }
            if inner.files.get(to).is_some_and(|file| file.read_only) {
                return Err(denied());
            }
            let moved = inner.files.remove(from).ok_or_else(not_found)?;
            inner.files.insert(to.to_path_buf(), moved);
            Ok(())
        })
    }

    fn remove(&self, path: &Path) -> io::Result<()> {
        self.op(|inner, action| {
            if matches!(action, Action::Cut) {
                return Ok(());
            }
            match inner.files.get(path) {
                None => Err(not_found()),
                Some(file) if file.read_only => Err(denied()),
                Some(_) => {
                    inner.files.remove(path);
                    Ok(())
                }
            }
        })
    }
}
