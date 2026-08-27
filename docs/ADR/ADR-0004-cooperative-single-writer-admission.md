# ADR-0004: Cooperative Single-Writer Admission

## Status

Accepted

## Context

The pre-MVP+10 baseline shows that multiple independent writable Tosumu handles
can currently open the same database. Transaction exclusion, transaction IDs,
and WAL next-LSN state are handle-local, so two writers can race without one
shared admission point. Read-only handles can coexist and currently observe a
live post-commit view rather than an LSN-pinned snapshot.

MVP+10 must first prevent cooperating writers from entering the physical write
path together. This decision is intentionally narrower than MVCC: it does not
define reader snapshots, retained versions, checkpoint pinning, or waiting
policy beyond fail-fast admission.

Rust's standard cross-platform file-locking API is unavailable at Tosumu's Rust
1.75 MSRV. AR-0010 therefore reviewed `fs4` 1.1.0 for exact, native-only,
sync-only use.

## Decision

Normal writable Tosumu database paths use one persistent advisory writer-lock
sidecar named by appending `.writer.lock` to the database path. For
`example.tsm`, the sidecar is `example.tsm.writer.lock`.

- Creation and writable open acquire a non-blocking exclusive lock before
  creating, opening, recovering, checkpointing, or publishing writable state.
- A writable pager retains the locked file handle for its full lifetime.
- Page-zero protector add, remove, and rekey operations acquire the same gate
  for their complete read/validate/edit/write session.
- Public recovery and checkpoint operations acquire the gate. Internal callers
  that already hold it use crate-private guarded implementations rather than
  reacquiring a non-reentrant lock.
- Read-only database, inspection, and stable-backup source paths do not acquire
  the writer gate. This decision does not add a reader registry.
- Backup and portable-export artifacts do not copy the writer sidecar. Privately
  owned staging files may use guarded internal recovery/checkpoint paths.
- The sidecar file persists after unlock and must not be deleted during ordinary
  cleanup. Deleting and recreating its pathname while a handle is locked could
  split cooperating writers across different file identities.
- The lock is advisory. It coordinates Tosumu paths that participate in this
  protocol; it does not prevent arbitrary or older software from writing the
  database or WAL directly.

`WalWriter` remains a low-level physical API without a database-identity input.
Its direct mutation methods do not participate in writer admission and are
explicitly unsupported while any database handle or coordinated maintenance
operation may use the same database/WAL pair. This exception is documented,
not hidden. A future independent caller that needs coordinated raw WAL mutation
must supply evidence for a coordination token or a revised public boundary.

### Failure contract

Lock contention returns the existing `TosumuError::FileBusy` contract:

- stable code: `FILE_OPEN_BUSY`;
- status: `busy`;
- `path`: the `.writer.lock` sidecar path;
- `operation`: `"acquiring database writer gate"`.

Failure to create/open the sidecar or another non-contention locking failure
remains an external I/O failure with its source preserved. Admission never
waits, retries indefinitely, steals a lock, deletes a sidecar, or guesses that
an owner is stale.

### Dependency admission

`tosumu-core` may use exactly `fs4` 1.1.0 with default features disabled and
only `sync` enabled, declared only for `cfg(any(unix, windows))`:

```toml
[target.'cfg(any(unix, windows))'.dependencies]
fs4 = { version = "=1.1.0", default-features = false, features = ["sync"] }
```

No `fs4` type or error enters Tosumu's public vocabulary. The resolved archive
checksum, platform closures, unsafe boundary, build script, licenses, MSRV, and
WASM exclusion are retained in AR-0010. Non-Unix/Windows targets retain build
compatibility without this dependency and report writable file storage as
unsupported; they do not silently use a no-op lock.

## Consequences

- A second cooperating writer fails before database/WAL mutation begins.
- Readers remain able to coexist with one writer and retain their current live-
  view behavior until a later snapshot decision.
- Protector maintenance and normal recovery/checkpoint cannot bypass the gate.
- One persistent empty sidecar becomes an operational artifact, not part of the
  authenticated database or WAL format.
- Direct raw WAL mutation remains possible but explicitly outside the
  concurrency contract.
- This decision changes no authenticated page bytes, WAL record bytes, LSN
  meaning, durability ordering, or format version.

## Alternatives Considered

- **Lock the database file exclusively.** Rejected because it would also reject
  readers and would not preserve the intended multiple-reader direction.
- **Use a process-local registry.** Rejected because path aliases are fragile
  and another process would remain uncoordinated.
- **Use create/delete lockfiles as ownership.** Rejected because crashes leave
  ambiguous stale files and stealing requires unsafe liveness guesses.
- **Raise the MSRV to use `std::fs::File` locks.** Rejected because Rust 1.89 is
  not justified solely by this mechanism.
- **Gate raw `WalWriter` by reversing its WAL path.** Rejected because deriving
  database identity from an arbitrary physical path is not a sound contract.
- **Add snapshots in the same change.** Rejected because writer admission is
  independently useful and the baseline contains no reader-pinning machinery.

## Reopening Triggers

Revisit this decision if an independent caller requires coordinated raw WAL
mutation, advisory locks are unsupported on a supported filesystem, sidecar
identity fails under supported path/rename behavior, the project MSRV reaches a
standard-library locking implementation, busy policy adds bounded waiting, or
reader snapshots require a unified coordination artifact.

## References

- `ADR-0001-storage-engine-layer-boundaries.md`
- `ADR-0002-authenticated-pager-trust-boundary.md`
- `ADR-0003-source-unit-cohesion-size-pressure-and-decomposition.md`
- `../Architectural Reviews/AR-0009-multiple-reader-execution-and-coordination.md`
- `../Architectural Reviews/AR-0010-dependency-trust-and-source-provenance.md`
- `../Plans/mvp-10-multiple-readers.md`
- `../Specifications/Tosumu Error Design Document.md`
