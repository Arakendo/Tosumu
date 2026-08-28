# AR-0009: Multiple-Reader Execution And Coordination

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-27 |
| Last reviewed | 2026-08-27 |
| Scope | Core storage / transaction coordination / platform mechanism |
| Trigger | MVP+10 requires a baseline for locking, LSN visibility, and reader/writer behavior before MVCC-style work begins |
| Related ADRs | ADR-0001, ADR-0002, ADR-0004, ADR-0005 |
| Related evidence | Main Feature Roadmap, SDD §§7 and 28.4, AR-0011, current pager/WAL/transaction implementation |

## Architectural Question

Which multiple-reader, snapshot, writer-gate, checkpoint, and execution-policy
semantics belong to Tosumu core, and which threading, waiting, and host
mechanisms remain replaceable implementation details?

## Context

The normative design describes a shared database handle, one writer, multiple
readers, committed-LSN snapshots, checkpoint blocking, and typed `Send`/`Sync`
intent. ADR-0004 admits the writer gate, and ADR-0005 now admits committed
generations, process-local reader pins, retained-WAL selection, finite limits,
and reader-aware checkpoint suppression. Their private executable composition
does not yet admit the public shared-handle/session API described by the SDD.

Concurrency risks mixing several separate questions: what a reader is allowed
to observe, how long its snapshot remains valid, how a writer is serialized,
what prevents checkpoint truncation, whether a caller waits or receives
`Busy`, and whether any mechanism uses threads, locks, queues, or asynchronous
work. A platform mechanism must not silently define storage visibility or
durability semantics.

## Evidence

- Tests or fuzzing: `tests/mvp10_baseline.rs` retains focused reader visibility,
  writer contention, maintenance-gate, and lifecycle evidence. It proves that
  existing read-only handles are live views, not snapshots.
- Independent consumers: the provider boundary now proves structured
  `FILE_OPEN_BUSY` rejection for a second cooperating writer; no independent
  consumer yet proves concurrent snapshot semantics.
- Diagnostics or audits: private diagnostics report active/maximum readers,
  oldest generation, retained WAL bytes/frame versions, checkpoint/latest
  horizons, and checkpoint-blocked state. This is not yet the public SDD
  `connection_info` schema.
- Repeated implementation friction: the first private transaction composed as
  `Sync` accidentally; SDD section 28.4 required a structural `Send`/`!Sync`
  correction before public admission.
- Missing evidence: public names and visibility, an independent snapshot
  consumer, session identity/age diagnostics, and any future blocking,
  cancellation, or timeout policy.

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

ADR-0004 accepted and the implementation completed this alternative as the
first coordination slice. It remains narrower than the alternatives for reader
snapshots.

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

| Path | Current mutation | Gate treatment |
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
writer sidecar. The accepted gate is cooperative and advisory: it coordinates
participating Tosumu mutation paths, not arbitrary file writers.
- Any change to the on-disk format, WAL retention model, public transaction
  contract, or ownership boundary requires explicit review and possibly a new
  ADR.

### Snapshot Admission Findings

- The current WAL LSN is an append position within one WAL lifetime, not a
  database-wide committed generation. `WalWriter::truncate` resets the next LSN
  to 1, and a successful transaction truncates immediately after copying its
  frames into the main file.
- `OFF_WAL_CHECKPOINT_LSN` exists in page zero but is initialized to zero and is
  not advanced by current commit, recovery, or checkpoint paths. It cannot yet
  serve as the committed-LSN source of truth.
- A commit record's assigned LSN is currently ignored by the pager. Page
  versions are per-page rewrite counters and do not identify one atomic commit.
- Successful transaction commit overwrites current main-file frames and then
  discards their WAL copies. Neither the previous main-file frame nor a retained
  WAL version remains available to an older reader.
- Ordinary `put` and `delete` calls outside an explicit transaction can write
  page frames directly to the main file. Snapshot publication cannot become
  coherent while this path bypasses a common commit generation.
- A read-only pager captures page-zero metadata and any committed WAL overlay
  once at open, but subsequent page misses read the live main file. This can
  combine open-time root/page-count state with later page contents; it is not a
  usable seed for an LSN-pinned contract.
- The SDD's intended shape—main file at a checkpoint generation, newer
  committed versions retained in WAL, and reads selecting the latest frame no
  newer than their captured LSN—requires a deliberate publication and
  checkpoint change. Adding only a reader registry would pin nothing.
- A process-local reader registry is only sufficient if snapshots belong to one
  shared database owner that retains the cross-process writer gate. Independent
  read-only handles must either remain explicitly live-view handles or join a
  separately admitted cross-process reader protocol.

## Disposition

Incubating for the broader MVP+10 reader/snapshot question. ADR-0004 accepts
Alternative D for the first, narrower writer-admission slice. Do not add a
general executor, async runtime, background worker, reader snapshot claim, or
public MVCC contract through that implementation.

## Required Follow-Up

- [x] Create `docs/Plans/mvp-10-multiple-readers.md` from the plan template.
- [x] Record the current file-lock, pager-lock, transaction, and WAL ownership
      graph.
- [x] Complete the executable baseline for simultaneous readers, writer
      contention, LSN visibility, and checkpoint interaction. Handle admission
      and visibility evidence now includes a private shared owner whose pinned
      logical reads survive concurrent commits and visibly defer checkpointing.
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
- [x] Define one monotonic committed-generation source of truth across commit,
      checkpoint, WAL truncation, close, recovery, and reopen.
      ADR-0005 accepts the AR-0011 contract and format 3 makes it executable.
- [x] Decide whether all ordinary writes become implicit transactions or use
      another single publication path; no main-file mutation may bypass the
      admitted commit generation.
- [x] Define frame residence and selection between the checkpointed main file
      and retained WAL versions, including crash ordering and page-zero state.
- [x] Scope snapshot readers to a shared database owner or admit a
      cross-process reader-registration protocol. Preserve independent live-view
      handles only if their weaker contract is explicit.
- [x] Decide the format and migration impact before implementing retained
      versions or assigning meaning to `OFF_WAL_CHECKPOINT_LSN`.
      Coordinate that decision with AR-0006.

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

### Cycle 5 -- 2026-08-27

- Status entering review: Incubating
- New evidence: ADR-0004 is implemented and validated through pager-lifetime
  admission, maintenance paths, provider reporting, and the full workspace
  suite. A source trace covered commit LSN assignment, page-zero checkpoint
  state, direct writes, read-only overlay, recovery, and WAL truncation.
- Findings: the current LSN is WAL-epoch-local, old page versions are discarded
  on successful commit, direct writes lack a commit generation, and read-only
  handles mix open-time metadata with live main-file reads. A registry alone
  cannot create snapshot semantics.
- Disposition: remain Incubating. Close the committed-generation, publication,
  version-residence, reader-scope, and format questions before Slice 2 code.
- Resulting ADR or documentation change: none; the snapshot admission gate is
  retained here and in the MVP+10 plan.

### Cycle 6 -- 2026-08-27

- Status entering review: Incubating
- New evidence: ADR-0005 accepted the format-v3 committed generation,
  shared-owner reader scope, retained-version selection, finite bounds, and
  zero-reader checkpoint ordering. The pager now owns the registry and proves
  that an active pin retains WAL and leaves the main-file horizon unchanged
  across writer and independent live-view reads; the next zero-reader commit
  checkpoints the complete latest state.
- Findings: the storage residence prerequisite is closed without inventing a
  public session or executor. Stable generation-selecting reader behavior and
  checkpoint diagnostics remain unresolved broader coordination work.
- Disposition: remain Incubating for the shared owner and reader API.
- Resulting ADR or documentation change: required storage-decision follow-ups
  are complete through ADR-0005; retained residence is private executable
  behavior.

### Cycle 7 -- 2026-08-27

- Status entering review: Incubating
- New evidence: a private `Arc<Mutex<BTree>>` owner serializes snapshot capture
  with commits, then lets a non-cloneable read transaction retain its pin while
  locking only for each point or range operation. A writer thread commits newer
  values while the reader repeatedly observes its captured generation.
  Diagnostics expose the finite pin bound, oldest generation, retained WAL
  bytes and frame versions, checkpoint/latest horizons, and checkpoint-blocked
  state. Reader drop is passive; the next zero-reader commit reclaims the WAL.
- Findings: the shared-owner scope accepted by ADR-0005 now has an executable
  ownership and lifecycle proof. The experiment does not settle public names,
  session policy, cancellation, timeouts, shutdown, or independent-handle
  snapshot semantics.
- Disposition: remain Incubating for the public shared-owner and reader API.
- Resulting ADR or documentation change: MVP+10 Slice 2 is complete as private
  mechanism; the initial checkpoint diagnostic items move into executable
  Slice 3 evidence.

### Cycle 8 -- 2026-08-27

- Status entering review: Incubating
- New evidence: the private owner is structurally `Send + Sync`, while its read
  transaction is `Send` but not `Sync` in accordance with SDD section 28.4. A
  lifecycle fixture drops every owner handle while a reader remains, proves the
  reader still owns the pager and cross-process writer gate, then drops the
  reader and reopens successfully with the newer retained commit recovered.
- Findings: reader ownership now composes with last-handle shutdown, writer
  exclusion, and retained-WAL recovery. Because no blocking API is admitted,
  timeout and cancellation semantics remain explicitly absent rather than
  implied by the private mutex.
- Disposition: remain Incubating for the public shared-owner and reader API.
- Resulting ADR or documentation change: private lifecycle traits and teardown
  evidence now agree with the normative SDD.

### Cycle 9 -- 2026-08-27

- Status entering review: Incubating
- New evidence: the README, public roadmap, file-format summary, security
  limitation, normative SDD implementation-status note, main roadmap audit,
  document index, and MVP+10 plan now distinguish accepted format-3 mechanics,
  private shared-reader evidence, and the unimplemented public API.
- Findings: format and private lifecycle claims are coherent. Existing public
  independent read-only handles remain live views, and committed generations
  remain local visibility evidence rather than anti-rollback freshness proof.
  The SDD's richer session IDs, ages, writer queue, busy policies, partial
  checkpoints, cancellation, and timeouts are still targets, not current API.
- Disposition: remain Incubating until public names and one independent caller
  reveal the shared-reader contract.
- Resulting ADR or documentation change: MVP+10's private implementation plan
  is complete; the main roadmap advances to public contract admission.
