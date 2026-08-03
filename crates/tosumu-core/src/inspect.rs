// Inspection and verification utilities for tosumu database files.
//
// Used by: `tosumu dump`, `tosumu hex`, `tosumu verify` CLI commands.
// Source of truth: DESIGN.md §12.1 (MVP +2).

use std::collections::HashSet;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

use crate::error::{Result, TosumuError};
use crate::format::*;
use crate::btree::BTree;
use crate::pager::Pager;
use crate::wal::{wal_path, WalReader, WalRecord};

// ── File header ───────────────────────────────────────────────────────────────

/// Parsed contents of the page-0 file header.
///
/// Does not require decryption — only validates the magic bytes.
pub struct HeaderInfo {
    pub format_version: u16,
    pub page_size: u16,
    pub min_reader_version: u16,
    pub flags: u16,
    pub page_count: u64,
    pub freelist_head: u64,
    pub root_page: u64,
    pub wal_checkpoint_lsn: u64,
    pub dek_id: u64,
    pub keyslot_count: u16,
    pub keyslot_region_pages: u16,
    /// Kind byte of the first keyslot.
    pub ks0_kind: u8,
    /// Version byte of the first keyslot.
    pub ks0_version: u8,
}

/// Read and parse the file header from `path`.
///
/// Only validates the magic bytes; does not authenticate or decrypt anything.
pub fn read_header_info(path: &Path) -> Result<HeaderInfo> {
    let mut file = File::open(path)?;
    let mut page0 = [0u8; PAGE_SIZE];
    file.read_exact(&mut page0)?;

    if !check_magic(&page0) {
        return Err(TosumuError::NotATosumFile);
    }

    let ks = KEYSLOT_REGION_OFFSET;
    Ok(HeaderInfo {
        format_version: read_u16(&page0, OFF_FORMAT_VERSION),
        page_size: read_u16(&page0, OFF_PAGE_SIZE),
        min_reader_version: read_u16(&page0, OFF_MIN_READER_VERSION),
        flags: read_u16(&page0, OFF_FLAGS),
        page_count: read_u64(&page0, OFF_PAGE_COUNT),
        freelist_head: read_u64(&page0, OFF_FREELIST_HEAD),
        root_page: read_u64(&page0, OFF_ROOT_PAGE),
        wal_checkpoint_lsn: read_u64(&page0, OFF_WAL_CHECKPOINT_LSN),
        dek_id: read_u64(&page0, OFF_DEK_ID),
        keyslot_count: read_u16(&page0, OFF_KEYSLOT_COUNT),
        keyslot_region_pages: read_u16(&page0, OFF_KEYSLOT_REGION_PAGES),
        ks0_kind: page0[ks + KS_OFF_KIND],
        ks0_version: page0[ks + KS_OFF_VERSION],
    })
}

// ── Raw frame ─────────────────────────────────────────────────────────────────

/// Read the raw (encrypted) 4096-byte frame for page `pgno`.
///
/// Page 0 is the plaintext file header; pages ≥ 1 are encrypted frames.
/// Does not decrypt or authenticate.
pub fn read_raw_frame(path: &Path, pgno: u64) -> Result<[u8; PAGE_SIZE]> {
    let offset = pgno
        .checked_mul(PAGE_SIZE as u64)
        .ok_or(TosumuError::InvalidArgument("page number overflow"))?;
    let mut file = File::open(path)?;
    let mut frame = [0u8; PAGE_SIZE];
    file.seek(SeekFrom::Start(offset))?;
    file.read_exact(&mut frame)?;
    Ok(frame)
}

// ── Page inspection ───────────────────────────────────────────────────────────

/// A single decoded record entry from a slotted leaf page.
pub enum RecordInfo {
    Live {
        key: Vec<u8>,
        value: Vec<u8>,
    },
    Tombstone {
        key: Vec<u8>,
    },
    /// Could not be decoded — carries slot index and raw record-type byte.
    Unknown {
        slot: u16,
        record_type: u8,
    },
}

/// Decoded summary of an encrypted data page.
pub struct PageSummary {
    pub pgno: u64,
    pub page_version: u64,
    pub page_type: u8,
    pub slot_count: u16,
    pub free_start: u16,
    pub free_end: u16,
    pub records: Vec<RecordInfo>,
}

pub struct PagesSummary {
    pub pages: Vec<PageListEntry>,
}

pub struct PageListEntry {
    pub pgno: u64,
    pub page_version: Option<u64>,
    pub page_type: Option<u8>,
    pub slot_count: Option<u16>,
    pub state: PageInspectState,
    pub issue: Option<String>,
}

pub enum PageInspectState {
    Ok,
    AuthFailed,
    Corrupt,
    Io,
}

pub struct TreeSummary {
    pub root_pgno: u64,
    pub root: TreeNodeSummary,
}

pub struct WalSummary {
    pub wal_exists: bool,
    pub wal_path: String,
    pub records: Vec<WalRecordSummary>,
}

pub struct WalRecordSummary {
    pub lsn: u64,
    pub kind: WalRecordSummaryKind,
}

pub enum WalRecordSummaryKind {
    Begin { txn_id: u64 },
    PageWrite { pgno: u64, page_version: u64 },
    Commit { txn_id: u64 },
    Checkpoint { up_to_lsn: u64 },
}

/// Whether recovery would replay or discard a parsed WAL transaction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecoveryDisposition {
    /// The transaction has a valid commit record and will be replayed.
    ReplayCommitted,
    /// The transaction has no commit record and will be discarded.
    DiscardUncommitted,
}

/// Structured observation of one WAL transaction relevant to recovery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryTransactionSummary {
    pub txn_id: u64,
    pub page_writes: u64,
    pub disposition: RecoveryDisposition,
}

/// Structured recovery observations derived from the WAL without mutating it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoverySummary {
    pub wal_exists: bool,
    pub transactions: Vec<RecoveryTransactionSummary>,
}

pub struct TreeNodeSummary {
    pub pgno: u64,
    pub page_version: u64,
    pub page_type: u8,
    pub slot_count: u16,
    pub free_start: u16,
    pub free_end: u16,
    pub next_leaf: Option<u64>,
    pub children: Vec<TreeChildSummary>,
}

pub struct TreeChildSummary {
    pub relation: TreeChildRelation,
    pub separator_key: Option<Vec<u8>>,
    pub child: Box<TreeNodeSummary>,
}

pub enum TreeChildRelation {
    Leftmost,
    Separator,
}

/// Decrypt and parse page `pgno`.
///
/// Returns `Err(InvalidArgument)` when `pgno` is 0 or out of range.
/// Opens the pager internally in read-only mode.
pub fn inspect_page(path: &Path, pgno: u64) -> Result<PageSummary> {
    let pager = Pager::open_readonly(path)?;
    inspect_page_from_pager(&pager, pgno)
}

pub fn inspect_tree(path: &Path) -> Result<TreeSummary> {
    let pager = Pager::open_readonly(path)?;
    inspect_tree_from_pager(&pager)
}

pub fn inspect_wal(path: &Path) -> Result<WalSummary> {
    let wal = wal_path(path);
    if !wal.exists() {
        return Ok(WalSummary {
            wal_exists: false,
            wal_path: wal.display().to_string(),
            records: Vec::new(),
        });
    }

    let records = WalReader::read_all(&wal)?
        .into_iter()
        .map(|(lsn, record)| WalRecordSummary {
            lsn,
            kind: match record {
                WalRecord::Begin { txn_id } => WalRecordSummaryKind::Begin { txn_id },
                WalRecord::PageWrite {
                    pgno, page_version, ..
                } => WalRecordSummaryKind::PageWrite { pgno, page_version },
                WalRecord::Commit { txn_id } => WalRecordSummaryKind::Commit { txn_id },
                WalRecord::Checkpoint { up_to_lsn } => {
                    WalRecordSummaryKind::Checkpoint { up_to_lsn }
                }
            },
        })
        .collect();

    Ok(WalSummary {
        wal_exists: true,
        wal_path: wal.display().to_string(),
        records,
    })
}

/// Classify WAL transactions as committed work to replay or uncommitted work
/// to discard. This is an observation only; it does not apply or truncate WAL.
pub fn inspect_recovery(path: &Path) -> Result<RecoverySummary> {
    let wal = inspect_wal(path)?;
    let mut transactions = Vec::new();
    let mut current: Option<RecoveryTransactionSummary> = None;

    for record in wal.records {
        match record.kind {
            WalRecordSummaryKind::Begin { txn_id } => {
                if let Some(previous) = current.take() {
                    transactions.push(previous);
                }
                current = Some(RecoveryTransactionSummary {
                    txn_id,
                    page_writes: 0,
                    disposition: RecoveryDisposition::DiscardUncommitted,
                });
            }
            WalRecordSummaryKind::PageWrite { .. } => {
                if let Some(transaction) = current.as_mut() {
                    transaction.page_writes += 1;
                }
            }
            WalRecordSummaryKind::Commit { txn_id } => {
                if let Some(mut transaction) = current.take() {
                    transaction.txn_id = txn_id;
                    transaction.disposition = RecoveryDisposition::ReplayCommitted;
                    transactions.push(transaction);
                }
            }
            WalRecordSummaryKind::Checkpoint { .. } => {}
        }
    }

    if let Some(transaction) = current {
        transactions.push(transaction);
    }

    Ok(RecoverySummary {
        wal_exists: wal.wal_exists,
        transactions,
    })
}

pub fn inspect_pages(path: &Path) -> Result<PagesSummary> {
    let pager = Pager::open_readonly(path)?;
    inspect_pages_from_pager(&pager)
}

/// Decrypt and parse page `pgno` from an already-open pager.
pub fn inspect_page_from_pager(pager: &Pager, pgno: u64) -> Result<PageSummary> {
    if pgno == 0 {
        return Err(TosumuError::InvalidArgument(
            "page 0 is the file header; use `dump` without --page to view it",
        ));
    }

    if pgno >= pager.page_count() {
        return Err(TosumuError::InspectPageOutOfRange {
            pgno,
            page_count: pager.page_count(),
        });
    }

    let (plaintext, page_version) = pager.read_page(pgno)?;
    let page_type = plaintext[0];
    let slot_count = read_u16(&plaintext, 2);
    let free_start = read_u16(&plaintext, 4);
    let free_end = read_u16(&plaintext, 6);

    let mut records = Vec::with_capacity(slot_count as usize);
    for i in 0..slot_count as usize {
        let slot_pos = PAGE_HEADER_SIZE + i * SLOT_SIZE;
        if slot_pos + SLOT_SIZE > PAGE_PLAINTEXT_SIZE {
            records.push(RecordInfo::Unknown {
                slot: i as u16,
                record_type: 0,
            });
            break;
        }
        let offset = read_u16(&plaintext, slot_pos) as usize;
        let length = read_u16(&plaintext, slot_pos + 2) as usize;

        if length == 0
            || offset < PAGE_HEADER_SIZE
            || offset < free_end as usize
            || offset + length > PAGE_PLAINTEXT_SIZE
        {
            records.push(RecordInfo::Unknown {
                slot: i as u16,
                record_type: 0,
            });
            continue;
        }

        let record = &plaintext[offset..offset + length];
        match record[0] {
            RECORD_LIVE if record.len() >= 5 => {
                let key_len = u16::from_le_bytes([record[1], record[2]]) as usize;
                let val_len = u16::from_le_bytes([record[3], record[4]]) as usize;
                if 5 + key_len + val_len <= record.len() {
                    records.push(RecordInfo::Live {
                        key: record[5..5 + key_len].to_vec(),
                        value: record[5 + key_len..5 + key_len + val_len].to_vec(),
                    });
                } else {
                    records.push(RecordInfo::Unknown {
                        slot: i as u16,
                        record_type: RECORD_LIVE,
                    });
                }
            }
            RECORD_TOMBSTONE if record.len() >= 3 => {
                let key_len = u16::from_le_bytes([record[1], record[2]]) as usize;
                if 3 + key_len <= record.len() {
                    records.push(RecordInfo::Tombstone {
                        key: record[3..3 + key_len].to_vec(),
                    });
                } else {
                    records.push(RecordInfo::Unknown {
                        slot: i as u16,
                        record_type: RECORD_TOMBSTONE,
                    });
                }
            }
            rt => records.push(RecordInfo::Unknown {
                slot: i as u16,
                record_type: rt,
            }),
        }
    }

    Ok(PageSummary {
        pgno,
        page_version,
        page_type,
        slot_count,
        free_start,
        free_end,
        records,
    })
}

pub fn inspect_pages_from_pager(pager: &Pager) -> Result<PagesSummary> {
    let mut pages = Vec::with_capacity(pager.page_count().saturating_sub(1) as usize);

    for pgno in 1..pager.page_count() {
        match pager.read_page(pgno) {
            Ok((plaintext, page_version)) => {
                pages.push(PageListEntry {
                    pgno,
                    page_version: Some(page_version),
                    page_type: Some(plaintext[PAGE_OFF_TYPE]),
                    slot_count: Some(read_u16(&plaintext, PAGE_OFF_SLOT_COUNT)),
                    state: PageInspectState::Ok,
                    issue: None,
                });
            }
            Err(TosumuError::AuthFailed { .. }) => {
                pages.push(PageListEntry {
                    pgno,
                    page_version: None,
                    page_type: None,
                    slot_count: None,
                    state: PageInspectState::AuthFailed,
                    issue: Some(
                        "authentication tag mismatch (page corrupted or tampered)".to_owned(),
                    ),
                });
            }
            Err(TosumuError::Corrupt { reason, .. }) => {
                pages.push(PageListEntry {
                    pgno,
                    page_version: None,
                    page_type: None,
                    slot_count: None,
                    state: PageInspectState::Corrupt,
                    issue: Some(format!("corrupt: {reason}")),
                });
            }
            Err(error) => {
                pages.push(PageListEntry {
                    pgno,
                    page_version: None,
                    page_type: None,
                    slot_count: None,
                    state: PageInspectState::Io,
                    issue: Some(format!("I/O error: {error}")),
                });
            }
        }
    }

    Ok(PagesSummary { pages })
}

pub fn inspect_tree_from_pager(pager: &Pager) -> Result<TreeSummary> {
    let root_pgno = pager.root_page();
    if root_pgno == 0 {
        return Err(TosumuError::Corrupt {
            pgno: 0,
            reason: "root_page is 0",
        });
    }

    let mut visited = HashSet::new();
    let root = inspect_tree_node_from_pager(pager, root_pgno, &mut visited, 1)?;
    Ok(TreeSummary { root_pgno, root })
}

fn inspect_tree_node_from_pager(
    pager: &Pager,
    pgno: u64,
    visited: &mut HashSet<u64>,
    depth: usize,
) -> Result<TreeNodeSummary> {
    const MAX_DEPTH: usize = 64;

    if depth > MAX_DEPTH {
        return Err(TosumuError::Corrupt {
            pgno,
            reason: "tree inspection exceeded maximum depth (cycle suspected)",
        });
    }

    if pgno == 0 || pgno >= pager.page_count() {
        return Err(TosumuError::Corrupt {
            pgno,
            reason: "tree node page number out of range",
        });
    }

    if !visited.insert(pgno) {
        return Err(TosumuError::Corrupt {
            pgno,
            reason: "tree inspection encountered a repeated page",
        });
    }

    let result = (|| {
        let (plaintext, page_version) = pager.read_page(pgno)?;
        let page_type = plaintext[PAGE_OFF_TYPE];
        let slot_count = read_u16(&plaintext, PAGE_OFF_SLOT_COUNT);
        let free_start = read_u16(&plaintext, PAGE_OFF_FREE_START);
        let free_end = read_u16(&plaintext, PAGE_OFF_FREE_END);

        match page_type {
            PAGE_TYPE_LEAF => Ok(TreeNodeSummary {
                pgno,
                page_version,
                page_type,
                slot_count,
                free_start,
                free_end,
                next_leaf: match read_u64(&plaintext, PAGE_OFF_LEFTMOST) {
                    0 => None,
                    next => Some(next),
                },
                children: Vec::new(),
            }),
            PAGE_TYPE_INTERNAL => {
                let mut children = Vec::with_capacity(slot_count as usize + 1);
                let leftmost = read_u64(&plaintext, PAGE_OFF_LEFTMOST);
                let leftmost_child =
                    inspect_tree_node_from_pager(pager, leftmost, visited, depth + 1)?;
                children.push(TreeChildSummary {
                    relation: TreeChildRelation::Leftmost,
                    separator_key: None,
                    child: Box::new(leftmost_child),
                });

                for index in 0..slot_count as usize {
                    let (separator_key, right_child) =
                        inspect_internal_slot(&plaintext, pgno, index)?;
                    let child =
                        inspect_tree_node_from_pager(pager, right_child, visited, depth + 1)?;
                    children.push(TreeChildSummary {
                        relation: TreeChildRelation::Separator,
                        separator_key: Some(separator_key),
                        child: Box::new(child),
                    });
                }

                Ok(TreeNodeSummary {
                    pgno,
                    page_version,
                    page_type,
                    slot_count,
                    free_start,
                    free_end,
                    next_leaf: None,
                    children,
                })
            }
            _ => Err(TosumuError::Corrupt {
                pgno,
                reason: "unexpected page type during tree inspection",
            }),
        }
    })();

    visited.remove(&pgno);
    result
}

fn inspect_internal_slot(
    page: &[u8; PAGE_PLAINTEXT_SIZE],
    pgno: u64,
    index: usize,
) -> Result<(Vec<u8>, u64)> {
    let slot_pos = PAGE_HEADER_SIZE + index * SLOT_SIZE;
    if slot_pos + SLOT_SIZE > PAGE_PLAINTEXT_SIZE {
        return Err(TosumuError::Corrupt {
            pgno,
            reason: "internal slot header overflow",
        });
    }

    let off = read_u16(page, slot_pos) as usize;
    let len = read_u16(page, slot_pos + 2) as usize;
    if off + len > PAGE_PLAINTEXT_SIZE || len < 10 {
        return Err(TosumuError::Corrupt {
            pgno,
            reason: "invalid internal slot",
        });
    }

    let rec = &page[off..off + len];
    let right_child = u64::from_le_bytes(rec[0..8].try_into().unwrap());
    let key_len = u16::from_le_bytes([rec[8], rec[9]]) as usize;
    if 10 + key_len > len {
        return Err(TosumuError::Corrupt {
            pgno,
            reason: "internal slot key overflow",
        });
    }

    Ok((rec[10..10 + key_len].to_vec(), right_child))
}

// ── Verification ─────────────────────────────────────────────────────────────

/// A single integrity problem found during `verify_file`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VerifyIssueKind {
    AuthFailed,
    Corrupt,
    Io,
}

pub struct VerifyIssue {
    pub pgno: u64,
    pub kind: VerifyIssueKind,
    pub description: String,
}

/// Per-page result across the three epistemic dimensions (§29.2).
pub struct PageVerifyResult {
    pub pgno: u64,
    /// `Some(v)` when AEAD passed, `None` when it failed.
    pub page_version: Option<u64>,
    pub auth_ok: bool,
    pub issue_kind: Option<VerifyIssueKind>,
    /// Human-readable description of any failure, or `None` when clean.
    pub issue: Option<String>,
}

/// Summary returned by `verify_file`.
pub struct VerifyReport {
    pub pages_checked: u64,
    pub pages_ok: u64,
    pub issues: Vec<VerifyIssue>,
    /// Per-page detail, always populated (used by `--explain`).
    pub page_results: Vec<PageVerifyResult>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BTreeVerificationIssueKind {
    Invalid,
    Incomplete,
    OverflowChainCorrupt,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BTreeVerificationIssue {
    pub kind: BTreeVerificationIssueKind,
    pub description: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BTreeVerification {
    pub checked: bool,
    pub ok: bool,
    pub issue: Option<BTreeVerificationIssue>,
}

pub struct VerificationReport {
    pub header: HeaderInfo,
    pub recovery: RecoverySummary,
    pub pages: VerifyReport,
    pub btree: BTreeVerification,
}

/// Open `path` and authenticate every data page (1..page_count).
///
/// Does not short-circuit on first error — all pages are checked and all
/// failures are collected. Returns `Err` only for fatal header-level errors
/// (bad magic, I/O failure reading page 0, etc.).
pub fn verify_file(path: &Path) -> Result<VerifyReport> {
    let pager = Pager::open_readonly(path)?;
    verify_pager(&pager)
}

/// Produce one structured verification snapshot without exposing storage
/// implementation types to consumers.
pub fn inspect_verification(path: &Path) -> Result<VerificationReport> {
    let header = read_header_info(path)?;
    let recovery = inspect_recovery(path)?;
    let pager = Pager::open_readonly(path)?;
    let pages = verify_pager(&pager)?;
    let btree = if pages.issues.is_empty() {
        match BTree::open_readonly(path).and_then(|tree| tree.check_invariants()) {
            Ok(()) => BTreeVerification {
                checked: true,
                ok: true,
                issue: None,
            },
            Err(TosumuError::OverflowChainCorrupt { .. }) => BTreeVerification {
                checked: true,
                ok: false,
                issue: Some(BTreeVerificationIssue {
                    kind: BTreeVerificationIssueKind::OverflowChainCorrupt,
                    description: "overflow chain corruption was found".to_owned(),
                }),
            },
            Err(error) => BTreeVerification {
                checked: true,
                ok: false,
                issue: Some(BTreeVerificationIssue {
                    kind: BTreeVerificationIssueKind::Invalid,
                    description: error.to_string(),
                }),
            },
        }
    } else {
        BTreeVerification {
            checked: false,
            ok: false,
            issue: Some(BTreeVerificationIssue {
                kind: BTreeVerificationIssueKind::Incomplete,
                description: "skipped because page integrity issues were found".to_owned(),
            }),
        }
    };

    Ok(VerificationReport {
        header,
        recovery,
        pages,
        btree,
    })
}

/// Authenticate every data page through an already-open pager.
pub fn verify_pager(pager: &Pager) -> Result<VerifyReport> {
    let page_count = pager.page_count();
    let pages_to_check = page_count.saturating_sub(1); // skip page 0

    let mut pages_ok = 0u64;
    let mut issues = Vec::new();
    let mut page_results = Vec::with_capacity(pages_to_check as usize);

    for pgno in 1..page_count {
        match pager.read_page(pgno) {
            Ok((_, version)) => {
                pages_ok += 1;
                page_results.push(PageVerifyResult {
                    pgno,
                    page_version: Some(version),
                    auth_ok: true,
                    issue_kind: None,
                    issue: None,
                });
            }
            Err(TosumuError::AuthFailed { .. }) => {
                let desc = "authentication tag mismatch (page corrupted or tampered)".to_owned();
                issues.push(VerifyIssue {
                    pgno,
                    kind: VerifyIssueKind::AuthFailed,
                    description: desc.clone(),
                });
                page_results.push(PageVerifyResult {
                    pgno,
                    page_version: None,
                    auth_ok: false,
                    issue_kind: Some(VerifyIssueKind::AuthFailed),
                    issue: Some(desc),
                });
            }
            Err(TosumuError::Corrupt { reason, .. }) => {
                let desc = format!("corrupt: {reason}");
                issues.push(VerifyIssue {
                    pgno,
                    kind: VerifyIssueKind::Corrupt,
                    description: desc.clone(),
                });
                page_results.push(PageVerifyResult {
                    pgno,
                    page_version: None,
                    auth_ok: false,
                    issue_kind: Some(VerifyIssueKind::Corrupt),
                    issue: Some(desc),
                });
            }
            Err(e) => {
                let desc = format!("I/O error: {e}");
                issues.push(VerifyIssue {
                    pgno,
                    kind: VerifyIssueKind::Io,
                    description: desc.clone(),
                });
                page_results.push(PageVerifyResult {
                    pgno,
                    page_version: None,
                    auth_ok: false,
                    issue_kind: Some(VerifyIssueKind::Io),
                    issue: Some(desc),
                });
            }
        }
    }

    Ok(VerifyReport {
        pages_checked: pages_to_check,
        pages_ok,
        issues,
        page_results,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        inspect_recovery, inspect_tree_node_from_pager, inspect_verification, RecoveryDisposition,
    };
    use crate::error::TosumuError;
    use crate::format::{FORMAT_VERSION, OFF_FORMAT_VERSION};
    use crate::page_store::PageStore;
    use crate::pager::Pager;
    use crate::wal::{wal_path, WalRecord, WalWriter};

    #[test]
    fn inspect_verification_returns_structured_core_snapshot() {
        let path = std::env::temp_dir().join(format!(
            "tosumu_inspect_verification_{}.tsm",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(wal_path(&path));

        let mut store = PageStore::create(&path).unwrap();
        store.put(b"asset/manifest", b"schema-v1").unwrap();
        drop(store);

        let report = inspect_verification(&path).unwrap();
        assert_eq!(report.header.format_version, 2);
        assert!(report.recovery.wal_exists);
        assert_eq!(report.pages.pages_ok, report.pages.pages_checked);
        assert!(report.btree.checked);
        assert!(report.btree.ok);
        assert!(report.btree.issue.is_none());

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(wal_path(&path));
    }

    #[test]
    fn inspect_verification_preserves_busy_as_structured_error() {
        let _lock = crate::wal::fault_injection::LOCK.lock().unwrap();
        let path = std::env::temp_dir().join(format!(
            "tosumu_inspect_verification_busy_{}.tsm",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(wal_path(&path));

        PageStore::create(&path).unwrap();
        crate::wal::fault_injection::arm(100);
        let error = match inspect_verification(&path) {
            Ok(_) => panic!("busy WAL access must prevent verification"),
            Err(error) => error,
        };
        crate::wal::fault_injection::disarm();

        assert!(matches!(error, TosumuError::FileBusy { .. }));
        assert_eq!(error.error_report().code, "FILE_OPEN_BUSY");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(wal_path(&path));
    }

    #[test]
    fn inspect_verification_rejects_newer_physical_format_without_migration() {
        let path = std::env::temp_dir().join(format!(
            "tosumu_inspect_newer_format_{}.tsm",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(wal_path(&path));

        PageStore::create(&path).unwrap();
        let mut page0 = std::fs::read(&path).unwrap();
        page0[OFF_FORMAT_VERSION..OFF_FORMAT_VERSION + 2].copy_from_slice(&3u16.to_le_bytes());
        std::fs::write(&path, &page0).unwrap();

        let error = match super::inspect_verification(&path) {
            Ok(_) => panic!("newer physical format must be rejected"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            TosumuError::NewerFormat {
                found: 3,
                supported_max: FORMAT_VERSION
            }
        ));

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(wal_path(&path));
    }

    #[test]
    fn inspect_tree_node_out_of_range_is_corrupt() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("tosumu_inspect_tree_out_of_range_{nanos}.tsm"));
        let _ = std::fs::remove_file(&path);

        PageStore::create(&path).unwrap();
        let pager = Pager::open_readonly(&path).unwrap();
        match inspect_tree_node_from_pager(
            &pager,
            pager.page_count(),
            &mut std::collections::HashSet::new(),
            1,
        ) {
            Err(TosumuError::Corrupt {
                pgno,
                reason: "tree node page number out of range",
            }) => assert_eq!(pgno, pager.page_count()),
            Err(other) => panic!("unexpected error: {other}"),
            Ok(_) => panic!("expected corruption error"),
        }

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn inspect_recovery_classifies_committed_and_uncommitted_transactions() {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tosumu_inspect_recovery_{nanos}.tsm"));
        let wal = wal_path(&path);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&wal);
        PageStore::create(&path).unwrap();
        let _ = std::fs::remove_file(&wal);

        let mut writer = WalWriter::create(&wal).unwrap();
        writer.append(&WalRecord::Begin { txn_id: 11 }).unwrap();
        writer
            .append(&WalRecord::PageWrite {
                pgno: 1,
                page_version: 2,
                frame: Box::new([0u8; crate::format::PAGE_SIZE]),
            })
            .unwrap();
        writer.append(&WalRecord::Commit { txn_id: 11 }).unwrap();
        writer.append(&WalRecord::Begin { txn_id: 12 }).unwrap();
        writer
            .append(&WalRecord::PageWrite {
                pgno: 2,
                page_version: 3,
                frame: Box::new([0u8; crate::format::PAGE_SIZE]),
            })
            .unwrap();
        writer.sync().unwrap();

        let summary = inspect_recovery(&path).unwrap();
        assert!(summary.wal_exists);
        assert_eq!(summary.transactions.len(), 2);
        assert_eq!(summary.transactions[0].txn_id, 11);
        assert_eq!(summary.transactions[0].page_writes, 1);
        assert_eq!(
            summary.transactions[0].disposition,
            RecoveryDisposition::ReplayCommitted
        );
        assert_eq!(summary.transactions[1].txn_id, 12);
        assert_eq!(summary.transactions[1].page_writes, 1);
        assert_eq!(
            summary.transactions[1].disposition,
            RecoveryDisposition::DiscardUncommitted
        );

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&wal);
    }
}
