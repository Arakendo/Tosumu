# AR-0009: Multiple-Reader Execution And Coordination

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-27 |
| Last reviewed | 2026-08-27 |
| Scope | Core storage / transaction coordination / platform mechanism |
| Trigger | MVP+10 requires a baseline for locking, LSN visibility, and reader/writer behavior before MVCC-style work begins |
| Related ADRs | ADR-0001, ADR-0002 |
| Related evidence | Main Feature Roadmap, SDD §§7 and 28.4, current pager/WAL/transaction implementation |

## Architectural Question

Which multiple-reader, snapshot, writer-gate, checkpoint, and execution-policy
semantics belong to Tosumu core, and which threading, waiting, and host
mechanisms remain replaceable implementation details?

## Context

The normative design describes a shared database handle, one writer, multiple
readers, committed-LSN snapshots, checkpoint blocking, and typed `Send`/`Sync`
intent. The roadmap correctly records MVP+10 as not started and requires an
executable baseline before implementation proceeds.

Concurrency risks mixing several separate questions: what a reader is allowed
to observe, how long its snapshot remains valid, how a writer is serialized,
what prevents checkpoint truncation, whether a caller waits or receives
`Busy`, and whether any mechanism uses threads, locks, queues, or asynchronous
work. A platform mechanism must not silently define storage visibility or
durability semantics.

## Evidence

- Tests or fuzzing: current storage and recovery tests establish single-process
  behavior, but no focused executable MVP+10 baseline is retained.
- Independent consumers: CLI and provider consumers use the current embedded
  surface; they do not yet prove concurrent snapshot semantics.
- Diagnostics or audits: the design names active-reader and checkpoint-blocking
  observations, but implementation parity is not established.
- Repeated implementation friction: none yet; this review precedes the first
  deliberate multiple-reader implementation slice.
- Missing evidence: actual lock ownership, current LSN visibility, cross-thread
  type behavior, writer contention, checkpoint pinning, cancellation, timeout,
  and long-lived-reader pressure.

## Ownership And Dependency Analysis

- Tosumu core owns transaction visibility, committed-LSN meaning, snapshot
  validity, writer serialization, checkpoint safety, and typed busy outcomes.
- The pager and WAL own their physical mechanics without learning SQL, session
  UI, or application scheduling semantics.
- A database/session composition layer may coordinate lifecycle and policy
  through core-owned contracts.
- Operating-system locks, condition variables, threads, async runtimes, timers,
  and host cancellation are mechanisms and must not become storage meaning.
- SQL and consumer layers may choose transaction scope but may not redefine
  which version of a record is visible or when WAL frames are reclaimable.

## Alternatives Considered

### Alternative A: Extend the current exclusive model minimally

- Benefits: smallest implementation and easiest recovery reasoning.
- Costs: may block readers unnecessarily and fail the intended MVP+10 goal.
- Failure mode: concurrency appears at callers while storage still exposes one
  global mutable lifetime.

### Alternative B: Adopt the complete SDD snapshot design immediately

- Benefits: follows the documented target directly.
- Costs: treats design prose as executable evidence and may freeze premature
  public types or on-disk assumptions.
- Failure mode: snapshot or checkpoint semantics are encoded before the current
  locking baseline is understood.

### Alternative C: Establish an executable baseline, then admit slices

- Benefits: separates observation from guarantee and lets visibility,
  coordination, and mechanism evolve independently.
- Costs: adds a measurement and review stage before feature implementation.
- Failure mode: a narrow first slice is mislabeled as full MVCC or multiple-
  reader completion.

### Alternative D: Admit one writer through a persistent advisory sidecar

- Mechanism: every normal writable pager open takes a non-blocking exclusive
  lock on a stable `<database>.writer.lock` sidecar and holds that file handle
  for the pager lifetime. Read-only handles do not take this lock.
- Benefits: cross-process cooperative writers share one fail-fast admission
  point while readers remain able to open; lock release follows file-handle
  lifetime after ordinary drop or process termination; no authenticated page or
  WAL byte changes.
- Costs: the sidecar path and participating mutation paths become an operational
  protocol; advisory locks cannot stop non-cooperating writers; Rust 1.75 needs
  a reviewed cross-platform locking dependency.
- Failure mode: a mutating path bypasses the gate, the sidecar is deleted and
  recreated while locked, or documentation overstates advisory cooperation as
  mandatory filesystem exclusion.

Locking the database file itself is not the preferred first slice: a whole-file
exclusive lock would also reject the multiple readers MVP+10 intends to retain.
A process-local registry is also insufficient because a second process could
still open an independent writer, while path aliases make identity fragile.

## Findings

- Visibility and checkpoint safety are storage semantics; threads and waiting
  primitives are mechanisms.
- Sequential execution and immediate `Busy` rejection must remain valid policy
  choices unless evidence proves otherwise.
- The SDD is a target design, not evidence that the current implementation
  already provides snapshot isolation or concurrent-reader guarantees.
- Rust's standard cross-platform `File` locking APIs require Rust 1.89, while
  Tosumu declares Rust 1.75. Raising the MSRV solely for this mechanism is not
  justified by the baseline.
- A persistent writer-only sidecar is the narrowest mechanism found that can
  reject cooperating cross-process writers without excluding readers. It does
  not establish snapshot visibility or checkpoint pinning.
- AR-0010 admitted exact native, sync-only `fs4` use for this bounded mechanism;
  ADR-0004 owns the sidecar and cooperative admission contract.

### Mutation-Path Inventory

| Path | Current mutation | Proposed gate treatment |
| --- | --- | --- |
| `KvStore` / `PageStore` / `BTree` create and writable open | Converges on a writable `Pager` | Acquire before creating/opening or recovering; retain for pager lifetime |
| Live pager page, header, allocation, and transaction writes | Mutates main file and WAL through the owned pager | Covered by the pager's retained writer guard |
| Protector add/remove/rekey | Opens and rewrites page zero through `Page0EditSession` | Acquire a short-lived writer guard before reading or editing page zero |
| Keyslot listing and all read-only opens | Reads page zero/main file and may overlay committed WAL in memory | No writer gate; future reader registration is separate |
| `wal::recover` / `wal::checkpoint` | Public functions mutate main file and/or truncate WAL | Public entry acquires the gate; pager/export callers use crate-private already-guarded variants |
| `WalWriter::{create, open, open_or_create, append, truncate}` | Public physical WAL mutation without a database path contract | Explicitly unsupported concurrently with database handles or coordinated maintenance; no false database identity is derived from a WAL path |
| Stable backup source | Copies a changing source and proves bounded stability | Remains an observing reader; do not serialize it behind the writer gate |
| Backup/export destinations and export staging | Creates previously absent or privately owned artifacts | No shared writer identity until publication; staging checkpoint uses an explicitly internal path |

Creation should acquire the stable sidecar before publishing a new main file;
the lock file may persist when unlocked and must never be deleted as ordinary
cleanup, because deleting and recreating a locked pathname can split writers
across different file identities. Backup and portable export do not copy the
writer sidecar. The proposed gate is cooperative and advisory: it coordinates
participating Tosumu mutation paths, not arbitrary file writers.
- Any change to the on-disk format, WAL retention model, public transaction
  contract, or ownership boundary requires explicit review and possibly a new
  ADR.

## Disposition

Incubating for the broader MVP+10 reader/snapshot question. ADR-0004 accepts
Alternative D for the first, narrower writer-admission slice. Do not add a
general executor, async runtime, background worker, reader snapshot claim, or
public MVCC contract through that implementation.

## Required Follow-Up

- [x] Create `docs/Plans/mvp-10-multiple-readers.md` from the plan template.
- [x] Record the current file-lock, pager-lock, transaction, and WAL ownership
      graph.
- [ ] Complete the executable baseline for simultaneous readers, writer
      contention, LSN visibility, and checkpoint interaction. Handle admission
      and visibility evidence now exists; reader-pinned checkpoint behavior
      cannot be exercised until a reader registry or equivalent contract exists.
- [x] Record `Send`/`Sync`, cancellation, timeout, shutdown, and long-lived
      reader behavior.
- [x] Decide whether the first implementation changes only private mechanism or
      accepts a durable public or format contract requiring an ADR.
- [x] Inventory every public or crate-visible path that can mutate the database
      file or WAL and state whether it participates in the writer gate.
- [x] Review the sync-only locking dependency and transitive platform closure
      under AR-0010, including Rust 1.75 and native/WASM behavior.
- [x] Define sidecar naming, persistence, backup/export treatment, advisory
      limitations, and `FILE_OPEN_BUSY` details before implementation.

## Reopening Triggers

- A consumer requires concurrent readers or cross-thread transactions.
- Reader lifetime begins pinning WAL or blocking checkpoint work.
- A proposed executor, async runtime, or background worker would affect commit
  or visibility ordering.
- Snapshot semantics require a new page or WAL representation.

## Review History

### Cycle 1 -- 2026-08-27

- Status entering review: Proposed
- New evidence: the MVP+10 roadmap gate was compared with SDD concurrency
  targets and the current lack of a retained baseline.
- Findings: ownership direction is clear enough to constrain experiments, but
  implementation semantics are not yet established.
- Disposition: Incubating
- Resulting ADR or documentation change: none

### Cycle 2 -- 2026-08-27

- Status entering review: Incubating
- New evidence: `tests/mvp10_baseline.rs` and the MVP+10 plan record that
  multiple readers and writers can open, read-only handles are live views after
  commit rather than LSN snapshots, transaction exclusion is handle-local, and
  no reader registry or checkpoint pin exists.
- Findings: current transient file-open retry is not a writer gate; current
  `Send + Sync` composition does not supply shared transaction ownership; an
  existing reader can observe a later main-file flush.
- Disposition: remain Incubating while the first writer-admission and visibility
  contract is selected.
- Resulting ADR or documentation change: none; no public or format contract has
  yet changed.

### Cycle 3 -- 2026-08-27

- Status entering review: Incubating
- New evidence: the workspace MSRV is Rust 1.75; standard `File` locks are only
  available from Rust 1.89. A sync-only `fs4` candidate declares Rust 1.75 and
  exposes whole-file shared/exclusive advisory locks on Unix and Windows.
- Findings: locking the database file would exclude readers; a process-local
  registry would not reject cross-process writers; a persistent writer-only
  sidecar is the smallest viable fail-fast mechanism, subject to mutation-path
  and dependency review.
- Disposition: remain Incubating with Alternative D preferred but not accepted.
- Resulting ADR or documentation change: AR-0010 receives the dependency
  candidate; no dependency, public API, sidecar, or format change was made.

### Cycle 4 -- 2026-08-27

- Status entering review: Incubating
- New evidence: mutation-path closure, exact sidecar lifecycle, busy details,
  native/WASM dependency behavior, and transitive platform/build boundaries
  are retained.
- Findings: normal writes and coordinated maintenance can share one database-
  identity gate; raw `WalWriter` cannot participate without inventing identity
  and is explicitly unsupported during concurrent database use.
- Disposition: remain Incubating for snapshots/checkpoint pinning; accept the
  narrower writer-admission decision through ADR-0004.
- Resulting ADR or documentation change: ADR-0004.
