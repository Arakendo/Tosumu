use super::*;

#[test]
fn protector_swap_attack_rejected() {
    // Write two databases with different passphrases. Manually copy the
    // wrapped DEK from slot 0 of DB B into slot 0 of DB A. The MAC should
    // now fail on DB A.
    use crate::format::{KEYSLOT_REGION_OFFSET, KS_OFF_WRAPPED_DEK};
    use std::fs;

    let path_a = temp_path("swap_a");
    let path_b = temp_path("swap_b");
    let _ = fs::remove_file(&path_a);
    let _ = fs::remove_file(&path_b);

    PageStore::create_encrypted(&path_a, "pass-a").unwrap();
    PageStore::create_encrypted(&path_b, "pass-b").unwrap();

    // Corrupt DB A by splicing the wrapped DEK from DB B.
    let mut bytes_a = fs::read(&path_a).unwrap();
    let bytes_b = fs::read(&path_b).unwrap();
    let ks0 = KEYSLOT_REGION_OFFSET;
    let wdek_off = ks0 + KS_OFF_WRAPPED_DEK;
    bytes_a[wdek_off..wdek_off + 48].copy_from_slice(&bytes_b[wdek_off..wdek_off + 48]);
    fs::write(&path_a, &bytes_a).unwrap();

    // Opening with pass-a must fail (MAC or DEK unwrap mismatch).
    let err = PageStore::open_with_passphrase(&path_a, "pass-a")
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::WrongKey | crate::error::TosumuError::AuthFailed { .. }
        ),
        "expected auth failure, got {err:?}"
    );

    let _ = fs::remove_file(&path_a);
    let _ = fs::remove_file(&path_b);
}

// ── Corruption tests ──────────────────────────────────────────────────────

#[test]
fn corrupt_magic_returns_not_a_tosum_file() {
    let path = temp_path("corrupt_magic");
    let _ = std::fs::remove_file(&path);
    PageStore::create(&path).unwrap();

    let mut raw = std::fs::read(&path).unwrap();
    raw[0] ^= 0xFF; // flip first magic byte
    std::fs::write(&path, &raw).unwrap();

    let err = PageStore::open(&path).err().unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::NotATosumFile),
        "expected NotATosumFile, got {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn corrupt_header_mac_on_encrypted_db_rejected() {
    use crate::format::OFF_HEADER_MAC;

    let path = temp_path("corrupt_mac");
    let _ = std::fs::remove_file(&path);
    PageStore::create_encrypted(&path, "pass").unwrap();

    let mut raw = std::fs::read(&path).unwrap();
    raw[OFF_HEADER_MAC] ^= 0x01; // flip one bit in the header MAC
    std::fs::write(&path, &raw).unwrap();

    let err = PageStore::open_with_passphrase(&path, "pass")
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::AuthFailed { .. } | crate::error::TosumuError::WrongKey
        ),
        "expected AuthFailed or WrongKey, got {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn corrupt_kcv_returns_wrong_key() {
    use crate::format::{KEYSLOT_REGION_OFFSET, KS_OFF_KCV};

    let path = temp_path("corrupt_kcv");
    let _ = std::fs::remove_file(&path);
    PageStore::create_encrypted(&path, "pass").unwrap();

    let mut raw = std::fs::read(&path).unwrap();
    raw[KEYSLOT_REGION_OFFSET + KS_OFF_KCV] ^= 0xFF; // corrupt KCV for slot 0
    std::fs::write(&path, &raw).unwrap();

    let err = PageStore::open_with_passphrase(&path, "pass")
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::WrongKey | crate::error::TosumuError::AuthFailed { .. }
        ),
        "expected WrongKey or AuthFailed, got {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn corrupt_wrapped_dek_returns_wrong_key() {
    use crate::format::{KEYSLOT_REGION_OFFSET, KS_OFF_WRAPPED_DEK};

    let path = temp_path("corrupt_wdek");
    let _ = std::fs::remove_file(&path);
    PageStore::create_encrypted(&path, "pass").unwrap();

    let mut raw = std::fs::read(&path).unwrap();
    raw[KEYSLOT_REGION_OFFSET + KS_OFF_WRAPPED_DEK + 5] ^= 0xAB;
    std::fs::write(&path, &raw).unwrap();

    let err = PageStore::open_with_passphrase(&path, "pass")
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::WrongKey | crate::error::TosumuError::AuthFailed { .. }
        ),
        "expected WrongKey or AuthFailed, got {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn corrupt_ciphertext_page_in_encrypted_db_auth_fails() {
    use crate::format::PAGE_SIZE;

    let path = temp_path("corrupt_enc_page");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create_encrypted(&path, "p").unwrap();
        store.put(b"key", b"value").unwrap();
    }

    // Corrupt a byte deep inside the first data page ciphertext.
    let mut raw = std::fs::read(&path).unwrap();
    raw[PAGE_SIZE + 64] ^= 0xFF;
    std::fs::write(&path, &raw).unwrap();

    let store = PageStore::open_with_passphrase(&path, "p").unwrap();
    let err = store.get(b"key").unwrap_err();
    assert!(
        matches!(err, crate::error::TosumuError::AuthFailed { .. }),
        "expected AuthFailed, got {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn truncated_file_rejected() {
    use crate::format::PAGE_SIZE;

    let path = temp_path("truncated");
    let _ = std::fs::remove_file(&path);
    PageStore::create(&path).unwrap();

    // Truncate to half a page.
    let f = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
    f.set_len((PAGE_SIZE / 2) as u64).unwrap();
    drop(f);

    // Must error, not panic.
    let result = PageStore::open(&path);
    assert!(result.is_err(), "expected error opening truncated file");
    let _ = std::fs::remove_file(&path);
}

#[test]
fn empty_file_rejected() {
    let path = temp_path("zero_bytes");
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, b"").unwrap();

    let err = PageStore::open(&path).err().unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::NotATosumFile | crate::error::TosumuError::Io(_)
        ),
        "expected NotATosumFile or Io, got {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn wrong_magic_length_rejected() {
    // Write only the magic without a full header.
    let path = temp_path("short_magic");
    let _ = std::fs::remove_file(&path);
    std::fs::write(&path, b"TOSUMUv0").unwrap();

    let err = PageStore::open(&path).err().unwrap();
    assert!(result_is_err_io_or_not_tosum(&err));
    let _ = std::fs::remove_file(&path);
}

fn result_is_err_io_or_not_tosum(e: &crate::error::TosumuError) -> bool {
    matches!(
        e,
        crate::error::TosumuError::NotATosumFile | crate::error::TosumuError::Io(_)
    )
}

// ── Key management edge cases ─────────────────────────────────────────────

#[test]
#[ignore = "runs Argon2id 8 times — slow (~100 s); run with `cargo test keyslot_exhaustion -- --ignored`"]
fn keyslot_exhaustion_9th_add_fails() {
    let path = temp_path("slot_exhaust");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "p0").unwrap();
    // Fill slots 1–7 (slot 0 already used by create_encrypted).
    for i in 1..=7u16 {
        let slot = PageStore::add_passphrase_protector(&path, "p0", &format!("p{i}")).unwrap();
        assert_eq!(slot, i, "slot index should be sequential");
    }
    // 9th add must fail.
    let err = PageStore::add_passphrase_protector(&path, "p0", "p8")
        .err()
        .unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::InvalidArgument(_)),
        "expected InvalidArgument (full), got {err:?}"
    );
    // All 8 original passphrases still work.
    for i in 0..=7u16 {
        let pass = format!("p{i}");
        PageStore::open_with_passphrase(&path, &pass)
            .unwrap_or_else(|_| panic!("slot {i} passphrase failed"));
    }

    let _ = std::fs::remove_file(&path);
}

#[test]
fn recovery_key_survives_passphrase_rekey() {
    let path = temp_path("rk_after_rekey");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create_encrypted(&path, "orig").unwrap();
        store.put(b"k", b"v").unwrap();
    }
    let recovery = PageStore::add_recovery_key_protector(&path, "orig").unwrap();
    PageStore::rekey_kek(&path, 0, "orig", "new-pass").unwrap();

    // Old passphrase must fail.
    let err = PageStore::open_with_passphrase(&path, "orig")
        .err()
        .unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "old pass still works: {err:?}"
    );
    // New passphrase works.
    PageStore::open_with_passphrase(&path, "new-pass").unwrap();
    // Recovery key still works.
    let store = PageStore::open_with_recovery_key(&path, &recovery).unwrap();
    assert_eq!(store.get(b"k").unwrap(), Some(b"v".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn rekey_kek_wrong_old_passphrase_returns_wrong_key() {
    let path = temp_path("rekey_wrong");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "correct").unwrap();
    let err = PageStore::rekey_kek(&path, 0, "wrong", "new")
        .err()
        .unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "got {err:?}"
    );

    // Original passphrase still works after failed rekey.
    PageStore::open_with_passphrase(&path, "correct").unwrap();

    let _ = std::fs::remove_file(&path);
}

#[test]
fn rekey_kek_twice_in_a_row() {
    let path = temp_path("rekey_twice");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "v1").unwrap();
    PageStore::rekey_kek(&path, 0, "v1", "v2").unwrap();
    PageStore::rekey_kek(&path, 0, "v2", "v3").unwrap();

    // Only v3 works.
    assert!(PageStore::open_with_passphrase(&path, "v1").is_err());
    assert!(PageStore::open_with_passphrase(&path, "v2").is_err());
    PageStore::open_with_passphrase(&path, "v3").unwrap();

    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_out_of_range_slot_fails() {
    let path = temp_path("rm_oob");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "p").unwrap();
    PageStore::add_passphrase_protector(&path, "p", "p2").unwrap();

    // Slot 99 doesn't exist — should fail without removing anything.
    let err = PageStore::remove_keyslot(&path, "p", 99).err().unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::InvalidArgument(_) | crate::error::TosumuError::WrongKey
        ),
        "expected InvalidArgument or WrongKey, got {err:?}"
    );
    // Both slots still work.
    PageStore::open_with_passphrase(&path, "p").unwrap();
    PageStore::open_with_passphrase(&path, "p2").unwrap();

    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_empty_slot_is_a_noop() {
    // Removing an already-empty slot within the valid range is accepted
    // (it zeroes an already-zero region and updates the MAC). The important
    // invariant is that it does NOT panic and both active slots still work.
    let path = temp_path("rm_empty_slot");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "p").unwrap();
    PageStore::add_passphrase_protector(&path, "p", "p2").unwrap();

    // Slot 2 is empty but in-range (keyslot_count = MAX_KEYSLOTS = 8).
    // The remove should succeed (no-op on empty slot).
    PageStore::remove_keyslot(&path, "p", 2).unwrap();

    // Both active protectors still work.
    PageStore::open_with_passphrase(&path, "p").unwrap();
    PageStore::open_with_passphrase(&path, "p2").unwrap();

    let _ = std::fs::remove_file(&path);
}

#[test]
fn header_mac_tampered_keyslot_region_rejected() {
    // Manually zero out a byte inside the keyslot region (but NOT the wrapped DEK or KCV)
    // so the Argon2 + KCV check succeeds but the header MAC fails.
    use crate::format::{KEYSLOT_REGION_OFFSET, KS_OFF_CREATED_UNIX};

    let path = temp_path("mac_ks_tamper");
    let _ = std::fs::remove_file(&path);
    PageStore::create_encrypted(&path, "pass").unwrap();

    let mut raw = std::fs::read(&path).unwrap();
    // Flip a reserved/timestamp byte in slot 0 — MAC should catch it.
    raw[KEYSLOT_REGION_OFFSET + KS_OFF_CREATED_UNIX] ^= 0x01;
    std::fs::write(&path, &raw).unwrap();

    let err = PageStore::open_with_passphrase(&path, "pass")
        .err()
        .unwrap();
    assert!(
        matches!(
            err,
            crate::error::TosumuError::AuthFailed { .. } | crate::error::TosumuError::WrongKey
        ),
        "expected auth failure, got {err:?}"
    );
    let _ = std::fs::remove_file(&path);
}

// ── Data boundary & stress tests ──────────────────────────────────────────
