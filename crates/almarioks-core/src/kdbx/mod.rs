//! KDBX wrappers (ADR-0001): open, create, KDF, key files and the KDBX 3.x policy.
//!
//! The only module that names types of the `keepass` crate, and the only one allowed to use the hashing and
//! random crates (constitution rule 4); nothing of those crates leaves it. The API takes bytes and keys: reading
//! and writing the vault file is `storage` (T8) and the session is `vault` (T9).

mod create;
mod header;
mod key;
mod open;

pub use create::{
    CALIBRATION_TARGET, MAX_NEW_VAULT_ITERATIONS, MIN_NEW_VAULT_ITERATIONS, NEW_VAULT_MEMORY_BYTES,
    NEW_VAULT_PARALLELISM, NewKeyFile, NewVaultKdf, create_vault, create_vault_file,
    measure_unlock,
};
pub use header::{
    Argon2Variant, FormatVersion, HeaderInfo, KdfParams, MAX_AES_KDF_ROUNDS, MAX_ARGON2_ITERATIONS,
    MAX_ARGON2_PARALLELISM, MAX_KDF_MEMORY_BYTES, STRONG_KDF_MIN_MEMORY_BYTES, read_header,
};
pub use key::{KeyFile, MasterPassword, VaultKey};
pub use open::{OpenedVault, open};

// Operations on an opened vault (T5 to T7): methods of `OpenedVault` over the types of `crate::vault`.
mod entries;
mod groups;
mod read;
pub use create::{
    DEFAULT_VAULT_NAME, NEW_VAULT_HISTORY_MAX_ITEMS, NEW_VAULT_HISTORY_MAX_SIZE,
    create_named_vault_file,
};
pub use groups::RECYCLE_BIN_NAME;

// KDBX 3.x to KDBX 4 (T4, RF-15).
pub use create::{convert_to_kdbx4, convert_vault_file};

// Search (T10): a method of `OpenedVault` over the types of `crate::search`.
mod find;

#[cfg(test)]
mod tests_entries;
#[cfg(test)]
mod tests_groups;
#[cfg(test)]
mod tests_read;

#[cfg(test)]
mod tests;

#[cfg(test)]
mod tests_search;

#[cfg(test)]
mod tests_convert;

#[cfg(test)]
mod tests_roundtrip_edit;
