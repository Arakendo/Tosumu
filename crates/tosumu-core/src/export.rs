use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::backup::create_stable_backup;
use crate::error::{Result, TosumuError};
use crate::page_store::PageStore;
use crate::wal::{checkpoint, wal_path};

/// Result of publishing a self-contained database file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PortableExportReport {
    pub source: PathBuf,
    pub destination: PathBuf,
    pub source_had_wal: bool,
    pub bytes: u64,
}

/// Create a self-contained database file without changing the source.
///
/// The source is first captured as a stable main/WAL pair. WAL recovery and
/// truncation happen only on a staging copy. The staging WAL is removed after
/// validation, and the single main file is then published with rename.
pub fn create_portable_export(source: &Path, destination: &Path) -> Result<PortableExportReport> {
    let destination_wal = wal_path(destination);
    if destination.exists() || destination_wal.exists() {
        return Err(TosumuError::InvalidArgument(
            "export destination already exists",
        ));
    }

    let staging = export_staging_path(destination);
    let staging_wal = wal_path(&staging);
    cleanup_export_temp(&staging, &staging_wal);

    let result = (|| {
        let backup = create_stable_backup(source, &staging)?;
        let source_had_wal = backup.destination_wal.is_some();

        if staging_wal.exists() {
            checkpoint(&staging, &staging_wal)?;
            std::fs::remove_file(&staging_wal)?;
        }

        let store = PageStore::open_readonly(&staging)?;
        store.scan()?;
        drop(store);
        let verification = crate::inspect::inspect_verification(&staging)?;
        if verification.pages.pages_ok != verification.pages.pages_checked
            || !verification.btree.checked
            || !verification.btree.ok
        {
            return Err(TosumuError::Corrupt {
                pgno: 0,
                reason: "portable export verification failed",
            });
        }

        let bytes = std::fs::metadata(&staging)?.len();
        std::fs::rename(&staging, destination)?;

        Ok(PortableExportReport {
            source: source.to_path_buf(),
            destination: destination.to_path_buf(),
            source_had_wal,
            bytes,
        })
    })();

    if result.is_err() {
        cleanup_export_temp(&staging, &staging_wal);
    }
    result
}

fn export_staging_path(destination: &Path) -> PathBuf {
    let file_name = destination
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("export");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    destination.with_file_name(format!(
        ".{file_name}.{}.{}.export.tmp",
        std::process::id(),
        nonce
    ))
}

fn cleanup_export_temp(staging: &Path, staging_wal: &Path) {
    let _ = std::fs::remove_file(staging);
    let _ = std::fs::remove_file(staging_wal);
}

#[cfg(test)]
mod tests {
    use super::create_portable_export;
    use crate::error::TosumuError;
    use crate::page_store::PageStore;
    use crate::wal::{wal_path, WalRecord, WalWriter};
    use std::path::PathBuf;

    fn paths(name: &str) -> (PathBuf, PathBuf) {
        let root =
            std::env::temp_dir().join(format!("tosumu_export_{name}_{}", std::process::id()));
        (
            root.with_extension("src.tsm"),
            root.with_extension("dest.tsm"),
        )
    }

    fn cleanup(path: &PathBuf) {
        let _ = std::fs::remove_file(path);
        let _ = std::fs::remove_file(wal_path(path));
    }

    #[test]
    fn portable_export_reopens_without_a_wal_sidecar() {
        let (source, destination) = paths("success");
        cleanup(&source);
        cleanup(&destination);

        let mut store = PageStore::create(&source).unwrap();
        store.put(b"asset/manifest", b"schema-v1").unwrap();
        store.put(b"asset/payload", &[0x01, 0x02, 0x03]).unwrap();
        drop(store);
        let source_bytes = std::fs::read(&source).unwrap();
        let source_wal = std::fs::read(wal_path(&source)).unwrap();

        let report = create_portable_export(&source, &destination).unwrap();
        assert_eq!(report.source, source);
        assert_eq!(report.destination, destination);
        assert!(report.source_had_wal);
        assert_eq!(report.bytes, std::fs::metadata(&destination).unwrap().len());
        assert!(!wal_path(&destination).exists());

        let exported = PageStore::open_readonly(&destination).unwrap();
        assert_eq!(
            exported.scan().unwrap(),
            vec![
                (b"asset/manifest".to_vec(), b"schema-v1".to_vec()),
                (b"asset/payload".to_vec(), vec![0x01, 0x02, 0x03]),
            ]
        );
        assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
        assert_eq!(std::fs::read(wal_path(&source)).unwrap(), source_wal);

        cleanup(&source);
        cleanup(&destination);
    }

    #[test]
    fn portable_export_rejects_existing_destination_without_replacement() {
        let (source, destination) = paths("existing");
        cleanup(&source);
        cleanup(&destination);

        PageStore::create(&source).unwrap();
        std::fs::write(&destination, b"sentinel").unwrap();
        let error = create_portable_export(&source, &destination).unwrap_err();
        assert!(matches!(error, TosumuError::InvalidArgument(_)));
        assert_eq!(std::fs::read(&destination).unwrap(), b"sentinel");

        cleanup(&source);
        cleanup(&destination);
    }

    #[test]
    fn portable_export_reconciliation_failure_publishes_no_destination_or_staging_file() {
        let (source, destination) = paths("corrupt_wal");
        cleanup(&source);
        cleanup(&destination);

        let mut store = PageStore::create(&source).unwrap();
        store.put(b"asset/manifest", b"schema-v1").unwrap();
        drop(store);

        let source_wal = wal_path(&source);
        let mut writer = WalWriter::open(&source_wal).unwrap();
        writer.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        writer.sync().unwrap();
        drop(writer);
        let mut wal_bytes = std::fs::read(&source_wal).unwrap();
        *wal_bytes.last_mut().unwrap() ^= 0xff;
        std::fs::write(&source_wal, wal_bytes).unwrap();

        let error = create_portable_export(&source, &destination).unwrap_err();
        assert!(matches!(error, TosumuError::CorruptRecord { .. }));
        assert!(!destination.exists());
        assert!(!wal_path(&destination).exists());

        let staging_prefix = format!(".{}.", destination.file_name().unwrap().to_string_lossy());
        let staged_files: Vec<_> = std::fs::read_dir(destination.parent().unwrap())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.starts_with(&staging_prefix) && name.ends_with(".export.tmp"))
            .collect();
        assert!(
            staged_files.is_empty(),
            "staging files remain: {staged_files:?}"
        );

        cleanup(&source);
        cleanup(&destination);
    }
}
