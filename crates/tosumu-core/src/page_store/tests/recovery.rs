use super::*;

#[test]
fn transaction_commit_visible_after_reopen() {
    let path = temp_path("txn_commit");
    let _ = std::fs::remove_file(&path);
    // Remove the WAL sidecar too.
    let wal = std::path::PathBuf::from(format!("{}.wal", path.display()));
    let _ = std::fs::remove_file(&wal);

    {
        let mut store = PageStore::create(&path).unwrap();
        store
            .transaction(|tx| {
                tx.put(b"a", b"1")?;
                tx.put(b"b", b"2")?;
                Ok(())
            })
            .unwrap();
    }

    let store = PageStore::open(&path).unwrap();
    assert_eq!(store.get(b"a").unwrap(), Some(b"1".to_vec()));
    assert_eq!(store.get(b"b").unwrap(), Some(b"2".to_vec()));

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

#[test]
fn transaction_rollback_leaves_no_data() {
    let path = temp_path("txn_rollback");
    let _ = std::fs::remove_file(&path);
    let wal = std::path::PathBuf::from(format!("{}.wal", path.display()));
    let _ = std::fs::remove_file(&wal);

    let mut store = PageStore::create(&path).unwrap();
    let result: Result<()> = store.transaction(|tx| {
        tx.put(b"x", b"lost")?;
        Err(crate::error::TosumuError::InvalidArgument(
            "deliberate rollback",
        ))
    });
    assert!(result.is_err());
    assert_eq!(
        store.get(b"x").unwrap(),
        None,
        "rolled-back write must not be visible"
    );

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

#[test]
fn transaction_propagates_committed_but_flush_failed_and_recovers_on_reopen() {
    use crate::test_helpers::{CrashFile, CrashPhase};

    let path = temp_path("txn_flush_fail");
    let wal = diff_wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);

    let mut store = PageStore::create(&path).unwrap();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let mut crash_file = CrashFile::new(file, CrashPhase::AfterWrite);

    let err = store
        .transaction_with_crash_file(
            |tx| {
                tx.put(b"outer-a", b"1")?;
                tx.put(b"outer-b", b"2")?;
                Ok(())
            },
            &mut crash_file,
        )
        .unwrap_err();
    assert!(matches!(err, TosumuError::CommittedButFlushFailed { .. }));

    drop(store);

    let reopened = PageStore::open(&path).unwrap();
    assert_eq!(reopened.get(b"outer-a").unwrap(), Some(b"1".to_vec()));
    assert_eq!(reopened.get(b"outer-b").unwrap(), Some(b"2".to_vec()));

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

#[test]
fn large_overflow_transaction_recovers_after_commit_flush_failure() {
    recover_large_value_after_commit_flush_failure("one_megabyte", 1024 * 1024);
}

#[test]
#[ignore = "large Tokimu recovery evidence; run explicitly with --ignored"]
fn tokimu_large_value_recovery_evidence_matrix() {
    for (label, size) in [
        ("one_megabyte", 1024 * 1024),
        ("sixteen_megabyte", 16 * 1024 * 1024),
        ("sixty_four_megabyte", crate::format::MAX_VALUE_SIZE),
    ] {
        recover_large_value_after_commit_flush_failure(label, size);
    }
}

fn recover_large_value_after_commit_flush_failure(label: &str, size: usize) {
    use crate::test_helpers::{CrashFile, CrashPhase};
    use sha2::{Digest, Sha256};

    let path = temp_path(&format!("txn_large_flush_fail_{label}"));
    let wal = diff_wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);

    let large: Vec<u8> = (0u8..=255).cycle().take(size).collect();
    let expected_hash: [u8; 32] = Sha256::digest(&large).into();
    let mut store = PageStore::create(&path).unwrap();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let mut crash_file = CrashFile::new(file, CrashPhase::AfterWrite);

    let err = store
        .transaction_with_crash_file(
            |tx| {
                tx.put(b"manifest", b"schema-v1")?;
                tx.put(b"payload", &large)?;
                Ok(())
            },
            &mut crash_file,
        )
        .unwrap_err();
    assert!(matches!(err, TosumuError::CommittedButFlushFailed { .. }));

    drop(store);
    let reopened = PageStore::open(&path).unwrap();
    assert_eq!(
        reopened.get(b"manifest").unwrap(),
        Some(b"schema-v1".to_vec())
    );
    let recovered = reopened.get(b"payload").unwrap().unwrap();
    assert_eq!(Sha256::digest(&recovered).as_slice(), expected_hash);
    assert_eq!(recovered, large);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

#[test]
fn consumer_asset_create_recovers_as_one_committed_generation() {
    let path = temp_path("asset_create_recovery");
    let wal = diff_wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);

    let asset = AssetGeneration::new(1);
    let mut store = PageStore::create(&path).unwrap();
    let mut crash = crash_file(&path);
    let result = store.transaction_with_crash_file(
        |tx| {
            for (key, value) in asset.records() {
                tx.put(key, value)?;
            }
            Ok(())
        },
        &mut crash,
    );
    assert!(matches!(
        result,
        Err(TosumuError::CommittedButFlushFailed { .. })
    ));
    drop(crash);
    drop(store);

    let reopened = PageStore::open(&path).unwrap();
    assert_asset(&reopened, &asset);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

#[test]
fn consumer_asset_overwrite_recovers_as_new_generation_without_mixing() {
    let path = temp_path("asset_overwrite_recovery");
    let wal = diff_wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);

    let old_asset = AssetGeneration::new(1);
    let new_asset = AssetGeneration::new(2);
    let mut store = PageStore::create(&path).unwrap();
    commit_asset(&mut store, &old_asset);

    let mut crash = crash_file(&path);
    let result = store.transaction_with_crash_file(
        |tx| {
            for (key, value) in new_asset.records() {
                tx.put(key, value)?;
            }
            Ok(())
        },
        &mut crash,
    );
    assert!(matches!(
        result,
        Err(TosumuError::CommittedButFlushFailed { .. })
    ));
    drop(crash);
    drop(store);

    let reopened = PageStore::open(&path).unwrap();
    assert_asset(&reopened, &new_asset);
    assert_ne!(
        reopened.get(b"asset/manifest").unwrap(),
        Some(old_asset.manifest)
    );

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

#[test]
fn consumer_asset_delete_recovers_as_one_empty_generation() {
    let path = temp_path("asset_delete_recovery");
    let wal = diff_wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);

    let asset = AssetGeneration::new(1);
    let mut store = PageStore::create(&path).unwrap();
    commit_asset(&mut store, &asset);

    let mut crash = crash_file(&path);
    let result = store.transaction_with_crash_file(
        |tx| {
            for (key, _) in asset.records() {
                tx.delete(key)?;
            }
            Ok(())
        },
        &mut crash,
    );
    assert!(matches!(
        result,
        Err(TosumuError::CommittedButFlushFailed { .. })
    ));
    drop(crash);
    drop(store);

    let reopened = PageStore::open(&path).unwrap();
    assert!(reopened.scan().unwrap().is_empty());
    reopened.tree.check_invariants().unwrap();

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

#[test]
fn consumer_asset_before_commit_discards_staged_generation() {
    use crate::wal::{wal_path, WalRecord, WalWriter};

    let path = temp_path("asset_before_commit_recovery");
    let staged_path = temp_path("asset_before_commit_staged");
    let wal = wal_path(&path);
    let staged_wal = wal_path(&staged_path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
    let _ = std::fs::remove_file(&staged_path);
    let _ = std::fs::remove_file(&staged_wal);

    let old_asset = AssetGeneration::new(1);
    let staged_asset = AssetGeneration::new(2);
    let mut store = PageStore::create(&path).unwrap();
    commit_asset(&mut store, &old_asset);
    drop(store);
    let _ = std::fs::remove_file(&wal);

    let mut staged = PageStore::create(&staged_path).unwrap();
    commit_asset(&mut staged, &staged_asset);
    drop(staged);
    let _ = std::fs::remove_file(&staged_wal);

    let staged_bytes = std::fs::read(&staged_path).unwrap();
    let page_count = staged_bytes.len() / crate::format::PAGE_SIZE;
    let mut writer = WalWriter::create(&wal).unwrap();
    writer.append(&WalRecord::Begin { txn_id: 7 }).unwrap();
    for pgno in 0..page_count {
        let start = pgno * crate::format::PAGE_SIZE;
        let end = start + crate::format::PAGE_SIZE;
        let mut frame = Box::new([0u8; crate::format::PAGE_SIZE]);
        frame.copy_from_slice(&staged_bytes[start..end]);
        writer
            .append(&WalRecord::PageWrite {
                pgno: pgno as u64,
                page_version: 1,
                frame,
            })
            .unwrap();
    }
    writer.sync().unwrap();
    drop(writer);

    let reopened = PageStore::open(&path).unwrap();
    assert_asset(&reopened, &old_asset);
    assert_ne!(
        reopened.get(b"asset/manifest").unwrap(),
        Some(staged_asset.manifest)
    );

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
    let _ = std::fs::remove_file(&staged_path);
    let _ = std::fs::remove_file(&staged_wal);
}

#[test]
fn transaction_propagates_committed_but_partial_write_failed_and_recovers_on_reopen() {
    use crate::test_helpers::{CrashFile, CrashPhase};

    let path = temp_path("txn_partial_flush_fail");
    let wal = diff_wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);

    let mut store = PageStore::create(&path).unwrap();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let mut crash_file = CrashFile::new(
        file,
        CrashPhase::MidWrite {
            fail_after_bytes: (crate::format::PAGE_SIZE / 2) as u64,
        },
    );

    let err = store
        .transaction_with_crash_file(
            |tx| {
                tx.put(b"outer-mid-a", b"1")?;
                tx.put(b"outer-mid-b", b"2")?;
                Ok(())
            },
            &mut crash_file,
        )
        .unwrap_err();
    assert!(matches!(err, TosumuError::CommittedButFlushFailed { .. }));

    drop(store);

    let reopened = PageStore::open(&path).unwrap();
    assert_eq!(reopened.get(b"outer-mid-a").unwrap(), Some(b"1".to_vec()));
    assert_eq!(reopened.get(b"outer-mid-b").unwrap(), Some(b"2".to_vec()));

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

#[test]
fn transaction_root_split_flush_failure_recovers_full_tree_on_reopen() {
    use crate::test_helpers::{CrashFile, CrashPhase};

    let path = temp_path("txn_split_flush_fail");
    let wal = diff_wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);

    let mut store = PageStore::create(&path).unwrap();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let mut crash_file = CrashFile::new(file, CrashPhase::AfterWrite);

    let err = store
        .transaction_with_crash_file(
            |tx| {
                for i in 0u32..500 {
                    tx.put(
                        format!("split-key-{i:05}").as_bytes(),
                        format!("split-val-{i:05}").as_bytes(),
                    )?;
                }
                Ok(())
            },
            &mut crash_file,
        )
        .unwrap_err();
    assert!(matches!(err, TosumuError::CommittedButFlushFailed { .. }));

    drop(store);

    let reopened = PageStore::open(&path).unwrap();
    assert!(
        reopened.stat().unwrap().tree_height >= 2,
        "expected recovered tree to retain a split root"
    );
    for i in 0u32..500 {
        assert_eq!(
            reopened
                .get(format!("split-key-{i:05}").as_bytes())
                .unwrap(),
            Some(format!("split-val-{i:05}").into_bytes()),
            "missing key after recovering split transaction: {i}",
        );
    }

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

#[test]
fn transaction_root_split_partial_write_recovers_full_tree_on_reopen() {
    use crate::test_helpers::{CrashFile, CrashPhase};

    let path = temp_path("txn_split_partial_flush_fail");
    let wal = diff_wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);

    let mut store = PageStore::create(&path).unwrap();
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(&path)
        .unwrap();
    let mut crash_file = CrashFile::new(
        file,
        CrashPhase::MidWrite {
            fail_after_bytes: ((crate::format::PAGE_SIZE * 3) / 2) as u64,
        },
    );

    let err = store
        .transaction_with_crash_file(
            |tx| {
                for i in 0u32..500 {
                    tx.put(
                        format!("split-mid-key-{i:05}").as_bytes(),
                        format!("split-mid-val-{i:05}").as_bytes(),
                    )?;
                }
                Ok(())
            },
            &mut crash_file,
        )
        .unwrap_err();
    assert!(matches!(err, TosumuError::CommittedButFlushFailed { .. }));

    drop(store);

    let reopened = PageStore::open(&path).unwrap();
    assert!(
        reopened.stat().unwrap().tree_height >= 2,
        "expected recovered tree to retain a split root after torn write"
    );
    for i in 0u32..500 {
        assert_eq!(
            reopened
                .get(format!("split-mid-key-{i:05}").as_bytes())
                .unwrap(),
            Some(format!("split-mid-val-{i:05}").into_bytes()),
            "missing key after recovering torn split transaction: {i}",
        );
    }

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

#[test]
fn spans_multiple_pages() {
    let path = temp_path("multipage");
    let _ = std::fs::remove_file(&path);

    let mut store = PageStore::create(&path).unwrap();
    // Each record: 5 + 10 + 100 = 115 bytes + 4 slot = 119 bytes.
    // Usable space per page: 4038 bytes ≈ 33 records per page.
    // Insert 100 to ensure we span at least 3 pages.
    for i in 0u32..100 {
        let k = format!("key{i:05}");
        let v = format!("value{i:05}-{}", "x".repeat(90));
        store.put(k.as_bytes(), v.as_bytes()).unwrap();
    }

    let before_pages = store.stat().unwrap().data_pages;
    assert!(
        before_pages > 1,
        "expected multiple pages, got {before_pages}"
    );
    drop(store);

    let store2 = PageStore::open(&path).unwrap();
    for i in 0u32..100 {
        let k = format!("key{i:05}");
        let v = format!("value{i:05}-{}", "x".repeat(90));
        assert_eq!(store2.get(k.as_bytes()).unwrap(), Some(v.into_bytes()));
    }

    let _ = std::fs::remove_file(&path);
}

// ── Passphrase-encryption tests ───────────────────────────────────────────
