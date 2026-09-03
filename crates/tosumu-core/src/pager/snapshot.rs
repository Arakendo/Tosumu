use crate::crypto::{decrypt_page, verify_header_mac};
use crate::error::{Result, TosumuError};
use crate::format::{read_u64, OFF_PAGE_COUNT, OFF_ROOT_PAGE, PAGE_PLAINTEXT_SIZE, PAGE_SIZE};
use crate::snapshot_registry::SnapshotPin;

use super::page0::{keyslot_count, read_header_mac_field, read_page0, validate_header};
use super::{validate_plaintext_header, Pager};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SnapshotDiagnostics {
    pub(crate) active: u64,
    pub(crate) maximum: u64,
    pub(crate) oldest_generation: Option<u64>,
    pub(crate) checkpoint_generation: u64,
    pub(crate) latest_generation: u64,
    pub(crate) retained_wal_bytes: u64,
    pub(crate) retained_frame_versions: u64,
    pub(crate) checkpoint_blocked: bool,
}

impl Pager {
    pub(crate) fn current_generation(&self) -> Result<u64> {
        self.ensure_healthy()?;
        Ok(self.committed_index.latest_commit_lsn())
    }

    pub(crate) fn pin_latest_snapshot(&self) -> Result<SnapshotPin> {
        self.snapshot_registry
            .register(self.committed_index.latest_commit_lsn())
    }

    pub(crate) fn with_snapshot_page<F, T>(&self, pin: &SnapshotPin, pgno: u64, f: F) -> Result<T>
    where
        F: FnOnce(&[u8; PAGE_PLAINTEXT_SIZE]) -> Result<T>,
    {
        self.ensure_healthy()?;
        self.validate_snapshot_owner(pin)?;
        let page0 = self.page0_at_generation(pin.generation())?;
        let snapshot_page_count = read_u64(&page0, OFF_PAGE_COUNT);
        if pgno == 0 || pgno >= snapshot_page_count {
            return Err(TosumuError::InvalidArgument(
                "page number is outside the snapshot",
            ));
        }
        let frame = self.frame_at_generation(pgno, pin.generation())?;
        let (plaintext, _) = decrypt_page(&self.page_key, pgno, &frame)?;
        validate_plaintext_header(&plaintext, pgno)?;
        f(&plaintext)
    }

    pub(crate) fn snapshot_metadata(&self, pin: &SnapshotPin) -> Result<(u64, u64)> {
        self.ensure_healthy()?;
        self.validate_snapshot_owner(pin)?;
        let page0 = self.page0_at_generation(pin.generation())?;
        Ok((
            read_u64(&page0, OFF_ROOT_PAGE),
            read_u64(&page0, OFF_PAGE_COUNT),
        ))
    }

    pub(crate) fn snapshot_diagnostics(&self) -> Result<SnapshotDiagnostics> {
        self.ensure_healthy()?;
        let registry = self.snapshot_registry.info()?;
        let retained_wal_bytes = self.wal.as_ref().map_or(Ok(0), |wal| wal.encoded_len())?;
        Ok(SnapshotDiagnostics {
            active: registry.active,
            maximum: registry.maximum,
            oldest_generation: registry.oldest_generation,
            checkpoint_generation: self.committed_index.checkpoint_lsn(),
            latest_generation: self.committed_index.latest_commit_lsn(),
            retained_wal_bytes,
            retained_frame_versions: self.committed_index.retained_version_count(),
            checkpoint_blocked: registry.active != 0,
        })
    }

    pub(super) fn frame_at_generation(
        &self,
        pgno: u64,
        generation: u64,
    ) -> Result<[u8; PAGE_SIZE]> {
        self.validate_generation(generation)?;
        if let Some(version) = self.committed_index.page_at(pgno, generation) {
            return Ok(*version.frame.as_ref());
        }
        self.read_frame(pgno)
    }

    fn page0_at_generation(&self, generation: u64) -> Result<[u8; PAGE_SIZE]> {
        self.validate_generation(generation)?;
        let page0 = if let Some(version) = self.committed_index.page_at(0, generation) {
            *version.frame.as_ref()
        } else {
            let mut file = self.file.try_clone()?;
            read_page0(&mut file)?
        };
        validate_header(&page0)?;
        if let Some(ref hmk) = self.header_mac_key {
            let count = keyslot_count(&page0);
            let stored_mac = read_header_mac_field(&page0)?;
            verify_header_mac(hmk, &page0, count, &stored_mac)?;
        }
        Ok(page0)
    }

    fn validate_generation(&self, generation: u64) -> Result<()> {
        if generation < self.committed_index.checkpoint_lsn()
            || generation > self.committed_index.latest_commit_lsn()
        {
            return Err(TosumuError::InvalidArgument(
                "snapshot generation is outside the retained interval",
            ));
        }
        Ok(())
    }

    fn validate_snapshot_owner(&self, pin: &SnapshotPin) -> Result<()> {
        if !pin.belongs_to(&self.snapshot_registry) {
            return Err(TosumuError::InvalidArgument(
                "snapshot belongs to a different database owner",
            ));
        }
        Ok(())
    }
}
