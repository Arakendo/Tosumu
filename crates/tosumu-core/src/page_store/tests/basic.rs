use super::*;

#[test]
fn create_open_round_trip() {
    let path = temp_path("round_trip");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create(&path).unwrap();
        store.put(b"hello", b"world").unwrap();
        store.put(b"foo", b"bar").unwrap();
    }

    let store = PageStore::open(&path).unwrap();
    assert_eq!(store.get(b"hello").unwrap(), Some(b"world".to_vec()));
    assert_eq!(store.get(b"foo").unwrap(), Some(b"bar".to_vec()));
    assert_eq!(store.get(b"missing").unwrap(), None);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn empty_file_opens_cleanly() {
    let path = temp_path("empty");
    let _ = std::fs::remove_file(&path);

    let store = PageStore::create(&path).unwrap();
    assert_eq!(store.stat().unwrap().data_pages, 1);
    let pairs = store.scan().unwrap();
    assert!(pairs.is_empty());

    let _ = std::fs::remove_file(&path);
}

/// The fresh root leaf must have slot_count == 0, and the B+ tree must
/// report height 1. This guards against any init_page regression where
/// free_start is set incorrectly (would cause ghost-slot reads).
#[test]
fn fresh_leaf_has_correct_header_state() {
    let path = temp_path("fresh_leaf");

    let store = PageStore::create(&path).unwrap();
    // Empty store: exactly one data page (the root leaf), height 1.
    assert_eq!(
        store.stat().unwrap().data_pages,
        1,
        "expected exactly one data page"
    );
    assert_eq!(
        store.tree.tree_height().unwrap(),
        1,
        "expected tree height 1 for empty store"
    );
    // Invariant checker also validates slot array bounds and free_start sanity.
    store.tree.check_invariants().unwrap();
    // No records should be readable.
    assert!(
        store.scan().unwrap().is_empty(),
        "fresh store must scan as empty"
    );
}

#[test]
fn delete_removes_key() {
    let path = temp_path("delete");
    let _ = std::fs::remove_file(&path);

    let mut store = PageStore::create(&path).unwrap();
    store.put(b"k", b"v").unwrap();
    store.delete(b"k").unwrap();
    assert_eq!(store.get(b"k").unwrap(), None);
    drop(store);

    // Survives reopen.
    let store2 = PageStore::open(&path).unwrap();
    assert_eq!(store2.get(b"k").unwrap(), None);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn overwrite_key() {
    let path = temp_path("overwrite");
    let _ = std::fs::remove_file(&path);

    let mut store = PageStore::create(&path).unwrap();
    store.put(b"k", b"v1").unwrap();
    store.put(b"k", b"v2").unwrap();
    assert_eq!(store.get(b"k").unwrap(), Some(b"v2".to_vec()));
    drop(store);

    let store2 = PageStore::open(&path).unwrap();
    assert_eq!(store2.get(b"k").unwrap(), Some(b"v2".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn scan_sorted() {
    let path = temp_path("scan");
    let _ = std::fs::remove_file(&path);

    let mut store = PageStore::create(&path).unwrap();
    store.put(b"c", b"3").unwrap();
    store.put(b"a", b"1").unwrap();
    store.put(b"b", b"2").unwrap();
    store.delete(b"b").unwrap();

    let pairs = store.scan().unwrap();
    assert_eq!(pairs.len(), 2);
    assert_eq!(pairs[0], (b"a".to_vec(), b"1".to_vec()));
    assert_eq!(pairs[1], (b"c".to_vec(), b"3".to_vec()));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn auth_failure_on_corrupted_page() {
    let path = temp_path("corrupt");
    let _ = std::fs::remove_file(&path);

    {
        let mut store = PageStore::create(&path).unwrap();
        store.put(b"key", b"val").unwrap();
    }

    // Corrupt the first data page (byte 4096 + 100 = inside the ciphertext).
    let mut raw = std::fs::read(&path).unwrap();
    raw[4096 + 100] ^= 0xFF;
    std::fs::write(&path, &raw).unwrap();

    let store = PageStore::open(&path).unwrap();
    let err = store.get(b"key").unwrap_err();
    assert!(matches!(err, crate::error::TosumuError::AuthFailed { .. }));

    let _ = std::fs::remove_file(&path);
}
