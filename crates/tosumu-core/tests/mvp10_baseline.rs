//! Executable observations for the MVP+10 coordination model.
//!
//! These tests preserve the baseline that MVP+10 must explain or deliberately
//! replace. They are observations, not snapshot-isolation or concurrency
//! guarantees.

use tosumu_core::pager::Pager;
use tosumu_core::wal::{checkpoint, wal_path, WalReader, WalWriter};
use tosumu_core::{KvStore, TosumuError};

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
fn second_writable_handle_is_rejected_by_writer_gate() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("writer-gate.tsm");
    let first = KvStore::create(&path).unwrap();

    let error = match KvStore::open(&path) {
        Ok(_) => panic!("second writer must not be admitted"),
        Err(error) => error,
    };

    assert!(first.get(b"missing").unwrap().is_none());
    match error {
        TosumuError::FileBusy {
            path: busy_path,
            operation,
        } => {
            assert_eq!(busy_path, path.with_extension("tsm.writer.lock"));
            assert_eq!(operation, "acquiring database writer gate");
        }
        other => panic!("expected FileBusy, got {other:?}"),
    }
}

#[test]
fn dropping_writer_releases_gate_but_preserves_sidecar() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("writer-release.tsm");
    let lock_path = path.with_extension("tsm.writer.lock");
    let writer = KvStore::create(&path).unwrap();

    assert!(lock_path.exists());
    drop(writer);

    let reopened = KvStore::open(&path).unwrap();
    assert!(reopened.get(b"missing").unwrap().is_none());
    assert!(lock_path.exists());
}

#[test]
fn protector_edit_and_checkpoint_share_writer_gate() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("maintenance-gate.tsm");
    let writer = KvStore::create(&path).unwrap();

    assert!(matches!(
        Pager::remove_keyslot(&path, "ignored-for-sentinel", 0),
        Err(TosumuError::FileBusy { .. })
    ));
    assert!(matches!(
        checkpoint(&path, &wal_path(&path)),
        Err(TosumuError::FileBusy { .. })
    ));

    drop(writer);
    checkpoint(&path, &wal_path(&path)).unwrap();
}

#[test]
fn successful_commit_retains_no_monotonic_committed_lsn() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("commit-lsn.tsm");
    let mut writer = KvStore::create(&path).unwrap();
    writer
        .transaction(|transaction| transaction.put(b"key", b"value"))
        .unwrap();
    drop(writer);

    let header = tosumu_core::inspect::read_header_info(&path).unwrap();
    assert_eq!(header.wal_checkpoint_lsn, 0);
    assert!(WalReader::read_all(&wal_path(&path)).unwrap().is_empty());

    let wal = WalWriter::open(&wal_path(&path)).unwrap();
    assert_eq!(wal.next_lsn(), 1);
}

#[test]
fn format_2_ordinary_put_checkpoints_its_staged_generation_immediately() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("direct-put.tsm");
    let mut writer = KvStore::create(&path).unwrap();
    writer.put(b"key", b"value").unwrap();
    drop(writer);

    assert!(WalReader::read_all(&wal_path(&path)).unwrap().is_empty());
    assert_eq!(
        KvStore::open_readonly(&path).unwrap().get(b"key").unwrap(),
        Some(b"value".to_vec())
    );
}
