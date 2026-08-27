use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};

use crate::error::{Result, TosumuError};
use crate::format::*;

// ── Page header helpers ───────────────────────────────────────────────────────

pub(super) fn write_u16_buf(buf: &mut [u8], offset: usize, v: u16) {
    buf[offset..offset + 2].copy_from_slice(&v.to_le_bytes());
}

/// Sanity-check the plaintext page header after decryption.
///
/// Catches cases where an attacker (or corrupted file) flips page_type or
/// free_start/free_end to values that would cause higher layers to misbehave.
///
/// The `free_start`/`free_end` bounds check only applies to LEAF and INTERNAL
/// pages: OVERFLOW and FREE pages do not use those header fields.
pub(super) fn validate_plaintext_header(page: &[u8; PAGE_PLAINTEXT_SIZE], pgno: u64) -> Result<()> {
    let page_type = page[PAGE_OFF_TYPE];
    match page_type {
        PAGE_TYPE_LEAF | PAGE_TYPE_INTERNAL => {
            // B-tree pages: validate the free-space region pointers.
            let free_start =
                u16::from_le_bytes([page[PAGE_OFF_FREE_START], page[PAGE_OFF_FREE_START + 1]])
                    as usize;
            let free_end =
                u16::from_le_bytes([page[PAGE_OFF_FREE_END], page[PAGE_OFF_FREE_END + 1]]) as usize;
            if free_start > free_end {
                return Err(TosumuError::Corrupt {
                    pgno,
                    reason: "decrypted page: free_start > free_end",
                });
            }
            if free_end > PAGE_PLAINTEXT_SIZE {
                return Err(TosumuError::Corrupt {
                    pgno,
                    reason: "decrypted page: free_end > PAGE_PLAINTEXT_SIZE",
                });
            }
        }
        PAGE_TYPE_OVERFLOW | PAGE_TYPE_FREE => {
            // Overflow and free pages don't use the btree header fields;
            // no further structural checks apply here.
        }
        _ => {
            return Err(TosumuError::Corrupt {
                pgno,
                reason: "decrypted page has unknown page_type",
            });
        }
    }
    Ok(())
}

// ── Keyslot helpers ───────────────────────────────────────────────────────────

/// Validate magic, format version and page size from a page-0 buffer.
pub(super) fn validate_header(page0: &[u8; PAGE_SIZE]) -> Result<()> {
    if !check_magic(page0) {
        return Err(TosumuError::NotATosumFile);
    }
    let fv = read_u16(page0, OFF_FORMAT_VERSION);
    if fv > FORMAT_VERSION {
        return Err(TosumuError::NewerFormat {
            found: fv,
            supported_max: FORMAT_VERSION,
        });
    }
    let ps = read_u16(page0, OFF_PAGE_SIZE);
    if ps as usize != PAGE_SIZE {
        return Err(TosumuError::PageSizeMismatch {
            found: ps,
            expected: PAGE_SIZE as u16,
        });
    }
    // Sanity-check keyslot_count in page 0: out-of-range values are clamped by every
    // caller, but validating here surfaces corruption/tampering at one central point.
    let kc = read_u16(page0, OFF_KEYSLOT_COUNT) as usize;
    if kc == 0 || kc > MAX_KEYSLOTS {
        return Err(TosumuError::Corrupt {
            pgno: 0,
            reason: "keyslot_count in header is out of valid range",
        });
    }
    Ok(())
}

/// Return the validated, clamped keyslot count from page 0.
///
/// Centralises the `max(1).min(MAX_KEYSLOTS)` clamping that every open path
/// needs. The value is already bounds-checked by `validate_header` before
/// this is called, so the clamp is a safety net for callers that may not have
/// invoked `validate_header` first.
pub(super) fn keyslot_count(page0: &[u8; PAGE_SIZE]) -> usize {
    (read_u16(page0, OFF_KEYSLOT_COUNT) as usize)
        .max(1)
        .min(MAX_KEYSLOTS)
}

/// Read the full page-0 from an open file.
pub(super) fn read_page0(file: &mut File) -> Result<[u8; PAGE_SIZE]> {
    let mut page0 = [0u8; PAGE_SIZE];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut page0)?;
    Ok(page0)
}

/// Write page-0 back to an open file and fsync.
pub(super) fn write_page0(file: &mut File, page0: &[u8; PAGE_SIZE]) -> Result<()> {
    file.seek(SeekFrom::Start(0))?;
    file.write_all(page0)?;
    file.sync_data()?;
    Ok(())
}

fn read_page0_field<const N: usize>(
    page0: &[u8; PAGE_SIZE],
    offset: usize,
    reason: &'static str,
) -> Result<[u8; N]> {
    let end = offset
        .checked_add(N)
        .ok_or(TosumuError::Corrupt { pgno: 0, reason })?;
    let bytes = page0
        .get(offset..end)
        .ok_or(TosumuError::Corrupt { pgno: 0, reason })?;
    let mut value = [0u8; N];
    value.copy_from_slice(bytes);
    Ok(value)
}

pub(super) fn read_keyslot_field<const N: usize>(
    page0: &[u8; PAGE_SIZE],
    slot_idx: usize,
    field_offset: usize,
    reason: &'static str,
) -> Result<[u8; N]> {
    let ks = KEYSLOT_REGION_OFFSET + slot_idx * KEYSLOT_SIZE;
    read_page0_field(page0, ks + field_offset, reason)
}

pub(super) fn read_header_mac_field(page0: &[u8; PAGE_SIZE]) -> Result<[u8; 32]> {
    read_page0_field(page0, OFF_HEADER_MAC, "bad header MAC length")
}

/// Write a single keyslot into `page0` at `slot_idx`.
pub(super) fn write_keyslot(
    page0: &mut [u8; PAGE_SIZE],
    slot_idx: usize,
    kind: u8,
    dek_id: u64,
    salt: &[u8; 16],
    kdf_params: &[u8; 32],
    wrap_nonce: &[u8; 12],
    wrapped_dek: &[u8; 48],
    kcv: &[u8; 32],
) {
    let ks = KEYSLOT_REGION_OFFSET + slot_idx * KEYSLOT_SIZE;
    // Zero the slot first (clears any previous data / reserved bytes).
    page0[ks..ks + KEYSLOT_SIZE].fill(0);
    page0[ks + KS_OFF_KIND] = kind;
    page0[ks + KS_OFF_VERSION] = 1;
    write_u64(page0, ks + KS_OFF_DEK_ID, dek_id);
    page0[ks + KS_OFF_SALT..ks + KS_OFF_SALT + 16].copy_from_slice(salt);
    page0[ks + KS_OFF_KDF_PARAMS..ks + KS_OFF_KDF_PARAMS + 32].copy_from_slice(kdf_params);
    page0[ks + KS_OFF_WRAP_NONCE..ks + KS_OFF_WRAP_NONCE + 12].copy_from_slice(wrap_nonce);
    page0[ks + KS_OFF_WRAPPED_DEK..ks + KS_OFF_WRAPPED_DEK + 48].copy_from_slice(wrapped_dek);
    page0[ks + KS_OFF_KCV..ks + KS_OFF_KCV + 32].copy_from_slice(kcv);
}

/// Find the first empty keyslot in the region.
pub(super) fn find_empty_slot(page0: &[u8; PAGE_SIZE], keyslot_count: usize) -> Result<u16> {
    for i in 0..keyslot_count {
        let ks = KEYSLOT_REGION_OFFSET + i * KEYSLOT_SIZE;
        if page0[ks + KS_OFF_KIND] == KEYSLOT_KIND_EMPTY {
            return Ok(i as u16);
        }
    }
    Err(TosumuError::InvalidArgument(
        "keyslot region is full (all 8 slots occupied)",
    ))
}

// ── File header construction ──────────────────────────────────────────────────

pub(super) fn write_file_header(page0: &mut [u8; PAGE_SIZE], dek: &[u8; 32]) {
    // Magic (8 bytes) + 8 bytes padding.
    page0[OFF_MAGIC..OFF_MAGIC + 8].copy_from_slice(MAGIC.as_slice());
    write_u16(page0, OFF_FORMAT_VERSION, FORMAT_VERSION);
    write_u16(page0, OFF_PAGE_SIZE, PAGE_SIZE as u16);
    write_u16(page0, OFF_MIN_READER_VERSION, MIN_READER_VERSION);
    write_u16(page0, OFF_FLAGS, 0x0003u16); // bit0=reserved(1), bit1=has_keyslots
    write_u64(page0, OFF_PAGE_COUNT, 1); // just page 0 for now
    write_u64(page0, OFF_FREELIST_HEAD, 0);
    write_u64(page0, OFF_ROOT_PAGE, 0);
    write_u64(page0, OFF_WAL_CHECKPOINT_LSN, 0);
    write_u64(page0, OFF_DEK_ID, 1);
    // dek_kat: leave as zero for MVP+1 (TODO Stage 4)
    write_u16(page0, OFF_KEYSLOT_COUNT, 1);
    write_u16(page0, OFF_KEYSLOT_REGION_PAGES, 0); // keyslots embedded in page 0
                                                   // header_mac: leave as zero for MVP+1 (TODO Stage 4)

    // Sentinel keyslot at offset KEYSLOT_REGION_OFFSET.
    let ks = KEYSLOT_REGION_OFFSET;
    page0[ks + KS_OFF_KIND] = KEYSLOT_KIND_SENTINEL;
    page0[ks + KS_OFF_VERSION] = 1;
    // Sentinel stores the DEK as plaintext in the wrapped_dek field — it is NOT
    // wrapped. The field name is shared with encrypted protectors for layout
    // compatibility. See docs/Specifications/Tosumu Software Design Document.md §8.11: Sentinel provides no confidentiality.
    // Only the first 32 bytes are used (vs 48 for AEAD-wrapped DEKs).
    //
    // MVP+1 note: page 0 is trusted for magic/version/page-size checks only.
    // Data pages are fully authenticated (AEAD + page_version). Page 0 fields
    // such as page_count, freelist_head and root_page are not MAC'd in MVP+1;
    // the header MAC is added for encrypted databases only (Passphrase/Recovery slots).
    page0[ks + KS_OFF_WRAPPED_DEK..ks + KS_OFF_WRAPPED_DEK + 32].copy_from_slice(dek);
}
