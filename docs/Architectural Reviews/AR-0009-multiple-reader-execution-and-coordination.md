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

## Findings

- Visibility and checkpoint safety are storage semantics; threads and waiting
  primitives are mechanisms.
- Sequential execution and immediate `Busy` rejection must remain valid policy
  choices unless evidence proves otherwise.
- The SDD is a target design, not evidence that the current implementation
  already provides snapshot isolation or concurrent-reader guarantees.
- Any change to the on-disk format, WAL retention model, public transaction
  contract, or ownership boundary requires explicit review and possibly a new
  ADR.

## Disposition

Incubating. Use this review as the architectural owner for the MVP+10 baseline.
Do not add a general executor, async runtime, background worker, or public MVCC
contract before the visibility and locking evidence exists.

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
- [ ] Decide whether the first implementation changes only private mechanism or
      accepts a durable public or format contract requiring an ADR.

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
