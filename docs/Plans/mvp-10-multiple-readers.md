# MVP+10 Multiple Readers And Coordination

| Field | Value |
| --- | --- |
| Status | Proposed; baseline recorded |
| Opened | 2026-08-27 |
| Last updated | 2026-08-27 |
| Owner | Tosumu maintainers |
| Target | MVP+10 core storage and embedded provider coordination |
| Related ADRs | ADR-0001, ADR-0002 |
| Related reviews | AR-0009 |
| Related CRs | None |
| Depends on | MVP+9 baseline, authenticated pager, WAL recovery, provider boundary |

## Status

The pre-MVP+10 executable baseline is recorded. The repository does not yet
implement a shared `Database`, sessions, committed-LSN snapshots, a writer
gate, reader pinning, or checkpoint coordination. AR-0009 remains incubating,
so this plan admits evidence work but does not yet authorize a public MVCC or
format contract.

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
    -> checkpoints an existing WAL
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
- Multiple writable handles can open simultaneously. There is no process-local
  or cross-process writer gate, and this observation is not evidence that
  concurrent writes are safe.
- Transaction exclusion is local to one mutable pager instance. Transaction
  IDs and WAL next-LSN state are also initialized per handle.
- WAL open/recovery has bounded retry for transient operating-system file
  conflicts, but that retry is not database writer serialization.
- There is no committed-LSN snapshot API, reader registry, checkpoint pin,
  cancellation token, busy timeout, coordinated shutdown operation, or
  long-lived-reader diagnostic. Dropping an independent handle is the only
  current lifecycle mechanism.

These are observations of the 2026-08-27 implementation. They are not
durability, isolation, fairness, or concurrency guarantees.

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

No public or format contract changes in Slice 0. Future shared-handle, session,
snapshot, busy-policy, or checkpoint types remain provisional. Before Slice 1
stabilizes any of them, AR-0009 must record the chosen semantics and determine
whether an ADR is required.

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

- [ ] Use the baseline to choose the smallest shared ownership boundary.
- [ ] Specify fail-fast writer admission and typed contention without adding an
      unbounded queue.
- [ ] Decide whether coordination is process-local, cross-process, or explicitly
      staged, and diagnose unsupported scope.
- [ ] Update AR-0009 and create an ADR if the boundary becomes durable.
- [ ] Exercise the contract through the provider and one independent caller.

Exit state: one writer can be admitted or rejected deterministically, without
claiming snapshot isolation.

### Slice 2: Committed-LSN Reader Snapshots

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

`FILE_OPEN_BUSY` currently represents bounded operating-system file-open
contention. It must not silently become writer-gate contention without reviewing
whether callers need a distinct stable code or structured detail. Unsupported
cross-process coordination, snapshot exhaustion, checkpoint blocking, timeout,
and cancellation must fail explicitly rather than wait indefinitely or fall
back to live reads.

## Compatibility And Migration

Slice 0 changes no format, WAL bytes, recovery behavior, Rust API, CLI, or
consumer contract. Any retained-version representation, committed-LSN field, or
cross-process lock protocol introduced later requires explicit compatibility
and migration treatment before implementation.

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
- Next slice: validate the baseline, then use AR-0009 to choose the smallest
  writer-admission contract.

## References

- `docs/Specifications/Tosumu Software Design Document.md` §§7.4-7.8, 28.4
- `docs/ADR/ADR-0001-storage-engine-layer-boundaries.md`
- `docs/ADR/ADR-0002-authenticated-pager-trust-boundary.md`
- `docs/Architectural Reviews/AR-0009-multiple-reader-execution-and-coordination.md`
- `docs/Plans/main-feature-roadmap.md`
