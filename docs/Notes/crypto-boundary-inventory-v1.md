# Crypto Boundary Inventory v1

| Field | Value |
| --- | --- |
| Status | Retained observation; not an architectural decision or audit |
| Observed | 2026-09-03 |
| Scope | Current format-v3 crypto operations, entropy, key residence, failures, and evidence |
| Owner | AR-0016 Gate C0 |

This inventory records the current implementation before a cryptographic
provider seam is introduced. It does not establish cryptographic correctness,
secure erasure, provider independence, module validation, or compliance.

## Durable Construction Inventory

| Operation | Current construction | Durable inputs or domain | Success result | Normalized failure |
| --- | --- | --- | --- | --- |
| DEK generation | `getrandom`, 32 bytes | New database creation | Raw `[u8; 32]` DEK | `RngFailed` |
| Page nonce | `getrandom`, 12 bytes | One nonce per page encryption | `[u8; 12]` | `RngFailed` |
| Subkey derivation | HKDF-SHA256, no salt | DEK; labels `tosumu/v1/page`, `tosumu/v1/header-mac`, `tosumu/v1/audit` | Three raw 32-byte keys | Infallible for fixed lengths |
| Page protection | ChaCha20-Poly1305 | Key; random nonce; AAD = `pgno LE || page_version LE || page_type`; 4,056-byte plaintext | 4,096-byte frame | `EncryptFailed` / `AuthFailed { pgno }` / bounded `Corrupt` checks |
| Passphrase KEK | Argon2id v0x13 | Passphrase; 16-byte salt; encoded m/t/p; 32-byte output | Raw `[u8; 32]` KEK | `InvalidArgument` at direct API; slot scan may collapse to `WrongKey` |
| Recovery KEK | Base32 decode then HKDF-SHA256 | Normalized 20-byte secret; label `tosumu/v1/recovery-kek` | Raw `[u8; 32]` KEK | `WrongKey` |
| Keyfile KEK | Exact raw bytes | File content must be 32 bytes | Raw `[u8; 32]` KEK | I/O or `InvalidArgument` |
| DEK wrapping | ChaCha20-Poly1305 | KEK; random nonce; AAD = `tosumu/v1/wrap || slot_index LE || dek_id LE || kind` | 12-byte nonce and 48-byte wrapped DEK | `RngFailed`, `EncryptFailed`, or `WrongKey` |
| KCV | ChaCha20-Poly1305 | KEK; zero nonce; 16 zero bytes; AAD `tosumu/v1/kcv` | Deterministic 32 bytes | Verification returns `WrongKey` |
| Header authentication | HMAC-SHA256 | Header plain region followed by active keyslot region | 32-byte MAC | `AuthFailed { pgno: None }` |
| Recovery-secret generation | `getrandom`, 20 bytes; Base32 without padding | New recovery protector | Four groups of eight uppercase characters | Currently panics on entropy failure |

## Entropy Call Sites

| Owner | Purpose | Current behavior |
| --- | --- | --- |
| `SystemEntropy::dek` | Database DEK | Fallible; maps to `RngFailed` |
| `SystemEntropy::nonce` | Page and wrap nonces | Fallible; maps to `RngFailed` |
| `SystemEntropy::recovery_secret_bytes` | Recovery secret | Preserves the infallible public signature and panic on source failure |
| `SystemEntropy::passphrase_salt` | Initial, added, and replacement protector salts | Fallible; maps to `RngFailed` |
| `SystemEntropy::database_identifier_seed` | Nonzero `dek_id` seed | Fallible; maps to `RngFailed` |

The initial ADR-0010 extraction now routes all Tosumu-owned source calls to
`getrandom` through this purpose-named private facade. It does not add runtime
selection, deterministic production entropy, or an entropy-quality claim.

## Secret Residence And Copy Inventory

| Secret or capability | Residence | Copy/lifetime observation |
| --- | --- | --- |
| DEK | Create/unlock/protector stack arrays; plaintext sentinel keyslot or wrapped keyslots | Copied into subkey derivation and protector operations; no Tosumu-owned zeroization observed |
| Page key | `Pager`, snapshot handle, and `RebuildContext` raw arrays | Retained for handle/snapshot/rebuild lifetime and copied because the representation is `Copy` |
| Header-MAC key | Optional raw array in `Pager` and `RebuildContext`; local in protector edits | Retained for encrypted handle lifetime; copied into rebuild context |
| Audit key | Local result of `derive_subkeys` | Derived but currently unused/discarded |
| Passphrase KEK | Local raw array | Re-derived while scanning slots; no explicit zeroization observed |
| Recovery/keyfile KEK | Local raw array | Keyfile path first allocates file bytes in a `Vec`; no explicit zeroization observed |
| Passphrase | Borrowed `&str` | Caller owns original storage and lifetime |
| Recovery secret | Owned `String` at generation; borrowed on use | Caller and intermediate strings own copies; normalization creates another string |
| Plaintext page | Fixed arrays returned inside pager closures | Pager bounds exposure but does not establish erasure after use |

The existing direct `zeroize` dependency is not evidence for any lifecycle
property because no Tosumu source use was observed.

## Call-Path Inventory

| Path | Operations involved |
| --- | --- |
| Sentinel create/open | DEK entropy, subkey derivation, plaintext DEK keyslot, page crypto |
| Encrypted create/open | DEK/salt/identifier entropy, Argon2id, KCV, wrap/unwrap, subkeys, header MAC, page crypto |
| Recovery-key open | Base32 normalization, recovery HKDF, KCV, unwrap, subkeys, header MAC, page crypto |
| Keyfile open | File read/length validation, KCV, unwrap, subkeys, header MAC, page crypto |
| Protector add/remove/rekey | Existing unlock, new entropy/KDF as applicable, wrap/KCV, header MAC, page-0 atomic write |
| Snapshot read | Retained page key, page authentication/decryption, header-MAC verification |
| WAL recovery/checkpoint | Encoded frames in WAL; pager authenticates recovered/visible frames and page 0 |
| Stable backup/export | Copies or rebuilds authenticated storage under existing pager/unlock behavior |
| VACUUM rebuild | Copies page-0 protector state and derived keys; decrypts source and re-encrypts staging pages with new nonces |
| Inspect/verify | Opens through pager; receives authenticated plaintext or structured failure |

## Existing Evidence And Gaps

| Concern | Retained evidence | Gap before private seam |
| --- | --- | --- |
| HKDF | Exact expected bytes for all three labels plus determinism/separation tests | Independent implementation comparison |
| Page AEAD | Fixed-nonce full-frame hash/prefix/suffix and decode plus round-trip/AAD rejection | Independent implementation comparison and controlled production-encrypt comparison after a seam exists |
| Argon2id | Exact reduced-cost fixture plus determinism/salt separation | Default-cost exact fixture if its test cost is admitted; independent comparison |
| Wrap | Fixed-nonce exact wrapped bytes, successful unwrap, and AAD/key rejection | Independent implementation comparison |
| KCV | Exact expected 32 bytes, verification, and determinism | Independent implementation comparison |
| Header MAC | Exact expected 32 bytes plus round-trip/keyslot tamper rejection | Independent implementation comparison |
| Recovery KEK | Exact expected 32 bytes plus parsing, normalization, determinism, and separation | Independent implementation comparison |
| File behavior | encrypted lifecycle, hostile input, recovery, snapshots, rebuild | A named conservation matrix comparing exact observable outcomes across extraction |
| Provider independence | RustCrypto implementation only | Second implementation or fixture consumer |
| Secret lifecycle | Type/call-site observation | No erasure, opaque-handle, dump/swap, or provider-lifetime evidence |

## Conservation Baseline Required

The first implementation slice must preserve:

- all format-v3 field offsets, sizes, reserved bytes, algorithms, parameters,
  labels, AAD ordering, endian encoding, and keyslot coverage;
- random-byte purpose and length without promising identical random output;
- exact deterministic outputs for fixed inputs;
- decrypt/verify acceptance of retained fixed ciphertext and MAC fixtures;
- existing public APIs and visibility unless separately reviewed;
- existing structured errors and current slot-scanning behavior;
- pager authentication ownership, WAL frame meaning, snapshot selection,
  recovery behavior, and rebuild publication;
- native and browser-WASM behavior; and
- the current security claim ceiling in `SECURITY.md`.

Known defects or awkward behavior, including the recovery-secret entropy panic,
must be recorded and corrected separately rather than disappearing inside the
mechanical seam change.

The exact vectors are implemented by
`crypto::tests::gate_c0_fixed_construction_vectors`. They pin current behavior
for refactoring; because their expected outputs were captured from the current
implementation, they are not independent evidence that the construction or
implementation is cryptographically correct.

The companion `crypto-file-conservation-matrix-v1.md` names the file-level
tests that preserve randomized creation, unlock, mutation, recovery, snapshot,
protector, inspection, and rebuild behavior.
