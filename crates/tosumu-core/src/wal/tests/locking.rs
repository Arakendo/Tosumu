use super::*;

// ── Lock-retry / FileBusy tests ───────────────────────────────────────────

/// After all retry attempts are exhausted with injected lock errors,
/// `recover()` must return `TosumuError::FileBusy` and leave both the
/// database file and the WAL sidecar byte-for-byte unchanged.
///
/// This verifies the invariant: lock errors are not corruption.
/// A failed recovery leaves files intact so the next `open()` can retry.
#[test]
fn recovery_returns_file_busy_after_exhausted_retries() {
    let _fi_lock = fault_injection::LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let _cleanup = FaultGuard;

    let db_p = tmp_db("fi_file_busy");
    let wal_p = tmp("fi_file_busy");
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);

    std::fs::write(&db_p, vec![0u8; PAGE_SIZE * 2]).unwrap();
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        let mut frame = Box::new([0u8; PAGE_SIZE]);
        frame[0] = 0xAB;
        w.append(&WalRecord::PageWrite {
            pgno: 1,
            page_version: 1,
            frame,
        })
        .unwrap();
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
    }

    let db_before = std::fs::read(&db_p).unwrap();
    let wal_before = std::fs::read(&wal_p).unwrap();

    // Exhaust all MAX_RETRIES+1 attempts.
    fault_injection::arm(MAX_RETRIES + 1);

    let err = recover(&db_p, &wal_p).unwrap_err();
    assert!(
        matches!(err, TosumuError::FileBusy { .. }),
        "expected FileBusy, got {err:?}",
    );

    // Both files must be byte-for-byte unchanged — no partial application.
    assert_eq!(
        std::fs::read(&db_p).unwrap(),
        db_before,
        "database must be unchanged"
    );
    assert_eq!(
        std::fs::read(&wal_p).unwrap(),
        wal_before,
        "WAL must be unchanged"
    );

    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}

/// When the injected fault count is fewer than MAX_RETRIES, `recover()`
/// retries successfully and applies the committed writes.
#[test]
fn recovery_retries_and_succeeds_after_transient_faults() {
    let _fi_lock = fault_injection::LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let _cleanup = FaultGuard;

    let db_p = tmp_db("fi_retry_ok");
    let wal_p = tmp("fi_retry_ok");
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);

    std::fs::write(&db_p, vec![0u8; PAGE_SIZE * 2]).unwrap();
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        let mut frame = Box::new([0u8; PAGE_SIZE]);
        frame[0] = 0xCC;
        w.append(&WalRecord::PageWrite {
            pgno: 1,
            page_version: 1,
            frame,
        })
        .unwrap();
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
    }

    // Fewer faults than MAX_RETRIES — recovery retries and succeeds.
    fault_injection::arm(MAX_RETRIES - 1);
    recover(&db_p, &wal_p).expect("recovery must succeed after transient faults");

    let raw = std::fs::read(&db_p).unwrap();
    assert_eq!(
        raw[PAGE_SIZE], 0xCC,
        "committed write must be applied after retry recovery"
    );

    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}

/// `TosumuError::FileBusy` must carry the path of the locked file and a
/// non-empty operation description — not silently swallowed as `Corrupt`.
#[test]
fn file_busy_error_contains_path_and_operation() {
    let _fi_lock = fault_injection::LOCK
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let _cleanup = FaultGuard;

    let db_p = tmp_db("fi_path_check");
    let wal_p = tmp("fi_path_check");
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);

    std::fs::write(&db_p, vec![0u8; PAGE_SIZE]).unwrap();
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
    }

    fault_injection::arm(MAX_RETRIES + 1);

    // recover() opens the WAL first — FileBusy path must be the WAL path.
    match recover(&db_p, &wal_p).unwrap_err() {
        TosumuError::FileBusy { path, operation } => {
            assert_eq!(path, wal_p, "FileBusy must identify the locked file");
            assert_eq!(
                operation, "opening WAL for record replay",
                "operation must identify which step failed"
            );
        }
        other => panic!("expected FileBusy, got {other:?}"),
    }

    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}

/// Simulate an AV-scanner-style exclusive lock on the WAL sidecar using a
/// real Windows OS file lock (`FILE_SHARE_NONE`).  The background thread
/// holds the lock for 25 ms; with MAX_RETRIES × 10 ms = 50 ms total budget,
/// `Pager::open` should retry and ultimately succeed.
///
/// Run manually: `cargo test -- av_style_lock --ignored`
#[test]
#[cfg(windows)]
#[ignore = "requires Windows file-locking semantics; run manually"]
fn av_style_lock_during_recovery_retries_then_succeeds() {
    use crate::btree::BTree;
    use std::os::windows::fs::OpenOptionsExt;

    let db_p = tmp_db("av_lock");
    let wal_p = wal_path(&db_p);
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);

    // Create a real DB so we have a valid header and a real encrypted frame.
    {
        let mut t = BTree::create(&db_p).unwrap();
        t.put(b"av_key", b"av_val").unwrap();
    }
    let real_frame = {
        let t = BTree::open(&db_p).unwrap();
        t.pager.read_raw_frame(1).unwrap()
    };
    // Simulate crash: zero page 1, rebuild WAL manually.
    let mut raw = std::fs::read(&db_p).unwrap();
    for b in &mut raw[PAGE_SIZE..PAGE_SIZE * 2] {
        *b = 0;
    }
    std::fs::write(&db_p, &raw).unwrap();
    let _ = std::fs::remove_file(&wal_p);
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        w.append(&WalRecord::PageWrite {
            pgno: 1,
            page_version: 1,
            frame: Box::new(real_frame),
        })
        .unwrap();
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
    }

    // Hold an exclusive OS lock on the WAL for 25 ms from a background thread.
    let wal_clone = wal_p.clone();
    let lock_thread = std::thread::spawn(move || {
        let _locked = OpenOptions::new()
            .read(true)
            .share_mode(0) // FILE_SHARE_NONE — exclusive
            .open(&wal_clone)
            .expect("test setup: failed to acquire exclusive OS lock on WAL");
        std::thread::sleep(std::time::Duration::from_millis(25));
        // Lock released on drop.
    });

    // Give the lock thread a moment to acquire before recovery starts.
    std::thread::sleep(std::time::Duration::from_millis(2));

    // Pager::open triggers WAL recovery with retry — must survive the lock.
    let t = BTree::open(&db_p).unwrap();
    assert_eq!(
        t.get(b"av_key").unwrap(),
        Some(b"av_val".to_vec()),
        "key must be visible after AV-style transient OS lock + retry recovery",
    );

    lock_thread.join().unwrap();
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}
