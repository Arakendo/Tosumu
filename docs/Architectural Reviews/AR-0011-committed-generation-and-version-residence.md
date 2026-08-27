# AR-0011: Committed Generation And Version Residence

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-27 |
| Last reviewed | 2026-08-27 |
| Scope | Core storage / WAL / snapshot visibility / format compatibility |
| Trigger | AR-0009 Slice 2 requires an executable committed-LSN and retained-version contract before snapshot code begins |
| Related ADRs | ADR-0001, ADR-0002, ADR-0004 |
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
| During full WAL truncate | Main file still durably represents the latest commit | Accept intact obsolete prefixes and a torn obsolete tail; do not infer a newer commit without a complete record |
| After full WAL truncate | Main file is authoritative at T | Seed the next record LSN above T rather than restarting at 1 |

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
- No evidence yet selects the WAL byte-limit policy, public shared-owner API, or
  v2 offline rewrite behavior.

## Disposition

Incubating with Alternative C preferred. The candidate is specific enough for
focused format, recovery, limit, and independent-caller evidence, but not yet
accepted as an ADR or authorized for semantic implementation.

## Required Follow-Up

- [ ] Falsify or confirm the candidate with a crash-ordering matrix for commit,
      main checkpoint, page-zero horizon, and WAL reclamation.
- [ ] Decide whether WAL epoch/base metadata is stored in a versioned WAL header
      or supplied only through the authenticated database header.
- [ ] Decide the fate of public raw `WalWriter` database mutation before a
      monotonic-LSN implementation.
- [ ] Select and diagnose finite active-reader and retained-WAL limits using
      representative transaction sizes, including the 64 MiB provider case.
- [ ] Define format-v3 open/refusal and optional v2 offline logical rewrite
      behavior under AR-0006.
- [ ] Build the storage contract behind a private boundary and exercise it
      through the provider plus one independent caller before stabilizing shared
      database/session types.
- [ ] Promote accepted generation, publication, retention, and compatibility
      rules to an ADR before changing semantic storage behavior.

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
