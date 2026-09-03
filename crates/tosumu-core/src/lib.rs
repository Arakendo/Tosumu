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
pub mod inspection_session;
pub mod log_store;
pub mod page_store;
pub mod pager;
pub mod provider;
pub mod vacuum;
pub mod wal;

// ADR-0006 exposes logical shared ownership while the pager, B+ tree,
// registration, and retained-version machinery remain internal.
mod shared_owner;
mod shared_store;
#[allow(dead_code)]
mod snapshot_registry;
#[cfg_attr(not(unix), allow(dead_code))]
mod vacuum_publication;
mod vacuum_rebuild;
mod writer_gate;

#[cfg(test)]
pub(crate) mod test_helpers;

/// Compile-time project name. Used by the CLI and by log output.
pub const NAME: &str = "tosumu";

pub use error::{ErrorDetail, ErrorReport, ErrorStatus, ErrorValue, TosumuError};
pub use provider::{KvStore, KvTransaction, MAX_KEY_SIZE, MAX_VALUE_SIZE};
pub use shared_store::{
    KvConditionalResult, KvConnectionInfo, KvReadTransaction, KvVersion, KvVersionedValue,
    KvWriteTransaction, SharedKvStore,
};
