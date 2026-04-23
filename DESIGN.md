# tosumu — Design Document

**Status:** Draft v0.1
**Project type:** Academic / learning
**Language:** Rust (stable)
**Target:** Single-file, single-process, embedded, page-based, authenticated-encrypted key/value store with an eventual toy SQL layer.

> **Name.** `tosumu` (written `to-su-mu`) is a conlang word meaning *knowledge-organization device* — literally "database." Components: `to` (knowledge / information) + `su` (organized structure) + `mu` (object / device). See §16.
>
> **Published at** https://github.com/Arakendo/tosumu. Dual-licensed MIT OR Apache-2.0. This is a public learning project: the crypto and storage design are documented, but neither has been independently reviewed or audited. Do not use `tosumu` to protect real secrets — see [`SECURITY.md`](SECURITY.md).

---

## 1. Goals and non-goals

### 1.1 Goals

- Learn, hands-on, how real embedded storage engines are built: pages, records, B+ trees, WAL, crash recovery.
- Produce a **small, finishable** engine. Correctness and clarity beat performance.
- Apply **per-page authenticated encryption** (AEAD) as a first-class concern, not a bolt-on.
- Be testable: deterministic, fuzzable, property-checkable.
- Be a single binary + a library crate.

### 1.2 Non-goals

- SQL completeness. No joins, no planner, no optimizer beyond trivial.
- Multi-process access. Single process, single writer.
- Networked / client-server operation.
- Feature parity with SQLite. We are *inspired by* SQLite, not cloning it.
- High performance. We will measure it, but we will not chase it.
- Portability exotica. Little-endian, 64-bit, POSIX-or-Windows file semantics.
- Advanced indexing (FSTs, full-text search, vector/embedding search, fuzzy matching). See §17 for why these are out of scope and how to use specialized tools if you need them.

### 1.3 Explicit "out of scope until proven necessary"

- Multiple tables / schemas (Stage 5 only).
- Secondary indexes (Stage 5+).
- Concurrency beyond single writer + multiple readers (Stage 6+, maybe).
- Compression.
- Replication.

---

## 2. Guiding principles

1. **Finishable by a mortal.** Every stage must produce something runnable and testable on its own.
2. **On-disk format is sacred.** The file format is documented before it is coded. Every byte has a reason.
3. **Crypto binds structure.** AEAD AAD must cover anything that would be dangerous to swap, reorder, or roll back.
4. **No silent corruption.** Any integrity failure surfaces as a typed error. Never "just a weird byte."
5. **Types over comments.** Layout is expressed in `#[repr(C)]` structs and enums, not prose.
6. **Tests before cleverness.** Property tests and fuzzers land with the module they test.

---

## 3. Technology choices

| Concern | Choice | Why |
|---|---|---|
| Language | Rust (stable, edition 2021) | Layout control + memory safety + crypto ecosystem. |
| Byte layout | `bytemuck` (primary) or `zerocopy` | Safe zero-copy cast of page bytes to typed headers. |
| Errors | `thiserror` | Typed, non-panicking error surface. |
| AEAD | `chacha20poly1305` (primary), `aes-gcm` (alt) | Audited RustCrypto crates. ChaCha20-Poly1305 chosen by default: constant-time on all CPUs, no AES-NI dependency. |
| RNG | `rand_core::OsRng` | OS CSPRNG for key/nonce salts. |
| KDF | `argon2` | Password → master key. |
| Logging | `tracing` + `tracing-subscriber` | Structured, level-filtered. |
| Testing | `proptest`, built-in `#[test]`, `cargo fuzz` (libFuzzer) | Property + fuzz coverage of decoders. |
| Benches | `criterion` | Only used for "is this obviously terrible?" checks. |
| CLI | `clap` (derive) | Standard, minimal. |

No async. The engine is synchronous. If we ever want async, we wrap at the edges.

---

## 4. Architecture overview

```
┌─────────────────────────────────────────────┐
│                    CLI                      │   bin/tosumu
├─────────────────────────────────────────────┤
│              Query layer (Stage 5)          │   parser, executor
├─────────────────────────────────────────────┤
│                  B+ Tree                    │   btree.rs
├─────────────────────────────────────────────┤
│         Transaction / WAL manager           │   wal.rs, txn.rs
├─────────────────────────────────────────────┤
│                   Pager                     │   pager.rs  (cache, dirty set)
├─────────────────────────────────────────────┤
│            Crypto layer (AEAD)              │   crypto.rs (transparent to pager)
├─────────────────────────────────────────────┤
│              File I/O + Page                │   page.rs, io.rs
└─────────────────────────────────────────────┘
```

Each layer only talks to the one directly below it. The crypto layer sits **between** the pager's cache and the file I/O: cached pages are plaintext; on-disk pages are ciphertext. Page numbers and versions are bound as AAD.

---

## 5. On-disk format

### 5.1 File layout

```
┌──────────────────┐  offset 0
│  File header     │  page 0  (partially plaintext; see §8)
├──────────────────┤
│  Page 1          │
├──────────────────┤
│  Page 2          │
├──────────────────┤
│      ...         │
└──────────────────┘
```

- Page size: **4096 bytes**, fixed at creation, stored in header.
- All integers little-endian.
- All offsets are byte offsets within a page unless stated otherwise.
- Every page after the header is either: leaf, internal, overflow, free, or WAL (WAL lives in a separate file in early stages — see §7).

### 5.2 File header (page 0)

Fixed layout. Plaintext fields are readable without any key so we can refuse to open wrong-version files, and so a user can enumerate which **key protectors** are configured before attempting to unlock.

| Offset | Size | Field | Notes |
|---|---|---|---|
| 0 | 16 | `magic` | ASCII `"TOSUMUv0\0......."` — 8 bytes of tag + NUL + 7 reserved bytes, zero-padded |
| 16 | 2 | `format_version` | what this file *is* (see §12) |
| 18 | 2 | `page_size` | 4096 |
| 20 | 2 | `min_reader_version` | lowest engine `format_version` allowed to open this file (see §12.1) |
| 22 | 2 | `flags` | bit 0 = encrypted; bit 1 = has keyslots |
| 24 | 8 | `page_count` | total pages including header |
| 32 | 8 | `freelist_head` | page number or 0 |
| 40 | 8 | `root_page` | B+ tree root (Stage 2) |
| 48 | 8 | `wal_checkpoint_lsn` | last durable LSN |
| 56 | 8 | `dek_id` | monotonic id of the currently-active DEK (for rotation, Stage 4b+) |
| 64 | 16 | `dek_kat` | AEAD of a fixed known-plaintext under the DEK; cheap wrong-DEK detection |
| 80 | 2 | `keyslot_count` | number of protector slots present (0 if unencrypted) |
| 82 | 2 | `keyslot_region_pages` | how many pages after page 0 hold the keyslot region |
| 84 | 20 | reserved | zero-filled |
| 104 | 32 | `header_mac` | HMAC-SHA256 over bytes `0..104` **and** the full keyslot region, using `header_mac_key` |
| 136 | ... | reserved / zero | pads to end of page 0 |

Everything after page 0 and the **keyslot region** uses the page frame in §5.3. The keyslot region (§8.7) is plaintext structured data, not encrypted pages, but it is covered by `header_mac`.

### 5.3 Page frame (encrypted pages)

```
┌─────────────────────────────────────────┐
│ nonce         (12 bytes, plaintext)     │
│ page_version  (8 bytes,  plaintext)     │  ← monotonic per page; also bound as AAD
│ ciphertext    (page_size - 12 - 8 - 16) │
│ auth_tag      (16 bytes)                │
└─────────────────────────────────────────┘
```

- AEAD: ChaCha20-Poly1305.
- **AAD** = `page_number (u64 LE) || page_version (u64 LE) || page_type (u8)`.
- Binding page number prevents an attacker from swapping ciphertext blobs between slots.
- Binding page version prevents rollback of a **single** page to an older valid ciphertext.
- Nonce strategy: **random 96-bit nonce per write**. With Poly1305's 2^32 safe-use limit per key, we're effectively unbounded for an engine at this scale; we still track a `page_version` for per-page rollback protection.

> **Known limitation — consistent multi-page rollback.** Per-page `page_version` does *not* prevent an attacker from rolling back *several* pages to a mutually consistent earlier snapshot. Detecting that requires either a global LSN bound into every page's AAD, a Merkle root stored in the header, or a checkpoint-signed manifest. This is explicitly deferred. Stage 6 or later may introduce a global LSN in the AAD; it is a non-goal for Stages 1–5. Future-us: do not feel clever about `page_version` beyond what it actually does.

> **Nonce strategy — future option.** `random 96-bit` is simple and safe for our write volumes. If operational reasoning becomes annoying (e.g. during crash/WAL replay analysis), the migration target is `random_prefix (64 bits) || monotonic_counter (32 bits)` per key. Documented here so we don't rediscover it at 2am.

When encryption is disabled (`flags bit 0 = 0`), the entire 4096 bytes is the plaintext page body. The nonce/version/tag fields are absent, and a CRC32C in the page header provides integrity only (see §14 Q4). This mode exists for Stages 1–3.

### 5.4 Slotted page (leaf data pages)

Inside the plaintext page body for leaf pages:

```
┌───────────────────────┐  offset 0
│ page header (fixed)   │
├───────────────────────┤
│ slot array (grows →)  │
│        ...            │
│                       │
│        ...            │
│ record heap (← grows) │
└───────────────────────┘  offset page_body_size
```

Page header (fixed, `#[repr(C)]`):

| Size | Field |
|---|---|
| 1 | `page_type` (1=leaf, 2=internal, 3=overflow, 4=free) |
| 1 | `flags` |
| 2 | `slot_count` |
| 2 | `free_start` (end of slot array) |
| 2 | `free_end` (start of heap, growing down) |
| 2 | `fragmented_bytes` (see §5.4.1) |
| 4 | reserved |
| 8 | `rightmost_child` or `next_leaf` depending on type |

#### 5.4.1 Fragmentation and compaction policy

`fragmented_bytes` is the count of bytes inside the record heap that are no longer referenced by any live slot (produced by deletes and in-place shrinks). Compaction rewrites the heap to reclaim this space.

Policy:

- A page is **eligible for compaction** when `fragmented_bytes >= page_body_size / 4`.
- Compaction is triggered **lazily on write**: before an insert/update that would otherwise fail with `OutOfSpace`, the pager tries compacting the target page first. No background sweeper.
- Compaction is a full heap rewrite: copy live records to a scratch buffer in slot order, reset `free_end`, rewrite slots, zero `fragmented_bytes`.
- Stage 1 may **skip `fragmented_bytes` entirely** and recompute live/dead bytes on demand during compaction. Tracking it in the header is a Stage 2+ optimization, not a Stage 1 requirement. (See §11.1.)

#### 5.4.2 Value size cap (Stage 1)

Stage 1 rejects any record where `key_len + value_len + record_overhead > page_body_size / 2` with `InvalidArgument`. This defers overflow pages cleanly to Stage 2 without the record encoder having to care. The cap is relaxed in Stage 2 when overflow chains land.

Slot entry: `{ offset: u16, length: u16 }`.

Record encoding (Stage 1, single table):
- `key_len: varint`
- `value_len: varint`
- `key_bytes`
- `value_bytes`

Records larger than some threshold (e.g. `page_body_size / 4`) spill to overflow pages via a chain. Not implemented until Stage 2.

### 5.5 Internal page (B+ tree, Stage 2)

Same slotted layout, slot payload is `{ separator_key, child_page_no: u64 }`. A dedicated `rightmost_child` lives in the page header.

### 5.6 Free page

A free page's body is a single `next_free: u64`. Linked list rooted at `freelist_head` in the file header.

---

## 6. Pager

### 6.1 Responsibilities

- Allocate / free pages.
- Read a page: decrypt, verify AAD, cache.
- Write a page: bump version, encrypt, enqueue for flush.
- Maintain a bounded in-memory cache of plaintext pages.
- Track dirty pages for the current transaction.
- Hand out **typed views** into cached pages; never raw pointers.

### 6.2 API sketch

```rust
pub struct Pager { /* ... */ }

pub struct PageRef<'a>    { /* &immutable view */ }
pub struct PageRefMut<'a> { /* &mut view, marks dirty on drop */ }

impl Pager {
    pub fn open(path: &Path, key: Option<&Key>) -> Result<Self>;
    pub fn get(&self, pgno: u64) -> Result<PageRef<'_>>;
    pub fn get_mut(&self, pgno: u64) -> Result<PageRefMut<'_>>;
    pub fn allocate(&self, page_type: PageType) -> Result<u64>;
    pub fn free(&self, pgno: u64) -> Result<()>;
    pub fn flush(&self) -> Result<()>;   // called by txn commit
    pub fn close(self) -> Result<()>;
}
```

Interior mutability via `RefCell` / `parking_lot::Mutex` depending on concurrency stage. Single-writer assumption keeps this honest.

> **Risk — borrow-checker fight.** Returning `PageRef<'_>` / `PageRefMut<'_>` tied to `&self` with interior mutability often collapses into lifetime pain once the B+ tree starts holding references into two pages at once (e.g. during a split). **Fallback design if this gets ugly:** switch to a handle-based API where `get` / `get_mut` return an owned `PageHandle(u64, Generation)` and all reads/writes go through short-lived closures:
>
> ```rust
> pager.with_page(pgno, |view| { ... })?;
> pager.with_page_mut(pgno, |view| { ... })?;
> ```
>
> This trades some ergonomics for zero lifetime gymnastics and is the known escape hatch. Decision deferred until Stage 2 actually needs cross-page references.

### 6.3 Cache

- Fixed-size LRU (`N` frames, e.g. 256).
- Plaintext only.
- Dirty pages cannot be evicted until flushed.

---

## 7. Transactions & WAL

### 7.1 Model

- **Single writer**, multiple logical readers (Stage 3+).
- Explicit `begin / commit / rollback`.
- Durability on commit: WAL fsync'd before returning.

### 7.2 WAL format (append-only, separate file `tosumu.wal`)

Each record:

```
┌─────────────┬──────────┬───────────┬────────┐
│ lsn (u64)   │ type (u8)│ payload   │ crc32c │
└─────────────┴──────────┴───────────┴────────┘
```

Record types:
- `Begin { txn_id }`
- `PageWrite { pgno, new_page_version, ciphertext_blob }` — full-page physical logging, Stage 3.
- `Commit { txn_id }`
- `Checkpoint { up_to_lsn }`

Physical logging (full page images) is chosen over logical logging for simplicity and because it composes cleanly with per-page AEAD: the WAL stores encrypted page frames identical to the ones that will land in the main file.

### 7.3 Recovery

On open:
1. Read file header, verify MAC.
2. Scan WAL from `wal_checkpoint_lsn` forward.
3. For every committed transaction, replay `PageWrite` records into the main file.
4. Discard records belonging to uncommitted transactions.
5. Advance checkpoint, truncate WAL.

Torn-write safety: a `PageWrite` is only applied if its CRC and AEAD tag both verify.

---

## 8. Cryptography

### 8.1 Threat model

**In scope:**
- Attacker with read/write access to the database file at rest.
- Attacker attempting page swap, page rollback, page reorder, truncation, or bit-flipping.
- Attacker attempting to detect *whether* a page changed (limited; sizes and access patterns leak — see out of scope).

**Out of scope:**
- Attacker with memory access to the running process.
- Side channels (cache timing, power).
- Traffic analysis of file modification patterns.
- Plaintext recovery from swap / hibernation files.

### 8.2 Key hierarchy (envelope encryption)

tosumu uses standard **envelope encryption**: page data is encrypted with a random per-database **DEK**, and the DEK is wrapped by one or more **KEKs**, each produced by a **protector** (passphrase, recovery key, TPM, …). Unlocking = "a protector releases a KEK, the KEK unwraps the DEK."

```
      protector (passphrase / recovery key / TPM / …)
                         │  produces
                         ▼
                        KEK
                         │  unwraps (AEAD)
                         ▼
                        DEK  ─────┐
                         │       │ HKDF (§8.3)
                 ┌───────┴───────┐
                 ▼                ▼
             page_key       header_mac_key
```

Properties this buys us (all deliberately):

- **Changing a passphrase rewrites one keyslot, not the database.**
- **Recovery = an extra protector**, not a whole parallel crypto scheme.
- **TPM is just another protector**, pluggable behind the same trait.
- **DEK rotation is possible** without page rewrite— sort of. See §8.8 for the honest caveat.

### 8.3 Keys

- **DEK** (`[u8; 32]`): generated from `OsRng` at `init`. Never leaves memory in plaintext. Persisted only as wrapped blobs in keyslots.
- From the DEK, derive two subkeys via HKDF-SHA256 with fixed info strings:
  - `page_key`   = `HKDF(DEK, info = "tosumu/v1/page")`
  - `header_mac_key` = `HKDF(DEK, info = "tosumu/v1/header-mac")`
- Each **protector** produces a 32-byte **KEK** used to AEAD-wrap the DEK into a keyslot.
- All in-memory keys live in `Zeroizing<[u8; 32]>`. Dropped keys are wiped.

### 8.4 Page AEAD

- Algorithm: ChaCha20-Poly1305.
- Key: `page_key` (derived from DEK, see §8.3).
- Nonce: 96-bit random, stored plaintext in the page frame.
- AAD: `pgno || page_version || page_type` (see §5.3).
- On decrypt failure: typed error `CorruptPage { pgno }`. No partial reads.

### 8.5 Header MAC

- The file header and keyslot region are not encrypted (we need `magic`, `format_version`, `page_size`, and protector metadata readable *before* anything is unlocked).
- Integrity via HMAC-SHA256 with `header_mac_key`, covering bytes `0..104` of page 0 **plus every byte of the keyslot region** (§8.7). This closes protector-downgrade / slot-tampering attacks.
- The MAC can only be verified *after* a protector unwraps the DEK, so wrong-protector detection is strong: wrong passphrase → DEK wrap tag fails **or** MAC mismatch → refuse to open.

### 8.6 Protectors

Every unlock path is a `KeyProtector`. Multiple protectors can coexist; any one of them can unlock the database.

```rust
pub trait KeyProtector {
    /// Given on-disk metadata and any user-supplied secret,
    /// derive the 32-byte KEK for this protector.
    fn derive_kek(
        &self,
        meta: &ProtectorMetadata,
        input: &ProtectorInput,
    ) -> Result<Zeroizing<[u8; 32]>>;
}
```

Initial protector types:

| Kind | Stage | Notes |
|---|---|---|
| `Passphrase` | 4a | Argon2id over passphrase + per-slot salt. |
| `RecoveryKey` | 4b | 256-bit random secret, shown to user once at init; encoded as a groups-of-6 Base32 string. |
| `Keyfile` | 4b (optional) | Raw 32 bytes read from a file path. |
| `Tpm` | 4c | Platform-backed; seals KEK to a TPM policy. Feature-flagged, not required to build tosumu. |
| `TpmPlusPin` | 4c | Combines a TPM-sealed secret with a user PIN through Argon2id. |

Protectors live behind a trait object; the storage engine never sees protector-specific fields.

### 8.7 Keyslot region (on-disk)

The keyslot region is a contiguous run of `keyslot_region_pages` pages immediately after page 0. It is a flat array of fixed-size **keyslots**. Non-populated slots are zeroed and marked `Empty`.

One keyslot (256 bytes, exact layout TBD during Stage 4a):

| Size | Field | Notes |
|---|---|---|
| 1 | `kind` | 0=Empty, 1=Passphrase, 2=RecoveryKey, 3=Keyfile, 4=Tpm, 5=TpmPlusPin |
| 1 | `version` | protector format version |
| 2 | `flags` | e.g. "requires PIN", "recovery-only" |
| 4 | `created_unix` | u32 seconds since epoch, for rotation diagnostics |
| 8 | `dek_id` | which DEK generation this slot wraps (§8.8) |
| 16 | `salt` | per-slot salt for KDF-based protectors |
| 32 | `kdf_params` | Argon2id params: m, t, p, version (packed) |
| 32 | `tpm_policy` | opaque to the core crate; interpreted by `TpmProtector` |
| 12 | `wrap_nonce` | ChaCha20-Poly1305 nonce for wrapping the DEK |
| 48 | `wrapped_dek` | 32-byte DEK ciphertext + 16-byte tag |
| 32 | `kek_kcv` | AEAD tag over a fixed known-plaintext under this KEK; enables "is this the right passphrase" without touching the DEK |
| 68 | reserved | zero-filled; accommodates future protector fields without a format bump |

AAD for DEK wrapping: `"tosumu/v1/wrap" || slot_index (u16 LE) || dek_id (u64 LE) || kind (u8)`. This binds each wrapped DEK to its slot and generation so an attacker cannot swap wrapped blobs between slots or replay an old slot from a previous rotation.

#### 8.7.1 Policy metadata

Slot `flags` expresses lightweight local policy, authenticated by `header_mac`:

- `recovery_only` — slot may only unlock if passphrase/TPM slots have failed N times (enforced by the CLI, not cryptographically).
- `require_pin` — TPM slot insists on a PIN.
- `created_by_rotation` — slot was added as part of a KEK rotation and is safe to retire after confirmation.

This is policy, not cryptography. It is documented as such.

### 8.8 Rotation

- **KEK rotation (cheap).** Re-wrap the DEK under a new KEK, write the result into a new keyslot (or overwrite the target slot), update `header_mac`. No page rewrite.
- **DEK rotation (expensive).** Generate a new DEK, bump `dek_id`, re-encrypt every page. Provided as a single `tosumu rekey` operation; runs offline. Stage 4b deliverable only if time permits — otherwise Stage 6.
- Per-page AAD does **not** currently include `dek_id`. A page encrypted under a previous DEK is distinguishable only by AEAD failure under the new DEK. That's adequate for a full-file rekey that runs to completion atomically (via WAL), but it is *not* adequate for incremental/online rekey. Called out so we don't quietly assume otherwise.

### 8.9 Nonce reuse risk

Random 96-bit nonces have a birthday bound around 2^48 encryptions per key before collision probability becomes meaningful. Acceptable for a toy engine. Documented so future-us doesn't re-derive it at 2am.

### 8.10 Known limitations (explicit)

- **Consistent multi-page rollback** is not detected. See §5.3.
- **DEK/KEK split does not protect against a compromised running process.** If malware can read process memory, it has the DEK. Envelope encryption protects *at rest*, not *at runtime*.
- **TPM protector does not imply remote attestation.** Sealing to a TPM policy proves "this machine in this state" locally; it says nothing to a remote verifier. Not a goal.
- **Recovery key secrecy is the user's problem.** If the recovery string is stored next to the database file, the recovery protector adds zero security. Documented in the CLI output at init time.

### 8.11 What is *not* protected

- The *existence* and *size* of the database.
- The number of pages.
- Which pages changed between two snapshots (access pattern leakage).
- The order and timing of writes.
- Anything readable from process memory while the database is open.

These are called out explicitly so the threat model is honest.

---

## 9. Error model

One top-level `Error` enum via `thiserror`. Variants include:

- `Io(std::io::Error)`
- `Corrupt { pgno: u64, reason: &'static str }`
- `AuthFailed { pgno: Option<u64> }`
- `WrongKey`
- `NoProtectorAccepted` — tried every configured protector, none produced a valid DEK
- `ProtectorUnavailable(&'static str)` — e.g. TPM not present on this machine
- `KeyslotTampered { slot: u16 }` — header MAC mismatch localized to keyslot region
- `VersionMismatch { found: u16, expected: u16 }`
- `NewerFormat { found: u16, supported_max: u16 }` — file is from a newer engine; refuse to open
- `MigrationRequired { from: u16, to: u16 }` — returned by `open_read_only` and by `open` when `auto_migrate = false`
- `MigrationFailed { step: &'static str, reason: String }`
- `OutOfSpace`
- `TxnConflict`
- `InvalidArgument(&'static str)`

No `unwrap` / `panic` on user-controlled input paths. Panics are reserved for "the programmer wrote a bug" invariants.

---

## 10. Testing strategy

Testing is a first-class concern. A storage engine is only as good as the confidence that it won't corrupt data, and confidence comes from systematic, repeatable, adversarial testing. This section is normative: every module ships with its tests, and every stage gate includes test requirements.

### 10.1 Testing philosophy

- **No untested code paths.** If a function can return an error, there's a test that triggers it.
- **Property tests > example tests.** Arbitrary inputs catch edge cases humans don't think of.
- **Fuzz everything that touches bytes.** Decoders, parsers, and crypto boundaries get fuzz targets.
- **Crash safety is not optional.** Stage 3's `CrashFs` harness runs on every commit after WAL lands.
- **Tests document behavior.** A test name like `test_compaction_preserves_slot_order` is a spec.
- **Realistic, not exhaustive.** This is a learning project. We aim for *high confidence*, not formal proof.

### 10.2 Test categories and organization

Tests live in three places:

| Location | What lives there | Run by |
|---|---|---|
| `crates/*/src/**/*.rs` (inline `#[cfg(test)]`) | Unit tests, doc tests, small property tests. | `cargo test` |
| `tests/*.rs` | Integration tests that exercise the public API across module boundaries. | `cargo test` |
| `fuzz/fuzz_targets/*.rs` | `cargo-fuzz` / libFuzzer targets. | `cargo fuzz run <target>` (manual, not CI) |

### 10.3 Unit tests (inline, per-module)

Standard `#[cfg(test)]` modules in each `.rs` file. Cover:

- **Happy paths.** Basic functionality works.
- **Boundary conditions.** Empty inputs, maximum sizes, alignment edges.
- **Error paths.** Every `Result<_, Error>` return has a test that produces `Err`.
- **Documented invariants.** If a comment says "slot offsets must be ascending," there's a test that violates it and expects a specific error.

**Examples by module:**

- `page.rs` unit tests:
  - `test_slotted_page_layout_roundtrip` — write slots, read them back.
  - `test_slot_insert_at_capacity_fails` — page full → `OutOfSpace`.
  - `test_overlapping_slots_rejected` — malformed page → `Corrupt`.
  - `test_fragmentation_tracking` — delete record → `fragmented_bytes` updated correctly.
- `record.rs` unit tests:
  - `test_varint_encode_decode` — round-trip LEB128 for `0, 1, 127, 128, u64::MAX`.
  - `test_record_with_empty_key` — zero-length keys are legal.
  - `test_record_exceeding_page_size_rejected` — size cap enforced.
- `pager.rs` unit tests:
  - `test_allocate_returns_sequential_pages` — first three allocations return `1, 2, 3`.
  - `test_free_page_reused` — allocate, free, allocate → same page number.
  - `test_dirty_page_not_evicted` — LRU can't evict a dirty page before flush.
  - `test_double_free_panics` — internal invariant; debug_assert caught in tests.

### 10.4 Property tests (`proptest`)

Property tests generate hundreds or thousands of random inputs and assert invariants hold. Ship with the module they test (same `#[cfg(test)]` block or in `tests/`).

**Core properties to test:**

| Module | Property | Generator |
|---|---|---|
| `page.rs` | Encode then decode = identity | Arbitrary `Vec<(key, value)>` that fits in a page |
| `page.rs` | Compaction preserves all records | Random sequence of `insert/delete`, then compact |
| `record.rs` | Varint round-trip for all `u64` | `proptest::num::u64::ANY` |
| `btree.rs` | Tree height is `O(log n)` | Arbitrary insert sequence, check `max_depth <= c * log2(record_count)` |
| `btree.rs` | All keys in sorted iterator order | Insert random keys, iterate, assert sorted |
| `btree.rs` | Tree invariants after deletes | Insert N, delete random subset, check child pointers + key ordering |
| `wal.rs` | Replay is idempotent | Write records, replay, replay again → same final state |
| `crypto.rs` | Decrypt(Encrypt(plaintext)) = plaintext | Arbitrary page bodies + random nonces |

**Shrinking is critical.** When a property test fails, `proptest` shrinks the input to a minimal failing case. That's gold for debugging.

**Example skeleton:**

```rust
#[cfg(test)]
mod proptests {
    use proptest::prelude::*;
    
    proptest! {
        #[test]
        fn page_encode_decode_roundtrip(records in arb_record_vec(1..20)) {
            let mut page = Page::new_leaf();
            for (k, v) in &records { page.insert(k, v)?; }
            
            let bytes = page.to_bytes();
            let page2 = Page::from_bytes(&bytes)?;
            
            for (k, v) in records {
                prop_assert_eq!(page2.get(&k)?, Some(v));
            }
        }
    }
}
```

### 10.5 Fuzz targets (`cargo fuzz`)

Fuzzing is continuous property testing with coverage-guided mutation. Targets live in `fuzz/fuzz_targets/`. Each is a small `fn` that takes `&[u8]` and must not panic.

**Required fuzz targets (one per stage):**

- **Stage 1:** `fuzz_page_decode` — feed arbitrary 4 KB blobs to the page decoder. Must return `Ok(page)` or `Err(Corrupt)`, never panic.
- **Stage 2:** `fuzz_btree_operations` — parse a bytestream as a sequence of `Insert(k,v) | Delete(k) | Get(k)` ops. Tree must stay consistent.
- **Stage 3:** `fuzz_wal_replay` — arbitrary WAL file → replay must not panic, only `Err(Corrupt)`.
- **Stage 4:** `fuzz_aead_frame` — feed arbitrary ciphertext blobs to the AEAD unwrap. Must return `Ok` or `AuthFailed`, never panic or leak plaintext.
- **Stage 4:** `fuzz_keyslot_parse` — arbitrary keyslot region bytes. Must not panic.

**Corpus management:** Seed each target with a few valid examples (checked into `fuzz/corpus/<target>/`). After finding a crash, add the minimized input as a regression test.

**CI integration:** Fuzz targets are **not** run in CI (too slow). They run manually before each stage release: `cargo fuzz run <target> -- -max_total_time=300` (5 min per target). Findings block release.

### 10.6 Integration tests (`tests/*.rs`)

Integration tests exercise the public API as a user would. They test cross-module interactions, not implementation details.

**Test file structure:**

```
tests/
├── stage1_storage.rs         # init, put, get, scan, reopen
├── stage2_btree.rs            # inserts/deletes with tree, range scans
├── stage3_transactions.rs     # begin/commit/rollback, crash recovery
├── stage4_encryption.rs       # encrypted DB open/close, protector unlock
├── stage4_keyslots.rs         # multiple protectors, rotation
├── fixtures/
│   ├── v1_unencrypted.tsm     # known-good DB from Stage 1
│   ├── v2_with_btree.tsm      # known-good DB from Stage 2
│   └── v3_encrypted.tsm       # known-good encrypted DB
└── common/
    └── mod.rs                 # shared test utilities
```

**Example tests:**

```rust
// tests/stage1_storage.rs
#[test]
fn test_reopen_preserves_data() {
    let path = tempfile::NamedTempFile::new().unwrap().into_temp_path();
    {
        let db = Database::create(&path).unwrap();
        db.put(b"key1", b"value1").unwrap();
        db.close().unwrap();
    }
    {
        let db = Database::open(&path).unwrap();
        assert_eq!(db.get(b"key1").unwrap(), Some(b"value1".to_vec()));
    }
}

#[test]
fn test_scan_returns_keys_in_insertion_order() {
    // Stage 1 has no B+ tree; scan is page order, which is insertion order
    let db = Database::create_temp().unwrap();
    db.put(b"zebra", b"z").unwrap();
    db.put(b"apple", b"a").unwrap();
    let keys: Vec<_> = db.scan().unwrap().map(|(k,_)| k).collect();
    assert_eq!(keys, vec![b"zebra", b"apple"]);
}
```

### 10.7 Crash simulation (`CrashFs` harness)

**Goal:** Prove that recovery is correct for every possible crash point during a transaction.

**Implementation (Stage 3):** A `CrashFs` struct wraps `std::fs::File` and intercepts `write()`, `flush()`, `sync_all()`. It maintains a log of pending operations and can:

- Truncate a write at byte N (simulates partial write).
- Drop the last M bytes of a region marked durable (simulates fsync lie).
- Reorder queued writes (simulates out-of-order completion).
- Inject a "crash" at an arbitrary point in a test, then reopen the DB and verify consistency.

**Test structure:**

```rust
#[test]
fn test_recovery_after_commit_interrupted() {
    let fs = CrashFs::new();
    let db = Database::open_with_fs(&fs, "test.tsm").unwrap();
    
    db.begin().unwrap();
    db.put(b"k1", b"v1").unwrap();
    
    // Inject crash during commit WAL fsync
    fs.crash_after_next_sync();
    let _ = db.commit(); // may fail
    drop(db);
    
    // Recovery: DB must either have the committed write or not, never partial
    let db2 = Database::open_with_fs(&fs, "test.tsm").unwrap();
    let val = db2.get(b"k1").unwrap();
    assert!(val == Some(b"v1".to_vec()) || val == None);
}
```

**Coverage target:** Crash at every await point in the commit path (10–20 injection sites). All must leave the DB consistent.

### 10.8 Known-answer tests (KATs) — crypto

KATs prevent accidental changes to cryptographic constructions. Each one specifies:

- Fixed inputs (plaintext, key, nonce, AAD).
- Expected ciphertext and auth tag.
- If output changes, the test fails → forces explicit acknowledgment in a commit.

**Required KATs:**

- Page AEAD: one plaintext page → fixed `page_key` → check ciphertext and tag.
- HKDF subkey derivation: fixed DEK → check that `page_key` and `header_mac_key` match known values.
- Argon2id KEK derivation: fixed passphrase + salt → check KEK output.
- DEK wrapping: fixed DEK + KEK → check wrapped blob + tag.
- Header MAC: fixed header bytes + known `header_mac_key` → check HMAC output.

KATs live in `crypto.rs` as unit tests with hardcoded hex constants.

### 10.9 Fixtures and golden files

Golden files are checked-in database files with known contents. They serve two purposes:

1. **Regression:** Load a v1 DB, verify it still opens and returns expected data.
2. **Migration testing:** Start with a `v1_unencrypted.tsm`, run migration, verify output matches `v2_expected.tsm`.

**Fixture naming convention:**

```
tests/fixtures/
├── v1_empty.tsm                # Format v1, no records
├── v1_with_3_records.tsm       # Format v1, 3 known key/value pairs
├── v2_btree_100_keys.tsm       # Format v2, 100 sequential keys
├── v4_encrypted_passphrase.tsm # Format v4, encrypted, passphrase = "test"
└── golden_outputs/
    └── after_v1_to_v2_migration.tsm
```

Fixtures are **small** (< 100 KB each), committed with Git LFS if they grow.

**Test example:**

```rust
#[test]
fn test_open_v1_fixture() {
    let db = Database::open("tests/fixtures/v1_with_3_records.tsm").unwrap();
    assert_eq!(db.get(b"key1").unwrap(), Some(b"val1".to_vec()));
    assert_eq!(db.get(b"key2").unwrap(), Some(b"val2".to_vec()));
    assert_eq!(db.get(b"key3").unwrap(), Some(b"val3".to_vec()));
}
```

### 10.10 Stage-specific acceptance tests

Every stage has a checklist of acceptance tests that must pass before the stage is marked "done." These are integration tests + manual CLI invocations.

**Stage 1 acceptance:**

- `cargo test --workspace` passes (all unit + integration tests).
- `tosumu init test.tsm && tosumu put test.tsm k1 v1 && tosumu get test.tsm k1` returns `v1`.
- `tosumu dump test.tsm` pretty-prints the header and page 1.
- `tosumu verify test.tsm` exits 0.
- Proptest for page encode/decode runs 10,000 cases without failure.
- Fuzz `fuzz_page_decode` for 5 minutes without finding a panic.

**Stage 2 acceptance:**

- Stage 1 tests still pass.
- Insert 10,000 sequential keys, verify tree height ≤ 5.
- Delete 5,000 random keys, verify remaining 5,000 are retrievable.
- Range scan returns keys in sorted order.

**Stage 3 acceptance:**

- All prior tests pass.
- `CrashFs` tests: inject crash at 20 commit-path sites → recovery always consistent.
- WAL replay fuzz target runs 5 min clean.

**Stage 4 acceptance:**

- All prior tests pass.
- Open encrypted DB with wrong passphrase → `WrongKey` (not panic, not partial plaintext).
- KATs for all crypto constructions pass.
- Rotate KEK → old passphrase fails, new passphrase succeeds.
- Add recovery key, delete passphrase slot → recovery key unlocks.

### 10.11 Performance and regression testing

Performance is **not** a primary goal, but catastrophic regressions are worth catching.

**What to track (starting Stage 2):**

- Throughput: inserts/sec for sequential keys (baseline: ~10K/sec on reference hardware).
- Latency: p50/p99 for `get` on a 100K-key DB (baseline: <100μs / <500μs).
- File size: DB with 10K × (32-byte key + 128-byte value) (baseline: <5 MB).

**Tooling:** `criterion` benchmarks in `benches/`. Run manually before releases; commit results to `benches/results/`. No CI gating (too noisy).

**Regression policy:** A 2× slowdown or file-size bloat is a blocker. A 10% change is noted but not a blocker.

### 10.12 Test coverage

**Target: line coverage ≥ 80% in `tosumu-core`.** This is realistic for a solo project without being a chore.

**How:**

- Run `cargo tarpaulin` or `cargo llvm-cov` locally before each stage release.
- Uncovered lines are either:
  - Unreachable (debug assertions, internal panics) — mark with `// coverage:ignore`.
  - Missing tests — add them.

**No CI gating on coverage.** Coverage is a diagnostic, not a gate. Human judgment is required.

### 10.13 What is *not* tested

Honest list of what this testing strategy does not cover:

- **Concurrency bugs** — Stage 1–5 are single-writer; no race-condition testing until Stage 6.
- **Long-running soak tests** — no 72-hour stress tests, no multi-TB database tests.
- **Formal verification** — no proof that the B+ tree implementation is correct. We trust testing + review.
- **Side-channel resistance** — no power analysis, no cache-timing tests. Out of scope per §8.1.
- **Platform-specific behavior** — Windows and Linux CI, but no BSD, no 32-bit, no ARM-specific tests.
- **Adversarial cryptanalysis** — RustCrypto primitives are trusted. Our *composition* is tested, not broken.

This is acceptable for a learning project. Document it so we don't quietly assume otherwise.

---

## 11. Roadmap (stages)

Each stage ends with a tagged release and a short write-up.

### Stage 1 — Storage only *(finishable in a weekend)*
- File header, page allocation, freelist.
- Slotted page leaf layout.
- Single implicit "table."
- CLI: `init`, `put <k> <v>`, `get <k>`, `scan`, `stat`, plus the debug trio in §11.1.
- **No encryption, no WAL, no B+ tree yet.** Linear scan across leaf pages.
- Property tests for page + record codec.
- **Reference:** See `REFERENCES.md` for LruCache (page cache eviction pattern) and RingBuffer (optional WAL buffering).

#### 11.1 Stage 1 simplifications (explicit)

To keep Stage 1 actually finishable, the following are **deliberately not built** and must not be smuggled in:

- `fragmented_bytes` is not tracked — recompute on demand if a compaction is ever triggered.
- No overflow pages. Records exceeding the §5.4.2 cap are rejected.
- No readers-plural — `open` takes an exclusive lock on the file.
- No varint debate: **LEB128**, unsigned, for both `key_len` and `value_len`. Decision closed.
- No background anything. All work happens on the calling thread.

#### 11.2 Stage 1 debug tooling (ships with Stage 1, not later)

Debugging a storage engine without visibility is a recipe for learned helplessness. These CLI subcommands are part of Stage 1's definition of done:

- `tosumu dump <path> [--page N]` — pretty-print the file header and/or a single page: type, slot count, free_start/free_end, and each slot's `{offset, length, key_preview}`.
- `tosumu hex <path> --page N` — raw hex+ASCII dump of one page, 16 bytes per line, with header-field annotations for page 0.
- `tosumu verify <path>` — walk every page, check page-type consistency, slot bounds (`offset + length <= page_body_size`), freelist reachability, and (Stage 4+) AEAD tag + header MAC. Report every anomaly, exit non-zero on any.

### Stage 2 — B+ tree index
- Internal pages, splits, merges (lazy deletes ok).
- Overflow pages for large values.
- Replace linear scan with tree walk.
- Property tests for tree invariants.
- **Reference:** See `REFERENCES.md` for BPlusTree (node splitting, leaf links, rebalancing algorithms).

### Stage 3 — Transactions + WAL
- `begin/commit/rollback`.
- WAL with physical logging.
- Recovery on open.
- `CrashFs` harness and crash tests.

### Stage 4 — Encryption

Split into three sub-stages because key management is its own discipline and cramming it into one stage is how toy projects quietly die.

#### Stage 4a — envelope encryption (one protector)
- Enable the page frame in §5.3 unconditionally for new encrypted databases.
- Generate a random DEK at `init`.
- Derive `page_key` and `header_mac_key` from DEK via HKDF (§8.3).
- One protector: **passphrase** (Argon2id). One keyslot.
- Keyslot region layout (§8.7), header MAC covers it.
- KATs for AEAD, HKDF info strings, and DEK-wrap AAD.
- CLI: `tosumu init --encrypt`, `tosumu open` prompts for passphrase.
- **`tosumu verify` extended** to check every keyslot's `kek_kcv` once unlocked, and the `dek_kat` field.

#### Stage 4b — multiple protectors + recovery + rotation
- Multiple keyslots; any one can unlock.
- **RecoveryKey** protector with one-time Base32 display at init.
- Optional **Keyfile** protector.
- CLI: `tosumu protector add|remove|list`, `tosumu rekey-kek` (cheap; rewraps DEK).
- `tosumu rekey-dek` (expensive full-file DEK rotation) — stretch; slips to Stage 6 if time is short.
- Tests: wrong-protector rejection, protector-swap attack (attacker swaps two wrapped blobs → must fail due to AAD binding, §8.7), header-MAC-covers-slot-region attack.

#### Stage 4c — TPM-backed protector (optional, feature-flagged)
- `tpm` Cargo feature. Core crate builds and passes all tests without it.
- `TpmProtector` seals a KEK to a TPM policy.
- `TpmPlusPinProtector` combines TPM-sealed secret + user PIN through Argon2id.
- Platform: Windows TBS or Linux `/dev/tpmrm0` via `tss-esapi` crate; pick one, document, move on.
- **Non-goal:** remote attestation, network key escrow, OS credential vault integration.

### Stage 5 — Toy query layer
- Parser for `CREATE TABLE`, `INSERT`, `SELECT ... WHERE key = ?`.
- Multiple tables → each table is a (rootpage, name) entry in a system catalog page.
- Still single-column primary key, no joins.

### Stage 6 — Stretch
- Multi-reader concurrency (MVCC snapshot by LSN).
- **Secondary indexes** — additional B+ trees mapping `(secondary_key, primary_key)`. Think `CREATE INDEX idx ON users(email)` for relational-style lookups. Not full-text, not fuzzy, not vectors (see §17).
- `VACUUM` — reclaim space from deleted records and rebuild indexes.
- Benchmarks vs SQLite on toy workloads, purely for humility.
- Explicit non-goals for Stage 6: no FSTs, no full-text search, no vector search, no spatial indexes. See §17 for why.
- **Optional optimization:** See `REFERENCES.md` for BloomFilter (per-page negative lookups to skip pages during scans).

---

## 12. Format evolution and migration policy

Humans are terrible migration engines. The file format will change; the engine’s job is to detect that, do the safe thing automatically, and refuse loudly when the safe thing is not possible. This section is normative: every format change must declare which category it belongs to and which rules apply.

### 12.1 Version fields

Two distinct `u16`s live in the header:

- **`format_version`** — what the file *is*. Bumped by every on-disk format change.
- **`min_reader_version`** — the lowest engine `format_version` that is permitted to open this file. A conservative writer sets this equal to `format_version`; a writer that knows a change is backwards-compatible may set it lower.

The engine itself has a `SUPPORTED_FORMAT` constant. Open rules:

| File's `format_version` | File's `min_reader_version` | Engine behavior |
|---|---|---|
| `== SUPPORTED_FORMAT` | any ≤ `SUPPORTED_FORMAT` | Open normally. |
| `< SUPPORTED_FORMAT` | any | Eligible for migration (§12.3). |
| `> SUPPORTED_FORMAT` | `≤ SUPPORTED_FORMAT` | Open **read-only**, print warning. |
| `> SUPPORTED_FORMAT` | `> SUPPORTED_FORMAT` | Refuse with `NewerFormat`. |

This lets us ship forward-compatible additions (e.g. a new optional header field) without immediately invalidating older binaries.

### 12.2 Migration categories

Every migration declares exactly one category. The category determines whether it runs automatically, whether a full rewrite is required, and how crash safety is guaranteed.

| Category | Examples | Auto on open? | Rewrite cost |
|---|---|---|---|
| **Metadata-only** | New optional header field with default; reserved flag becomes meaningful. | Yes | O(1) |
| **Keyslot-metadata** | New protector kind; per-slot field extension within reserved space. | Yes | O(keyslots) |
| **Page-local rewrite** | Slotted-page header layout tweak; freelist encoding change. | **No** (explicit) | O(pages) |
| **Index rebuild** | B+ tree node format change; new order or comparator. | **No** (explicit) | O(records), drops+rebuilds tree |
| **Full logical export/import** | Any change the other categories can’t express. | **No** (explicit) | O(records), new file |
| **Crypto-structural** | AAD composition change; DEK-wrap scheme change. | **No** (explicit) | Varies; often full rewrite |

Rule of thumb: **if it touches every page, it is not automatic**.

### 12.3 Policy

- **Safe automatic migrations happen on open.** Metadata-only and keyslot-metadata categories upgrade transparently, inside a transaction, and update `format_version` + `min_reader_version` before returning.
- **Destructive or long-running migrations require an explicit call.** Page-local, index rebuild, logical export/import, and crypto-structural migrations are performed only by `Database::migrate(path, opts)` or `tosumu migrate`.
- `open_read_only` **never** migrates.
- Every migration is **idempotent**: detects whether it has already run (via `format_version`) and is safe to re-invoke.
- Every migration ships with its own test: starting from a checked-in fixture file of the pre-migration format, open/migrate/verify must produce the expected post-migration fixture.

### 12.4 Crash-safety model

Two implementation strategies are permitted. Each migration declares which it uses.

**A. Copy-and-swap (default for heavy migrations).**
1. Write new file next to the original: `app.db.migrating`.
2. fsync the new file and its directory.
3. Rename `app.db` → `app.db.pre-v{N}.bak` (or delete if `--no-backup`).
4. Rename `app.db.migrating` → `app.db`.
5. fsync the directory.

On crash at any step, the original file is intact and an orphan `.migrating` file is cleaned up at next open.

**B. In-place via WAL (only for metadata-only / keyslot-metadata).**
1. Begin transaction.
2. Patch header and/or keyslot region.
3. Commit (WAL fsync first).

In-place is only permitted for migrations whose entire delta fits in a single transaction and touches no data pages.

### 12.5 Backups

- Automatic migrations **always** write a `.pre-v{N}.bak` next to the file before the first page changes, unless `--no-backup` is passed.
- `tosumu backup <path>` is a first-class command and is implicitly invoked before any explicit migration.
- The engine refuses to delete a `.bak` file. That’s the user’s call.

### 12.6 Migration trait and registry

Migrations are explicit structs implementing a common trait. No if-branch soup in `open()`.

```rust
pub trait FormatMigration: Send + Sync {
    const FROM: u16;
    const TO: u16;
    const CATEGORY: MigrationCategory;

    fn validate_preconditions(&self, db: &Database) -> Result<()>;
    fn migrate(&self, ctx: &mut MigrationCtx) -> Result<()>;
    fn validate_postconditions(&self, db: &Database) -> Result<()>;
}

inventory::collect!(&'static dyn FormatMigration);
```

The engine builds a migration **chain** at open time: it walks registered migrations and verifies there is exactly one path from `file.format_version` to `SUPPORTED_FORMAT`. Ambiguous or missing links fail fast with a descriptive error.

### 12.7 Library API

```rust
impl Database {
    /// Auto-applies migrations in categories allowed by `opts.auto_migrate_policy`
    /// (default: metadata-only + keyslot-metadata). Heavier categories return
    /// `MigrationRequired`.
    pub fn open(path: &Path, opts: OpenOptions) -> Result<Database>;

    /// Never migrates. Returns `MigrationRequired` if the file is older.
    pub fn open_read_only(path: &Path) -> Result<Database>;

    /// Explicit migration runner. Applies every queued migration up to
    /// `SUPPORTED_FORMAT`, with backup and post-validation. No-op if already current.
    pub fn migrate(path: &Path, opts: MigrateOptions) -> Result<MigrationReport>;

    /// Dry-run: report what migrating this file would do, without touching it.
    pub fn inspect(path: &Path) -> Result<MigrationPlan>;
}
```

`MigrationPlan` includes: current `format_version`, target `format_version`, ordered list of migration steps with category and estimated rewrite cost, whether a backup will be created, and whether unlock (passphrase / TPM) will be required.

### 12.8 Key-management migrations

Key-management changes are **keyslot-metadata** migrations almost by construction, because the DEK/KEK split (§8) was designed so rotation rewrites the header, not the pages. Covered operations, all automatic-eligible:

- Rotate a KEK (rewrap DEK under a new protector-derived KEK).
- Add/remove a protector slot.
- Extend per-slot reserved bytes when a new protector version lands.

Exceptions that are **not** automatic:

- Full DEK rotation (§8.8) — crypto-structural, rewrites every page.
- AAD composition change — crypto-structural.

### 12.9 Schema migrations (Stage 5+)

Format migrations (§12.1–8) change how bytes are laid out. Schema migrations change what the bytes *mean*. They are a separate, higher-layer concern and live in the query crate.

Sketch:

```rust
db.migrate_schema([
    schema::create_table("users", &[...]),
    schema::add_column("users", "email", Type::Text),
    schema::backfill("users", |row| { /* user logic */ }),
])?;
```

Rules inherited from §12.3:
- Purely additive steps (new table, new nullable column) are automatic-eligible.
- Data-transforming steps require an explicit callback and explicit invocation.
- Destructive steps (drop column/table) refuse to run under `open()` — `migrate_schema` only.

A system catalog page tracks applied schema migration ids (monotonic integers). Re-running is a no-op.

### 12.10 CLI surface

Added in Stage 1 (even before any migrations exist), so the commands are muscle memory by the time they matter:

```
tosumu migrate <path>              # apply all pending migrations, with backup
tosumu migrate --dry-run <path>    # print MigrationPlan, touch nothing
tosumu migrate --no-backup <path>  # skip the .bak; refuses on destructive categories
tosumu inspect <path>              # format_version, min_reader_version, protectors
tosumu backup <path>                # explicit snapshot via copy-and-fsync
tosumu verify <path>                # already defined §11.2; also checks version fields
```

### 12.11 What this section does *not* promise

- No automatic **downgrade**. Ever. Downgrading is "use the backup."
- No partial migration on open. Either the whole auto-eligible chain applies, or none of it does.
- No silent destructive behavior. Any migration that touches more than metadata requires explicit opt-in.

---

## 13. Repository layout

```
Database/
├── Cargo.toml                 (workspace)
├── DESIGN.md                  (this file)
├── README.md
├── SECURITY.md
├── LICENSE-MIT
├── LICENSE-APACHE
├── .gitignore
├── .github/
│   └── workflows/
│       └── ci.yml
├── crates/
│   ├── tosumu-core/           (library: pager, btree, wal, crypto)
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── error.rs
│   │       ├── page.rs
│   │       ├── record.rs
│   │       ├── pager.rs
│   │       ├── wal.rs
│   │       ├── btree.rs
│   │       └── crypto.rs
│   └── tosumu-cli/            (binary, produces `tosumu` executable)
│       ├── Cargo.toml
│       └── src/main.rs
├── fuzz/                      (cargo-fuzz targets, added in Stage 1.5)
└── tests/                     (integration tests, CrashFs harness)
```

Workspace so Stage 5's query crate can slot in cleanly without bloating the core crate.

---

## 14. Open questions

These are tracked here, not silently deferred.

1. **Page size.** 4 KB is the obvious default. Do we want to make it configurable at `init` time for experimentation (e.g. 8 KB, 16 KB)? *Tentative: yes, settable at init, immutable after.*
2. **Endianness on disk.** Little-endian hardcoded. Any reason to revisit? *Tentative: no.*
3. ~~**Varint flavor.**~~ **Closed.** LEB128, unsigned. See §11.1.
4. **Checksum vs MAC for unencrypted mode.** If a user opts out of encryption, do we still CRC pages? *Tentative: yes, CRC32C in the page header.*
5. **WAL in separate file vs embedded.** Starting with a separate `tosumu.wal` file. Embedded WAL (SQLite-style) is possible later but adds complexity.
6. **Free page zeroing.** Do we zero freed pages on disk? *Tentative: yes when encrypted (cheap), optional when not.*
7. **Pager API shape.** References-with-lifetimes vs. closure/handle-based. Default is references; escape hatch documented in §6.2. Decision deferred to Stage 2.
8. **Global LSN in AEAD AAD.** Would close the consistent-multi-page-rollback gap in §5.3. Cost: every write bumps a global counter that must be durable before the write lands. Deferred to Stage 6.
9. **Keyslot count default.** 8 slots = 1 page at 256 B/slot + header overhead, which is plenty. Bigger means wasted space; smaller means rotation is annoying. *Tentative: 8 slots, fixed at init.*
10. **TPM library choice.** `tss-esapi` (cross-platform but Linux-centric) vs. platform-native (`windows` crate TBS bindings on Windows). *Tentative: `tss-esapi` for portability; revisit in Stage 4c.*
11. **`dek_id` in page AAD.** Including it would enable safe incremental rekey but breaks every existing page on DEK rotation. §8.8 currently says no; revisit if online rekey becomes a goal.
12. **Default `auto_migrate_policy`.** Ship with auto = {metadata-only, keyslot-metadata}. Should page-local rewrite ever be auto under a size threshold (e.g. <1 MB file)? *Tentative: no. Explicit is safer and consistent.*
13. **Backup retention.** Do we cap the number of `.pre-v{N}.bak` files we leave behind? *Tentative: no. Engine never deletes backups; that’s the user's call per §12.5.*

---

## 15. Definition of done (per stage)

A stage is "done" when:

1. **All acceptance tests for that stage pass** (§10.10). This includes:
   - All unit tests (`cargo test --workspace`).
   - Stage-specific integration tests in `tests/`.
   - CLI manual smoke tests listed in the stage's acceptance criteria.
   - Property tests where applicable (page encode/decode for Stage 1, B+ tree invariants for Stage 2).
   - Fuzz targets run for 5 minutes each without panics (manual, pre-release).
   - CrashFs tests (Stage 3+).
   - KATs (Stage 4+).
2. **The on-disk format section** of this doc (§5) has been updated *before* code was merged for any format change.
3. **Any format change** is accompanied by a registered `FormatMigration` (§12.6) and a fixture-based migration test (§10.9).
4. **Test coverage** in `tosumu-core` is ≥80% (§10.12). Run `cargo tarpaulin` or `cargo llvm-cov` and review uncovered lines.
5. **`cargo fmt`, `cargo clippy -- -D warnings`, and `cargo test`** are all clean on stable Rust.
6. **A short retrospective** is appended to a `STAGES.md` (future) describing what surprised us and what we'd do differently.
7. **Version tag** created: `git tag v0.{stage}.0 && git push --tags`.

---

## 16. Name

**`tosumu`** — a conlang word meaning *knowledge-organization device*.

Etymology (registered form `to-su-mu`):

- `to` — knowledge / information
- `su` — organized structure
- `mu` — object / device

Composed: *"knowledge-organization device"* → database, library, knowledge-store.

Written form: **`tosumu`** (lowercase, no hyphens, ASCII-only).

Conventions:

- Crate names: `tosumu-core`, `tosumu-cli`, future `tosumu-query`.
- Binary: `tosumu` (e.g. `tosumu init`, `tosumu migrate`).
- File magic: ASCII `TOSUMUv0` in the first 8 bytes of page 0.
- HKDF info strings: `"tosumu/v{N}/<role>"` — see §8.3.
- AAD prefixes for DEK wrapping: `"tosumu/v{N}/wrap"` — see §8.7.
- Default file extension: `.tsm` (short) or `.tosumu` (explicit). `.tsm` for CLI examples.
- Pronunciation: *TOH-soo-moo*.

---

## 17. Advanced indexing and future directions

This section explicitly addresses indexing features beyond a basic B+ tree, so the project scope is honest and the "finishable by a mortal" goal stays intact.

### 17.1 What tosumu *does* support (Stages 1–6)

- **Primary key index** (Stage 2): B+ tree over the primary key. Supports point lookups (`get(key)`) and range scans (`scan(start_key..end_key)`).
- **Secondary indexes** (Stage 6, stretch): Additional B+ trees mapping `(secondary_key, primary_key)`. Standard relational DB feature. Supports lookups like `SELECT * FROM users WHERE email = ?`.

That's it. That's the entire indexing story for tosumu as designed.

### 17.2 What tosumu explicitly does *not* support

The following advanced indexing features are **out of scope** for Stages 1–6 and are unlikely to be added to the core engine:

#### 17.2.1 Finite State Transducers (FSTs)

**What:** Compressed trie structure mapping strings → values, used by Tantivy (Lucene's Rust cousin) for dictionary lookups and prefix search. Orders of magnitude more space-efficient than a B+ tree for string keys with common prefixes.

**Why not in tosumu:**
- FSTs are read-only or append-mostly. tosumu's design assumes mutable records.
- FST construction is a separate build phase (sort keys, build trie, serialize). tosumu's B+ tree is incrementally mutable.
- The complexity budget for Stage 2 is "implement a B+ tree"; FSTs are a lateral move into different territory.

**If you wanted it:**
Build a separate `tosumu-fst` crate wrapping the `fst` crate from BurntSushi. Store the FST as a blob in a tosumu record, rebuild it periodically. Treat tosumu as dumb storage and the FST as an external index.

#### 17.2.2 Full-text search (Lucene-style inverted indexes)

**What:** Tokenize documents, build inverted index mapping `term → [doc_id, doc_id, ...]`, support boolean queries (`"rust" AND "database"`), ranking (TF-IDF, BM25), highlighting.

**Why not in tosumu:**
- This is **an entire search engine**, not a database feature. Lucene, Elasticsearch, Tantivy, MeiliSearch are purpose-built for this.
- Building a competitive full-text engine is a multi-year project. tosumu is a learning project about page-based storage and crypto, not information retrieval.
- The right architecture is: tosumu stores documents → external indexer (Tantivy, Sonic, etc.) builds the inverted index → queries go to the indexer.

**If you wanted it:**
Use Tantivy or MeiliSearch as the index layer. Store document IDs in tosumu, forward search queries to the indexer, fetch the resulting doc IDs from tosumu. Don't try to build a search engine inside a key/value store.

#### 17.2.3 Vector / embedding search (semantic / AI-powered search)

**What:** Store high-dimensional vectors (e.g. sentence embeddings from BERT, CLIP image vectors), support approximate nearest neighbor (ANN) queries, return top-K most similar items. Used for semantic search, recommendation, RAG pipelines.

**Why not in tosumu:**
- ANN algorithms (HNSW, IVF, product quantization) are **fundamentally different** from B+ trees. They're graph-based or partition-based, not sorted-key-based.
- This is an active research area. State-of-the-art changes every 18 months. Not a fit for a "finishable" learning project.
- Storage engines that bolt on vector search (pgvector, SQLite-vss) are essentially embedding a separate vector index library (hnswlib, faiss) and exposing it through SQL syntax. That's a thin integration layer, not a core engine feature.

**If you wanted it:**
- Store vectors as blobs in tosumu records: `key → (metadata, vector_blob)`.
- Build a separate ANN index in memory or in a specialized vector DB (Qdrant, Milvus, Weaviate, Chroma).
- Query the vector DB for top-K doc IDs, then fetch the metadata from tosumu.
- Or: use the `hnswlib-rs` crate to build an in-memory HNSW index over tosumu-stored vectors on startup. Treat tosumu as durable storage for the graph, reconstruct the index in RAM.

#### 17.2.4 Fuzzy / typo-tolerant search

**What:** Match queries with up to N typos (Levenshtein distance), support prefix/suffix wildcards, phonetic matching (Soundex, Metaphone).

**Why not in tosumu:**
- Fuzzy search requires either:
  - **Preprocessing:** Build an n-gram index or BK-tree at write time (expensive, complex).
  - **Scan-time computation:** Linear scan + edit-distance on every record (slow).
- The right place for this is in an application layer that calls tosumu for retrieval after narrowing candidates.
- MeiliSearch and Typesense specialize in typo-tolerant search. Use them, don't rebuild them.

**If you wanted it:**
- Application-layer fuzzy matching: fetch candidate keys via prefix scan from tosumu's B+ tree, compute edit distance in the application, filter results.
- Or: store phonetic hashes (Soundex codes) as secondary keys, query by phonetic hash.
- Or: use an external fuzzy-search library (like `fuzzy-matcher` or `nucleo` crates) against an in-memory key list loaded from tosumu.

### 17.3 What *could* be added as extensions (hypothetical Stage 7+)

If tosumu reaches Stage 6 and someone wants to continue the learning journey, here are reasonable next steps that don't violate the core design:

**A. Spatial indexes (R-tree / Geohash)**

Store `(lat, lon)` pairs, support bounding-box queries. This is a well-understood problem with clear algorithms (R-tree, Geohash grid) and fits the "page-based index" model. Would live as a separate index type alongside the B+ tree.

**Complexity:** Medium. A decent learning project extension.

**B. Bloom filters for negative lookups**

Per-page or per-table Bloom filters stored in the header to skip scanning pages that provably don't contain a key. Common in LSM-tree engines (LevelDB, RocksDB). Fits cleanly into tosumu's architecture.

**Complexity:** Low. Good Stage 6+ addition.

**C. Prefix compression in B+ tree nodes**

Store `["apple", "application", "apply"]` as `["apple", "+lication", "+ly"]` with shared prefix factored out. Saves space in string-heavy workloads. Standard B+ tree optimization.

**Complexity:** Medium. Fits existing B+ tree code, no new data structures.

**D. Write-optimized log-structured merge tree (LSM) mode**

Replace the in-place B+ tree with a log-structured append-only design (SSTables + compaction). Completely different write path. Would be a fork or a separate mode flag at `init` time.

**Complexity:** High. This is "build a different database."

**E. Multi-column indexes (composite keys)**

Support `CREATE INDEX idx ON users(last_name, first_name)`. Requires extending the B+ tree key encoding to handle tuples. Fits existing Stage 6 "secondary indexes" work cleanly.

**Complexity:** Low-medium. Natural Stage 6 extension.

### 17.4 What will *never* be added

Some features are fundamentally incompatible with tosumu's design or goals:

- **Distributed / replicated storage.** tosumu is single-file, single-process by design. Consensus, replication, sharding are out of scope. Use CockroachDB, TiKV, or FoundationDB if you need that.
- **OLAP / columnar storage.** tosumu is row-oriented. Columnar compression, vectorized execution, and analytics queries belong in DuckDB, ClickHouse, or DataFusion.
- **Streaming / time-series ingestion.** High-write-rate time-series workloads want a specialized engine (TimescaleDB, InfluxDB, QuestDB). tosumu's WAL and B+ tree are not optimized for append-heavy loads.
- **Graph queries (Cypher, SPARQL).** Graph traversal algorithms (BFS, shortest path, pattern matching) need adjacency-list or edge-list representations. Store graphs in Neo4j, or build a graph layer on top of tosumu as an experiment, but it won't be first-class.

### 17.5 The honest answer

If you need full-text search, use **Tantivy** or **MeiliSearch**.
If you need vector search, use **Qdrant** or **pgvector**.
If you need spatial queries, use **PostGIS** or **SpatiaLite**.
If you need graphs, use **Neo4j** or **SurrealDB**.

tosumu is a learning project about building a small, correct, encrypted, page-based key/value store with a toy SQL layer. It does that one thing well (eventually). It is not a search engine, a vector database, a graph database, or a data warehouse.

Trying to be all of those would make it none of them.

The right architecture for a real system is: **tosumu stores records, specialized engines index them**. Keep the separation of concerns clean.