# Crypto File Conservation Matrix v1

| Field | Value |
| --- | --- |
| Status | Retained executable baseline; not provider-independence evidence |
| Observed | 2026-09-03 |
| Scope | Format-v3 encrypted create/open, mutation, recovery, protectors, snapshots, inspection, and rebuild |
| Owner | AR-0016 Gate C0 |

This matrix names the existing file-level evidence that must remain green
across the first private crypto seam. It complements the exact construction
vectors in `crypto::tests::gate_c0_fixed_construction_vectors`.

The matrix intentionally does not require two randomly created databases to
have identical bytes. It pins exact deterministic construction outputs and
then pins file-format fields, acceptance/refusal behavior, authenticated
meaning, recovery outcome, and publication behavior around random values.

## Required Matrix

| Boundary | Executable evidence | Conserved observation |
| --- | --- | --- |
| Encrypted create/open | `page_store::tests::protectors::encrypted_create_open_roundtrip` | Correct passphrase reopens and reads committed data |
| Wrong or missing unlock | `encrypted_wrong_passphrase_returns_wrong_key`; `encrypted_open_without_passphrase_returns_wrong_key` | Refusal remains `WrongKey`; no fallback to sentinel interpretation |
| Plaintext exclusion | `encrypted_data_is_not_plaintext_in_file` | Stored frame does not contain the tested plaintext value |
| Transaction and reopen | `storage_behavior::encrypted_transaction_commit_survives_reopen`; `encrypted_autocommit_after_transaction_survives_reopen` | Encrypted committed values survive ordinary reopen |
| Atomic write and snapshot | integration test `encrypted_owner_commits_and_rolls_back_atomic_write_closures` | Captured encrypted snapshot remains stable while one generation commits; rollback publishes nothing |
| Page authentication | `basic::auth_failure_on_corrupted_page`; `hostile_input::corrupt_ciphertext_page_in_encrypted_db_auth_fails` | Modified page frames fail authentication with page identity |
| Header/keyslot authentication | `corrupt_header_mac_on_encrypted_db_rejected`; `header_mac_tampered_keyslot_region_rejected`; `protector_swap_attack_rejected` | Header and protector substitution do not produce accepted plaintext |
| Protector interoperability | `multi_slot_second_passphrase_can_unlock`; `recovery_key_roundtrip`; `keyfile_roundtrip` | All admitted format-v3 protector kinds recover the same database state |
| Protector mutation | `rekey_kek_old_fails_new_succeeds`; `remove_second_slot_original_pass_still_works` | Rekey/removal change only intended unlock authority |
| Protector crash states | `crash_before_rekey_write_old_passphrase_recovers`; `crash_mid_rekey_torn_page_rejected`; `crash_before_add_protector_write_original_still_works` | Prior complete page 0 remains usable or torn authenticated state is rejected |
| WAL encoded-frame recovery | `wal::tests::recovery::integration_recover_real_pager_frame`; pager phase-two failure tests | Complete committed encoded frames recover; ambiguous phase-two failures retain typed outcome |
| Rebuild key continuity | `vacuum_rebuild::tests::encrypted_rebuild_verifies_with_the_original_passphrase`; recovery-key and keyfile variants | Rebuild preserves protector authority and re-encrypts verified logical state |
| Rebuild nonce freshness | `pager::tests::rebuild_staging_preserves_protectors_and_generation_with_fresh_page_nonce` | Staging does not reuse the source page frame/nonce |
| Independent public caller | `provider_boundary::external_consumer_gets_wrong_key_for_encrypted_store`; encrypted `SharedKvStore` integration test | Public storage callers see Tosumu errors and storage meaning, not backend types |
| Inspection | `inspect::tests::inspect_verification_supports_passphrase_protected_store`; recovery/keyfile companion test | Verification uses the same authenticated pager boundary |

## Baseline Result

On 2026-09-03:

- `cargo test --workspace --tests` passed;
- the core library reported 254 passed and five explicitly ignored tests;
- the external core integration suites reported 39 passed and three explicitly
  ignored expensive measurement cases; and
- no test in the named matrix failed.

These counts describe that invocation only. They do not include ignored tests,
fuzz targets, or a completed Criterion benchmark run.

## Seam Rerun Rule

The first private seam change must rerun:

1. the exact fixed construction vector;
2. every named matrix test, normally through the full workspace test suite;
3. strict Clippy and formatting;
4. native and browser-WASM compilation relevant to the changed boundary;
5. strict documentation; and
6. the existing performance benchmark as a separately recorded observation if
   the page hot path gains dispatch or allocation.

A changed random nonce, salt, or identifier is expected between executions. A
changed field size, label, AAD domain, error class, unlock decision, recovery
outcome, snapshot meaning, or rebuild authority is not a mechanical seam.
