use tosumu_core::{SharedKvStore, TosumuError};

#[test]
fn public_caller_conserves_a_snapshot_across_bounded_pages() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("bounded-snapshot-scan.tsm");
    let database = SharedKvStore::create(&path).unwrap();

    database
        .write(|transaction| {
            for index in 0..160 {
                let key = format!("key-{index:04}");
                let value = vec![index as u8; 41 + index % 37];
                transaction.put(key.as_bytes(), &value)?;
            }
            Ok(())
        })
        .unwrap();

    let snapshot = database.snapshot().unwrap();
    database.put(b"key-0042", b"committed later").unwrap();
    database
        .put(b"key-0200", b"outside captured generation")
        .unwrap();

    let lower = b"key-0010";
    let upper = b"key-0149";
    let expected = snapshot.scan(lower, upper).unwrap();
    let mut actual = Vec::new();
    let mut start = lower.to_vec();
    let mut page_count = 0;

    loop {
        let page = snapshot.scan_page(&start, upper, 9, 2_048).unwrap();
        assert!(page.pairs.len() <= 9);
        assert!(page.blocked_entry_payload_bytes.is_none());
        actual.extend(page.pairs);
        page_count += 1;

        match page.next_start_inclusive {
            Some(next) => start = next,
            None => break,
        }
    }

    assert!(page_count > 2);
    assert_eq!(actual, expected);
    assert_eq!(
        actual
            .iter()
            .find(|(key, _)| key == b"key-0042")
            .map(|(_, value)| value.as_slice()),
        Some(vec![42; 41 + 42 % 37].as_slice())
    );
}

#[test]
fn public_caller_can_make_progress_after_a_byte_blocked_entry() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("byte-blocked-snapshot-scan.tsm");
    let database = SharedKvStore::create(&path).unwrap();
    let key = b"large";
    let value = vec![0x5a; 20_000];
    database.put(key, &value).unwrap();
    let snapshot = database.snapshot().unwrap();

    let blocked = snapshot.scan_page(key, key, 1, 100).unwrap();
    let required = (key.len() + value.len()) as u64;
    assert!(blocked.pairs.is_empty());
    assert_eq!(
        blocked.next_start_inclusive.as_deref(),
        Some(key.as_slice())
    );
    assert_eq!(blocked.blocked_entry_payload_bytes, Some(required));

    let admitted = snapshot.scan_page(key, key, 1, required).unwrap();
    assert_eq!(admitted.pairs, vec![(key.to_vec(), value)]);
    assert_eq!(admitted.next_start_inclusive, None);
    assert_eq!(admitted.blocked_entry_payload_bytes, None);
}

#[test]
fn public_caller_gets_typed_invalid_limit_and_range_failures() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("invalid-bounded-snapshot-scan.tsm");
    let database = SharedKvStore::create(&path).unwrap();
    let snapshot = database.snapshot().unwrap();

    for result in [
        snapshot.scan_page(b"a", b"z", 0, 1),
        snapshot.scan_page(b"a", b"z", 1, 0),
        snapshot.scan_page(b"z", b"a", 1, 1),
    ] {
        assert!(matches!(
            result,
            Err(TosumuError::InvalidArgument(
                "bounded scan requires positive limits and start <= end"
            ))
        ));
    }
}
