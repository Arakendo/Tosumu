//! Executable observations for the pre-MVP+10 coordination model.
//!
//! These tests preserve the baseline that MVP+10 must explain or deliberately
//! replace. They are observations, not snapshot-isolation or concurrency
//! guarantees.

use tosumu_core::KvStore;

fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn kv_store_is_currently_send_and_sync() {
    assert_send_sync::<KvStore>();
}

#[test]
fn simultaneous_readonly_handles_can_open_and_read() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("multiple-readers.tsm");
    let mut writer = KvStore::create(&path).unwrap();
    writer.put(b"key", b"value").unwrap();

    let first = KvStore::open_readonly(&path).unwrap();
    let second = KvStore::open_readonly(&path).unwrap();

    assert_eq!(first.get(b"key").unwrap(), Some(b"value".to_vec()));
    assert_eq!(second.get(b"key").unwrap(), Some(b"value".to_vec()));
}

#[test]
fn readonly_handle_is_a_live_view_after_writer_commit() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("live-view.tsm");
    let mut writer = KvStore::create(&path).unwrap();
    writer.put(b"key", b"before").unwrap();
    let reader = KvStore::open_readonly(&path).unwrap();

    writer
        .transaction(|transaction| transaction.put(b"key", b"after"))
        .unwrap();

    assert_eq!(reader.get(b"key").unwrap(), Some(b"after".to_vec()));
}

#[test]
fn readonly_open_during_transaction_sees_precommit_state_then_live_state() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("commit-visibility.tsm");
    let mut writer = KvStore::create(&path).unwrap();
    writer.put(b"key", b"before").unwrap();

    let mut reader = None;
    writer
        .transaction(|transaction| {
            transaction.put(b"key", b"after")?;
            let opened = KvStore::open_readonly(&path)?;
            assert_eq!(opened.get(b"key")?, Some(b"before".to_vec()));
            reader = Some(opened);
            Ok(())
        })
        .unwrap();

    let reader = reader.unwrap();
    assert_eq!(reader.get(b"key").unwrap(), Some(b"after".to_vec()));
}

#[test]
fn second_writable_handle_opens_without_a_writer_gate() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("writer-gate.tsm");
    let first = KvStore::create(&path).unwrap();

    let second = KvStore::open(&path).unwrap();

    assert!(first.get(b"missing").unwrap().is_none());
    assert!(second.get(b"missing").unwrap().is_none());
}
