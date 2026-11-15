//! Open a vault from its bytes (RF-10 to RF-13, RF-16, RF-94; detection for RF-14 and RF-17).
//!
//! Order: our header reader first (format, field sizes, KDF limits), then the library inside `catch_unwind` as a
//! second barrier, then the library's error mapped to a stable code by format version.

use std::{
    fmt,
    panic::{self, AssertUnwindSafe},
};

use keepass::{
    Database, DatabaseKey,
    db::DatabaseOpenError,
    error::{DatabaseKeyError, DatabaseVersionParseError},
};

use super::{
    header::{self, FormatVersion, HeaderInfo, KdfParams},
    key::VaultKey,
};
use crate::error::{CoreError, CoreResult};

/// A decrypted vault. The library's database stays inside; the rest of the core sees our own types.
pub struct OpenedVault {
    pub(crate) db: Database,
    header: HeaderInfo,
}

impl OpenedVault {
    /// What the header says: version and KDF.
    #[must_use]
    pub fn header(&self) -> HeaderInfo {
        self.header
    }

    /// File format version.
    #[must_use]
    pub fn version(&self) -> FormatVersion {
        self.header.version
    }

    /// KDBX 3.x: every change answers `SoloLectura` (RF-14, enforced by the vault in T4).
    #[must_use]
    pub fn is_read_only(&self) -> bool {
        self.header.is_read_only()
    }

    /// The vault's key derivation parameters.
    #[must_use]
    pub fn kdf(&self) -> KdfParams {
        self.header.kdf
    }

    /// The vault opens but its protection against brute force is weak (RF-17).
    #[must_use]
    pub fn is_kdf_weak(&self) -> bool {
        self.header.is_kdf_weak()
    }

    /// Number of entries in the vault, history not counted.
    #[must_use]
    pub fn entry_count(&self) -> usize {
        self.db.num_entries()
    }

    /// Number of groups in the vault, the root included.
    #[must_use]
    pub fn group_count(&self) -> usize {
        self.db.num_groups()
    }
}

impl fmt::Debug for OpenedVault {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Never the content: it holds the decrypted secrets.
        f.debug_struct("OpenedVault")
            .field("header", &self.header)
            .finish_non_exhaustive()
    }
}

/// Opens a vault with its key.
///
/// # Errors
/// - [`CoreError::FormatoNoSoportado`], [`CoreError::ArchivoDanado`], [`CoreError::ParametrosKdfExcesivos`]: from
///   the header, before any key is derived (see [`header::read_header`]).
/// - [`CoreError::LlaveIncorrecta`]: the key does not open it, whatever part is wrong (RF-11).
/// - [`CoreError::ArchivoDanado`]: the content does not verify, or the library failed in an unexpected way.
pub fn open(bytes: &[u8], key: &VaultKey) -> CoreResult<OpenedVault> {
    let header = header::parse(bytes)?.info;
    let db = parse_guarded(bytes, key.to_database_key()?, header.version)?;
    Ok(OpenedVault { db, header })
}

/// Calls the library and turns a panic into `ArchivoDanado`, so a file the header reader did not foresee cannot
/// take the app down (RF-94). Relies on `panic = "unwind"`, the default of every profile of this workspace.
pub(crate) fn parse_guarded(
    bytes: &[u8],
    key: DatabaseKey,
    version: FormatVersion,
) -> CoreResult<Database> {
    match panic::catch_unwind(AssertUnwindSafe(|| Database::parse(bytes, key))) {
        Ok(Ok(db)) => Ok(db),
        Ok(Err(error)) => Err(map_open_error(version, &error)),
        Err(_) => Err(CoreError::ArchivoDanado),
    }
}

/// Library error → stable code. The enums are `#[non_exhaustive]`: anything not listed is `ArchivoDanado`.
fn map_open_error(version: FormatVersion, error: &DatabaseOpenError) -> CoreError {
    // KDBX 3.x has no header HMAC: a wrong key shows up as a padding error of the last block, which is also what
    // a truncated 3.x file gives. The spec accepts that both say `LlaveIncorrecta` (spec "Casos límite").
    if version.major == 3 && matches!(error, DatabaseOpenError::Cryptography(_)) {
        return CoreError::LlaveIncorrecta;
    }
    match error {
        DatabaseOpenError::Key(DatabaseKeyError::IncorrectKey) => CoreError::LlaveIncorrecta,
        DatabaseOpenError::Key(DatabaseKeyError::EmptyKey) => CoreError::LlaveVacia,
        DatabaseOpenError::UnsupportedVersion
        | DatabaseOpenError::VersionParse(DatabaseVersionParseError::InvalidKDBXVersion {
            ..
        }) => CoreError::FormatoNoSoportado,
        _ => CoreError::ArchivoDanado,
    }
}
