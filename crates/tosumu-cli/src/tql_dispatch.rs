//! Read-only execution adapter for the incubating TQL parser.
//!
//! The adapter accepts public provider and inspection facts explicitly. It does
//! not open files, invoke physical storage internals, or render terminal text.

use tosumu_core::inspect::{VerificationReport, WalSummary};
use tosumu_core::{KvStore, TosumuError};

use crate::tql::TqlCommand;

/// Result of a TQL command before any CLI or JSON presentation is applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TqlOutcome {
    Status(StatusOutcome),
    Check(CheckOutcome),
    Description(DescriptionOutcome),
    WalStatus(WalStatusOutcome),
}

/// Source-backed store facts exposed by `STATUS`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct StatusOutcome {
    pub page_count: u64,
    pub data_pages: u64,
    pub tree_height: usize,
}

/// Whether a supplied verification snapshot establishes a particular fact.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum CheckState {
    Passed,
    Failed,
    NotChecked,
}

/// Source-backed integrity facts exposed by `CHECK`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct CheckOutcome {
    pub page_integrity: CheckState,
    pub pages_checked: u64,
    pub pages_ok: u64,
    pub page_issue_count: usize,
    pub tree_integrity: CheckState,
}

/// Safe value-presence metadata exposed by `DESCRIBE`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum DescriptionOutcome {
    Found { key: String, value_bytes: usize },
    Missing { key: String },
}

/// Bounded WAL facts exposed by `WAL STATUS`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct WalStatusOutcome {
    pub wal_exists: bool,
    pub record_count: usize,
}

/// Executes one parsed read-only command against explicit public capability
/// inputs. `CHECK` reports `NotChecked` when no verification snapshot is
/// supplied rather than inventing an integrity result.
pub(crate) fn execute(
    command: &TqlCommand,
    store: &KvStore,
    verification: Option<&VerificationReport>,
    wal: Option<&WalSummary>,
) -> Result<TqlOutcome, TosumuError> {
    match command {
        TqlCommand::Status => {
            let stat = store.stat()?;
            Ok(TqlOutcome::Status(StatusOutcome {
                page_count: stat.page_count,
                data_pages: stat.data_pages,
                tree_height: stat.tree_height,
            }))
        }
        TqlCommand::Check => Ok(TqlOutcome::Check(check_outcome(verification))),
        TqlCommand::Describe { key } => match store.get(key.as_bytes())? {
            Some(value) => Ok(TqlOutcome::Description(DescriptionOutcome::Found {
                key: key.clone(),
                value_bytes: value.len(),
            })),
            None => Ok(TqlOutcome::Description(DescriptionOutcome::Missing {
                key: key.clone(),
            })),
        },
        TqlCommand::WalStatus => {
            let wal = wal.ok_or(TosumuError::InvalidArgument(
                "WAL STATUS requires a public WAL summary",
            ))?;
            Ok(TqlOutcome::WalStatus(WalStatusOutcome {
                wal_exists: wal.wal_exists,
                record_count: wal.records.len(),
            }))
        }
    }
}

fn check_outcome(verification: Option<&VerificationReport>) -> CheckOutcome {
    let Some(verification) = verification else {
        return CheckOutcome {
            page_integrity: CheckState::NotChecked,
            pages_checked: 0,
            pages_ok: 0,
            page_issue_count: 0,
            tree_integrity: CheckState::NotChecked,
        };
    };

    CheckOutcome {
        page_integrity: if verification.pages.issues.is_empty() {
            CheckState::Passed
        } else {
            CheckState::Failed
        },
        pages_checked: verification.pages.pages_checked,
        pages_ok: verification.pages.pages_ok,
        page_issue_count: verification.pages.issues.len(),
        tree_integrity: match (verification.btree.checked, verification.btree.ok) {
            (false, _) => CheckState::NotChecked,
            (true, true) => CheckState::Passed,
            (true, false) => CheckState::Failed,
        },
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::io::{Read, Seek, SeekFrom, Write};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    use tosumu_core::inspect::{inspect_verification, inspect_wal};

    use super::*;

    fn test_path(label: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("tosumu-tql-{label}-{nonce}.tsm"))
    }

    #[test]
    fn read_only_dispatch_returns_source_facts_without_mutating_the_store() {
        let path = test_path("read-only");
        {
            let mut writable = KvStore::create(&path).expect("create test store");
            writable
                .put(b"player/42", b"payload")
                .expect("write test value");
        }
        let before = fs::read(&path).expect("read database before dispatch");
        let wal_path = tosumu_core::wal::wal_path(&path);
        let wal_before = fs::read(&wal_path).expect("read WAL before dispatch");
        let store = KvStore::open_readonly(&path).expect("open read-only store");
        let verification = inspect_verification(&path).expect("inspect healthy store");
        let wal = inspect_wal(&path).expect("inspect public WAL summary");

        let status = execute(&TqlCommand::Status, &store, Some(&verification), None)
            .expect("status should read public facts");
        assert!(matches!(
            status,
            TqlOutcome::Status(StatusOutcome {
                page_count: 2,
                data_pages: 1,
                tree_height: 1,
            })
        ));

        let check = execute(&TqlCommand::Check, &store, Some(&verification), None)
            .expect("check should map the supplied verification snapshot");
        assert_eq!(
            check,
            TqlOutcome::Check(CheckOutcome {
                page_integrity: CheckState::Passed,
                pages_checked: verification.pages.pages_checked,
                pages_ok: verification.pages.pages_ok,
                page_issue_count: 0,
                tree_integrity: CheckState::Passed,
            })
        );

        let described = execute(
            &TqlCommand::Describe {
                key: "player/42".to_owned(),
            },
            &store,
            Some(&verification),
            None,
        )
        .expect("describe should read value metadata");
        assert_eq!(
            described,
            TqlOutcome::Description(DescriptionOutcome::Found {
                key: "player/42".to_owned(),
                value_bytes: 7,
            })
        );

        let missing = execute(
            &TqlCommand::Describe {
                key: "missing".to_owned(),
            },
            &store,
            Some(&verification),
            None,
        )
        .expect("missing describe should remain a successful observation");
        assert_eq!(
            missing,
            TqlOutcome::Description(DescriptionOutcome::Missing {
                key: "missing".to_owned(),
            })
        );

        let wal_status = execute(
            &TqlCommand::WalStatus,
            &store,
            Some(&verification),
            Some(&wal),
        )
        .expect("WAL STATUS should read the supplied public WAL summary");
        assert_eq!(
            wal_status,
            TqlOutcome::WalStatus(WalStatusOutcome {
                wal_exists: wal.wal_exists,
                record_count: wal.records.len(),
            })
        );

        let not_checked = execute(&TqlCommand::Check, &store, None, None)
            .expect("missing verification is an explicit state, not an error");
        assert_eq!(
            not_checked,
            TqlOutcome::Check(CheckOutcome {
                page_integrity: CheckState::NotChecked,
                pages_checked: 0,
                pages_ok: 0,
                page_issue_count: 0,
                tree_integrity: CheckState::NotChecked,
            })
        );

        let missing_wal_summary = execute(&TqlCommand::WalStatus, &store, None, None)
            .expect_err("WAL STATUS must require the explicit public summary");
        assert!(matches!(
            missing_wal_summary,
            TosumuError::InvalidArgument(_)
        ));

        drop(store);
        assert_eq!(
            fs::read(&path).expect("read database after dispatch"),
            before,
            "read-only commands must not mutate database bytes"
        );
        assert_eq!(
            fs::read(&wal_path).expect("read WAL after dispatch"),
            wal_before,
            "read-only commands must not mutate WAL bytes"
        );
        fs::remove_file(path).expect("remove temporary database");
        fs::remove_file(wal_path).expect("remove temporary WAL");
    }

    #[test]
    fn check_maps_a_real_integrity_failure_without_claiming_tree_results() {
        let path = test_path("integrity-failure");
        {
            let mut writable = KvStore::create(&path).expect("create test store");
            writable
                .put(b"asset/manifest", b"fixture-schema-v1")
                .expect("write test value");
        }
        flip_byte_at(&path, tosumu_core::format::PAGE_SIZE as u64);

        let verification = inspect_verification(&path).expect("inspect corrupted store");
        assert!(!verification.pages.issues.is_empty());
        assert!(!verification.btree.checked);
        assert_eq!(
            check_outcome(Some(&verification)),
            CheckOutcome {
                page_integrity: CheckState::Failed,
                pages_checked: verification.pages.pages_checked,
                pages_ok: verification.pages.pages_ok,
                page_issue_count: verification.pages.issues.len(),
                tree_integrity: CheckState::NotChecked,
            }
        );

        let wal_path = tosumu_core::wal::wal_path(&path);
        fs::remove_file(path).expect("remove temporary database");
        fs::remove_file(wal_path).expect("remove temporary WAL");
    }

    fn flip_byte_at(path: &std::path::Path, offset: u64) {
        let mut file = fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)
            .expect("open database for fixture corruption");
        file.seek(SeekFrom::Start(offset))
            .expect("seek to fixture byte");
        let mut byte = [0u8; 1];
        file.read_exact(&mut byte).expect("read fixture byte");
        byte[0] ^= 0x01;
        file.seek(SeekFrom::Start(offset))
            .expect("seek to fixture byte");
        file.write_all(&byte).expect("write fixture corruption");
        file.flush().expect("flush fixture corruption");
    }
}
