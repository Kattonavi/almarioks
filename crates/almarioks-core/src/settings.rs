//! Settings and recent vaults (T12, RF-19, RF-20, RF-82), stored outside the vault.
//!
//! The file is JSON with three keys: `auto_lock_minutes`, `clipboard_seconds` and `recent_vaults`, and a fourth
//! one, `key_files`, only while some recent vault has a key file remembered (RF-111). Reading never fails: each
//! field that is missing, of the wrong type, out of range or unknown falls back to its default on its own, and a
//! file that is not JSON gives all the defaults (RF-82). No key and no password ever enters this type (RF-19).
//! Where the file lives is decided by the app and arrives here as a parameter.
//!
//! # The key file remembered for a recent vault (RF-111)
//! Only its path, as text: never what the file holds, nor a hash, a size or a date of it. Two things always hold,
//! whoever calls and whatever the file says (security review 0.1, second round, C-1, C-2 and C-4):
//! - a path is remembered only for a vault that is in the recent list, so it goes away with it;
//! - the path passes [`is_rememberable_key_file`], checked the same way when it is read from the file and when it
//!   is set.
//!
//! An older version of the app ignores `key_files` (an unknown field), and a file without it has none remembered.

use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use serde_json::{Map, Value, json};

use crate::error::{CoreError, CoreResult};

/// Choices for the inactivity lock, in minutes (RF-29). There is no "never".
pub const AUTO_LOCK_CHOICES: [u32; 4] = [1, 5, 15, 30];
/// Inactivity lock when the file says nothing valid (RF-29).
pub const DEFAULT_AUTO_LOCK_MINUTES: u32 = 5;
/// Allowed range of the clipboard clearing time, in seconds (RF-73).
pub const CLIPBOARD_SECONDS_RANGE: std::ops::RangeInclusive<u32> = 10..=120;
/// Clipboard clearing time when the file says nothing valid (RF-73).
pub const DEFAULT_CLIPBOARD_SECONDS: u32 = 20;
/// How many recent vaults are remembered (RF-19).
pub const MAX_RECENT_VAULTS: usize = 5;
/// Most characters (code points) of a key file path that is remembered (RF-111).
pub const MAX_KEY_FILE_PATH_CHARS: usize = 1024;

const KEY_AUTO_LOCK: &str = "auto_lock_minutes";
const KEY_CLIPBOARD: &str = "clipboard_seconds";
const KEY_RECENT: &str = "recent_vaults";
const KEY_KEY_FILES: &str = "key_files";

/// Whether the path of a key file may be remembered (RF-111; security review 0.1, second round, C-2): only a full
/// path on a drive with a letter, `X:\folder\file`.
///
/// Decided over the text, so the answer is the same on every system: a letter, `:`, a separator (`\` or `/`), no
/// other `:` anywhere, at most [`MAX_KEY_FILE_PATH_CHARS`] characters, and a file name at the end. That leaves out
/// network paths (`\\server\share`, `//server/share`), `\\?\` and `\\.\` paths, relative ones, `X:name`, and
/// alternate streams (`file:stream`): the app never asks the disk about a remembered path that names another
/// machine or a device.
///
/// What it cannot tell: a drive letter that is a mapped network drive or a `subst` (declared in the review).
#[must_use]
pub fn is_rememberable_key_file(path: &str) -> bool {
    let mut chars = path.chars();
    let (Some(drive), Some(':'), Some('\\' | '/')) = (chars.next(), chars.next(), chars.next())
    else {
        return false;
    };
    let rest = chars.as_str();
    let file_name = rest.rsplit(['\\', '/']).next().unwrap_or_default();
    drive.is_ascii_alphabetic()
        && !rest.contains(':')
        && !matches!(file_name, "" | "." | "..")
        && path.chars().count() <= MAX_KEY_FILE_PATH_CHARS
}

/// A recent vault and whether its file is still there (RF-20).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecentVault {
    /// Where the vault was last opened.
    pub path: PathBuf,
    /// The file is still at `path`.
    pub exists: bool,
}

/// The person's settings. Fields are private so that the ranges always hold.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Settings {
    auto_lock_minutes: u32,
    clipboard_seconds: u32,
    recent_vaults: Vec<PathBuf>,
    /// Recent vault → the key file it was last opened with (RF-111). The module documentation says what always
    /// holds.
    key_files: BTreeMap<PathBuf, String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            auto_lock_minutes: DEFAULT_AUTO_LOCK_MINUTES,
            clipboard_seconds: DEFAULT_CLIPBOARD_SECONDS,
            recent_vaults: Vec::new(),
            key_files: BTreeMap::new(),
        }
    }
}

impl Settings {
    /// Reads settings from the bytes of the file. Never fails: see the module documentation (RF-82).
    #[must_use]
    pub fn from_bytes(bytes: &[u8]) -> Self {
        let mut settings = Self::default();
        let Ok(Value::Object(fields)) = serde_json::from_slice::<Value>(bytes) else {
            return settings;
        };
        if let Some(minutes) = number(&fields, KEY_AUTO_LOCK)
            && AUTO_LOCK_CHOICES.contains(&minutes)
        {
            settings.auto_lock_minutes = minutes;
        }
        if let Some(seconds) = number(&fields, KEY_CLIPBOARD)
            && CLIPBOARD_SECONDS_RANGE.contains(&seconds)
        {
            settings.clipboard_seconds = seconds;
        }
        if let Some(Value::Array(items)) = fields.get(KEY_RECENT) {
            // The file lists the most recent first. Items that are not a non-empty text are skipped, and
            // duplicates or more than 5 (a hand-edited file) are dropped keeping the first ones.
            for item in items {
                if let Value::String(text) = item
                    && !text.is_empty()
                {
                    let path = PathBuf::from(text);
                    if !settings.recent_vaults.contains(&path) {
                        settings.recent_vaults.push(path);
                    }
                }
            }
            settings.recent_vaults.truncate(MAX_RECENT_VAULTS);
        }
        if let Some(Value::Object(items)) = fields.get(KEY_KEY_FILES) {
            // RF-111: validated as when it is set. An entry for a vault that is not recent, or whose value is not
            // a text that may be remembered, is dropped here without a word and without looking at the disk; it
            // leaves the file the next time the file is written. At most one per recent vault, so at most 5.
            for (vault, key_file) in items {
                if let Value::String(key_file) = key_file {
                    settings.set_key_file(Path::new(vault), Some(Path::new(key_file)));
                }
            }
        }
        settings
    }

    /// The text to write to the file.
    #[must_use]
    pub fn to_text(&self) -> String {
        // Paths that are not valid UTF-8 never get here: `remember_vault` refuses them.
        let recent: Vec<Value> = self
            .recent_vaults
            .iter()
            .filter_map(|path| path.to_str())
            .map(Value::from)
            .collect();
        let mut value = json!({
            KEY_AUTO_LOCK: self.auto_lock_minutes,
            KEY_CLIPBOARD: self.clipboard_seconds,
            KEY_RECENT: recent,
        });
        // RF-111: the key is only there while something is remembered, so the file of a person who uses no key
        // file elsewhere is the file of always.
        let key_files: Map<String, Value> = self
            .key_files
            .iter()
            .filter_map(|(vault, key_file)| Some((vault.to_str()?.to_owned(), json!(key_file))))
            .collect();
        if let Value::Object(fields) = &mut value
            && !key_files.is_empty()
        {
            fields.insert(KEY_KEY_FILES.to_owned(), Value::Object(key_files));
        }
        format!("{value:#}\n")
    }

    /// Minutes without activity before the vault locks.
    #[must_use]
    pub const fn auto_lock_minutes(&self) -> u32 {
        self.auto_lock_minutes
    }

    /// Seconds before the clipboard is cleared.
    #[must_use]
    pub const fn clipboard_seconds(&self) -> u32 {
        self.clipboard_seconds
    }

    /// Recent vaults, the most recent first, at most 5, without duplicates.
    #[must_use]
    pub fn recent_vaults(&self) -> &[PathBuf] {
        &self.recent_vaults
    }

    /// The key file remembered for a recent vault (RF-111): the path it was last opened with, or `None` if none
    /// is remembered or the vault is not in the recent list. Only a path: whether the file is still there is the
    /// caller's question.
    #[must_use]
    pub fn key_file(&self, vault: &Path) -> Option<&Path> {
        self.key_files.get(vault).map(Path::new)
    }

    /// Remembers `key_file` as the key file of the recent vault `vault`, in the place of the one it had, or
    /// forgets the one it had with `None` (RF-111). For an unlock that worked, right after
    /// [`Self::remember_vault`], and for nothing else (security review 0.1, second round, C-1).
    ///
    /// A path that may not be remembered ([`is_rememberable_key_file`]), or a vault that is not in the recent
    /// list, leaves nothing remembered for that vault.
    pub fn set_key_file(&mut self, vault: &Path, key_file: Option<&Path>) {
        self.key_files.remove(vault);
        let rememberable = key_file
            .and_then(Path::to_str)
            .filter(|text| is_rememberable_key_file(text));
        if let Some(text) = rememberable
            && self.recent_vaults.iter().any(|known| known == vault)
        {
            self.key_files.insert(vault.to_path_buf(), text.to_owned());
        }
    }

    /// Sets the inactivity lock.
    ///
    /// # Errors
    /// `ArgumentoInvalido` if it is not one of 1, 5, 15 or 30.
    pub fn set_auto_lock_minutes(&mut self, minutes: u32) -> CoreResult<()> {
        if !AUTO_LOCK_CHOICES.contains(&minutes) {
            return Err(CoreError::ArgumentoInvalido);
        }
        self.auto_lock_minutes = minutes;
        Ok(())
    }

    /// Sets the clipboard clearing time.
    ///
    /// # Errors
    /// `ArgumentoInvalido` if it is not between 10 and 120.
    pub fn set_clipboard_seconds(&mut self, seconds: u32) -> CoreResult<()> {
        if !CLIPBOARD_SECONDS_RANGE.contains(&seconds) {
            return Err(CoreError::ArgumentoInvalido);
        }
        self.clipboard_seconds = seconds;
        Ok(())
    }

    /// A vault was opened: it goes first, without duplicates, and only the last 5 stay (RF-19). Paths are compared
    /// as they are (no case folding, no resolving links: decisión pendiente). An empty path or one that is not
    /// valid UTF-8 cannot be stored in the file and is ignored.
    ///
    /// The key file remembered for this vault stays as it was, and the one of the vault that falls off the list
    /// goes with it (RF-111).
    pub fn remember_vault(&mut self, path: &Path) {
        if path.as_os_str().is_empty() || path.to_str().is_none() {
            return;
        }
        self.recent_vaults.retain(|known| known != path);
        self.recent_vaults.insert(0, path.to_path_buf());
        self.recent_vaults.truncate(MAX_RECENT_VAULTS);
        self.forget_key_files_of_vaults_not_recent();
    }

    /// Removes a vault from the list (RF-20, "quitar de la lista"), and with it the key file remembered for it
    /// (RF-111). Nothing happens if it is not there.
    pub fn forget_vault(&mut self, path: &Path) {
        self.recent_vaults.retain(|known| known != path);
        self.forget_key_files_of_vaults_not_recent();
    }

    /// RF-111: a key file is remembered only for a recent vault.
    fn forget_key_files_of_vaults_not_recent(&mut self) {
        let recent = &self.recent_vaults;
        self.key_files
            .retain(|vault, _| recent.iter().any(|known| known == vault));
    }

    /// The recent vaults with whether each is still there (RF-20). `exists` is the question "is this path there?",
    /// injected so that the rule is tested without the disk; the app passes [`Path::exists`].
    #[must_use]
    pub fn recent_with_status(&self, exists: impl Fn(&Path) -> bool) -> Vec<RecentVault> {
        self.recent_vaults
            .iter()
            .map(|path| RecentVault {
                exists: exists(path),
                path: path.clone(),
            })
            .collect()
    }

    /// Reads the settings file. A missing, unreadable or damaged file gives the defaults (RF-82).
    #[must_use]
    pub fn load(path: &Path) -> Self {
        fs::read(path).map_or_else(|_| Self::default(), |bytes| Self::from_bytes(&bytes))
    }

    /// Writes the settings file: a temporary file next to it, synced, then renamed over it, so the old file is
    /// never left half written. Creates the folder if needed.
    ///
    /// # Errors
    /// `GuardadoFallido` if it cannot be written (decisión pendiente: the spec has no code for settings).
    pub fn save(&self, path: &Path) -> CoreResult<()> {
        let file_name = path.file_name().ok_or(CoreError::GuardadoFallido)?;
        if let Some(folder) = path
            .parent()
            .filter(|folder| !folder.as_os_str().is_empty())
        {
            fs::create_dir_all(folder).map_err(|_| CoreError::GuardadoFallido)?;
        }
        let mut temp_name = file_name.to_os_string();
        temp_name.push(".tmp");
        let temp = path.with_file_name(temp_name);
        let written = File::create(&temp).and_then(|mut file| {
            file.write_all(self.to_text().as_bytes())?;
            file.sync_all()
        });
        let replaced = written.and_then(|()| fs::rename(&temp, path));
        if replaced.is_err() {
            // Best effort: the original file is untouched and the error below says the save failed.
            drop(fs::remove_file(&temp));
            return Err(CoreError::GuardadoFallido);
        }
        Ok(())
    }
}

/// An unsigned whole number field; anything else (text, negative, decimal, missing) is `None`.
fn number(fields: &Map<String, Value>, key: &str) -> Option<u32> {
    fields
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
}
