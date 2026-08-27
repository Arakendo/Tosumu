//! Provider-neutral, bounded inspection observations.
//!
//! This module is intentionally below the CLI and UI-provider boundaries. It
//! collects stable facts from `inspect` without retaining paths, pager handles,
//! terminal state, JSON envelopes, or renderer-specific state.

use std::path::Path;

use crate::error::{ErrorStatus, Result};
use crate::inspect::{
    inspect_pages, inspect_pages_from_pager, inspect_tree, inspect_tree_from_pager,
    inspect_verification, inspect_wal, read_header_info, read_header_info_from_page0, verify_pager,
    BTreeVerification, BTreeVerificationIssue, BTreeVerificationIssueKind, PageInspectState,
    RecoveryDisposition, TreeSummary, VerifyIssueKind, WalRecordSummaryKind,
};
use crate::page_store::PageStore;
use crate::pager::Pager;

/// Schema version for the provider-neutral inspection observation.
pub const INSPECTION_OBSERVATION_SCHEMA_V1: u16 = 1;

/// Conservative default upper bound for an inspection upload before a host
/// adapter has chosen a more specific policy.
pub const DEFAULT_INSPECTION_BYTE_INPUT_LIMIT: usize = 16 * 1024 * 1024;

/// Bounded collection limits chosen by the consuming application or provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct InspectionObservationLimits {
    pub page_limit: usize,
    pub verification_issue_limit: usize,
    pub wal_record_limit: usize,
    pub tree_depth_limit: usize,
    pub tree_node_limit: usize,
    pub keyslot_limit: usize,
}

impl Default for InspectionObservationLimits {
    fn default() -> Self {
        Self {
            page_limit: 128,
            verification_issue_limit: 64,
            wal_record_limit: 128,
            tree_depth_limit: 8,
            tree_node_limit: 256,
            keyslot_limit: 32,
        }
    }
}

/// A result that preserves a typed reason when an optional inspection section
/// cannot be read. Consumers can show this directly instead of guessing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InspectionSection<T> {
    Available(T),
    Unavailable(InspectionUnavailable),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionUnavailable {
    pub code: &'static str,
    pub status: ErrorStatus,
    pub message: String,
}

/// Complete provider-neutral inspection snapshot for one store at one instant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionObservation {
    pub schema_version: u16,
    pub header: InspectionHeader,
    pub verification: InspectionVerification,
    pub pages: InspectionPageList,
    pub tree: InspectionSection<InspectionTree>,
    pub wal: InspectionSection<InspectionWal>,
    pub keyslots: InspectionSection<InspectionKeyslots>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionHeader {
    pub format_version: u16,
    pub page_size: u16,
    pub page_count: u64,
    pub root_page: u64,
    pub flags: u16,
    pub keyslot_count: u16,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionVerification {
    pub pages_checked: u64,
    pub pages_ok: u64,
    pub btree_checked: bool,
    pub btree_ok: bool,
    pub recovery: InspectionRecovery,
    pub issues: Vec<InspectionIssue>,
    pub issues_truncated: u64,
    pub btree_issue: Option<InspectionIssue>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionRecovery {
    pub wal_exists: bool,
    pub replay_committed: u64,
    pub discard_uncommitted: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionIssue {
    pub code: &'static str,
    pub message: String,
    pub page_number: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionPageList {
    pub total: u64,
    pub entries: Vec<InspectionPage>,
    pub truncated: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionPage {
    pub page_number: u64,
    pub page_version: Option<u64>,
    pub page_type: Option<u8>,
    pub slot_count: Option<u16>,
    pub state: InspectionPageState,
    pub issue: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectionPageState {
    Ok,
    AuthFailed,
    Corrupt,
    Io,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionTree {
    pub root_page: u64,
    pub nodes: Vec<InspectionTreeNode>,
    pub truncated: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionTreeNode {
    pub page_number: u64,
    pub page_version: u64,
    pub page_type: u8,
    pub slot_count: u16,
    pub next_leaf: Option<u64>,
    pub depth: usize,
    pub child_count: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionWal {
    pub exists: bool,
    pub record_count: u64,
    pub records: Vec<InspectionWalRecord>,
    pub truncated: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionWalRecord {
    pub lsn: u64,
    pub kind: InspectionWalRecordKind,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InspectionWalRecordKind {
    Begin { transaction_id: u64 },
    PageWrite { page_number: u64, page_version: u64 },
    Commit { transaction_id: u64 },
    Checkpoint { up_to_lsn: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionKeyslots {
    pub total: u64,
    pub slots: Vec<InspectionKeyslot>,
    pub truncated: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionKeyslot {
    pub slot: u16,
    pub kind: u8,
}

/// Provider-neutral view intent. Providers decide how a view is reached and
/// rendered; this enum only records which inspection facts the user requested.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum InspectionView {
    Header,
    Detail,
    Verify,
    Tree,
    Wal,
    Keyslots,
}

/// Stable, provider-neutral interaction state for one inspection observation.
///
/// This intentionally excludes host paths, pager handles, terminal focus,
/// scrolling, prompt buffers, and watch timing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionSession {
    pub view: InspectionView,
    pub selected_page: Option<u64>,
    pub filter_query: String,
    pub revision: u64,
}

impl Default for InspectionSession {
    fn default() -> Self {
        Self {
            view: InspectionView::Detail,
            selected_page: None,
            filter_query: String::new(),
            revision: 0,
        }
    }
}

/// Semantic interaction requests that a native or browser provider may
/// translate from its own input mechanisms.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InspectionCommand {
    SelectView(InspectionView),
    SelectPage { page_number: u64 },
    SetFilter { query: String },
    ClearFilter,
    Navigate(PageNavigation),
    Refresh { expected_revision: u64 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageNavigation {
    First,
    Previous,
    Next,
    Last,
}

/// Deterministic effect of applying an inspection command.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InspectionCommandOutcome {
    Applied(InspectionCommandEffect),
    Rejected(InspectionCommandRejection),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum InspectionCommandEffect {
    ViewSelected { view: InspectionView },
    PageSelected { page_number: Option<u64> },
    FilterChanged { query: String, match_count: usize },
    RefreshRequested { next_revision: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InspectionCommandRejection {
    pub code: &'static str,
    pub message: String,
}

/// Applies semantic inspection intent without mutating storage facts.
///
/// The supplied observation is immutable input. Refresh only advances the
/// session revision and asks the host to provide a later observation; it does
/// not open files, schedule a watch loop, or decide a refresh cadence.
pub fn apply_inspection_command(
    session: &mut InspectionSession,
    observation: &InspectionObservation,
    command: InspectionCommand,
) -> InspectionCommandOutcome {
    match command {
        InspectionCommand::SelectView(view) => {
            session.view = view;
            InspectionCommandOutcome::Applied(InspectionCommandEffect::ViewSelected { view })
        }
        InspectionCommand::SelectPage { page_number } => {
            if !matching_page_numbers(observation, &session.filter_query).contains(&page_number) {
                return rejected(
                    "PAGE_NOT_IN_OBSERVATION",
                    format!(
                        "page {page_number} is not available in the current bounded observation"
                    ),
                );
            }
            session.selected_page = Some(page_number);
            InspectionCommandOutcome::Applied(InspectionCommandEffect::PageSelected {
                page_number: session.selected_page,
            })
        }
        InspectionCommand::SetFilter { query } => {
            let query = query.trim().to_owned();
            session.filter_query.clone_from(&query);
            let matching_pages = matching_page_numbers(observation, &query);
            if session
                .selected_page
                .is_some_and(|page| !matching_pages.contains(&page))
            {
                session.selected_page = None;
            }
            InspectionCommandOutcome::Applied(InspectionCommandEffect::FilterChanged {
                query,
                match_count: matching_pages.len(),
            })
        }
        InspectionCommand::ClearFilter => {
            session.filter_query.clear();
            InspectionCommandOutcome::Applied(InspectionCommandEffect::FilterChanged {
                query: String::new(),
                match_count: observation.pages.entries.len(),
            })
        }
        InspectionCommand::Navigate(navigation) => {
            let matching_pages = matching_page_numbers(observation, &session.filter_query);
            let next = navigate_pages(session.selected_page, &matching_pages, navigation);
            session.selected_page = next;
            InspectionCommandOutcome::Applied(InspectionCommandEffect::PageSelected {
                page_number: next,
            })
        }
        InspectionCommand::Refresh { expected_revision } => {
            if expected_revision != session.revision {
                return rejected(
                    "STALE_INSPECTION_REVISION",
                    format!(
                        "refresh expected inspection revision {expected_revision}, current revision is {}",
                        session.revision
                    ),
                );
            }
            session.revision = session.revision.saturating_add(1);
            InspectionCommandOutcome::Applied(InspectionCommandEffect::RefreshRequested {
                next_revision: session.revision,
            })
        }
    }
}

fn rejected(code: &'static str, message: String) -> InspectionCommandOutcome {
    InspectionCommandOutcome::Rejected(InspectionCommandRejection { code, message })
}

fn matching_page_numbers(observation: &InspectionObservation, query: &str) -> Vec<u64> {
    let normalized_query = query.trim().to_ascii_lowercase();
    observation
        .pages
        .entries
        .iter()
        .filter(|page| {
            normalized_query.is_empty()
                || page.page_number.to_string().contains(&normalized_query)
                || page_state_label(page.state).contains(&normalized_query)
                || page
                    .issue
                    .as_deref()
                    .is_some_and(|issue| issue.to_ascii_lowercase().contains(&normalized_query))
        })
        .map(|page| page.page_number)
        .collect()
}

fn page_state_label(state: InspectionPageState) -> &'static str {
    match state {
        InspectionPageState::Ok => "ok",
        InspectionPageState::AuthFailed => "auth_failed",
        InspectionPageState::Corrupt => "corrupt",
        InspectionPageState::Io => "io",
    }
}

fn navigate_pages(
    selected_page: Option<u64>,
    pages: &[u64],
    navigation: PageNavigation,
) -> Option<u64> {
    let current_index =
        selected_page.and_then(|page| pages.iter().position(|candidate| *candidate == page));
    match navigation {
        PageNavigation::First => pages.first().copied(),
        PageNavigation::Last => pages.last().copied(),
        PageNavigation::Previous => current_index
            .map(|index| index.saturating_sub(1))
            .and_then(|index| pages.get(index).copied())
            .or_else(|| pages.first().copied()),
        PageNavigation::Next => current_index
            .map(|index| index.saturating_add(1).min(pages.len().saturating_sub(1)))
            .and_then(|index| pages.get(index).copied())
            .or_else(|| pages.first().copied()),
    }
}

/// Collect a bounded snapshot from a store without retaining its path or open
/// storage handles. Header, verification, and page inspection are foundational
/// facts; optional derived sections preserve their own unavailable outcome.
pub fn inspect_observation(
    path: &Path,
    limits: InspectionObservationLimits,
) -> Result<InspectionObservation> {
    let header = read_header_info(path)?;
    let verification = inspect_verification(path)?;
    let pages = inspect_pages(path)?;

    Ok(InspectionObservation {
        schema_version: INSPECTION_OBSERVATION_SCHEMA_V1,
        header: InspectionHeader {
            format_version: header.format_version,
            page_size: header.page_size,
            page_count: header.page_count,
            root_page: header.root_page,
            flags: header.flags,
            keyslot_count: header.keyslot_count,
        },
        verification: map_verification(verification, limits.verification_issue_limit),
        pages: map_pages(pages, limits.page_limit),
        tree: section_from(inspect_tree(path), |tree| map_tree(tree, limits)),
        wal: section_from(inspect_wal(path), |wal| {
            map_wal(wal, limits.wal_record_limit)
        }),
        keyslots: section_from(PageStore::list_keyslots(path), |slots| {
            map_keyslots(slots, limits.keyslot_limit)
        }),
    })
}

/// Collect an inspection observation through an already-open pager.
///
/// Native providers use this when they have already resolved an unlock method.
/// It preserves the same provider-neutral output shape without serializing the
/// pager, unlock material, or host-side session details. B-tree invariant
/// checking is explicitly unavailable here because the caller has supplied a
/// pager rather than an open B-tree; it remains a distinct derived fact rather
/// than being silently inferred from readable pages.
pub fn inspect_observation_from_pager(
    path: &Path,
    pager: &Pager,
    limits: InspectionObservationLimits,
) -> Result<InspectionObservation> {
    let header = read_header_info(path)?;
    let recovery = crate::inspect::inspect_recovery(path)?;
    let pages = inspect_pages_from_pager(pager)?;
    let verification = verify_pager(pager)?;
    let btree = BTreeVerification {
        checked: false,
        ok: false,
        issue: Some(BTreeVerificationIssue {
            kind: BTreeVerificationIssueKind::Incomplete,
            description: "B-tree invariants were not evaluated by the supplied pager observation"
                .to_owned(),
        }),
    };

    Ok(InspectionObservation {
        schema_version: INSPECTION_OBSERVATION_SCHEMA_V1,
        header: InspectionHeader {
            format_version: header.format_version,
            page_size: header.page_size,
            page_count: header.page_count,
            root_page: header.root_page,
            flags: header.flags,
            keyslot_count: header.keyslot_count,
        },
        verification: map_verification_parts(
            verification,
            recovery,
            btree,
            limits.verification_issue_limit,
        ),
        pages: map_pages(pages, limits.page_limit),
        tree: section_from(inspect_tree_from_pager(pager), |tree| {
            map_tree(tree, limits)
        }),
        wal: section_from(inspect_wal(path), |wal| {
            map_wal(wal, limits.wal_record_limit)
        }),
        keyslots: section_from(PageStore::list_keyslots(path), |slots| {
            map_keyslots(slots, limits.keyslot_limit)
        }),
    })
}

/// Collect the header facts that can be established from a bounded raw byte
/// input without inventing a filesystem path, provider handle, or protector.
///
/// The resulting optional sections are deliberately unavailable. Page,
/// B-tree, WAL, and keyslot inspection require an approved unlock and pager
/// path; a browser or other byte-only host must surface that limitation rather
/// than treating an uploaded file as an opened database.
pub fn inspect_observation_from_bytes(
    bytes: &[u8],
    byte_limit: usize,
) -> Result<InspectionObservation> {
    if bytes.len() > byte_limit {
        return Err(crate::error::TosumuError::InvalidArgument(
            "inspection byte input exceeds configured limit",
        ));
    }

    let header = read_header_info_from_page0(bytes)?;
    if !(crate::format::MIN_SUPPORTED_FORMAT_VERSION..=crate::format::FORMAT_VERSION)
        .contains(&header.format_version)
    {
        return Err(crate::error::TosumuError::UnsupportedFormat {
            found: header.format_version,
            supported_min: crate::format::MIN_SUPPORTED_FORMAT_VERSION,
            supported_max: crate::format::FORMAT_VERSION,
        });
    }
    if usize::from(header.page_size) != crate::format::PAGE_SIZE {
        return Err(crate::error::TosumuError::PageSizeMismatch {
            found: header.page_size,
            expected: crate::format::PAGE_SIZE as u16,
        });
    }
    Ok(InspectionObservation {
        schema_version: INSPECTION_OBSERVATION_SCHEMA_V1,
        header: InspectionHeader {
            format_version: header.format_version,
            page_size: header.page_size,
            page_count: header.page_count,
            root_page: header.root_page,
            flags: header.flags,
            keyslot_count: header.keyslot_count,
        },
        verification: InspectionVerification {
            pages_checked: 0,
            pages_ok: 0,
            btree_checked: false,
            btree_ok: false,
            recovery: InspectionRecovery {
                wal_exists: false,
                replay_committed: 0,
                discard_uncommitted: 0,
            },
            issues: Vec::new(),
            issues_truncated: 0,
            btree_issue: Some(InspectionIssue {
                code: "RAW_BYTES_REQUIRE_UNLOCK",
                message: "B-tree verification requires an approved protector and pager path"
                    .to_owned(),
                page_number: None,
            }),
        },
        pages: InspectionPageList {
            total: header.page_count,
            entries: Vec::new(),
            truncated: header.page_count,
        },
        tree: unavailable_section(
            "RAW_BYTES_TREE_UNAVAILABLE",
            "Tree inspection requires an approved protector and pager path",
        ),
        wal: unavailable_section(
            "RAW_BYTES_WAL_UNAVAILABLE",
            "WAL inspection requires a provider-owned companion-file path",
        ),
        keyslots: unavailable_section(
            "RAW_BYTES_KEYSLOTS_UNAVAILABLE",
            "Keyslot enumeration requires a provider-owned protected-store path",
        ),
    })
}

fn section_from<T, U>(result: Result<T>, map: impl FnOnce(T) -> U) -> InspectionSection<U> {
    match result {
        Ok(value) => InspectionSection::Available(map(value)),
        Err(error) => {
            let report = error.error_report();
            InspectionSection::Unavailable(InspectionUnavailable {
                code: report.code,
                status: report.status,
                message: report.message,
            })
        }
    }
}

fn unavailable_section<T>(code: &'static str, message: &'static str) -> InspectionSection<T> {
    InspectionSection::Unavailable(InspectionUnavailable {
        code,
        status: ErrorStatus::Unsupported,
        message: message.to_owned(),
    })
}

fn map_verification(
    verification: crate::inspect::VerificationReport,
    issue_limit: usize,
) -> InspectionVerification {
    map_verification_parts(
        verification.pages,
        verification.recovery,
        verification.btree,
        issue_limit,
    )
}

fn map_verification_parts(
    verification: crate::inspect::VerifyReport,
    recovery: crate::inspect::RecoverySummary,
    btree: BTreeVerification,
    issue_limit: usize,
) -> InspectionVerification {
    let crate::inspect::RecoverySummary {
        wal_exists,
        transactions,
        ..
    } = recovery;
    let BTreeVerification {
        checked: btree_checked,
        ok: btree_ok,
        issue: btree_issue,
    } = btree;
    let total_issues = verification.issues.len();
    let issues = verification
        .issues
        .into_iter()
        .take(issue_limit)
        .map(|issue| InspectionIssue {
            code: verify_issue_code(issue.kind),
            message: issue.description,
            page_number: Some(issue.pgno),
        })
        .collect();
    let (replay_committed, discard_uncommitted) =
        transactions
            .into_iter()
            .fold((0, 0), |(replay, discard), transaction| {
                match transaction.disposition {
                    RecoveryDisposition::ReplayCommitted => (replay + 1, discard),
                    RecoveryDisposition::DiscardUncommitted => (replay, discard + 1),
                }
            });
    let btree_issue = btree_issue.map(|issue| InspectionIssue {
        code: match issue.kind {
            crate::inspect::BTreeVerificationIssueKind::Invalid => "BTREE_INVALID",
            crate::inspect::BTreeVerificationIssueKind::Incomplete => "BTREE_INCOMPLETE",
            crate::inspect::BTreeVerificationIssueKind::OverflowChainCorrupt => {
                "BTREE_OVERFLOW_CORRUPT"
            }
        },
        message: issue.description,
        page_number: None,
    });

    InspectionVerification {
        pages_checked: verification.pages_checked,
        pages_ok: verification.pages_ok,
        btree_checked,
        btree_ok,
        recovery: InspectionRecovery {
            wal_exists,
            replay_committed,
            discard_uncommitted,
        },
        issues,
        issues_truncated: total_issues.saturating_sub(issue_limit) as u64,
        btree_issue,
    }
}

fn verify_issue_code(kind: VerifyIssueKind) -> &'static str {
    match kind {
        VerifyIssueKind::AuthFailed => "PAGE_AUTH_FAILED",
        VerifyIssueKind::Corrupt => "PAGE_CORRUPT",
        VerifyIssueKind::Io => "PAGE_IO_FAILED",
    }
}

fn map_pages(pages: crate::inspect::PagesSummary, page_limit: usize) -> InspectionPageList {
    let total = pages.pages.len();
    let entries = pages
        .pages
        .into_iter()
        .take(page_limit)
        .map(|page| InspectionPage {
            page_number: page.pgno,
            page_version: page.page_version,
            page_type: page.page_type,
            slot_count: page.slot_count,
            state: match page.state {
                PageInspectState::Ok => InspectionPageState::Ok,
                PageInspectState::AuthFailed => InspectionPageState::AuthFailed,
                PageInspectState::Corrupt => InspectionPageState::Corrupt,
                PageInspectState::Io => InspectionPageState::Io,
            },
            issue: page.issue,
        })
        .collect();

    InspectionPageList {
        total: total as u64,
        entries,
        truncated: total.saturating_sub(page_limit) as u64,
    }
}

fn map_tree(tree: TreeSummary, limits: InspectionObservationLimits) -> InspectionTree {
    let mut nodes = Vec::new();
    let mut truncated = false;
    collect_tree_nodes(&tree.root, 0, limits, &mut nodes, &mut truncated);
    InspectionTree {
        root_page: tree.root_pgno,
        nodes,
        truncated,
    }
}

fn collect_tree_nodes(
    node: &crate::inspect::TreeNodeSummary,
    depth: usize,
    limits: InspectionObservationLimits,
    nodes: &mut Vec<InspectionTreeNode>,
    truncated: &mut bool,
) {
    if depth > limits.tree_depth_limit || nodes.len() >= limits.tree_node_limit {
        *truncated = true;
        return;
    }
    nodes.push(InspectionTreeNode {
        page_number: node.pgno,
        page_version: node.page_version,
        page_type: node.page_type,
        slot_count: node.slot_count,
        next_leaf: node.next_leaf,
        depth,
        child_count: node.children.len(),
    });
    for child in &node.children {
        collect_tree_nodes(&child.child, depth + 1, limits, nodes, truncated);
    }
}

fn map_wal(wal: crate::inspect::WalSummary, record_limit: usize) -> InspectionWal {
    let record_count = wal.records.len();
    let records = wal
        .records
        .into_iter()
        .take(record_limit)
        .map(|record| InspectionWalRecord {
            lsn: record.lsn,
            kind: match record.kind {
                WalRecordSummaryKind::Begin { txn_id } => InspectionWalRecordKind::Begin {
                    transaction_id: txn_id,
                },
                WalRecordSummaryKind::PageWrite { pgno, page_version } => {
                    InspectionWalRecordKind::PageWrite {
                        page_number: pgno,
                        page_version,
                    }
                }
                WalRecordSummaryKind::Commit { txn_id } => InspectionWalRecordKind::Commit {
                    transaction_id: txn_id,
                },
                WalRecordSummaryKind::Checkpoint { up_to_lsn } => {
                    InspectionWalRecordKind::Checkpoint { up_to_lsn }
                }
            },
        })
        .collect();
    InspectionWal {
        exists: wal.wal_exists,
        record_count: record_count as u64,
        records,
        truncated: record_count.saturating_sub(record_limit) as u64,
    }
}

fn map_keyslots(slots: Vec<(u16, u8)>, limit: usize) -> InspectionKeyslots {
    let total = slots.len();
    let slots = slots
        .into_iter()
        .take(limit)
        .map(|(slot, kind)| InspectionKeyslot { slot, kind })
        .collect();
    InspectionKeyslots {
        total: total as u64,
        slots,
        truncated: total.saturating_sub(limit) as u64,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::page_store::PageStore;

    fn observation_with_pages(
        entries: &[(u64, InspectionPageState, Option<&str>)],
    ) -> InspectionObservation {
        InspectionObservation {
            schema_version: INSPECTION_OBSERVATION_SCHEMA_V1,
            header: InspectionHeader {
                format_version: 1,
                page_size: 4096,
                page_count: entries.len() as u64,
                root_page: 1,
                flags: 0,
                keyslot_count: 0,
            },
            verification: InspectionVerification {
                pages_checked: entries.len() as u64,
                pages_ok: entries.len() as u64,
                btree_checked: true,
                btree_ok: true,
                recovery: InspectionRecovery {
                    wal_exists: false,
                    replay_committed: 0,
                    discard_uncommitted: 0,
                },
                issues: Vec::new(),
                issues_truncated: 0,
                btree_issue: None,
            },
            pages: InspectionPageList {
                total: entries.len() as u64,
                entries: entries
                    .iter()
                    .map(|(page_number, state, issue)| InspectionPage {
                        page_number: *page_number,
                        page_version: Some(1),
                        page_type: Some(1),
                        slot_count: Some(0),
                        state: *state,
                        issue: issue.map(str::to_owned),
                    })
                    .collect(),
                truncated: 0,
            },
            tree: InspectionSection::Available(InspectionTree {
                root_page: 1,
                nodes: Vec::new(),
                truncated: false,
            }),
            wal: InspectionSection::Available(InspectionWal {
                exists: false,
                record_count: 0,
                records: Vec::new(),
                truncated: 0,
            }),
            keyslots: InspectionSection::Available(InspectionKeyslots {
                total: 0,
                slots: Vec::new(),
                truncated: 0,
            }),
        }
    }

    fn new_store_path(label: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "tosumu_inspection_session_{label}_{}_{}.tsm",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&path);
        path
    }

    #[test]
    fn observation_is_bounded_and_does_not_retain_a_path() {
        let path = new_store_path("bounded");
        let mut store = PageStore::create(&path).unwrap();
        store.put(b"asset/manifest", b"schema-v1").unwrap();
        drop(store);

        let observation = inspect_observation(
            &path,
            InspectionObservationLimits {
                page_limit: 0,
                verification_issue_limit: 0,
                wal_record_limit: 0,
                tree_depth_limit: 0,
                tree_node_limit: 0,
                keyslot_limit: 0,
            },
        )
        .unwrap();

        assert_eq!(observation.schema_version, INSPECTION_OBSERVATION_SCHEMA_V1);
        assert!(observation.pages.total > 0);
        assert!(observation.pages.entries.is_empty());
        assert!(observation.pages.truncated > 0);
        assert!(
            matches!(observation.tree, InspectionSection::Available(ref tree) if tree.nodes.is_empty() && tree.truncated)
        );
        assert!(
            matches!(observation.keyslots, InspectionSection::Available(ref slots) if slots.slots.is_empty() && slots.truncated == slots.total)
        );

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn observation_uses_stable_page_numbers_not_provider_indices() {
        let path = new_store_path("page_numbers");
        let mut store = PageStore::create(&path).unwrap();
        store.put(b"asset/manifest", b"schema-v1").unwrap();
        drop(store);

        let observation =
            inspect_observation(&path, InspectionObservationLimits::default()).unwrap();
        let page = observation.pages.entries.first().unwrap();
        assert!(page.page_number >= 1);
        assert_eq!(page.state, InspectionPageState::Ok);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn unchanged_store_produces_a_deterministic_headless_observation() {
        let path = new_store_path("deterministic");
        let mut store = PageStore::create(&path).unwrap();
        store.put(b"asset/manifest", b"schema-v1").unwrap();
        drop(store);

        let first = inspect_observation(&path, InspectionObservationLimits::default()).unwrap();
        let second = inspect_observation(&path, InspectionObservationLimits::default()).unwrap();

        assert_eq!(first, second);
        assert!(matches!(first.tree, InspectionSection::Available(_)));
        assert!(matches!(first.wal, InspectionSection::Available(_)));
        assert!(matches!(first.keyslots, InspectionSection::Available(_)));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn corrupt_tree_is_an_explicit_unavailable_section_not_an_empty_tree() {
        use crate::format::OFF_ROOT_PAGE;

        let path = new_store_path("partial_tree");
        let mut store = PageStore::create(&path).unwrap();
        store.put(b"asset/manifest", b"schema-v1").unwrap();
        drop(store);

        let mut header = std::fs::read(&path).unwrap();
        header[OFF_ROOT_PAGE..OFF_ROOT_PAGE + 8].copy_from_slice(&0u64.to_le_bytes());
        std::fs::write(&path, header).unwrap();

        let observation =
            inspect_observation(&path, InspectionObservationLimits::default()).unwrap();

        assert!(!observation.pages.entries.is_empty());
        assert!(!observation.verification.btree_checked);
        assert!(matches!(
            observation.tree,
            InspectionSection::Unavailable(InspectionUnavailable {
                code: "PAGE_DECODE_CORRUPT",
                status: ErrorStatus::IntegrityFailure,
                ..
            })
        ));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn invalid_physical_store_fails_before_claiming_an_observation() {
        let path = new_store_path("invalid");
        std::fs::write(&path, b"not a tosumu file").unwrap();

        let error = inspect_observation(&path, InspectionObservationLimits::default()).unwrap_err();
        assert_eq!(error.error_report().status, ErrorStatus::ExternalFailure);

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn byte_input_observation_retains_only_header_facts() {
        let path = new_store_path("byte_input");
        let mut store = PageStore::create(&path).unwrap();
        store.put(b"asset/manifest", b"schema-v1").unwrap();
        drop(store);

        let bytes = std::fs::read(&path).unwrap();
        let observation =
            inspect_observation_from_bytes(&bytes, DEFAULT_INSPECTION_BYTE_INPUT_LIMIT).unwrap();

        assert_eq!(observation.schema_version, INSPECTION_OBSERVATION_SCHEMA_V1);
        assert!(observation.header.page_count > 0);
        assert_eq!(observation.verification.pages_checked, 0);
        assert!(observation.pages.entries.is_empty());
        assert_eq!(observation.pages.truncated, observation.pages.total);
        assert!(matches!(
            observation.tree,
            InspectionSection::Unavailable(InspectionUnavailable {
                code: "RAW_BYTES_TREE_UNAVAILABLE",
                status: ErrorStatus::Unsupported,
                ..
            })
        ));
        assert!(matches!(
            observation.wal,
            InspectionSection::Unavailable(InspectionUnavailable {
                code: "RAW_BYTES_WAL_UNAVAILABLE",
                ..
            })
        ));

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn byte_input_rejects_oversized_and_malformed_payloads() {
        let oversized = vec![0u8; 17];
        let error = inspect_observation_from_bytes(&oversized, 16).unwrap_err();
        assert_eq!(error.error_report().status, ErrorStatus::InvalidInput);

        let malformed = vec![0u8; crate::format::PAGE_SIZE];
        let error = inspect_observation_from_bytes(&malformed, DEFAULT_INSPECTION_BYTE_INPUT_LIMIT)
            .unwrap_err();
        assert_eq!(
            error.error_report().code,
            crate::error::codes::FORMAT_NOT_TOSUMU
        );
    }

    #[test]
    fn byte_input_rejects_header_versions_outside_the_supported_interval() {
        let path = new_store_path("unsupported_byte_input");
        let store = PageStore::create(&path).unwrap();
        drop(store);

        let original = std::fs::read(&path).unwrap();
        for unsupported_version in [
            crate::format::MIN_SUPPORTED_FORMAT_VERSION - 1,
            crate::format::FORMAT_VERSION + 1,
        ] {
            let mut bytes = original.clone();
            bytes[crate::format::OFF_FORMAT_VERSION..crate::format::OFF_FORMAT_VERSION + 2]
                .copy_from_slice(&unsupported_version.to_le_bytes());

            let error = inspect_observation_from_bytes(&bytes, DEFAULT_INSPECTION_BYTE_INPUT_LIMIT)
                .unwrap_err();
            assert!(matches!(
                error,
                crate::error::TosumuError::UnsupportedFormat {
                    found,
                    supported_min: crate::format::MIN_SUPPORTED_FORMAT_VERSION,
                    supported_max: crate::format::FORMAT_VERSION,
                } if found == unsupported_version
            ));
        }

        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn session_commands_select_stable_pages_and_clamp_navigation() {
        let observation = observation_with_pages(&[
            (3, InspectionPageState::Ok, None),
            (9, InspectionPageState::Corrupt, Some("checksum mismatch")),
            (42, InspectionPageState::Ok, None),
        ]);
        let mut session = InspectionSession::default();

        assert_eq!(
            apply_inspection_command(
                &mut session,
                &observation,
                InspectionCommand::SelectView(InspectionView::Tree),
            ),
            InspectionCommandOutcome::Applied(InspectionCommandEffect::ViewSelected {
                view: InspectionView::Tree,
            })
        );
        assert_eq!(
            apply_inspection_command(
                &mut session,
                &observation,
                InspectionCommand::SelectPage { page_number: 9 },
            ),
            InspectionCommandOutcome::Applied(InspectionCommandEffect::PageSelected {
                page_number: Some(9),
            })
        );
        assert_eq!(
            apply_inspection_command(
                &mut session,
                &observation,
                InspectionCommand::Navigate(PageNavigation::Next),
            ),
            InspectionCommandOutcome::Applied(InspectionCommandEffect::PageSelected {
                page_number: Some(42),
            })
        );
        assert_eq!(
            apply_inspection_command(
                &mut session,
                &observation,
                InspectionCommand::Navigate(PageNavigation::Next),
            ),
            InspectionCommandOutcome::Applied(InspectionCommandEffect::PageSelected {
                page_number: Some(42),
            })
        );
        assert_eq!(session.view, InspectionView::Tree);
        assert_eq!(session.selected_page, Some(42));
    }

    #[test]
    fn unavailable_page_is_rejected_without_mutating_the_session() {
        let observation = observation_with_pages(&[(7, InspectionPageState::Ok, None)]);
        let mut session = InspectionSession::default();
        let before = session.clone();

        let result = apply_inspection_command(
            &mut session,
            &observation,
            InspectionCommand::SelectPage { page_number: 99 },
        );

        assert!(matches!(
            result,
            InspectionCommandOutcome::Rejected(InspectionCommandRejection {
                code: "PAGE_NOT_IN_OBSERVATION",
                ..
            })
        ));
        assert_eq!(session, before);
    }

    #[test]
    fn filter_and_empty_navigation_remain_explicit() {
        let observation = observation_with_pages(&[
            (3, InspectionPageState::Ok, None),
            (9, InspectionPageState::Corrupt, Some("checksum mismatch")),
        ]);
        let mut session = InspectionSession::default();

        assert_eq!(
            apply_inspection_command(
                &mut session,
                &observation,
                InspectionCommand::SetFilter {
                    query: "corrupt".to_owned(),
                },
            ),
            InspectionCommandOutcome::Applied(InspectionCommandEffect::FilterChanged {
                query: "corrupt".to_owned(),
                match_count: 1,
            })
        );
        assert_eq!(
            apply_inspection_command(
                &mut session,
                &observation,
                InspectionCommand::SetFilter {
                    query: "nothing".to_owned(),
                },
            ),
            InspectionCommandOutcome::Applied(InspectionCommandEffect::FilterChanged {
                query: "nothing".to_owned(),
                match_count: 0,
            })
        );
        assert_eq!(
            apply_inspection_command(
                &mut session,
                &observation,
                InspectionCommand::Navigate(PageNavigation::First),
            ),
            InspectionCommandOutcome::Applied(InspectionCommandEffect::PageSelected {
                page_number: None,
            })
        );
        assert_eq!(
            apply_inspection_command(&mut session, &observation, InspectionCommand::ClearFilter),
            InspectionCommandOutcome::Applied(InspectionCommandEffect::FilterChanged {
                query: String::new(),
                match_count: 2,
            })
        );
    }

    #[test]
    fn refresh_requires_the_current_session_revision() {
        let observation = observation_with_pages(&[(1, InspectionPageState::Ok, None)]);
        let mut session = InspectionSession::default();

        assert_eq!(
            apply_inspection_command(
                &mut session,
                &observation,
                InspectionCommand::Refresh {
                    expected_revision: 0,
                },
            ),
            InspectionCommandOutcome::Applied(InspectionCommandEffect::RefreshRequested {
                next_revision: 1,
            })
        );
        assert!(matches!(
            apply_inspection_command(
                &mut session,
                &observation,
                InspectionCommand::Refresh {
                    expected_revision: 0,
                },
            ),
            InspectionCommandOutcome::Rejected(InspectionCommandRejection {
                code: "STALE_INSPECTION_REVISION",
                ..
            })
        ));
        assert_eq!(session.revision, 1);
    }
}
