use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::{Result, TosumuError};
use crate::inspect::{
    inspect_verification, inspect_verification_with_keyfile, inspect_verification_with_passphrase,
    inspect_verification_with_recovery_key, VerificationReport,
};
use crate::page_store::PageStore;
use crate::pager::OpenUnlock;
use crate::wal::wal_path;
use crate::writer_gate::WriterGuard;

#[derive(Clone, Copy)]
pub(crate) enum VacuumUnlock<'a> {
    Sentinel,
    Passphrase(&'a str),
    RecoveryKey(&'a str),
    Keyfile(&'a Path),
}

impl<'a> VacuumUnlock<'a> {
    pub(crate) fn pager_unlock(self) -> OpenUnlock<'a> {
        match self {
            Self::Sentinel => OpenUnlock::Sentinel,
            Self::Passphrase(passphrase) => OpenUnlock::Passphrase(passphrase),
            Self::RecoveryKey(recovery_key) => OpenUnlock::RecoveryKey(recovery_key),
            Self::Keyfile(keyfile) => OpenUnlock::Keyfile(keyfile),
        }
    }

    fn open_readonly(self, path: &Path) -> Result<PageStore> {
        match self {
            Self::Sentinel => PageStore::open_readonly(path),
            Self::Passphrase(passphrase) => {
                PageStore::open_with_passphrase_readonly(path, passphrase)
            }
            Self::RecoveryKey(recovery_key) => {
                PageStore::open_with_recovery_key_readonly(path, recovery_key)
            }
            Self::Keyfile(keyfile) => PageStore::open_with_keyfile_readonly(path, keyfile),
        }
    }

    fn verify(self, path: &Path) -> Result<VerificationReport> {
        match self {
            Self::Sentinel => inspect_verification(path),
            Self::Passphrase(passphrase) => inspect_verification_with_passphrase(path, passphrase),
            Self::RecoveryKey(recovery_key) => {
                inspect_verification_with_recovery_key(path, recovery_key)
            }
            Self::Keyfile(keyfile) => inspect_verification_with_keyfile(path, keyfile),
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(crate) struct StagingRebuild {
    pub(crate) logical_records: u64,
    pub(crate) source_pages: u64,
    pub(crate) staging_pages: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum RebuildPhase {
    StagingCreation,
    RecordCopy(u64),
    Verification,
}

pub(crate) fn open_guarded_source(
    source_path: &Path,
    unlock: VacuumUnlock<'_>,
    writer_guard: &WriterGuard,
) -> Result<PageStore> {
    PageStore::open_with_writer_guard(source_path, unlock.pager_unlock(), writer_guard)
}

/// Copy one logical record per transaction. This deliberately favors a simple
/// fixed upper bound over throughput: because one legal Tosumu value fits the
/// transaction WAL budget, no batch can exceed that budget merely due to the
/// number of records selected by VACUUM.
pub(crate) fn rebuild_and_verify_staging(
    source: &mut PageStore,
    staging_path: &Path,
    unlock: VacuumUnlock<'_>,
) -> Result<StagingRebuild> {
    rebuild_and_verify_staging_with_observer(source, staging_path, unlock, |_| Ok(()))
}

fn rebuild_and_verify_staging_with_observer(
    source: &mut PageStore,
    staging_path: &Path,
    unlock: VacuumUnlock<'_>,
    mut observe: impl FnMut(RebuildPhase) -> Result<()>,
) -> Result<StagingRebuild> {
    if staging_path.exists() || wal_path(staging_path).exists() {
        return Err(TosumuError::InvalidArgument(
            "VACUUM staging path already exists",
        ));
    }

    let source_pages = source.stat()?.page_count;
    let source_records = source.scan()?;
    let source_digest = logical_digest(&source_records);
    let context = source.rebuild_context()?;
    observe(RebuildPhase::StagingCreation)?;

    let mut owns_staging = false;
    let result = (|| {
        let mut staging = PageStore::create_rebuild_staging(staging_path, &context)?;
        owns_staging = true;
        for (record_index, (key, value)) in source_records.iter().enumerate() {
            observe(RebuildPhase::RecordCopy(record_index as u64))?;
            staging.put(key, value)?;
        }
        let staging_pages = staging.stat()?.page_count;
        drop(staging);

        let staging_wal = wal_path(staging_path);
        if staging_wal.exists() && std::fs::metadata(&staging_wal)?.len() != 0 {
            return Err(TosumuError::Io(std::io::Error::other(
                "VACUUM staging WAL is not empty after rebuild",
            )));
        }

        observe(RebuildPhase::Verification)?;

        let reopened = unlock.open_readonly(staging_path)?;
        let staged_records = reopened.scan()?;
        drop(reopened);
        if staged_records.len() != source_records.len()
            || logical_digest(&staged_records) != source_digest
        {
            return Err(TosumuError::Corrupt {
                pgno: 0,
                reason: "VACUUM staging logical verification failed",
            });
        }

        let verification = unlock.verify(staging_path)?;
        if verification.pages.pages_ok != verification.pages.pages_checked
            || !verification.btree.checked
            || !verification.btree.ok
        {
            return Err(TosumuError::Corrupt {
                pgno: 0,
                reason: "VACUUM staging structured verification failed",
            });
        }

        Ok(StagingRebuild {
            logical_records: source_records.len() as u64,
            source_pages,
            staging_pages,
        })
    })();

    if result.is_err() && owns_staging {
        cleanup_owned_staging(staging_path);
    }
    result
}

fn cleanup_owned_staging(staging_path: &Path) {
    let _ = std::fs::remove_file(staging_path);
    let _ = std::fs::remove_file(wal_path(staging_path));
    let _ = std::fs::remove_file(crate::writer_gate::writer_lock_path(staging_path));
}

fn logical_digest(records: &[(Vec<u8>, Vec<u8>)]) -> [u8; 32] {
    let mut digest = Sha256::new();
    for (key, value) in records {
        digest.update((key.len() as u64).to_le_bytes());
        digest.update(key);
        digest.update((value.len() as u64).to_le_bytes());
        digest.update(value);
    }
    digest.finalize().into()
}

#[cfg(test)]
mod tests {
    use super::{
        open_guarded_source, rebuild_and_verify_staging, rebuild_and_verify_staging_with_observer,
        RebuildPhase, VacuumUnlock,
    };
    use crate::error::TosumuError;
    use crate::page_store::PageStore;
    use crate::wal::wal_path;
    use crate::writer_gate::WriterGuard;

    #[cfg(any(unix, windows))]
    #[test]
    fn guarded_source_keeps_writer_gate_through_rebuild_work() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("guarded-source.tsm");
        let staging_path = directory.path().join("guarded-staging.tsm");
        let mut initial = PageStore::create(&source_path).unwrap();
        initial.put(b"key", b"value").unwrap();
        drop(initial);

        let maintenance_guard = WriterGuard::acquire(&source_path).unwrap();
        let mut source =
            open_guarded_source(&source_path, VacuumUnlock::Sentinel, &maintenance_guard).unwrap();
        drop(maintenance_guard);

        assert!(matches!(
            WriterGuard::acquire(&source_path),
            Err(TosumuError::FileBusy { .. })
        ));
        rebuild_and_verify_staging(&mut source, &staging_path, VacuumUnlock::Sentinel).unwrap();
        assert!(matches!(
            WriterGuard::acquire(&source_path),
            Err(TosumuError::FileBusy { .. })
        ));

        drop(source);
        WriterGuard::acquire(&source_path).unwrap();
    }

    #[test]
    fn rebuild_copies_all_logical_records_and_leaves_an_empty_wal() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source.tsm");
        let staging_path = directory.path().join("staging.tsm");
        let mut source = PageStore::create(&source_path).unwrap();
        for index in 0..80u32 {
            source
                .put(
                    format!("key-{index:04}").as_bytes(),
                    &vec![(index % 251) as u8; 900],
                )
                .unwrap();
        }
        for index in (0..80u32).step_by(2) {
            source.delete(format!("key-{index:04}").as_bytes()).unwrap();
        }
        let expected = source.scan().unwrap();

        let report =
            rebuild_and_verify_staging(&mut source, &staging_path, VacuumUnlock::Sentinel).unwrap();

        assert_eq!(report.logical_records, expected.len() as u64);
        assert_eq!(
            PageStore::open_readonly(&staging_path)
                .unwrap()
                .scan()
                .unwrap(),
            expected
        );
        assert_eq!(std::fs::metadata(wal_path(&staging_path)).unwrap().len(), 0);
        assert!(report.staging_pages <= report.source_pages);
    }

    #[test]
    fn encrypted_rebuild_verifies_with_the_original_passphrase() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source-encrypted.tsm");
        let staging_path = directory.path().join("staging-encrypted.tsm");
        let mut source = PageStore::create_encrypted(&source_path, "vacuum secret").unwrap();
        source.put(b"catalog", b"preserved bytes").unwrap();

        rebuild_and_verify_staging(
            &mut source,
            &staging_path,
            VacuumUnlock::Passphrase("vacuum secret"),
        )
        .unwrap();

        let staged =
            PageStore::open_with_passphrase_readonly(&staging_path, "vacuum secret").unwrap();
        assert_eq!(
            staged.get(b"catalog").unwrap(),
            Some(b"preserved bytes".to_vec())
        );
    }

    #[test]
    fn recovery_key_rebuild_verifies_with_the_original_recovery_key() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source-recovery.tsm");
        let staging_path = directory.path().join("staging-recovery.tsm");
        let mut created = PageStore::create_encrypted(&source_path, "initial secret").unwrap();
        created.put(b"catalog", b"recovery protected").unwrap();
        drop(created);
        let recovery_key =
            PageStore::add_recovery_key_protector(&source_path, "initial secret").unwrap();
        let mut source = PageStore::open_with_recovery_key(&source_path, &recovery_key).unwrap();

        rebuild_and_verify_staging(
            &mut source,
            &staging_path,
            VacuumUnlock::RecoveryKey(&recovery_key),
        )
        .unwrap();

        let staged =
            PageStore::open_with_recovery_key_readonly(&staging_path, &recovery_key).unwrap();
        assert_eq!(
            staged.get(b"catalog").unwrap(),
            Some(b"recovery protected".to_vec())
        );
    }

    #[test]
    fn keyfile_rebuild_verifies_with_the_original_keyfile() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source-keyfile.tsm");
        let staging_path = directory.path().join("staging-keyfile.tsm");
        let keyfile_path = directory.path().join("protector.key");
        std::fs::write(&keyfile_path, [0x5au8; 32]).unwrap();
        let mut created = PageStore::create_encrypted(&source_path, "initial secret").unwrap();
        created.put(b"catalog", b"keyfile protected").unwrap();
        drop(created);
        PageStore::add_keyfile_protector(&source_path, "initial secret", &keyfile_path).unwrap();
        let mut source = PageStore::open_with_keyfile(&source_path, &keyfile_path).unwrap();

        rebuild_and_verify_staging(
            &mut source,
            &staging_path,
            VacuumUnlock::Keyfile(&keyfile_path),
        )
        .unwrap();

        let staged = PageStore::open_with_keyfile_readonly(&staging_path, &keyfile_path).unwrap();
        assert_eq!(
            staged.get(b"catalog").unwrap(),
            Some(b"keyfile protected".to_vec())
        );
    }

    #[test]
    fn existing_staging_artifact_is_never_overwritten() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source.tsm");
        let staging_path = directory.path().join("staging.tsm");
        let mut source = PageStore::create(&source_path).unwrap();
        source.put(b"key", b"value").unwrap();
        std::fs::write(&staging_path, b"sentinel").unwrap();

        assert!(
            rebuild_and_verify_staging(&mut source, &staging_path, VacuumUnlock::Sentinel).is_err()
        );
        assert_eq!(std::fs::read(staging_path).unwrap(), b"sentinel");
    }

    #[test]
    fn owned_staging_artifacts_are_cleaned_after_verification_failure() {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source-encrypted.tsm");
        let staging_path = directory.path().join("staging-encrypted.tsm");
        let mut source = PageStore::create_encrypted(&source_path, "correct secret").unwrap();
        source.put(b"key", b"value").unwrap();

        let error = rebuild_and_verify_staging(
            &mut source,
            &staging_path,
            VacuumUnlock::Passphrase("wrong secret"),
        )
        .unwrap_err();

        assert!(matches!(error, crate::error::TosumuError::WrongKey));
        assert!(!staging_path.exists());
        assert!(!wal_path(&staging_path).exists());
        assert!(!crate::writer_gate::writer_lock_path(&staging_path).exists());
    }

    #[test]
    fn injected_copy_failure_preserves_source_and_cleans_owned_staging() {
        assert_injected_rebuild_failure_cleans(RebuildPhase::RecordCopy(2));
    }

    #[test]
    fn injected_verification_failure_preserves_source_and_cleans_owned_staging() {
        assert_injected_rebuild_failure_cleans(RebuildPhase::Verification);
    }

    fn assert_injected_rebuild_failure_cleans(failing_phase: RebuildPhase) {
        let directory = tempfile::tempdir().unwrap();
        let source_path = directory.path().join("source.tsm");
        let staging_path = directory.path().join("staging.tsm");
        let mut source = PageStore::create(&source_path).unwrap();
        for index in 0..5u32 {
            source
                .put(
                    format!("key-{index}").as_bytes(),
                    format!("value-{index}").as_bytes(),
                )
                .unwrap();
        }
        let expected = source.scan().unwrap();

        let error = rebuild_and_verify_staging_with_observer(
            &mut source,
            &staging_path,
            VacuumUnlock::Sentinel,
            |phase| {
                if phase == failing_phase {
                    Err(TosumuError::Io(std::io::Error::other(
                        "injected rebuild failure",
                    )))
                } else {
                    Ok(())
                }
            },
        )
        .unwrap_err();

        assert!(matches!(error, TosumuError::Io(_)));
        assert_eq!(source.scan().unwrap(), expected);
        assert!(!staging_path.exists());
        assert!(!wal_path(&staging_path).exists());
        assert!(!crate::writer_gate::writer_lock_path(&staging_path).exists());
    }
}
