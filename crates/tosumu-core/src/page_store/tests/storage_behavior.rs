use super::*;

#[test]
fn empty_value_is_valid() {
    let path = temp_path("empty_val");
    let _ = std::fs::remove_file(&path);

    let mut store = PageStore::create(&path).unwrap();
    store.put(b"k", b"").unwrap();
    assert_eq!(store.get(b"k").unwrap(), Some(b"".to_vec()));
    drop(store);

    let store2 = PageStore::open(&path).unwrap();
    assert_eq!(store2.get(b"k").unwrap(), Some(b"".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn empty_key_rejected() {
    let path = temp_path("empty_key");
    let _ = std::fs::remove_file(&path);

    let mut store = PageStore::create(&path).unwrap();
    let err = store.put(b"", b"v").err().unwrap();
    assert!(matches!(err, crate::error::TosumuError::InvalidArgument(_)));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn binary_keys_with_null_bytes() {
    let path = temp_path("binary_keys");
    let _ = std::fs::remove_file(&path);

    let mut store = PageStore::create(&path).unwrap();
    store.put(b"\x00abc\x00", b"null-interior").unwrap();
    store.put(b"\xff\xff\xff", b"all-ff").unwrap();
    store.put(b"\x00", b"single-null").unwrap();

    assert_eq!(
        store.get(b"\x00abc\x00").unwrap(),
        Some(b"null-interior".to_vec())
    );
    assert_eq!(
        store.get(b"\xff\xff\xff").unwrap(),
        Some(b"all-ff".to_vec())
    );
    assert_eq!(store.get(b"\x00").unwrap(), Some(b"single-null".to_vec()));
    drop(store);

    let store2 = PageStore::open(&path).unwrap();
    assert_eq!(
        store2.get(b"\x00abc\x00").unwrap(),
        Some(b"null-interior".to_vec())
    );
    assert_eq!(
        store2.get(b"\xff\xff\xff").unwrap(),
        Some(b"all-ff".to_vec())
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn large_value_forces_overflow_pages() {
    use crate::format::RECORD_MAX_KV;

    let path = temp_path("overflow_val");
    let _ = std::fs::remove_file(&path);

    // A value just beyond the inline record limit requires overflow pages.
    let big_val: Vec<u8> = (0u8..=255u8).cycle().take(RECORD_MAX_KV + 1).collect();

    let mut store = PageStore::create(&path).unwrap();
    store.put(b"big", &big_val).unwrap();
    assert_eq!(
        store.get(b"big").unwrap().as_deref(),
        Some(big_val.as_slice())
    );
    drop(store);

    let store2 = PageStore::open(&path).unwrap();
    assert_eq!(
        store2.get(b"big").unwrap().as_deref(),
        Some(big_val.as_slice())
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn many_overwrites_same_key_final_value_correct() {
    let path = temp_path("overwrite_stress");
    let _ = std::fs::remove_file(&path);

    let mut store = PageStore::create(&path).unwrap();
    for i in 0u32..500 {
        store.put(b"x", format!("value-{i}").as_bytes()).unwrap();
    }
    assert_eq!(store.get(b"x").unwrap(), Some(b"value-499".to_vec()));
    drop(store);

    let store2 = PageStore::open(&path).unwrap();
    assert_eq!(store2.get(b"x").unwrap(), Some(b"value-499".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn delete_all_scan_returns_empty() {
    let path = temp_path("delete_all");
    let _ = std::fs::remove_file(&path);

    let mut store = PageStore::create(&path).unwrap();
    for i in 0u32..50 {
        store.put(format!("key-{i:04}").as_bytes(), b"val").unwrap();
    }
    for i in 0u32..50 {
        store.delete(format!("key-{i:04}").as_bytes()).unwrap();
    }
    assert!(
        store.scan().unwrap().is_empty(),
        "scan should be empty after delete-all"
    );
    drop(store);

    let store2 = PageStore::open(&path).unwrap();
    assert!(store2.scan().unwrap().is_empty());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn freelist_reuse_bounds_page_count() {
    // Write N keys, delete them all, write N keys again.
    // The second round should reuse freelist pages, so final page_count
    // should be close to (not double) the single-round count.
    let path = temp_path("freelist_reuse");
    let _ = std::fs::remove_file(&path);

    let n = 200u32;
    let mut store = PageStore::create(&path).unwrap();
    for i in 0..n {
        store.put(format!("k{i:04}").as_bytes(), b"data").unwrap();
    }
    let pages_after_first = store.stat().unwrap().page_count;

    for i in 0..n {
        store.delete(format!("k{i:04}").as_bytes()).unwrap();
    }
    for i in 0..n {
        store.put(format!("k{i:04}").as_bytes(), b"data2").unwrap();
    }
    let pages_after_second = store.stat().unwrap().page_count;

    // Second round must not have grown by more than the first round did
    // (some slack for compaction overhead is ok, but not 2x).
    assert!(
        pages_after_second <= pages_after_first * 2,
        "page_count blew up: {pages_after_first} → {pages_after_second}"
    );

    let _ = std::fs::remove_file(&path);
}

#[test]
fn encrypted_transaction_commit_survives_reopen() {
    let path = temp_path("enc_txn");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create_encrypted(&path, "txn-pass").unwrap();
        store
            .transaction(|s| {
                s.put(b"a", b"1")?;
                s.put(b"b", b"2")?;
                s.put(b"c", b"3")?;
                Ok(())
            })
            .unwrap();
    }

    let store = PageStore::open_with_passphrase(&path, "txn-pass").unwrap();
    assert_eq!(store.get(b"a").unwrap(), Some(b"1".to_vec()));
    assert_eq!(store.get(b"b").unwrap(), Some(b"2".to_vec()));
    assert_eq!(store.get(b"c").unwrap(), Some(b"3".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn encrypted_autocommit_after_transaction_survives_reopen() {
    let path = temp_path("enc_txn_then_put");
    let wal = diff_wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);

    {
        let mut store = PageStore::create_encrypted(&path, "txn-put-pass").unwrap();
        store
            .transaction(|tx| {
                tx.put(b"a", b"1")?;
                tx.put(b"a", b"2")?;
                Ok(())
            })
            .unwrap();
        store.put(b"b", b"3").unwrap();
    }

    let reopened = PageStore::open_with_passphrase(&path, "txn-put-pass").unwrap();
    assert_eq!(reopened.get(b"a").unwrap(), Some(b"2".to_vec()));
    assert_eq!(reopened.get(b"b").unwrap(), Some(b"3".to_vec()));

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}
