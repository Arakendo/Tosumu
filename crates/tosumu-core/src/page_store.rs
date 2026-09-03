// PageStore — put/get/delete/scan backed by the B+ tree.
//
// Source of truth: docs/Specifications/Tosumu Software Design Document.md §12.0 (MVP +3).
//
// PageStore is a thin facade over BTree. The B+ tree handles page
// allocation, splitting, and sorted leaf-chain iteration.
// The public API is unchanged from MVP +1 so all existing tests pass.
//
// Record encoding inside slotted pages:
//   Live record:  [0x01: u8][key_len: u16 LE][val_len: u16 LE][key...][val...]
//   Tombstone:    [0x02: u8][key_len: u16 LE][key...]
//
// Slot entry: { offset: u16 LE, length: u16 LE } — 4 bytes per slot.
// Offsets are relative to the start of the decrypted page body (0..PAGE_PLAINTEXT_SIZE).
//
// ── Read-path semantics ──────────────────────────────────────────────────────
//
// Scan / get correctness relies on two ordering invariants (see btree.rs):
//
//   1. Page-order = write-order: records are always appended; pages are
//      always scanned in ascending pgno order.  A later write for the same
//      key always lands on the same or a higher pgno.  Last-write-wins is
//      therefore equivalent to last-pgno-wins.
//
//   2. Slot-order = write-order within a page: within a single leaf page,
//      slots are appended; a later slot for the same key (live or tombstone)
//      is always at a higher slot index.
//
// These invariants must be preserved if freelist reuse or compaction is
// added in future stages — any violation makes get/scan silently incorrect.

use std::path::Path;

use crate::btree::BTree;
use crate::error::{Result, TosumuError};
use crate::pager::{OpenUnlock, Pager, RebuildContext};
use crate::writer_gate::WriterGuard;

/// High-level key-value store backed by the B+ tree.
pub struct PageStore {
    tree: BTree,
}

/// Summary information about the store. Returned by `stat()`.
pub struct StoreStat {
    pub page_count: u64,
    pub data_pages: u64,
    /// Height of the B+ tree (1 = root is a single leaf).
    pub tree_height: usize,
}

impl PageStore {
    // ── Construction ─────────────────────────────────────────────────────────

    /// Create a new `.tsm` file. Fails if `path` already exists.
    pub fn create(path: &Path) -> Result<Self> {
        Ok(PageStore {
            tree: BTree::create(path)?,
        })
    }

    pub(crate) fn open_with_writer_guard(
        path: &Path,
        unlock: OpenUnlock<'_>,
        writer_guard: &WriterGuard,
    ) -> Result<Self> {
        Ok(Self {
            tree: BTree::open_with_writer_guard(path, unlock, writer_guard)?,
        })
    }

    pub(crate) fn create_rebuild_staging(path: &Path, context: &RebuildContext) -> Result<Self> {
        Ok(Self {
            tree: BTree::create_rebuild_staging(path, context)?,
        })
    }

    pub(crate) fn rebuild_context(&mut self) -> Result<RebuildContext> {
        self.tree.rebuild_context()
    }

    /// Open an existing `.tsm` file.
    pub fn open(path: &Path) -> Result<Self> {
        Ok(PageStore {
            tree: BTree::open(path)?,
        })
    }

    /// Open an existing `.tsm` file in read-only mode.
    pub fn open_readonly(path: &Path) -> Result<Self> {
        Ok(PageStore {
            tree: BTree::open_readonly(path)?,
        })
    }

    /// Create a new passphrase-protected `.tsm` file.
    pub fn create_encrypted(path: &Path, passphrase: &str) -> Result<Self> {
        Ok(PageStore {
            tree: BTree::create_encrypted(path, passphrase)?,
        })
    }

    /// Open a passphrase-protected `.tsm` file.
    pub fn open_with_passphrase(path: &Path, passphrase: &str) -> Result<Self> {
        Ok(PageStore {
            tree: BTree::open_with_passphrase(path, passphrase)?,
        })
    }

    /// Open a passphrase-protected `.tsm` file in read-only mode.
    pub fn open_with_passphrase_readonly(path: &Path, passphrase: &str) -> Result<Self> {
        Ok(PageStore {
            tree: BTree::open_with_passphrase_readonly(path, passphrase)?,
        })
    }

    /// Open a recovery-key-protected `.tsm` file.
    pub fn open_with_recovery_key(path: &Path, recovery_str: &str) -> Result<Self> {
        Ok(PageStore {
            tree: BTree::open_with_recovery_key(path, recovery_str)?,
        })
    }

    /// Open a recovery-key-protected `.tsm` file in read-only mode.
    pub fn open_with_recovery_key_readonly(path: &Path, recovery_str: &str) -> Result<Self> {
        Ok(PageStore {
            tree: BTree::open_with_recovery_key_readonly(path, recovery_str)?,
        })
    }

    /// Open a keyfile-protected `.tsm` file.
    pub fn open_with_keyfile(path: &Path, keyfile_path: &Path) -> Result<Self> {
        Ok(PageStore {
            tree: BTree::open_with_keyfile(path, keyfile_path)?,
        })
    }

    /// Open a keyfile-protected `.tsm` file in read-only mode.
    pub fn open_with_keyfile_readonly(path: &Path, keyfile_path: &Path) -> Result<Self> {
        Ok(PageStore {
            tree: BTree::open_with_keyfile_readonly(path, keyfile_path)?,
        })
    }

    // ── Key management ───────────────────────────────────────────────────────

    /// Add a passphrase protector. Returns the slot index used.
    pub fn add_passphrase_protector(
        path: &Path,
        unlock_passphrase: &str,
        new_passphrase: &str,
    ) -> Result<u16> {
        Pager::add_passphrase_protector(path, unlock_passphrase, new_passphrase)
    }

    /// Add a passphrase protector, unlocking the DEK with a recovery key.
    pub fn add_passphrase_protector_with_recovery_key(
        path: &Path,
        recovery_str: &str,
        new_passphrase: &str,
    ) -> Result<u16> {
        Pager::add_passphrase_protector_with_recovery_key(path, recovery_str, new_passphrase)
    }

    /// Add a passphrase protector, unlocking the DEK with a keyfile protector.
    pub fn add_passphrase_protector_with_keyfile(
        path: &Path,
        keyfile_path: &Path,
        new_passphrase: &str,
    ) -> Result<u16> {
        Pager::add_passphrase_protector_with_keyfile(path, keyfile_path, new_passphrase)
    }

    /// Add a recovery-key protector. Returns the one-time recovery string.
    pub fn add_recovery_key_protector(path: &Path, unlock_passphrase: &str) -> Result<String> {
        Pager::add_recovery_key_protector(path, unlock_passphrase)
    }

    /// Add a recovery-key protector, unlocking the DEK with an existing recovery key.
    pub fn add_recovery_key_protector_with_recovery_key(
        path: &Path,
        recovery_str: &str,
    ) -> Result<String> {
        Pager::add_recovery_key_protector_with_recovery_key(path, recovery_str)
    }

    /// Add a recovery-key protector, unlocking the DEK with a keyfile protector.
    pub fn add_recovery_key_protector_with_keyfile(
        path: &Path,
        keyfile_path: &Path,
    ) -> Result<String> {
        Pager::add_recovery_key_protector_with_keyfile(path, keyfile_path)
    }

    /// Add a recovery-key protector using a caller-supplied recovery string.
    pub fn add_recovery_key_protector_with_secret(
        path: &Path,
        unlock_passphrase: &str,
        recovery_str: &str,
    ) -> Result<()> {
        Pager::add_recovery_key_protector_with_secret(path, unlock_passphrase, recovery_str)
    }

    /// Add a recovery-key protector using an existing recovery key and caller-supplied secret.
    pub fn add_recovery_key_protector_with_recovery_key_and_secret(
        path: &Path,
        recovery_str: &str,
        new_recovery_str: &str,
    ) -> Result<()> {
        Pager::add_recovery_key_protector_with_recovery_key_and_secret(
            path,
            recovery_str,
            new_recovery_str,
        )
    }

    /// Add a recovery-key protector using a keyfile unlock and caller-supplied secret.
    pub fn add_recovery_key_protector_with_keyfile_and_secret(
        path: &Path,
        keyfile_path: &Path,
        recovery_str: &str,
    ) -> Result<()> {
        Pager::add_recovery_key_protector_with_keyfile_and_secret(path, keyfile_path, recovery_str)
    }

    /// Add a keyfile protector. Returns the slot index used.
    pub fn add_keyfile_protector(
        path: &Path,
        unlock_passphrase: &str,
        keyfile_path: &Path,
    ) -> Result<u16> {
        Pager::add_keyfile_protector(path, unlock_passphrase, keyfile_path)
    }

    /// Add a keyfile protector, unlocking with an existing recovery key.
    pub fn add_keyfile_protector_with_recovery_key(
        path: &Path,
        recovery_str: &str,
        keyfile_path: &Path,
    ) -> Result<u16> {
        Pager::add_keyfile_protector_with_recovery_key(path, recovery_str, keyfile_path)
    }

    /// Add a keyfile protector, unlocking with another keyfile protector.
    pub fn add_keyfile_protector_with_keyfile(
        path: &Path,
        unlock_keyfile_path: &Path,
        keyfile_path: &Path,
    ) -> Result<u16> {
        Pager::add_keyfile_protector_with_keyfile(path, unlock_keyfile_path, keyfile_path)
    }

    /// Remove the keyslot at `slot_idx` (must not be the last active slot).
    pub fn remove_keyslot(path: &Path, unlock_passphrase: &str, slot_idx: u16) -> Result<()> {
        Pager::remove_keyslot(path, unlock_passphrase, slot_idx)
    }

    /// Remove a keyslot, unlocking the DEK with a recovery key.
    pub fn remove_keyslot_with_recovery_key(
        path: &Path,
        recovery_str: &str,
        slot_idx: u16,
    ) -> Result<()> {
        Pager::remove_keyslot_with_recovery_key(path, recovery_str, slot_idx)
    }

    /// Remove a keyslot, unlocking the DEK with a keyfile protector.
    pub fn remove_keyslot_with_keyfile(
        path: &Path,
        keyfile_path: &Path,
        slot_idx: u16,
    ) -> Result<()> {
        Pager::remove_keyslot_with_keyfile(path, keyfile_path, slot_idx)
    }

    /// Rotate the KEK for the Passphrase slot at `slot_idx`.
    pub fn rekey_kek(
        path: &Path,
        slot_idx: u16,
        old_passphrase: &str,
        new_passphrase: &str,
    ) -> Result<()> {
        Pager::rekey_kek(path, slot_idx, old_passphrase, new_passphrase)
    }

    /// Rotate a Passphrase slot using a recovery key to unlock the DEK.
    pub fn rekey_kek_with_recovery_key(
        path: &Path,
        slot_idx: u16,
        recovery_str: &str,
        new_passphrase: &str,
    ) -> Result<()> {
        Pager::rekey_kek_with_recovery_key(path, slot_idx, recovery_str, new_passphrase)
    }

    /// Rotate a Passphrase slot using a keyfile protector to unlock the DEK.
    pub fn rekey_kek_with_keyfile(
        path: &Path,
        slot_idx: u16,
        keyfile_path: &Path,
        new_passphrase: &str,
    ) -> Result<()> {
        Pager::rekey_kek_with_keyfile(path, slot_idx, keyfile_path, new_passphrase)
    }

    /// List active keyslots. Returns `Vec<(slot_index, kind_byte)>`.
    pub fn list_keyslots(path: &Path) -> Result<Vec<(u16, u8)>> {
        Pager::list_keyslots(path)
    }

    // ── Writes ───────────────────────────────────────────────────────────────

    /// Insert or update a key-value pair.
    pub fn put(&mut self, key: &[u8], value: &[u8]) -> Result<()> {
        validate_key(key)?;
        validate_value(value)?;
        self.tree.put(key, value)
    }

    /// Delete a key. No-op if the key does not exist.
    pub fn delete(&mut self, key: &[u8]) -> Result<()> {
        validate_key(key)?;
        self.tree.delete(key)
    }

    // ── Reads ─────────────────────────────────────────────────────────────────

    /// Retrieve the current value for `key`, or `None` if not present.
    pub fn get(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        validate_key(key)?;
        self.tree.get(key)
    }

    /// Return all live key-value pairs, sorted by key.
    pub fn scan(&self) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        self.tree.scan_physical()
    }

    /// Return all live key-value pairs where `start <= key <= end`, sorted by key.
    pub fn scan_range(&self, start: &[u8], end: &[u8]) -> Result<Vec<(Vec<u8>, Vec<u8>)>> {
        if start.is_empty() {
            return Err(TosumuError::InvalidArgument("start key must not be empty"));
        }
        if end.is_empty() {
            return Err(TosumuError::InvalidArgument("end key must not be empty"));
        }
        self.tree.scan_by_key(start, end)
    }

    /// Return summary statistics.
    pub fn stat(&self) -> Result<StoreStat> {
        let page_count = self.tree.page_count();
        Ok(StoreStat {
            page_count,
            data_pages: page_count.saturating_sub(1),
            tree_height: self.tree.tree_height()?,
        })
    }

    /// Execute a write transaction atomically.
    ///
    /// The closure receives `&mut PageStore`. All `put` / `delete` calls inside
    /// the closure are buffered and written to the WAL. On `Ok(())` the
    /// transaction is committed (WAL fsynced, dirty pages flushed to `.tsm`).
    /// On `Err(_)` the transaction is rolled back (dirty pages discarded).
    ///
    /// Commit semantics: if the process crashes after `commit_txn` returns but
    /// before the dirty-page flush completes, recovery will replay the WAL on
    /// next open and restore the committed state.
    pub fn transaction<F, T>(&mut self, f: F) -> Result<T>
    where
        F: FnOnce(&mut PageStore) -> Result<T>,
    {
        self.tree.begin_txn()?;
        match f(self) {
            Ok(v) => {
                self.tree.commit_txn()?;
                Ok(v)
            }
            Err(e) => {
                self.tree.rollback_txn();
                Err(e)
            }
        }
    }

    #[cfg(test)]
    fn transaction_with_crash_file<F, T>(
        &mut self,
        f: F,
        crash_file: &mut crate::test_helpers::CrashFile,
    ) -> Result<T>
    where
        F: FnOnce(&mut PageStore) -> Result<T>,
    {
        self.tree.begin_txn()?;
        match f(self) {
            Ok(v) => {
                self.tree.commit_txn_with_crash_file(crash_file)?;
                Ok(v)
            }
            Err(e) => {
                self.tree.rollback_txn();
                Err(e)
            }
        }
    }
}

// ── Validation ────────────────────────────────────────────────────────────────

fn validate_key(key: &[u8]) -> Result<()> {
    if key.is_empty() {
        return Err(TosumuError::InvalidArgument("key must not be empty"));
    }
    if key.len() > u16::MAX as usize {
        return Err(TosumuError::InvalidArgument("key exceeds u16 maximum"));
    }
    Ok(())
}

fn validate_value(value: &[u8]) -> Result<()> {
    if value.len() > crate::format::MAX_VALUE_SIZE {
        return Err(TosumuError::ValueTooLarge {
            actual: value.len() as u64,
            maximum: crate::format::MAX_VALUE_SIZE as u64,
        });
    }
    Ok(())
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
#[path = "page_store/tests/mod.rs"]
mod tests;
