//! Where a new vault is proposed to live (RF-97, RF-99): `Documents › AlmarioKS › <name>.kdbx`, with its key file
//! beside it.
//!
//! Pure rules over text and paths: nothing here reads or writes the disk, and the Documents folder enters as a
//! parameter (the app asks the system for it; the tests give a temporary one).
//!
//! # The file name of a vault, from its name ([`file_stem`])
//! In this order (`src/ipc/fake.ts` follows the same steps, and both are tested with the same table):
//! 1. The characters Windows does not take in a file name (`\ / : * ? " < > |`) and every control character
//!    (Unicode `Cc`, tab and line feed included) are removed.
//! 2. Spaces (`White_Space`) are trimmed at both ends, and so are the dots and spaces at the end, which Windows
//!    drops on its own.
//! 3. It is cut to [`MAX_FILE_STEM_CHARS`] characters, and the end is trimmed again.
//! 4. If what comes before its first dot is a device name of Windows (`CON`, `NUL`, `COM1`…), a `_` goes after it.
//! 5. If nothing is left, it is [`DEFAULT_FILE_STEM`].

use std::path::{Path, PathBuf};

/// The folder inside Documents where vaults are proposed (RF-97). The only folder the app ever creates.
pub const DEFAULT_FOLDER_NAME: &str = "AlmarioKS";
/// Extension of a vault file, with its dot.
pub const VAULT_EXTENSION: &str = ".kdbx";
/// Extension of a key file made by the app, with its dot (RF-5).
pub const KEY_FILE_EXTENSION: &str = ".keyx";
/// File name (without extension) of a vault whose name leaves nothing usable (RF-97).
pub const DEFAULT_FILE_STEM: &str = "bóveda";
/// Most characters (code points) of the file name taken from the vault's name, before the extension. A device
/// name gets one more (step 4). Far below the 255 UTF-16 units of a file name on Windows, so the whole path of a
/// vault in Documents stays short.
pub const MAX_FILE_STEM_CHARS: usize = 100;

/// Characters Windows does not take in a file name (RF-97).
const NOT_IN_A_FILE_NAME: [char; 9] = ['\\', '/', ':', '*', '?', '"', '<', '>', '|'];

/// Device names of Windows: a file may not be called like one of them, whatever its extension. Compared without
/// telling ASCII upper from lower case.
const DEVICE_NAMES: [&str; 32] = [
    "CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "COM0", "COM1", "COM2", "COM3", "COM4",
    "COM5", "COM6", "COM7", "COM8", "COM9", "COM¹", "COM²", "COM³", "LPT0", "LPT1", "LPT2", "LPT3",
    "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9", "LPT¹", "LPT²", "LPT³",
];

fn is_device_name(text: &str) -> bool {
    DEVICE_NAMES
        .iter()
        .any(|device| device.eq_ignore_ascii_case(text))
}

/// Drops the dots and spaces at the end: Windows does not keep them in a file name.
fn trim_end(text: &str) -> &str {
    text.trim_end_matches(|c: char| c == '.' || c.is_whitespace())
}

/// The file name (without extension) of a vault called `name`: see the steps in the [module](self). Never empty,
/// and always a single component that Windows accepts.
#[must_use]
pub fn file_stem(name: &str) -> String {
    let kept: String = name
        .chars()
        .filter(|c| !NOT_IN_A_FILE_NAME.contains(c) && !c.is_control())
        .collect();
    let cut: String = trim_end(kept.trim())
        .chars()
        .take(MAX_FILE_STEM_CHARS)
        .collect();
    let stem = trim_end(&cut);
    if stem.is_empty() {
        return DEFAULT_FILE_STEM.to_owned();
    }
    let (base, rest) = stem
        .split_once('.')
        .map_or((stem, None), |(base, rest)| (base, Some(rest)));
    if !is_device_name(base.trim_end()) {
        return stem.to_owned();
    }
    match rest {
        Some(rest) => format!("{base}_.{rest}"),
        None => format!("{base}_"),
    }
}

/// The file name proposed for a vault called `name`: [`file_stem`] and `.kdbx`.
#[must_use]
pub fn vault_file_name(name: &str) -> String {
    format!("{}{VAULT_EXTENSION}", file_stem(name))
}

/// The name of the key file that goes beside a vault file (RF-99): the same name with `.keyx` in the place of a
/// final `.kdbx` (in any case), or added to a name that does not end in `.kdbx`. Never the vault's own name.
#[must_use]
pub fn key_file_name(vault_file: &str) -> String {
    let split = vault_file
        .len()
        .checked_sub(VAULT_EXTENSION.len())
        .and_then(|at| vault_file.split_at_checked(at));
    let stem = match split {
        Some((stem, extension)) if extension.eq_ignore_ascii_case(VAULT_EXTENSION) => stem,
        Some(_) | None => vault_file,
    };
    format!("{stem}{KEY_FILE_EXTENSION}")
}

/// The folder where vaults are proposed: [`DEFAULT_FOLDER_NAME`] inside the person's Documents (RF-97).
#[must_use]
pub fn default_folder(documents: &Path) -> PathBuf {
    documents.join(DEFAULT_FOLDER_NAME)
}

/// Whether `vault_path` is a file directly inside the [default folder](default_folder) of `documents`.
///
/// The comparison is by path components, as written: nothing is resolved, so a path that only gets there through
/// `..`, a subfolder or another spelling of the same folder is not "in" it. This is what decides whether the app
/// may create the folder (RF-97), so it errs on the side of "no".
#[must_use]
pub fn is_in_default_folder(documents: &Path, vault_path: &Path) -> bool {
    vault_path.file_name().is_some() && vault_path.parent() == Some(&default_folder(documents))
}

/// Where a new vault and the key file beside it would go (RF-97, RF-99). Nothing is created by building one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VaultLocation {
    /// The vault file.
    pub vault_path: PathBuf,
    /// Its file name, with extension.
    pub vault_file: String,
    /// The key file beside it: same folder, same name, `.keyx`.
    pub key_file_path: PathBuf,
    /// Its file name, with extension.
    pub key_file: String,
    /// Whether the vault goes directly inside `Documents › AlmarioKS`.
    pub in_default_folder: bool,
}

impl VaultLocation {
    /// The location proposed for a vault called `name` (RF-97): [`vault_file_name`] inside the
    /// [default folder](default_folder) of `documents`.
    #[must_use]
    pub fn proposed(documents: &Path, name: &str) -> Self {
        let folder = default_folder(documents);
        let vault_file = vault_file_name(name);
        let key_file = key_file_name(&vault_file);
        Self {
            vault_path: folder.join(&vault_file),
            key_file_path: folder.join(&key_file),
            vault_file,
            key_file,
            in_default_folder: true,
        }
    }

    /// The location the person chose themselves: `vault_path` as it is, and its key file beside it (RF-99).
    /// `documents` is only for [`Self::in_default_folder`]; without it the answer is "no".
    ///
    /// `None` if the path is not valid text or does not end in a file name: a root, a path that ends in a
    /// separator, in `.` or in `..`, or a name with a drive and no folder (`C:name`).
    #[must_use]
    pub fn chosen(documents: Option<&Path>, vault_path: &Path) -> Option<Self> {
        let vault_file = vault_path.file_name()?.to_str()?.to_owned();
        // `Path` reads `folder/` and `folder/.` as the file `folder`: only what is written after the last
        // separator counts as the file name, so the path answered and the name shown are the same thing.
        let written = vault_path.to_str()?.rsplit(std::path::is_separator).next();
        if written != Some(vault_file.as_str()) {
            return None;
        }
        let key_file = key_file_name(&vault_file);
        Some(Self {
            vault_path: vault_path.to_path_buf(),
            key_file_path: vault_path.with_file_name(&key_file),
            vault_file,
            key_file,
            in_default_folder: documents
                .is_some_and(|documents| is_in_default_folder(documents, vault_path)),
        })
    }
}
