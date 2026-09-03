use tosumu_core::{KvConditionalResult, KvReadTransaction, KvVersion, SharedKvStore, TosumuError};

fn assert_send<T: Send>() {}
fn assert_send_sync<T: Send + Sync>() {}

#[test]
fn conditional_writes_report_versions_and_reject_stale_or_wrong_values() {
    assert_send_sync::<KvVersion>();

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("conditional-shared-kv-store.tsm");
    let database = SharedKvStore::create(&path).unwrap();
    database.put(b"key", b"one").unwrap();

    let observed = database.get_with_version(b"key").unwrap();
    assert_eq!(observed.value, Some(b"one".to_vec()));

    database.put(b"unrelated", b"commit").unwrap();
    let stale = database
        .put_if_version(b"key", &observed.version, b"stale")
        .unwrap();
    assert!(matches!(stale, KvConditionalResult::NotApplied(_)));
    assert!(stale.version().generation() > observed.version.generation());
    assert_eq!(database.get(b"key").unwrap(), Some(b"one".to_vec()));

    let before_mismatch = stale.version().generation();
    let mismatch = database
        .compare_and_set(b"key", b"wrong", b"not-written")
        .unwrap();
    assert!(!mismatch.applied());
    assert_eq!(mismatch.version().generation(), before_mismatch);

    let applied = database.compare_and_set(b"key", b"one", b"two").unwrap();
    assert!(applied.applied());
    assert!(applied.version().generation() > before_mismatch);

    let by_version = database
        .put_if_version(b"key", applied.version(), b"three")
        .unwrap();
    assert!(by_version.applied());
    assert_eq!(database.get(b"key").unwrap(), Some(b"three".to_vec()));

    let before_invalid = by_version.version().generation();
    assert!(matches!(
        database.put_if_absent(b"", b"invalid"),
        Err(TosumuError::InvalidArgument("key must not be empty"))
    ));
    assert_eq!(
        database.connection_info().unwrap().latest_generation,
        before_invalid
    );
    database.put(b"after-error", b"usable").unwrap();
}

#[test]
fn concurrent_put_if_absent_has_exactly_one_winner() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("put-if-absent-shared-kv-store.tsm");
    let database = SharedKvStore::create(&path).unwrap();
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));

    let spawn_candidate = |value: &'static [u8]| {
        let candidate = database.clone();
        let barrier = std::sync::Arc::clone(&barrier);
        std::thread::spawn(move || {
            barrier.wait();
            candidate.put_if_absent(b"winner", value).unwrap()
        })
    };
    let first = spawn_candidate(b"first");
    let second = spawn_candidate(b"second");
    barrier.wait();

    let outcomes = [first.join().unwrap(), second.join().unwrap()];
    assert_eq!(
        outcomes.iter().filter(|outcome| outcome.applied()).count(),
        1
    );
    assert_eq!(outcomes[0].version(), outcomes[1].version());
    assert!(matches!(
        database.get(b"winner").unwrap().as_deref(),
        Some(b"first") | Some(b"second")
    ));
}

#[test]
fn version_tokens_are_rejected_by_other_and_reopened_owners() {
    let directory = tempfile::tempdir().unwrap();
    let first_path = directory.path().join("version-owner-first.tsm");
    let second_path = directory.path().join("version-owner-second.tsm");
    let first = SharedKvStore::create(&first_path).unwrap();
    let second = SharedKvStore::create(&second_path).unwrap();
    let token = first.get_with_version(b"missing").unwrap().version;

    assert!(matches!(
        second.put_if_version(b"key", &token, b"value"),
        Err(TosumuError::InvalidArgument(
            "version token belongs to a different or reopened shared KV store"
        ))
    ));
    assert_eq!(second.get(b"key").unwrap(), None);

    drop(first);
    let reopened = SharedKvStore::open(&first_path).unwrap();
    assert!(matches!(
        reopened.put_if_version(b"key", &token, b"value"),
        Err(TosumuError::InvalidArgument(
            "version token belongs to a different or reopened shared KV store"
        ))
    ));
    assert_eq!(reopened.get(b"key").unwrap(), None);
}

#[test]
fn external_caller_observes_stable_snapshot_while_shared_writer_advances() {
    assert_send_sync::<SharedKvStore>();
    assert_send::<KvReadTransaction>();

    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("shared-kv-store.tsm");
    let database = SharedKvStore::create(&path).unwrap();
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
    let reopened = SharedKvStore::open(&path).unwrap();
    assert_eq!(reopened.get(b"a").unwrap(), Some(b"new".to_vec()));
}

#[test]
fn encrypted_owner_commits_and_rolls_back_atomic_write_closures() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("encrypted-shared-kv-store.tsm");
    let database = SharedKvStore::create_encrypted(&path, "correct horse").unwrap();
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
        SharedKvStore::open_with_passphrase(&path, "wrong passphrase"),
        Err(TosumuError::WrongKey)
    ));
    let reopened = SharedKvStore::open_with_passphrase(&path, "correct horse").unwrap();
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
    let path = directory.path().join("shared-kv-write-reentry.tsm");
    let database = SharedKvStore::create(&path).unwrap();
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
    let path = directory.path().join("shared-kv-write-panic.tsm");
    let database = SharedKvStore::create(&path).unwrap();
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
    let reopened = SharedKvStore::open(&path).unwrap();
    assert_eq!(reopened.get(b"key").unwrap(), Some(b"committed".to_vec()));
}
