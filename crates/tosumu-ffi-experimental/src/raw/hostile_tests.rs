use super::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HandleKind {
    Database,
    Snapshot,
    Bytes,
    Error,
    Connection,
    ScanPage,
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
    for outcome in [snapshot, bytes, connection, page] {
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
