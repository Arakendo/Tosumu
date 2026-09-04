# AR-0019: Foreign Atomic Mutation Batch Boundary

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-09-03 |
| Last reviewed | 2026-09-03 |
| Scope | C ABI adapter / provider-neutral atomic writes / foreign ownership |
| Trigger | MVP+11 Slice 3 requires atomic multi-mutation behavior without carrying a borrowed Rust transaction across calls |
| Related ADRs | ADR-0001, ADR-0003, ADR-0005, ADR-0006, ADR-0007 |
| Related reviews | AR-0011, AR-0012, AR-0017 |
| Related evidence | `SharedKvStore::write`, staged-view core tests, experimental handle registry, independent C harness |

## Architectural Question

Should the first foreign atomic-write experiment own a bounded copied command
batch in the adapter, or should Tosumu first replace its callback-scoped core
writer with an owned transaction that survives across calls?

## Context

ADR-0006 deliberately exposes `KvWriteTransaction<'_>` only inside
`SharedKvStore::write`. The borrow is `!Send + !Sync`, cannot escape the
callback, and holds the shared owner while its staged B+ tree view exists. This
shape makes commit-on-`Ok`, rollback-on-`Err`, re-entry rejection, and panic
poisoning explicit in Rust.

The experimental C adapter has no callbacks. Its database, snapshot, result,
and error handles own values, but none can truthfully represent the borrowed
write lifetime. Adding `transaction_begin` would require an owned core state
machine, a way to retain or reacquire writer exclusion, thread and cancellation
rules, abandoned-transaction cleanup, checkpoint and retention behavior, and a
safe relationship between a transaction and its parent database. It would also
invite callers to hold the only writer across arbitrary application work.

The immediate caller requirement is narrower: submit more than one copied
`put`/`delete` mutation and publish all of them as one existing committed
generation. The future service design independently describes this as a
batched request whose clients do not own transaction lifetimes.

## Evidence

- Tests or fuzzing: core transaction tests already prove multi-key
  commit/rollback, read-your-writes inside the Rust callback, crash recovery,
  transaction-WAL limits, and panic conservation. The FFI hostile corpus proves
  kind-checked bounded handles, finalizer-thread close, and consuming close/use
  races, but has not exercised a mutable batch.
- Independent consumers: the C harness needs atomic multi-mutation behavior;
  it has not requested interactive staged reads, savepoints, or a long-lived
  writer. The SDD service sketch also uses one submitted batch.
- Diagnostics or audits: the existing C outcome algebra can represent success
  and structured storage failure. It reserves `not_applied`, but no foreign
  conditional-write contract or owner-scoped version token exists.
- Repeated implementation friction: an exact post-commit generation cannot be
  recovered by calling `connection_info` after `write`; another writer may
  commit between those operations. The callback can observe only pre-commit
  state through its current public contract.
- Missing evidence: memory-limit calibration, duplicate-key expectations from
  a real language wrapper, conditional batch demand, accurate generation-result
  demand, cancellation, and any consumer requiring interactive transactions.

## Ownership And Dependency Analysis

Core owns logical mutation, atomic commit/rollback, WAL admission, durable
generation publication, and typed storage outcomes. The adapter may own copied
foreign representation, builder state, per-batch resource admission, and
one-shot submission. It must not own page/WAL planning or reinterpret whether a
core commit occurred.

The admitted experimental direction is:

```text
C caller
   |
   v
adapter-owned bounded batch of copied logical commands
   |
   | one consuming execute call
   v
SharedKvStore::write(callback-scoped KvWriteTransaction)
   |
   v
existing atomic storage and recovery mechanism
```

No mutex guard, `BTree`, `KvWriteTransaction`, borrowed slice, or pointer into
foreign memory survives an exported call. `tosumu-core` remains unaware of C
handles, batch registry state, and adapter limits.

## Candidate Batch Contract

The private prototype may add a mutable batch handle with these rules:

- creation produces an empty adapter-owned builder;
- append copies every key and value before returning and validates existing
  core key/value limits immediately;
- only unconditional logical `put` and `delete` commands are admitted first;
- commands execute in append order inside one `SharedKvStore::write` callback;
- repeated keys are allowed, so the last successful command determines the
  staged value; no intermediate state is externally visible;
- close/abort discards copied commands without touching the database;
- execute consumes and removes the batch before entering core, whether the
  resulting operation commits, rolls back, panics, or returns a storage error;
- a caller that wants to retry must construct a new batch and must respect
  existing committed-but-flush-failed outcome semantics;
- append and execute are creating-thread-only; close remains
  finalizer-thread-safe; and
- explicit command-count and aggregate copied-payload ceilings reject growth
  before allocation. Checked accounting includes every copied key and value,
  while adapter framing overhead and core WAL limits remain separate bounds.

The first prototype returns ordinary success without a generation token. It
must not obtain a supposed commit generation through a later racy observation
or infer one from private counters. An exact generation result requires a
separate provider-neutral core contract and ADR-0006/0007 review.

The first prototype also excludes conditional commands. `put_if_absent`, value
compare-and-set, and generation conditions require a deliberate answer about
whether predicates observe the initial committed view or earlier staged
commands, whether one miss rejects the whole batch, and how a truthful
`NotApplied` generation is returned. Reserving an outcome tag is not evidence
for those semantics.

## Alternatives Considered

### Alternative A: Adapter-Owned Copied Command Batch

- Benefits: reuses the accepted callback-scoped core transaction; retains no
  borrow or writer lock across calls; copies foreign inputs at a narrow unsafe
  boundary; naturally composes with a future stateless service request.
- Costs: adds a mutable adapter handle and a second resource ceiling; cannot
  offer interactive staged reads or an exact commit generation through the
  current core API.
- Failure mode: calling it a transaction would encourage assumptions about
  reads, savepoints, conditions, or lifetime that it does not implement.

### Alternative B: Single Call With A Foreign Descriptor Array

- Benefits: no builder handle or abandoned-batch state; naturally one-shot.
- Costs: requires nested pointer/length validation or a new durable command
  encoding before Swift/Kotlin caller evidence; large calls concentrate all
  foreign allocation pressure at one unsafe entry point.
- Failure mode: descriptor aliasing and partial validation create a larger
  unsafe surface, while a byte encoding can accidentally become a compatibility
  protocol.

### Alternative C: Core-Owned Transaction Handle

- Benefits: could eventually provide interactive staged reads, conditions, and
  a common owned abstraction for more than FFI.
- Costs: changes ADR-0006's lifetime and writer-exclusion model; introduces
  abandoned writer, thread movement, cancellation, poisoning, parent-close,
  checkpoint-pressure, and long-duration admission questions without a caller.
- Failure mode: foreign application pauses retain the only writer, or cleanup
  implicitly performs fallible storage work after the caller has lost the
  original operation context.

### Alternative D: Expose Several Existing Single Writes

- Benefits: no new API or state.
- Costs: provides no multi-mutation atomicity.
- Failure mode: callers infer that adjacent successful calls form one commit
  and observe or recover partial logical updates.

## Findings

The current requirement is command batching, not an owned transaction. A
copied adapter batch preserves the core's accepted scoped writer and introduces
no new storage or format semantics. Its name must say `batch`, not
`transaction`, and its one-shot execution must be explicit.

The prototype still needs falsification around capacity, append failure,
duplicate keys, empty batches, close/execute races, wrong-thread use, panic,
rollback, and committed-but-flush-failed outcomes. Until that evidence exists,
the candidate state machine and limits are experimental.

## Disposition

**Incubating.** Admit a private adapter-owned copied `put`/`delete` batch
prototype only. Do not add a core-owned transaction, conditional batch command,
generation result, stable ABI symbol, or compatibility promise in this cycle.

## Required Follow-Up

- [ ] Retain concrete command-count and copied-payload limits and show rejection
      leaves the builder usable and the database untouched.
- [ ] Implement the private batch state machine without changing
      `tosumu-core` or durable bytes.
- [ ] Add Rust hostile cases for wrong-kind/stale/thread/close/execute races,
      append allocation accounting, duplicate keys, empty batches, and panic.
- [ ] Extend the independent C caller with commit, explicit abort, rollback,
      reopen conservation, and limit cases.
- [ ] Decide from wrapper or service evidence whether conditional commands or
      exact generation results justify reopening ADR-0006/0007.

## Reopening Triggers

Reopen before adding staged reads, scans, savepoints, conditional commands,
serialized or cross-owner version tokens, returned commit generations,
non-consuming retry, cancellation, asynchronous submission, cross-thread
mutation, or any owned core write transaction. Reopen if measured copied-batch
memory or preparation cost requires snapshot-based parallel write planning.

## Review History

### Cycle 1 -- 2026-09-03

- Status entering review: Proposed.
- New evidence: Slice 2 completed the foreign ownership corpus; the existing
  core transaction remains callback-scoped, while both C and the future service
  shape require only submitted atomic mutation groups.
- Findings: a copied command batch can reuse the accepted core write mechanism
  without retaining a borrow or writer authority across calls. Exact generation
  and conditional outcomes cannot be honestly projected from the current core
  contract.
- Disposition: Incubating; authorize the bounded private adapter batch
  prototype and no core or compatibility change.
- Resulting ADR or documentation change: AR-0019 opened; MVP+11 Slice 3 remains
  active pending the prototype and independent C evidence.
