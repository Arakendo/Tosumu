# ADR-0005: Committed Generation And Retained-WAL Snapshots

## Status

Accepted

## Context

MVP+10 requires one writer to continue committing while multiple readers hold
stable snapshots. Format v2 cannot provide that contract: successful commits
copy pages into the main file and truncate the WAL, ordinary writes may bypass
WAL publication, WAL LSNs restart after truncation, and independent read-only
handles observe later main-file state.

ADR-0004 supplies cooperative single-writer admission. AR-0011 evaluated where
committed versions live, what an LSN means, how checkpoint/recovery ordering
survives crashes, which bounds are enforceable, and whether the first snapshot
format is compatible with v2.

## Decision

Tosumu's first snapshot-capable storage format is format v3. It uses the main
file at one checkpoint generation plus complete committed versions retained in
the persistent WAL.

### Generation and publication

- Generation 0 is a newly created checkpointed database.
- The LSN of a matching durable `Commit` record is the atomic generation of all
  page writes in that transaction.
- Page-write LSNs remain physical record identities; page versions remain AEAD
  inputs. Neither independently publishes logical state.
- Database generations never decrease or restart after checkpoint, WAL
  truncation, close, recovery, or reopen.
- Every logical mutation publishes through the common WAL transaction path.
  Direct main-file auto-commit is not a valid v3 publication mechanism.
- Transactions stage final unique dirty-page frames in memory. Caller rollback
  appends no WAL bytes. Commit computes and admits the complete encoded size,
  then appends `Begin`, sorted final `PageWrite` records, optional page zero,
  and `Commit` before syncing once.
- An append or sync failure during commit poisons the handle. Reopen validates
  the stream and trims only a structurally incomplete physical tail; the engine
  does not discard a complete commit merely because sync returned an error.

### Epoch and retained versions

- Authenticated page-zero `wal_checkpoint_lsn` is the sole durable epoch/base
  authority. Tosumu does not duplicate it in a WAL header solely to seed LSNs.
- The main file represents exactly `wal_checkpoint_lsn`.
- An empty WAL starts at `wal_checkpoint_lsn + 1`. A nonempty WAL must contain a
  structurally valid, monotonically increasing post-horizon stream.
- For snapshot N, a page resolves to its newest WAL frame whose owning commit
  generation is `<= N`, falling back to the checkpointed main-file frame.
- Uncommitted frames never enter the committed index or become reader-visible.

### Ownership and reader scope

- The first implementation uses one shared database owner that retains the
  ADR-0004 writer guard and owns a process-local snapshot registry.
- A read transaction captures the latest durable committed generation and pins
  it until drop. Drop only unregisters the pin; it performs no fallible hidden
  checkpoint work.
- Independent `open_readonly` handles remain ungated live-view handles. This
  decision does not claim cross-process snapshots or pinning.
- Public database/session/read-transaction type names remain provisional until
  the private storage contract has an independent caller.

### Checkpoint and crash ordering

The initial snapshot slice does not checkpoint while any registered snapshot
exists. With zero registered readers, a full checkpoint may:

1. apply the latest committed frame for each changed data page;
2. write page zero with the target `wal_checkpoint_lsn` and its applicable
   protector authentication;
3. sync the main file;
4. truncate and sync the existing persistent WAL sidecar.

Before the main-file sync, WAL is the recovery authority. After it, page zero
and main represent the target generation; surviving WAL bytes at or below that
horizon are obsolete and may be ignored or replayed idempotently. The sidecar
is truncated, not deleted, so correctness does not depend on directory-entry
replacement. Prefix checkpointing and passive/waiting checkpoint modes require
later evidence.

### Bounds and diagnostics

The first private implementation uses these experimental defaults:

- 160 MiB maximum final encoded WAL bytes per transaction;
- 512 MiB maximum retained committed WAL bytes; and
- 64 registered process-local snapshots.

Commit admission checks transaction and resulting retained sizes with checked
arithmetic before appending `Begin`. Snapshot admission checks the registry
limit before registration. Limit rejection is typed, fail-fast, and leaves WAL
and registry state unchanged. Diagnostics expose retained committed bytes, the
oldest pin, and registration pressure.

These values are engine defaults, not format constants or durability claims.
Public configuration remains provisional. The transaction default is grounded
in executable evidence: replacing one valid 64 MiB value currently produces
33,274 final frames and 137,388,396 encoded WAL bytes, so 128 MiB is invalid.

### Compatibility and raw WAL

- Ordinary v3 open accepts only its explicit supported format interval and
  refuses v2 with `FORMAT_VERSION_UNSUPPORTED` before recovery or mutation.
- No automatic, open-time, or in-place migration is permitted.
- A future v2 preservation tool, if justified by non-regenerable data, performs
  an offline logical rewrite into a distinct verified v3 destination and does
  not reinterpret v2 WAL records as retained v3 history.
- Raw `WalWriter` is not a coordinated database mutation path in v3. It may
  remain crate-private or behind an explicitly physical fixture boundary.

## Consequences

- One cooperating writer can publish while process-local readers retain stable
  generations without copying the whole database.
- The format and recovery protocol change incompatibly even though existing
  authenticated page-frame bytes can be reused.
- Long-lived snapshots can reject later writes at a finite retained-history
  boundary; the engine reports that pressure rather than growing silently.
- The initial checkpoint policy favors a small auditable mechanism over prefix
  reclamation concurrency.
- Existing format-v2 behavior remains unchanged until v3 implementation lands;
  this ADR does not retroactively claim snapshot isolation for current handles.
- This decision does not add cross-process reader coordination, external
  freshness/anti-rollback guarantees, background checkpointing, or a stable
  public session API.

## Alternatives Considered

- **Copy the database for each reader.** Rejected because capture and storage
  are O(database size) and do not match the intended embedded read path.
- **Keep before-images only in process memory.** Rejected because preservation
  can be bypassed, pressure becomes invisible, and reopen/recovery diverges from
  the WAL model.
- **Block writers with reader file locks.** Rejected because it does not satisfy
  writer-can-commit-while-readers-live.
- **Add a duplicate WAL epoch header.** Rejected because page zero already owns
  the checkpoint horizon and duplicate authorities create mismatch states.
- **Treat v3 as compatible with v2.** Rejected because v2 writers can reset LSNs,
  bypass publication, and destroy retained history.

## Reopening Triggers

Revisit this decision if an independent consumer requires cross-process pinned
snapshots, measured workloads cannot operate within the private defaults,
partial checkpointing becomes necessary, crash fixtures falsify the admitted
ordering, or non-regenerable v2 data justifies offline rewrite tooling.

## References

- `ADR-0001-storage-engine-layer-boundaries.md`
- `ADR-0002-authenticated-pager-trust-boundary.md`
- `ADR-0003-source-unit-cohesion-size-pressure-and-decomposition.md`
- `ADR-0004-cooperative-single-writer-admission.md`
- `../Architectural Reviews/AR-0006-format-evolution-and-migration-boundary.md`
- `../Architectural Reviews/AR-0009-multiple-reader-execution-and-coordination.md`
- `../Architectural Reviews/AR-0011-committed-generation-and-version-residence.md`
- `../Plans/mvp-10-multiple-readers.md`
- `../Specifications/Tosumu Software Design Document.md`
- `../Specifications/Tosumu Error Design Document.md`
