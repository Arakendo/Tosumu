# AR-0011: Committed Generation And Version Residence

| Field | Value |
| --- | --- |
| Status | Accepted |
| Opened | 2026-08-27 |
| Last reviewed | 2026-08-27 |
| Scope | Core storage / WAL / snapshot visibility / format compatibility |
| Trigger | AR-0009 Slice 2 requires an executable committed-LSN and retained-version contract before snapshot code begins |
| Related ADRs | ADR-0001, ADR-0002, ADR-0004, ADR-0005 |
| Related evidence | AR-0006, AR-0009, MVP+10 baseline tests, SDD §§7.2-7.8 and 13 |

## Architectural Question

What durable generation, page-version residence, transaction framing, and
checkpoint ordering can support LSN-pinned readers without weakening pager
authentication or letting format-v2 writers destroy retained history?

## Context

ADR-0004 now admits one cooperating writer while leaving readers ungated and
live-view. The next MVP+10 slice cannot be implemented by adding a reader count
to the current pager. Successful commits currently copy new frames into the
main file and truncate their WAL records; ordinary writes can bypass the WAL;
the next WAL LSN and transaction ID restart after truncation or reopen; and the
page-zero checkpoint LSN remains zero.

The SDD intends a checkpointed main file plus newer committed WAL frames. That
shape is directionally useful, but format-v2 does not implement its required
generation and retention rules. This review makes the missing rules explicit
without accepting a public `Database` API or format v3 prematurely.

## Evidence

- Tests or fuzzing: `tests/mvp10_baseline.rs` proves current live-view reads,
  cooperative writer rejection, unchanged checkpoint LSN, WAL LSN reset after
  successful commit, and direct publication without a WAL commit generation.
- Independent consumers: the provider boundary exercises transactions,
  recovery, backup/export, 64 MiB values, and structured writer contention. It
  does not yet require a stable snapshot API.
- Diagnostics or audits: inspection exposes `wal_checkpoint_lsn`, WAL records,
  and recovery disposition, but current values do not describe an active
  snapshot horizon.
- Repeated implementation friction: recovery and read-only overlay independently
  reconstruct committed transactions with a global committed-ID set and one
  current-transaction cursor. Retained WAL across reopen would expose duplicate
  transaction IDs and malformed framing that current clean-open truncation
  avoids.
- Missing evidence: retained-WAL size distribution, acceptable reader and WAL
  limits, cross-process snapshot demand, partial-checkpoint crash fixtures, and
  an independent caller for a shared database/session owner.

## Ownership And Dependency Analysis

- Core owns committed-generation meaning, transaction framing, page selection,
  checkpoint safety, retained-history bounds, and corruption diagnostics.
- Pager owns authenticated frame encode/decode and may select a physical frame
  only through core-owned snapshot state.
- WAL owns durable records and scan validation; a record's byte position or an
  operating-system file handle does not define commit visibility by itself.
- A future shared database owner may hold the writer guard, committed index,
  and process-local reader registry. It must not move SQL, host scheduling, or
  consumer transaction meaning into core.
- Hosts may choose when to start or end a read transaction. They may not
  reinterpret an LSN, reclaim a pinned frame, or bypass a retention limit.
- Independent legacy read-only handles are not automatically snapshot owners.
  Their current live-view behavior must remain explicit unless they join an
  admitted coordination protocol.

## Candidate Contract

The preferred candidate uses a checkpointed main-file generation plus retained
committed WAL versions under one shared database owner.

| Concern | Candidate meaning |
| --- | --- |
| Genesis | Committed generation 0 is the newly created checkpointed main file. |
| Commit generation | The LSN assigned to a matching durable `Commit` record is the generation published by that transaction. |
| Monotonicity | Database commit generations never decrease or restart after checkpoint, WAL truncation, close, recovery, or reopen. |
| Main file | Data and page-zero metadata represent exactly `wal_checkpoint_lsn`. |
| WAL | Contains complete transaction records newer than the main-file checkpoint, plus any older records not yet safely reclaimed. |
| Page selection | For snapshot N, select the newest frame for the page whose owning commit generation is `<= N`; fall back to the checkpointed main frame. |
| Current view | The writer and a newly opened snapshot use the latest durable committed generation, not the latest appended page-write LSN. |
| Uncommitted data | Never enters a committed page index and remains invisible outside the writer transaction. |
| Reader pin | A registered read transaction retains its captured generation until drop. |
| Reclamation horizon | No frame required to reconstruct the oldest registered snapshot may be reclaimed. |

Page-write record LSNs remain physical record identities. They do not become
independent visible generations. Every page write between one valid `Begin` and
its matching `Commit` is published atomically at the commit record's LSN.

### Publication ordering

Every ordinary logical mutation must use one transaction publication path.
An implicit single-operation transaction is acceptable; a direct main-file
write is not.

```text
Begin + PageWrite records
    -> append Commit
    -> fsync WAL
    -> publish commit LSN and immutable committed-frame index in memory
    -> return success
```

The commit path does not need to overwrite the main file before returning.
Failure before the durable commit is uncommitted. Failure after the WAL fsync
is a committed outcome even if in-memory publication or later checkpoint work
fails; reopen reconstructs the durable committed index from WAL.

### WAL structural validation

A retained WAL must be validated as a transaction stream, not classified by a
global set of transaction IDs alone:

- record LSNs are strictly increasing and greater than the durable checkpoint
  horizon when they represent unreclaimed work;
- `Begin` cannot nest;
- `PageWrite` requires one active transaction;
- `Commit` must match that active transaction;
- only the matching commit publishes those frames;
- an incomplete tail transaction remains uncommitted;
- transaction IDs cannot be assumed unique unless the writer seeds its next ID
  from validated retained state.

The implementation may either make transaction IDs monotonic across reopen or
scope them to structurally parsed transactions. It may not use ID equality
alone to merge records from different WAL lifetimes.

### Checkpoint ordering

For a target generation T that does not pass the oldest reader horizon:

```text
apply the latest committed frame <= T for each changed data page
    -> write page-zero metadata for T with wal_checkpoint_lsn = T
    -> authenticate page zero where its protector requires a header MAC
    -> fsync the main file
    -> mark records <= T reclaimable
    -> reclaim only through a crash-safe WAL operation
```

If a crash occurs before the main-file fsync, the retained WAL remains the
recovery authority. If it occurs after that fsync but before reclamation, replay
or ignoring records at or below the checkpoint is idempotent. WAL bytes must
not be removed before the checkpointed main state and page-zero horizon are
durable.

The WAL sidecar remains present when empty. Reclamation truncates and syncs the
existing file rather than deleting and recreating it, so correctness does not
depend on a directory-entry durability window. Checkpoint success is not
reported until the truncation metadata is synced. If that sync fails, the main
file is already authoritative at T and reopening must accept any surviving
obsolete prefix/tail as records at or below the checkpoint horizon.

The preferred Slice 2 scope is narrower: do not move the main checkpoint while
any registered snapshot exists. When the reader count is zero, a full
checkpoint may advance to the latest durable commit and truncate the entire
WAL. This keeps the main file immutable for every Slice 2 snapshot and avoids
prefix rewrite before partial checkpointing has its own crash evidence.

Reader `Drop` only unregisters the pin. It must not hide fallible checkpoint or
reclamation work. A later explicit checkpoint, close operation, write-admission
path, or documented maintenance trigger may observe zero readers and perform
the full checkpoint. Prefix compaction, passive checkpoint reports, and waiting
modes remain Slice 3 work, but retained growth still needs an explicit limit
and diagnostic before Slice 2 completes.

### Crash-ordering matrix

| Last completed boundary | Durable interpretation after reopen | Required handling |
| --- | --- | --- |
| Before `Commit` append | Transaction is uncommitted | Ignore its frames; retain the prior committed generation |
| After `Commit` append but before WAL sync | Commit durability is not established | Surface the write/sync failure; recovery accepts only a complete durable record stream |
| After WAL sync but before in-memory publication | Transaction is committed | Rebuild the committed index from WAL; prepare allocations before sync so post-sync publication is an infallible state change |
| During data-page checkpoint writes before main-file sync | WAL remains authoritative | Expose no handle before recovery reapplies committed frames and restores coherent page zero |
| After checkpointed page zero and main-file sync, before WAL truncate | Main file durably represents T; WAL may contain obsolete records `<= T` | Ignore or idempotently replay obsolete records, then finish reclamation |
| After WAL `set_len(0)` but before truncation sync | Main file still durably represents T; reclamation durability is unknown | Report checkpoint failure; on reopen accept an empty WAL or surviving obsolete bytes, all bounded by T |
| After full WAL truncate and sync | Main file is authoritative at T and the persistent WAL is empty | Seed the next record LSN at `T + 1` rather than restarting at 1 |
| Reopen with page-zero horizon T and nonempty WAL | Main is the base at T; only complete committed transactions above T can advance current state | Reject decreasing/duplicate post-horizon LSNs, ignore obsolete records `<= T`, and ignore only a structurally incomplete final transaction/torn tail |

The in-memory committed-frame index must be fully prepared before the commit
fsync. After that fsync, publication should be an infallible pointer/state swap;
otherwise an allocation failure would require a new committed-but-not-published
terminal outcome.

### Reader scope

The preferred first scope is read transactions created by one shared database
owner. That owner retains ADR-0004's cross-process writer guard for its lifetime,
so another cooperating process cannot publish writes behind its process-local
reader registry. Independent `open_readonly` handles remain live-view handles
and do not block checkpointing unless a later review admits a cross-process
reader protocol.

This scope does not yet accept the SDD's public `Database`, `Session`, or
`ReadTransaction` types. Independent caller evidence must shape that API after
the storage contract is executable behind a private boundary.

### Format and raw-WAL impact

This candidate is behaviorally incompatible with format v2 even if existing
page and `PageWrite` byte layouts are reused:

- v2 writers reset LSNs and truncate retained records;
- v2 readers do not select frames by owning commit generation;
- v2 ordinary writes may bypass the WAL;
- v2 recovery does not validate the retained transaction stream described
  above.

Format v3 is therefore the leading candidate and must exclude v2 writers.
AR-0006 owns the clean-break versus offline-rewrite decision. Automatic
migration during ordinary open remains rejected.

The public raw `WalWriter` boundary also reopens. An empty physical WAL cannot
derive the database's next monotonic LSN without checkpoint context. Viable
follow-ups are a versioned WAL header/epoch, database-seeded crate-private
mutation, or a deliberately physical API that cannot be used as a database WAL
writer. Continuing to open an empty database WAL at LSN 1 is not compatible
with this candidate.

The preferred resolution is one source of epoch authority rather than a second
WAL header:

- page-zero `wal_checkpoint_lsn`, under the existing protector-specific header
  authentication rules, supplies the durable lower bound;
- the database-owned WAL opener validates retained record LSNs and seeds an
  empty WAL at `wal_checkpoint_lsn + 1`;
- inspection continues to start from the database path and can report both the
  checkpoint horizon and retained records without treating a detached WAL as a
  database;
- backup and export continue treating main/WAL as a pair and do not need to
  reconcile duplicate epoch fields; and
- raw `WalWriter` construction remains useful only as an epoch-local physical
  fixture mechanism and is removed from the future coordinated database
  mutation path. Its mutation visibility should become crate-private or an
  explicitly non-database test-support boundary before format v3 ships.

This preference does not strengthen Sentinel header authentication. It relies
only on the page-zero trust rules already documented for each protector.

### Retained-growth evidence

The current physical framing makes the lower bound measurable:

- `Begin` and `Commit` each occupy 25 bytes;
- one full-page `PageWrite` occupies 4,129 bytes; and
- a 64 MiB logical value requires at least 16,636 overflow pages at the current
  4,034-byte overflow payload size.

Before accounting for the leaf, page-zero, split, or rewritten metadata frames,
that one value therefore requires at least 68,690,094 WAL bytes (about 65.51
MiB). A practical retained-WAL limit must admit more than that lower bound.

The overwrite case is materially larger. The current B+ tree allocates the new
overflow chain before retiring the old one, so replacing an existing maximum
value with no reusable freelist dirties both 16,636-page chains, the leaf, and
page zero. An ignored executable evidence test measures exactly 33,274 final
page frames and 137,388,396 encoded WAL bytes (about 131.02 MiB). This falsifies
a 128 MiB transaction ceiling even though every individual value is valid.

More importantly, `MAX_VALUE_SIZE` is a per-value bound, not a transaction
bound. The public transaction closure may write any number of accepted values
and may rewrite a page repeatedly. Before commit staging, rollback discarded
dirty in-memory frames but left an already-appended uncommitted WAL tail. The
staged path now makes ordinary rollback append-free and exposes the final unique
frame set, but no hard ceiling exists until commit admission checks that exact
set before appending `Begin`.

Tokimu's current independent adapter supplies the representative shape: it
serializes the entire Resource Space as one JSON value and commits that value
in one Tosumu transaction. Its corpus does not admit a narrower application
maximum, so the engine's supported 64 MiB value boundary and overwrite behavior
remain the conservative evidence for the initial private default.

The preferred limit shape is therefore provisional but explicit:

- cap active snapshot registrations independently and reject a new snapshot
  with a typed resource/busy diagnostic when the cap is reached;
- expose retained committed bytes and the oldest pin before enforcing policy;
- do not advertise a hard retained-WAL byte cap until transactions themselves
  have an enforceable WAL-frame or byte budget and failed/uncommitted tails can
  be reclaimed safely; and
- treat a configurable soft watermark as checkpoint/admission pressure, not as
  proof that the sidecar cannot exceed that number.

Numeric defaults were blocked until representative Tokimu pressure and an
append-free ordinary-abort contract existed. Commit staging and the executed
maximum-value overwrite now provide that first private-contract evidence; later
independent callers may still reopen the defaults before public stabilization.

With commit staging admitted and Tokimu's current one-snapshot-value adapter as
the representative provider, the preferred initial private limits are:

| Limit | Candidate default | Rationale |
| --- | ---: | --- |
| Final encoded WAL bytes per transaction | 160 MiB | Admits the measured 131.02 MiB maximum-value overwrite with about 22% structural headroom; exact size is checked before append. |
| Retained committed WAL bytes | 512 MiB | Admits three measured worst-case overwrites plus framing/headroom; a fourth cannot begin publication while history is pinned. |
| Registered process-local snapshots | 64 | Keeps registry/debug output finite while retained bytes, rather than pin count alone, remains the primary pressure control. |

These are experimental engine defaults, not format constants or durability
guarantees. The private owner may accept lower configured limits, but never
below the bytes required by a transaction it has already admitted. At commit,
the writer checks both `transaction_bytes <= transaction_limit` and
`retained_committed_bytes + transaction_bytes <= retained_limit` using checked
arithmetic before appending `Begin`. Rejection is typed and leaves the WAL
unchanged. Snapshot registration is likewise fail-fast at its count limit.

The public names and configurability remain provisional until the private
storage contract and independent caller exercise these outcomes.

### Transaction staging and abort mechanics

The preferred abort contract avoids fallible physical rollback for ordinary
caller cancellation. The pager already retains the final encrypted frame for
each dirty page in a transaction-local map. It should stage the WAL transaction
from that map at commit time instead of appending `Begin` and every intermediate
`PageWrite` during mutation:

```text
mutate transaction-local pages
    -> on caller error: discard pages; no WAL bytes were appended
    -> on commit: build final page-zero frame and sorted unique page frames
    -> compute exact encoded transaction bytes and enforce the admitted budget
    -> append Begin + final PageWrite frames + Commit
    -> sync WAL
    -> publish committed index with an infallible state swap
```

This makes the transaction byte budget a bound over final unique frames rather
than an operation counter. Rewriting one page repeatedly does not inflate the
committed history, and a rejected over-budget transaction fails before its
first WAL byte. It also preserves the existing closure guarantee that returning
an error rolls back without hiding a second cleanup failure.

An append or sync failure during commit is different: durability may be
ambiguous and a torn/incomplete tail may exist. The handle must become poisoned
and reject further work; reopen validates the stream, trims only the incomplete
physical tail, and determines whether a complete synced commit exists. The
engine must not silently truncate a complete commit merely because `sync`
returned an error. The admitted transaction budget bounds this exceptional
tail even before a retained-WAL ceiling is selected.

## Alternatives Considered

### Alternative A: Copy the whole database for each read transaction

- Benefits: immutable snapshots can avoid retained WAL and reader pinning.
- Costs: O(database size) work and storage per read; stable capture still needs
  coordination and does not match the intended embedded read path.
- Failure mode: bounded double-copy observation is mistaken for cheap MVCC or a
  guaranteed atomic capture under continuous writes.

### Alternative B: Keep process-local before-images in memory

- Benefits: no immediate physical-format change; old frames disappear with
  readers after process exit.
- Costs: duplicates authenticated frames in memory, complicates recovery and
  reopen semantics, and diverges from the normative checkpoint/WAL model.
- Failure mode: a path overwrites main state before preserving the required
  before-image, or memory pressure becomes unbounded and invisible.

### Alternative C: Retain committed WAL versions above a checkpoint horizon

- Benefits: matches the SDD direction, reuses authenticated full-page frames,
  gives commit LSN one inspectable meaning, and composes with recovery.
- Costs: requires common publication, retained-history indexing, reader pins,
  checkpoint redesign, bounds, and a compatibility decision.
- Failure mode: LSN epochs restart, transaction framing is ambiguous, or
  checkpoint removes a frame still needed by a reader.

### Alternative D: Hold a shared file lock for each reader

- Benefits: small mechanism and cross-process visibility.
- Costs: a writer cannot publish while any reader lives, contrary to the
  multiple-reader snapshot target.
- Failure mode: the project labels blocking read/write exclusion as MVCC.

## Findings

- Alternative C is the only candidate that currently satisfies the intended
  writer-can-commit-while-readers-live contract without copying the database.
- Existing page frames can be reused, but their reuse does not avoid a durable
  format and compatibility decision.
- Commit LSN, not page-write LSN or page version, is the viable atomic snapshot
  generation.
- The shared owner and writer guard can make the first reader registry
  process-local without claiming that independent read-only handles are pinned.
- Common publication, structural WAL validation, raw-WAL treatment, and a
  retained-growth limit are prerequisites, not cleanup after snapshot code.
- A per-value size limit alone cannot enforce a retained-WAL ceiling because one
  transaction may contain multiple values; exact staged-commit admission must
  enforce the aggregate.
- Commit-time staging of final unique frames can make ordinary rollback
  infallible and the transaction WAL size exactly preflightable; commit I/O
  ambiguity instead poisons the handle until validated reopen.
- Initial private reader/transaction/WAL defaults and v2 refusal are selected;
  public shared-owner API and configuration names still lack caller evidence.

## Disposition

Accepted through ADR-0005. Alternative C, retained committed WAL versions above
one checkpoint horizon, is now binding architecture. Public owner/session names
and the remaining implementation evidence stay provisional as listed below.

## Required Follow-Up

- [x] Confirm the candidate ordering with a crash matrix for commit, main
      checkpoint, page-zero horizon, persistent-sidecar truncation, and reopen.
- [ ] Turn the admitted matrix into format-v3 crash fixtures before semantic
      snapshot behavior ships.
- [x] Prefer page-zero checkpoint state as the sole epoch/base authority; do not
      add a duplicate WAL header solely to seed monotonic LSNs.
- [x] Remove raw `WalWriter` from the future coordinated database mutation path;
      retain at most a crate-private or explicit physical fixture boundary.
- [x] Quantify the current 64 MiB single-value WAL lower bound and establish
      that the existing per-value limit does not bound a transaction.
- [x] Define the transaction abort mechanism: stage final unique frames, reject
      an over-budget commit before append, and poison on commit I/O ambiguity so
      reopen can validate and trim only an incomplete tail.
- [x] Select evidence-backed initial private limits: 160 MiB encoded WAL per
      transaction, 512 MiB retained committed WAL, and 64 registered snapshots.
      Keep public configuration provisional until caller evidence.
- [x] Define format-v3 open/refusal and optional v2 offline logical rewrite
      behavior under AR-0006: exact v3 support, explicit v2 refusal, and no
      rewrite until non-regenerable data establishes preservation pressure.
- [ ] Build the storage contract behind a private boundary and exercise it
      through the provider plus one independent caller before stabilizing shared
      database/session types.
- [x] Promote accepted generation, publication, retention, and compatibility
      rules to ADR-0005 before changing semantic storage behavior.

## Reopening Triggers

- A consumer requires snapshots across independent processes.
- Representative WAL pressure makes retained full-page frames infeasible.
- Crash evidence requires a different main/WAL publication order.
- A versioned WAL header changes backup, export, inspection, or raw-WAL scope.
- An independent caller requires a public ownership shape before the private
  contract can be exercised.

## Review History

### Cycle 1 -- 2026-08-27

- Status entering review: Proposed
- New evidence: AR-0009 Cycle 5 and focused MVP+10 tests establish the missing
  generation, publication, and retention behavior in format v2. Additional
  source tracing found transaction-ID restart and raw-WAL LSN restart hazards.
- Findings: retained committed WAL versions above a checkpoint horizon are the
  only current candidate that supports nonblocking snapshot readers without
  whole-database copies. It requires format, raw-WAL, limit, and crash decisions.
- Disposition: Incubating with Alternative C preferred.
- Resulting ADR or documentation change: none.

### Cycle 2 -- 2026-08-27

- Status entering review: Incubating
- New evidence: recovery and read-only WAL overlay now share one private
  sequential transaction analyzer. Four focused tests cover matching commit,
  incomplete reuse of a prior transaction ID, mismatched commit, and nested
  begin behavior; all 211 active core tests and strict workspace Clippy pass.
- Findings: commitment can be derived from sequential framing without global
  transaction-ID uniqueness. This closes one unsafe assumption before retained
  WAL work, but strict malformed-stream diagnostics still need record-offset
  provenance and remain part of the format-v3 design.
- Disposition: remain Incubating with Alternative C preferred.
- Resulting ADR or documentation change: no semantic or format change; shared
  private transaction analysis is now executable evidence.

### Cycle 3 -- 2026-08-27

- Status entering review: Incubating
- New evidence: supported inspection begins with a database path, stable backup
  preserves a main/WAL pair, page zero already carries the checkpoint horizon,
  and no independent production caller requires raw WAL mutation. The raw
  writer is otherwise used primarily by physical/recovery fixtures.
- Findings: a WAL header would duplicate epoch authority and introduce mismatch
  states without current consumer value. The database owner can seed monotonic
  LSNs from page zero and validated retained records. Raw mutation cannot remain
  a coordinated database path because it lacks that context.
- Disposition: remain Incubating; prefer page zero as the sole epoch authority
  and a database-seeded internal writer boundary.
- Resulting ADR or documentation change: none until the complete snapshot
  contract is promoted.

### Cycle 4 -- 2026-08-27

- Status entering review: Incubating
- New evidence: current record framing makes a 64 MiB value consume at least
  68,690,094 WAL bytes, but `MAX_VALUE_SIZE` applies per value and transaction
  closures have no aggregate frame/byte bound. Rollback does not truncate the
  appended uncommitted tail. Tokimu's adapter commits one whole serialized
  Resource Space snapshot but does not yet define its maximum size.
- Findings: a pre-begin retention check is only a soft watermark while admitted
  transactions can overshoot without bound. A hard retained-WAL ceiling first
  requires bounded transaction admission and safe tail reclamation.
- Disposition: remain Incubating; separate reader registration, transaction,
  and retained-history limits rather than hiding all three behind one WAL size.
- Resulting ADR or documentation change: none until numeric defaults and abort
  mechanics have executable evidence.

### Cycle 5 -- 2026-08-27

- Status entering review: Incubating
- New evidence: the candidate already orders page frames and authenticated page
  zero before one main-file sync; current reclamation truncates and syncs an
  existing persistent sidecar rather than deleting it.
- Findings: every crash boundary has one recovery authority. Before main sync,
  WAL wins; after main sync, page zero at T wins and surviving WAL bytes at or
  below T are obsolete. An empty reopened WAL must start at `T + 1`. The matrix
  does not require prefix checkpointing while Slice 2 forbids checkpoint with
  registered readers.
- Disposition: ordering candidate confirmed for Slice 2; executable format-v3
  crash fixtures remain an implementation gate.
- Resulting ADR or documentation change: none until the format contract is
  accepted.

### Cycle 6 -- 2026-08-27

- Status entering review: Incubating
- New evidence: AR-0006 records the exact v3 behavioral delta, current
  direction-specific version validation, and Tokimu's regenerable format-2
  fixture posture.
- Findings: snapshot-capable ordinary open must accept an explicit v3 interval
  and refuse v2 before recovery. Optional preservation is a future offline
  logical rewrite to a separate destination, never automatic or in-place.
- Disposition: remain Incubating; compatibility behavior is specific enough for
  an ADR but remains coupled to unresolved transaction/retention limits.
- Resulting ADR or documentation change: AR-0006 Cycle 3 records the preferred
  clean-break contract.

### Cycle 7 -- 2026-08-27

- Status entering review: Incubating
- New evidence: current pager mutation already keeps one final encrypted frame
  per dirty page, while eagerly appended WAL records duplicate intermediate
  writes and make rollback reclamation fallible. `HANDLE_POISONED` already
  exists as a typed terminal handle state.
- Findings: commit-time WAL staging makes ordinary rollback append-free and
  permits exact budget admission over final unique frames. A commit append/sync
  failure remains durability-ambiguous and must poison the handle until reopen
  validates the tail; it must not be treated as ordinary rollback.
- Disposition: remain Incubating; prefer staged commit as the transaction-bound
  mechanism. Numeric budget evidence remains open.
- Resulting ADR or documentation change: no format change; implementation may
  first prove equivalent format-v2 commit/recovery behavior as preparation.

### Cycle 8 -- 2026-08-27

- Status entering review: Incubating
- New evidence: format-v2 pager commits now stage `Begin`, sorted final unique
  page frames, optional page zero, and `Commit` only after the closure succeeds.
  Focused tests prove caller rollback leaves a zero-byte WAL, three rewrites of
  one page emit one page frame, recovery publishes the final value, and a
  phase-two flush failure rejects subsequent reads and writes with
  `HANDLE_POISONED`.
- Findings: staged commit preserves committed/recovery behavior while removing
  ordinary rollback tails and intermediate-frame amplification. The final frame
  set is now available for exact budget admission before append.
- Disposition: remain Incubating; staged commit prerequisite is executable.
  Numeric budget, retained versions, and snapshot ownership remain open.
- Resulting ADR or documentation change: SDD fatal-session wording now includes
  commit-path ambiguity; no physical-format change.

### Cycle 9 -- 2026-08-27

- Status entering review: Incubating
- New evidence: exact record-size tests confirm 25-byte boundary records and
  4,129-byte page writes. An ignored max-value overwrite fixture executes the
  current allocate-new/retire-old path and measures 33,274 final page frames,
  137,388,396 WAL bytes, and a successful assertion in 140 seconds.
- Findings: 128 MiB is too small for one valid overwrite. Commit staging permits
  exact pre-append checks, so transaction and retained-history ceilings can be
  hard rather than advisory watermarks.
- Disposition: remain Incubating; prefer initial private defaults of 160 MiB per
  transaction, 512 MiB retained committed WAL, and 64 snapshots. Public naming
  and configuration await the private contract and independent caller.
- Resulting ADR or documentation change: no behavior change; executable sizing
  evidence is retained as an ignored large test.

### Cycle 10 -- 2026-08-27

- Status entering review: Incubating
- New evidence: generation, epoch, commit staging, crash ordering, v3 refusal,
  reader ownership, checkpoint scope, and finite private defaults now have one
  coherent candidate backed by focused and large-pressure tests.
- Findings: the storage mechanism is sufficiently specific to authorize a
  private format-v3 implementation without stabilizing public session types.
- Disposition: Accepted through ADR-0005.
- Resulting ADR or documentation change: ADR-0005 is binding; this review keeps
  detailed evidence and remaining implementation/caller follow-up.

### Cycle 11 -- 2026-08-27

- Status entering review: Accepted
- New evidence: a private `CommittedWalIndex` now groups page frames by owning
  commit LSN above a supplied checkpoint horizon. Focused tests prove atomic
  generation selection across two pages and exclude checkpointed/incomplete
  transactions. The existing read-only WAL overlay is its first real caller.
- Findings: snapshot page selection is executable without changing format-v2
  open, checkpoint, or live-view behavior. Monotonic stream validation and a
  shared owner remain subsequent v3 gates.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: no public or physical-format change.

### Cycle 12 -- 2026-08-27

- Status entering review: Accepted
- New evidence: recovery and read-only database overlay now use a strict WAL
  stream reader that rejects duplicate or decreasing LSNs at the offending byte
  offset. Focused fixtures cover both order failures; physical inspection can
  still decode records without claiming database-valid ordering.
- Findings: CRC-valid records are insufficient for a database WAL when their
  generation order is ambiguous. Strict database consumption closes that
  prerequisite without changing record bytes or current valid behavior.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: no format change; malformed ordering
  now fails explicitly on database recovery/overlay paths.

### Cycle 13 -- 2026-08-27

- Status entering review: Accepted
- New evidence: writable pager construction now opens its WAL through a private
  database-seeded path using `wal_checkpoint_lsn + 1`. An empty sidecar adopts
  that lower bound; an existing higher next LSN never moves backward. The
  public raw physical writer retains its epoch-local behavior.
- Findings: page zero is now the executable sole epoch authority at the database
  boundary, although valid format-v2 files continue to seed 1 because their
  checkpoint field remains zero.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: no format or public API change.

### Cycle 14 -- 2026-08-27

- Status entering review: Accepted
- New evidence: a private process-local snapshot registry now issues one
  non-cloneable pin per admitted generation, reports the deterministic oldest
  pinned generation, removes registrations on drop, and enforces the accepted
  default ceiling of 64 registrations. Exhaustion emits structured
  `SNAPSHOT_LIMIT_REACHED` with `active` and `maximum` details.
- Findings: snapshot lifetime and finite admission can be made structural
  before a public session vocabulary exists. The registry remains staged until
  the shared database owner is its first production caller; it does not yet pin
  WAL bytes or change current read behavior.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: the Error Design Document now records
  the snapshot-exhaustion contract; no format or public snapshot API change.

### Cycle 15 -- 2026-08-27

- Status entering review: Accepted
- New evidence: pager commit now computes the exact encoded size of `Begin`,
  final unique page frames (including page zero when dirty), and `Commit` before
  appending. The private default is 160 MiB. A focused low-limit fixture proves
  rejection leaves a zero-byte WAL, restores transaction metadata, and leaves
  the handle reusable.
- Findings: the accepted transaction ceiling is executable over the staged
  representation and cannot be amplified by repeat writes to one page.
  `TRANSACTION_WAL_TOO_LARGE` reports `actual` and `maximum`; unchanged work
  cannot succeed by retrying, so its status is `InvalidInput`, not `Busy`.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: the Error Design Document records the
  pre-append rejection contract; retained-history admission remains subsequent.

### Cycle 16 -- 2026-08-27

- Status entering review: Accepted
- New evidence: ordinary pager open and bounded byte inspection now admit one
  explicit physical-format interval and reject versions on either side with
  direction-neutral `UnsupportedFormat` details. The stable external code
  remains `FORMAT_VERSION_UNSUPPORTED`.
- Findings: moving the admitted interval from exact v2 to exact v3 can now
  refuse v2 before recovery or mutation without a false "newer" diagnosis.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: AR-0006 is accepted through ADR-0005;
  format constants have not moved yet.

### Cycle 17 -- 2026-08-27

- Status entering review: Accepted
- New evidence: commit admission now checks current encoded WAL bytes plus the
  exact staged transaction size against the private 512 MiB retention ceiling
  using checked arithmetic before `Begin`. A focused low-limit fixture proves
  rejection leaves a zero-byte WAL, rolls back metadata, and keeps the handle
  reusable.
- Findings: retained-history capacity now has a distinct typed pressure signal:
  `WAL_RETENTION_LIMIT_REACHED` is `Busy` because releasing snapshots and a
  later checkpoint may make unchanged work admissible. Transaction oversize
  remains `InvalidInput` because retry alone cannot change its size.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: the Error Design Document now records
  the retained-pressure details; retained commit residence is still subsequent.

### Cycle 18 -- 2026-08-27

- Status entering review: Accepted
- New evidence: standalone B+ tree and provider `put`/`delete` now open a
  staged pager transaction when no explicit transaction is active. A focused
  fixture proves the write consumes WAL LSNs before format-2's immediate
  checkpoint leaves the sidecar empty and that reopen sees the value. Pager
  page-mutation and transaction primitives are crate-private.
- Findings: ordinary logical writes and explicit transaction closures now share
  one `Begin` / final unique frames / `Commit` / sync publication mechanism.
  Database consumers can no longer bypass it through public raw page mutation.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: MVP+10 Slice 2 publication-path item
  is complete; format-2 residence and visible behavior remain unchanged.

### Cycle 19 -- 2026-08-27

- Status entering review: Accepted
- New evidence: the admitted ordinary-open interval is now exactly physical
  format 3. Every commit includes an authenticated page-zero frame whose
  `wal_checkpoint_lsn` equals the transaction's actual `Commit` LSN. In the
  current zero-reader mode, data plus page zero sync before WAL truncation, and
  the live writer retains `commit_lsn + 1`; reopen seeds from page zero.
  Focused tests prove successive generations advance across reopen and crash
  recovery preserves the committed value and horizon.
- Findings: the sole durable epoch authority and zero-reader crash ordering are
  executable. Format 2 is refused in both pager and byte-inspection paths
  before recovery or mutation. Reader-pinned retained residence remains the
  next behavior change.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: current fixtures move deliberately to
  format 3; MVP+10's monotonic-generation item is complete.

### Cycle 20 -- 2026-08-27

- Status entering review: Accepted
- New evidence: the pager now owns the bounded process-local registry. Any
  active pin suppresses phase-two main-file writes and WAL truncation after a
  durable commit; a prebuilt latest-frame map becomes the writer's visible
  committed view. After the pin drops, the next zero-reader commit checkpoints
  every retained latest page plus page zero before truncation. A focused test
  proves the main file remains at the old horizon while WAL is retained, both
  the writer and an independent live-view handle observe the commit, and the
  later full checkpoint survives reopen.
- Findings: ADR-0005's initial all-or-nothing residence rule is executable
  inside the pager owner without a public session vocabulary. This does not yet
  prove that an older reader selects only frames at or below its generation;
  generation-selecting read ownership remains the next Slice 2 gate.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: retained residence and zero-reader
  reclamation are active private behavior; no public snapshot API is added.

### Cycle 21 -- 2026-08-27

- Status entering review: Accepted
- New evidence: the pager's latest-only retained map is now a committed-version
  index prepared before WAL sync. Immutable frame bodies are shared across
  index clones, and each generation remains selectable by owning commit LSN.
  A private pinned read validates ownership, resolves page zero at the captured
  generation, bounds page numbers by that version's page count, and decrypts
  only the newest data frame no newer than the pin. A focused test holds pins
  at generation 0 and the first commit while a second commit updates the page
  and allocates another; the pins continue to see 0 and 1 while the writer sees
  2, and both reject the newly allocated page.
- Findings: authenticated page selection and snapshot metadata bounds are now
  executable, including distinct simultaneous generations. This is not yet an
  independently callable B+ tree read transaction or a public session API.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: the private page-level no-newer-than
  visibility item is complete; shared logical-reader ownership remains next.

### Cycle 22 -- 2026-08-27

- Status entering review: Accepted
- New evidence: B+ tree point lookup is the first logical consumer of the
  pinned pager view. One private read-source contract supplies root, page count,
  and authenticated page access to a shared traversal and overflow decoder for
  both current and pinned reads. A focused fixture captures an overflow-backed
  value, replaces and frees its chain, performs 500 later commits that split
  the original root, and still resolves the old value and old root while
  excluding a later key. The writer sees the replacement throughout.
- Findings: stable snapshot meaning now composes across internal traversal,
  leaf lookup, overflow residence, page reuse writes, and root change without
  cloning the B+ tree algorithm. Range scans and a shared owner/session caller
  remain subsequent work.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: private logical point visibility is
  executable; no public read-transaction API is added.

### Cycle 23 -- 2026-08-27

- Status entering review: Accepted
- New evidence: the read-source-parameterized B+ tree pipeline now performs
  ordered range scans as well as point lookup. A focused multi-leaf fixture
  captures 200 rows, including an overflow-backed value, then commits deletes,
  overwrites, and later inserts before repeating both current and pinned scans.
  The current scan changes while the pinned scan exactly reproduces the
  captured ordered key/value sequence and historical leaf links.
- Findings: logical snapshot coherence now covers repeated traversal across a
  retained leaf chain and bounded overflow decoding without a parallel scan
  implementation. The remaining Slice 2 boundary is ownership/lifetime and
  pressure diagnostics rather than page or B+ tree visibility mechanics.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: private logical range visibility is
  executable; no public iterator, session, or read-transaction API is added.

### Cycle 24 -- 2026-08-27

- Status entering review: Accepted
- New evidence: a private shared B+ tree owner now serializes generation
  capture with commit publication. Its non-cloneable read transaction retains
  the process-local pin across separately locked point and range operations. A
  threaded fixture proves current reads advance while the pinned transaction
  remains stable, reports its retained-WAL pressure, and confirms that dropping
  the reader performs no hidden checkpoint before the next zero-reader commit.
- Findings: ADR-0005's owner-scoped pin lifetime, all-or-nothing checkpoint
  suppression, and passive release behavior compose at the logical ownership
  boundary. Diagnostic counts and byte totals are private evidence, not yet a
  stable inspection schema or public API.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: private Slice 2 ownership and pressure
  evidence is complete; AR-0009 continues to govern public API admission.

### Cycle 25 -- 2026-08-27

- Status entering review: Accepted
- New evidence: a read transaction retained after all explicit shared-owner
  handles drop continues to resolve its captured generation and keeps the
  database writer gate admitted. Dropping that last reader releases the pager;
  writable reopen then recovers the newer committed value from retained WAL.
- Findings: pin ownership, writer-gate lifetime, and recovery share the same
  last-owner boundary. The transaction is structurally `Send` but not `Sync`,
  matching the SDD rather than exposing the mutex's stronger accidental auto
  trait.
- Disposition: ADR-0005 remains accepted without revision.
- Resulting ADR or documentation change: private teardown and reopen evidence
  closes the admitted Slice 3 lifecycle case without adding a public API.
