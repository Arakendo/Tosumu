# TOKIMU-001 Provider Baseline

Status: Slice 0 baseline and design decisions
Source CR: [`tokimu-001-tasset-storage-provider-boundary.md`](tokimu-001-tasset-storage-provider-boundary.md)
Plan: [`tokimu-001-implementation-plan.md`](tokimu-001-implementation-plan.md)
Updated: 2026-08-03

## Candidate Provider

`tosumu_core::page_store::PageStore` is the current candidate implementation
owner. It provides create/open, read-only open, get, put, delete, scan,
range-scan, statistics, and closure-based transactions. The admitted Tokimu
surface will be a small core-owned KV facade introduced in Slice 1. The facade
will expose only consumer operations and structured core errors; it will not
admit protector-management methods or physical storage types that are also
reachable from `PageStore`.

The consumer-visible types currently reachable from the candidate operations
are `PageStore`, `StoreStat`, `TosumuError`, `ErrorReport`, `ErrorStatus`,
`ErrorDetail`, and `ErrorValue`. `BTree`, `Pager`, WAL records, page frames,
crypto frames, SQL types, and CLI types are not part of the admitted contract.

## Lifecycle and Concurrency Baseline

- `create` creates a `.tsm` file and its `.wal` sidecar; `open` and
  `open_readonly` reopen existing files.
- There is no explicit close method and no `Drop` implementation. Normal Rust
  drop closes the owned file handles; a future explicit close/flush operation
  is not required for the current commit path because committed writes flush
  before the transaction returns.
- Writes require a mutable handle. A read-only handle rejects mutation through
  the structured `InvalidArgument`/permission path enforced by the pager.
- Transactions are single-level and single-writer per handle. Commit fsyncs
  the WAL before flushing dirty pages; a flush failure returns
  `CommittedButFlushFailed` and requires the caller to close and reopen for
  WAL recovery.
- File operations retry transient sharing/lock failures and eventually return
  `TosumuError::FileBusy { path, operation }`. There is no explicit advisory
  lock or process-wide writer registry. The current contract is
  single-process, single-writer; multi-process writes and MVCC are deferred.
  Consumers must treat busy as retryable rather than assuming writer
  coordination.
- `KvStore` is `Send + Sync` through its current Rust fields, but mutation still
  requires exclusive `&mut` access. `Sync` does not make concurrent writes or
  multiple handles safe at the logical database contract level. The external
  provider test asserts the auto-traits while keeping all writes serialized.

## Current Limits and Errors

- Keys must be non-empty and are limited to `u16::MAX` bytes. Larger keys
  return `TosumuError::InvalidArgument` with the stable report code for
  invalid input.
- Values may be empty. Inline values use a `u16` length; larger values use the
  version-2 overflow path up to the enforced 64 MiB maximum. Larger values
  return `TosumuError::ValueTooLarge` with `actual` and `maximum` details in
  the structured error report.
- The live inline record header stores key and value lengths as `u16`; overflow
  leaf records store a checked `u64` logical length and first-page reference.
- The physical format is currently version `2`; newer versions are rejected
  with `TosumuError::NewerFormat`. No automatic migration is performed by
  open paths.

## Slice 2 Format Decision

The following alternatives were considered:

| Design | Decision | Reason |
| --- | --- | --- |
| Widened logical value length plus a Tosumu-owned overflow chain | **Choose** | Preserves one KV operation and one logical value while keeping chunk reconstruction, corruption checks, reclamation, and allocation limits inside core. |
| Hidden chunk manifest and chunk records | Reject | Adds an internal key namespace and manifest lifecycle that complicates scans, delete/overwrite reclamation, and atomicity without improving the consumer contract. |
| Defer large values behind a streaming API | Reject for Slice 2 | Does not satisfy the required 1 MiB, 16 MiB, and 64 MiB whole-value acceptance evidence; streaming remains explicitly deferred unless measurements require it. |

Slice 2 will use a checked `u64` logical value length in overflow metadata and
an enforced maximum of `64 * 1024 * 1024` bytes. That maximum is deliberately
the first accepted target, so the 64 MiB fixture is the largest legal logical
value until measurements justify a new format/configuration decision. Inputs
above the maximum must fail before allocation, page traversal, or mutation.

Each overflow page currently carries a next-page pointer followed by one
contiguous payload segment. The leaf record carries the checked logical length
and first-page reference; core owns reconstruction and reclamation. Verification
rejects missing, cyclic, wrong-page-type, extra-segment, and oversized chains,
including a final reconstructed length that differs from the declared logical
length. Segment counts and all offset/length arithmetic are checked before
allocation or traversal. Duplicate, out-of-order, and overlapping segments are
not representable in this linear chain layout and remain future inspection
cases if the format gains explicit segment metadata.

The implementation owner is `tosumu-core::format` for constants and encoding,
`tosumu-core::btree` for chain allocation/reclamation and logical KV
operations, and `tosumu-core::inspect` for structured overflow findings. The
provider facade remains unchanged.

Changing the record wire format and adding reachable overflow records will
increment `format_version` to `2`. Version 1 files remain readable by the
version-1 path; version 2 files require the new engine, and an older engine
must return `NewerFormat` without attempting a partial open. No automatic
migration is added. A future migration or export tool must be explicit and
separately validated.

## Slice 2 Evidence

- `cargo test -p tosumu-core --test provider_boundary external_consumer_round_trips_one_megabyte_value_after_reopen`: passed.
- `cargo test -p tosumu-core --test provider_boundary external_consumer_round_trips_sixteen_megabyte_value_after_reopen`: passed in 34.36 seconds.
- `cargo test -p tosumu-core --test provider_boundary external_consumer_round_trips_maximum_value_after_reopen`: passed in 138.36 seconds.
- `cargo test -p tosumu-core tokimu_large_value_recovery_evidence_matrix -- --ignored --nocapture`: passed for 1 MiB, 16 MiB, and 64 MiB overflow payloads after committed flush failure and reopen recovery.
- `cargo test -p tosumu-core --test provider_boundary large_value_write_measurement_one_megabyte -- --ignored --nocapture`: passed; one overwrite took 3,287.9 ms with a logical copy volume of 1,048,576 bytes.
- `cargo test -p tosumu-core --test provider_boundary large_value_write_measurement_sixteen_megabytes -- --ignored --nocapture`: passed; one overwrite took 49,666.2 ms with a logical copy volume of 16,777,216 bytes.
- Provider split, compaction, overwrite/delete, scan, and page-count reuse regressions pass.
- `cargo test -p tosumu-core overflow_chain_corruption_is_structured`: passed for missing, cyclic, and extra-segment chains.
- The inline-boundary and over-limit tests pass; over-limit input leaves the
  existing value and page count unchanged. Large get/scan checks compare
  SHA-256 digests as well as complete byte vectors.

The 1/16/64 MiB tests compare complete reconstructed byte vectors after close
and reopen. The measurement helpers independently cover 1 MiB, 16 MiB, and the
64 MiB maximum value, but only the 1 MiB and 16 MiB probes have completed so
far. The observed timings and logical copy volumes are repeatable baseline
evidence; allocator/peak-RSS counts and the streaming decision remain open.

## Existing Recovery, Backup, and Verification Evidence

- WAL records contain begin, page-write, commit, and checkpoint records. Only
  committed transactions are replayed during recovery; torn WAL tails are
  rejected or trimmed according to the WAL reader/writer path.
- `PageStore::transaction` already proves closure rollback and committed-WAL
  recovery in core tests, including dirty-page flush failure and torn writes.
- Stable backup is currently CLI-owned in `tosumu-cli`; it is not yet a core
  consumer contract and is therefore Slice 4 work.
- Core inspection already exposes verification reports and per-page findings,
  but the stable embedded inspection boundary and overflow findings remain
  Slice 6 work.
- `page_store::tests::large_overflow_transaction_recovers_after_commit_flush_failure`
  passes: a committed manifest plus 1 MiB overflow payload is restored after
  the dirty-page flush fails and WAL recovery runs on reopen; the recovered
  payload hash is compared with its pre-commit hash.
- The consumer-shaped `page_store` recovery cases
  `consumer_asset_create_recovers_as_one_committed_generation`,
  `consumer_asset_overwrite_recovers_as_new_generation_without_mixing`, and
  `consumer_asset_delete_recovers_as_one_empty_generation` pass. Each uses a
  deterministic four-record asset with a 1 MiB overflow payload, injects an
  after-commit flush failure, reopens, and checks the complete logical
  generation plus B+ tree invariants.
- `consumer_asset_before_commit_discards_staged_generation` also passes: a
  complete staged asset written to WAL without a commit record is ignored on
  reopen, leaving the prior generation intact.
- `tosumu_core::inspect::inspect_recovery` now returns structured transaction
  observations with committed-replay or uncommitted-discard dispositions and
  page-write counts. WAL parse failures and busy/open failures remain typed
  `TosumuError` results rather than display-text classifications.

## Slice 0 Exit Criteria

Slice 0 is complete when this baseline is kept alongside the CR, the plan
links it, and the format/version decisions above are accepted before API or
wire-format edits begin. The next implementation slice is the external-crate
provider boundary and its public-surface integration test.