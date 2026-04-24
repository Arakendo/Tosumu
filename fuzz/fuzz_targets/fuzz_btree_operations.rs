#![no_main]

// Fuzz target: fuzz_btree_operations
//
// Feed arbitrary bytes to drive a sequence of put/get/delete operations
// against a temporary BTree file. Asserts that:
//   1. No panics occur for any input.
//   2. After put(k, v), get(k) == Some(v)  (when the operation succeeded).
//   3. After delete(k), get(k) == None      (when the operation succeeded).
//
// Input layout (greedy):
//   For each operation:
//     [op: u8][key_len: u8][val_len: u8][key bytes][val bytes]
//   op 0 = put, op 1 = delete, anything else = get (no-op for assert)

use libfuzzer_sys::fuzz_target;
use std::path::PathBuf;
use tosumu_core::btree::BTree;

fuzz_target!(|data: &[u8]| {
    let path = {
        let tid = std::thread::current().id();
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        PathBuf::from(format!("/tmp/fuzz_btree_{tid:?}_{ts}.tsm"))
    };

    let _ = std::fs::remove_file(&path);
    let mut tree = match BTree::create(&path) {
        Ok(t) => t,
        Err(_) => return,
    };

    let mut pos = 0;
    while pos + 3 <= data.len() {
        let op = data[pos];
        let key_len = data[pos + 1] as usize;
        let val_len = data[pos + 2] as usize;
        pos += 3;

        if pos + key_len > data.len() { break; }
        let key = &data[pos..pos + key_len];
        pos += key_len;

        if key.is_empty() { continue; }

        match op % 3 {
            0 => {
                // put
                if pos + val_len > data.len() { break; }
                let val = &data[pos..pos + val_len];
                pos += val_len;
                if let Ok(()) = tree.put(key, val) {
                    // Verify round-trip.
                    if let Ok(got) = tree.get(key) {
                        assert_eq!(got, Some(val.to_vec()));
                    }
                }
            }
            1 => {
                // delete
                if let Ok(()) = tree.delete(key) {
                    if let Ok(got) = tree.get(key) {
                        assert_eq!(got, None);
                    }
                }
            }
            _ => {
                // get — just ensure no panic
                let _ = tree.get(key);
            }
        }
    }

    let _ = std::fs::remove_file(&path);
});
