use std::collections::BTreeMap;
use std::sync::Arc;

use crate::error::{Result, TosumuError};
use crate::format::PAGE_SIZE;

use super::{for_each_committed_transaction, WalRecord};

#[derive(Debug, Clone)]
pub(crate) struct CommittedPageVersion {
    pub(crate) commit_lsn: u64,
    pub(crate) frame: Arc<[u8; PAGE_SIZE]>,
}

/// Process-local index of committed page versions newer than one checkpoint.
///
/// Commit LSN is the atomic generation. Multiple page writes owned by the same
/// matching commit become visible together; incomplete transactions contribute
/// no versions.
#[derive(Debug, Clone)]
pub(crate) struct CommittedWalIndex {
    checkpoint_lsn: u64,
    latest_commit_lsn: u64,
    pages: BTreeMap<u64, Vec<CommittedPageVersion>>,
}

impl CommittedWalIndex {
    pub(crate) fn empty(checkpoint_lsn: u64) -> Self {
        Self {
            checkpoint_lsn,
            latest_commit_lsn: checkpoint_lsn,
            pages: BTreeMap::new(),
        }
    }

    pub(crate) fn from_records(records: &[(u64, WalRecord)], checkpoint_lsn: u64) -> Result<Self> {
        let mut index = Self::empty(checkpoint_lsn);

        for_each_committed_transaction(records, |_, commit_lsn, transaction_records| {
            if commit_lsn <= checkpoint_lsn {
                return Ok(());
            }
            for (_, record) in transaction_records {
                if let WalRecord::PageWrite { pgno, frame, .. } = record {
                    index
                        .pages
                        .entry(*pgno)
                        .or_default()
                        .push(CommittedPageVersion {
                            commit_lsn,
                            frame: Arc::new(**frame),
                        });
                }
            }
            index.latest_commit_lsn = commit_lsn;
            Ok(())
        })?;

        Ok(index)
    }

    pub(crate) fn append_generation<'a, I>(&mut self, commit_lsn: u64, frames: I) -> Result<()>
    where
        I: IntoIterator<Item = (u64, &'a [u8; PAGE_SIZE])>,
    {
        if commit_lsn <= self.latest_commit_lsn {
            return Err(TosumuError::CorruptRecord {
                offset: 0,
                reason: "committed generation does not advance WAL index",
            });
        }
        for (pgno, frame) in frames {
            self.pages
                .entry(pgno)
                .or_default()
                .push(CommittedPageVersion {
                    commit_lsn,
                    frame: Arc::new(*frame),
                });
        }
        self.latest_commit_lsn = commit_lsn;
        Ok(())
    }

    pub(crate) fn checkpoint_lsn(&self) -> u64 {
        self.checkpoint_lsn
    }

    pub(crate) fn latest_commit_lsn(&self) -> u64 {
        self.latest_commit_lsn
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn retained_version_count(&self) -> u64 {
        self.pages.values().fold(0u64, |count, versions| {
            count.saturating_add(u64::try_from(versions.len()).unwrap_or(u64::MAX))
        })
    }

    pub(crate) fn page_at(&self, pgno: u64, snapshot_lsn: u64) -> Option<&CommittedPageVersion> {
        self.pages
            .get(&pgno)?
            .iter()
            .rev()
            .find(|version| version.commit_lsn <= snapshot_lsn)
    }

    pub(crate) fn for_each_page_at<F>(&self, snapshot_lsn: u64, mut visit: F) -> Result<()>
    where
        F: FnMut(u64, &CommittedPageVersion) -> Result<()>,
    {
        for pgno in self.pages.keys().copied() {
            if let Some(version) = self.page_at(pgno, snapshot_lsn) {
                visit(pgno, version)?;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn page_write(pgno: u64, marker: u8) -> WalRecord {
        let mut frame = Box::new([0u8; PAGE_SIZE]);
        frame[0] = marker;
        WalRecord::PageWrite {
            pgno,
            page_version: u64::from(marker),
            frame,
        }
    }

    #[test]
    fn page_selection_uses_owning_commit_generation() {
        let records = vec![
            (1, WalRecord::Begin { txn_id: 1 }),
            (2, page_write(1, 0x11)),
            (3, WalRecord::Commit { txn_id: 1 }),
            (4, WalRecord::Begin { txn_id: 2 }),
            (5, page_write(1, 0x22)),
            (6, page_write(2, 0x23)),
            (7, WalRecord::Commit { txn_id: 2 }),
        ];

        let index = CommittedWalIndex::from_records(&records, 0).unwrap();
        assert_eq!(index.latest_commit_lsn(), 7);
        assert!(index.page_at(1, 2).is_none());
        assert_eq!(index.page_at(1, 3).unwrap().frame[0], 0x11);
        assert_eq!(index.page_at(1, 6).unwrap().frame[0], 0x11);
        assert_eq!(index.page_at(1, 7).unwrap().frame[0], 0x22);
        assert!(index.page_at(2, 6).is_none());
        assert_eq!(index.page_at(2, 7).unwrap().frame[0], 0x23);
    }

    #[test]
    fn checkpointed_and_incomplete_transactions_contribute_no_versions() {
        let records = vec![
            (1, WalRecord::Begin { txn_id: 1 }),
            (2, page_write(1, 0x11)),
            (3, WalRecord::Commit { txn_id: 1 }),
            (4, WalRecord::Begin { txn_id: 2 }),
            (5, page_write(1, 0x22)),
        ];

        let index = CommittedWalIndex::from_records(&records, 3).unwrap();
        assert_eq!(index.latest_commit_lsn(), 3);
        assert!(index.page_at(1, u64::MAX).is_none());
    }

    #[test]
    fn prepared_generation_append_preserves_older_page_versions() {
        let mut first = [0u8; PAGE_SIZE];
        first[0] = 0x31;
        let mut second = [0u8; PAGE_SIZE];
        second[0] = 0x42;
        let mut index = CommittedWalIndex::empty(3);

        index.append_generation(6, [(1, &first)]).unwrap();
        let mut prepared = index.clone();
        prepared.append_generation(9, [(1, &second)]).unwrap();

        assert_eq!(index.latest_commit_lsn(), 6);
        assert_eq!(prepared.checkpoint_lsn(), 3);
        assert_eq!(prepared.page_at(1, 6).unwrap().frame[0], 0x31);
        assert_eq!(prepared.page_at(1, 9).unwrap().frame[0], 0x42);
    }
}
