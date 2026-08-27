use super::*;

fn page_write(pgno: u64, marker: u8) -> WalRecord {
    let mut frame = Box::new([0u8; PAGE_SIZE]);
    frame[0] = marker;
    WalRecord::PageWrite {
        pgno,
        page_version: 1,
        frame,
    }
}

fn committed_pages(records: &[(u64, WalRecord)]) -> Vec<(u64, u8)> {
    let mut pages = Vec::new();
    for_each_committed_transaction(records, |_, _, transaction_records| {
        for (_, record) in transaction_records {
            if let WalRecord::PageWrite { pgno, frame, .. } = record {
                pages.push((*pgno, frame[0]));
            }
        }
        Ok(())
    })
    .unwrap();
    pages
}

#[test]
fn matching_commit_publishes_only_its_sequential_transaction() {
    let records = vec![
        (1, WalRecord::Begin { txn_id: 7 }),
        (2, page_write(1, 0x11)),
        (3, WalRecord::Commit { txn_id: 7 }),
        (4, WalRecord::Begin { txn_id: 8 }),
        (5, page_write(2, 0x22)),
    ];

    assert_eq!(committed_pages(&records), vec![(1, 0x11)]);
}

#[test]
fn reused_transaction_id_does_not_commit_a_later_incomplete_transaction() {
    let records = vec![
        (1, WalRecord::Begin { txn_id: 7 }),
        (2, page_write(1, 0x11)),
        (3, WalRecord::Commit { txn_id: 7 }),
        (4, WalRecord::Begin { txn_id: 7 }),
        (5, page_write(2, 0x22)),
    ];

    assert_eq!(committed_pages(&records), vec![(1, 0x11)]);
}

#[test]
fn mismatched_commit_does_not_publish_active_frames() {
    let records = vec![
        (1, WalRecord::Begin { txn_id: 7 }),
        (2, page_write(1, 0x11)),
        (3, WalRecord::Commit { txn_id: 8 }),
    ];

    assert!(committed_pages(&records).is_empty());
}

#[test]
fn nested_begin_abandons_the_incomplete_outer_sequence() {
    let records = vec![
        (1, WalRecord::Begin { txn_id: 7 }),
        (2, page_write(1, 0x11)),
        (3, WalRecord::Begin { txn_id: 8 }),
        (4, page_write(2, 0x22)),
        (5, WalRecord::Commit { txn_id: 8 }),
    ];

    assert_eq!(committed_pages(&records), vec![(2, 0x22)]);
}
