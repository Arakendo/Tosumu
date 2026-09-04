use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HandleKind {
    Database,
    Snapshot,
    Bytes,
    Error,
    Connection,
    ScanPage,
    Batch,
}

#[derive(Clone, Copy)]
struct Handle {
    kind: HandleKind,
    value: u64,
}

struct Operation {
    name: &'static str,
    kind: HandleKind,
    call: fn(u64) -> TosumuExperimentalV1Outcome,
}

fn assert_boundary(name: &str, outcome: TosumuExperimentalV1Outcome, status: u32) {
    assert_eq!(outcome.tag, boundary::TAG_BOUNDARY_FAILURE, "{name}");
    assert_eq!(outcome.status, status, "{name}");
    assert_eq!(outcome.payload, 0, "{name}");
}

fn operations() -> Vec<Operation> {
    let operations = vec![
        Operation {
            name: "database_close",
            kind: HandleKind::Database,
            call: |handle| tosumu_experimental_v1_database_close(handle),
        },
        Operation {
            name: "database_connection_info",
            kind: HandleKind::Database,
            call: |handle| tosumu_experimental_v1_database_connection_info(handle),
        },
        Operation {
            name: "database_put",
            kind: HandleKind::Database,
            call: |handle| unsafe {
                tosumu_experimental_v1_database_put(handle, b"k".as_ptr(), 1, b"v".as_ptr(), 1)
            },
        },
        Operation {
            name: "database_delete",
            kind: HandleKind::Database,
            call: |handle| unsafe {
                tosumu_experimental_v1_database_delete(handle, b"k".as_ptr(), 1)
            },
        },
        Operation {
            name: "database_get",
            kind: HandleKind::Database,
            call: |handle| unsafe { tosumu_experimental_v1_database_get(handle, b"k".as_ptr(), 1) },
        },
        Operation {
            name: "batch_append_put",
            kind: HandleKind::Batch,
            call: |handle| unsafe {
                tosumu_experimental_v1_batch_append_put(handle, b"k".as_ptr(), 1, b"v".as_ptr(), 1)
            },
        },
        Operation {
            name: "batch_append_delete",
            kind: HandleKind::Batch,
            call: |handle| unsafe {
                tosumu_experimental_v1_batch_append_delete(handle, b"k".as_ptr(), 1)
            },
        },
        Operation {
            name: "batch_close",
            kind: HandleKind::Batch,
            call: |handle| tosumu_experimental_v1_batch_close(handle),
        },
        Operation {
            name: "snapshot_begin",
            kind: HandleKind::Database,
            call: |handle| tosumu_experimental_v1_snapshot_begin(handle),
        },
        Operation {
            name: "snapshot_generation",
            kind: HandleKind::Snapshot,
            call: |handle| tosumu_experimental_v1_snapshot_generation(handle),
        },
        Operation {
            name: "snapshot_get",
            kind: HandleKind::Snapshot,
            call: |handle| unsafe { tosumu_experimental_v1_snapshot_get(handle, b"k".as_ptr(), 1) },
        },
        Operation {
            name: "snapshot_scan_page",
            kind: HandleKind::Snapshot,
            call: |handle| unsafe {
                tosumu_experimental_v1_snapshot_scan_page(
                    handle,
                    b"a".as_ptr(),
                    1,
                    b"z".as_ptr(),
                    1,
                    1,
                    16,
                )
            },
        },
        Operation {
            name: "snapshot_close",
            kind: HandleKind::Snapshot,
            call: |handle| tosumu_experimental_v1_snapshot_close(handle),
        },
        Operation {
            name: "bytes_length",
            kind: HandleKind::Bytes,
            call: |handle| tosumu_experimental_v1_bytes_length(handle),
        },
        Operation {
            name: "bytes_copy",
            kind: HandleKind::Bytes,
            call: |handle| unsafe {
                tosumu_experimental_v1_bytes_copy(handle, std::ptr::null_mut(), 0)
            },
        },
        Operation {
            name: "bytes_close",
            kind: HandleKind::Bytes,
            call: |handle| tosumu_experimental_v1_bytes_close(handle),
        },
        Operation {
            name: "connection_field",
            kind: HandleKind::Connection,
            call: |handle| tosumu_experimental_v1_connection_field(handle, 1),
        },
        Operation {
            name: "connection_close",
            kind: HandleKind::Connection,
            call: |handle| tosumu_experimental_v1_connection_close(handle),
        },
        Operation {
            name: "scan_page_pair_count",
            kind: HandleKind::ScanPage,
            call: |handle| tosumu_experimental_v1_scan_page_pair_count(handle),
        },
        Operation {
            name: "scan_page_pair_key",
            kind: HandleKind::ScanPage,
            call: |handle| tosumu_experimental_v1_scan_page_pair_key(handle, 0),
        },
        Operation {
            name: "scan_page_pair_value",
            kind: HandleKind::ScanPage,
            call: |handle| tosumu_experimental_v1_scan_page_pair_value(handle, 0),
        },
        Operation {
            name: "scan_page_next_start",
            kind: HandleKind::ScanPage,
            call: |handle| tosumu_experimental_v1_scan_page_next_start(handle),
        },
        Operation {
            name: "scan_page_blocked_entry_payload_bytes",
            kind: HandleKind::ScanPage,
            call: |handle| tosumu_experimental_v1_scan_page_blocked_entry_payload_bytes(handle),
        },
        Operation {
            name: "scan_page_close",
            kind: HandleKind::ScanPage,
            call: |handle| tosumu_experimental_v1_scan_page_close(handle),
        },
        Operation {
            name: "error_code",
            kind: HandleKind::Error,
            call: |handle| tosumu_experimental_v1_error_code(handle),
        },
        Operation {
            name: "error_status",
            kind: HandleKind::Error,
            call: |handle| tosumu_experimental_v1_error_status(handle),
        },
        Operation {
            name: "error_message",
            kind: HandleKind::Error,
            call: |handle| tosumu_experimental_v1_error_message(handle),
        },
        Operation {
            name: "error_detail_count",
            kind: HandleKind::Error,
            call: |handle| tosumu_experimental_v1_error_detail_count(handle),
        },
        Operation {
            name: "error_detail_key",
            kind: HandleKind::Error,
            call: |handle| tosumu_experimental_v1_error_detail_key(handle, 0),
        },
        Operation {
            name: "error_detail_type",
            kind: HandleKind::Error,
            call: |handle| tosumu_experimental_v1_error_detail_type(handle, 0),
        },
        Operation {
            name: "error_detail_scalar",
            kind: HandleKind::Error,
            call: |handle| tosumu_experimental_v1_error_detail_scalar(handle, 0),
        },
        Operation {
            name: "error_detail_string",
            kind: HandleKind::Error,
            call: |handle| tosumu_experimental_v1_error_detail_string(handle, 0),
        },
        Operation {
            name: "error_close",
            kind: HandleKind::Error,
            call: |handle| tosumu_experimental_v1_error_close(handle),
        },
    ];
    #[cfg(feature = "ffi-test-hooks")]
    {
        let mut operations = operations;
        operations.push(Operation {
            name: "test_inject_database_panic",
            kind: HandleKind::Database,
            call: |handle| tosumu_experimental_v1_test_inject_database_panic(handle),
        });
        operations.push(Operation {
            name: "test_inject_database_panic_after_write_acquisition",
            kind: HandleKind::Database,
            call: |handle| {
                tosumu_experimental_v1_test_inject_database_panic_after_write_acquisition(handle)
            },
        });
        operations
    }
    #[cfg(not(feature = "ffi-test-hooks"))]
    {
        operations
    }
}

#[test]
fn every_handle_operation_rejects_zero_forged_wrong_kind_and_stale_handles() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("hostile-handles.tsm");
    let database_path = database_path.to_str().unwrap().as_bytes();
    let missing_path = directory.path().join("missing.tsm");
    let missing_path = missing_path.to_str().unwrap().as_bytes();

    let database = unsafe {
        tosumu_experimental_v1_database_create(database_path.as_ptr(), database_path.len())
    };
    assert_eq!(database.tag, boundary::TAG_SUCCESS);
    assert_eq!(
        unsafe {
            tosumu_experimental_v1_database_put(
                database.payload,
                b"k".as_ptr(),
                1,
                b"v".as_ptr(),
                1,
            )
        }
        .tag,
        boundary::TAG_SUCCESS
    );
    let snapshot = tosumu_experimental_v1_snapshot_begin(database.payload);
    let bytes = unsafe { tosumu_experimental_v1_snapshot_get(snapshot.payload, b"k".as_ptr(), 1) };
    let connection = tosumu_experimental_v1_database_connection_info(database.payload);
    let page = unsafe {
        tosumu_experimental_v1_snapshot_scan_page(
            snapshot.payload,
            b"a".as_ptr(),
            1,
            b"z".as_ptr(),
            1,
            1,
            16,
        )
    };
    let error =
        unsafe { tosumu_experimental_v1_database_open(missing_path.as_ptr(), missing_path.len()) };
    let batch = tosumu_experimental_v1_batch_create();
    for outcome in [snapshot, bytes, connection, page, batch] {
        assert_eq!(outcome.tag, boundary::TAG_SUCCESS);
    }
    assert_eq!(error.tag, boundary::TAG_ERROR);

    let handles = [
        Handle {
            kind: HandleKind::Database,
            value: database.payload,
        },
        Handle {
            kind: HandleKind::Snapshot,
            value: snapshot.payload,
        },
        Handle {
            kind: HandleKind::Bytes,
            value: bytes.payload,
        },
        Handle {
            kind: HandleKind::Error,
            value: error.payload,
        },
        Handle {
            kind: HandleKind::Connection,
            value: connection.payload,
        },
        Handle {
            kind: HandleKind::ScanPage,
            value: page.payload,
        },
        Handle {
            kind: HandleKind::Batch,
            value: batch.payload,
        },
    ];
    let raw_handles: Vec<_> = handles.iter().map(|handle| handle.value).collect();
    assert_eq!(
        boundary::registered_handle_count(&raw_handles),
        handles.len()
    );

    let operations = operations();
    for operation in &operations {
        assert_boundary(
            operation.name,
            (operation.call)(0),
            boundary::BOUNDARY_INVALID_HANDLE,
        );
        assert_boundary(
            operation.name,
            (operation.call)(u64::MAX),
            boundary::BOUNDARY_INVALID_HANDLE,
        );
        for handle in handles
            .iter()
            .filter(|handle| handle.kind != operation.kind)
        {
            assert_boundary(
                operation.name,
                (operation.call)(handle.value),
                boundary::BOUNDARY_WRONG_KIND,
            );
        }
    }

    for handle in handles {
        let outcome = match handle.kind {
            HandleKind::Database => tosumu_experimental_v1_database_close(handle.value),
            HandleKind::Snapshot => tosumu_experimental_v1_snapshot_close(handle.value),
            HandleKind::Bytes => tosumu_experimental_v1_bytes_close(handle.value),
            HandleKind::Error => tosumu_experimental_v1_error_close(handle.value),
            HandleKind::Connection => tosumu_experimental_v1_connection_close(handle.value),
            HandleKind::ScanPage => tosumu_experimental_v1_scan_page_close(handle.value),
            HandleKind::Batch => tosumu_experimental_v1_batch_close(handle.value),
        };
        assert_eq!(outcome.tag, boundary::TAG_SUCCESS);
    }
    assert_eq!(boundary::registered_handle_count(&raw_handles), 0);

    for operation in &operations {
        let stale = handles
            .iter()
            .find(|handle| handle.kind == operation.kind)
            .unwrap();
        assert_boundary(
            operation.name,
            (operation.call)(stale.value),
            boundary::BOUNDARY_INVALID_HANDLE,
        );
    }
}

struct PointerOperation {
    name: &'static str,
    null_call: fn(u64, u64) -> TosumuExperimentalV1Outcome,
    oversized_call: fn(u64, u64) -> TosumuExperimentalV1Outcome,
}

#[test]
fn every_borrowed_input_rejects_null_and_unrepresentable_lengths() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("hostile-inputs.tsm");
    let path = path.to_str().unwrap().as_bytes();
    let database = unsafe { tosumu_experimental_v1_database_create(path.as_ptr(), path.len()) };
    assert_eq!(database.tag, boundary::TAG_SUCCESS);
    let snapshot = tosumu_experimental_v1_snapshot_begin(database.payload);
    assert_eq!(snapshot.tag, boundary::TAG_SUCCESS);
    let batch = tosumu_experimental_v1_batch_create();
    assert_eq!(batch.tag, boundary::TAG_SUCCESS);

    let operations = [
        PointerOperation {
            name: "database_create path",
            null_call: |_, _| unsafe {
                tosumu_experimental_v1_database_create(std::ptr::null(), 1)
            },
            oversized_call: |_, _| unsafe {
                tosumu_experimental_v1_database_create(b"x".as_ptr(), usize::MAX)
            },
        },
        PointerOperation {
            name: "database_open path",
            null_call: |_, _| unsafe { tosumu_experimental_v1_database_open(std::ptr::null(), 1) },
            oversized_call: |_, _| unsafe {
                tosumu_experimental_v1_database_open(b"x".as_ptr(), usize::MAX)
            },
        },
        PointerOperation {
            name: "database_put key",
            null_call: |database, _| unsafe {
                tosumu_experimental_v1_database_put(database, std::ptr::null(), 1, b"v".as_ptr(), 1)
            },
            oversized_call: |database, _| unsafe {
                tosumu_experimental_v1_database_put(
                    database,
                    b"k".as_ptr(),
                    usize::MAX,
                    b"v".as_ptr(),
                    1,
                )
            },
        },
        PointerOperation {
            name: "database_put value",
            null_call: |database, _| unsafe {
                tosumu_experimental_v1_database_put(database, b"k".as_ptr(), 1, std::ptr::null(), 1)
            },
            oversized_call: |database, _| unsafe {
                tosumu_experimental_v1_database_put(
                    database,
                    b"k".as_ptr(),
                    1,
                    b"v".as_ptr(),
                    usize::MAX,
                )
            },
        },
        PointerOperation {
            name: "database_delete key",
            null_call: |database, _| unsafe {
                tosumu_experimental_v1_database_delete(database, std::ptr::null(), 1)
            },
            oversized_call: |database, _| unsafe {
                tosumu_experimental_v1_database_delete(database, b"k".as_ptr(), usize::MAX)
            },
        },
        PointerOperation {
            name: "database_get key",
            null_call: |database, _| unsafe {
                tosumu_experimental_v1_database_get(database, std::ptr::null(), 1)
            },
            oversized_call: |database, _| unsafe {
                tosumu_experimental_v1_database_get(database, b"k".as_ptr(), usize::MAX)
            },
        },
        PointerOperation {
            name: "snapshot_get key",
            null_call: |_, snapshot| unsafe {
                tosumu_experimental_v1_snapshot_get(snapshot, std::ptr::null(), 1)
            },
            oversized_call: |_, snapshot| unsafe {
                tosumu_experimental_v1_snapshot_get(snapshot, b"k".as_ptr(), usize::MAX)
            },
        },
        PointerOperation {
            name: "snapshot_scan_page start",
            null_call: |_, snapshot| unsafe {
                tosumu_experimental_v1_snapshot_scan_page(
                    snapshot,
                    std::ptr::null(),
                    1,
                    b"z".as_ptr(),
                    1,
                    1,
                    1,
                )
            },
            oversized_call: |_, snapshot| unsafe {
                tosumu_experimental_v1_snapshot_scan_page(
                    snapshot,
                    b"a".as_ptr(),
                    usize::MAX,
                    b"z".as_ptr(),
                    1,
                    1,
                    1,
                )
            },
        },
        PointerOperation {
            name: "snapshot_scan_page end",
            null_call: |_, snapshot| unsafe {
                tosumu_experimental_v1_snapshot_scan_page(
                    snapshot,
                    b"a".as_ptr(),
                    1,
                    std::ptr::null(),
                    1,
                    1,
                    1,
                )
            },
            oversized_call: |_, snapshot| unsafe {
                tosumu_experimental_v1_snapshot_scan_page(
                    snapshot,
                    b"a".as_ptr(),
                    1,
                    b"z".as_ptr(),
                    usize::MAX,
                    1,
                    1,
                )
            },
        },
    ];

    for operation in operations {
        assert_boundary(
            operation.name,
            (operation.null_call)(database.payload, snapshot.payload),
            boundary::BOUNDARY_INVALID_POINTER,
        );
        assert_boundary(
            operation.name,
            (operation.oversized_call)(database.payload, snapshot.payload),
            boundary::BOUNDARY_LENGTH_OUT_OF_RANGE,
        );
    }

    for (name, outcome) in [
        ("batch_append_put key", unsafe {
            tosumu_experimental_v1_batch_append_put(
                batch.payload,
                std::ptr::null(),
                1,
                b"v".as_ptr(),
                1,
            )
        }),
        ("batch_append_put value", unsafe {
            tosumu_experimental_v1_batch_append_put(
                batch.payload,
                b"k".as_ptr(),
                1,
                std::ptr::null(),
                1,
            )
        }),
        ("batch_append_delete key", unsafe {
            tosumu_experimental_v1_batch_append_delete(batch.payload, std::ptr::null(), 1)
        }),
    ] {
        assert_boundary(name, outcome, boundary::BOUNDARY_INVALID_POINTER);
    }
    for (name, outcome) in [
        ("batch_append_put key", unsafe {
            tosumu_experimental_v1_batch_append_put(
                batch.payload,
                b"k".as_ptr(),
                usize::MAX,
                b"v".as_ptr(),
                1,
            )
        }),
        ("batch_append_put value", unsafe {
            tosumu_experimental_v1_batch_append_put(
                batch.payload,
                b"k".as_ptr(),
                1,
                b"v".as_ptr(),
                usize::MAX,
            )
        }),
        ("batch_append_delete key", unsafe {
            tosumu_experimental_v1_batch_append_delete(batch.payload, b"k".as_ptr(), usize::MAX)
        }),
    ] {
        assert_boundary(name, outcome, boundary::BOUNDARY_LENGTH_OUT_OF_RANGE);
    }

    assert_eq!(
        tosumu_experimental_v1_batch_close(batch.payload).tag,
        boundary::TAG_SUCCESS
    );

    assert_eq!(
        tosumu_experimental_v1_snapshot_close(snapshot.payload).tag,
        boundary::TAG_SUCCESS
    );
    assert_eq!(
        tosumu_experimental_v1_database_close(database.payload).tag,
        boundary::TAG_SUCCESS
    );
}

#[test]
fn empty_absent_capacity_and_index_outcomes_remain_distinct() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bounded-inputs.tsm");
    let path = path.to_str().unwrap().as_bytes();
    let database = unsafe { tosumu_experimental_v1_database_create(path.as_ptr(), path.len()) };
    assert_eq!(database.tag, boundary::TAG_SUCCESS);

    let empty_put = unsafe {
        tosumu_experimental_v1_database_put(
            database.payload,
            b"empty".as_ptr(),
            5,
            std::ptr::null(),
            0,
        )
    };
    assert_eq!(empty_put.tag, boundary::TAG_SUCCESS);
    let empty =
        unsafe { tosumu_experimental_v1_database_get(database.payload, b"empty".as_ptr(), 5) };
    assert_eq!(empty.tag, boundary::TAG_SUCCESS);
    assert_eq!(
        tosumu_experimental_v1_bytes_length(empty.payload).payload,
        0
    );
    let empty_copy =
        unsafe { tosumu_experimental_v1_bytes_copy(empty.payload, std::ptr::null_mut(), 0) };
    assert_eq!(empty_copy.tag, boundary::TAG_SUCCESS);
    assert_eq!(empty_copy.payload, 0);

    let absent =
        unsafe { tosumu_experimental_v1_database_get(database.payload, b"missing".as_ptr(), 7) };
    assert_eq!(absent.tag, boundary::TAG_ABSENT);

    let empty_key =
        unsafe { tosumu_experimental_v1_database_get(database.payload, std::ptr::null(), 0) };
    assert_eq!(empty_key.tag, boundary::TAG_ABSENT);

    assert_eq!(
        unsafe { tosumu_experimental_v1_database_create([0xff].as_ptr(), 1) }.status,
        boundary::BOUNDARY_INVALID_UTF8
    );
    assert_eq!(
        unsafe { tosumu_experimental_v1_database_create(b"a\0b".as_ptr(), 3) }.status,
        boundary::BOUNDARY_INVALID_PATH
    );
    assert_eq!(
        unsafe { tosumu_experimental_v1_database_create(std::ptr::null(), 0) }.status,
        boundary::BOUNDARY_INVALID_PATH
    );

    assert_eq!(
        unsafe {
            tosumu_experimental_v1_database_put(
                database.payload,
                b"data".as_ptr(),
                4,
                b"four".as_ptr(),
                4,
            )
        }
        .tag,
        boundary::TAG_SUCCESS
    );
    let data =
        unsafe { tosumu_experimental_v1_database_get(database.payload, b"data".as_ptr(), 4) };
    let mut too_small = [0xa5; 2];
    let required = unsafe {
        tosumu_experimental_v1_bytes_copy(data.payload, too_small.as_mut_ptr(), too_small.len())
    };
    assert_eq!(required.tag, boundary::TAG_SUCCESS);
    assert_eq!(required.payload, 4);
    assert_eq!(too_small, [0xa5; 2]);
    assert_boundary(
        "null output destination",
        unsafe { tosumu_experimental_v1_bytes_copy(data.payload, std::ptr::null_mut(), 4) },
        boundary::BOUNDARY_INVALID_POINTER,
    );

    let oversized_key = vec![0; tosumu_core::MAX_KEY_SIZE + 1];
    let oversized = unsafe {
        tosumu_experimental_v1_database_put(
            database.payload,
            oversized_key.as_ptr(),
            oversized_key.len(),
            b"v".as_ptr(),
            1,
        )
    };
    assert_eq!(oversized.tag, boundary::TAG_ERROR);
    assert_eq!(
        tosumu_experimental_v1_error_status(oversized.payload).payload,
        1
    );

    let snapshot = tosumu_experimental_v1_snapshot_begin(database.payload);
    let connection = tosumu_experimental_v1_database_connection_info(database.payload);
    let page = unsafe {
        tosumu_experimental_v1_snapshot_scan_page(
            snapshot.payload,
            b"data".as_ptr(),
            4,
            b"data".as_ptr(),
            4,
            1,
            16,
        )
    };
    assert_boundary(
        "connection invalid field",
        tosumu_experimental_v1_connection_field(connection.payload, u32::MAX),
        boundary::BOUNDARY_INVALID_INDEX,
    );
    assert_boundary(
        "page invalid pair index",
        tosumu_experimental_v1_scan_page_pair_key(page.payload, u64::MAX),
        boundary::BOUNDARY_INVALID_INDEX,
    );
    assert_boundary(
        "error invalid detail index",
        tosumu_experimental_v1_error_detail_key(oversized.payload, u64::MAX),
        boundary::BOUNDARY_INVALID_INDEX,
    );

    for outcome in [
        tosumu_experimental_v1_scan_page_close(page.payload),
        tosumu_experimental_v1_connection_close(connection.payload),
        tosumu_experimental_v1_snapshot_close(snapshot.payload),
        tosumu_experimental_v1_bytes_close(data.payload),
        tosumu_experimental_v1_bytes_close(empty.payload),
        tosumu_experimental_v1_error_close(oversized.payload),
        tosumu_experimental_v1_database_close(database.payload),
    ] {
        assert_eq!(outcome.tag, boundary::TAG_SUCCESS);
    }
}

#[test]
fn every_handle_kind_obeys_its_operation_and_finalizer_thread_rules() {
    let directory = tempfile::tempdir().unwrap();
    let database_path = directory.path().join("thread-rules.tsm");
    let database_path = database_path.to_str().unwrap().as_bytes();
    let missing_path = directory.path().join("missing.tsm");
    let missing_path = missing_path.to_str().unwrap().as_bytes();
    let database = unsafe {
        tosumu_experimental_v1_database_create(database_path.as_ptr(), database_path.len())
    };
    assert_eq!(
        unsafe {
            tosumu_experimental_v1_database_put(
                database.payload,
                b"k".as_ptr(),
                1,
                b"v".as_ptr(),
                1,
            )
        }
        .tag,
        boundary::TAG_SUCCESS
    );
    let snapshot = tosumu_experimental_v1_snapshot_begin(database.payload);
    let bytes = unsafe { tosumu_experimental_v1_snapshot_get(snapshot.payload, b"k".as_ptr(), 1) };
    let connection = tosumu_experimental_v1_database_connection_info(database.payload);
    let page = unsafe {
        tosumu_experimental_v1_snapshot_scan_page(
            snapshot.payload,
            b"a".as_ptr(),
            1,
            b"z".as_ptr(),
            1,
            1,
            16,
        )
    };
    let error =
        unsafe { tosumu_experimental_v1_database_open(missing_path.as_ptr(), missing_path.len()) };
    let batch = tosumu_experimental_v1_batch_create();

    let observations = std::thread::spawn(move || {
        [
            tosumu_experimental_v1_database_connection_info(database.payload),
            tosumu_experimental_v1_snapshot_generation(snapshot.payload),
            unsafe { tosumu_experimental_v1_batch_append_delete(batch.payload, b"k".as_ptr(), 1) },
            tosumu_experimental_v1_bytes_length(bytes.payload),
            tosumu_experimental_v1_error_status(error.payload),
            tosumu_experimental_v1_connection_field(connection.payload, 1),
            tosumu_experimental_v1_scan_page_pair_count(page.payload),
        ]
    })
    .join()
    .unwrap();
    for (name, outcome) in ["database", "snapshot", "batch"]
        .into_iter()
        .zip(observations[..3].iter().copied())
    {
        assert_boundary(name, outcome, boundary::BOUNDARY_WRONG_THREAD);
    }
    for outcome in &observations[3..] {
        assert_eq!(outcome.tag, boundary::TAG_SUCCESS);
    }

    let handles = [
        Handle {
            kind: HandleKind::Database,
            value: database.payload,
        },
        Handle {
            kind: HandleKind::Snapshot,
            value: snapshot.payload,
        },
        Handle {
            kind: HandleKind::Bytes,
            value: bytes.payload,
        },
        Handle {
            kind: HandleKind::Error,
            value: error.payload,
        },
        Handle {
            kind: HandleKind::Connection,
            value: connection.payload,
        },
        Handle {
            kind: HandleKind::ScanPage,
            value: page.payload,
        },
        Handle {
            kind: HandleKind::Batch,
            value: batch.payload,
        },
    ];
    let raw_handles: Vec<_> = handles.iter().map(|handle| handle.value).collect();
    let closes = std::thread::spawn(move || {
        handles.map(|handle| match handle.kind {
            HandleKind::Database => tosumu_experimental_v1_database_close(handle.value),
            HandleKind::Snapshot => tosumu_experimental_v1_snapshot_close(handle.value),
            HandleKind::Bytes => tosumu_experimental_v1_bytes_close(handle.value),
            HandleKind::Error => tosumu_experimental_v1_error_close(handle.value),
            HandleKind::Connection => tosumu_experimental_v1_connection_close(handle.value),
            HandleKind::ScanPage => tosumu_experimental_v1_scan_page_close(handle.value),
            HandleKind::Batch => tosumu_experimental_v1_batch_close(handle.value),
        })
    })
    .join()
    .unwrap();
    for outcome in closes {
        assert_eq!(outcome.tag, boundary::TAG_SUCCESS);
    }
    assert_eq!(boundary::registered_handle_count(&raw_handles), 0);
}

#[test]
fn concurrent_close_and_use_linearize_at_registry_lookup() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("close-use-race.tsm");
    let path = path.to_str().unwrap().as_bytes();
    let database = unsafe { tosumu_experimental_v1_database_create(path.as_ptr(), path.len()) };
    assert_eq!(
        unsafe {
            tosumu_experimental_v1_database_put(
                database.payload,
                b"k".as_ptr(),
                1,
                b"v".as_ptr(),
                1,
            )
        }
        .tag,
        boundary::TAG_SUCCESS
    );
    let snapshot = tosumu_experimental_v1_snapshot_begin(database.payload);

    for _ in 0..64 {
        let bytes =
            unsafe { tosumu_experimental_v1_snapshot_get(snapshot.payload, b"k".as_ptr(), 1) };
        assert_eq!(bytes.tag, boundary::TAG_SUCCESS);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
        let reader_barrier = std::sync::Arc::clone(&barrier);
        let reader = std::thread::spawn(move || {
            reader_barrier.wait();
            tosumu_experimental_v1_bytes_length(bytes.payload)
        });
        let closer_barrier = std::sync::Arc::clone(&barrier);
        let closer = std::thread::spawn(move || {
            closer_barrier.wait();
            tosumu_experimental_v1_bytes_close(bytes.payload)
        });
        barrier.wait();
        let read = reader.join().unwrap();
        let closed = closer.join().unwrap();
        assert_eq!(closed.tag, boundary::TAG_SUCCESS);
        match read.tag {
            boundary::TAG_SUCCESS => assert_eq!(read.payload, 1),
            boundary::TAG_BOUNDARY_FAILURE => {
                assert_eq!(read.status, boundary::BOUNDARY_INVALID_HANDLE);
            }
            _ => panic!("close/use race returned an unexpected outcome"),
        }
        assert_boundary(
            "post-race stale bytes",
            tosumu_experimental_v1_bytes_length(bytes.payload),
            boundary::BOUNDARY_INVALID_HANDLE,
        );
    }

    assert_eq!(
        tosumu_experimental_v1_snapshot_close(snapshot.payload).tag,
        boundary::TAG_SUCCESS
    );
    assert_eq!(
        tosumu_experimental_v1_database_close(database.payload).tag,
        boundary::TAG_SUCCESS
    );
}

#[test]
fn database_and_snapshot_close_races_preserve_completed_or_stale_outcomes() {
    let directory = tempfile::tempdir().unwrap();

    for index in 0..16 {
        let path = directory.path().join(format!("database-race-{index}.tsm"));
        let path = path.to_str().unwrap().as_bytes();
        let database = unsafe { tosumu_experimental_v1_database_create(path.as_ptr(), path.len()) };
        assert_eq!(
            unsafe {
                tosumu_experimental_v1_database_put(
                    database.payload,
                    b"k".as_ptr(),
                    1,
                    b"v".as_ptr(),
                    1,
                )
            }
            .tag,
            boundary::TAG_SUCCESS
        );

        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let closer_barrier = std::sync::Arc::clone(&barrier);
        let closer = std::thread::spawn(move || {
            closer_barrier.wait();
            tosumu_experimental_v1_database_close(database.payload)
        });
        barrier.wait();
        let read =
            unsafe { tosumu_experimental_v1_database_get(database.payload, b"k".as_ptr(), 1) };
        assert_eq!(closer.join().unwrap().tag, boundary::TAG_SUCCESS);
        match read.tag {
            boundary::TAG_SUCCESS => {
                assert_eq!(tosumu_experimental_v1_bytes_length(read.payload).payload, 1);
                assert_eq!(
                    tosumu_experimental_v1_bytes_close(read.payload).tag,
                    boundary::TAG_SUCCESS
                );
            }
            boundary::TAG_BOUNDARY_FAILURE => {
                assert_eq!(read.status, boundary::BOUNDARY_INVALID_HANDLE);
            }
            _ => panic!("database close/use race returned an unexpected outcome"),
        }
        assert_boundary(
            "post-race stale database",
            unsafe { tosumu_experimental_v1_database_get(database.payload, b"k".as_ptr(), 1) },
            boundary::BOUNDARY_INVALID_HANDLE,
        );
    }

    let path = directory.path().join("snapshot-races.tsm");
    let path = path.to_str().unwrap().as_bytes();
    let database = unsafe { tosumu_experimental_v1_database_create(path.as_ptr(), path.len()) };
    assert_eq!(database.tag, boundary::TAG_SUCCESS);
    for _ in 0..64 {
        let snapshot = tosumu_experimental_v1_snapshot_begin(database.payload);
        assert_eq!(snapshot.tag, boundary::TAG_SUCCESS);
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(2));
        let closer_barrier = std::sync::Arc::clone(&barrier);
        let closer = std::thread::spawn(move || {
            closer_barrier.wait();
            tosumu_experimental_v1_snapshot_close(snapshot.payload)
        });
        barrier.wait();
        let generation = tosumu_experimental_v1_snapshot_generation(snapshot.payload);
        assert_eq!(closer.join().unwrap().tag, boundary::TAG_SUCCESS);
        match generation.tag {
            boundary::TAG_SUCCESS => assert_eq!(generation.status, 0),
            boundary::TAG_BOUNDARY_FAILURE => {
                assert_eq!(generation.status, boundary::BOUNDARY_INVALID_HANDLE);
            }
            _ => panic!("snapshot close/use race returned an unexpected outcome"),
        }
        assert_boundary(
            "post-race stale snapshot",
            tosumu_experimental_v1_snapshot_generation(snapshot.payload),
            boundary::BOUNDARY_INVALID_HANDLE,
        );
    }
    assert_eq!(
        tosumu_experimental_v1_database_close(database.payload).tag,
        boundary::TAG_SUCCESS
    );
}
