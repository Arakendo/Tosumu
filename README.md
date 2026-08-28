# tosumu

> *knowledge-organization device*

`tosumu` is a small, page-based, authenticated-encrypted embedded database written in Rust. It is an **early-stage database project** — a clean-room implementation inspired by SQLite's structure, with per-page AEAD and envelope (DEK/KEK) key management designed in from day one rather than bolted on.

The name is a conlang word: `to` (knowledge) + `su` (organized structure) + `mu` (object / device) → *knowledge-organization device*.

## Status

**MVP+10 is in progress behind private APIs.** The core storage engine,
encryption/key-management stack, interactive TUI viewer, and narrow primary-key
SQL path are implemented. Format 3 now has committed generations, retained WAL,
bounded reader pins, and private generation-stable B+ tree reads. A public
shared database/session/read-transaction API is not yet supported. See the
[Main Feature Roadmap](docs/Plans/main-feature-roadmap.md) for the canonical
delivery checklist.

An opt-in `experimental-shared-readers` Cargo feature exposes a deliberately
unstable logical KV prototype for caller-shape testing. Its module and names are
not compatibility commitments and are excluded from the default feature set.

| MVP | Capability | State |
|---|---|---|
| 0 | Append-log store, CLI (put/get/scan) | ✅ done |
| +1 | Real on-disk format: 4 KB pages, slotted layout, freelist | ✅ done |
| +2 | Debug trio: `dump`, `hex`, `verify`; fuzz target for page decode | ✅ done |
| +3 | B+ tree index, overflow pages, sorted scan | ✅ done |
| +4 | Write-ahead log, transactions, crash recovery | ✅ done |
| +5 | `CrashWriter` harness, `check_invariants()`, property tests | ✅ done |
| +6 | Envelope encryption: per-page AEAD, single passphrase protector, KATs | ✅ done |
| +7 | Multiple protectors: up to 8 keyslots, recovery key, KEK rotation, `protector` CLI | ✅ done |
| +8 | Interactive TUI viewer (`tosumu view`) | ✅ done |
| +9 | Initial SQL layer and `tosumu sql` CLI | baseline done; audit/scan open |
| +10 | MVCC / multiple readers | 🚧 private mechanism in progress |

## Warning

> **This is a pre-audit, pre-stability database project. Do not use `tosumu` to protect real secrets.**
>
> The crypto design is carefully documented, but it is not audited, not reviewed, not hardened, and not production-ready. See [`SECURITY.md`](SECURITY.md).

## What it is

- **Single-file, single-process, embedded** — like SQLite in shape.
- **4 KB pages** — slotted layout, B+ tree index, overflow pages for large values.
- **Write-ahead log** — physical (full-page) logging, crash-recoverable at any write site.
- **Per-page AEAD** — ChaCha20-Poly1305; page number, version, and type bound as AAD.
- **Envelope encryption** — random DEK per database, HKDF-derived page key and MAC key. DEK wrapped by up to 8 independent **protectors** (passphrase or recovery key today; keyfile, TPM, Secure Enclave planned). Rotate a passphrase without rewriting pages.
- **Header MAC** — HMAC-SHA256 over the full keyslot region; protector-swap and cross-DB splice attacks are rejected at open time.
- **`#![forbid(unsafe_code)]`** throughout.

## What it is not

- Not SQL-complete and not a general-purpose query optimizer; MVP+9 provides a narrow primary-key SQL path.
- No public multi-process data-sharing or snapshot-reader API; cooperating
  writers are excluded across processes, and independent read-only handles
  remain live views rather than pinned snapshots.
- Not networked.
- Not a drop-in SQLite replacement.
- Not audited crypto.

## Build and run

```sh
cargo build --release
cargo test --workspace

# Unencrypted DB
cargo run -- init app.tsm
cargo run -- put app.tsm hello world
cargo run -- get app.tsm hello

# Encrypted DB
cargo run -- init --encrypt app.tsm           # prompts for passphrase
cargo run -- protector add-recovery app.tsm   # displays one-time recovery key
cargo run -- protector list app.tsm
cargo run -- rekey-kek --slot 0 app.tsm       # rotate passphrase without page rewrite
```

MSRV: Rust 1.75, edition 2021.

## Crypto stack (summary)

| Primitive | Use |
|---|---|
| ChaCha20-Poly1305 | Per-page AEAD |
| HKDF-SHA256 | DEK → page_key, header_mac_key, audit_key |
| Argon2id (m=65536, t=3, p=1) | Passphrase → KEK |
| HMAC-SHA256 | Header MAC over keyslot region |
| ChaCha20-Poly1305 | DEK wrap/unwrap |
| HKDF-SHA256 (no Argon2id) | Recovery key → KEK |

Full details: `docs/Specifications/Tosumu Software Design Document.md §8`.

## Fuzz targets

Six `cargo fuzz` targets in `fuzz/fuzz_targets/`: page decode, B+ tree ops, WAL replay, AEAD frame, keyslot parse, and B+ tree crash boundaries. Run manually before each milestone: `cargo fuzz run <target> -- -max_total_time=300`.

## Roadmap

See [`docs/Specifications/Tosumu Software Design Document.md §12`](docs/Specifications/Tosumu%20Software%20Design%20Document.md) for the full MVP and stage breakdown. MVP+8 is complete: `tosumu view` provides a cross-platform TUI (`ratatui` + `crossterm`) for inspecting file header, pages, B+ tree structure, WAL records, and per-keyslot detail on encrypted databases. MVP+9 adds the initial SQL layer and `tosumu sql` CLI path. MVP+10 is now proving MVCC storage and ownership privately before admitting its public API.

## License

Dual-licensed under either of:

- Apache License, Version 2.0 ([`LICENSE-APACHE`](LICENSE-APACHE))
- MIT license ([`LICENSE-MIT`](LICENSE-MIT))

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

## Further reading

- [`docs/Specifications/Tosumu Software Design Document.md`](docs/Specifications/Tosumu%20Software%20Design%20Document.md) — the design doc. Source of truth for all decisions.
- [`docs/Specifications/Tosumu Error Design Document.md`](docs/Specifications/Tosumu%20Error%20Design%20Document.md) — structured error design, code/status model, and boundary-mapping plan.
- [`docs/Specifications/Tosumu Inspect API Specification.md`](docs/Specifications/Tosumu%20Inspect%20API%20Specification.md) — machine-readable inspection contract for the TUI, harness, and future companion tools.
- [`SECURITY.md`](SECURITY.md) — threat model summary and responsible disclosure.
- [`docs/Specifications/Tosumu Reference Implementations.md`](docs/Specifications/Tosumu%20Reference%20Implementations.md) — reference implementations that informed the design.
