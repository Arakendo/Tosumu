use std::fs::File;
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::error::{Result, TosumuError};
use crate::wal::wal_path;

const MAX_BACKUP_ATTEMPTS: u32 = 5;

/// Result of a stable main-file and optional WAL-sidecar snapshot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BackupReport {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub destination_wal: Option<PathBuf>,
    pub attempts: u32,
}

/// Capture a stable copy of a database and its WAL sidecar.
///
/// The source is copied twice and the two main/WAL pairs must match before the
/// staged files are published. Existing destination files are never replaced.
/// A changing source returns [`TosumuError::FileBusy`] after bounded retries.
pub fn create_stable_backup(source: &Path, destination: &Path) -> Result<BackupReport> {
    let destination_wal = wal_path(destination);
    if destination.exists() || destination_wal.exists() {
        return Err(TosumuError::InvalidArgument(
            "backup destination already exists",
        ));
    }

    let staged_main = backup_temp_path(destination, "main");
    let staged_wal = backup_temp_path(&destination_wal, "wal");
    let probe_main = backup_temp_path(destination, "main-probe");
    let probe_wal = backup_temp_path(&destination_wal, "wal-probe");
    cleanup_backup_temp(&staged_main, &staged_wal);
    cleanup_backup_temp(&probe_main, &probe_wal);

    let source_wal = wal_path(source);
    let mut copied_wal = false;
    let mut stable = false;
    let mut attempts = 0;

    for attempt in 1..=MAX_BACKUP_ATTEMPTS {
        attempts = attempt;
        cleanup_backup_temp(&staged_main, &staged_wal);
        cleanup_backup_temp(&probe_main, &probe_wal);

        std::fs::copy(source, &staged_main).map_err(|error| {
            cleanup_backup_temp(&staged_main, &staged_wal);
            TosumuError::Io(error)
        })?;
        let copied_wal_a = copy_optional_file(&source_wal, &staged_wal).map_err(|error| {
            cleanup_backup_temp(&staged_main, &staged_wal);
            error
        })?;

        std::fs::copy(source, &probe_main).map_err(|error| {
            cleanup_backup_temp(&staged_main, &staged_wal);
            TosumuError::Io(error)
        })?;
        let copied_wal_b = copy_optional_file(&source_wal, &probe_wal).map_err(|error| {
            cleanup_backup_temp(&staged_main, &staged_wal);
            cleanup_backup_temp(&probe_main, &probe_wal);
            error
        })?;

        let wal_matches = copied_wal_a == copied_wal_b
            && (!copied_wal_a
                || files_equal(&staged_wal, &probe_wal).map_err(|error| {
                    cleanup_backup_temp(&staged_main, &staged_wal);
                    cleanup_backup_temp(&probe_main, &probe_wal);
                    TosumuError::Io(error)
                })?);
        let main_matches = files_equal(&staged_main, &probe_main).map_err(|error| {
            cleanup_backup_temp(&staged_main, &staged_wal);
            cleanup_backup_temp(&probe_main, &probe_wal);
            TosumuError::Io(error)
        })?;

        if main_matches && wal_matches {
            copied_wal = copied_wal_a;
            stable = true;
            break;
        }
    }

    cleanup_backup_temp(&probe_main, &probe_wal);

    if !stable {
        cleanup_backup_temp(&staged_main, &staged_wal);
        return Err(TosumuError::FileBusy {
            path: source.to_path_buf(),
            operation: "capturing a stable backup snapshot",
        });
    }

    if copied_wal {
        std::fs::rename(&staged_wal, &destination_wal).map_err(|error| {
            let _ = std::fs::remove_file(&staged_main);
            let _ = std::fs::remove_file(&staged_wal);
            TosumuError::Io(error)
        })?;
    }

    std::fs::rename(&staged_main, destination).map_err(|error| {
        let _ = std::fs::remove_file(&staged_main);
        if copied_wal {
            let _ = std::fs::remove_file(&destination_wal);
        }
        TosumuError::Io(error)
    })?;

    Ok(BackupReport {
        source: source.to_path_buf(),
        destination: destination.to_path_buf(),
        destination_wal: copied_wal.then_some(destination_wal),
        attempts,
    })
}

fn backup_temp_path(destination: &Path, kind: &str) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("backup");
    destination.with_file_name(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        kind
    ))
}

fn copy_optional_file(source: &Path, destination: &Path) -> Result<bool> {
    let _ = std::fs::remove_file(destination);
    if source.exists() {
        std::fs::copy(source, destination)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

fn cleanup_backup_temp(main: &Path, wal: &Path) {
    let _ = std::fs::remove_file(main);
    let _ = std::fs::remove_file(wal);
}

fn files_equal(first: &Path, second: &Path) -> std::io::Result<bool> {
    let first_meta = std::fs::metadata(first)?;
    let second_meta = std::fs::metadata(second)?;
    if first_meta.len() != second_meta.len() {
        return Ok(false);
    }

    let mut first_file = File::open(first)?;
    let mut second_file = File::open(second)?;
    let mut first_buffer = [0u8; 8192];
    let mut second_buffer = [0u8; 8192];

    loop {
        let first_read = first_file.read(&mut first_buffer)?;
        let second_read = second_file.read(&mut second_buffer)?;
        if first_read != second_read {
            return Ok(false);
        }
        if first_read == 0 {
            return Ok(true);
        }
        if first_buffer[..first_read] != second_buffer[..second_read] {
            return Ok(false);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::create_stable_backup;
    use crate::error::TosumuError;
    use crate::page_store::PageStore;
    use std::path::PathBuf;

    fn paths(name: &str) -> (PathBuf, PathBuf) {
        let root = std::env::temp_dir().join(format!("tosumu_backup_{name}_{}", std::process::id()));
        (root.with_extension("src.tsm"), root.with_extension("dest.tsm"))
    }

    fn cleanup(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(crate::wal::wal_path(path));
    }

    #[test]
    fn stable_backup_copies_database_and_reports_paths() {
        let (source, destination) = paths("success");
        cleanup(&source);
        cleanup(&destination);

        let mut store = PageStore::create(&source).unwrap();
        store.put(b"asset/manifest", b"schema-v1").unwrap();
        drop(store);

        let report = create_stable_backup(&source, &destination).unwrap();
        assert_eq!(report.source, source);
        assert_eq!(report.destination, destination);
        assert_eq!(report.attempts, 1);
        assert!(report.destination_wal.is_some());

        let copied = PageStore::open(&destination).unwrap();
        assert_eq!(copied.get(b"asset/manifest").unwrap(), Some(b"schema-v1".to_vec()));

        cleanup(&source);
        cleanup(&destination);
    }

    #[test]
    fn stable_backup_rejects_existing_destination_without_replacement() {
        let (source, destination) = paths("existing");
        cleanup(&source);
        cleanup(&destination);

        PageStore::create(&source).unwrap();
        std::fs::write(&destination, b"sentinel").unwrap();
        let error = create_stable_backup(&source, &destination).unwrap_err();
        assert!(matches!(error, TosumuError::InvalidArgument(_)));
        assert_eq!(std::fs::read(&destination).unwrap(), b"sentinel");

        cleanup(&source);
        cleanup(&destination);
    }
}
