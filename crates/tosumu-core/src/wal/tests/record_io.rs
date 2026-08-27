use super::*;

#[test]
fn write_and_read_all_record_types() {
    let p = tmp("all_types");
    let _ = std::fs::remove_file(&p);

    let mut writer = WalWriter::create(&p).unwrap();
    writer.append(&WalRecord::Begin { txn_id: 1 }).unwrap();

    let mut frame = Box::new([0u8; PAGE_SIZE]);
    frame[0] = 0xAB;
    writer
        .append(&WalRecord::PageWrite {
            pgno: 5,
            page_version: 7,
            frame,
        })
        .unwrap();
    writer.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
    writer
        .append(&WalRecord::Checkpoint { up_to_lsn: 3 })
        .unwrap();
    writer.sync().unwrap();

    let records = WalReader::read_all(&p).unwrap();
    assert_eq!(records.len(), 4);

    assert!(matches!(records[0].1, WalRecord::Begin { txn_id: 1 }));
    if let WalRecord::PageWrite {
        pgno,
        page_version,
        ref frame,
    } = records[1].1
    {
        assert_eq!(pgno, 5);
        assert_eq!(page_version, 7);
        assert_eq!(frame[0], 0xAB);
    } else {
        panic!("expected PageWrite");
    }
    assert!(matches!(records[2].1, WalRecord::Commit { txn_id: 1 }));
    assert!(matches!(
        records[3].1,
        WalRecord::Checkpoint { up_to_lsn: 3 }
    ));

    let _ = std::fs::remove_file(&p);
}

#[test]
fn lsn_increments() {
    let p = tmp("lsn");
    let _ = std::fs::remove_file(&p);

    let mut w = WalWriter::create(&p).unwrap();
    let l1 = w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
    let l2 = w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
    assert_eq!(l1, 1);
    assert_eq!(l2, 2);
    w.sync().unwrap();

    let records = WalReader::read_all(&p).unwrap();
    assert_eq!(records[0].0, 1);
    assert_eq!(records[1].0, 2);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn open_continues_lsn() {
    let p = tmp("open_lsn");
    let _ = std::fs::remove_file(&p);

    {
        let mut w = WalWriter::create(&p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
    }

    let mut w2 = WalWriter::open(&p).unwrap();
    assert_eq!(w2.next_lsn(), 3);
    let l3 = w2.append(&WalRecord::Begin { txn_id: 2 }).unwrap();
    assert_eq!(l3, 3);
    w2.sync().unwrap();

    let records = WalReader::read_all(&p).unwrap();
    assert_eq!(records.len(), 3);

    let _ = std::fs::remove_file(&p);
}

#[test]
fn wal_writer_open_truncates_partial_tail_before_append() {
    let p = tmp("open_truncates_partial_tail");
    let _ = std::fs::remove_file(&p);

    {
        let mut w = WalWriter::create(&p).unwrap();
        w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
        w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
        w.sync().unwrap();
    }
    let safe_len = std::fs::metadata(&p).unwrap().len();

    {
        let mut w = WalWriter::open(&p).unwrap();
        let mut frame = Box::new([0u8; PAGE_SIZE]);
        frame[0] = 0xAA;
        w.append(&WalRecord::Begin { txn_id: 2 }).unwrap();
        let last_complete_len = std::fs::metadata(&p).unwrap().len();
        w.append(&WalRecord::PageWrite {
            pgno: 9,
            page_version: 1,
            frame,
        })
        .unwrap();
        w.sync().unwrap();
        drop(w);

        let f = OpenOptions::new().write(true).open(&p).unwrap();
        f.set_len(safe_len + 30).unwrap();
        drop(f);

        let mut w = WalWriter::open(&p).unwrap();
        assert_eq!(std::fs::metadata(&p).unwrap().len(), last_complete_len);
        let lsn = w.append(&WalRecord::Begin { txn_id: 3 }).unwrap();
        assert_eq!(lsn, 4);
        w.sync().unwrap();
    }

    let records = WalReader::read_all(&p).unwrap();
    assert_eq!(records.len(), 4);
    assert!(matches!(records[2].1, WalRecord::Begin { txn_id: 2 }));
    assert!(matches!(records[3].1, WalRecord::Begin { txn_id: 3 }));

    let _ = std::fs::remove_file(&p);
}

#[test]
fn wal_writer_open_rejects_crc_corruption_before_append() {
    let p = tmp("open_crc_corrupt");
    let _ = std::fs::remove_file(&p);

    let mut w = WalWriter::create(&p).unwrap();
    w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
    w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
    w.sync().unwrap();

    let mut raw = std::fs::read(&p).unwrap();
    let second_record_start = RECORD_HEADER_SIZE + 8;
    raw[second_record_start + 5] ^= 0xFF;
    std::fs::write(&p, &raw).unwrap();

    let err = match WalWriter::open(&p) {
        Ok(_) => panic!("expected CRC-corrupt WAL open to fail"),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        TosumuError::CorruptRecord { offset, reason: "WAL record CRC mismatch" } if offset > 0
    ));

    let _ = std::fs::remove_file(&p);
}

#[test]
fn crc_corruption_is_reported() {
    let p = tmp("crc");
    let _ = std::fs::remove_file(&p);

    let mut w = WalWriter::create(&p).unwrap();
    w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
    w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
    w.sync().unwrap();

    // Flip a byte in the second record's payload.
    let mut raw = std::fs::read(&p).unwrap();
    let second_record_start = RECORD_HEADER_SIZE + 8; // first record: 17 hdr + 8 payload
    if raw.len() > second_record_start + 5 {
        raw[second_record_start + 5] ^= 0xFF;
    }
    std::fs::write(&p, &raw).unwrap();

    let err = WalReader::read_all(&p).unwrap_err();
    assert!(matches!(
        err,
        TosumuError::CorruptRecord { offset, reason: "WAL record CRC mismatch" } if offset > 0
    ));

    let _ = std::fs::remove_file(&p);
}

#[test]
fn recover_reports_mid_log_crc_corruption() {
    let wal_p = tmp("recover_crc_corrupt_wal");
    let db_p = tmp_db("recover_crc_corrupt_db");
    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);

    std::fs::write(&db_p, vec![0u8; PAGE_SIZE]).unwrap();

    let mut w = WalWriter::create(&wal_p).unwrap();
    w.append(&WalRecord::Begin { txn_id: 1 }).unwrap();
    w.append(&WalRecord::Commit { txn_id: 1 }).unwrap();
    w.append(&WalRecord::Begin { txn_id: 2 }).unwrap();
    w.append(&WalRecord::Commit { txn_id: 2 }).unwrap();
    w.sync().unwrap();

    let mut raw = std::fs::read(&wal_p).unwrap();
    let second_record_start = RECORD_HEADER_SIZE + 8;
    raw[second_record_start + 5] ^= 0xFF;
    std::fs::write(&wal_p, &raw).unwrap();

    let err = recover(&db_p, &wal_p).unwrap_err();
    assert!(matches!(
        err,
        TosumuError::CorruptRecord { offset, reason: "WAL record CRC mismatch" } if offset > 0
    ));

    let _ = std::fs::remove_file(&wal_p);
    let _ = std::fs::remove_file(&db_p);
}

#[test]
fn next_record_rejects_oversized_payload_len_before_allocation() {
    let p = tmp("oversized_payload_reader");
    let _ = std::fs::remove_file(&p);

    let mut raw = Vec::new();
    raw.extend_from_slice(&1u64.to_le_bytes());
    raw.push(RT_BEGIN);
    raw.extend_from_slice(&u32::MAX.to_le_bytes());
    std::fs::write(&p, raw).unwrap();

    let mut rdr = WalReader::open(&p).unwrap();
    let err = rdr.next_record().unwrap_err();
    assert!(matches!(
        err,
        TosumuError::CorruptRecord {
            offset: 0,
            reason: "WAL payload_len out of range"
        }
    ));

    let _ = std::fs::remove_file(&p);
}

#[test]
fn wal_writer_open_rejects_oversized_payload_len_before_allocation() {
    let p = tmp("oversized_payload_open");
    let _ = std::fs::remove_file(&p);

    let mut raw = Vec::new();
    raw.extend_from_slice(&1u64.to_le_bytes());
    raw.push(RT_BEGIN);
    raw.extend_from_slice(&u32::MAX.to_le_bytes());
    std::fs::write(&p, raw).unwrap();

    let err = match WalWriter::open(&p) {
        Ok(_) => panic!("expected oversized payload_len to be rejected"),
        Err(err) => err,
    };
    assert!(matches!(
        err,
        TosumuError::CorruptRecord {
            offset: 0,
            reason: "WAL payload_len out of range"
        }
    ));

    let _ = std::fs::remove_file(&p);
}
