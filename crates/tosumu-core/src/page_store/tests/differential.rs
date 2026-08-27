use super::*;

#[test]
fn differential_crash_recovery_matches_btreemap_model() {
    let path = temp_path("diff_crash_recovery");
    let wal = diff_wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);

    let mut model = BTreeMap::<Vec<u8>, Vec<u8>>::new();
    let mut store = PageStore::create_encrypted(&path, "diff-pass").unwrap();

    for step in 0..100usize {
        match step % 10 {
            0 => {
                drop(store);
                store = PageStore::open_with_passphrase(&path, "diff-pass").unwrap();
            }
            1..=5 => {
                let key_index = (step * 7) % 41;
                let key = diff_key(key_index);
                let value = diff_value(step, key_index);
                model.insert(key.clone(), value.clone());
                store.put(&key, &value).unwrap();
            }
            6 | 7 => {
                let key_index = (step * 11) % 41;
                let key = diff_key(key_index);
                model.remove(&key);
                store.delete(&key).unwrap();
            }
            _ => {
                let key_a_index = (step * 5) % 41;
                let key_b_index = (step * 13 + 3) % 41;
                let key_a = diff_key(key_a_index);
                let key_b = diff_key(key_b_index);
                let value_a = diff_value(step, key_a_index + 50);
                let value_b = diff_value(step, key_b_index + 75);

                model.insert(key_a.clone(), value_a.clone());
                model.insert(key_b.clone(), value_b.clone());

                store
                    .transaction(|tx| {
                        tx.put(&key_a, &value_a)?;
                        tx.put(&key_b, &value_b)?;
                        Ok(())
                    })
                    .unwrap();
            }
        }

        assert_model_matches_store(&store, &model, &format!("step {step}"));
    }

    drop(store);
    let reopened = PageStore::open_with_passphrase(&path, "diff-pass").unwrap();
    assert_model_matches_store(&reopened, &model, "final reopen");

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal);
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(16))]

    #[test]
    fn prop_differential_crash_recovery_matches_btreemap_model(
        ops in prop::collection::vec(
            prop_oneof![
                (arb_diff_key(), arb_diff_value()).prop_map(|(key, value)| DiffOp::Put(key, value)),
                arb_diff_key().prop_map(DiffOp::Delete),
                Just(DiffOp::CrashReopen),
                (arb_diff_key(), arb_diff_value(), arb_diff_key(), arb_diff_value())
                    .prop_map(|(key_a, value_a, key_b, value_b)| DiffOp::TxnPutPair(key_a, value_a, key_b, value_b)),
            ],
            1..=60,
        )
    ) {
        let path = temp_path("prop_diff_crash_recovery");
        let wal = diff_wal_path(&path);
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&wal);

        let mut model = BTreeMap::<Vec<u8>, Vec<u8>>::new();
        let mut store = PageStore::create_encrypted(&path, "prop-diff-pass").unwrap();

        for (step, op) in ops.iter().enumerate() {
            match op {
                DiffOp::Put(key, value) => {
                    model.insert(key.clone(), value.clone());
                    store.put(key, value).unwrap();
                }
                DiffOp::Delete(key) => {
                    model.remove(key);
                    store.delete(key).unwrap();
                }
                DiffOp::CrashReopen => {
                    drop(store);
                    store = PageStore::open_with_passphrase(&path, "prop-diff-pass").unwrap();
                }
                DiffOp::TxnPutPair(key_a, value_a, key_b, value_b) => {
                    model.insert(key_a.clone(), value_a.clone());
                    model.insert(key_b.clone(), value_b.clone());
                    store.transaction(|tx| {
                        tx.put(key_a, value_a)?;
                        tx.put(key_b, value_b)?;
                        Ok(())
                    }).unwrap();
                }
            }

            prop_assert!(
                store.tree.check_invariants().is_ok(),
                "check_invariants failed after step {}: {:?}",
                step,
                op
            );

            let actual = store.scan().unwrap();
            let expected = model_scan(&model);
            prop_assert_eq!(actual, expected, "model mismatch after step {}: {:?}", step, op);
        }

        drop(store);
        let reopened = PageStore::open_with_passphrase(&path, "prop-diff-pass").unwrap();
        prop_assert!(reopened.tree.check_invariants().is_ok(), "check_invariants failed after final reopen");
        prop_assert_eq!(reopened.scan().unwrap(), model_scan(&model), "model mismatch after final reopen");

        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(&wal);
    }
}
