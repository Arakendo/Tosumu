#![cfg(feature = "experimental-shared-readers")]

use tosumu_core::experimental::{ReadTransaction, SharedKvDatabase};
use tosumu_core::TosumuError;

fn assert_send<T: Send>() {}
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn external_caller_observes_stable_snapshot_while_shared_writer_advances() {
    assert_send_sync::<SharedKvDatabase>();
    assert_send::<ReadTransaction>();

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("experimental-shared-readers.tsm");
    let database = SharedKvDatabase::create(&path).unwrap();
    database.put(b"a", b"captured").unwrap();
    database.put(b"b", b"stable").unwrap();

    let reader = database.snapshot().unwrap();
    let generation = reader.generation();
    let writer = database.clone();
    std::thread::spawn(move || {
        writer.put(b"a", b"new").unwrap();
        writer.put(b"c", b"later").unwrap();
    })
    .join()
    .unwrap();

    assert_eq!(database.get(b"a").unwrap(), Some(b"new".to_vec()));
    assert_eq!(reader.get(b"a").unwrap(), Some(b"captured".to_vec()));
    assert_eq!(
        reader.scan(b"a", b"z").unwrap(),
        vec![
            (b"a".to_vec(), b"captured".to_vec()),
            (b"b".to_vec(), b"stable".to_vec()),
        ]
    );

    let info = database.connection_info().unwrap();
    assert_eq!(info.active_readers, 1);
    assert_eq!(info.oldest_reader_generation, Some(generation));
    assert!(info.latest_generation > generation);
    assert!(info.checkpoint_blocked);
    assert!(info.retained_wal_bytes > 0);

    drop(reader);
    assert!(!database.connection_info().unwrap().checkpoint_blocked);

    drop(database);
    let reopened = SharedKvDatabase::open(&path).unwrap();
    assert_eq!(reopened.get(b"a").unwrap(), Some(b"new".to_vec()));
}

#[test]
fn encrypted_owner_commits_and_rolls_back_atomic_write_closures() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory
        .path()
        .join("experimental-encrypted-shared-readers.tsm");
    let database = SharedKvDatabase::create_encrypted(&path, "correct horse").unwrap();
    database.put(b"a", b"captured").unwrap();
    database.put(b"b", b"remove-after-capture").unwrap();

    let reader = database.snapshot().unwrap();
    let before_commit = database.connection_info().unwrap().latest_generation;
    database
        .write(|transaction| {
            transaction.put(b"a", b"committed")?;
            transaction.delete(b"b")?;
            transaction.put(b"c", b"same-generation")?;
            assert_eq!(transaction.get(b"a")?, Some(b"committed".to_vec()));
            Ok(())
        })
        .unwrap();

    assert!(database.connection_info().unwrap().latest_generation > before_commit);
    assert_eq!(database.get(b"a").unwrap(), Some(b"committed".to_vec()));
    assert_eq!(database.get(b"b").unwrap(), None);
    assert_eq!(
        reader.scan(b"a", b"z").unwrap(),
        vec![
            (b"a".to_vec(), b"captured".to_vec()),
            (b"b".to_vec(), b"remove-after-capture".to_vec()),
        ]
    );

    let before_rollback = database.connection_info().unwrap().latest_generation;
    let error = database
        .write(|transaction| {
            transaction.put(b"a", b"rolled-back")?;
            transaction.delete(b"c")?;
            Err::<(), _>(TosumuError::InvalidArgument("caller rollback"))
        })
        .unwrap_err();
    assert!(matches!(
        error,
        TosumuError::InvalidArgument("caller rollback")
    ));
    assert_eq!(
        database.connection_info().unwrap().latest_generation,
        before_rollback
    );
    assert_eq!(database.get(b"a").unwrap(), Some(b"committed".to_vec()));
    assert_eq!(
        database.get(b"c").unwrap(),
        Some(b"same-generation".to_vec())
    );

    drop(reader);
    drop(database);
    assert!(matches!(
        SharedKvDatabase::open_with_passphrase(&path, "wrong passphrase"),
        Err(TosumuError::WrongKey)
    ));
    let reopened = SharedKvDatabase::open_with_passphrase(&path, "correct horse").unwrap();
    assert_eq!(reopened.get(b"a").unwrap(), Some(b"committed".to_vec()));
    assert_eq!(reopened.get(b"b").unwrap(), None);
    assert_eq!(
        reopened.get(b"c").unwrap(),
        Some(b"same-generation".to_vec())
    );
}

#[test]
fn write_callback_reentry_fails_without_deadlock_or_generation_change() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("experimental-write-reentry.tsm");
    let database = SharedKvDatabase::create(&path).unwrap();
    database.put(b"key", b"committed").unwrap();
    let captured_generation = database.connection_info().unwrap().latest_generation;
    let reader = database.snapshot().unwrap();
    let reentrant = database.clone();

    let error = database
        .write(|transaction| {
            transaction.put(b"key", b"staged")?;
            reentrant.get(b"key")?;
            Ok(())
        })
        .unwrap_err();

    assert!(matches!(
        error,
        TosumuError::InvalidArgument(
            "shared database owner cannot be re-entered from its write callback"
        )
    ));

    let reader_error = database
        .write(|transaction| {
            transaction.put(b"key", b"staged-again")?;
            reader.get(b"key")?;
            Ok(())
        })
        .unwrap_err();
    assert!(matches!(
        reader_error,
        TosumuError::InvalidArgument(
            "shared database owner cannot be re-entered from its write callback"
        )
    ));
    assert_eq!(
        database.connection_info().unwrap().latest_generation,
        captured_generation
    );
    assert_eq!(database.get(b"key").unwrap(), Some(b"committed".to_vec()));
}

#[test]
fn panicking_write_callback_publishes_nothing_and_requires_reopen() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("experimental-write-panic.tsm");
    let database = SharedKvDatabase::create(&path).unwrap();
    database.put(b"key", b"committed").unwrap();

    let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let _: Result<(), TosumuError> = database.write(|transaction| {
            transaction.put(b"key", b"staged")?;
            panic!("caller panic");
        });
    }));
    assert!(panic.is_err());
    assert!(matches!(database.get(b"key"), Err(TosumuError::Poisoned)));

    drop(database);
    let reopened = SharedKvDatabase::open(&path).unwrap();
    assert_eq!(reopened.get(b"key").unwrap(), Some(b"committed".to_vec()));
}
