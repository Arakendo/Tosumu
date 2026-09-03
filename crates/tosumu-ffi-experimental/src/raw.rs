#![deny(unsafe_op_in_unsafe_fn)]
#![allow(clippy::missing_safety_doc)]

use std::panic::{catch_unwind, AssertUnwindSafe};

use crate::boundary::{self, CallFailure, Kind};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TosumuExperimentalV1Outcome {
    pub tag: u32,
    pub status: u32,
    pub payload: u64,
}

impl TosumuExperimentalV1Outcome {
    const fn success(payload: u64) -> Self {
        Self {
            tag: boundary::TAG_SUCCESS,
            status: 0,
            payload,
        }
    }

    const fn absent() -> Self {
        Self {
            tag: boundary::TAG_ABSENT,
            status: 0,
            payload: 0,
        }
    }

    const fn boundary(status: u32) -> Self {
        Self {
            tag: boundary::TAG_BOUNDARY_FAILURE,
            status,
            payload: 0,
        }
    }
}

fn failure(error: CallFailure) -> TosumuExperimentalV1Outcome {
    match error {
        CallFailure::Boundary(status) => TosumuExperimentalV1Outcome::boundary(status),
        CallFailure::Core(error) => match boundary::insert_error(error) {
            Ok(handle) => TosumuExperimentalV1Outcome {
                tag: boundary::TAG_ERROR,
                status: 0,
                payload: handle,
            },
            Err(status) => TosumuExperimentalV1Outcome::boundary(status),
        },
    }
}

fn contained(
    database_to_poison: Option<u64>,
    operation: impl FnOnce() -> Result<TosumuExperimentalV1Outcome, CallFailure>,
) -> TosumuExperimentalV1Outcome {
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(outcome)) => outcome,
        Ok(Err(error)) => failure(error),
        Err(_) => {
            if let Some(handle) = database_to_poison {
                boundary::poison_database(handle);
            }
            TosumuExperimentalV1Outcome::boundary(boundary::BOUNDARY_PANIC)
        }
    }
}

unsafe fn input<'a>(pointer: *const u8, length: usize) -> Result<&'a [u8], CallFailure> {
    if length == 0 {
        return Ok(&[]);
    }
    if pointer.is_null() {
        return Err(CallFailure::Boundary(boundary::BOUNDARY_INVALID_POINTER));
    }
    // SAFETY: The C contract requires a non-null readable region of `length`
    // bytes for this call. The null and zero-length cases were handled above.
    Ok(unsafe { std::slice::from_raw_parts(pointer, length) })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_abi_version() -> u32 {
    1
}

#[no_mangle]
pub unsafe extern "C" fn tosumu_experimental_v1_database_create(
    path: *const u8,
    path_length: usize,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        let path = unsafe { input(path, path_length) }?;
        boundary::create(path).map(TosumuExperimentalV1Outcome::success)
    })
}

#[no_mangle]
pub unsafe extern "C" fn tosumu_experimental_v1_database_open(
    path: *const u8,
    path_length: usize,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        let path = unsafe { input(path, path_length) }?;
        boundary::open(path).map(TosumuExperimentalV1Outcome::success)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_database_close(
    database: u64,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::close(database, Kind::Database)
            .map(|()| TosumuExperimentalV1Outcome::success(0))
            .map_err(CallFailure::Boundary)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_database_connection_info(
    database: u64,
) -> TosumuExperimentalV1Outcome {
    contained(Some(database), || {
        boundary::connection_info(database).map(TosumuExperimentalV1Outcome::success)
    })
}

#[no_mangle]
pub unsafe extern "C" fn tosumu_experimental_v1_database_put(
    database: u64,
    key: *const u8,
    key_length: usize,
    value: *const u8,
    value_length: usize,
) -> TosumuExperimentalV1Outcome {
    contained(Some(database), || {
        let key = unsafe { input(key, key_length) }?;
        let value = unsafe { input(value, value_length) }?;
        boundary::put(database, key, value).map(|()| TosumuExperimentalV1Outcome::success(0))
    })
}

#[no_mangle]
pub unsafe extern "C" fn tosumu_experimental_v1_database_delete(
    database: u64,
    key: *const u8,
    key_length: usize,
) -> TosumuExperimentalV1Outcome {
    contained(Some(database), || {
        let key = unsafe { input(key, key_length) }?;
        boundary::delete(database, key).map(|()| TosumuExperimentalV1Outcome::success(0))
    })
}

#[no_mangle]
pub unsafe extern "C" fn tosumu_experimental_v1_database_get(
    database: u64,
    key: *const u8,
    key_length: usize,
) -> TosumuExperimentalV1Outcome {
    contained(Some(database), || {
        let key = unsafe { input(key, key_length) }?;
        Ok(match boundary::get(database, key)? {
            Some(handle) => TosumuExperimentalV1Outcome::success(handle),
            None => TosumuExperimentalV1Outcome::absent(),
        })
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_snapshot_begin(
    database: u64,
) -> TosumuExperimentalV1Outcome {
    contained(Some(database), || {
        boundary::snapshot_begin(database).map(TosumuExperimentalV1Outcome::success)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_snapshot_generation(
    snapshot: u64,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::snapshot_generation(snapshot).map(TosumuExperimentalV1Outcome::success)
    })
}

#[no_mangle]
pub unsafe extern "C" fn tosumu_experimental_v1_snapshot_get(
    snapshot: u64,
    key: *const u8,
    key_length: usize,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        let key = unsafe { input(key, key_length) }?;
        Ok(match boundary::snapshot_get(snapshot, key)? {
            Some(handle) => TosumuExperimentalV1Outcome::success(handle),
            None => TosumuExperimentalV1Outcome::absent(),
        })
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_snapshot_close(
    snapshot: u64,
) -> TosumuExperimentalV1Outcome {
    close(snapshot, Kind::Snapshot)
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_bytes_length(bytes: u64) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        let length = u64::try_from(boundary::bytes(bytes).map_err(CallFailure::Boundary)?.len())
            .map_err(|_| CallFailure::Boundary(boundary::BOUNDARY_REGISTRY_FULL))?;
        Ok(TosumuExperimentalV1Outcome::success(length))
    })
}

#[no_mangle]
pub unsafe extern "C" fn tosumu_experimental_v1_bytes_copy(
    bytes: u64,
    destination: *mut u8,
    capacity: usize,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        let bytes = boundary::bytes(bytes).map_err(CallFailure::Boundary)?;
        let required = u64::try_from(bytes.len())
            .map_err(|_| CallFailure::Boundary(boundary::BOUNDARY_REGISTRY_FULL))?;
        if capacity < bytes.len() || bytes.is_empty() {
            return Ok(TosumuExperimentalV1Outcome::success(required));
        }
        if destination.is_null() {
            return Err(CallFailure::Boundary(boundary::BOUNDARY_INVALID_POINTER));
        }
        // SAFETY: The C contract requires a writable non-overlapping region of
        // `capacity` bytes. Capacity and null were checked; source is private.
        unsafe { std::ptr::copy_nonoverlapping(bytes.as_ptr(), destination, bytes.len()) };
        Ok(TosumuExperimentalV1Outcome::success(required))
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_bytes_close(bytes: u64) -> TosumuExperimentalV1Outcome {
    close(bytes, Kind::Bytes)
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_connection_field(
    connection: u64,
    field: u32,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        Ok(
            match boundary::connection_field(connection, field).map_err(CallFailure::Boundary)? {
                Some(value) => TosumuExperimentalV1Outcome::success(value),
                None => TosumuExperimentalV1Outcome::absent(),
            },
        )
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_connection_close(
    connection: u64,
) -> TosumuExperimentalV1Outcome {
    close(connection, Kind::Connection)
}

#[cfg(feature = "ffi-test-hooks")]
#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_test_inject_database_panic(
    database: u64,
) -> TosumuExperimentalV1Outcome {
    contained(Some(database), || {
        panic!("experimental C boundary panic injection")
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_error_code(error: u64) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::error_code(error).map(TosumuExperimentalV1Outcome::success)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_error_status(error: u64) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::error_status(error)
            .map(TosumuExperimentalV1Outcome::success)
            .map_err(CallFailure::Boundary)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_error_message(error: u64) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::error_message(error).map(TosumuExperimentalV1Outcome::success)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_error_detail_count(
    error: u64,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::error_detail_count(error)
            .map(TosumuExperimentalV1Outcome::success)
            .map_err(CallFailure::Boundary)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_error_detail_key(
    error: u64,
    index: u64,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::error_detail_key(error, index).map(TosumuExperimentalV1Outcome::success)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_error_detail_type(
    error: u64,
    index: u64,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::error_detail_type(error, index)
            .map(TosumuExperimentalV1Outcome::success)
            .map_err(CallFailure::Boundary)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_error_detail_scalar(
    error: u64,
    index: u64,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::error_detail_scalar(error, index)
            .map(TosumuExperimentalV1Outcome::success)
            .map_err(CallFailure::Boundary)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_error_detail_string(
    error: u64,
    index: u64,
) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::error_detail_string(error, index).map(TosumuExperimentalV1Outcome::success)
    })
}

#[no_mangle]
pub extern "C" fn tosumu_experimental_v1_error_close(error: u64) -> TosumuExperimentalV1Outcome {
    close(error, Kind::Error)
}

fn close(handle: u64, kind: Kind) -> TosumuExperimentalV1Outcome {
    contained(None, || {
        boundary::close(handle, kind)
            .map(|()| TosumuExperimentalV1Outcome::success(0))
            .map_err(CallFailure::Boundary)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path_bytes(path: &std::path::Path) -> Vec<u8> {
        path.to_str()
            .expect("temporary path must be UTF-8")
            .as_bytes()
            .to_vec()
    }

    unsafe fn read_bytes(handle: u64) -> Vec<u8> {
        let length = tosumu_experimental_v1_bytes_length(handle);
        assert_eq!(length.tag, boundary::TAG_SUCCESS);
        let mut output = vec![0; usize::try_from(length.payload).unwrap()];
        let copied =
            unsafe { tosumu_experimental_v1_bytes_copy(handle, output.as_mut_ptr(), output.len()) };
        assert_eq!(copied, length);
        assert_eq!(
            tosumu_experimental_v1_bytes_close(handle).tag,
            boundary::TAG_SUCCESS
        );
        output
    }

    #[test]
    fn lifecycle_preserves_snapshot_and_owned_results() {
        let directory = tempfile::tempdir().unwrap();
        let path = path_bytes(&directory.path().join("ffi.tsm"));
        let key = b"key";
        let first = b"first";
        let second = b"second";

        let database = unsafe { tosumu_experimental_v1_database_create(path.as_ptr(), path.len()) };
        assert_eq!(database.tag, boundary::TAG_SUCCESS);
        let put = unsafe {
            tosumu_experimental_v1_database_put(
                database.payload,
                key.as_ptr(),
                key.len(),
                first.as_ptr(),
                first.len(),
            )
        };
        assert_eq!(put.tag, boundary::TAG_SUCCESS);
        let snapshot = tosumu_experimental_v1_snapshot_begin(database.payload);
        assert_eq!(snapshot.tag, boundary::TAG_SUCCESS);
        let put = unsafe {
            tosumu_experimental_v1_database_put(
                database.payload,
                key.as_ptr(),
                key.len(),
                second.as_ptr(),
                second.len(),
            )
        };
        assert_eq!(put.tag, boundary::TAG_SUCCESS);

        let latest = unsafe {
            tosumu_experimental_v1_database_get(database.payload, key.as_ptr(), key.len())
        };
        let pinned = unsafe {
            tosumu_experimental_v1_snapshot_get(snapshot.payload, key.as_ptr(), key.len())
        };
        assert_eq!(latest.tag, boundary::TAG_SUCCESS);
        assert_eq!(pinned.tag, boundary::TAG_SUCCESS);
        let connection = tosumu_experimental_v1_database_connection_info(database.payload);
        assert_eq!(connection.tag, boundary::TAG_SUCCESS);
        assert_eq!(
            tosumu_experimental_v1_connection_field(connection.payload, 1).payload,
            1
        );
        assert_eq!(
            tosumu_experimental_v1_database_close(database.payload).tag,
            boundary::TAG_SUCCESS
        );
        assert_eq!(
            tosumu_experimental_v1_connection_close(connection.payload).tag,
            boundary::TAG_SUCCESS
        );
        assert_eq!(unsafe { read_bytes(latest.payload) }, second);
        assert_eq!(unsafe { read_bytes(pinned.payload) }, first);

        let after_close = unsafe {
            tosumu_experimental_v1_snapshot_get(snapshot.payload, key.as_ptr(), key.len())
        };
        assert_eq!(after_close.tag, boundary::TAG_SUCCESS);
        assert_eq!(unsafe { read_bytes(after_close.payload) }, first);
        assert_eq!(
            tosumu_experimental_v1_snapshot_close(snapshot.payload).tag,
            boundary::TAG_SUCCESS
        );
        assert_eq!(
            tosumu_experimental_v1_snapshot_close(snapshot.payload).status,
            boundary::BOUNDARY_INVALID_HANDLE
        );
    }

    #[test]
    fn core_error_is_owned_and_survives_failed_open() {
        let directory = tempfile::tempdir().unwrap();
        let path = path_bytes(&directory.path().join("missing.tsm"));
        let outcome = unsafe { tosumu_experimental_v1_database_open(path.as_ptr(), path.len()) };
        assert_eq!(outcome.tag, boundary::TAG_ERROR);

        let code = tosumu_experimental_v1_error_code(outcome.payload);
        let message = tosumu_experimental_v1_error_message(outcome.payload);
        assert_eq!(code.tag, boundary::TAG_SUCCESS);
        assert_eq!(message.tag, boundary::TAG_SUCCESS);
        assert!(!unsafe { read_bytes(code.payload) }.is_empty());
        assert!(!unsafe { read_bytes(message.payload) }.is_empty());
        assert_eq!(
            tosumu_experimental_v1_error_close(outcome.payload).tag,
            boundary::TAG_SUCCESS
        );
    }

    #[test]
    fn null_nonempty_input_and_wrong_kind_fail_at_boundary() {
        let invalid = unsafe { tosumu_experimental_v1_database_open(std::ptr::null(), 1) };
        assert_eq!(invalid.tag, boundary::TAG_BOUNDARY_FAILURE);
        assert_eq!(invalid.status, boundary::BOUNDARY_INVALID_POINTER);

        let directory = tempfile::tempdir().unwrap();
        let path = path_bytes(&directory.path().join("ffi.tsm"));
        let database = unsafe { tosumu_experimental_v1_database_create(path.as_ptr(), path.len()) };
        assert_eq!(
            tosumu_experimental_v1_bytes_length(database.payload).status,
            boundary::BOUNDARY_WRONG_KIND
        );
        assert_eq!(
            tosumu_experimental_v1_database_close(database.payload).tag,
            boundary::TAG_SUCCESS
        );
    }

    #[cfg(feature = "ffi-test-hooks")]
    #[test]
    fn contained_panic_poisoning_preserves_close() {
        let directory = tempfile::tempdir().unwrap();
        let path = path_bytes(&directory.path().join("panic.tsm"));
        let database = unsafe { tosumu_experimental_v1_database_create(path.as_ptr(), path.len()) };
        let panic_result = tosumu_experimental_v1_test_inject_database_panic(database.payload);
        assert_eq!(panic_result.tag, boundary::TAG_BOUNDARY_FAILURE);
        assert_eq!(panic_result.status, boundary::BOUNDARY_PANIC);

        let get =
            unsafe { tosumu_experimental_v1_database_get(database.payload, std::ptr::null(), 0) };
        assert_eq!(get.status, boundary::BOUNDARY_POISONED);
        assert_eq!(
            tosumu_experimental_v1_database_close(database.payload).tag,
            boundary::TAG_SUCCESS
        );
    }
}
