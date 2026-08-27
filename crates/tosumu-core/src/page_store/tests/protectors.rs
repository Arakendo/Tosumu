use super::*;

#[test]
fn encrypted_create_open_roundtrip() {
    let path = temp_path("enc_roundtrip");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create_encrypted(&path, "correct-horse").unwrap();
        store.put(b"secret", b"value").unwrap();
    }

    let store = PageStore::open_with_passphrase(&path, "correct-horse").unwrap();
    assert_eq!(store.get(b"secret").unwrap(), Some(b"value".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn encrypted_wrong_passphrase_returns_wrong_key() {
    let path = temp_path("enc_wrongkey");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "correct-horse").unwrap();

    let err = PageStore::open_with_passphrase(&path, "wrong-horse")
        .err()
        .unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "expected WrongKey, got {err:?}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn encrypted_open_without_passphrase_returns_wrong_key() {
    let path = temp_path("enc_nopw");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "somepass").unwrap();

    // Plain open() must refuse, not panic or silently succeed.
    let err = PageStore::open(&path).err().unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "expected WrongKey, got {err:?}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn encrypted_data_is_not_plaintext_in_file() {
    let path = temp_path("enc_notplain");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create_encrypted(&path, "p4ssw0rd").unwrap();
        store.put(b"confidential", b"secret_value_123").unwrap();
    }

    // The raw bytes of the file must not contain the plaintext value.
    let raw = std::fs::read(&path).unwrap();
    let needle = b"secret_value_123";
    let found = raw.windows(needle.len()).any(|w| w == needle);
    assert!(
        !found,
        "plaintext found in encrypted file — encryption is broken"
    );

    let _ = std::fs::remove_file(&path);
}

// ── MVP +7: key-management tests ──────────────────────────────────────────

#[test]
fn multi_slot_second_passphrase_can_unlock() {
    let path = temp_path("multi_slot");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create_encrypted(&path, "pass-a").unwrap();
        store.put(b"key", b"val").unwrap();
    }
    let slot = PageStore::add_passphrase_protector(&path, "pass-a", "pass-b").unwrap();
    assert!(slot >= 1, "second protector should be in slot ≥1");

    // Both passphrases can open the DB.
    let store_a = PageStore::open_with_passphrase(&path, "pass-a").unwrap();
    assert_eq!(store_a.get(b"key").unwrap(), Some(b"val".to_vec()));
    let store_b = PageStore::open_with_passphrase(&path, "pass-b").unwrap();
    assert_eq!(store_b.get(b"key").unwrap(), Some(b"val".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn recovery_key_roundtrip() {
    let path = temp_path("recovery_roundtrip");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create_encrypted(&path, "main-pass").unwrap();
        store.put(b"secret", b"data").unwrap();
    }
    let recovery = PageStore::add_recovery_key_protector(&path, "main-pass").unwrap();

    // Recovery key must look like XXXXXXXX-XXXXXXXX-XXXXXXXX-XXXXXXXX
    let parts: Vec<&str> = recovery.split('-').collect();
    assert_eq!(parts.len(), 4, "recovery key should have 4 groups");
    assert!(
        parts.iter().all(|p| p.len() == 8),
        "each group should be 8 chars"
    );

    // Must open with recovery key.
    let store = PageStore::open_with_recovery_key(&path, &recovery).unwrap();
    assert_eq!(store.get(b"secret").unwrap(), Some(b"data".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn keyfile_roundtrip() {
    use crate::format::KEYSLOT_KIND_KEYFILE;

    let path = temp_path("keyfile_roundtrip");
    let keyfile = temp_path("keyfile_roundtrip.bin");
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&keyfile);

    std::fs::write(&keyfile, [0xA5u8; 32]).unwrap();

    {
        let mut store = PageStore::create_encrypted(&path, "p").unwrap();
        store.put(b"secret", b"data").unwrap();
    }

    let slot = PageStore::add_keyfile_protector(&path, "p", &keyfile).unwrap();
    let slots = PageStore::list_keyslots(&path).unwrap();
    assert!(slots
        .iter()
        .any(|&(idx, kind)| idx == slot && kind == KEYSLOT_KIND_KEYFILE));

    let store = PageStore::open_with_keyfile(&path, &keyfile).unwrap();
    assert_eq!(store.get(b"secret").unwrap(), Some(b"data".to_vec()));

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&keyfile);
}

#[test]
fn wrong_recovery_key_returns_wrong_key() {
    let path = temp_path("wrong_recovery");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "p").unwrap();
    let _real = PageStore::add_recovery_key_protector(&path, "p").unwrap();

    let err = PageStore::open_with_recovery_key(&path, "AAAAAAAA-BBBBBBBB-CCCCCCCC-DDDDDDDD")
        .err()
        .unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "got {err:?}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_last_slot_is_rejected() {
    let path = temp_path("remove_last");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "only-pass").unwrap();

    let err = PageStore::remove_keyslot(&path, "only-pass", 0)
        .err()
        .unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::InvalidArgument(_)),
        "expected InvalidArgument, got {err:?}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn remove_second_slot_original_pass_still_works() {
    let path = temp_path("remove_second");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "orig").unwrap();
    let slot = PageStore::add_passphrase_protector(&path, "orig", "extra").unwrap();
    PageStore::remove_keyslot(&path, "orig", slot).unwrap();

    // Original pass still works.
    let store = PageStore::open_with_passphrase(&path, "orig").unwrap();
    drop(store);
    // Removed pass no longer works.
    let err = PageStore::open_with_passphrase(&path, "extra")
        .err()
        .unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "got {err:?}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn rekey_kek_old_fails_new_succeeds() {
    let path = temp_path("rekey_kek");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "old-pass").unwrap();
    PageStore::rekey_kek(&path, 0, "old-pass", "new-pass").unwrap();

    let err = PageStore::open_with_passphrase(&path, "old-pass")
        .err()
        .unwrap();
    assert!(
        matches!(err, crate::error::TosumuError::WrongKey),
        "old pass still works: {err:?}"
    );

    PageStore::open_with_passphrase(&path, "new-pass").unwrap();

    let _ = std::fs::remove_file(&path);
}

#[test]
fn list_keyslots_returns_active_slots() {
    let path = temp_path("list_slots");
    let _ = std::fs::remove_file(&path);

    PageStore::create_encrypted(&path, "p").unwrap();
    let slots = PageStore::list_keyslots(&path).unwrap();
    assert_eq!(slots.len(), 1);
    assert_eq!(slots[0].0, 0);

    PageStore::add_passphrase_protector(&path, "p", "p2").unwrap();
    let slots = PageStore::list_keyslots(&path).unwrap();
    assert_eq!(slots.len(), 2);

    let _ = std::fs::remove_file(&path);
}
