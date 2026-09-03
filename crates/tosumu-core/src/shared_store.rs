//! Supported shared key/value owner and snapshot transaction boundary.
//!
//! ADR-0006 and ADR-0007 define this deliberately small synchronous API.
//! Physical pager, B+ tree, WAL, and registry types remain private.

use std::marker::PhantomData;
use std::path::Path;
use std::rc::Rc;
use std::sync::Arc;

use crate::btree::BTree;
use crate::error::{Result, TosumuError};
use crate::shared_owner::{
    ConditionalPutResult as CoreConditionalPutResult, ReadTransaction as CoreReadTransaction,
    SharedBTreeOwner,
};

/// Cloneable shared owner for one writable key/value database.
#[derive(Clone)]
pub struct SharedKvStore {
    owner: SharedBTreeOwner,
    version_scope: Arc<()>,
}

/// Owner-scoped database-generation token for optimistic writes.
///
/// The generation is durable, but this token is valid only for clones of the
/// live [`SharedKvStore`] that created it. It is not a per-key revision and is
/// intentionally not serializable across close/reopen.
#[derive(Clone)]
pub struct KvVersion {
    version_scope: Arc<()>,
    generation: u64,
}

/// A logical value or absence observed atomically with its database version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KvVersionedValue {
    /// Value at the observed generation, or `None` when the key was absent.
    pub value: Option<Vec<u8>>,
    /// Owner-scoped database generation observed with `value`.
    pub version: KvVersion,
}

/// Outcome of a completed conditional write.
#[derive(Debug, Clone, PartialEq, Eq)]
#[must_use = "conditional writes may leave the value unchanged"]
pub enum KvConditionalResult {
    /// The condition matched and the value was committed at this version.
    Applied(KvVersion),
    /// The condition did not match; no mutation or generation advance occurred.
    NotApplied(KvVersion),
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
            version_scope: Arc::new(()),
        })
    }

    /// Open an unencrypted shared database for writing and reads.
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            owner: SharedBTreeOwner::open(path)?,
            version_scope: Arc::new(()),
        })
    }

    /// Create a passphrase-protected shared database.
    pub fn create_encrypted(path: &Path, passphrase: &str) -> Result<Self> {
        Ok(Self {
            owner: SharedBTreeOwner::create_encrypted(path, passphrase)?,
            version_scope: Arc::new(()),
        })
    }

    /// Open a passphrase-protected shared database.
    pub fn open_with_passphrase(path: &Path, passphrase: &str) -> Result<Self> {
        Ok(Self {
            owner: SharedBTreeOwner::open_with_passphrase(path, passphrase)?,
            version_scope: Arc::new(()),
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

    /// Read a value or absence atomically with the current database version.
    pub fn get_with_version(&self, key: &[u8]) -> Result<KvVersionedValue> {
        let (value, generation) = self.owner.get_with_generation(key)?;
        Ok(KvVersionedValue {
            value,
            version: self.version(generation),
        })
    }

    /// Insert `value` only when `key` is currently absent.
    pub fn put_if_absent(&self, key: &[u8], value: &[u8]) -> Result<KvConditionalResult> {
        let result = self.owner.put_if_absent(key, value)?;
        Ok(self.conditional_result(result))
    }

    /// Replace `key` only when its current value exactly equals `expected`.
    ///
    /// Value equality alone does not prevent an ABA change. Use
    /// [`SharedKvStore::put_if_version`] when an intervening commit must reject
    /// the write even if the bytes later become equal again.
    pub fn compare_and_set(
        &self,
        key: &[u8],
        expected: &[u8],
        value: &[u8],
    ) -> Result<KvConditionalResult> {
        let result = self.owner.compare_and_set(key, expected, value)?;
        Ok(self.conditional_result(result))
    }

    /// Insert or replace `key` only when `expected` is this owner's current
    /// database version.
    ///
    /// Any intervening commit invalidates the token, including a commit to a
    /// different key. A token from another or reopened owner is rejected as an
    /// invalid argument.
    pub fn put_if_version(
        &self,
        key: &[u8],
        expected: &KvVersion,
        value: &[u8],
    ) -> Result<KvConditionalResult> {
        self.validate_version(expected)?;
        let result = self
            .owner
            .put_if_generation(key, expected.generation, value)?;
        Ok(self.conditional_result(result))
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

    fn version(&self, generation: u64) -> KvVersion {
        KvVersion {
            version_scope: Arc::clone(&self.version_scope),
            generation,
        }
    }

    fn validate_version(&self, version: &KvVersion) -> Result<()> {
        if Arc::ptr_eq(&self.version_scope, &version.version_scope) {
            Ok(())
        } else {
            Err(TosumuError::InvalidArgument(
                "version token belongs to a different or reopened shared KV store",
            ))
        }
    }

    fn conditional_result(&self, result: CoreConditionalPutResult) -> KvConditionalResult {
        let version = self.version(result.generation);
        if result.applied {
            KvConditionalResult::Applied(version)
        } else {
            KvConditionalResult::NotApplied(version)
        }
    }
}

impl KvVersion {
    /// Return the durable database commit generation represented by this token.
    pub fn generation(&self) -> u64 {
        self.generation
    }
}

impl std::fmt::Debug for KvVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("KvVersion")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl PartialEq for KvVersion {
    fn eq(&self, other: &Self) -> bool {
        self.generation == other.generation
            && Arc::ptr_eq(&self.version_scope, &other.version_scope)
    }
}

impl Eq for KvVersion {}

impl KvConditionalResult {
    /// Return whether the conditional mutation was committed.
    pub fn applied(&self) -> bool {
        matches!(self, Self::Applied(_))
    }

    /// Return the database version observed after the operation.
    pub fn version(&self) -> &KvVersion {
        match self {
            Self::Applied(version) | Self::NotApplied(version) => version,
        }
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

    /// Read an inclusive ordered key range from the transaction's staged view.
    ///
    /// Earlier puts and deletes in this same callback are reflected in the
    /// returned pairs. The scan is committed only if the callback succeeds.
    pub fn scan(&self, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.tree.scan_by_key(start, end)
    }
}
