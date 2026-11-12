//! Vault in memory (T5 to T7, T9): the session state and the operations on entries and groups, on our own types.
//!
//! The types of the content live here ([`model`]); the operations that read or change a decrypted vault are
//! methods of [`crate::kdbx::OpenedVault`], because only `kdbx` may name `keepass` types (ADR-0001).

mod model;
pub use model::*;

/// Where the session is (ADR-0002). `Open` and `Locked` get their data in T9.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VaultState {
    /// No vault chosen.
    #[default]
    Closed,
    /// A vault is chosen but its keys are not in memory.
    Locked,
    /// The vault is decrypted in memory.
    Open,
}
