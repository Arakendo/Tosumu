//! Opt-in prototypes whose names and shape are not compatibility contracts.
//!
//! Enable `experimental-shared-readers` to exercise the MVP+10 shared-reader
//! boundary. This module exists to gather independent caller evidence for
//! AR-0009. It may change or disappear before the final public API is admitted.

use std::path::Path;

use crate::error::Result;
use crate::shared_owner::{ReadTransaction as CoreReadTransaction, SharedBTreeOwner};

/// Experimental shared key/value database owner.
#[derive(Clone)]
pub struct SharedKvDatabase {
    owner: SharedBTreeOwner,
}

/// Experimental generation-pinned read transaction.
///
/// This value is `Send` but deliberately not `Sync`. It may move to another
/// thread, but one transaction cannot be shared for concurrent use.
pub struct ReadTransaction {
    inner: CoreReadTransaction,
}

/// Bounded process-local snapshot and checkpoint observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionInfo {
    pub active_readers: u64,
    pub maximum_readers: u64,
    pub oldest_reader_generation: Option<u64>,
    pub checkpoint_generation: u64,
    pub latest_generation: u64,
    pub retained_wal_bytes: u64,
    pub retained_frame_versions: u64,
    pub checkpoint_blocked: bool,
}

impl SharedKvDatabase {
    /// Create an unencrypted experimental shared database.
    pub fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            owner: SharedBTreeOwner::create(path)?,
        })
    }

    /// Open an unencrypted experimental shared database for writing and reads.
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            owner: SharedBTreeOwner::open(path)?,
        })
    }

    /// Insert or replace one logical value through the shared writer owner.
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.owner.put(key, value)
    }

    /// Read the latest committed logical value.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.owner.get(key)
    }

    /// Capture and pin the latest durable committed generation.
    pub fn snapshot(&self) -> Result<ReadTransaction> {
        Ok(ReadTransaction {
            inner: self.owner.read_transaction()?,
        })
    }

    /// Observe bounded process-local snapshot and retained-WAL pressure.
    pub fn connection_info(&self) -> Result<ConnectionInfo> {
        let info = self.owner.diagnostics()?;
        Ok(ConnectionInfo {
            active_readers: info.active,
            maximum_readers: info.maximum,
            oldest_reader_generation: info.oldest_generation,
            checkpoint_generation: info.checkpoint_generation,
            latest_generation: info.latest_generation,
            retained_wal_bytes: info.retained_wal_bytes,
            retained_frame_versions: info.retained_frame_versions,
            checkpoint_blocked: info.checkpoint_blocked,
        })
    }
}

impl ReadTransaction {
    /// Return the durable generation captured when this transaction opened.
    pub fn generation(&self) -> u64 {
        self.inner.generation()
    }

    /// Read one logical value from the captured generation.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.inner.get(key)
    }

    /// Read an inclusive ordered key range from the captured generation.
    pub fn scan(&self, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.inner.scan_by_key(start, end)
    }
}
