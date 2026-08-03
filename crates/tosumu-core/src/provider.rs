//! Consumer-facing key/value provider boundary.
//!
//! This module intentionally exposes logical key/value operations only. Storage
//! implementation types such as `Pager`, `BTree`, WAL records, and page frames
//! remain behind the provider boundary.

use std::path::Path;

use crate::error::Result;
use crate::page_store::{PageStore, StoreStat};

/// Maximum key size accepted by the current physical record format.
pub const MAX_KEY_SIZE: usize = u16::MAX as usize;

/// Maximum value size accepted by the current physical record format.
pub const MAX_VALUE_SIZE: usize = crate::format::MAX_VALUE_SIZE;

/// Admitted embeddable key/value provider for consumer adapters.
///
/// ```no_run
/// use std::path::Path;
/// use tosumu_core::KvStore;
///
/// # fn main() -> Result<(), tosumu_core::TosumuError> {
/// let mut store = KvStore::create(Path::new("asset.tsm"))?;
/// store.transaction(|transaction| {
///     transaction.put(b"manifest", b"schema-v1")?;
///     transaction.put(b"payload", &[0, 1, 2, 255])?;
///     Ok(())
/// })?;
/// # Ok(())
/// # }
/// ```
pub struct KvStore {
    store: PageStore,
}

impl KvStore {
    /// Create a new unencrypted store at `path`.
    pub fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            store: PageStore::create(path)?,
        })
    }

    /// Open an existing unencrypted store at `path`.
    pub fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            store: PageStore::open(path)?,
        })
    }

    /// Open an existing store in read-only mode.
    pub fn open_readonly(path: &Path) -> Result<Self> {
        Ok(Self {
            store: PageStore::open_readonly(path)?,
        })
    }

    /// Insert or replace one logical value.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.store.put(key, value)
    }

    /// Delete one logical value. Deleting a missing key is a no-op.
    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.store.delete(key)
    }

    /// Read one logical value, returning `None` when the key is absent.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.store.get(key)
    }

    /// Read all logical values in key order.
    pub fn scan(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.store.scan()
    }

    /// Return provider statistics without exposing physical storage types.
    pub fn stat(&self) -> Result<StoreStat> {
        self.store.stat()
    }

    /// Commit multiple logical writes atomically.
    ///
    /// If the closure returns an error, all writes made through `transaction`
    /// are rolled back and the error is returned unchanged.
    pub fn transaction<F, T>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce(&mut KvTransaction<'_>) -> Result<T>,
    {
        self.store
            .transaction(|store| f(&mut KvTransaction { store }))
    }
}

/// Borrowed transaction handle exposed to the provider callback.
pub struct KvTransaction<'a> {
    store: &'a mut PageStore,
}

impl KvTransaction<'_> {
    /// Insert or replace one logical value in the active transaction.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        self.store.put(key, value)
    }

    /// Delete one logical value in the active transaction.
    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        self.store.delete(key)
    }
}