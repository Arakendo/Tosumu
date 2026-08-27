use super::*;

// ── WAL append crash invariant tests ─────────────────────────────────────
//
// Invariant: recovery must bring the database to either the old clean state
// or the new fully-committed state — never a partial transaction.

/// Create a baseline BTree at the given path with a single "base_key".
fn baseline_btree(db_p: &PathBuf, wal_p: &PathBuf) {
    use crate::btree::BTree;
    let _ = std::fs::remove_file(db_p);
    let _ = std::fs::remove_file(wal_p);
    let mut t = BTree::create(db_p).unwrap();
    t.put(b"base_key", b"base_val").unwrap();
    // Remove the WAL sidecar created by BTree::create so we start fresh.
    let _ = std::fs::remove_file(wal_p);
}

/// Assert recovery leaves old state intact (base_key visible, new_key absent)
/// and that structural invariants pass.
fn assert_old_state(db_p: &Path) {
    use crate::btree::BTree;
    let t = BTree::open(db_p).unwrap();
    assert_eq!(
        t.get(b"base_key").unwrap(),
        Some(b"base_val".to_vec()),
        "base_key must survive uncommitted crash scenario"
    );
    assert_eq!(
        t.get(b"new_key").unwrap(),
        None,
        "new_key from uncommitted txn must not appear after recovery"
    );
    t.check_invariants().unwrap();
}

#[test]
fn crash_mid_page_write_old_state_preserved() {
    // Scenario: Begin written, PageWrite truncated mid-record (CRC never reached).
    // Recovery: partial tail silently ignored, no Commit → old state.
    let db_p = tmp_db("crash_mid_pw");
    let wal_p = wal_path(&db_p);
    baseline_btree(&db_p, &wal_p);
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        let after_begin = std::fs::metadata(&wal_p).unwrap().len();
        w.append(&WalRecord::PageWrite {
            pgno: 1,
            page_version: 1,
            frame: Box::new([0u8; PAGE_SIZE]),
        })
        .unwrap();
        w.sync().unwrap();
        // Truncate 30 bytes into the PageWrite record (well before its CRC).
        let f = OpenOptions::new().write(true).open(&wal_p).unwrap();
        f.set_len(after_begin + 30).unwrap();
    }
    assert_old_state(&db_p);
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}

#[test]
fn crash_complete_page_write_no_commit_old_state_preserved() {
    // Scenario: Begin + complete PageWrite, but no Commit record.
    // Recovery: sees Begin+PageWrite but no Commit → uncommitted → old state.
    let db_p = tmp_db("crash_no_commit");
    let wal_p = wal_path(&db_p);
    baseline_btree(&db_p, &wal_p);
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        w.append(&WalRecord::PageWrite {
            pgno: 1,
            page_version: 1,
            frame: Box::new([0u8; PAGE_SIZE]),
        })
        .unwrap();
        // Intentionally omit Commit.
        w.sync().unwrap();
    }
    assert_old_state(&db_p);
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}

#[test]
fn crash_mid_commit_record_old_state_preserved() {
    // Scenario: Begin + PageWrite + 10 bytes of Commit (torn write, CRC absent).
    // Recovery: partial Commit at tail is silently ignored → txn not committed → old state.
    let db_p = tmp_db("crash_mid_commit");
    let wal_p = wal_path(&db_p);
    baseline_btree(&db_p, &wal_p);
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        w.append(&WalRecord::PageWrite {
            pgno: 1,
            page_version: 1,
            frame: Box::new([0u8; PAGE_SIZE]),
        })
        .unwrap();
        let after_page_write = std::fs::metadata(&wal_p).unwrap().len();
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
        // Truncate 10 bytes into the Commit record (lsn+type+partial len — CRC absent).
        let f = OpenOptions::new().write(true).open(&wal_p).unwrap();
        f.set_len(after_page_write + 10).unwrap();
    }
    assert_old_state(&db_p);
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}

// ── Checkpoint WAL-invariant test ─────────────────────────────────────────

#[test]
fn checkpoint_wal_not_truncated_when_apply_fails() {
    // Named invariant: "WAL is never truncated unless apply succeeded."
    //
    // Proof by construction: checkpoint() calls recover() first; if recover()
    // fails (here: db file does not exist), set_len(0) is never reached.
    let db_p = tmp_db("ckpt_inv");
    let wal_p = wal_path(&db_p);
    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
    // Write a valid committed transaction into the WAL.
    {
        let mut w = WalWriter::create(&wal_p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        w.append(&WalRecord::PageWrite {
            pgno: 1,
            page_version: 1,
            frame: Box::new([0xABu8; PAGE_SIZE]),
        })
        .unwrap();
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
    }
    let wal_size_before = std::fs::metadata(&wal_p).unwrap().len();
    // db_p does not exist → recover() fails → checkpoint() must propagate the error.
    let result = checkpoint(&db_p, &wal_p);
    assert!(
        result.is_err(),
        "checkpoint must fail when db file is missing"
    );
    // WAL must be unchanged — it was not truncated.
    let wal_size_after = std::fs::metadata(&wal_p).unwrap().len();
    assert_eq!(
        wal_size_before, wal_size_after,
        "WAL must not be truncated when recover() failed"
    );
    let _ = std::fs::remove_file(&wal_p);
}

#[test]
fn checkpoint_truncate_crash_preserves_recovered_db_and_leaves_wal_intact() {
    use crate::btree::BTree;

    let db_p = tmp_db("ckpt_truncate_crash");
    let wal_p = wal_path(&db_p);
    baseline_btree(&db_p, &wal_p);

    let real_frame = {
        let t = BTree::open(&db_p).unwrap();
        t.pager.read_raw_frame(1).unwrap()
    };
    let _ = std::fs::remove_file(&wal_p);

    {
        let mut raw = std::fs::read(&db_p).unwrap();
        for b in &mut raw[PAGE_SIZE..PAGE_SIZE * 2] {
            *b = 0;
        }
        std::fs::write(&db_p, &raw).unwrap();
    }

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

    let wal_size_before = std::fs::metadata(&wal_p).unwrap().len();

    recover(&db_p, &wal_p).unwrap();

    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&wal_p)
        .unwrap();
    let mut crash_file = CrashFile::new(file, CrashPhase::DuringTruncate);
    let err = truncate_wal_after_checkpoint(&mut crash_file).unwrap_err();
    assert!(matches!(err, TosumuError::Io(io) if io.kind() == ErrorKind::BrokenPipe));

    let wal_size_after = std::fs::metadata(&wal_p).unwrap().len();
    assert_eq!(
        wal_size_before, wal_size_after,
        "WAL must remain intact when truncate crashes after recovery succeeded"
    );

    let t = BTree::open(&db_p).unwrap();
    assert_eq!(t.get(b"base_key").unwrap(), Some(b"base_val".to_vec()));

    let _ = std::fs::remove_file(&db_p);
    let _ = std::fs::remove_file(&wal_p);
}
