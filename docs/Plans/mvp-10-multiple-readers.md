# MVP+10 Multiple Readers And Coordination

| Field | Value |
| --- | --- |
| Status | In progress; Slice 1 complete |
| Opened | 2026-08-27 |
| Last updated | 2026-08-27 |
| Owner | Tosumu maintainers |
| Target | MVP+10 core storage and embedded provider coordination |
| Related ADRs | ADR-0001, ADR-0002, ADR-0004 |
| Related reviews | AR-0009, AR-0011 |
| Related CRs | None |
| Depends on | MVP+9 baseline, authenticated pager, WAL recovery, provider boundary |

## Status

The pre-MVP+10 executable baseline is recorded, and ADR-0004's cooperative
single-writer admission is implemented. The repository does not yet implement
a shared `Database`, sessions, committed-LSN snapshots, reader pinning, or
reader-aware checkpoint coordination. AR-0009 remains incubating, so this plan
does not yet authorize a public MVCC or format contract.

## Purpose

Establish and then deliberately evolve Tosumu's single-writer/multiple-reader
behavior without letting threads, locks, or waiting mechanisms accidentally
define storage visibility. MVP+10 must make writer exclusion, reader snapshot
meaning, and checkpoint safety explicit and inspectable.

## Trigger And Evidence

- The SDD describes a future shared database handle, committed-LSN snapshots,
  one writer, reader-pinned WAL frames, and checkpoint diagnostics.
- Current consumers hold independent `KvStore`/`PageStore` instances rather
  than sessions over one shared engine owner.
- Existing recovery tests establish single-handle durability but do not prove
  concurrent snapshot isolation.
- `tests/mvp10_baseline.rs` demonstrates current handle, visibility, and writer
  admission behavior without treating it as a guarantee.
- AR-0009 requires a focused executable baseline before implementation begins.

## Current State

### Ownership Graph

```text
independent KvStore / PageStore handle
    -> independent BTree
        -> independent Pager
            -> one File and optional WalWriter per handle

writable Pager open
    -> acquires and retains <database>.writer.lock
    -> checkpoints an existing WAL under that guard
    -> opens its own WalWriter

read-only Pager open
    -> opens the main file read-only
    -> overlays committed WAL frames in that handle's memory
    -> does not register or pin a reader
```

- `KvStore` currently satisfies `Send + Sync`, but there is no shared engine
  handle or session/transaction type that expresses cross-thread ownership.
- Multiple read-only handles can open simultaneously.
- An existing read-only handle observes main-file changes after a writer
  commits. It is a live view, not an LSN-pinned snapshot.
- A read-only handle opened during an uncommitted transaction observes the
  pre-transaction main-file state; after commit, that same handle can observe
  the newly flushed state.
- One writable handle is admitted at a time across cooperating processes by a
  persistent advisory sidecar lock. A second writable open fails immediately
  with structured `FILE_OPEN_BUSY` details.
- Transaction exclusion is local to one mutable pager instance. Transaction
  IDs and WAL next-LSN state are also initialized per handle.
- Protector edits and public recovery/checkpoint operations participate in the
  same gate. Direct raw `WalWriter` mutation remains outside the coordination
  contract as documented by ADR-0004.
- There is no committed-LSN snapshot API, reader registry, checkpoint pin,
  cancellation token, busy timeout, coordinated shutdown operation, or
  long-lived-reader diagnostic. Dropping an independent handle is the only
  current lifecycle mechanism.

These are observations of the 2026-08-27 implementation. The writer gate is an
accepted cooperative admission guarantee; the reader observations are not
snapshot-isolation, freshness, fairness, or broader concurrency guarantees.

## Goals

- Define one core-owned committed visibility model for readers.
- Enforce at most one admitted writer with typed, bounded contention behavior.
- Prevent checkpoint/truncation from invalidating active reader snapshots.
- Expose bounded diagnostics for active readers, writer contention, and
  checkpoint blocking.
- Retain synchronous and fail-fast operation as valid host policy choices.

## Non-Goals

- SQL transaction meaning, query scheduling, or consumer request policy.
- A general executor, async runtime, background worker, or unbounded queue.
- Secondary indexes, `VACUUM`, logical SQL scans, replication, or networking.
- Treating current live-view behavior as snapshot isolation.
- Changing the on-disk or WAL format without a separate accepted decision and
  migration behavior.

## Ownership And Dependency Boundary

Core owns committed visibility, snapshot validity, writer serialization,
checkpoint safety, and typed busy outcomes. Pager and WAL retain physical page,
record, recovery, and publication mechanics. Hosts choose whether and how to
wait, cancel, schedule, or move work across threads.

```text
consumer transaction scope and scheduling
    -> provider/session composition
        -> core visibility and coordination contracts
            -> pager, WAL, recovery, and physical format
```

No SQL or consumer meaning may flow into `tosumu-core`, and platform primitives
must not become the definition of commit visibility.

## Public Contract Impact

Slice 0 changed no public or format contract. Slice 1 reuses the public
`FileBusy`/`FILE_OPEN_BUSY` vocabulary and adds the persistent writer-sidecar
operational contract accepted by ADR-0004; it changes no authenticated page or
WAL bytes. Future shared-handle, session, snapshot, busy-policy, or reader-aware
checkpoint types remain provisional under AR-0009.

## Implementation Slices

### Slice 0: Executable Baseline And Boundary Confirmation

- [x] Read the normative transaction design, ADR-0001, ADR-0002, and AR-0009.
- [x] Record the current handle and ownership graph.
- [x] Add focused observations for simultaneous readers, commit visibility,
      writable-handle admission, and `Send + Sync` intent.
- [x] Record absent LSN, cancellation, timeout, shutdown, checkpoint-pinning,
      and long-lived-reader behavior.
- [x] Confirm that the baseline changes no public API or on-disk bytes.

Exit state: current behavior is reproducible and observation is separated from
the target guarantees.

### Slice 1: First Coordination Contract

- [x] Use the baseline to choose the smallest candidate ownership boundary: a
      pager-lifetime writer guard plus short-lived guarded maintenance paths.
- [x] Audit the preferred persistent writer-sidecar mechanism and its sync-only
      locking dependency under AR-0009 and AR-0010.
- [x] Specify fail-fast writer admission and typed contention without adding an
      unbounded queue.
- [x] Decide whether coordination is process-local, cross-process, or explicitly
      staged, and diagnose unsupported scope.
- [x] Update AR-0009 and create an ADR because the sidecar and busy behavior are
      a durable operational contract.
- [x] Exercise the contract through the provider and one independent caller.

Exit state: one writer can be admitted or rejected deterministically, without
claiming snapshot isolation.

### Slice 2: Committed-LSN Reader Snapshots

- [x] Trace current LSN assignment, page-zero checkpoint state, main-file
      publication, WAL truncation, direct writes, and read-only overlay behavior.
- [ ] Define a monotonic committed generation across checkpoint and reopen.
- [ ] Route explicit transactions and ordinary writes through one atomic
      publication path.
- [ ] Decide main-file/WAL version residence, page selection, crash ordering,
      and format/migration impact.
- [ ] Decide whether snapshots are scoped to one shared database owner or join
      a cross-process reader protocol.
- [ ] Define the committed-LSN source of truth and reader capture point.
- [ ] Retain versions needed by the oldest active reader.
- [ ] Prove that a reader does not observe commits newer than its snapshot.
- [ ] Bound and diagnose long-lived-reader WAL pressure.
- [ ] Stop for a format decision if retained versions require new WAL or page
      representation.

Exit state: snapshot visibility and lifetime are executable contracts.

### Slice 3: Checkpoint Coordination And Diagnostics

- [ ] Define passive and blocking checkpoint behavior around active readers.
- [ ] Report frames retained, oldest reader LSN, and the blocking owner without
      leaking host or consumer meaning.
- [ ] Add crash, cancellation, timeout, shutdown, and cross-handle contention
      evidence appropriate to the admitted mechanisms.
- [ ] Reconcile the SDD, AR-0009, public docs, and compatibility claims.

Exit state: writer, reader, and checkpoint lifecycles compose without hidden
waiting or stale-frame truncation.

## Validation Matrix

| Concern | Evidence | Command Or Artifact | Required Result |
| --- | --- | --- | --- |
| Current behavior | MVP+10 baseline integration tests | `cargo test -p tosumu-core --test mvp10_baseline` | Pass |
| Recovery | Existing pager/WAL recovery suites | `cargo test -p tosumu-core wal` | Pass |
| Provider boundary | External consumer suite | `cargo test -p tosumu-core --test provider_boundary` | Pass |
| Static quality | Workspace formatting and Clippy | Standard workspace commands | Pass |
| Documentation | Strict MkDocs build | `mkdocs build --strict` | Pass |

## Failure And Diagnostic Semantics

ADR-0004 admits `FILE_OPEN_BUSY` for non-blocking writer-gate contention. Its
structured path names the `.writer.lock` sidecar and its operation is
`acquiring database writer gate`. Unsupported snapshot exhaustion, reader-aware
checkpoint blocking, timeout, and cancellation behavior must still fail
explicitly rather than wait indefinitely or fall back to live reads.

## Compatibility And Migration

Slice 1 changes no format, WAL bytes, recovery ordering, Rust API shape, or CLI
command. It adds a persistent `.writer.lock` operational artifact and changes a
second cooperating writable open from admission to structured busy rejection.
Any retained-version representation or committed-LSN field introduced later
requires explicit compatibility and migration treatment before implementation.

## Security And Trust

Reader snapshots must remain inside the authenticated pager trust boundary.
Coordination must not expose plaintext pages, key material, raw WAL frames, or
stronger freshness claims. A committed LSN is local visibility evidence, not an
external anti-rollback or witness guarantee.

## Performance And Resource Bounds

Baseline tests establish behavior, not throughput. Future slices must bound
writer wait, reader registration, retained WAL growth, and checkpoint work.
Long-lived readers must produce observable pressure rather than unbounded,
silent retention.

## Risks And Mitigations

| Risk | Impact | Mitigation Or Evidence |
| --- | --- | --- |
| Mechanism defines semantics | Platform-specific behavior becomes contract | Specify visibility before selecting locks or runtimes |
| Two writers mutate one WAL | Lost updates or corrupt recovery ordering | First implementation slice admits or rejects one writer |
| Live read mislabeled snapshot | Repeat reads observe different commits | Baseline test names current live-view behavior explicitly |
| Reader pins grow without bound | Disk/resource exhaustion | Bounded diagnostics and explicit policy before retention |
| Premature format work | Unmigratable compatibility boundary | Stop at AR/ADR gate before changing WAL or page bytes |

## Completion Criteria

- [ ] One-writer admission and reader snapshot visibility are explicit and
      executable.
- [ ] Checkpoint safety accounts for every active reader.
- [ ] Busy, timeout, cancellation, and unsupported behavior are typed and
      bounded.
- [ ] Public, format, recovery, security, and documentation claims agree.
- [ ] Full workspace validation and applicable crash evidence pass.

## Parking Or Reopening Criteria

Park before semantic implementation if AR-0009 cannot choose a visibility or
ownership boundary without consumer evidence. Reopen for a consumer requiring
concurrent reads, a demonstrated writer race, unacceptable WAL growth, or a
format/recovery design that can retain committed versions safely.

## Progress Log

### 2026-08-27

- Recorded the current ownership and lifecycle graph.
- Added focused executable observations for handle concurrency, pre/post-commit
  visibility, writable admission, and `Send + Sync`.
- Validation passed formatting, strict workspace Clippy, all five focused
  baseline tests, and `mkdocs build --strict`; Rust reported only the known
  incremental-cache hard-link fallback warning.
- Kept all target snapshot and checkpoint semantics provisional under AR-0009.
- Reviewed mechanism constraints: standard file locking would raise the Rust
  1.75 MSRV, a database-file exclusive lock would reject readers, and a
  process-local registry would not coordinate other processes.
- Retained a persistent advisory writer-lock sidecar as the preferred first
  candidate, pending mutation-path inventory and sync-only dependency review.
- Inventoried normal pager writes, protector edits, recovery/checkpoint,
  direct public WAL mutation, and backup/export paths. Direct `WalWriter`
  mutation remains an unresolved coordination bypass.
- Retained the exact `fs4` 1.1.0 sync-only native closure and found that an
  unconditional dependency breaks `wasm32-unknown-unknown`; any admitted
  dependency must be target-specific to Unix/Windows with an explicit
  unsupported non-native writer path.
- At that checkpoint, implementation remained gated on scoping the bypass and
  completing the transitive dependency review.
- Closed admission prerequisites in ADR-0004: exact sidecar lifecycle, guarded
  database/maintenance paths, raw-WAL unsupported concurrency scope,
  `FILE_OPEN_BUSY` details, and bounded dependency admission are now explicit.
- Implemented ADR-0004 with a native-only exact `fs4` dependency, a retained
  pager-lifetime guard, and guarded protector, recovery, checkpoint, and
  portable-export staging paths.
- Updated the executable baseline so the only reader/writer semantic change is
  rejection of the second cooperating writer; existing live-reader behavior is
  unchanged.
- Exercised structured busy reporting through the public `KvStore` provider
  boundary and exercised independent maintenance participation through public
  checkpoint/protector calls.
- Validation passed focused writer-gate and provider tests, all 207 active core
  library tests, strict workspace Clippy, the full workspace all-targets suite,
  formatting, and `mkdocs build --strict`. The native-only locking dependency
  is absent from the WASM dependency graph; a direct core WASM check remains
  blocked earlier by the pre-existing unconditional `getrandom` configuration.
- Slice 1 is complete. Slice 2 remains gated on an admitted committed-LSN
  snapshot and version-retention design under AR-0009.
- Traced the Slice 2 storage prerequisites. Current LSNs reset with WAL
  truncation, page zero does not advance its checkpoint LSN, successful commits
  discard old frames, ordinary writes may bypass WAL, and read-only handles mix
  open-time metadata with live page reads. AR-0009 Cycle 5 therefore blocks
  snapshot implementation until publication, version residence, reader scope,
  and format impact are explicit.
- Reopened AR-0006 with the first concrete incompatible-format pressure. A v2
  writer would not preserve snapshot WAL history even though the checkpoint-LSN
  field already exists, so a snapshot format must exclude old writers and
  choose between a clean pre-stability break and an explicit offline logical
  rewrite; automatic migration on open remains rejected.
- Opened AR-0011 with a concrete preferred candidate: the durable `Commit`
  record LSN is the atomic generation; the main file represents the page-zero
  checkpoint horizon; newer committed page versions remain in WAL; snapshots
  select the newest owning commit no later than their captured generation; and
  one shared database owner retains the writer gate plus a process-local reader
  registry. Crash ordering, WAL epoch metadata, raw-WAL scope, finite retention
  limits, and v3 migration behavior remain required evidence before an ADR.
- Replaced duplicate recovery/read-overlay committed-ID classification with one
  private sequential transaction analyzer. Reusing a previously committed
  transaction ID no longer makes an incomplete later sequence appear committed;
  focused framing tests, all 211 active core tests, and strict workspace Clippy
  pass without changing snapshot, format, or public API behavior.
- Preferred page-zero checkpoint state over a duplicate WAL header as the sole
  monotonic-LSN epoch authority. Future database-owned WAL mutation is seeded
  from that checkpoint plus validated retained records; raw `WalWriter`
  mutation is reduced to an internal or explicit physical-fixture boundary and
  does not participate in the format-v3 database contract.
- Quantified retained-WAL pressure: one 64 MiB value requires at least 16,636
  overflow frames and 68,690,094 WAL bytes before leaf/header metadata. The
  existing per-value cap cannot bound a transaction because a closure may write
  arbitrarily many values, and rollback does not reclaim its appended WAL tail.
  Hard retention limits are therefore gated on transaction budgeting and safe
  tail reclamation; a pre-begin watermark alone is only pressure telemetry.
- Confirmed the Slice 2 crash ordering: WAL remains authoritative until data
  pages plus authenticated page zero at T are synced; afterward main is the
  base at T and persistent-sidecar truncation may be retried safely. Reopening
  an empty WAL seeds `T + 1`; surviving obsolete bytes cannot publish a new
  generation. Format-v3 crash fixtures remain required before implementation
  claims the behavior.
- Preferred a clean format-v3 pre-stability break. Snapshot-capable ordinary
  open accepts only its explicit supported interval and refuses v2 before WAL
  recovery; the current direction-specific `NewerFormat` failure must be
  generalized for older unsupported files. Any future v2 preservation path is
  an offline logical rewrite to a separate verified destination, admitted only
  when non-regenerable data supplies evidence for it.

## References

- `docs/Specifications/Tosumu Software Design Document.md` §§7.4-7.8, 28.4
- `docs/ADR/ADR-0001-storage-engine-layer-boundaries.md`
- `docs/ADR/ADR-0002-authenticated-pager-trust-boundary.md`
- `docs/ADR/ADR-0004-cooperative-single-writer-admission.md`
- `docs/Architectural Reviews/AR-0009-multiple-reader-execution-and-coordination.md`
- `docs/Architectural Reviews/AR-0011-committed-generation-and-version-residence.md`
- `docs/Plans/main-feature-roadmap.md`
