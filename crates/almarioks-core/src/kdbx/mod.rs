//! KDBX wrappers (ADR-0001): open, create, KDF, key files and the KDBX 3.x policy.
//!
//! The only module that names types of the `keepass` crate, and the only one allowed to use the hashing and
//! random crates (constitution rule 4); nothing of those crates leaves it. The API takes bytes and keys: reading
//! and writing the vault file is `storage` (T8) and the session is `vault` (T9).

mod header;
mod key;
mod open;

pub use header::{
    Argon2Variant, FormatVersion, HeaderInfo, KdfParams, MAX_AES_KDF_ROUNDS, MAX_ARGON2_ITERATIONS,
    MAX_ARGON2_PARALLELISM, MAX_KDF_MEMORY_BYTES, STRONG_KDF_MIN_MEMORY_BYTES, read_header,
};
pub use key::{KeyFile, MasterPassword, VaultKey};
pub use open::{OpenedVault, open};

#[cfg(test)]
mod tests;
