use super::*;

#[test]
fn two_encrypted_dbs_keys_are_independent() {
    // DEK of DB A must not unlock DB B — cross-DB confusion attack.
    let path_a = temp_path("xdb_a");
    let path_b = temp_path("xdb_b");
    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);

    // Same passphrase, different DBs → different DEKs → can't cross-open.
    PageStore::create_encrypted(&path_a, "shared-pass").unwrap();
    PageStore::create_encrypted(&path_b, "shared-pass").unwrap();

    // Read the wrapped DEK from A and patch it into B's page-0.
    use crate::format::{KEYSLOT_REGION_OFFSET, KS_OFF_WRAPPED_DEK, OFF_HEADER_MAC};
    let mut raw_b = std::fs::read(&path_b).unwrap();
    let raw_a = std::fs::read(&path_a).unwrap();
    let wdek_off = KEYSLOT_REGION_OFFSET + KS_OFF_WRAPPED_DEK;
    raw_b[wdek_off..wdek_off + 48].copy_from_slice(&raw_a[wdek_off..wdek_off + 48]);
    // Also copy the MAC from A so the MAC check passes.
    raw_b[OFF_HEADER_MAC..OFF_HEADER_MAC + 32]
        .copy_from_slice(&raw_a[OFF_HEADER_MAC..OFF_HEADER_MAC + 32]);
    std::fs::write(&path_b, &raw_b).unwrap();

    // Opening B with the shared passphrase must fail — MAC was computed over A's keyslot data.
    let err = PageStore::open_with_passphrase(&path_b, "shared-pass")
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::WrongKey | crate::error::TosumuError::AuthFailed { .. }
        ),
        "cross-DB splice must be rejected, got {err:?}"
    );

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}

#[test]
fn encrypted_db_with_1000_keys_survives_reopen() {
    let path = temp_path("enc_1000");
    let _ = std::fs::remove_file(&path);

    let n = 1000u32;
    {
        let mut store = PageStore::create_encrypted(&path, "stress-pass").unwrap();
        for i in 0..n {
            store
                .put(
                    format!("key-{i:06}").as_bytes(),
                    format!("val-{i}").as_bytes(),
                )
                .unwrap();
        }
    }
    let store = PageStore::open_with_passphrase(&path, "stress-pass").unwrap();
    for i in 0..n {
        let expected = format!("val-{i}").into_bytes();
        assert_eq!(
            store.get(format!("key-{i:06}").as_bytes()).unwrap(),
            Some(expected),
            "key {i} missing after reopen"
        );
    }

    let _ = std::fs::remove_file(&path);
}

// ── Adversarial: crash simulation ─────────────────────────────────────────

/// Simulates crash-before-write during rekey: snapshot page0, run rekey, restore snapshot.
/// Old passphrase must still work; new passphrase must not.
#[test]
fn crash_before_rekey_write_old_passphrase_recovers() {
    let path = temp_path("crash_before_rekey");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "old").unwrap();

    // Snapshot the page0 before rekey.
    let snapshot = std::fs::read(&path).unwrap();

    // Run rekey successfully.
    PageStore::rekey_kek(&path, 0, "old", "new").unwrap();

    // Restore the pre-rekey snapshot (simulates crash before write_page0 succeeded).
    std::fs::write(&path, &snapshot).unwrap();

    // Old passphrase must still work.
    PageStore::open_with_passphrase(&path, "old").unwrap();
    // New passphrase must not.
    let err = PageStore::open_with_passphrase(&path, "new").err().unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "got {err:?}"
    );

    let _ = std::fs::remove_file(&path);
}

/// Simulates torn page0 write during rekey (first 2048 bytes from new state,
/// remaining 2048 from old state). Must be rejected cleanly — no panic, no wrong-key accept.
#[test]
fn crash_mid_rekey_torn_page_rejected() {
    let path = temp_path("crash_mid_rekey");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "old").unwrap();
    let old_bytes = std::fs::read(&path).unwrap();

    // Run rekey to get the "new" page0.
    PageStore::rekey_kek(&path, 0, "old", "new").unwrap();
    let new_bytes = std::fs::read(&path).unwrap();

    // Simulate a torn write: keyslots from the new state but MAC from the old state.
    // This represents a crash after slot 0 was updated on disk but before the MAC
    // field was written — the most dangerous torn-write scenario.
    use crate::format::OFF_HEADER_MAC;
    let mut torn = new_bytes.clone();
    torn[OFF_HEADER_MAC..OFF_HEADER_MAC + 32]
        .copy_from_slice(&old_bytes[OFF_HEADER_MAC..OFF_HEADER_MAC + 32]);
    std::fs::write(&path, &torn).unwrap();

    // old pass: AEAD unwrap of new slot with old KEK fails → WrongKey.
    let err = PageStore::open_with_passphrase(&path, "old").err().unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "old pass on torn page should be WrongKey, got {err:?}"
    );
    // new pass: DEK unwraps fine from the new slot, but old MAC doesn't match → AuthFailed.
    let err2 = PageStore::open_with_passphrase(&path, "new").err().unwrap();
    assert!(
        matches!(err2, crate::error::TosumuError::AuthFailed { .. }),
        "new pass on torn page should be AuthFailed (stale MAC), got {err2:?}"
    );

    let _ = std::fs::remove_file(&path);
}

/// Simulates crash-before-write during add_passphrase_protector.
/// Original passphrase must still work; new slot must not appear.
#[test]
fn crash_before_add_protector_write_original_still_works() {
    let path = temp_path("crash_before_add");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "main").unwrap();
    let snapshot = std::fs::read(&path).unwrap();

    PageStore::add_passphrase_protector(&path, "main", "extra").unwrap();

    // Restore pre-add snapshot.
    std::fs::write(&path, &snapshot).unwrap();

    // Original works, extra does not.
    PageStore::open_with_passphrase(&path, "main").unwrap();
    let err = PageStore::open_with_passphrase(&path, "extra")
        .err()
        .unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "got {err:?}"
    );

    // Only 1 slot listed.
    let slots = PageStore::list_keyslots(&path).unwrap();
    assert_eq!(slots.len(), 1);

    let _ = std::fs::remove_file(&path);
}

/// Recovery key must survive a crash-before-write of a second add_passphrase operation.
#[test]
fn crash_before_second_add_recovery_key_still_works() {
    let path = temp_path("crash_before_second");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "main").unwrap();
    let recovery = PageStore::add_recovery_key_protector(&path, "main").unwrap();
    let snapshot = std::fs::read(&path).unwrap();

    // Attempt to add another slot — then crash.
    PageStore::add_passphrase_protector(&path, "main", "extra").unwrap();
    std::fs::write(&path, &snapshot).unwrap();

    // Recovery key still works (was in the snapshot).
    PageStore::open_with_recovery_key(&path, &recovery).unwrap();
    // 'extra' never made it.
    assert!(PageStore::open_with_passphrase(&path, "extra").is_err());

    let _ = std::fs::remove_file(&path);
}

// ── Adversarial: slot reuse and stale AAD ─────────────────────────────────

/// Add A (slot 0), add B (slot 1), remove A.
/// Add C — should land in slot 0.
/// B still works, C works via slot 0 (new AAD), old A wrapped blob rejected.
#[test]
fn slot_reuse_aad_binding_rejects_old_blob() {
    use crate::format::{KEYSLOT_REGION_OFFSET, KS_OFF_WRAPPED_DEK};
    use std::fs;

    let path = temp_path("slot_reuse_aad");
    let _ = fs::remove_file(&path);

    // Create: slot 0 = A.
    PageStore::create_encrypted(&path, "pass-a").unwrap();
    // Snapshot the wrapped DEK from slot 0 while A occupies it.
    let bytes_with_a = fs::read(&path).unwrap();
    let a_wrapped_blob: [u8; 48] = bytes_with_a[KEYSLOT_REGION_OFFSET + KS_OFF_WRAPPED_DEK
        ..KEYSLOT_REGION_OFFSET + KS_OFF_WRAPPED_DEK + 48]
        .try_into()
        .unwrap();

    // Add B → slot 1.
    let slot_b = PageStore::add_passphrase_protector(&path, "pass-a", "pass-b").unwrap();
    assert_eq!(slot_b, 1);

    // Remove A → slot 0 emptied.
    PageStore::remove_keyslot(&path, "pass-b", 0).unwrap();

    // Add C → should reuse slot 0.
    let slot_c = PageStore::add_passphrase_protector(&path, "pass-b", "pass-c").unwrap();
    assert_eq!(slot_c, 0, "expected C to reuse slot 0");

    // Normal paths work.
    PageStore::open_with_passphrase(&path, "pass-b").unwrap();
    PageStore::open_with_passphrase(&path, "pass-c").unwrap();
    // A is gone.
    assert!(PageStore::open_with_passphrase(&path, "pass-a").is_err());

    // Now splice A's old wrapped DEK blob back into slot 0, keeping C's other fields.
    // The AEAD tag was bound to A's Argon2id KEK; the KCV will now mismatch.
    let mut raw = fs::read(&path).unwrap();
    raw[KEYSLOT_REGION_OFFSET + KS_OFF_WRAPPED_DEK
        ..KEYSLOT_REGION_OFFSET + KS_OFF_WRAPPED_DEK + 48]
        .copy_from_slice(&a_wrapped_blob);
    fs::write(&path, &raw).unwrap();

    // C must now fail (KCV or unwrap mismatch), B may still work (different slot).
    // The interesting case: does A's blob accidentally unlock with A's passphrase?
    // It must not — the MAC is now broken.
    let err = PageStore::open_with_passphrase(&path, "pass-a")
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::WrongKey | crate::error::TosumuError::AuthFailed { .. }
        ),
        "stale blob in reused slot must not unlock: {err:?}"
    );

    let _ = fs::remove_file(&path);
}

/// Add and remove slots many times; only the currently-valid passphrase must work.
#[test]
fn add_remove_cycle_only_current_pass_works() {
    let path = temp_path("cycle_slots");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "p0").unwrap();

    for round in 0..5u32 {
        let new_pass = format!("round-{round}");
        PageStore::add_passphrase_protector(&path, "p0", &new_pass).unwrap();
        // p0 and new_pass both work now.
        PageStore::open_with_passphrase(&path, "p0").unwrap();
        PageStore::open_with_passphrase(&path, &new_pass).unwrap();

        // Remove the round pass (slot != 0, since p0 is in slot 0).
        let slots = PageStore::list_keyslots(&path).unwrap();
        let round_slot = slots
            .iter()
            .find(|&&(idx, kind)| idx != 0 && kind == crate::format::KEYSLOT_KIND_PASSPHRASE)
            .map(|&(idx, _)| idx)
            .expect("round slot not found");
        PageStore::remove_keyslot(&path, "p0", round_slot).unwrap();

        // new_pass must no longer work.
        assert!(
            PageStore::open_with_passphrase(&path, &new_pass).is_err(),
            "round-{round} pass still works after removal"
        );
    }
    // p0 still works after all the churn.
    PageStore::open_with_passphrase(&path, "p0").unwrap();

    let _ = std::fs::remove_file(&path);
}

// ── Adversarial: slot order invariance ────────────────────────────────────

/// Passphrase in slot 1 (not 0) can still unlock.
#[test]
fn passphrase_in_non_zero_slot_unlocks() {
    let path = temp_path("non_zero_slot");
    let _ = std::fs::remove_file(&path);

    // slot 0 = "p0", slot 1 = "p1"
    PageStore::create_encrypted(&path, "p0").unwrap();
    PageStore::add_passphrase_protector(&path, "p0", "p1").unwrap();
    // Remove slot 0, so slot 1 is the only protector.
    PageStore::remove_keyslot(&path, "p1", 0).unwrap();

    // Only p1 should work now, accessed via slot 1.
    let store = PageStore::open_with_passphrase(&path, "p1").unwrap();
    drop(store);
    assert!(PageStore::open_with_passphrase(&path, "p0").is_err());

    let _ = std::fs::remove_file(&path);
}

/// After removing slot 0, re-adding produces slot 0 again; data is intact.
#[test]
fn slot_zero_reuse_data_intact() {
    let path = temp_path("slot0_reuse");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create_encrypted(&path, "p0").unwrap();
        store.put(b"sentinel", b"value").unwrap();
    }
    PageStore::add_passphrase_protector(&path, "p0", "p1").unwrap();
    PageStore::remove_keyslot(&path, "p1", 0).unwrap();
    // Add p2 — should reuse slot 0.
    let slot = PageStore::add_passphrase_protector(&path, "p1", "p2").unwrap();
    assert_eq!(slot, 0, "expected slot 0 reuse");

    // Both p1 and p2 work, data intact.
    let s1 = PageStore::open_with_passphrase(&path, "p1").unwrap();
    assert_eq!(s1.get(b"sentinel").unwrap(), Some(b"value".to_vec()));
    drop(s1);
    let s2 = PageStore::open_with_passphrase(&path, "p2").unwrap();
    assert_eq!(s2.get(b"sentinel").unwrap(), Some(b"value".to_vec()));

    let _ = std::fs::remove_file(&path);
}

// ── Adversarial: targeted header field corruption ─────────────────────────

/// Flip the kind byte of keyslot 0. Must be rejected (MAC covers keyslot region).
#[test]
fn corrupt_keyslot_kind_byte_rejected() {
    use crate::format::{KEYSLOT_REGION_OFFSET, KS_OFF_KIND};

    let path = temp_path("corrupt_kind");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "pass").unwrap();

    let mut raw = std::fs::read(&path).unwrap();
    raw[KEYSLOT_REGION_OFFSET + KS_OFF_KIND] ^= 0x01;
    std::fs::write(&path, &raw).unwrap();

    let err = PageStore::open_with_passphrase(&path, "pass")
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::AuthFailed { .. } | crate::error::TosumuError::WrongKey
        ),
        "kind byte flip must be caught by MAC: {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

/// Corrupt the keyslot_count field in the header (OFF_KEYSLOT_COUNT).
/// System must reject (MAC covers that field) or handle gracefully without panic.
#[test]
fn corrupt_keyslot_count_field_rejected_or_graceful() {
    use crate::format::OFF_KEYSLOT_COUNT;

    let path = temp_path("corrupt_ks_count");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "pass").unwrap();

    let mut raw = std::fs::read(&path).unwrap();
    // Set count to 255 (way above MAX_KEYSLOTS=8).
    raw[OFF_KEYSLOT_COUNT] = 0xFF;
    raw[OFF_KEYSLOT_COUNT + 1] = 0xFF;
    std::fs::write(&path, &raw).unwrap();

    // Must not panic; either reject via MAC or clamp.
    let result = PageStore::open_with_passphrase(&path, "pass");
    // If it succeeds, the clamping worked and data is accessible.
    // If it fails, the MAC caught the tampered count field.
    if let Err(e) = &result {
        assert!(
            matches!(
                e,
                crate::error::TosumuError::AuthFailed { .. }
                    | crate::error::TosumuError::WrongKey
                    | crate::error::TosumuError::Corrupt { .. }
            ),
            "unexpected error: {e:?}"
        );
    }
    let _ = std::fs::remove_file(&path);
}

/// Flip the nonce bytes inside the keyslot (KS_OFF_WRAP_NONCE).
/// AEAD unwrap must fail → WrongKey.
#[test]
fn corrupt_wrap_nonce_causes_aead_failure() {
    use crate::format::{KEYSLOT_REGION_OFFSET, KS_OFF_WRAP_NONCE};

    let path = temp_path("corrupt_nonce");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "pass").unwrap();

    let mut raw = std::fs::read(&path).unwrap();
    raw[KEYSLOT_REGION_OFFSET + KS_OFF_WRAP_NONCE] ^= 0xFF;
    std::fs::write(&path, &raw).unwrap();

    let err = PageStore::open_with_passphrase(&path, "pass")
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::WrongKey | crate::error::TosumuError::AuthFailed { .. }
        ),
        "nonce flip must fail: {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

/// Flip single bits across every byte of the full keyslot region.
/// Every single-bit flip must be rejected (not panic, not silently succeed).
#[test]
#[ignore = "slow: 256 bytes × 8 bits = 2048 Argon2id calls; run with --include-ignored"]
fn single_bit_flip_in_keyslot_region_always_rejected() {
    use crate::format::{KEYSLOT_REGION_OFFSET, KEYSLOT_SIZE};

    let path = temp_path("bitflip_sweep");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "pass").unwrap();
    let original = std::fs::read(&path).unwrap();

    // Only sweep the single occupied slot (256 bytes) to keep test fast.
    let slot_region = KEYSLOT_REGION_OFFSET..KEYSLOT_REGION_OFFSET + KEYSLOT_SIZE;

    for byte_off in slot_region {
        for bit in 0u8..8 {
            let mut raw = original.clone();
            raw[byte_off] ^= 1 << bit;
            std::fs::write(&path, &raw).unwrap();

            let result = PageStore::open_with_passphrase(&path, "pass");
            assert!(
                result.is_err(),
                "bit flip at byte {byte_off} bit {bit} was silently accepted"
            );
            let e = result.err().unwrap();
            assert!(
                    !matches!(e, crate::error::TosumuError::Corrupt { .. }),
                    "bit flip at byte {byte_off} bit {bit} caused Corrupt (should be AuthFailed/WrongKey): {e:?}"
                );
        }
    }
    let _ = std::fs::remove_file(&path);
}

// ── Adversarial: cross-DB full slot splice ────────────────────────────────

/// Copy *entire* slot 0 from DB A into slot 0 of DB B (same passphrase).
/// Must fail because DEK_ID differs → AEAD AAD mismatch on unwrap.
#[test]
fn cross_db_full_slot_splice_rejected() {
    use crate::format::{KEYSLOT_REGION_OFFSET, KEYSLOT_SIZE};

    let path_a = temp_path("xdb_full_a");
    let path_b = temp_path("xdb_full_b");
    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);

    PageStore::create_encrypted(&path_a, "shared").unwrap();
    PageStore::create_encrypted(&path_b, "shared").unwrap();

    // Copy entire slot 0 from A into B.
    let raw_a = std::fs::read(&path_a).unwrap();
    let mut raw_b = std::fs::read(&path_b).unwrap();
    let slot_start = KEYSLOT_REGION_OFFSET;
    raw_b[slot_start..slot_start + KEYSLOT_SIZE]
        .copy_from_slice(&raw_a[slot_start..slot_start + KEYSLOT_SIZE]);
    // Also copy A's MAC so the header MAC check passes.
    use crate::format::OFF_HEADER_MAC;
    raw_b[OFF_HEADER_MAC..OFF_HEADER_MAC + 32]
        .copy_from_slice(&raw_a[OFF_HEADER_MAC..OFF_HEADER_MAC + 32]);
    std::fs::write(&path_b, &raw_b).unwrap();

    // Opening B with shared passphrase must fail — DEK_ID in B's header
    // differs from the DEK_ID baked into A's AEAD ciphertext.
    let err = PageStore::open_with_passphrase(&path_b, "shared")
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::WrongKey | crate::error::TosumuError::AuthFailed { .. }
        ),
        "full slot splice from different DB must be rejected: {err:?}"
    );

    let _ = std::fs::remove_file(&path_a);
    let _ = std::fs::remove_file(&path_b);
}

// ── Adversarial: header snapshot rollback ─────────────────────────────────

/// Snapshot header before adding a recovery key, restore it after.
/// Recovery key must no longer work (not in snapshot), original pass still works.
#[test]
fn snapshot_rollback_removes_recovery_key() {
    let path = temp_path("rollback_rk");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "main").unwrap();
    let snapshot = std::fs::read(&path).unwrap();

    let recovery = PageStore::add_recovery_key_protector(&path, "main").unwrap();

    // Verify recovery works before rollback.
    PageStore::open_with_recovery_key(&path, &recovery).unwrap();

    // Roll back to pre-add snapshot.
    std::fs::write(&path, &snapshot).unwrap();

    // Recovery key is gone.
    let err = PageStore::open_with_recovery_key(&path, &recovery)
        .err()
        .unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "got {err:?}"
    );

    // Main passphrase still works (was in snapshot).
    PageStore::open_with_passphrase(&path, "main").unwrap();

    let _ = std::fs::remove_file(&path);
}

/// Snapshot after rekey, restore before rekey. Old passphrase should work again
/// because the file is literally back to the old state (not a hybrid).
#[test]
fn snapshot_pre_rekey_restores_old_passphrase() {
    let path = temp_path("rollback_rekey");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "v1").unwrap();
    let snapshot = std::fs::read(&path).unwrap();

    PageStore::rekey_kek(&path, 0, "v1", "v2").unwrap();
    // v2 works now.
    PageStore::open_with_passphrase(&path, "v2").unwrap();

    // Roll back to pre-rekey.
    std::fs::write(&path, &snapshot).unwrap();

    // v1 works again, v2 does not.
    PageStore::open_with_passphrase(&path, "v1").unwrap();
    assert!(PageStore::open_with_passphrase(&path, "v2").is_err());

    let _ = std::fs::remove_file(&path);
}

/// Hybrid: header from new state, keyslot data from old state (torn cross-operation).
#[test]
fn hybrid_header_new_slots_old_rejected() {
    use crate::format::{KEYSLOT_REGION_OFFSET, KEYSLOT_SIZE, MAX_KEYSLOTS};

    let path = temp_path("hybrid_torn");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "p").unwrap();
    let old_bytes = std::fs::read(&path).unwrap();

    PageStore::rekey_kek(&path, 0, "p", "p2").unwrap();
    let new_bytes = std::fs::read(&path).unwrap();

    // Hybrid: take new fixed-header fields (page_count etc.) but keep old keyslot bytes.
    let mut hybrid = new_bytes.clone();
    hybrid[KEYSLOT_REGION_OFFSET..KEYSLOT_REGION_OFFSET + KEYSLOT_SIZE * MAX_KEYSLOTS]
        .copy_from_slice(
            &old_bytes[KEYSLOT_REGION_OFFSET..KEYSLOT_REGION_OFFSET + KEYSLOT_SIZE * MAX_KEYSLOTS],
        );
    // Keep new MAC so it's technically "authenticated" — but MAC was computed over new slots.
    // This means the MAC will not match the old slot data → should be caught.
    std::fs::write(&path, &hybrid).unwrap();

    let err = PageStore::open_with_passphrase(&path, "p").err().unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::AuthFailed { .. } | crate::error::TosumuError::WrongKey
        ),
        "hybrid header+keyslot must be rejected by MAC: {err:?}"
    );
    let err2 = PageStore::open_with_passphrase(&path, "p2").err().unwrap();
    assert!(
        matches!(
            err2,
            crate::error::TosumuError::AuthFailed { .. } | crate::error::TosumuError::WrongKey
        ),
        "hybrid must reject new pass too: {err2:?}"
    );

    let _ = std::fs::remove_file(&path);
}

// ── Adversarial: user stupidity / failed ops leave file unchanged ─────────

/// Wrong passphrase many times → file is bitwise identical each time.
#[test]
fn failed_open_does_not_mutate_file() {
    let path = temp_path("no_mutate_open");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "real").unwrap();
    let original = std::fs::read(&path).unwrap();

    for _ in 0..5 {
        let _ = PageStore::open_with_passphrase(&path, "wrong");
    }

    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "read-only open mutated the file"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
#[allow(clippy::permissions_set_readonly_false)]
fn open_readonly_works_on_readonly_file() {
    let path = temp_path("readonly_file_open");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create(&path).unwrap();
        store.put(b"k", b"v").unwrap();
    }

    let mut perms = std::fs::metadata(&path).unwrap().permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms).unwrap();

    let store = PageStore::open_readonly(&path).unwrap();
    assert_eq!(store.get(b"k").unwrap(), Some(b"v".to_vec()));
    assert_eq!(store.scan().unwrap(), vec![(b"k".to_vec(), b"v".to_vec())]);
    assert_eq!(store.stat().unwrap().page_count, 2);

    #[cfg(windows)]
    {
        let mut perms = std::fs::metadata(&path).unwrap().permissions();
        perms.set_readonly(false);
        std::fs::set_permissions(&path, perms).unwrap();
    }
    let _ = std::fs::remove_file(&path);
}

/// Failed rekey (wrong old passphrase) → file is bitwise identical.
#[test]
fn failed_rekey_does_not_mutate_file() {
    let path = temp_path("no_mutate_rekey");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "real").unwrap();
    let original = std::fs::read(&path).unwrap();

    for _ in 0..3 {
        let _ = PageStore::rekey_kek(&path, 0, "wrong", "new");
    }

    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "failed rekey mutated the file"
    );
    PageStore::open_with_passphrase(&path, "real").unwrap();
    let _ = std::fs::remove_file(&path);
}

/// Failed add_passphrase_protector (wrong unlock passphrase) → file unchanged.
#[test]
fn failed_add_protector_does_not_mutate_file() {
    let path = temp_path("no_mutate_add");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "real").unwrap();
    let original = std::fs::read(&path).unwrap();

    let _ = PageStore::add_passphrase_protector(&path, "wrong", "new");

    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "failed add_protector mutated the file"
    );
    let _ = std::fs::remove_file(&path);
}

/// Wrong recovery key → file unchanged, real passphrase still works.
#[test]
fn failed_recovery_open_does_not_mutate_file() {
    let path = temp_path("no_mutate_recovery");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "real").unwrap();
    let original = std::fs::read(&path).unwrap();

    for _ in 0..3 {
        let _ = PageStore::open_with_recovery_key(&path, "AAAAAAAA-BBBBBBBB-CCCCCCCC-DDDDDDDD");
    }

    assert_eq!(
        std::fs::read(&path).unwrap(),
        original,
        "failed recovery open mutated the file"
    );
    PageStore::open_with_passphrase(&path, "real").unwrap();
    let _ = std::fs::remove_file(&path);
}

// ── Adversarial: invariant check after every key-management op ────────────

/// After each of: create, add, remove, rekey, add recovery — verify:
///   1. list_keyslots returns sane slot count
///   2. db closes and reopens successfully
///   3. put/get round-trips correctly through each reopen
#[test]
fn invariants_hold_after_every_key_management_op() {
    let path = temp_path("invariants_km");
    let _ = std::fs::remove_file(&path);

    // Step 1: create.
    {
        let mut s = PageStore::create_encrypted(&path, "p0").unwrap();
        s.put(b"marker", b"created").unwrap();
    }
    let slots = PageStore::list_keyslots(&path).unwrap();
    assert_eq!(slots.len(), 1, "after create");
    assert_eq!(
        PageStore::open_with_passphrase(&path, "p0")
            .unwrap()
            .get(b"marker")
            .unwrap(),
        Some(b"created".to_vec())
    );

    // Step 2: add passphrase.
    let slot1 = PageStore::add_passphrase_protector(&path, "p0", "p1").unwrap();
    let slots = PageStore::list_keyslots(&path).unwrap();
    assert_eq!(slots.len(), 2, "after add passphrase");
    PageStore::open_with_passphrase(&path, "p0").unwrap();
    PageStore::open_with_passphrase(&path, "p1").unwrap();

    // Step 3: add recovery key.
    let recovery = PageStore::add_recovery_key_protector(&path, "p0").unwrap();
    let slots = PageStore::list_keyslots(&path).unwrap();
    assert_eq!(slots.len(), 3, "after add recovery key");
    PageStore::open_with_recovery_key(&path, &recovery).unwrap();

    // Step 4: rekey slot 1.
    PageStore::rekey_kek(&path, slot1, "p1", "p1-new").unwrap();
    let slots = PageStore::list_keyslots(&path).unwrap();
    assert_eq!(slots.len(), 3, "after rekey (count unchanged)");
    assert!(PageStore::open_with_passphrase(&path, "p1").is_err());
    PageStore::open_with_passphrase(&path, "p1-new").unwrap();

    // Step 5: remove slot 1 (p1-new).
    PageStore::remove_keyslot(&path, "p0", slot1).unwrap();
    let slots = PageStore::list_keyslots(&path).unwrap();
    assert_eq!(slots.len(), 2, "after remove slot 1");
    assert!(PageStore::open_with_passphrase(&path, "p1-new").is_err());
    PageStore::open_with_passphrase(&path, "p0").unwrap();
    PageStore::open_with_recovery_key(&path, &recovery).unwrap();

    // Step 6: data is still intact.
    let s = PageStore::open_with_passphrase(&path, "p0").unwrap();
    assert_eq!(s.get(b"marker").unwrap(), Some(b"created".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn recovery_key_only_database_can_still_manage_protectors() {
    let path = temp_path("recovery_only_manage");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create_encrypted(&path, "main").unwrap();
        store.put(b"marker", b"ok").unwrap();
    }

    let recovery = PageStore::add_recovery_key_protector(&path, "main").unwrap();
    PageStore::remove_keyslot(&path, "main", 0).unwrap();

    let slot =
        PageStore::add_passphrase_protector_with_recovery_key(&path, &recovery, "p1").unwrap();
    PageStore::open_with_passphrase(&path, "p1").unwrap();

    let recovery2 =
        PageStore::add_recovery_key_protector_with_recovery_key(&path, &recovery).unwrap();
    PageStore::open_with_recovery_key(&path, &recovery2).unwrap();

    PageStore::rekey_kek_with_recovery_key(&path, slot, &recovery, "p1-new").unwrap();
    assert!(PageStore::open_with_passphrase(&path, "p1").is_err());
    PageStore::open_with_passphrase(&path, "p1-new").unwrap();

    PageStore::remove_keyslot_with_recovery_key(&path, &recovery, slot).unwrap();
    assert!(PageStore::open_with_passphrase(&path, "p1-new").is_err());
    PageStore::open_with_recovery_key(&path, &recovery).unwrap();
    PageStore::open_with_recovery_key(&path, &recovery2).unwrap();

    let slots = PageStore::list_keyslots(&path).unwrap();
    assert_eq!(slots.len(), 2);

    let store = PageStore::open_with_recovery_key(&path, &recovery).unwrap();
    assert_eq!(store.get(b"marker").unwrap(), Some(b"ok".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn keyfile_only_database_can_still_manage_protectors() {
    let path = temp_path("keyfile_only_manage");
    let keyfile = temp_path("keyfile_only_manage.bin");
    let keyfile2 = temp_path("keyfile_only_manage_2.bin");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&keyfile);
    let _ = std::fs::remove_file(&keyfile2);

    std::fs::write(&keyfile, [0x11u8; 32]).unwrap();
    std::fs::write(&keyfile2, [0x22u8; 32]).unwrap();

    {
        let mut store = PageStore::create_encrypted(&path, "main").unwrap();
        store.put(b"marker", b"ok").unwrap();
    }

    let key_slot = PageStore::add_keyfile_protector(&path, "main", &keyfile).unwrap();
    PageStore::remove_keyslot(&path, "main", 0).unwrap();

    let slot = PageStore::add_passphrase_protector_with_keyfile(&path, &keyfile, "p1").unwrap();
    PageStore::open_with_passphrase(&path, "p1").unwrap();

    PageStore::add_recovery_key_protector_with_keyfile_and_secret(
        &path,
        &keyfile,
        "AAAA-BBBB-CCCC-DDDD-EEEE-FFFF-GGGG-HHHH",
    )
    .unwrap();
    PageStore::open_with_recovery_key(&path, "AAAA-BBBB-CCCC-DDDD-EEEE-FFFF-GGGG-HHHH").unwrap();

    let slot2 = PageStore::add_keyfile_protector_with_keyfile(&path, &keyfile, &keyfile2).unwrap();
    PageStore::open_with_keyfile(&path, &keyfile2).unwrap();

    PageStore::rekey_kek_with_keyfile(&path, slot, &keyfile, "p1-new").unwrap();
    assert!(PageStore::open_with_passphrase(&path, "p1").is_err());
    PageStore::open_with_passphrase(&path, "p1-new").unwrap();

    PageStore::remove_keyslot_with_keyfile(&path, &keyfile, slot).unwrap();
    PageStore::remove_keyslot_with_keyfile(&path, &keyfile, key_slot).unwrap();
    assert!(PageStore::open_with_keyfile(&path, &keyfile).is_err());
    PageStore::open_with_keyfile(&path, &keyfile2).unwrap();

    let slots = PageStore::list_keyslots(&path).unwrap();
    assert!(slots.iter().any(|&(idx, _)| idx == slot2));

    let store = PageStore::open_with_keyfile(&path, &keyfile2).unwrap();
    assert_eq!(store.get(b"marker").unwrap(), Some(b"ok".to_vec()));

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&keyfile);
    let _ = std::fs::remove_file(&keyfile2);
}
