use std::path::PathBuf;
use std::time::Instant;

use sha2::{Digest, Sha256};
use tosumu_core::format::{FORMAT_VERSION, OFF_FORMAT_VERSION, PAGE_SIZE};
use tosumu_core::inspect::VerifyIssueKind;
use tosumu_core::page_store::PageStore;
use tosumu_core::{KvStore, TosumuError};

fn assert_send_sync<T: Send + Sync>() {}

fn temp_store_path(name: &str) -> PathBuf {
    let file = tempfile::Builder::new()
        .prefix(&format!("tosumu_provider_{name}_"))
        .suffix(".tsm")
        .tempfile()
        .expect("allocate temporary path");
    let path = file.path().to_path_buf();
    drop(file);
    path
}

fn remove_store_files(path: &PathBuf) {
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_file(format!("{}.wal", path.display()));
}

fn fixture_records() -> Vec<(Vec<u8>, Vec<u8>)> {
    vec![
        (b"asset/manifest".to_vec(), b"fixture-schema-v1".to_vec()),
        (
            b"asset/provenance".to_vec(),
            b"source:tokimu-test\nrevision:0001".to_vec(),
        ),
        (
            b"asset/dependencies".to_vec(),
            b"base-material\nshared-mesh".to_vec(),
        ),
        (
            b"asset/diagnostics".to_vec(),
            b"warning:fixture-only\nstatus:clean".to_vec(),
        ),
        (b"asset/payload-small".to_vec(), vec![0x00, 0x01, 0xfe, 0xff]),
        (
            b"asset/payload-large".to_vec(),
            (0u8..=255).cycle().take(1024 * 1024).collect(),
        ),
    ]
}

fn fixture_hashes(records: &[(Vec<u8>, Vec<u8>)]) -> Vec<(Vec<u8>, String)> {
    let mut hashes: Vec<_> = records
        .iter()
        .map(|(key, value)| {
            let mut hasher = Sha256::new();
            hasher.update(value);
            (key.clone(), format!("{:x}", hasher.finalize()))
        })
        .collect();
    hashes.sort_by(|left, right| left.0.cmp(&right.0));
    hashes
}

#[test]
fn external_consumer_can_commit_and_reopen_multiple_records() {
    assert_send_sync::<KvStore>();
    let path = temp_store_path("commit");
    remove_store_files(&path);
    {
        let mut store = KvStore::create(&path).unwrap();
        store
            .transaction(|transaction| {
                transaction.put(b"manifest", b"schema-v1")?;
                transaction.put(b"payload/a", &[0x00, 0x01, 0xff])?;
                transaction.put(b"payload/b", b"binary-data")?;
                Ok(())
            })
            .unwrap();
    }

    let store = KvStore::open(&path).unwrap();
    assert_eq!(store.get(b"manifest").unwrap(), Some(b"schema-v1".to_vec()));
    assert_eq!(
        store.get(b"payload/a").unwrap(),
        Some(vec![0x00, 0x01, 0xff])
    );
    assert_eq!(store.get(b"payload/b").unwrap(), Some(b"binary-data".to_vec()));

    remove_store_files(&path);
}

#[test]
fn external_consumer_keeps_database_identities_isolated() {
    let first = temp_store_path("identity_first");
    let second = temp_store_path("identity_second");
    remove_store_files(&first);
    remove_store_files(&second);

    {
        let mut store = KvStore::create(&first).unwrap();
        store.put(b"asset/manifest", b"first").unwrap();
    }
    {
        let mut store = KvStore::create(&second).unwrap();
        store.put(b"asset/manifest", b"second").unwrap();
    }

    assert_eq!(
        KvStore::open(&first)
            .unwrap()
            .get(b"asset/manifest")
            .unwrap(),
        Some(b"first".to_vec())
    );
    assert_eq!(
        KvStore::open(&second)
            .unwrap()
            .get(b"asset/manifest")
            .unwrap(),
        Some(b"second".to_vec())
    );

    remove_store_files(&first);
    remove_store_files(&second);
}

#[test]
fn external_consumer_gets_wrong_key_for_encrypted_store() {
    let path = temp_store_path("wrong_key");
    remove_store_files(&path);

    PageStore::create_encrypted(&path, "correct-horse").unwrap();
    let error = match PageStore::open_with_passphrase(&path, "wrong-horse") {
        Ok(_) => panic!("wrong passphrase must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(error, TosumuError::WrongKey));

    remove_store_files(&path);
}

#[test]
fn external_consumer_gets_structured_error_for_newer_physical_format() {
    let path = temp_store_path("newer_format");
    remove_store_files(&path);
    let newer_version = FORMAT_VERSION + 1;

    PageStore::create(&path).unwrap();
    let mut page0 = std::fs::read(&path).unwrap();
    page0[OFF_FORMAT_VERSION..OFF_FORMAT_VERSION + 2]
        .copy_from_slice(&newer_version.to_le_bytes());
    std::fs::write(&path, page0).unwrap();

    let error = match tosumu_core::inspect::inspect_verification(&path) {
        Ok(_) => panic!("newer physical format must be rejected"),
        Err(error) => error,
    };
    assert!(matches!(
        error,
        TosumuError::NewerFormat {
            found,
            supported_max: FORMAT_VERSION
        } if found == newer_version
    ));

    remove_store_files(&path);
}

#[test]
fn external_consumer_gets_structured_finding_for_corrupt_page() {
    let path = temp_store_path("corrupt_page");
    remove_store_files(&path);
    let mut store = PageStore::create(&path).unwrap();
    store.put(b"asset/manifest", b"fixture-schema-v1").unwrap();
    drop(store);

    let mut bytes = std::fs::read(&path).unwrap();
    bytes[PAGE_SIZE + 100] ^= 0xff;
    std::fs::write(&path, bytes).unwrap();

    let report = tosumu_core::inspect::inspect_verification(&path).unwrap();
    assert_eq!(report.pages.pages_ok, 0);
    assert_eq!(report.pages.issues.len(), 1);
    assert_eq!(report.pages.issues[0].pgno, 1);
    assert_eq!(report.pages.issues[0].kind, VerifyIssueKind::AuthFailed);
    assert_eq!(report.pages.page_results[0].issue_kind, Some(VerifyIssueKind::AuthFailed));
    assert!(!report.btree.checked);

    remove_store_files(&path);
}

#[test]
fn external_consumer_can_backup_with_source_handle_open() {
    let source = temp_store_path("open_source_backup");
    let destination = temp_store_path("open_source_backup_dest");
    remove_store_files(&source);
    remove_store_files(&destination);

    let mut store = KvStore::create(&source).unwrap();
    store.put(b"asset/manifest", b"fixture-schema-v1").unwrap();

    let report = tosumu_core::backup::create_stable_backup(&source, &destination).unwrap();
    assert_eq!(report.source, source);
    assert_eq!(
        KvStore::open(&destination)
            .unwrap()
            .get(b"asset/manifest")
            .unwrap(),
        Some(b"fixture-schema-v1".to_vec())
    );

    drop(store);
    remove_store_files(&source);
    remove_store_files(&destination);
}

#[test]
fn external_consumer_rollback_exposes_no_partial_asset() {
    let path = temp_store_path("rollback");
    remove_store_files(&path);

    let mut store = KvStore::create(&path).unwrap();
    let result: std::result::Result<(), TosumuError> = store.transaction(|transaction| {
        transaction.put(b"manifest", b"uncommitted")?;
        transaction.put(b"payload", b"uncommitted-bytes")?;
        Err(TosumuError::InvalidArgument("deliberate rollback"))
    });

    assert!(result.is_err());
    assert_eq!(store.get(b"manifest").unwrap(), None);
    assert_eq!(store.get(b"payload").unwrap(), None);

    remove_store_files(&path);
}

#[test]
fn external_consumer_atomically_commits_and_rolls_back_large_asset() {
    let path = temp_store_path("large_transaction");
    remove_store_files(&path);

    let large: Vec<u8> = (0u8..=255).cycle().take(1024 * 1024).collect();
    let mut store = KvStore::create(&path).unwrap();
    store
        .transaction(|transaction| {
            transaction.put(b"manifest", b"schema-v1")?;
            transaction.put(b"payload", &large)?;
            Ok(())
        })
        .unwrap();
    assert_eq!(store.get(b"payload").unwrap(), Some(large.clone()));

    let page_count_after_commit = store.stat().unwrap().page_count;
    let result: std::result::Result<(), TosumuError> = store.transaction(|transaction| {
        transaction.put(b"manifest", b"uncommitted")?;
        transaction.put(b"discarded", &large)?;
        Err(TosumuError::InvalidArgument("deliberate large rollback"))
    });
    assert!(result.is_err());
    assert_eq!(store.get(b"manifest").unwrap(), Some(b"schema-v1".to_vec()));
    assert_eq!(store.get(b"discarded").unwrap(), None);
    assert_eq!(store.stat().unwrap().page_count, page_count_after_commit);

    drop(store);
    let reopened = KvStore::open(&path).unwrap();
    assert_eq!(reopened.get(b"payload").unwrap(), Some(large));
    assert_eq!(reopened.get(b"discarded").unwrap(), None);

    remove_store_files(&path);
}

#[test]
fn external_consumer_gets_structured_error_for_readonly_mutation() {
    let path = temp_store_path("readonly");
    remove_store_files(&path);

    {
        let mut store = KvStore::create(&path).unwrap();
        store.put(b"existing", b"value").unwrap();
    }

    let mut readonly = KvStore::open_readonly(&path).unwrap();
    let error = readonly.put(b"new", b"value").unwrap_err();
    assert!(matches!(error, TosumuError::InvalidArgument(_)));
    assert_eq!(error.error_report().status.as_str(), "invalid_input");
    assert_eq!(readonly.get(b"existing").unwrap(), Some(b"value".to_vec()));

    remove_store_files(&path);
}

#[test]
fn external_consumer_gets_structured_error_for_oversized_value() {
    let path = temp_store_path("oversized");
    remove_store_files(&path);

    let mut store = KvStore::create(&path).unwrap();
    store.put(b"existing", b"untouched").unwrap();
    let page_count = store.stat().unwrap().page_count;
    let value = vec![0u8; tosumu_core::MAX_VALUE_SIZE + 1];
    let error = store.put(b"large", &value).unwrap_err();

    assert!(matches!(
        error,
        TosumuError::ValueTooLarge { actual, maximum }
            if actual == (tosumu_core::MAX_VALUE_SIZE + 1) as u64
                && maximum == tosumu_core::MAX_VALUE_SIZE as u64
    ));
    let report = error.error_report();
    assert_eq!(report.code, "VALUE_TOO_LARGE");
    assert_eq!(report.detail_u64("actual"), Some(value.len() as u64));
    assert_eq!(
        report.detail_u64("maximum"),
        Some(tosumu_core::MAX_VALUE_SIZE as u64)
    );
    assert_eq!(store.get(b"existing").unwrap(), Some(b"untouched".to_vec()));
    assert_eq!(store.stat().unwrap().page_count, page_count);

    remove_store_files(&path);
}

#[test]
fn external_consumer_round_trips_empty_and_inline_boundary_values() {
    let path = temp_store_path("inline_boundaries");
    remove_store_files(&path);

    let mut store = KvStore::create(&path).unwrap();
    for (key, size) in [
        (b"empty".as_slice(), 0usize),
        (b"below".as_slice(), u16::MAX as usize - 1),
        (b"at".as_slice(), u16::MAX as usize),
        (b"above".as_slice(), u16::MAX as usize + 1),
    ] {
        let value = vec![size as u8; size];
        store.put(key, &value).unwrap();
        assert_eq!(store.get(key).unwrap(), Some(value));
    }

    remove_store_files(&path);
}

#[test]
fn external_consumer_round_trips_one_megabyte_value_after_reopen() {
    let path = temp_store_path("one_megabyte");
    remove_store_files(&path);

    let value: Vec<u8> = (0u8..=255).cycle().take(1024 * 1024).collect();
    {
        let mut store = KvStore::create(&path).unwrap();
        store.put(b"payload", &value).unwrap();
        assert_eq!(store.get(b"payload").unwrap(), Some(value.clone()));
    }

    let reopened = KvStore::open(&path).unwrap();
    assert_eq!(reopened.get(b"payload").unwrap(), Some(value.clone()));
    assert_eq!(reopened.scan().unwrap(), vec![(b"payload".to_vec(), value)]);

    remove_store_files(&path);
}

#[test]
fn external_consumer_round_trips_sixteen_megabyte_value_after_reopen() {
    round_trip_large_value("sixteen_megabyte", 16 * 1024 * 1024);
}

#[test]
fn external_consumer_round_trips_maximum_value_after_reopen() {
    round_trip_large_value("sixty_four_megabyte", tosumu_core::MAX_VALUE_SIZE);
}

fn round_trip_large_value(name: &str, size: usize) {
    let path = temp_store_path(name);
    remove_store_files(&path);

    let value: Vec<u8> = (0u8..=255).cycle().take(size).collect();
    let expected_hash = Sha256::digest(&value);
    {
        let mut store = KvStore::create(&path).unwrap();
        store.put(b"payload", &value).unwrap();
    }

    let reopened = KvStore::open(&path).unwrap();
    let fetched = reopened.get(b"payload").unwrap().unwrap();
    assert_eq!(Sha256::digest(&fetched), expected_hash);
    let scanned = reopened.scan().unwrap();
    assert_eq!(scanned.len(), 1);
    assert_eq!(Sha256::digest(&scanned[0].1), expected_hash);
    assert_eq!(scanned, vec![(b"payload".to_vec(), value)]);

    remove_store_files(&path);
}

#[test]
fn external_consumer_overwrite_and_delete_large_value() {
    let path = temp_store_path("large_lifecycle");
    remove_store_files(&path);

    let large: Vec<u8> = (0u8..=255).cycle().take(1024 * 1024).collect();
    let replacement = b"small replacement".to_vec();
    {
        let mut store = KvStore::create(&path).unwrap();
        store.put(b"payload", &large).unwrap();
        let allocated_pages = store.stat().unwrap().page_count;
        store.put(b"payload", &replacement).unwrap();
        assert_eq!(store.get(b"payload").unwrap(), Some(replacement.clone()));
        assert_eq!(store.stat().unwrap().page_count, allocated_pages);
        store.put(b"payload", &large).unwrap();
        assert_eq!(store.stat().unwrap().page_count, allocated_pages);
        store.delete(b"payload").unwrap();
        assert_eq!(store.get(b"payload").unwrap(), None);
    }

    let reopened = KvStore::open(&path).unwrap();
    assert_eq!(reopened.get(b"payload").unwrap(), None);
    assert!(reopened.scan().unwrap().is_empty());

    remove_store_files(&path);
}

#[test]
fn external_consumer_keeps_large_value_when_leaf_splits() {
    let path = temp_store_path("large_split");
    remove_store_files(&path);

    let large: Vec<u8> = (0u8..=255).cycle().take(1024 * 1024).collect();
    let mut store = KvStore::create(&path).unwrap();
    store.put(b"large", &large).unwrap();
    for index in 0..500u32 {
        store.put(format!("key-{index:03}").as_bytes(), b"small").unwrap();
    }

    assert_eq!(store.get(b"large").unwrap(), Some(large));
    remove_store_files(&path);
}

#[test]
fn external_consumer_keeps_large_value_when_leaf_compacts() {
    let path = temp_store_path("large_compaction");
    remove_store_files(&path);

    let large: Vec<u8> = (0u8..=255).cycle().take(1024 * 1024).collect();
    let mut store = KvStore::create(&path).unwrap();
    store.put(b"large", &large).unwrap();
    for index in 0..120u32 {
        store.put(format!("key-{index:03}").as_bytes(), &[index as u8; 100]).unwrap();
    }
    for index in 0..80u32 {
        store.delete(format!("key-{index:03}").as_bytes()).unwrap();
    }

    assert_eq!(store.get(b"large").unwrap(), Some(large));
    remove_store_files(&path);
}

struct AssetFixture {
    manifest: Vec<u8>,
    provenance: Vec<u8>,
    payload_small: Vec<u8>,
    payload_large: Vec<u8>,
}

impl AssetFixture {
    fn deterministic() -> Self {
        Self {
            manifest: b"fixture-schema-v1".to_vec(),
            provenance: b"source:tokimu-test\nrevision:0001".to_vec(),
            payload_small: (0u8..=31).collect(),
            payload_large: (0u8..=255).cycle().take(1024 * 1024).collect(),
        }
    }

    fn records(&self) -> [(&'static [u8], &[u8]); 4] {
        [
            (b"asset/manifest", &self.manifest),
            (b"asset/provenance", &self.provenance),
            (b"asset/payload-small", &self.payload_small),
            (b"asset/payload-large", &self.payload_large),
        ]
    }

    fn hashes(&self) -> Vec<([u8; 32], [u8; 32])> {
        self.records()
            .iter()
            .map(|(_, value)| {
                let hash: [u8; 32] = Sha256::digest(value).into();
                (hash, hash)
            })
            .collect()
    }
}

#[test]
fn external_consumer_fixture_hashes_survive_commit_overwrite_scan_and_reopen() {
    let path = temp_store_path("fixture_hashes");
    remove_store_files(&path);

    let fixture = AssetFixture::deterministic();
    let expected_hashes = fixture.hashes();
    let mut store = KvStore::create(&path).unwrap();
    store
        .transaction(|transaction| {
            for (key, value) in fixture.records() {
                transaction.put(key, value)?;
            }
            Ok(())
        })
        .unwrap();

    for (index, (key, _)) in fixture.records().iter().enumerate() {
        let value = store.get(key).unwrap().unwrap();
        assert_eq!(Sha256::digest(value).as_slice(), expected_hashes[index].0);
    }

    store.put(b"asset/payload-small", b"replacement").unwrap();
    let replacement_hash: [u8; 32] = Sha256::digest(b"replacement").into();
    assert_eq!(
        Sha256::digest(store.get(b"asset/payload-small").unwrap().unwrap()).as_slice(),
        replacement_hash
    );

    let scan = store.scan().unwrap();
    for (key, value) in scan {
        let expected = if key == b"asset/payload-small" {
            replacement_hash
        } else {
            fixture
                .records()
                .iter()
                .find(|(fixture_key, _)| *fixture_key == key.as_slice())
                .map(|(_, fixture_value)| Sha256::digest(fixture_value).into())
                .unwrap()
        };
        assert_eq!(Sha256::digest(value).as_slice(), expected);
    }

    drop(store);
    let reopened = KvStore::open(&path).unwrap();
    assert_eq!(
        Sha256::digest(reopened.get(b"asset/payload-large").unwrap().unwrap()).as_slice(),
        expected_hashes[3].0
    );
    remove_store_files(&path);
}

#[test]
#[ignore = "expensive size measurement; run explicitly with --ignored --nocapture"]
fn large_value_write_measurements_record_one_overwrite_per_size() {
    for (label, size) in [
        ("1 MiB", 1024 * 1024),
        ("16 MiB", 16 * 1024 * 1024),
        ("64 MiB", tosumu_core::MAX_VALUE_SIZE),
    ] {
        let path = temp_store_path("size_measurement");
        remove_store_files(&path);
        let value: Vec<u8> = (0u8..=255).cycle().take(size).collect();
        let mut store = KvStore::create(&path).unwrap();
        store.put(b"payload", &value).unwrap();

        let start = Instant::now();
        store.put(b"payload", &value).unwrap();
        let elapsed = start.elapsed();
        eprintln!(
            "large-value overwrite: {label}, bytes={size}, elapsed_ms={:.1}, logical_copy_volume_bytes={}",
            elapsed.as_secs_f64() * 1000.0,
            size
        );
        assert!(!elapsed.is_zero());
        remove_store_files(&path);
    }
}

#[test]
fn external_consumer_fixture_round_trips_backup_export_and_verification() {
    let source = temp_store_path("fixture_source");
    let backup = temp_store_path("fixture_backup");
    let export = temp_store_path("fixture_export");
    remove_store_files(&source);
    remove_store_files(&backup);
    remove_store_files(&export);
    let records = fixture_records();
    let expected_hashes = fixture_hashes(&records);

    {
        let mut store = KvStore::create(&source).unwrap();
        store
            .transaction(|transaction| {
                for (key, value) in &records {
                    transaction.put(key, value)?;
                }
                Ok(())
            })
            .unwrap();
    }

    let reopened = KvStore::open(&source).unwrap();
    assert_eq!(fixture_hashes(&reopened.scan().unwrap()), expected_hashes);

    let backup_report = tosumu_core::backup::create_stable_backup(&source, &backup).unwrap();
    assert!(backup_report.destination_wal.is_some());
    let backed_up = KvStore::open(&backup).unwrap();
    assert_eq!(fixture_hashes(&backed_up.scan().unwrap()), expected_hashes);

    let export_report = tosumu_core::export::create_portable_export(&source, &export).unwrap();
    assert!(export_report.source_had_wal);
    assert!(!std::path::PathBuf::from(format!("{}.wal", export.display())).exists());
    let exported = KvStore::open_readonly(&export).unwrap();
    assert_eq!(fixture_hashes(&exported.scan().unwrap()), expected_hashes);

    let verification = tosumu_core::inspect::inspect_verification(&export).unwrap();
    assert_eq!(verification.pages.pages_ok, verification.pages.pages_checked);
    assert!(verification.btree.checked);
    assert!(verification.btree.ok);

    remove_store_files(&source);
    remove_store_files(&backup);
    remove_store_files(&export);
}