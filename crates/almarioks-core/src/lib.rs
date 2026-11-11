//! AlmarioKS domain core.
//!
//! No Tauri and no network (constitution rule 8). For now it only exposes its version: the KDBX wrappers
//! (history order, KDF, key file, KDBX 3.1 policy) arrive with release 0.1.

/// Version of this crate, shared by the whole workspace (`[workspace.package]`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
