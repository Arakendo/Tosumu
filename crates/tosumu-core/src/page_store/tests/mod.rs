use super::*;
use proptest::prelude::*;
use std::collections::BTreeMap;
use std::fs::OpenOptions;
use std::path::PathBuf;
use tempfile;

fn temp_path(name: &str) -> PathBuf {
    // Use tempfile to get a collision-free OS-assigned path.
    // We immediately close the placeholder file so the store can create it fresh.
    let f = tempfile::Builder::new()
        .prefix(&format!("tosumu_{name}_"))
        .suffix(".tsm")
        .tempfile()
        .expect("tempfile allocation failed");
    let path = f.path().to_path_buf();
    drop(f);
    path
}

fn model_scan(model: &BTreeMap<Vec<u8>, Vec<u8>>) -> Vec<(Vec<u8>, Vec<u8>)> {
    model
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect()
}

fn diff_key(index: usize) -> Vec<u8> {
    format!("key-{index:02}").into_bytes()
}

fn diff_value(step: usize, salt: usize) -> Vec<u8> {
    let repeat = 1 + ((step + salt) % 5);
    format!("value-{step:03}-{salt:02}-{}", "x".repeat(repeat * 12)).into_bytes()
}

#[derive(Clone)]
struct AssetGeneration {
    manifest: Vec<u8>,
    provenance: Vec<u8>,
    payload_small: Vec<u8>,
    payload_large: Vec<u8>,
}

impl AssetGeneration {
    fn new(version: u8) -> Self {
        Self {
            manifest: format!("fixture-schema-v{version}").into_bytes(),
            provenance: format!("source:tokimu-test\nrevision:{version:04}").into_bytes(),
            payload_small: vec![version; 32],
            payload_large: (0u8..=255)
                .cycle()
                .map(|byte| byte.wrapping_add(version))
                .take(1024 * 1024)
                .collect(),
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
}

fn commit_asset(store: &mut PageStore, asset: &AssetGeneration) {
    store
        .transaction(|tx| {
            for (key, value) in asset.records() {
                tx.put(key, value)?;
            }
            Ok(())
        })
        .unwrap();
}

fn assert_asset(store: &PageStore, asset: &AssetGeneration) {
    for (key, value) in asset.records() {
        assert_eq!(
            store.get(key).unwrap(),
            Some(value.to_vec()),
            "asset key {key:?}"
        );
    }
    assert_eq!(store.scan().unwrap().len(), asset.records().len());
    store.tree.check_invariants().unwrap();
}

fn crash_file(path: &std::path::Path) -> crate::test_helpers::CrashFile {
    let file = OpenOptions::new()
        .read(true)
        .write(true)
        .open(path)
        .unwrap();
    crate::test_helpers::CrashFile::new(file, crate::test_helpers::CrashPhase::AfterWrite)
}

fn diff_wal_path(path: &std::path::Path) -> PathBuf {
    crate::wal::wal_path(path)
}

#[derive(Debug, Clone)]
enum DiffOp {
    Put(Vec<u8>, Vec<u8>),
    Delete(Vec<u8>),
    CrashReopen,
    TxnPutPair(Vec<u8>, Vec<u8>, Vec<u8>, Vec<u8>),
}

fn arb_diff_key() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(0u8..16, 1..=4)
}

fn arb_diff_value() -> impl Strategy<Value = Vec<u8>> {
    prop::collection::vec(any::<u8>(), 0..=24)
}

fn assert_model_matches_store(
    store: &PageStore,
    model: &BTreeMap<Vec<u8>, Vec<u8>>,
    context: &str,
) {
    store.tree.check_invariants().unwrap();
    assert_eq!(
        store.scan().unwrap(),
        model_scan(model),
        "model mismatch after {context}"
    );
}

mod basic;
mod differential;
mod hostile_input;
mod key_management_resilience;
mod protectors;
mod recovery;
mod storage_behavior;
