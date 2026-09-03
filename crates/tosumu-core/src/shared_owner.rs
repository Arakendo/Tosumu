use std::cell::Cell;
use std::cell::RefCell;
use std::collections::HashSet;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::btree::BTree;
use crate::error::{Result, TosumuError};
use crate::pager::SnapshotDiagnostics;
use crate::snapshot_registry::SnapshotPin;

thread_local! {
    static ACTIVE_WRITE_CALLBACKS: RefCell<HashSet<usize>> = RefCell::new(HashSet::new());
}

const REENTRANT_WRITE_MESSAGE: &str =
    "shared database owner cannot be re-entered from its write callback";

/// Private executable ownership model for AR-0009.
///
/// Capturing a snapshot and publishing a commit both pass through `state`, so
/// the captured generation cannot fall inside a commit. Read operations take
/// the mutex only while resolving one logical operation; their pin survives
/// independently and keeps the required WAL versions resident.
#[derive(Clone)]
pub(crate) struct SharedBTreeOwner {
    state: Arc<Mutex<BTree>>,
}

pub(crate) struct ReadTransaction {
    // Fields drop in declaration order. Release the registration before the
    // last shared owner can release its pager and writer guard.
    pin: SnapshotPin,
    state: Arc<Mutex<BTree>>,
    // The SDD requires read transactions to be movable but not shareable.
    // Their logical snapshot is exclusive per use even though the database
    // owner itself is shared.
    _not_sync: PhantomData<Cell<()>>,
}

pub(crate) struct ConditionalPutResult {
    pub(crate) applied: bool,
    pub(crate) generation: u64,
}

struct WriteCallbackScope {
    owner_id: usize,
}

impl SharedBTreeOwner {
    pub(crate) fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(BTree::create(path)?)),
        })
    }

    pub(crate) fn open(path: &Path) -> Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(BTree::open(path)?)),
        })
    }

    pub(crate) fn create_encrypted(path: &Path, passphrase: &str) -> Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(BTree::create_encrypted(path, passphrase)?)),
        })
    }

    pub(crate) fn open_with_passphrase(path: &Path, passphrase: &str) -> Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(BTree::open_with_passphrase(path, passphrase)?)),
        })
    }

    pub(crate) fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.lock()?.put(key, value)
    }

    pub(crate) fn delete(&self, key: &[u8]) -> Result<()> {
        self.lock()?.delete(key)
    }

    pub(crate) fn write<F, T>(&self, operation: F) -> Result<T>
    where
        F: FnOnce(&mut BTree) -> Result<T>,
    {
        let mut tree = self.lock()?;
        tree.begin_txn()?;
        let _scope = self.enter_write_callback();
        match operation(&mut tree) {
            Ok(value) => {
                tree.commit_txn()?;
                Ok(value)
            }
            Err(error) => {
                tree.rollback_txn();
                Err(error)
            }
        }
    }

    pub(crate) fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.lock()?.get(key)
    }

    pub(crate) fn get_with_generation(&self, key: &[u8]) -> Result<(Option<Vec<u8>>, u64)> {
        let tree = self.lock()?;
        let value = tree.get(key)?;
        Ok((value, tree.current_generation()?))
    }

    pub(crate) fn put_if_absent(&self, key: &[u8], value: &[u8]) -> Result<ConditionalPutResult> {
        self.conditional_put(key, value, |current, _| current.is_none())
    }

    pub(crate) fn compare_and_set(
        &self,
        key: &[u8],
        expected: &[u8],
        value: &[u8],
    ) -> Result<ConditionalPutResult> {
        self.conditional_put(key, value, |current, _| current == Some(expected))
    }

    pub(crate) fn put_if_generation(
        &self,
        key: &[u8],
        expected_generation: u64,
        value: &[u8],
    ) -> Result<ConditionalPutResult> {
        self.conditional_put(key, value, |_, generation| {
            generation == expected_generation
        })
    }

    pub(crate) fn read_transaction(&self) -> Result<ReadTransaction> {
        let pin = self.lock()?.pin_snapshot()?;
        Ok(ReadTransaction {
            pin,
            state: Arc::clone(&self.state),
            _not_sync: PhantomData,
        })
    }

    pub(crate) fn diagnostics(&self) -> Result<SnapshotDiagnostics> {
        self.lock()?.snapshot_diagnostics()
    }

    fn lock(&self) -> Result<MutexGuard<'_, BTree>> {
        if self.write_callback_is_active() {
            return Err(TosumuError::InvalidArgument(REENTRANT_WRITE_MESSAGE));
        }
        self.state.lock().map_err(|_| TosumuError::Poisoned)
    }

    fn conditional_put<F>(
        &self,
        key: &[u8],
        value: &[u8],
        condition: F,
    ) -> Result<ConditionalPutResult>
    where
        F: FnOnce(Option<&[u8]>, u64) -> bool,
    {
        let mut tree = self.lock()?;
        let current = tree.get(key)?;
        let generation = tree.current_generation()?;
        if !condition(current.as_deref(), generation) {
            return Ok(ConditionalPutResult {
                applied: false,
                generation,
            });
        }

        tree.begin_txn()?;
        if let Err(error) = tree.put(key, value) {
            tree.rollback_txn();
            return Err(error);
        }
        tree.commit_txn()?;
        Ok(ConditionalPutResult {
            applied: true,
            generation: tree.current_generation()?,
        })
    }

    fn owner_id(&self) -> usize {
        Arc::as_ptr(&self.state) as usize
    }

    fn write_callback_is_active(&self) -> bool {
        let owner_id = self.owner_id();
        ACTIVE_WRITE_CALLBACKS.with(|active| active.borrow().contains(&owner_id))
    }

    fn enter_write_callback(&self) -> WriteCallbackScope {
        let owner_id = self.owner_id();
        ACTIVE_WRITE_CALLBACKS.with(|active| {
            active.borrow_mut().insert(owner_id);
        });
        WriteCallbackScope { owner_id }
    }
}

impl Drop for WriteCallbackScope {
    fn drop(&mut self) {
        ACTIVE_WRITE_CALLBACKS.with(|active| {
            active.borrow_mut().remove(&self.owner_id);
        });
    }
}

impl ReadTransaction {
    pub(crate) fn generation(&self) -> u64 {
        self.pin.generation()
    }

    pub(crate) fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.lock()?.get_at_snapshot(&self.pin, key)
    }

    pub(crate) fn scan_by_key(&self, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.lock()?.scan_at_snapshot(&self.pin, start, end)
    }

    fn lock(&self) -> Result<MutexGuard<'_, BTree>> {
        let owner_id = Arc::as_ptr(&self.state) as usize;
        if ACTIVE_WRITE_CALLBACKS.with(|active| active.borrow().contains(&owner_id)) {
            return Err(TosumuError::InvalidArgument(REENTRANT_WRITE_MESSAGE));
        }
        self.state.lock().map_err(|_| TosumuError::Poisoned)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_send<T: Send>() {}
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn pins_repeatable_reads_while_a_shared_writer_commits() {
        assert_send_sync::<SharedBTreeOwner>();
        assert_send::<ReadTransaction>();

        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("shared-owner.tsm");
        let owner = SharedBTreeOwner::create(&path).unwrap();
        owner.put(b"a", b"old").unwrap();
        owner.put(b"b", b"stable").unwrap();

        let reader = owner.read_transaction().unwrap();
        let captured_generation = reader.generation();
        assert_eq!(reader.get(b"a").unwrap(), Some(b"old".to_vec()));

        let writer = owner.clone();
        std::thread::spawn(move || {
            writer.put(b"a", b"new").unwrap();
            writer.put(b"c", b"later").unwrap();
        })
        .join()
        .unwrap();

        assert_eq!(owner.get(b"a").unwrap(), Some(b"new".to_vec()));
        assert_eq!(reader.get(b"a").unwrap(), Some(b"old".to_vec()));
        assert_eq!(
            reader.scan_by_key(b"a", b"z").unwrap(),
            vec![
                (b"a".to_vec(), b"old".to_vec()),
                (b"b".to_vec(), b"stable".to_vec()),
            ]
        );

        let pinned = owner.diagnostics().unwrap();
        assert_eq!(pinned.active, 1);
        assert_eq!(pinned.maximum, 64);
        assert_eq!(pinned.oldest_generation, Some(captured_generation));
        assert!(pinned.latest_generation > captured_generation);
        assert!(pinned.checkpoint_generation < pinned.latest_generation);
        assert!(pinned.retained_wal_bytes > 0);
        assert!(pinned.retained_frame_versions > 0);
        assert!(pinned.checkpoint_blocked);

        drop(reader);
        let released = owner.diagnostics().unwrap();
        assert_eq!(released.active, 0);
        assert_eq!(released.oldest_generation, None);
        assert!(!released.checkpoint_blocked);
        assert!(released.retained_wal_bytes > 0);

        // Reader drop is passive. The next ordinary zero-reader commit performs
        // the already-defined full checkpoint and WAL truncation.
        owner.put(b"d", b"checkpoint-trigger").unwrap();
        let checkpointed = owner.diagnostics().unwrap();
        assert_eq!(
            checkpointed.checkpoint_generation,
            checkpointed.latest_generation
        );
        assert_eq!(checkpointed.retained_wal_bytes, 0);
        assert_eq!(checkpointed.retained_frame_versions, 0);
    }

    #[test]
    fn last_read_transaction_keeps_owner_and_writer_gate_alive_until_drop() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("reader-owned-lifetime.tsm");
        let owner = SharedBTreeOwner::create(&path).unwrap();
        owner.put(b"key", b"captured").unwrap();

        let reader = owner.read_transaction().unwrap();
        owner.put(b"key", b"committed-later").unwrap();
        drop(owner);

        assert!(matches!(
            BTree::open(&path),
            Err(TosumuError::FileBusy { .. })
        ));
        assert_eq!(reader.get(b"key").unwrap(), Some(b"captured".to_vec()));

        drop(reader);
        let reopened = BTree::open(&path).unwrap();
        assert_eq!(
            reopened.get(b"key").unwrap(),
            Some(b"committed-later".to_vec())
        );
    }
}
