use std::fs::{File, OpenOptions};
use std::path::Path;

use crate::crypto::{
    compute_header_mac, derive_passphrase_kek, derive_recovery_kek, derive_subkeys, unwrap_dek,
    verify_kcv,
};
use crate::error::{Result, TosumuError};
use crate::format::*;
use crate::writer_gate::WriterGuard;

use super::page0::{keyslot_count, read_keyslot_field, read_page0, validate_header, write_page0};

pub(super) enum ProtectorUnlock<'a> {
    Passphrase(&'a str),
    RecoveryKey(&'a str),
    Keyfile(&'a Path),
}

pub(super) struct Page0EditSession {
    file: File,
    _writer_guard: WriterGuard,
    pub(super) page0: [u8; PAGE_SIZE],
    pub(super) dek_id: u64,
    pub(super) keyslot_count: usize,
}

impl Page0EditSession {
    pub(super) fn open(path: &Path) -> Result<Self> {
        let writer_guard = WriterGuard::acquire(path)?;
        let mut file = OpenOptions::new().read(true).write(true).open(path)?;
        let page0 = read_page0(&mut file)?;
        validate_header(&page0)?;
        Ok(Self {
            dek_id: read_u64(&page0, OFF_DEK_ID),
            keyslot_count: keyslot_count(&page0),
            file,
            _writer_guard: writer_guard,
            page0,
        })
    }

    pub(super) fn open_unlocked(
        path: &Path,
        unlock: ProtectorUnlock<'_>,
    ) -> Result<(Self, [u8; 32], [u8; 32])> {
        let session = Self::open(path)?;
        let dek = unlock_key_management_dek(
            &session.page0,
            unlock,
            session.dek_id,
            session.keyslot_count,
        )?;
        let (_, hmk, _) = derive_subkeys(&dek);
        Ok((session, dek, hmk))
    }

    pub(super) fn commit(mut self, hmk: &[u8; 32]) -> Result<()> {
        let mac = compute_header_mac(hmk, &self.page0, self.keyslot_count);
        self.page0[OFF_HEADER_MAC..OFF_HEADER_MAC + 32].copy_from_slice(&mac);
        write_page0(&mut self.file, &self.page0)
    }
}

pub(super) fn read_keyfile_kek(path: &Path) -> Result<[u8; 32]> {
    let bytes = std::fs::read(path)?;
    if bytes.len() != 32 {
        return Err(TosumuError::InvalidArgument(
            "keyfile must contain exactly 32 raw bytes",
        ));
    }
    let mut kek = [0u8; 32];
    kek.copy_from_slice(&bytes);
    Ok(kek)
}

/// Try to unlock the database using a passphrase, scanning all keyslots.
///
/// Returns `(dek, is_encrypted)`. For Sentinel DBs, `is_encrypted` is false.
pub(super) fn try_unlock_passphrase(
    page0: &[u8; PAGE_SIZE],
    passphrase: &str,
    dek_id: u64,
    keyslot_count: usize,
) -> Result<([u8; 32], bool)> {
    // First check slot 0 for Sentinel (unencrypted DB).
    let ks0 = KEYSLOT_REGION_OFFSET;
    if page0[ks0 + KS_OFF_KIND] == KEYSLOT_KIND_SENTINEL {
        let mut dek = [0u8; 32];
        dek.copy_from_slice(&page0[ks0 + KS_OFF_WRAPPED_DEK..ks0 + KS_OFF_WRAPPED_DEK + 32]);
        return Ok((dek, false));
    }

    // Scan all Passphrase slots.
    for i in 0..keyslot_count {
        let ks = KEYSLOT_REGION_OFFSET + i * KEYSLOT_SIZE;
        if page0[ks + KS_OFF_KIND] != KEYSLOT_KIND_PASSPHRASE {
            continue;
        }
        let salt = read_keyslot_field::<16>(page0, i, KS_OFF_SALT, "bad keyslot salt length")?;
        let kdf_params =
            read_keyslot_field::<32>(page0, i, KS_OFF_KDF_PARAMS, "bad keyslot kdf_params length")?;
        let kcv = read_keyslot_field::<32>(page0, i, KS_OFF_KCV, "bad keyslot KCV length")?;
        let wrap_nonce =
            read_keyslot_field::<12>(page0, i, KS_OFF_WRAP_NONCE, "bad keyslot wrap nonce length")?;
        let wrapped_dek = read_keyslot_field::<48>(
            page0,
            i,
            KS_OFF_WRAPPED_DEK,
            "bad keyslot wrapped DEK length",
        )?;

        let kek = match derive_passphrase_kek(passphrase, &salt, &kdf_params) {
            Ok(k) => k,
            Err(_) => continue,
        };
        if verify_kcv(&kek, &kcv).is_err() {
            continue;
        }
        if let Ok(dek) = unwrap_dek(
            &kek,
            &wrap_nonce,
            &wrapped_dek,
            i as u16,
            dek_id,
            KEYSLOT_KIND_PASSPHRASE,
        ) {
            return Ok((dek, true));
        }
    }
    Err(TosumuError::WrongKey)
}

/// Try to unlock the database with a pre-derived KEK, scanning for a specific kind.
pub(super) fn try_unlock_with_kek(
    page0: &[u8; PAGE_SIZE],
    kek: &[u8; 32],
    dek_id: u64,
    keyslot_count: usize,
    kind: u8,
) -> Result<[u8; 32]> {
    for i in 0..keyslot_count {
        let ks = KEYSLOT_REGION_OFFSET + i * KEYSLOT_SIZE;
        if page0[ks + KS_OFF_KIND] != kind {
            continue;
        }
        let kcv = read_keyslot_field::<32>(page0, i, KS_OFF_KCV, "bad keyslot KCV length")?;
        if verify_kcv(kek, &kcv).is_err() {
            continue;
        }
        let wrap_nonce =
            read_keyslot_field::<12>(page0, i, KS_OFF_WRAP_NONCE, "bad keyslot wrap nonce length")?;
        let wrapped_dek = read_keyslot_field::<48>(
            page0,
            i,
            KS_OFF_WRAPPED_DEK,
            "bad keyslot wrapped DEK length",
        )?;
        if let Ok(dek) = unwrap_dek(kek, &wrap_nonce, &wrapped_dek, i as u16, dek_id, kind) {
            return Ok(dek);
        }
    }
    Err(TosumuError::WrongKey)
}

fn unlock_key_management_dek(
    page0: &[u8; PAGE_SIZE],
    unlock: ProtectorUnlock<'_>,
    dek_id: u64,
    keyslot_count: usize,
) -> Result<[u8; 32]> {
    match unlock {
        ProtectorUnlock::Passphrase(passphrase) => {
            let (dek, _) = try_unlock_passphrase(page0, passphrase, dek_id, keyslot_count)?;
            Ok(dek)
        }
        ProtectorUnlock::RecoveryKey(recovery_str) => {
            let kek = derive_recovery_kek(recovery_str)?;
            try_unlock_with_kek(
                page0,
                &kek,
                dek_id,
                keyslot_count,
                KEYSLOT_KIND_RECOVERY_KEY,
            )
        }
        ProtectorUnlock::Keyfile(keyfile_path) => {
            let kek = read_keyfile_kek(keyfile_path)?;
            try_unlock_with_kek(page0, &kek, dek_id, keyslot_count, KEYSLOT_KIND_KEYFILE)
        }
    }
}
