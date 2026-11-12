//! What the commands share: the state Tauri manages for the whole app.
//!
//! Everything the system provides enters through [`AppParts`], so the tests build the same app over a temporary
//! settings file and fakes, and never touch the person's real configuration, clipboard or screen.

use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex, MutexGuard, PoisonError},
};

use almarioks_core::{CoreResult, settings::Settings};

use crate::{
    clipboard::{ClipboardError, ClipboardService},
    commands::{dialogs::FileDialogs, url::UrlOpener},
};

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
    // The session of the core (the open vault, its lock and the inactivity timer) goes here: T13/T14.
}

impl AppState {
    /// Assembles the state.
    #[must_use]
    pub fn new(parts: AppParts) -> Self {
        Self {
            settings: Arc::new(SettingsStore::new(parts.settings_file)),
            clipboard: parts.clipboard.map(Arc::new),
            dialogs: parts.dialogs,
            url_opener: parts.url_opener,
        }
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

/// The settings file and its lock (RF-82). The file is read once, on first use, and every change is validated,
/// written and only then kept, all under the same lock.
///
/// Every method may touch the disk: call them from `spawn_blocking`, never straight from an `async` command.
pub struct SettingsStore {
    file: PathBuf,
    current: Mutex<Option<Settings>>,
}

impl SettingsStore {
    /// A store over `file`. Nothing is read yet.
    #[must_use]
    pub const fn new(file: PathBuf) -> Self {
        Self {
            file,
            current: Mutex::new(None),
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

    /// Applies `change` to a copy, writes the file and only then keeps the copy: all or nothing. If `change`
    /// leaves the settings as they were, nothing is written.
    ///
    /// # Errors
    /// Whatever `change` answers (nothing is kept), or `GuardadoFallido` if the file cannot be written (the
    /// settings in memory stay as they were).
    pub fn update(
        &self,
        change: impl FnOnce(&mut Settings) -> CoreResult<()>,
    ) -> CoreResult<Settings> {
        let mut current = self.lock();
        let before = self.loaded(&mut current);
        let mut after = before.clone();
        change(&mut after)?;
        if after != *before {
            after.save(&self.file)?;
            *before = after.clone();
        }
        Ok(after)
    }

    fn loaded<'a>(&self, current: &'a mut Option<Settings>) -> &'a mut Settings {
        current.get_or_insert_with(|| Settings::load(&self.file))
    }

    fn lock(&self) -> MutexGuard<'_, Option<Settings>> {
        // A panic while holding the lock cannot leave half-applied settings: `update` swaps a finished copy.
        self.current.lock().unwrap_or_else(PoisonError::into_inner)
    }
}
