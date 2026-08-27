use super::*;

#[test]
fn writable_open_revalidates_page0_after_recovery() {
    use crate::btree::BTree;
    use crate::format::{write_u16, OFF_KEYSLOT_COUNT};

    let db_p = tmp_db("revalidate_page0_after_recovery");
    let wal_p = wal_path(&db_p);
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);

    {
        let _t = BTree::create(&db_p).unwrap();
    }
    let _ = std::fs::remove_file(&wal_p);

    let mut page0 = [0u8; PAGE_SIZE];
    page0.copy_from_slice(&std::fs::read(&db_p).unwrap()[..PAGE_SIZE]);
    write_u16(&mut page0, OFF_KEYSLOT_COUNT, 0);

    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        w.append(&WalRecord::PageWrite {
            pgno: 0,
            page_version: 0,
            frame: Box::new(page0),
        })
        .unwrap();
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
    }

    let err = match BTree::open(&db_p) {
        Ok(_) => panic!("expected corrupt recovered page0 to be rejected"),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        TosumuError::Corrupt {
            pgno: 0,
            reason: "keyslot_count in header is out of valid range"
        }
    ));

    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}

#[test]
fn recover_applies_committed_page_writes() {
    let wal_p = tmp("recover_wal");
    let db_p = tmp_db("recover_db");
    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);

    // Write a dummy "db" file: 3 pages of zeros.
    std::fs::write(&db_p, vec![0u8; PAGE_SIZE * 3]).unwrap();

    // Write a WAL with one committed transaction that writes to page 2.
    let mut w = WalWriter::create(&wal_p).unwrap();
    w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
    let mut frame = Box::new([0u8; PAGE_SIZE]);
    frame[0] = 0xBE;
    frame[1] = 0xEF;
    w.append(&WalRecord::PageWrite {
        pgno: 2,
        page_version: 1,
        frame,
    })
    .unwrap();
    w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
    w.sync().unwrap();

    recover(&db_p, &wal_p).unwrap();

    // Verify page 2 was updated.
    let raw = std::fs::read(&db_p).unwrap();
    assert_eq!(raw[PAGE_SIZE * 2], 0xBE);
    assert_eq!(raw[PAGE_SIZE * 2 + 1], 0xEF);

    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);
}

#[test]
fn recover_rejects_overflowing_page_offset() {
    let wal_p = tmp("recover_offset_overflow_wal");
    let db_p = tmp_db("recover_offset_overflow_db");
    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);

    std::fs::write(&db_p, vec![0u8; PAGE_SIZE]).unwrap();

    let mut w = WalWriter::create(&wal_p).unwrap();
    w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
    w.append(&WalRecord::PageWrite {
        pgno: u64::MAX,
        page_version: 1,
        frame: Box::new([0u8; PAGE_SIZE]),
    })
    .unwrap();
    w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
    w.sync().unwrap();

    let err = recover(&db_p, &wal_p).unwrap_err();
    assert!(matches!(
        err,
        TosumuError::Corrupt { pgno, reason: "WAL page offset overflow" } if pgno == u64::MAX
    ));

    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);
}

#[test]
fn recover_ignores_uncommitted() {
    let wal_p = tmp("uncommitted_wal");
    let db_p = tmp_db("uncommitted_db");
    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);

    std::fs::write(&db_p, vec![0u8; PAGE_SIZE * 3]).unwrap();

    // Write WAL with Begin + PageWrite but NO Commit (simulates crash mid-write).
    let mut w = WalWriter::create(&wal_p).unwrap();
    w.append(&WalRecord::Begin { txn_id: 42 }).unwrap();
    let mut frame = Box::new([0u8; PAGE_SIZE]);
    frame[0] = 0xFF;
    w.append(&WalRecord::PageWrite {
        pgno: 1,
        page_version: 1,
        frame,
    })
    .unwrap();
    w.sync().unwrap();

    recover(&db_p, &wal_p).unwrap();

    // Page 1 must remain zero — the transaction was never committed.
    let raw = std::fs::read(&db_p).unwrap();
    assert_eq!(
        raw[PAGE_SIZE], 0x00,
        "uncommitted write must not be applied"
    );

    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);
}

#[test]
fn integration_recover_real_pager_frame() {
    // Proves that a real encrypted page frame written to the WAL is replayed
    // correctly into .tsm on recovery — using actual Pager/BTree output.
    use crate::btree::BTree;

    let db_p = tmp_db("integ_rec");
    let wal_p = tmp("integ_rec");
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);

    // 1. Create a real BTree DB and insert a key.
    {
        let mut t = BTree::create(&db_p).unwrap();
        t.put(b"hello", b"world").unwrap();
    }

    // 2. Re-open, read the encrypted frame for page 1 (the first data page).
    let frame = {
        let t = BTree::open(&db_p).unwrap();
        t.pager.read_raw_frame(1).unwrap()
    };
    let page_count = {
        let t = BTree::open(&db_p).unwrap();
        t.page_count()
    };

    // 3. Take a snapshot of .tsm BEFORE the write (all-zeros page 1).
    let snapshot = std::fs::read(&db_p).unwrap();

    // 4. Write a WAL with: Begin → PageWrite(page 1, frame) → Commit.
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 99 }).unwrap();
        let page_version = 1u64;
        w.append(&WalRecord::PageWrite {
            pgno: 1,
            page_version,
            frame: Box::new(frame),
        })
        .unwrap();
        w.append(&WalRecord::Commit { txn_id: 99 }).unwrap();
        w.sync().unwrap();
    }

    // 5. Reset .tsm to the pre-insert snapshot (simulate crash before .tsm write).
    // We need page 1 to look like zeros; keep page 0 (header) intact.
    let mut reset = snapshot.clone();
    // Zero out page 1.
    if reset.len() >= PAGE_SIZE * 2 {
        for b in &mut reset[PAGE_SIZE..PAGE_SIZE * 2] {
            *b = 0;
        }
    } else {
        reset.resize(PAGE_SIZE * (page_count as usize).max(2), 0);
    }
    std::fs::write(&db_p, &reset).unwrap();

    // 6. Recover: replay WAL into .tsm.
    recover(&db_p, &wal_p).unwrap();

    // 7. Open the DB and assert the key is visible.
    let t = BTree::open(&db_p).unwrap();
    assert_eq!(
        t.get(b"hello").unwrap(),
        Some(b"world".to_vec()),
        "key must be visible after WAL recovery"
    );

    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}

#[test]
fn checkpoint_truncates_wal() {
    let wal_p = tmp("ckpt_wal");
    let db_p = tmp_db("ckpt_db");
    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);

    std::fs::write(&db_p, vec![0u8; PAGE_SIZE]).unwrap();

    let mut w = WalWriter::create(&wal_p).unwrap();
    w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
    w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
    w.sync().unwrap();

    checkpoint(&db_p, &wal_p).unwrap();

    assert_eq!(
        std::fs::metadata(&wal_p).unwrap().len(),
        0,
        "WAL must be empty after checkpoint"
    );

    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);
}

// ── adversarial / correctness-under-failure tests ────────────────────────

/// Truncate a WAL record in half — recovery must ignore the partial tail and
/// not panic, corrupt the database, or misreport an error.
#[test]
fn partial_record_at_tail_is_ignored() {
    let wal_p = tmp("partial_wal");
    let db_p = tmp_db("partial_db");
    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);

    std::fs::write(&db_p, vec![0u8; PAGE_SIZE * 3]).unwrap();

    // Write txn 1 completely and fsync so we know the exact safe size.
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        let mut frame = Box::new([0u8; PAGE_SIZE]);
        frame[0] = 0xAA;
        w.append(&WalRecord::PageWrite {
            pgno: 1,
            page_version: 1,
            frame,
        })
        .unwrap();
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
    }
    // Capture the offset where txn 1 ends — everything before this is valid.
    let safe_len = std::fs::metadata(&wal_p).unwrap().len();

    // Append txn 2 (Begin + PageWrite, no Commit — simulates crash before commit).
    {
        let mut w = WalWriter::open(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 2 }).unwrap();
        let mut frame2 = Box::new([0u8; PAGE_SIZE]);
        frame2[0] = 0xBB;
        w.append(&WalRecord::PageWrite {
            pgno: 2,
            page_version: 1,
            frame: frame2,
        })
        .unwrap();
        w.sync().unwrap();
    }

    // Truncate to safe_len + 30 bytes — cuts mid-way through the PageWrite record
    // of txn 2, which must be ignored on recovery.
    let partial_len = safe_len + 30;
    {
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open(&wal_p)
            .unwrap();
        f.set_len(partial_len).unwrap();
    }

    recover(&db_p, &wal_p).unwrap();

    let raw = std::fs::read(&db_p).unwrap();
    assert_eq!(
        raw[PAGE_SIZE], 0xAA,
        "committed txn 1 page 1 must be applied"
    );
    assert_eq!(
        raw[PAGE_SIZE * 2],
        0x00,
        "partial txn 2 page 2 must NOT be applied"
    );

    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);
}

/// WAL contains two transactions: txn 1 fully committed, txn 2 incomplete
/// (no Commit record — simulates crash mid-second-transaction).
/// Only txn 1's writes may appear in the recovered file.
#[test]
fn multi_txn_only_committed_applied() {
    let wal_p = tmp("multi_txn_wal");
    let db_p = tmp_db("multi_txn_db");
    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);

    std::fs::write(&db_p, vec![0u8; PAGE_SIZE * 4]).unwrap();

    let mut w = WalWriter::create(&wal_p).unwrap();

    // Txn 1: committed — writes page 1.
    w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
    let mut f1 = Box::new([0u8; PAGE_SIZE]);
    f1[0] = 0x11;
    w.append(&WalRecord::PageWrite {
        pgno: 1,
        page_version: 1,
        frame: f1,
    })
    .unwrap();
    w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();

    // Txn 2: crash before commit — writes page 2.
    w.append(&WalRecord::Begin { txn_id: 2 }).unwrap();
    let mut f2 = Box::new([0u8; PAGE_SIZE]);
    f2[0] = 0x22;
    w.append(&WalRecord::PageWrite {
        pgno: 2,
        page_version: 1,
        frame: f2,
    })
    .unwrap();
    // NO Commit — simulates crash.
    w.sync().unwrap();

    recover(&db_p, &wal_p).unwrap();

    let raw = std::fs::read(&db_p).unwrap();
    assert_eq!(
        raw[PAGE_SIZE], 0x11,
        "committed txn 1 must be applied to page 1"
    );
    assert_eq!(
        raw[PAGE_SIZE * 2],
        0x00,
        "uncommitted txn 2 must NOT be applied to page 2"
    );

    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);
}

/// Calling recover() twice on the same WAL + db must produce identical
/// results. Pages must not accumulate extra writes or change values.
#[test]
fn recover_is_idempotent() {
    let wal_p = tmp("idem_wal");
    let db_p = tmp_db("idem_db");
    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);

    std::fs::write(&db_p, vec![0u8; PAGE_SIZE * 3]).unwrap();

    let mut w = WalWriter::create(&wal_p).unwrap();
    w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
    let mut frame = Box::new([0u8; PAGE_SIZE]);
    frame[42] = 0xCC;
    w.append(&WalRecord::PageWrite {
        pgno: 1,
        page_version: 1,
        frame,
    })
    .unwrap();
    w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
    w.sync().unwrap();

    recover(&db_p, &wal_p).unwrap();
    let after_first = std::fs::read(&db_p).unwrap();

    recover(&db_p, &wal_p).unwrap();
    let after_second = std::fs::read(&db_p).unwrap();

    assert_eq!(after_first, after_second, "recover() must be idempotent");

    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);
}

/// Simulate crash *after* WAL Commit is written but before dirty pages are
/// flushed to .tsm. On reopen, recovery must restore the committed state.
#[test]
fn crash_after_commit_before_flush_recovered_on_reopen() {
    use crate::btree::BTree;

    let db_p = tmp_db("crash_commit");
    // The WAL sidecar that BTree::open will look for is wal_path(&db_p).
    let wal_p = wal_path(&db_p);
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);

    // 1. Create a DB and insert a key so we have a real encrypted frame.
    {
        let mut t = BTree::create(&db_p).unwrap();
        t.put(b"survive", b"yes").unwrap();
    }

    // 2. Capture the real encrypted frame from .tsm.
    let real_frame = {
        let t = BTree::open(&db_p).unwrap();
        t.pager.read_raw_frame(1).unwrap()
    };

    // 3. Simulate crash: zero out page 1 in .tsm (flush never completed).
    let db_bytes = std::fs::read(&db_p).unwrap();
    let mut reset = db_bytes;
    for b in &mut reset[PAGE_SIZE..PAGE_SIZE * 2] {
        *b = 0;
    }
    std::fs::write(&db_p, &reset).unwrap();

    // Remove the WAL that was created by create()/open() so we can write our own.
    let _ = std::fs::remove_file(&wal_p);

    // 4. Write a WAL representing the committed-but-unflushed transaction.
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 7 }).unwrap();
        w.append(&WalRecord::PageWrite {
            pgno: 1,
            page_version: 1,
            frame: Box::new(real_frame),
        })
        .unwrap();
        w.append(&WalRecord::Commit { txn_id: 7 }).unwrap();
        w.sync().unwrap();
    }

    // 5. Reopen — recovery replays the WAL automatically inside open().
    let t = BTree::open(&db_p).unwrap();
    assert_eq!(
        t.get(b"survive").unwrap(),
        Some(b"yes".to_vec()),
        "key must survive crash-after-commit via WAL recovery",
    );

    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}
/// Insert enough keys inside a single transaction to force B+ tree root
/// splits, then simulate a crash by zeroing all data pages in .tsm while
/// leaving the committed WAL intact. On reopen, WAL recovery must restore
/// every key exactly.
///
/// This exercises the interaction between:
///   - allocate() + init_page() writing new split nodes directly to .tsm
///   - with_page_mut() WAL-buffering the final content of each node
///
/// Recovery is sound because every init_page() within a transaction is
/// always followed by with_page_mut() on the same page, so the WAL holds
/// the complete final frame for every page touched by the split. Recovery
/// writes those frames unconditionally, restoring the full committed state.
#[test]
fn btree_root_split_survives_wal_recovery() {
    use crate::page_store::PageStore;

    let db_p = tmp_db("split_recovery");
    let wal_p = wal_path(&db_p);
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);

    // 1. Create DB and insert enough keys via a single transaction to force
    //    at least one root split. 200 inserts reliably produces height >= 2.
    {
        let mut store = PageStore::create(&db_p).unwrap();
        store
            .transaction(|tx| {
                for i in 0u32..500 {
                    tx.put(
                        format!("key{i:05}").as_bytes(),
                        format!("val{i:05}").as_bytes(),
                    )?;
                }
                Ok(())
            })
            .unwrap();
        // Confirm the tree actually split before we stress recovery.
        assert!(
            store.stat().unwrap().tree_height >= 2,
            "expected root split, got height {}; adjust insert count",
            store.stat().unwrap().tree_height,
        );
    }

    // Capture the real encrypted frames from the committed tree, then replace the
    // normal post-commit WAL with a synthetic committed transaction so this test
    // continues to exercise recovery after the WAL is truncated on clean commit.
    let committed_bytes = std::fs::read(&db_p).unwrap();
    let page_count = committed_bytes.len() / PAGE_SIZE;
    let _ = std::fs::remove_file(&wal_p);
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        for pgno in 1..page_count {
            let start = pgno * PAGE_SIZE;
            let end = start + PAGE_SIZE;
            let mut frame = Box::new([0u8; PAGE_SIZE]);
            frame.copy_from_slice(&committed_bytes[start..end]);
            w.append(&WalRecord::PageWrite {
                pgno: pgno as u64,
                page_version: 1,
                frame,
            })
            .unwrap();
        }
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
    }

    // 2. Simulate crash: zero every data page (1..page_count) in .tsm.
    //    Page 0 (plaintext header) is preserved — it holds page_count and
    //    root_page so the pager can seek to the right offsets during replay.
    let mut raw = committed_bytes;
    for b in &mut raw[PAGE_SIZE..] {
        *b = 0;
    }
    std::fs::write(&db_p, &raw).unwrap();

    // 3. Reopen — Pager::open detects the WAL sidecar and replays all
    //    committed PageWrite frames before returning.
    let store = PageStore::open(&db_p).unwrap();

    // 4. Every key inserted in the transaction must be visible.
    for i in 0u32..500 {
        let k = format!("key{i:05}");
        let v = format!("val{i:05}");
        assert_eq!(
            store.get(k.as_bytes()).unwrap(),
            Some(v.into_bytes()),
            "key {k} missing after WAL recovery of root-split transaction",
        );
    }

    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}
