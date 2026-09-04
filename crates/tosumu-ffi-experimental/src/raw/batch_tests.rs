use super::*;

fn assert_success(outcome: TosumuExperimentalV1Outcome) -> u64 {
    assert_eq!(outcome.tag, boundary::TAG_SUCCESS);
    assert_eq!(outcome.status, 0);
    outcome.payload
}

fn assert_boundary(outcome: TosumuExperimentalV1Outcome, status: u32) {
    assert_eq!(outcome.tag, boundary::TAG_BOUNDARY_FAILURE);
    assert_eq!(outcome.status, status);
    assert_eq!(outcome.payload, 0);
}

unsafe fn create_database(path: &std::path::Path) -> u64 {
    let path = path.to_str().unwrap().as_bytes();
    assert_success(unsafe { tosumu_experimental_v1_database_create(path.as_ptr(), path.len()) })
}

unsafe fn append_put(batch: u64, key: &[u8], value: &[u8]) -> TosumuExperimentalV1Outcome {
    unsafe {
        tosumu_experimental_v1_batch_append_put(
            batch,
            key.as_ptr(),
            key.len(),
            value.as_ptr(),
            value.len(),
        )
    }
}

unsafe fn append_delete(batch: u64, key: &[u8]) -> TosumuExperimentalV1Outcome {
    unsafe { tosumu_experimental_v1_batch_append_delete(batch, key.as_ptr(), key.len()) }
}

unsafe fn read(database: u64, key: &[u8]) -> Option<Vec<u8>> {
    let outcome = unsafe { tosumu_experimental_v1_database_get(database, key.as_ptr(), key.len()) };
    if outcome.tag == boundary::TAG_ABSENT {
        return None;
    }
    let bytes = assert_success(outcome);
    let length = assert_success(tosumu_experimental_v1_bytes_length(bytes));
    let mut value = vec![0; usize::try_from(length).unwrap()];
    assert_eq!(
        assert_success(unsafe {
            tosumu_experimental_v1_bytes_copy(bytes, value.as_mut_ptr(), value.len())
        }),
        length
    );
    assert_success(tosumu_experimental_v1_bytes_close(bytes));
    Some(value)
}

#[test]
fn copied_batch_executes_in_order_once_and_abort_touches_nothing() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("batch.tsm");
    let database = unsafe { create_database(&path) };

    let batch = assert_success(tosumu_experimental_v1_batch_create());
    assert_success(unsafe { append_put(batch, b"first", b"one") });
    assert_success(unsafe { append_put(batch, b"same", b"before") });
    assert_success(unsafe { append_delete(batch, b"first") });
    assert_success(unsafe { append_put(batch, b"same", b"after") });
    assert_success(tosumu_experimental_v1_database_execute_batch(
        database, batch,
    ));
    assert_boundary(
        tosumu_experimental_v1_database_execute_batch(database, batch),
        boundary::BOUNDARY_INVALID_HANDLE,
    );
    assert_eq!(unsafe { read(database, b"first") }, None);
    assert_eq!(unsafe { read(database, b"same") }, Some(b"after".to_vec()));

    let aborted = assert_success(tosumu_experimental_v1_batch_create());
    assert_success(unsafe { append_put(aborted, b"aborted", b"never") });
    assert_success(tosumu_experimental_v1_batch_close(aborted));
    assert_eq!(unsafe { read(database, b"aborted") }, None);
    assert_success(tosumu_experimental_v1_database_close(database));
}

#[test]
fn batch_limits_reject_before_copy_and_leave_builder_usable() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("batch-limits.tsm");
    let database = unsafe { create_database(&path) };
    let batch = assert_success(tosumu_experimental_v1_batch_create());

    let oversized = vec![0x55; usize::try_from(boundary::MAX_BATCH_PAYLOAD_BYTES + 1).unwrap()];
    assert_boundary(
        unsafe { append_put(batch, b"large", &oversized) },
        boundary::BOUNDARY_BATCH_LIMIT_REACHED,
    );
    assert_success(unsafe { append_put(batch, b"small", b"accepted") });
    assert_success(tosumu_experimental_v1_database_execute_batch(
        database, batch,
    ));
    assert_eq!(
        unsafe { read(database, b"small") },
        Some(b"accepted".to_vec())
    );

    let commands = assert_success(tosumu_experimental_v1_batch_create());
    for index in 0..boundary::MAX_BATCH_COMMANDS {
        let key = index.to_le_bytes();
        assert_success(unsafe { append_delete(commands, &key) });
    }
    assert_boundary(
        unsafe { append_delete(commands, b"one-too-many") },
        boundary::BOUNDARY_BATCH_LIMIT_REACHED,
    );
    assert_success(tosumu_experimental_v1_batch_close(commands));
    assert_success(tosumu_experimental_v1_database_close(database));
}

#[test]
fn empty_and_failed_batches_are_consumed_without_mutation() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("batch-failure.tsm");
    let database = unsafe { create_database(&path) };
    assert_success(unsafe {
        tosumu_experimental_v1_database_put(database, b"stable".as_ptr(), 6, b"before".as_ptr(), 6)
    });

    let empty = assert_success(tosumu_experimental_v1_batch_create());
    assert_boundary(
        tosumu_experimental_v1_database_execute_batch(database, empty),
        boundary::BOUNDARY_EMPTY_BATCH,
    );
    assert_boundary(
        tosumu_experimental_v1_batch_close(empty),
        boundary::BOUNDARY_INVALID_HANDLE,
    );

    let invalid = assert_success(tosumu_experimental_v1_batch_create());
    assert_success(unsafe { append_put(invalid, b"staged", b"never") });
    let invalid_key = vec![b'k'; tosumu_core::MAX_KEY_SIZE + 1];
    let rejected = unsafe { append_put(invalid, &invalid_key, b"value") };
    assert_eq!(rejected.tag, boundary::TAG_ERROR);
    assert_success(tosumu_experimental_v1_error_close(rejected.payload));
    assert_success(tosumu_experimental_v1_database_execute_batch(
        database, invalid,
    ));
    assert_eq!(
        unsafe { read(database, b"staged") },
        Some(b"never".to_vec())
    );
    assert_eq!(
        unsafe { read(database, b"stable") },
        Some(b"before".to_vec())
    );
    assert_success(tosumu_experimental_v1_database_close(database));
}

#[test]
fn execute_validates_both_handles_and_linearizes_with_close() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("batch-handles.tsm");
    let database = unsafe { create_database(&path) };

    let retained = assert_success(tosumu_experimental_v1_batch_create());
    assert_success(unsafe { append_put(retained, b"retained", b"yes") });
    assert_boundary(
        tosumu_experimental_v1_database_execute_batch(u64::MAX, retained),
        boundary::BOUNDARY_INVALID_HANDLE,
    );
    assert_success(tosumu_experimental_v1_database_execute_batch(
        database, retained,
    ));

    assert_boundary(
        tosumu_experimental_v1_database_execute_batch(database, database),
        boundary::BOUNDARY_WRONG_KIND,
    );
    let stale = assert_success(tosumu_experimental_v1_batch_create());
    assert_success(tosumu_experimental_v1_batch_close(stale));
    assert_boundary(
        tosumu_experimental_v1_database_execute_batch(database, stale),
        boundary::BOUNDARY_INVALID_HANDLE,
    );

    for index in 0..32u64 {
        let batch = assert_success(tosumu_experimental_v1_batch_create());
        let key = format!("race-{index}");
        assert_success(unsafe { append_put(batch, key.as_bytes(), b"value") });
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let close_barrier = std::sync::Arc::clone(&barrier);
        let closer = std::thread::spawn(move || {
            close_barrier.wait();
            tosumu_experimental_v1_batch_close(batch)
        });
        barrier.wait();
        let executed = tosumu_experimental_v1_database_execute_batch(database, batch);
        let closed = closer.join().unwrap();
        match (executed.tag, closed.tag) {
            (boundary::TAG_SUCCESS, boundary::TAG_BOUNDARY_FAILURE) => {
                assert_eq!(closed.status, boundary::BOUNDARY_INVALID_HANDLE);
                assert_eq!(
                    unsafe { read(database, key.as_bytes()) },
                    Some(b"value".to_vec())
                );
            }
            (boundary::TAG_BOUNDARY_FAILURE, boundary::TAG_SUCCESS) => {
                assert_eq!(executed.status, boundary::BOUNDARY_INVALID_HANDLE);
                assert_eq!(unsafe { read(database, key.as_bytes()) }, None);
            }
            _ => panic!("batch close/execute race returned an unexpected outcome pair"),
        }
    }

    assert_success(tosumu_experimental_v1_database_close(database));
}

#[cfg(feature = "ffi-test-hooks")]
#[test]
fn injected_error_and_panic_publish_no_staged_batch_commands() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("batch-faults.tsm");
    let path_bytes = path.to_str().unwrap().as_bytes();
    let database = unsafe { create_database(&path) };

    let failed = assert_success(tosumu_experimental_v1_batch_create());
    assert_success(unsafe { append_put(failed, b"error-staged", b"never") });
    assert_success(tosumu_experimental_v1_test_batch_append_failure(failed, 1));
    let failure = tosumu_experimental_v1_database_execute_batch(database, failed);
    assert_eq!(failure.tag, boundary::TAG_ERROR);
    assert_success(tosumu_experimental_v1_error_close(failure.payload));
    assert_eq!(unsafe { read(database, b"error-staged") }, None);
    let after_error = assert_success(tosumu_experimental_v1_batch_create());
    assert_success(unsafe { append_put(after_error, b"still-usable", b"yes") });
    assert_success(tosumu_experimental_v1_database_execute_batch(
        database,
        after_error,
    ));
    assert_eq!(
        unsafe { read(database, b"still-usable") },
        Some(b"yes".to_vec())
    );

    let panicking = assert_success(tosumu_experimental_v1_batch_create());
    assert_success(unsafe { append_put(panicking, b"panic-staged", b"never") });
    assert_success(tosumu_experimental_v1_test_batch_append_failure(
        panicking, 2,
    ));
    assert_boundary(
        tosumu_experimental_v1_database_execute_batch(database, panicking),
        boundary::BOUNDARY_PANIC,
    );
    assert_boundary(
        unsafe { tosumu_experimental_v1_database_get(database, b"panic-staged".as_ptr(), 12) },
        boundary::BOUNDARY_POISONED,
    );
    assert_success(tosumu_experimental_v1_database_close(database));

    let reopened = assert_success(unsafe {
        tosumu_experimental_v1_database_open(path_bytes.as_ptr(), path_bytes.len())
    });
    assert_eq!(unsafe { read(reopened, b"error-staged") }, None);
    assert_eq!(unsafe { read(reopened, b"panic-staged") }, None);
    assert_success(tosumu_experimental_v1_database_close(reopened));
}
