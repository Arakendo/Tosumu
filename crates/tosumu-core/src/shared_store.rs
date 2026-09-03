//! Supported shared key/value owner and snapshot transaction boundary.
//!
//! ADR-0006 defines this deliberately small synchronous API. Physical pager,
//! B+ tree, WAL, and registry types remain private.

use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;

use crate::btree::BTree;
use crate::error::Result;
use crate::shared_owner::{ReadTransaction as CoreReadTransaction, SharedBTreeOwner};

/// Cloneable shared owner for one writable key/value database.
#[derive(Clone)]
pub struct SharedKvStore {
    owner: SharedBTreeOwner,
}

/// Generation-pinned logical read transaction.
///
/// This value is `Send` but deliberately not `Sync`. It may move to another
/// thread, but one transaction cannot be shared for concurrent use.
pub struct KvReadTransaction {
    inner: CoreReadTransaction,
}

/// Exclusive write transaction passed to [`SharedKvStore::write`].
///
/// This borrowed value is deliberately neither `Send` nor `Sync`.
pub struct KvWriteTransaction<'a> {
    tree: &'a mut BTree,
    _not_send_or_sync: PhantomData<Rc<()>>,
}

/// Bounded process-local snapshot and checkpoint observations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct KvConnectionInfo {
    /// Number of currently registered read transactions.
    pub active_readers: u64,
    /// Maximum read transactions admitted by this owner.
    pub maximum_readers: u64,
    /// Oldest generation retained for an active reader, if any.
    pub oldest_reader_generation: Option<u64>,
    /// Generation represented by the checkpointed main file.
    pub checkpoint_generation: u64,
    /// Latest committed generation visible to current reads.
    pub latest_generation: u64,
    /// Encoded bytes currently resident in the WAL sidecar.
    pub retained_wal_bytes: u64,
    /// Committed page-frame versions currently retained in memory.
    pub retained_frame_versions: u64,
    /// Whether process-local readers currently suppress checkpointing.
    pub checkpoint_blocked: bool,
}

impl SharedKvStore {
    /// Create an unencrypted shared database.
    pub fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            owner: SharedBTreeOwner::create(path)?,
        })
    }

    /// Open an unencrypted shared database for writing and reads.
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            owner: SharedBTreeOwner::open(path)?,
        })
    }

    /// Create a passphrase-protected shared database.
    pub fn create_encrypted(path: &Path, passphrase: &str) -> Result<Self> {
        Ok(Self {
            owner: SharedBTreeOwner::create_encrypted(path, passphrase)?,
        })
    }

    /// Open a passphrase-protected shared database.
    pub fn open_with_passphrase(path: &Path, passphrase: &str) -> Result<Self> {
        Ok(Self {
            owner: SharedBTreeOwner::open_with_passphrase(path, passphrase)?,
        })
    }

    /// Insert or replace one logical value as one committed generation.
    pub fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.owner.put(key, value)
    }

    /// Delete one logical value as one committed generation.
    pub fn delete(&self, key: &[u8]) -> Result<()> {
        self.owner.delete(key)
    }

    /// Execute multiple logical mutations as one committed generation.
    ///
    /// Returning `Ok` commits the staged changes. Returning `Err` rolls them
    /// back and preserves the caller's error. Re-entering this same owner
    /// through a captured clone or snapshot returns `InvalidArgument`; use only
    /// the supplied transaction inside the callback. A panic publishes no
    /// staged WAL bytes, poisons the owner, and requires drop plus reopen.
    pub fn write<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut KvWriteTransaction<'_>) -> Result<T>,
    {
        self.owner.write(|tree| {
            operation(&mut KvWriteTransaction {
                tree,
                _not_send_or_sync: PhantomData,
            })
        })
    }

    /// Read the latest committed logical value.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.owner.get(key)
    }

    /// Capture and pin the latest durable committed generation.
    pub fn snapshot(&self) -> Result<KvReadTransaction> {
        Ok(KvReadTransaction {
            inner: self.owner.read_transaction()?,
        })
    }

    /// Observe bounded process-local snapshot and retained-WAL pressure.
    pub fn connection_info(&self) -> Result<KvConnectionInfo> {
        let info = self.owner.diagnostics()?;
        Ok(KvConnectionInfo {
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

impl KvReadTransaction {
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

impl KvWriteTransaction<'_> {
    /// Insert or replace one logical value in the active transaction.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.tree.put(key, value)
    }

    /// Delete one logical value in the active transaction.
    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.tree.delete(key)
    }

    /// Read the transaction's current staged logical view.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.tree.get(key)
    }
}
