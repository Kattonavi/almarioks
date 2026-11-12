//! What the commands share: the state Tauri manages for the whole app.
//!
//! Everything the system provides enters through [`AppParts`], so the tests build the same app over a temporary
//! settings file and fakes, and never touch the person's real configuration, clipboard or screen.

use std::{
    path::{Path, PathBuf},
    sync::{
        Arc, Mutex, MutexGuard, PoisonError,
        atomic::{AtomicU32, Ordering},
    },
};

use almarioks_core::{CoreResult, kdbx::NewVaultKdf, session::Session, settings::Settings};

use crate::{
    clipboard::{ClipboardError, ClipboardService},
    commands::{dialogs::FileDialogs, url::UrlOpener},
    lock::{IdleClock, LockCenter, LockParts, WebviewReloader, WindowEvents},
};

/// Chooses the KDF of a new vault (RF-2). Production calibrates it on this machine, which takes seconds; the
/// tests give a fixed one.
pub type KdfChooser = dyn Fn() -> CoreResult<NewVaultKdf> + Send + Sync;

/// Asks the system for the person's Documents folder (RF-97), every time it is needed: the person can move it
/// while the app runs. `None` if the system has none to give. The tests give a temporary folder.
pub type DocumentsFolder = dyn Fn() -> Option<PathBuf> + Send + Sync;

/// Name of the settings file inside the app's configuration folder (RF-82).
pub const SETTINGS_FILE_NAME: &str = "settings.json";

/// The pieces the app is assembled from. Production fills them from the system (`crate::system_parts`).
pub struct AppParts {
    /// Where the settings live (RF-82): outside the vault, in the configuration folder Tauri gives.
    pub settings_file: PathBuf,
    /// The clipboard service, or why it could not start. The app starts either way; copying answers the error.
    pub clipboard: Result<ClipboardService, ClipboardError>,
    /// Who asks the person for a file (the system's dialogs).
    pub dialogs: Arc<dyn FileDialogs>,
    /// Who opens a validated URL (the default browser).
    pub url_opener: Arc<dyn UrlOpener>,
    /// The clocks of the inactivity lock (RF-29).
    pub idle_clock: Arc<dyn IdleClock>,
    /// Who reloads the webview after a lock (RF-32, AMENAZAS W2).
    pub reloader: Arc<dyn WebviewReloader>,
    /// The KDF of a vault created or converted here (RF-2).
    pub new_vault_kdf: Arc<KdfChooser>,
    /// The person's Documents folder, where new vaults are proposed (RF-97).
    pub documents_dir: Arc<DocumentsFolder>,
}

/// The state managed by Tauri: `tauri::State<'_, AppState>` in every command.
///
/// Each part is behind an `Arc` so a command can move it into `spawn_blocking`; none of them is a lock that a
/// command could hold across an `.await`.
pub struct AppState {
    settings: Arc<SettingsStore>,
    clipboard: Result<Arc<ClipboardService>, ClipboardError>,
    dialogs: Arc<dyn FileDialogs>,
    url_opener: Arc<dyn UrlOpener>,
    /// The session of the core with its lock, its inactivity timer and its events (`crate::lock`).
    lock: Arc<LockCenter>,
    new_vault_kdf: Arc<KdfChooser>,
    documents_dir: Arc<DocumentsFolder>,
}

impl AppState {
    /// Assembles the state: a closed session wired to the clipboard, the settings and `window`.
    #[must_use]
    pub fn new(parts: AppParts, window: Arc<dyn WindowEvents>) -> Self {
        let settings = Arc::new(SettingsStore::new(parts.settings_file));
        let clipboard = parts.clipboard.map(Arc::new);
        let lock = Arc::new(LockCenter::new(LockParts {
            clipboard: clipboard.as_ref().ok().cloned(),
            settings: Arc::clone(&settings),
            window,
            reloader: parts.reloader,
            clock: parts.idle_clock,
        }));
        Self {
            settings,
            clipboard,
            dialogs: parts.dialogs,
            url_opener: parts.url_opener,
            lock,
            new_vault_kdf: parts.new_vault_kdf,
            documents_dir: parts.documents_dir,
        }
    }

    /// The session of the core: the vault, open or not.
    #[must_use]
    pub fn session(&self) -> Arc<Session> {
        self.lock.session()
    }

    /// The lock: who asks for it, the inactivity timer and the exit.
    #[must_use]
    pub fn lock_center(&self) -> Arc<LockCenter> {
        Arc::clone(&self.lock)
    }

    /// Chooses the KDF of a new vault. Slow in production: call it from `spawn_blocking`.
    #[must_use]
    pub fn new_vault_kdf(&self) -> Arc<KdfChooser> {
        Arc::clone(&self.new_vault_kdf)
    }

    /// The person's Documents folder as the system gives it now (RF-97), or `None` if it gives none or one that
    /// is not a full path. Asks the system: call it from `spawn_blocking`.
    #[must_use]
    pub fn documents_dir(&self) -> Arc<DocumentsFolder> {
        let ask = Arc::clone(&self.documents_dir);
        Arc::new(move || ask().filter(|folder| folder.is_absolute()))
    }

    /// The settings and recent vaults.
    #[must_use]
    pub fn settings(&self) -> Arc<SettingsStore> {
        Arc::clone(&self.settings)
    }

    /// The clipboard service.
    ///
    /// # Errors
    /// [`ClipboardError::Unavailable`] if it could not start with the app.
    pub fn clipboard(&self) -> Result<Arc<ClipboardService>, ClipboardError> {
        self.clipboard.clone()
    }

    /// The file dialogs.
    #[must_use]
    pub fn dialogs(&self) -> Arc<dyn FileDialogs> {
        Arc::clone(&self.dialogs)
    }

    /// The opener of URLs, for `commands::url::open_url` (RF-49).
    #[must_use]
    pub fn url_opener(&self) -> Arc<dyn UrlOpener> {
        Arc::clone(&self.url_opener)
    }
}

/// Writes the settings to their file. Production uses [`Settings::save`]; the tests give one that fails or hangs.
pub type SettingsWriter = dyn Fn(&Settings, &Path) -> CoreResult<()> + Send + Sync;

/// What [`SettingsStore::auto_lock_minutes`] holds until the file has been read: no choice of the settings is 0.
const NOT_READ_YET: u32 = 0;

/// The settings file and its lock (RF-82). The file is read once, on first use, and every change is validated,
/// written and only then kept, all under the same lock.
///
/// That lock is held while the file is written, and a disk can hang. The inactivity lock (RF-29) must not wait
/// for it, so the one value it needs is also kept outside the lock: see [`Self::auto_lock_minutes`].
///
/// Every method may touch the disk: call them from `spawn_blocking`, never straight from an `async` command.
pub struct SettingsStore {
    file: PathBuf,
    write: Box<SettingsWriter>,
    current: Mutex<Option<Settings>>,
    /// The minutes of the inactivity lock as the file has them: stored when the file is first read and after
    /// every change that reached the disk, never before. [`NOT_READ_YET`] until then.
    auto_lock_minutes: AtomicU32,
}

impl SettingsStore {
    /// A store over `file`. Nothing is read yet.
    #[must_use]
    pub fn new(file: PathBuf) -> Self {
        Self::with_writer(file, Box::new(Settings::save))
    }

    /// A store over `file` whose changes are written by `write`.
    #[must_use]
    pub fn with_writer(file: PathBuf, write: Box<SettingsWriter>) -> Self {
        Self {
            file,
            write,
            current: Mutex::new(None),
            auto_lock_minutes: AtomicU32::new(NOT_READ_YET),
        }
    }

    /// Where the settings are stored.
    #[must_use]
    pub fn file(&self) -> &Path {
        &self.file
    }

    /// The current settings. A missing or damaged file gives the defaults, field by field (RF-82).
    #[must_use]
    pub fn read(&self) -> Settings {
        let mut current = self.lock();
        self.loaded(&mut current).clone()
    }

    /// The minutes of the inactivity lock (RF-29), for the heartbeat and the countdown of the window. Once the
    /// file has been read this never takes the lock nor touches the disk, so a change that hangs while it is
    /// written cannot keep the vault from locking. It is the value that is on disk: a change that is still
    /// being written, or that could not be written, does not count.
    ///
    /// Before the first read it reads the file, as [`Self::read`] does.
    #[must_use]
    pub fn auto_lock_minutes(&self) -> u32 {
        match self.auto_lock_minutes.load(Ordering::SeqCst) {
            NOT_READ_YET => self.read().auto_lock_minutes(),
            minutes => minutes,
        }
    }

    /// Applies `change` to a copy, writes the file and only then keeps the copy: all or nothing. If `change`
    /// leaves the settings as they were, nothing is written.
    ///
    /// # Errors
    /// Whatever `change` answers (nothing is kept), or `GuardadoFallido` if the file cannot be written (the
    /// settings in memory, and the minutes the inactivity lock reads, stay as they were).
    pub fn update(
        &self,
        change: impl FnOnce(&mut Settings) -> CoreResult<()>,
    ) -> CoreResult<Settings> {
        let mut current = self.lock();
        let before = self.loaded(&mut current);
        let mut after = before.clone();
        change(&mut after)?;
        if after != *before {
            (self.write)(&after, &self.file)?;
            self.auto_lock_minutes
                .store(after.auto_lock_minutes(), Ordering::SeqCst);
            *before = after.clone();
        }
        Ok(after)
    }

    fn loaded<'a>(&self, current: &'a mut Option<Settings>) -> &'a mut Settings {
        current.get_or_insert_with(|| {
            let settings = Settings::load(&self.file);
            self.auto_lock_minutes
                .store(settings.auto_lock_minutes(), Ordering::SeqCst);
            settings
        })
    }

    fn lock(&self) -> MutexGuard<'_, Option<Settings>> {
        // A panic while holding the lock cannot leave half-applied settings: `update` swaps a finished copy.
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
