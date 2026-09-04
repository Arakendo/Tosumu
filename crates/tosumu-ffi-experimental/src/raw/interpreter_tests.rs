use super::*;

fn insert_bytes(bytes: Vec<u8>) -> u64 {
    match boundary::insert_bytes(bytes) {
        Ok(handle) => handle,
        Err(CallFailure::Boundary(status)) => {
            panic!("focused interpreter test rejected the handle with status {status}")
        }
        Err(CallFailure::Core(error)) => {
            panic!("focused interpreter test rejected the handle: {error}")
        }
    }
}

#[test]
fn interpreter_checks_slice_copy_and_close_race_contract() {
    let source = b"interpreted bytes".to_vec();
    let borrowed = match unsafe { input(source.as_ptr(), source.len()) } {
        Ok(bytes) => bytes,
        Err(CallFailure::Boundary(status)) => {
            panic!("valid source input was rejected with status {status}")
        }
        Err(CallFailure::Core(error)) => panic!("valid source input was rejected: {error}"),
    };
    assert_eq!(borrowed, source);
    let empty = match unsafe { input(std::ptr::null(), 0) } {
        Ok(bytes) => bytes,
        Err(CallFailure::Boundary(status)) => {
            panic!("null zero-length input was rejected with status {status}")
        }
        Err(CallFailure::Core(error)) => {
            panic!("null zero-length input was rejected: {error}")
        }
    };
    assert!(empty.is_empty());

    let bytes = insert_bytes(source.clone());
    let mut destination = vec![0; source.len()];
    let copied = unsafe {
        tosumu_experimental_v1_bytes_copy(bytes, destination.as_mut_ptr(), destination.len())
    };
    assert_eq!(copied.tag, boundary::TAG_SUCCESS);
    assert_eq!(copied.payload, source.len() as u64);
    assert_eq!(destination, source);
    assert_eq!(
        tosumu_experimental_v1_bytes_close(bytes).tag,
        boundary::TAG_SUCCESS
    );

    let raced = insert_bytes(b"race".to_vec());
    let barrier = std::sync::Arc::new(std::sync::Barrier::new(3));
    let reader_barrier = std::sync::Arc::clone(&barrier);
    let reader = std::thread::spawn(move || {
        reader_barrier.wait();
        tosumu_experimental_v1_bytes_length(raced)
    });
    let closer_barrier = std::sync::Arc::clone(&barrier);
    let closer = std::thread::spawn(move || {
        closer_barrier.wait();
        tosumu_experimental_v1_bytes_close(raced)
    });
    barrier.wait();
    let read = reader.join().unwrap();
    let closed = closer.join().unwrap();
    assert_eq!(closed.tag, boundary::TAG_SUCCESS);
    assert!(
        (read.tag == boundary::TAG_SUCCESS && read.payload == 4)
            || (read.tag == boundary::TAG_BOUNDARY_FAILURE
                && read.status == boundary::BOUNDARY_INVALID_HANDLE)
    );
    assert_eq!(
        tosumu_experimental_v1_bytes_length(raced).status,
        boundary::BOUNDARY_INVALID_HANDLE
    );
}
