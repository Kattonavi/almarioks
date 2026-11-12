//! AlmarioKS domain core.
//!
//! No Tauri and no network (constitution rule 8). What the system provides (clock, disk, clipboard) enters as a
//! parameter or a trait, so every rule can be tested without the real machine (`docs/specs/001-boveda/plan.md`).
//!
//! Dependency rule (ADR-0001): only [`kdbx`] names types of the `keepass` crate; the rest sees our own types.

#![forbid(unsafe_code)]

pub mod error;
pub mod generator;
pub mod kdbx;
pub mod search;
pub mod session;
pub mod settings;
pub mod storage;
pub mod strength;
pub mod text;
pub mod vault;

pub use error::{CoreError, CoreResult};

/// Version of this crate, shared by the whole workspace (`[workspace.package]`).
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
