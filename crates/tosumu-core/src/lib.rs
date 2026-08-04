//! `tosumu-core` — core library for the tosumu embedded database.
//!
//! Pre-alpha. See `docs/Specifications/Tosumu Software Design Document.md` at the repository root for the source of truth.
#![forbid(unsafe_code)]

pub mod backup;
pub mod btree;
pub mod crypto;
pub mod error;
pub mod export;
pub mod format;
pub mod inspect;
pub mod log_store;
pub mod page_store;
pub mod pager;
pub mod provider;
pub mod wal;

#[cfg(test)]
pub(crate) mod test_helpers;

/// Compile-time project name. Used by the CLI and by log output.
pub const NAME: &str = "tosumu";

pub use error::{ErrorDetail, ErrorReport, ErrorStatus, ErrorValue, TosumuError};
pub use provider::{KvStore, KvTransaction, MAX_KEY_SIZE, MAX_VALUE_SIZE};
