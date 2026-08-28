use std::cell::Cell;
use std::marker::PhantomData;
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use crate::btree::BTree;
use crate::error::{Result, TosumuError};
use crate::pager::SnapshotDiagnostics;
use crate::snapshot_registry::SnapshotPin;

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

impl SharedBTreeOwner {
    pub(crate) fn create(path: &Path) -> Result<Self> {
        Ok(Self {
            state: Arc::new(Mutex::new(BTree::create(path)?)),
        })
    }

    pub(crate) fn put(&self, key: &[u8], value: &[u8]) -> Result<()> {
        self.lock()?.put(key, value)
    }

    pub(crate) fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        self.lock()?.get(key)
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
        self.state.lock().map_err(|_| TosumuError::Poisoned)
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
