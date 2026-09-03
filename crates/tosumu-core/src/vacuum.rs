use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::error::{Result, TosumuError};
use crate::vacuum_publication::{
    ensure_supported, replace_database, PublicationDurability, PublicationError,
};
use crate::vacuum_rebuild::{
    ensure_staging_space, open_guarded_source, rebuild_and_verify_staging, verify_source,
    VacuumUnlock,
};
use crate::wal::wal_path;
use crate::writer_gate::{writer_lock_path, WriterGuard};

/// Observations from one completed offline VACUUM.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VacuumReport {
    pub source: PathBuf,
    pub logical_records: u64,
    pub bytes_before: u64,
    pub bytes_after: u64,
    pub pages_before: u64,
    pub pages_after: u64,
    pub publication_durably_confirmed: bool,
}

/// Rebuild and replace a sentinel-protected database while holding its writer
/// gate. This operation is unavailable on platforms lacking the publication
/// guarantees required by ADR-0009.
pub fn vacuum(source: &Path) -> Result<VacuumReport> {
    vacuum_inner(source, VacuumUnlock::Sentinel)
}

/// Rebuild and replace a database unlocked by `passphrase`.
pub fn vacuum_with_passphrase(source: &Path, passphrase: &str) -> Result<VacuumReport> {
    vacuum_inner(source, VacuumUnlock::Passphrase(passphrase))
}

/// Rebuild and replace a database unlocked by a recovery key.
pub fn vacuum_with_recovery_key(source: &Path, recovery_key: &str) -> Result<VacuumReport> {
    vacuum_inner(source, VacuumUnlock::RecoveryKey(recovery_key))
}

/// Rebuild and replace a database unlocked by a keyfile protector.
pub fn vacuum_with_keyfile(source: &Path, keyfile: &Path) -> Result<VacuumReport> {
    vacuum_inner(source, VacuumUnlock::Keyfile(keyfile))
}

fn vacuum_inner(source: &Path, unlock: VacuumUnlock<'_>) -> Result<VacuumReport> {
    // This must remain the first operation: unsupported targets cannot acquire
    // a gate, checkpoint WAL, or create staging artifacts.
    ensure_supported().map_err(map_publication_error)?;
    vacuum_supported(source, unlock)
}

fn vacuum_supported(source: &Path, unlock: VacuumUnlock<'_>) -> Result<VacuumReport> {
    vacuum_supported_with_publisher(source, unlock, replace_database, || Ok(()))
}

fn vacuum_supported_with_publisher(
    source: &Path,
    unlock: VacuumUnlock<'_>,
    publish: impl FnOnce(&Path, &Path) -> std::result::Result<PublicationDurability, PublicationError>,
    before_source_open: impl FnOnce() -> Result<()>,
) -> Result<VacuumReport> {
    let staging = staging_path(source);
    let staging_wal = wal_path(&staging);
    let staging_lock = writer_lock_path(&staging);

    let mut owns_staging = false;
    let result = (|| {
        let writer_guard = WriterGuard::acquire(source)?;
        before_source_open()?;
        let mut source_store = open_guarded_source(source, unlock, &writer_guard)?;
        let bytes_before = std::fs::metadata(source)?.len();
        verify_source(source, unlock)?;
        ensure_staging_space(source, &staging)?;

        let rebuilt = rebuild_and_verify_staging(&mut source_store, &staging, unlock)?;
        owns_staging = true;
        drop(source_store);

        let source_wal = wal_path(source);
        if source_wal.exists() && std::fs::metadata(&source_wal)?.len() != 0 {
            return Err(TosumuError::Io(std::io::Error::other(
                "VACUUM source WAL is not empty before publication",
            )));
        }

        let bytes_after = std::fs::metadata(&staging)?.len();
        remove_file_if_present(&staging_wal)?;
        remove_file_if_present(&staging_lock)?;

        let durability = publish(&staging, source).map_err(map_publication_error)?;
        drop(writer_guard);

        Ok(VacuumReport {
            source: source.to_path_buf(),
            logical_records: rebuilt.logical_records,
            bytes_before,
            bytes_after,
            pages_before: rebuilt.source_pages,
            pages_after: rebuilt.staging_pages,
            publication_durably_confirmed: durability == PublicationDurability::Confirmed,
        })
    })();

    // These are uniquely named, recognized VACUUM artifacts. After successful
    // replacement the staging main name is already absent. After a
    // durability-uncertain result this cleanup never touches the new source.
    if owns_staging {
        cleanup_staging(&staging, &staging_wal, &staging_lock);
    }
    result
}

fn remove_file_if_present(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error.into()),
    }
}

fn cleanup_staging(staging: &Path, staging_wal: &Path, staging_lock: &Path) {
    let _ = std::fs::remove_file(staging);
    let _ = std::fs::remove_file(staging_wal);
    let _ = std::fs::remove_file(staging_lock);
}

fn staging_path(source: &Path) -> PathBuf {
    let file_name = source
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("database");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    source.with_file_name(format!(
        ".{file_name}.{}.{}.vacuum.tmp",
        std::process::id(),
        nonce
    ))
}

fn map_publication_error(error: PublicationError) -> TosumuError {
    match error {
        PublicationError::UnsupportedPlatform { platform } => {
            TosumuError::VacuumPlatformUnsupported { platform }
        }
        PublicationError::PrePublication { source, .. } => TosumuError::Io(source),
        PublicationError::DurabilityUncertain {
            destination,
            source,
        } => TosumuError::VacuumDurabilityUncertain {
            path: destination,
            source,
        },
    }
}

#[cfg(test)]
mod tests {
    use crate::error::TosumuError;
    use crate::page_store::PageStore;
    use std::cell::Cell;
    use std::io;
    use std::path::Path;

    #[cfg(any(unix, windows))]
    #[test]
    fn replacement_failure_keeps_source_authoritative_cleans_staging_and_retains_gate() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.tsm");
        let mut store = PageStore::create(&source).unwrap();
        store.put(b"key", b"old value").unwrap();
        drop(store);
        let gate_observed = Cell::new(false);

        let error = super::vacuum_supported_with_publisher(
            &source,
            crate::vacuum_rebuild::VacuumUnlock::Sentinel,
            |_, destination| {
                assert!(matches!(
                    crate::writer_gate::WriterGuard::acquire(destination),
                    Err(TosumuError::FileBusy { .. })
                ));
                gate_observed.set(true);
                Err(
                    crate::vacuum_publication::PublicationError::PrePublication {
                        operation: "injected replacement failure",
                        source: io::Error::new(io::ErrorKind::PermissionDenied, "injected"),
                    },
                )
            },
            || Ok(()),
        )
        .unwrap_err();

        assert!(matches!(error, TosumuError::Io(_)));
        assert!(gate_observed.get());
        assert_eq!(
            PageStore::open_readonly(&source)
                .unwrap()
                .get(b"key")
                .unwrap(),
            Some(b"old value".to_vec())
        );
        assert!(vacuum_staging_entries(directory.path()).is_empty());
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn post_replacement_uncertainty_never_restores_the_old_source() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.tsm");
        let mut store = PageStore::create(&source).unwrap();
        for index in 0..40u32 {
            store
                .put(
                    format!("key-{index:04}").as_bytes(),
                    &vec![index as u8; 700],
                )
                .unwrap();
        }
        for index in 0..30u32 {
            store.delete(format!("key-{index:04}").as_bytes()).unwrap();
        }
        let expected = store.scan().unwrap();
        drop(store);

        let error = super::vacuum_supported_with_publisher(
            &source,
            crate::vacuum_rebuild::VacuumUnlock::Sentinel,
            |staging, destination| {
                std::fs::rename(staging, destination).unwrap();
                Err(
                    crate::vacuum_publication::PublicationError::DurabilityUncertain {
                        destination: destination.to_path_buf(),
                        source: io::Error::other("injected directory sync failure"),
                    },
                )
            },
            || Ok(()),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            TosumuError::VacuumDurabilityUncertain { .. }
        ));
        assert_eq!(
            PageStore::open_readonly(&source).unwrap().scan().unwrap(),
            expected
        );
        assert!(vacuum_staging_entries(directory.path()).is_empty());
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn failure_before_source_open_prevents_checkpoint_and_staging() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.tsm");
        let mut store = PageStore::create(&source).unwrap();
        store.put(b"key", b"value").unwrap();
        drop(store);
        let source_before = std::fs::read(&source).unwrap();
        let wal = crate::wal::wal_path(&source);
        let wal_before = std::fs::read(&wal).unwrap();

        let error = super::vacuum_supported_with_publisher(
            &source,
            crate::vacuum_rebuild::VacuumUnlock::Sentinel,
            |_, _| panic!("publication must not run"),
            || {
                assert!(matches!(
                    crate::writer_gate::WriterGuard::acquire(&source),
                    Err(TosumuError::FileBusy { .. })
                ));
                Err(TosumuError::Io(io::Error::other(
                    "injected before source open",
                )))
            },
        )
        .unwrap_err();

        assert!(matches!(error, TosumuError::Io(_)));
        assert_eq!(std::fs::read(&source).unwrap(), source_before);
        assert_eq!(std::fs::read(wal).unwrap(), wal_before);
        assert!(vacuum_staging_entries(directory.path()).is_empty());
    }

    #[cfg(any(unix, windows))]
    #[test]
    fn invalid_source_structure_is_rejected_before_staging_creation() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.tsm");
        let mut store = PageStore::create(&source).unwrap();
        store.put(b"key", b"value").unwrap();
        drop(store);

        let mut bytes = std::fs::read(&source).unwrap();
        bytes[crate::format::PAGE_SIZE + 64] ^= 1;
        std::fs::write(&source, &bytes).unwrap();

        let error = super::vacuum_supported_with_publisher(
            &source,
            crate::vacuum_rebuild::VacuumUnlock::Sentinel,
            |_, _| panic!("publication must not run"),
            || Ok(()),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            TosumuError::Corrupt {
                pgno: 0,
                reason: "VACUUM source structured verification failed"
            }
        ));
        assert_eq!(std::fs::read(&source).unwrap(), bytes);
        assert!(vacuum_staging_entries(directory.path()).is_empty());
    }

    #[cfg(any(unix, windows))]
    fn vacuum_staging_entries(directory: &Path) -> Vec<std::fs::DirEntry> {
        std::fs::read_dir(directory)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().contains("vacuum.tmp"))
            .collect()
    }

    #[cfg(not(unix))]
    #[test]
    fn unsupported_platform_refuses_before_source_or_sidecar_mutation() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.tsm");
        let mut store = PageStore::create(&source).unwrap();
        store.put(b"key", b"value").unwrap();
        drop(store);
        let source_before = std::fs::read(&source).unwrap();
        let wal = crate::wal::wal_path(&source);
        let wal_before = std::fs::read(&wal).unwrap();
        let lock = crate::writer_gate::writer_lock_path(&source);
        let lock_before = std::fs::read(&lock).unwrap();

        let error = super::vacuum(&source).unwrap_err();

        assert!(matches!(
            error,
            TosumuError::VacuumPlatformUnsupported { .. }
        ));
        assert_eq!(std::fs::read(&source).unwrap(), source_before);
        assert_eq!(std::fs::read(wal).unwrap(), wal_before);
        assert_eq!(std::fs::read(lock).unwrap(), lock_before);
        let staged: Vec<_> = std::fs::read_dir(directory.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().contains("vacuum.tmp"))
            .collect();
        assert!(staged.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn unix_vacuum_reclaims_pages_and_preserves_logical_records() {
        let directory = tempfile::tempdir().unwrap();
        let source = directory.path().join("source.tsm");
        let mut store = crate::page_store::PageStore::create(&source).unwrap();
        for index in 0..100u32 {
            store
                .put(
                    format!("key-{index:04}").as_bytes(),
                    &vec![index as u8; 1_200],
                )
                .unwrap();
        }
        for index in 0..90u32 {
            store.delete(format!("key-{index:04}").as_bytes()).unwrap();
        }
        let expected = store.scan().unwrap();
        drop(store);

        let report = super::vacuum(&source).unwrap();

        assert_eq!(report.logical_records, expected.len() as u64);
        assert!(report.pages_after < report.pages_before);
        assert!(report.bytes_after < report.bytes_before);
        assert!(report.publication_durably_confirmed);
        assert_eq!(
            crate::page_store::PageStore::open_readonly(&source)
                .unwrap()
                .scan()
                .unwrap(),
            expected
        );
        assert!(crate::writer_gate::writer_lock_path(&source).exists());
    }
}
