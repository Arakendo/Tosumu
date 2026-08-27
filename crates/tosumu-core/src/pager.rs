// Pager — page-level I/O with AEAD encryption/decryption.
//
// Source of truth: docs/Specifications/Tosumu Software Design Document.md §6.
//
// The pager owns the file handle and the page_key. It exposes a
// closure-based API (§28.9): the caller never holds a reference to
// page bytes beyond the closure call.
//
// For MVP+1 there is no in-memory cache (every read hits the file).
// Cache is a Stage 2 concern.
//
// ── Validation layering ──────────────────────────────────────────────────────
//
// Pager guarantees (before handing bytes to callers):
//   1. Bytes are authentically decrypted (AEAD tag verified).
//   2. The page_type field is a known value (LEAF, INTERNAL, OVERFLOW, FREE).
//   3. For LEAF/INTERNAL: free_start ≤ free_end ≤ PAGE_PLAINTEXT_SIZE.
//   4. The pgno is within the allocated range (≥ 1, < page_count).
//
// What pager does NOT guarantee (higher-layer responsibility):
//   - Slot array integrity (no overlapping regions, no out-of-bounds offsets).
//   - Record encoding correctness (key/value lengths, tombstone semantics).
//   - B-tree structural invariants (sorted keys, chain pointers, height).
//
// In practice: btree.rs / PageStore handle semantic correctness; pager
// handles physical correctness. Debugging hint: if data looks decrypted but
// logically wrong, the bug is above the pager layer.

use std::collections::HashMap;
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::Path;

use crate::crypto::{
    compute_header_mac, compute_kcv, decrypt_page, derive_passphrase_kek, derive_recovery_kek,
    derive_subkeys, encrypt_page, generate_dek, generate_recovery_secret, pack_kdf_params,
    unwrap_dek, verify_header_mac, verify_kcv, wrap_dek, ARGON2_M_COST, ARGON2_P_COST,
    ARGON2_T_COST,
};
use crate::error::{Result, TosumuError};
use crate::format::*;
use crate::wal::{wal_path, CommittedWalIndex, WalReader, WalRecord, WalWriter};
use crate::writer_gate::WriterGuard;

#[path = "pager/page0.rs"]
mod page0;
#[path = "pager/unlock.rs"]
mod unlock;

use page0::*;
use unlock::*;

#[derive(Clone, Copy, Eq, PartialEq)]
enum PagerHealth {
    Healthy,
    Poisoned,
}

/// The pager. Holds an open file and the derived page key.
pub struct Pager {
    file: File,
    /// Retains cooperative single-writer admission for writable handles.
    _writer_guard: Option<WriterGuard>,
    page_key: [u8; 32],
    /// For passphrase-protected databases: the HMAC key used to MAC page0.
    /// None for Sentinel databases (no header MAC).
    header_mac_key: Option<[u8; 32]>,
    // Cached from the file header. Written back on allocate / flush_header.
    page_count: u64,
    freelist_head: u64,
    /// B+ tree root page number (0 = not yet set). Persisted at OFF_ROOT_PAGE.
    root_page: u64,
    // ── WAL / transaction state ───────────────────────────────────────────
    /// WAL writer, open for the lifetime of writable Pagers.
    wal: Option<WalWriter>,
    /// Read-only handles never open the WAL for appending and reject mutation APIs.
    read_only: bool,
    /// Set after a commit-path I/O failure leaves durability or cache state ambiguous.
    health: PagerHealth,
    /// Whether a transaction is currently active.
    txn_active: bool,
    /// txn_id of the current open transaction.
    txn_id: u64,
    /// Counter for generating unique txn_ids.
    next_txn_id: u64,
    /// Dirty page frames buffered during the current transaction.
    /// Keyed by pgno so lookups are O(1); latest write wins.
    dirty_pages: HashMap<u64, Box<[u8; PAGE_SIZE]>>,
    /// Set when `flush_header()` is deferred during a transaction.
    /// Cleared (and the real write performed) at commit or rollback.
    pending_header_flush: bool,
    /// Snapshot of page_count / root_page / freelist_head taken at begin_txn.
    /// Used to restore in-memory state on rollback.
    txn_saved_page_count: u64,
    txn_saved_root_page: u64,
    txn_saved_freelist_head: u64,
}

trait PagerPhaseTwoFile: Seek + Write {
    fn sync_data(&mut self) -> std::io::Result<()>;
}

impl PagerPhaseTwoFile for File {
    fn sync_data(&mut self) -> std::io::Result<()> {
        File::sync_data(self)
    }
}

#[cfg(test)]
impl PagerPhaseTwoFile for crate::test_helpers::CrashFile {
    fn sync_data(&mut self) -> std::io::Result<()> {
        crate::test_helpers::CrashFile::sync_data(self)
    }
}

enum OpenUnlock<'a> {
    Sentinel,
    Passphrase(&'a str),
    RecoveryKey(&'a str),
    Keyfile(&'a Path),
}

impl Pager {
    // ── Construction ─────────────────────────────────────────────────────────

    /// Create a new database file at `path`.
    ///
    /// Generates a DEK, writes the file header (page 0) with a Sentinel
    /// keyslot, and returns a ready-to-use Pager.
    ///
    /// # Security: Sentinel provides NO confidentiality
    ///
    /// The DEK is stored in plaintext in the Sentinel keyslot. Anyone with
    /// read access to the file can decrypt all pages. The AEAD layer provides
    /// *integrity only* until a passphrase or recovery-key protector is added
    /// via `create_encrypted` / `add_passphrase_protector`.
    ///
    /// Do NOT use `create()` for user-facing databases that require secrecy.
    pub fn create(path: &Path) -> Result<Self> {
        let writer_guard = WriterGuard::acquire(path)?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)?;

        let dek = generate_dek()?;
        let (page_key, _header_mac_key, _audit_key) = derive_subkeys(&dek);

        // Build page 0 (plaintext file header + Sentinel keyslot).
        let mut page0 = [0u8; PAGE_SIZE];
        write_file_header(&mut page0, &dek);

        file.write_all(&page0)?;
        file.sync_data()?;

        // Open/create WAL sidecar.  Failure is fatal: we must not advertise
        // transaction semantics while WAL is unavailable.
        let wal = WalWriter::open_or_create_seeded(&wal_path(path), wal_seed_from_page0(&page0)?)?;

        Ok(Pager {
            file,
            _writer_guard: Some(writer_guard),
            page_key,
            header_mac_key: None,
            page_count: 1,
            freelist_head: 0,
            root_page: 0,
            wal: Some(wal),
            read_only: false,
            health: PagerHealth::Healthy,
            txn_active: false,
            txn_id: 0,
            next_txn_id: 1,
            dirty_pages: HashMap::new(),
            pending_header_flush: false,
            txn_saved_page_count: 0,
            txn_saved_root_page: 0,
            txn_saved_freelist_head: 0,
        })
    }

    /// Open an existing database file at `path`.
    ///
    /// Reads the Sentinel keyslot to recover the DEK, verifies the header.
    ///
    /// # Security: Sentinel provides NO confidentiality
    ///
    /// If the file was created with `create()` (Sentinel keyslot), the DEK is
    /// read from plaintext — this path offers integrity checks but **zero
    /// confidentiality**. Use `open_with_passphrase()` or
    /// `open_with_recovery_key()` for databases that require secrecy.
    pub fn open(path: &Path) -> Result<Self> {
        Self::open_inner(path, false, OpenUnlock::Sentinel)
    }

    /// Open an existing database file in read-only mode.
    pub fn open_readonly(path: &Path) -> Result<Self> {
        Self::open_inner(path, true, OpenUnlock::Sentinel)
    }

    /// Create a new passphrase-protected database file at `path`.
    ///
    /// Generates a DEK, wraps it with Argon2id-derived KEK, stores the wrapped DEK
    /// in keyslot 0 (Passphrase kind), and writes a header MAC over the full keyslot region.
    /// The keyslot region is pre-allocated to MAX_KEYSLOTS so future protectors can be added
    /// without a page rewrite.
    pub fn create_encrypted(path: &Path, passphrase: &str) -> Result<Self> {
        let writer_guard = WriterGuard::acquire(path)?;
        let mut file = OpenOptions::new()
            .read(true)
            .write(true)
            .create_new(true)
            .open(path)?;

        let dek = generate_dek()?;
        let (page_key, header_mac_key, _audit_key) = derive_subkeys(&dek);

        // Random 16-byte salt for this slot.
        let mut salt = [0u8; 16];
        getrandom::getrandom(&mut salt).map_err(|_| TosumuError::RngFailed)?;

        // Derive KEK from passphrase.
        let kdf_params = pack_kdf_params(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST);
        let kek = derive_passphrase_kek(passphrase, &salt, &kdf_params)?;

        // Generate a unique per-database DEK_ID so that a full-slot splice from a
        // different database fails AEAD unwrap even when the same passphrase is used.
        let mut dek_id_buf = [0u8; 8];
        getrandom::getrandom(&mut dek_id_buf).map_err(|_| TosumuError::RngFailed)?;
        let dek_id = u64::from_le_bytes(dek_id_buf) | 1; // ensure non-zero

        // Wrap the DEK.
        let (wrap_nonce, wrapped_dek) = wrap_dek(&kek, &dek, 0, dek_id, KEYSLOT_KIND_PASSPHRASE)?;

        // Compute KCV.
        let kcv = compute_kcv(&kek);

        // Build page 0.
        let mut page0 = [0u8; PAGE_SIZE];
        write_file_header(&mut page0, &dek); // writes sentinel slot 0; we overwrite below
        write_u64(&mut page0, OFF_DEK_ID, dek_id); // overwrite hardcoded 1 with per-DB random id
                                                   // Set keyslot count to MAX_KEYSLOTS so the full region is MAC'd.
        write_u16(&mut page0, OFF_KEYSLOT_COUNT, MAX_KEYSLOTS as u16);

        // Overwrite slot 0 with passphrase data.
        write_keyslot(
            &mut page0,
            0,
            KEYSLOT_KIND_PASSPHRASE,
            dek_id,
            &salt,
            &kdf_params,
            &wrap_nonce,
            &wrapped_dek,
            &kcv,
        );

        // Zero slots 1..MAX_KEYSLOTS (they're already zero from the array init, just ensure).
        for i in 1..MAX_KEYSLOTS {
            let ks = KEYSLOT_REGION_OFFSET + i * KEYSLOT_SIZE;
            page0[ks..ks + KEYSLOT_SIZE].fill(0);
        }

        // Compute and store the header MAC (covers header plain region + full keyslot region).
        let mac = compute_header_mac(&header_mac_key, &page0, MAX_KEYSLOTS);
        page0[OFF_HEADER_MAC..OFF_HEADER_MAC + 32].copy_from_slice(&mac);

        file.write_all(&page0)?;
        file.sync_data()?;

        // Open/create WAL sidecar.  Failure is fatal.
        let wal = WalWriter::open_or_create_seeded(&wal_path(path), wal_seed_from_page0(&page0)?)?;

        Ok(Pager {
            file,
            _writer_guard: Some(writer_guard),
            page_key,
            header_mac_key: Some(header_mac_key),
            page_count: 1,
            freelist_head: 0,
            root_page: 0,
            wal: Some(wal),
            read_only: false,
            health: PagerHealth::Healthy,
            txn_active: false,
            txn_id: 0,
            next_txn_id: 1,
            dirty_pages: HashMap::new(),
            pending_header_flush: false,
            txn_saved_page_count: 0,
            txn_saved_root_page: 0,
            txn_saved_freelist_head: 0,
        })
    }

    /// Open a passphrase-protected database file.
    ///
    /// Scans all keyslots, trying the passphrase against every Passphrase slot.
    /// If any slot accepts, the DEK is unwrapped and the header MAC is verified.
    /// Also accepts Sentinel databases (passphrase is ignored).
    pub fn open_with_passphrase(path: &Path, passphrase: &str) -> Result<Self> {
        Self::open_inner(path, false, OpenUnlock::Passphrase(passphrase))
    }

    /// Open a passphrase-protected database file in read-only mode.
    pub fn open_with_passphrase_readonly(path: &Path, passphrase: &str) -> Result<Self> {
        Self::open_inner(path, true, OpenUnlock::Passphrase(passphrase))
    }

    /// Open a database using a recovery key string.
    ///
    /// Scans all keyslots, trying the recovery key against every RecoveryKey slot.
    pub fn open_with_recovery_key(path: &Path, recovery_str: &str) -> Result<Self> {
        Self::open_inner(path, false, OpenUnlock::RecoveryKey(recovery_str))
    }

    /// Open a recovery-key-protected database file in read-only mode.
    pub fn open_with_recovery_key_readonly(path: &Path, recovery_str: &str) -> Result<Self> {
        Self::open_inner(path, true, OpenUnlock::RecoveryKey(recovery_str))
    }

    /// Open a database using a raw 32-byte KEK loaded from `keyfile_path`.
    pub fn open_with_keyfile(path: &Path, keyfile_path: &Path) -> Result<Self> {
        Self::open_inner(path, false, OpenUnlock::Keyfile(keyfile_path))
    }

    /// Open a keyfile-protected database file in read-only mode.
    pub fn open_with_keyfile_readonly(path: &Path, keyfile_path: &Path) -> Result<Self> {
        Self::open_inner(path, true, OpenUnlock::Keyfile(keyfile_path))
    }

    // ── Key management ───────────────────────────────────────────────────────

    /// Add a passphrase protector to an existing database.
    ///
    /// `unlock` is called first to open the database (it must already be unlockable).
    /// Then a new Passphrase keyslot is written in the first empty slot.
    /// Returns the slot index used.
    pub fn add_passphrase_protector(
        path: &Path,
        unlock_passphrase: &str,
        new_passphrase: &str,
    ) -> Result<u16> {
        Self::add_passphrase_protector_inner(
            path,
            ProtectorUnlock::Passphrase(unlock_passphrase),
            new_passphrase,
        )
    }

    /// Add a passphrase protector, unlocking the DEK with a recovery key.
    pub fn add_passphrase_protector_with_recovery_key(
        path: &Path,
        recovery_str: &str,
        new_passphrase: &str,
    ) -> Result<u16> {
        Self::add_passphrase_protector_inner(
            path,
            ProtectorUnlock::RecoveryKey(recovery_str),
            new_passphrase,
        )
    }

    /// Add a passphrase protector, unlocking the DEK with a keyfile protector.
    pub fn add_passphrase_protector_with_keyfile(
        path: &Path,
        keyfile_path: &Path,
        new_passphrase: &str,
    ) -> Result<u16> {
        Self::add_passphrase_protector_inner(
            path,
            ProtectorUnlock::Keyfile(keyfile_path),
            new_passphrase,
        )
    }

    fn add_passphrase_protector_inner(
        path: &Path,
        unlock: ProtectorUnlock<'_>,
        new_passphrase: &str,
    ) -> Result<u16> {
        let (mut session, dek, hmk) = Page0EditSession::open_unlocked(path, unlock)?;

        let slot_idx = find_empty_slot(&session.page0, session.keyslot_count)?;

        let mut salt = [0u8; 16];
        getrandom::getrandom(&mut salt).map_err(|_| TosumuError::RngFailed)?;
        let kdf_params = pack_kdf_params(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST);
        let kek = derive_passphrase_kek(new_passphrase, &salt, &kdf_params)?;
        let (wrap_nonce, wrapped_dek) = wrap_dek(
            &kek,
            &dek,
            slot_idx,
            session.dek_id,
            KEYSLOT_KIND_PASSPHRASE,
        )?;
        let kcv = compute_kcv(&kek);

        write_keyslot(
            &mut session.page0,
            slot_idx as usize,
            KEYSLOT_KIND_PASSPHRASE,
            session.dek_id,
            &salt,
            &kdf_params,
            &wrap_nonce,
            &wrapped_dek,
            &kcv,
        );

        session.commit(&hmk)?;
        Ok(slot_idx)
    }

    /// Add a recovery-key protector to an existing database.
    ///
    /// Returns the one-time recovery string that must be shown to the user.
    pub fn add_recovery_key_protector(path: &Path, unlock_passphrase: &str) -> Result<String> {
        let recovery_str = generate_recovery_secret();
        Self::add_recovery_key_protector_with_secret_inner(
            path,
            ProtectorUnlock::Passphrase(unlock_passphrase),
            &recovery_str,
        )?;
        Ok(recovery_str)
    }

    /// Add a recovery-key protector, unlocking the DEK with an existing recovery key.
    pub fn add_recovery_key_protector_with_recovery_key(
        path: &Path,
        recovery_str: &str,
    ) -> Result<String> {
        let new_recovery = generate_recovery_secret();
        Self::add_recovery_key_protector_with_secret_inner(
            path,
            ProtectorUnlock::RecoveryKey(recovery_str),
            &new_recovery,
        )?;
        Ok(new_recovery)
    }

    /// Add a recovery-key protector, unlocking the DEK with a keyfile protector.
    pub fn add_recovery_key_protector_with_keyfile(
        path: &Path,
        keyfile_path: &Path,
    ) -> Result<String> {
        let recovery_str = generate_recovery_secret();
        Self::add_recovery_key_protector_with_secret_inner(
            path,
            ProtectorUnlock::Keyfile(keyfile_path),
            &recovery_str,
        )?;
        Ok(recovery_str)
    }

    /// Add a recovery-key protector using a caller-supplied recovery string.
    pub fn add_recovery_key_protector_with_secret(
        path: &Path,
        unlock_passphrase: &str,
        recovery_str: &str,
    ) -> Result<()> {
        Self::add_recovery_key_protector_with_secret_inner(
            path,
            ProtectorUnlock::Passphrase(unlock_passphrase),
            recovery_str,
        )
    }

    /// Add a recovery-key protector with an existing recovery key and caller-supplied secret.
    pub fn add_recovery_key_protector_with_recovery_key_and_secret(
        path: &Path,
        recovery_str: &str,
        new_recovery_str: &str,
    ) -> Result<()> {
        Self::add_recovery_key_protector_with_secret_inner(
            path,
            ProtectorUnlock::RecoveryKey(recovery_str),
            new_recovery_str,
        )
    }

    /// Add a recovery-key protector with a keyfile unlock and caller-supplied secret.
    pub fn add_recovery_key_protector_with_keyfile_and_secret(
        path: &Path,
        keyfile_path: &Path,
        recovery_str: &str,
    ) -> Result<()> {
        Self::add_recovery_key_protector_with_secret_inner(
            path,
            ProtectorUnlock::Keyfile(keyfile_path),
            recovery_str,
        )
    }

    fn add_recovery_key_protector_with_secret_inner(
        path: &Path,
        unlock: ProtectorUnlock<'_>,
        recovery_str: &str,
    ) -> Result<()> {
        let (mut session, dek, hmk) = Page0EditSession::open_unlocked(path, unlock)?;

        let slot_idx = find_empty_slot(&session.page0, session.keyslot_count)?;

        let kek = derive_recovery_kek(&recovery_str)?;
        let (wrap_nonce, wrapped_dek) = wrap_dek(
            &kek,
            &dek,
            slot_idx,
            session.dek_id,
            KEYSLOT_KIND_RECOVERY_KEY,
        )?;
        let kcv = compute_kcv(&kek);

        // Recovery key has no KDF params (HKDF, not Argon2id); salt field is zeroed.
        let zero_salt = [0u8; 16];
        let zero_kdf_params = [0u8; 32];
        write_keyslot(
            &mut session.page0,
            slot_idx as usize,
            KEYSLOT_KIND_RECOVERY_KEY,
            session.dek_id,
            &zero_salt,
            &zero_kdf_params,
            &wrap_nonce,
            &wrapped_dek,
            &kcv,
        );

        session.commit(&hmk)
    }

    /// Add a keyfile protector to an existing database.
    pub fn add_keyfile_protector(
        path: &Path,
        unlock_passphrase: &str,
        keyfile_path: &Path,
    ) -> Result<u16> {
        Self::add_keyfile_protector_inner(
            path,
            ProtectorUnlock::Passphrase(unlock_passphrase),
            keyfile_path,
        )
    }

    /// Add a keyfile protector, unlocking the DEK with a recovery key.
    pub fn add_keyfile_protector_with_recovery_key(
        path: &Path,
        recovery_str: &str,
        keyfile_path: &Path,
    ) -> Result<u16> {
        Self::add_keyfile_protector_inner(
            path,
            ProtectorUnlock::RecoveryKey(recovery_str),
            keyfile_path,
        )
    }

    /// Add a keyfile protector, unlocking the DEK with another keyfile protector.
    pub fn add_keyfile_protector_with_keyfile(
        path: &Path,
        unlock_keyfile_path: &Path,
        keyfile_path: &Path,
    ) -> Result<u16> {
        Self::add_keyfile_protector_inner(
            path,
            ProtectorUnlock::Keyfile(unlock_keyfile_path),
            keyfile_path,
        )
    }

    fn add_keyfile_protector_inner(
        path: &Path,
        unlock: ProtectorUnlock<'_>,
        keyfile_path: &Path,
    ) -> Result<u16> {
        let (mut session, dek, hmk) = Page0EditSession::open_unlocked(path, unlock)?;

        let slot_idx = find_empty_slot(&session.page0, session.keyslot_count)?;
        let kek = read_keyfile_kek(keyfile_path)?;
        let (wrap_nonce, wrapped_dek) =
            wrap_dek(&kek, &dek, slot_idx, session.dek_id, KEYSLOT_KIND_KEYFILE)?;
        let kcv = compute_kcv(&kek);
        let zero_salt = [0u8; 16];
        let zero_kdf_params = [0u8; 32];

        write_keyslot(
            &mut session.page0,
            slot_idx as usize,
            KEYSLOT_KIND_KEYFILE,
            session.dek_id,
            &zero_salt,
            &zero_kdf_params,
            &wrap_nonce,
            &wrapped_dek,
            &kcv,
        );

        session.commit(&hmk)?;
        Ok(slot_idx)
    }

    /// Remove the keyslot at `slot_idx`, zeroing it.
    ///
    /// Refuses to remove the last active slot (that would brick the database).
    pub fn remove_keyslot(path: &Path, unlock_passphrase: &str, slot_idx: u16) -> Result<()> {
        Self::remove_keyslot_inner(
            path,
            ProtectorUnlock::Passphrase(unlock_passphrase),
            slot_idx,
        )
    }

    /// Remove a keyslot, unlocking the DEK with a recovery key.
    pub fn remove_keyslot_with_recovery_key(
        path: &Path,
        recovery_str: &str,
        slot_idx: u16,
    ) -> Result<()> {
        Self::remove_keyslot_inner(path, ProtectorUnlock::RecoveryKey(recovery_str), slot_idx)
    }

    /// Remove a keyslot, unlocking the DEK with a keyfile protector.
    pub fn remove_keyslot_with_keyfile(
        path: &Path,
        keyfile_path: &Path,
        slot_idx: u16,
    ) -> Result<()> {
        Self::remove_keyslot_inner(path, ProtectorUnlock::Keyfile(keyfile_path), slot_idx)
    }

    fn remove_keyslot_inner(path: &Path, unlock: ProtectorUnlock<'_>, slot_idx: u16) -> Result<()> {
        let (mut session, _, hmk) = Page0EditSession::open_unlocked(path, unlock)?;

        if slot_idx as usize >= session.keyslot_count {
            return Err(TosumuError::InvalidArgument("slot index out of range"));
        }

        // Count active slots before removal.
        let active: usize = (0..session.keyslot_count)
            .filter(|&i| {
                let ks = KEYSLOT_REGION_OFFSET + i * KEYSLOT_SIZE;
                session.page0[ks + KS_OFF_KIND] != KEYSLOT_KIND_EMPTY
                    && session.page0[ks + KS_OFF_KIND] != KEYSLOT_KIND_SENTINEL
            })
            .count();

        if active <= 1 {
            return Err(TosumuError::InvalidArgument(
                "cannot remove the last active keyslot",
            ));
        }

        // Zero the slot.
        let ks = KEYSLOT_REGION_OFFSET + slot_idx as usize * KEYSLOT_SIZE;
        session.page0[ks..ks + KEYSLOT_SIZE].fill(0);

        session.commit(&hmk)
    }

    /// Rotate the KEK for the Passphrase slot at `slot_idx`.
    ///
    /// Re-wraps the DEK under a new KEK derived from `new_passphrase`.
    pub fn rekey_kek(
        path: &Path,
        slot_idx: u16,
        old_passphrase: &str,
        new_passphrase: &str,
    ) -> Result<()> {
        let mut session = Page0EditSession::open(path)?;

        if slot_idx as usize >= session.keyslot_count {
            return Err(TosumuError::InvalidArgument("slot index out of range"));
        }

        // Verify target slot is Passphrase kind.
        let ks = KEYSLOT_REGION_OFFSET + slot_idx as usize * KEYSLOT_SIZE;
        if session.page0[ks + KS_OFF_KIND] != KEYSLOT_KIND_PASSPHRASE {
            return Err(TosumuError::InvalidArgument(
                "slot is not a Passphrase slot",
            ));
        }

        // Unlock target slot with old passphrase.
        let salt = read_keyslot_field::<16>(
            &session.page0,
            slot_idx as usize,
            KS_OFF_SALT,
            "bad keyslot salt length",
        )?;
        let kdf_params = read_keyslot_field::<32>(
            &session.page0,
            slot_idx as usize,
            KS_OFF_KDF_PARAMS,
            "bad keyslot kdf_params length",
        )?;
        let wrap_nonce = read_keyslot_field::<12>(
            &session.page0,
            slot_idx as usize,
            KS_OFF_WRAP_NONCE,
            "bad keyslot wrap nonce length",
        )?;
        let wrapped_dek = read_keyslot_field::<48>(
            &session.page0,
            slot_idx as usize,
            KS_OFF_WRAPPED_DEK,
            "bad keyslot wrapped DEK length",
        )?;
        let kcv = read_keyslot_field::<32>(
            &session.page0,
            slot_idx as usize,
            KS_OFF_KCV,
            "bad keyslot KCV length",
        )?;

        let old_kek = derive_passphrase_kek(old_passphrase, &salt, &kdf_params)?;
        verify_kcv(&old_kek, &kcv)?;
        let dek = unwrap_dek(
            &old_kek,
            &wrap_nonce,
            &wrapped_dek,
            slot_idx,
            session.dek_id,
            KEYSLOT_KIND_PASSPHRASE,
        )?;
        let (_, hmk, _) = derive_subkeys(&dek);

        // Wrap under new passphrase with fresh salt.
        let mut new_salt = [0u8; 16];
        getrandom::getrandom(&mut new_salt).map_err(|_| TosumuError::RngFailed)?;
        let new_kdf_params = pack_kdf_params(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST);
        let new_kek = derive_passphrase_kek(new_passphrase, &new_salt, &new_kdf_params)?;
        let (new_nonce, new_wrapped) = wrap_dek(
            &new_kek,
            &dek,
            slot_idx,
            session.dek_id,
            KEYSLOT_KIND_PASSPHRASE,
        )?;
        let new_kcv = compute_kcv(&new_kek);

        write_keyslot(
            &mut session.page0,
            slot_idx as usize,
            KEYSLOT_KIND_PASSPHRASE,
            session.dek_id,
            &new_salt,
            &new_kdf_params,
            &new_nonce,
            &new_wrapped,
            &new_kcv,
        );

        session.commit(&hmk)
    }

    /// Rotate the KEK for a Passphrase slot using a recovery key to unlock the DEK.
    pub fn rekey_kek_with_recovery_key(
        path: &Path,
        slot_idx: u16,
        recovery_str: &str,
        new_passphrase: &str,
    ) -> Result<()> {
        let (mut session, dek, hmk) =
            Page0EditSession::open_unlocked(path, ProtectorUnlock::RecoveryKey(recovery_str))?;

        if slot_idx as usize >= session.keyslot_count {
            return Err(TosumuError::InvalidArgument("slot index out of range"));
        }

        let ks = KEYSLOT_REGION_OFFSET + slot_idx as usize * KEYSLOT_SIZE;
        if session.page0[ks + KS_OFF_KIND] != KEYSLOT_KIND_PASSPHRASE {
            return Err(TosumuError::InvalidArgument(
                "slot is not a Passphrase slot",
            ));
        }

        let mut new_salt = [0u8; 16];
        getrandom::getrandom(&mut new_salt).map_err(|_| TosumuError::RngFailed)?;
        let new_kdf_params = pack_kdf_params(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST);
        let new_kek = derive_passphrase_kek(new_passphrase, &new_salt, &new_kdf_params)?;
        let (new_nonce, new_wrapped) = wrap_dek(
            &new_kek,
            &dek,
            slot_idx,
            session.dek_id,
            KEYSLOT_KIND_PASSPHRASE,
        )?;
        let new_kcv = compute_kcv(&new_kek);

        write_keyslot(
            &mut session.page0,
            slot_idx as usize,
            KEYSLOT_KIND_PASSPHRASE,
            session.dek_id,
            &new_salt,
            &new_kdf_params,
            &new_nonce,
            &new_wrapped,
            &new_kcv,
        );

        session.commit(&hmk)
    }

    /// Rotate the KEK for a Passphrase slot using a keyfile protector to unlock the DEK.
    pub fn rekey_kek_with_keyfile(
        path: &Path,
        slot_idx: u16,
        keyfile_path: &Path,
        new_passphrase: &str,
    ) -> Result<()> {
        let (mut session, dek, hmk) =
            Page0EditSession::open_unlocked(path, ProtectorUnlock::Keyfile(keyfile_path))?;

        if slot_idx as usize >= session.keyslot_count {
            return Err(TosumuError::InvalidArgument("slot index out of range"));
        }

        let ks = KEYSLOT_REGION_OFFSET + slot_idx as usize * KEYSLOT_SIZE;
        if session.page0[ks + KS_OFF_KIND] != KEYSLOT_KIND_PASSPHRASE {
            return Err(TosumuError::InvalidArgument(
                "slot is not a Passphrase slot",
            ));
        }

        let mut new_salt = [0u8; 16];
        getrandom::getrandom(&mut new_salt).map_err(|_| TosumuError::RngFailed)?;
        let new_kdf_params = pack_kdf_params(ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST);
        let new_kek = derive_passphrase_kek(new_passphrase, &new_salt, &new_kdf_params)?;
        let (new_nonce, new_wrapped) = wrap_dek(
            &new_kek,
            &dek,
            slot_idx,
            session.dek_id,
            KEYSLOT_KIND_PASSPHRASE,
        )?;
        let new_kcv = compute_kcv(&new_kek);

        write_keyslot(
            &mut session.page0,
            slot_idx as usize,
            KEYSLOT_KIND_PASSPHRASE,
            session.dek_id,
            &new_salt,
            &new_kdf_params,
            &new_nonce,
            &new_wrapped,
            &new_kcv,
        );

        session.commit(&hmk)
    }

    /// List active keyslots. Returns `Vec<(slot_index, kind_byte)>`.
    pub fn list_keyslots(path: &Path) -> Result<Vec<(u16, u8)>> {
        let mut file = OpenOptions::new().read(true).open(path)?;
        let page0 = read_page0(&mut file)?;
        validate_header(&page0)?;

        let kc = keyslot_count(&page0);
        let mut result = Vec::new();
        for i in 0..kc {
            let ks = KEYSLOT_REGION_OFFSET + i * KEYSLOT_SIZE;
            let kind = page0[ks + KS_OFF_KIND];
            // Exclude empty slots and the Sentinel bootstrap slot — callers expect
            // only user-visible protectors (passphrase, recovery key, etc.).
            if kind != KEYSLOT_KIND_EMPTY && kind != KEYSLOT_KIND_SENTINEL {
                result.push((i as u16, kind));
            }
        }
        Ok(result)
    }

    /// Decrypt page `pgno` and return `(plaintext, page_version)`.
    ///
    /// Prefer `with_page` for normal reads; this is for inspection tooling that
    /// also needs the page_version field.
    pub fn read_page(&self, pgno: u64) -> Result<([u8; PAGE_PLAINTEXT_SIZE], u64)> {
        self.ensure_healthy()?;
        self.validate_data_pgno(pgno)?;
        let frame = if let Some(buffered) = self.dirty_pages.get(&pgno) {
            **buffered
        } else {
            self.read_frame(pgno)?
        };
        decrypt_page(&self.page_key, pgno, &frame)
    }

    /// Read-only access to page `pgno`. Closure receives the decrypted plaintext.
    ///
    /// Also checks the dirty-page buffer so that read-your-own-writes works
    /// correctly inside a transaction (navigating tree structure after splits).
    pub fn with_page<F, T>(&self, pgno: u64, f: F) -> Result<T>
    where
        F: FnOnce(&[u8; PAGE_PLAINTEXT_SIZE]) -> Result<T>,
    {
        self.ensure_healthy()?;
        self.validate_data_pgno(pgno)?;
        // Read-your-own-writes: check dirty buffer first when inside a transaction.
        let frame = if let Some(buffered) = self.dirty_pages.get(&pgno) {
            **buffered
        } else {
            self.read_frame(pgno)?
        };
        let (plaintext, _version) = decrypt_page(&self.page_key, pgno, &frame)?;
        validate_plaintext_header(&plaintext, pgno)?;
        f(&plaintext)
    }

    /// Read-write access to page `pgno`. Closure receives a mutable plaintext
    /// buffer; on return the page is re-encrypted with a new nonce and
    /// incremented page_version.
    ///
    /// - Outside a transaction: writes directly to `.tsm` (auto-commit, for
    ///   internal ops like `init_page` and header flushes).
    /// - Inside a transaction (`begin_txn` called): buffers the final encrypted
    ///   frame in memory; `commit_txn` stages the WAL and then flushes `.tsm`.
    pub fn with_page_mut<F>(&mut self, pgno: u64, f: F) -> Result<()>
    where
        F: FnOnce(&mut [u8; PAGE_PLAINTEXT_SIZE]) -> Result<()>,
    {
        self.ensure_writable()?;
        self.validate_data_pgno(pgno)?;

        // For reads: check dirty buffer first (read-your-own-writes).
        let frame = if let Some(buffered) = self.dirty_pages.get(&pgno) {
            **buffered
        } else {
            self.read_frame(pgno)?
        };

        let (mut plaintext, version) = decrypt_page(&self.page_key, pgno, &frame)?;
        validate_plaintext_header(&plaintext, pgno)?;

        f(&mut plaintext)?;

        let new_version = version.checked_add(1).ok_or_else(|| TosumuError::Corrupt {
            pgno,
            reason: "page_version overflow: page has been written u64::MAX times",
        })?;
        let new_frame = encrypt_page(
            &self.page_key,
            pgno,
            new_version,
            plaintext[PAGE_OFF_TYPE],
            &plaintext,
        )?;

        if self.txn_active {
            // Keep only the final frame for this page. The complete WAL
            // transaction is staged from this map at commit time.
            self.dirty_pages.insert(pgno, Box::new(new_frame));
        } else {
            // Auto-commit path: write directly to .tsm.
            self.write_frame(pgno, &new_frame)?;
        }
        Ok(())
    }

    /// Allocate and initialise a new page. Returns its page number.
    ///
    /// Outside a transaction: the frame is written to disk *before* incrementing
    /// `page_count` and flushing the header, so a crash leaves an unreachable
    /// trailing page rather than a stale `page_count`.
    ///
    /// Inside a transaction: the frame is buffered in the dirty-page list and the
    /// header flush is deferred until `commit_txn`. Rollback restores `page_count`
    /// and discards the buffered frame — no orphaned pages are written to .tsm.
    ///
    pub fn allocate(&mut self, page_type: u8) -> Result<u64> {
        if self.freelist_head != 0 {
            let pgno = self.freelist_head;
            let next = self.with_page(pgno, |page| {
                if page[PAGE_OFF_TYPE] != PAGE_TYPE_FREE {
                    return Err(TosumuError::Corrupt {
                        pgno,
                        reason: "freelist points to a non-free page",
                    });
                }
                Ok(read_u64(page, PAGE_OFF_LEFTMOST))
            })?;
            self.freelist_head = next;
            self.init_page(pgno, page_type)?;
            self.flush_header()?;
            return Ok(pgno);
        }

        let pgno = self.page_count;
        // Write the initialised frame first, so the file is extended before
        // page_count advertises the new page.
        self.init_page(pgno, page_type)?;
        self.page_count += 1;
        self.flush_header()?;
        Ok(pgno)
    }

    /// Release an existing data page onto the persistent freelist.
    pub(crate) fn release_page(&mut self, pgno: u64) -> Result<()> {
        if pgno == 0 || pgno >= self.page_count {
            return Err(TosumuError::InvalidArgument(
                "invalid page number for release",
            ));
        }
        self.init_page(pgno, PAGE_TYPE_FREE)?;
        let previous_head = self.freelist_head;
        self.with_page_mut(pgno, |page| {
            write_u64(page, PAGE_OFF_LEFTMOST, previous_head);
            Ok(())
        })?;
        self.freelist_head = pgno;
        self.flush_header()
    }

    /// Initialize a newly allocated page and write it to disk (or buffer in WAL
    /// if a transaction is active).
    ///
    /// `pgno` must be > 0 and <= `page_count` (i.e. the next page to be
    /// allocated, or an existing page being re-initialised).
    pub fn init_page(&mut self, pgno: u64, page_type: u8) -> Result<()> {
        self.ensure_writable()?;
        if pgno == 0 || pgno > self.page_count {
            return Err(TosumuError::InvalidArgument(
                "invalid page number for initialization",
            ));
        }
        let mut plaintext = [0u8; PAGE_PLAINTEXT_SIZE];
        // Set page header: type, free_start, free_end.
        plaintext[PAGE_OFF_TYPE] = page_type;
        // PAGE_OFF_FLAGS = 0 (already zero)
        write_u16_buf(&mut plaintext, PAGE_OFF_SLOT_COUNT, 0u16);
        write_u16_buf(&mut plaintext, PAGE_OFF_FREE_START, PAGE_HEADER_SIZE as u16);
        write_u16_buf(
            &mut plaintext,
            PAGE_OFF_FREE_END,
            PAGE_PLAINTEXT_SIZE as u16,
        );
        // fragmented_bytes=0, reserved=0, next_leaf=0 — already zero
        let frame = encrypt_page(&self.page_key, pgno, 1, page_type, &plaintext)?;
        if self.txn_active {
            // Buffer the final frame. Commit publishes it through the WAL;
            // rollback can discard it without fallible physical cleanup.
            self.dirty_pages.insert(pgno, Box::new(frame));
        } else {
            self.write_frame(pgno, &frame)?
        }
        Ok(())
    }

    pub fn page_count(&self) -> u64 {
        self.page_count
    }

    // ── Transaction API ───────────────────────────────────────────────────────

    /// Begin a write transaction. Must not be called while one is already open.
    pub fn begin_txn(&mut self) -> Result<()> {
        self.ensure_writable()?;
        assert!(!self.txn_active, "nested transactions are not supported");
        self.txn_id = self.next_txn_id;
        self.next_txn_id += 1;
        self.txn_active = true;
        // Snapshot header fields so rollback can restore them.
        self.txn_saved_page_count = self.page_count;
        self.txn_saved_root_page = self.root_page;
        self.txn_saved_freelist_head = self.freelist_head;
        Ok(())
    }

    /// Commit the current transaction: write Commit record, fsync WAL, flush dirty pages to .tsm.
    ///
    /// Two-phase semantics:
    /// - **Phase 1** (WAL write + fsync): if this fails the transaction is un-committed;
    ///   roll back as normal.
    /// - **Phase 2** (.tsm flush): by this point the transaction is durable in the WAL.
    ///   A failure here returns [`TosumuError::CommittedButFlushFailed`].  The handle is
    ///   marked idle so subsequent calls do not panic, but the caller must reopen — WAL
    ///   recovery will replay the committed transaction automatically.
    fn flush_committed_pages<T: PagerPhaseTwoFile>(
        flush_file: &mut T,
        pages: &[(u64, Box<[u8; PAGE_SIZE]>)],
        page0_frame: Option<&Box<[u8; PAGE_SIZE]>>,
    ) -> Result<()> {
        for (pgno, frame) in pages {
            let offset = *pgno * PAGE_SIZE as u64;
            flush_file.seek(SeekFrom::Start(offset))?;
            flush_file.write_all(frame.as_ref())?;
        }
        if let Some(frame) = page0_frame {
            flush_file.seek(SeekFrom::Start(0))?;
            flush_file.write_all(frame.as_ref())?;
        }
        if !pages.is_empty() || page0_frame.is_some() {
            flush_file.sync_data()?;
        }
        Ok(())
    }

    fn commit_txn_with_phase_two_file<T: PagerPhaseTwoFile>(
        &mut self,
        flush_file: &mut T,
    ) -> Result<()> {
        self.ensure_writable()?;
        assert!(
            self.txn_active,
            "commit_txn called with no active transaction"
        );

        // Phase 1: make the transaction durable in the WAL.
        // Build page 0 bytes once so they can be reused in both the WAL record and
        // the .tsm flush (avoids reading page 0 twice and eliminates a second fsync).
        let page0_frame = if self.pending_header_flush {
            match self.build_updated_page0() {
                Ok(page0) => Some(Box::new(page0)),
                Err(error) => {
                    self.health = PagerHealth::Poisoned;
                    self.txn_active = false;
                    return Err(error);
                }
            }
        } else {
            None
        };

        // Drain and sort final unique page frames before writing the first WAL
        // byte. This is also the representation a transaction budget can
        // preflight without counting repeated mutations of the same page.
        let mut pages: Vec<(u64, Box<[u8; PAGE_SIZE]>)> = self.dirty_pages.drain().collect();
        pages.sort_unstable_by_key(|(pgno, _)| *pgno);

        let txn_id = self.txn_id;
        let wal_result = (|| -> Result<()> {
            self.wal_mut()?.append(&WalRecord::Begin { txn_id })?;
            for (pgno, frame) in &pages {
                let page_version = u64::from_le_bytes(
                    frame[PAGE_VERSION_OFFSET..PAGE_VERSION_OFFSET + PAGE_VERSION_SIZE]
                        .try_into()
                        .expect("page-version field has a fixed width"),
                );
                self.wal_mut()?.append(&WalRecord::PageWrite {
                    pgno: *pgno,
                    page_version,
                    frame: frame.clone(),
                })?;
            }
            if let Some(frame) = &page0_frame {
                self.wal_mut()?.append(&WalRecord::PageWrite {
                    pgno: 0,
                    page_version: 0,
                    frame: frame.clone(),
                })?;
            }
            self.wal_mut()?.append(&WalRecord::Commit { txn_id })?;
            self.wal_mut()?.sync()
        })();
        if let Err(error) = wal_result {
            self.health = PagerHealth::Poisoned;
            self.txn_active = false;
            self.pending_header_flush = false;
            return Err(error);
        }

        // Transaction is now committed. Clear txn_active before the .tsm flush
        // so a failure has one explicit terminal handle state.
        self.txn_active = false;
        self.pending_header_flush = false;

        // Phase 2: write all pages (data + optional page 0) to .tsm with a single
        // fsync at the end.  WAL recovery covers crashes, so this flush is opportunistic.
        // A failure here means the data is safe in the WAL but the handle's caches no
        // longer reflect .tsm — caller must reopen.
        let flush_result = Self::flush_committed_pages(flush_file, &pages, page0_frame.as_ref());
        if let Err(e) = flush_result {
            self.health = PagerHealth::Poisoned;
            let io_err = match e {
                TosumuError::Io(io) => io,
                other => return Err(other),
            };
            return Err(TosumuError::CommittedButFlushFailed { source: io_err });
        }

        // The committed frames now live in .tsm, so leaving them in the WAL would
        // let a later reopen replay stale snapshots over newer auto-commit writes.
        if let Err(error) = self.wal_mut()?.truncate() {
            self.health = PagerHealth::Poisoned;
            return Err(error);
        }
        Ok(())
    }

    pub fn commit_txn(&mut self) -> Result<()> {
        let mut flush_file = self.file.try_clone()?;
        self.commit_txn_with_phase_two_file(&mut flush_file)
    }

    #[cfg(test)]
    pub(crate) fn commit_txn_with_crash_file(
        &mut self,
        crash_file: &mut crate::test_helpers::CrashFile,
    ) -> Result<()> {
        self.commit_txn_with_phase_two_file(crash_file)
    }

    /// Roll back the current transaction: discard dirty pages and restore
    /// header fields (page_count, root_page) to the values at begin_txn.
    pub fn rollback_txn(&mut self) {
        self.dirty_pages.clear();
        self.pending_header_flush = false;
        self.page_count = self.txn_saved_page_count;
        self.root_page = self.txn_saved_root_page;
        self.freelist_head = self.txn_saved_freelist_head;
        self.txn_active = false;
    }

    /// Return the B+ tree root page number (0 if not yet set).
    pub fn root_page(&self) -> u64 {
        self.root_page
    }

    /// Persist a new B+ tree root page number.
    pub fn set_root_page(&mut self, pgno: u64) -> Result<()> {
        self.ensure_writable()?;
        self.root_page = pgno;
        self.flush_header()
    }

    // ── Header flush ─────────────────────────────────────────────────────────

    /// Write updated page_count, freelist_head and root_page back to page 0.
    ///
    /// Inside a transaction, the write is deferred: `pending_header_flush` is
    /// set to `true` and the actual write is performed at `commit_txn` time
    /// (included in the WAL and then flushed to .tsm with the other dirty pages).
    /// On rollback the saved snapshot values are restored instead.
    pub fn flush_header(&mut self) -> Result<()> {
        self.ensure_writable()?;
        if self.txn_active {
            self.pending_header_flush = true;
            return Ok(());
        }
        let page0 = self.build_updated_page0()?;
        self.file.seek(SeekFrom::Start(0))?;
        self.file.write_all(&page0)?;
        self.file.sync_data()?;
        Ok(())
    }

    /// Build the updated page-0 bytes (header fields + MAC) without writing to disk.
    fn build_updated_page0(&mut self) -> Result<[u8; PAGE_SIZE]> {
        let mut page0 = [0u8; PAGE_SIZE];
        self.file.seek(SeekFrom::Start(0))?;
        self.file.read_exact(&mut page0)?;
        write_u64(&mut page0, OFF_PAGE_COUNT, self.page_count);
        write_u64(&mut page0, OFF_FREELIST_HEAD, self.freelist_head);
        write_u64(&mut page0, OFF_ROOT_PAGE, self.root_page);
        if let Some(ref hmk) = self.header_mac_key {
            let mac = compute_header_mac(hmk, &page0, MAX_KEYSLOTS);
            page0[OFF_HEADER_MAC..OFF_HEADER_MAC + 32].copy_from_slice(&mac);
        }
        Ok(page0)
    }

    // ── private ──────────────────────────────────────────────────────────────

    // ── Helpers ───────────────────────────────────────────────────────────────

    fn validate_data_pgno(&self, pgno: u64) -> Result<()> {
        if pgno == 0 {
            return Err(TosumuError::InvalidArgument(
                "page 0 is the file header, not a data page",
            ));
        }
        if pgno >= self.page_count {
            return Err(TosumuError::InvalidArgument("page number out of range"));
        }
        Ok(())
    }

    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn read_raw_frame(&self, pgno: u64) -> Result<[u8; PAGE_SIZE]> {
        self.read_frame(pgno)
    }

    fn read_frame(&self, pgno: u64) -> Result<[u8; PAGE_SIZE]> {
        self.validate_data_pgno(pgno)?;
        let mut frame = [0u8; PAGE_SIZE];
        let offset = pgno * PAGE_SIZE as u64;
        // PERF TODO: try_clone() issues a syscall and creates a new OS file handle per read.
        // This is acceptable for MVP+1 (no page cache) but should be replaced in Stage 2
        // with either a page cache (preferred) or interior mutability (RefCell<File>).
        // Tracking: remove try_clone when page cache is introduced.
        let mut f = self.file.try_clone()?;
        f.seek(SeekFrom::Start(offset))?;
        f.read_exact(&mut frame)?;
        Ok(frame)
    }

    fn ensure_writable(&self) -> Result<()> {
        self.ensure_healthy()?;
        if self.read_only {
            return Err(TosumuError::InvalidArgument("database handle is read-only"));
        }
        Ok(())
    }

    fn ensure_healthy(&self) -> Result<()> {
        if self.health == PagerHealth::Poisoned {
            return Err(TosumuError::Poisoned);
        }
        Ok(())
    }

    fn wal_mut(&mut self) -> Result<&mut WalWriter> {
        self.wal
            .as_mut()
            .ok_or(TosumuError::InvalidArgument("database handle is read-only"))
    }

    fn write_frame(&mut self, pgno: u64, frame: &[u8; PAGE_SIZE]) -> Result<()> {
        // Note: pgno validation is intentionally omitted here. write_frame is
        // called from init_page (via allocate) with pgno == page_count, which
        // is the next-to-be-allocated slot and is correctly > the current
        // validate_data_pgno upper bound. Callers (with_page_mut, commit_txn,
        // init_page) are responsible for their own precondition checks.
        let offset = pgno * PAGE_SIZE as u64;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.write_all(frame)?;
        self.file.sync_data()?;
        Ok(())
    }
}

fn open_file_for_mode(path: &Path, read_only: bool) -> Result<File> {
    let mut options = OpenOptions::new();
    options.read(true);
    if !read_only {
        options.write(true);
    }
    Ok(options.open(path)?)
}

fn sentinel_dek_from_page0(page0: &[u8; PAGE_SIZE]) -> Result<[u8; 32]> {
    let ks_start = KEYSLOT_REGION_OFFSET;
    let ks_kind = page0[ks_start + KS_OFF_KIND];
    match ks_kind {
        KEYSLOT_KIND_SENTINEL => {
            let mut dek = [0u8; 32];
            dek.copy_from_slice(
                &page0[ks_start + KS_OFF_WRAPPED_DEK..ks_start + KS_OFF_WRAPPED_DEK + 32],
            );
            Ok(dek)
        }
        KEYSLOT_KIND_PASSPHRASE | KEYSLOT_KIND_RECOVERY_KEY | KEYSLOT_KIND_KEYFILE => {
            Err(TosumuError::WrongKey)
        }
        _ => Err(TosumuError::NotATosumFile),
    }
}

fn header_mac_from_page0(page0: &[u8; PAGE_SIZE]) -> [u8; 32] {
    let mut stored_mac = [0u8; 32];
    stored_mac.copy_from_slice(&page0[OFF_HEADER_MAC..OFF_HEADER_MAC + 32]);
    stored_mac
}

fn unlock_page0_for_open(
    page0: &[u8; PAGE_SIZE],
    unlock: OpenUnlock<'_>,
) -> Result<([u8; 32], Option<[u8; 32]>)> {
    let dek_id = read_u64(page0, OFF_DEK_ID);
    let keyslot_count = keyslot_count(page0);

    match unlock {
        OpenUnlock::Sentinel => {
            let dek = sentinel_dek_from_page0(page0)?;
            let (page_key, _, _) = derive_subkeys(&dek);
            Ok((page_key, None))
        }
        OpenUnlock::Passphrase(passphrase) => {
            let (dek, is_encrypted) =
                try_unlock_passphrase(page0, passphrase, dek_id, keyslot_count)?;
            let (page_key, derived_hmk, _) = derive_subkeys(&dek);
            let header_mac_key = if is_encrypted {
                verify_header_mac(
                    &derived_hmk,
                    page0,
                    keyslot_count,
                    &header_mac_from_page0(page0),
                )?;
                Some(derived_hmk)
            } else {
                None
            };
            Ok((page_key, header_mac_key))
        }
        OpenUnlock::RecoveryKey(recovery_str) => {
            let kek = derive_recovery_kek(recovery_str)?;
            let dek = try_unlock_with_kek(
                page0,
                &kek,
                dek_id,
                keyslot_count,
                KEYSLOT_KIND_RECOVERY_KEY,
            )?;
            let (page_key, derived_hmk, _) = derive_subkeys(&dek);
            verify_header_mac(
                &derived_hmk,
                page0,
                keyslot_count,
                &header_mac_from_page0(page0),
            )?;
            Ok((page_key, Some(derived_hmk)))
        }
        OpenUnlock::Keyfile(keyfile_path) => {
            let kek = read_keyfile_kek(keyfile_path)?;
            let dek =
                try_unlock_with_kek(page0, &kek, dek_id, keyslot_count, KEYSLOT_KIND_KEYFILE)?;
            let (page_key, derived_hmk, _) = derive_subkeys(&dek);
            verify_header_mac(
                &derived_hmk,
                page0,
                keyslot_count,
                &header_mac_from_page0(page0),
            )?;
            Ok((page_key, Some(derived_hmk)))
        }
    }
}

fn finish_open_for_mode(
    file: File,
    page_key: [u8; 32],
    header_mac_key: Option<[u8; 32]>,
    page0: &[u8; PAGE_SIZE],
    path: &Path,
    read_only: bool,
    writer_guard: Option<WriterGuard>,
) -> Result<Pager> {
    if read_only {
        finish_open_readonly(file, page_key, header_mac_key, page0, path)
    } else {
        let writer_guard = writer_guard.expect("writable pager open must hold the writer gate");
        finish_open(file, page_key, header_mac_key, page0, path, writer_guard)
    }
}

impl Pager {
    fn open_inner(path: &Path, read_only: bool, unlock: OpenUnlock<'_>) -> Result<Self> {
        let writer_guard = if read_only {
            None
        } else {
            Some(WriterGuard::acquire(path)?)
        };
        let mut file = open_file_for_mode(path, read_only)?;
        let page0 = read_page0(&mut file)?;
        validate_header(&page0)?;

        let (page_key, header_mac_key) = unlock_page0_for_open(&page0, unlock)?;
        finish_open_for_mode(
            file,
            page_key,
            header_mac_key,
            &page0,
            path,
            read_only,
            writer_guard,
        )
    }
}

/// Complete a Pager open given a derived page_key + optional MAC key.
fn finish_open(
    mut file: File,
    page_key: [u8; 32],
    header_mac_key: Option<[u8; 32]>,
    page0: &[u8; PAGE_SIZE],
    path: &Path,
    writer_guard: WriterGuard,
) -> Result<Pager> {
    let page_count = read_u64(page0, OFF_PAGE_COUNT);

    // Sanity-check the file length against the advertised page count.
    // A truncated file means the header is lying; reject rather than trust it.
    //
    // Note: we check `<` not `!=`. The crash-safe allocate ordering writes the
    // new frame *before* incrementing page_count; a crash between those two
    // steps leaves a file that is exactly one page longer than page_count
    // predicts.  Rejecting that case would prevent opening after a crash.
    // Individual page integrity is guaranteed by AEAD regardless of trailing
    // bytes, so the looser bound here does not weaken security.
    let file_len = file.metadata()?.len();
    let expected_len = page_count
        .checked_mul(PAGE_SIZE as u64)
        .ok_or(TosumuError::Corrupt {
            pgno: 0,
            reason: "page_count overflow",
        })?;
    if file_len < expected_len {
        return Err(TosumuError::FileTruncated {
            expected: expected_len,
            found: file_len,
        });
    }

    let freelist_head = read_u64(page0, OFF_FREELIST_HEAD);
    let root_page = read_u64(page0, OFF_ROOT_PAGE);

    let wp = wal_path(path);
    if wp.exists() {
        // If committed frames exist, replay them once and truncate the WAL so
        // future opens cannot reapply stale snapshots over newer auto-commit writes.
        crate::wal::checkpoint_guarded(path, &wp, &writer_guard)?;
        // Re-read page0 after recovery/checkpoint so page_count/root_page are current.
        file.seek(SeekFrom::Start(0))?;
        let mut refreshed = [0u8; PAGE_SIZE];
        file.read_exact(&mut refreshed)?;
        validate_header(&refreshed)?;
        if let Some(ref hmk) = header_mac_key {
            let keyslot_count = keyslot_count(&refreshed);
            let stored_mac = read_header_mac_field(&refreshed)?;
            verify_header_mac(hmk, &refreshed, keyslot_count, &stored_mac)?;
        }
        let page_count = read_u64(&refreshed, OFF_PAGE_COUNT);
        let refreshed_file_len = file.metadata()?.len();
        let expected_len =
            page_count
                .checked_mul(PAGE_SIZE as u64)
                .ok_or(TosumuError::Corrupt {
                    pgno: 0,
                    reason: "page_count overflow",
                })?;
        if refreshed_file_len < expected_len {
            return Err(TosumuError::FileTruncated {
                expected: expected_len,
                found: refreshed_file_len,
            });
        }
        let freelist_head = read_u64(&refreshed, OFF_FREELIST_HEAD);
        let root_page = read_u64(&refreshed, OFF_ROOT_PAGE);
        let wal = WalWriter::open_or_create_seeded(&wp, wal_seed_from_page0(page0)?)?;
        return Ok(Pager {
            file,
            _writer_guard: Some(writer_guard),
            page_key,
            header_mac_key,
            page_count,
            freelist_head,
            root_page,
            wal: Some(wal),
            read_only: false,
            health: PagerHealth::Healthy,
            txn_active: false,
            txn_id: 0,
            next_txn_id: 1,
            dirty_pages: HashMap::new(),
            pending_header_flush: false,
            txn_saved_page_count: 0,
            txn_saved_root_page: 0,
            txn_saved_freelist_head: 0,
        });
    }
    let wal = WalWriter::open_or_create_seeded(&wp, wal_seed_from_page0(page0)?)?;
    Ok(Pager {
        file,
        _writer_guard: Some(writer_guard),
        page_key,
        header_mac_key,
        page_count,
        freelist_head,
        root_page,
        wal: Some(wal),
        read_only: false,
        health: PagerHealth::Healthy,
        txn_active: false,
        txn_id: 0,
        next_txn_id: 1,
        dirty_pages: HashMap::new(),
        pending_header_flush: false,
        txn_saved_page_count: 0,
        txn_saved_root_page: 0,
        txn_saved_freelist_head: 0,
    })
}

fn finish_open_readonly(
    file: File,
    page_key: [u8; 32],
    header_mac_key: Option<[u8; 32]>,
    page0: &[u8; PAGE_SIZE],
    path: &Path,
) -> Result<Pager> {
    let page_count = read_u64(page0, OFF_PAGE_COUNT);
    let file_len = file.metadata()?.len();
    let expected_len = page_count
        .checked_mul(PAGE_SIZE as u64)
        .ok_or(TosumuError::Corrupt {
            pgno: 0,
            reason: "page_count overflow",
        })?;
    if file_len < expected_len {
        return Err(TosumuError::FileTruncated {
            expected: expected_len,
            found: file_len,
        });
    }

    let mut pager = Pager {
        file,
        _writer_guard: None,
        page_key,
        header_mac_key,
        page_count,
        freelist_head: read_u64(page0, OFF_FREELIST_HEAD),
        root_page: read_u64(page0, OFF_ROOT_PAGE),
        wal: None,
        read_only: true,
        health: PagerHealth::Healthy,
        txn_active: false,
        txn_id: 0,
        next_txn_id: 1,
        dirty_pages: HashMap::new(),
        pending_header_flush: false,
        txn_saved_page_count: 0,
        txn_saved_root_page: 0,
        txn_saved_freelist_head: 0,
    };

    let wp = wal_path(path);
    if wp.exists() {
        overlay_committed_wal(&wp, &mut pager)?;
    }
    Ok(pager)
}

fn wal_seed_from_page0(page0: &[u8; PAGE_SIZE]) -> Result<u64> {
    read_u64(page0, OFF_WAL_CHECKPOINT_LSN)
        .checked_add(1)
        .ok_or(TosumuError::Corrupt {
            pgno: 0,
            reason: "wal_checkpoint_lsn overflow",
        })
}

fn overlay_committed_wal(wal_path: &Path, pager: &mut Pager) -> Result<()> {
    let records = WalReader::read_all_strict(wal_path)?;
    let index = CommittedWalIndex::from_records(&records, 0)?;
    let latest = index.latest_commit_lsn();
    index.for_each_page_at(latest, |pgno, version| {
        if pgno == 0 {
            let page0 = version.frame.as_ref();
            validate_header(page0)?;
            if let Some(ref hmk) = pager.header_mac_key {
                let keyslot_count = keyslot_count(page0);
                let stored_mac = read_header_mac_field(page0)?;
                verify_header_mac(hmk, page0, keyslot_count, &stored_mac)?;
            }
            pager.page_count = read_u64(page0, OFF_PAGE_COUNT);
            pager.freelist_head = read_u64(page0, OFF_FREELIST_HEAD);
            pager.root_page = read_u64(page0, OFF_ROOT_PAGE);
        } else {
            pager.dirty_pages.insert(pgno, version.frame.clone());
        }
        Ok(())
    })
}

// ── Pager unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::btree::BTree;
    use crate::test_helpers::{CrashFile, CrashPhase};
    use tempfile::TempDir;

    /// Create a Pager with one allocated data page. Returns (Pager, TempDir, pgno=1).
    fn setup_one_page() -> (Pager, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.tsm");
        let mut p = Pager::create(&path).unwrap();
        p.allocate(PAGE_TYPE_LEAF).unwrap(); // page 1
        (p, dir)
    }

    // ── pgno validation ───────────────────────────────────────────────────────

    #[test]
    fn read_frame_rejects_pgno_zero() {
        let (p, _dir) = setup_one_page();
        let err = p.read_raw_frame(0).unwrap_err();
        assert!(
            matches!(err, TosumuError::InvalidArgument(_)),
            "expected InvalidArgument, got {err:?}",
        );
    }

    #[test]
    fn read_frame_rejects_pgno_out_of_range() {
        let (p, _dir) = setup_one_page();
        // page_count == 2 (page 0 + page 1); pgno 2 is out of range
        let err = p.read_raw_frame(2).unwrap_err();
        assert!(
            matches!(err, TosumuError::InvalidArgument(_)),
            "expected InvalidArgument, got {err:?}",
        );
    }

    #[test]
    fn with_page_rejects_pgno_zero() {
        let (p, _dir) = setup_one_page();
        let err = p.with_page(0, |_| Ok(())).unwrap_err();
        assert!(matches!(err, TosumuError::InvalidArgument(_)));
    }

    // ── validate_plaintext_header (via with_page after raw frame surgery) ─────

    /// Flip byte `offset` in data page `pgno` to `value` by writing directly
    /// into the *ciphertext* at the position that corresponds to offset within
    /// the plaintext nonce-XOR stream.  That is: we can't flip plaintext bytes
    /// without breaking the AEAD tag, so instead we corrupt the *ciphertext*
    /// byte at CIPHERTEXT_OFFSET + offset to produce an arbitrary tag failure.
    ///
    /// The correct way to test plaintext-header validation is to intercept
    /// *after* decrypt — but since pager hides that internally, the easiest
    /// approach is to write a synthetic encrypted frame directly.
    ///
    /// Here we use a simpler trick: allocate a page, read the raw frame,
    /// use the pager's `with_page` machinery (which will decrypt correctly),
    /// then call `validate_plaintext_header` directly as a unit test of the
    /// free function (it is pub(super) within this test module).
    #[test]
    fn validate_plaintext_header_rejects_unknown_page_type() {
        let mut page = [0u8; PAGE_PLAINTEXT_SIZE];
        // Set up a well-formed LEAF page first
        page[PAGE_OFF_TYPE] = PAGE_TYPE_LEAF;
        let free_start = PAGE_HEADER_SIZE as u16;
        let free_end = PAGE_PLAINTEXT_SIZE as u16;
        page[PAGE_OFF_FREE_START..PAGE_OFF_FREE_START + 2]
            .copy_from_slice(&free_start.to_le_bytes());
        page[PAGE_OFF_FREE_END..PAGE_OFF_FREE_END + 2].copy_from_slice(&free_end.to_le_bytes());
        assert!(validate_plaintext_header(&page, 1).is_ok());

        // Flip to an unknown type
        page[PAGE_OFF_TYPE] = 0xFF;
        let err = validate_plaintext_header(&page, 1).unwrap_err();
        assert!(
            matches!(err, TosumuError::Corrupt { pgno: 1, .. }),
            "expected Corrupt {{ pgno: 1 }}, got {err:?}",
        );
    }

    #[test]
    fn validate_plaintext_header_rejects_free_start_gt_free_end() {
        let mut page = [0u8; PAGE_PLAINTEXT_SIZE];
        page[PAGE_OFF_TYPE] = PAGE_TYPE_LEAF;
        // free_start > free_end
        page[PAGE_OFF_FREE_START..PAGE_OFF_FREE_START + 2].copy_from_slice(&200u16.to_le_bytes());
        page[PAGE_OFF_FREE_END..PAGE_OFF_FREE_END + 2].copy_from_slice(&100u16.to_le_bytes());
        let err = validate_plaintext_header(&page, 1).unwrap_err();
        assert!(matches!(err, TosumuError::Corrupt { pgno: 1, .. }));
    }

    #[test]
    fn validate_plaintext_header_rejects_free_end_overflow() {
        let mut page = [0u8; PAGE_PLAINTEXT_SIZE];
        page[PAGE_OFF_TYPE] = PAGE_TYPE_INTERNAL;
        // free_end > PAGE_PLAINTEXT_SIZE
        let too_big = (PAGE_PLAINTEXT_SIZE + 1) as u16;
        page[PAGE_OFF_FREE_START..PAGE_OFF_FREE_START + 2].copy_from_slice(&0u16.to_le_bytes());
        page[PAGE_OFF_FREE_END..PAGE_OFF_FREE_END + 2].copy_from_slice(&too_big.to_le_bytes());
        let err = validate_plaintext_header(&page, 1).unwrap_err();
        assert!(matches!(err, TosumuError::Corrupt { pgno: 1, .. }));
    }

    #[test]
    fn validate_plaintext_header_accepts_overflow_and_free_page_types() {
        // OVERFLOW and FREE pages don't carry free-space pointers; validator
        // must not reject them even when those bytes are zero.
        for &pt in &[PAGE_TYPE_OVERFLOW, PAGE_TYPE_FREE] {
            let mut page = [0u8; PAGE_PLAINTEXT_SIZE];
            page[PAGE_OFF_TYPE] = pt;
            // Leave free_start=0, free_end=0 — invalid for btree pages but
            // must be accepted for OVERFLOW / FREE.
            assert!(
                validate_plaintext_header(&page, 1).is_ok(),
                "page_type {pt} should be accepted",
            );
        }
    }

    #[test]
    fn commit_txn_flush_failure_returns_committed_but_flush_failed_and_reopens_cleanly() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("flush_fail.tsm");
        let wal = crate::wal::wal_path(&path);
        let _ = std::fs::remove_file(&wal);

        let mut tree = BTree::create(&path).unwrap();
        tree.begin_txn().unwrap();
        tree.put(b"recover-me", b"value").unwrap();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let mut crash_file = CrashFile::new(file, CrashPhase::AfterWrite);
        let err = tree
            .pager
            .commit_txn_with_phase_two_file(&mut crash_file)
            .unwrap_err();
        assert!(matches!(err, TosumuError::CommittedButFlushFailed { .. }));

        drop(tree);

        let reopened = BTree::open(&path).unwrap();
        assert_eq!(
            reopened.get(b"recover-me").unwrap(),
            Some(b"value".to_vec())
        );
    }

    #[test]
    fn commit_txn_partial_write_returns_committed_but_flush_failed_and_reopens_cleanly() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("partial_flush_fail.tsm");
        let wal = crate::wal::wal_path(&path);
        let _ = std::fs::remove_file(&wal);

        let mut tree = BTree::create(&path).unwrap();
        tree.begin_txn().unwrap();
        tree.put(b"recover-midwrite", b"value").unwrap();

        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&path)
            .unwrap();
        let mut crash_file = CrashFile::new(
            file,
            CrashPhase::MidWrite {
                fail_after_bytes: (PAGE_SIZE / 2) as u64,
            },
        );
        let err = tree
            .pager
            .commit_txn_with_phase_two_file(&mut crash_file)
            .unwrap_err();
        assert!(matches!(err, TosumuError::CommittedButFlushFailed { .. }));

        drop(tree);

        let reopened = BTree::open(&path).unwrap();
        assert_eq!(
            reopened.get(b"recover-midwrite").unwrap(),
            Some(b"value".to_vec())
        );
    }

    // ── truncation detection ──────────────────────────────────────────────────

    #[test]
    fn open_rejects_truncated_file() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("trunc.tsm");
        {
            let mut p = Pager::create(&path).unwrap();
            p.allocate(PAGE_TYPE_LEAF).unwrap();
            // page_count == 2; file should be 2 * PAGE_SIZE bytes
        }
        // Truncate to less than 2 pages
        let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
        file.set_len((PAGE_SIZE as u64) + 1).unwrap(); // one byte short of 2 pages
        drop(file);

        let result = Pager::open(&path);
        assert!(
            matches!(result, Err(TosumuError::FileTruncated { .. })),
            "expected FileTruncated, got: {:?}",
            result.err(),
        );
    }
}
