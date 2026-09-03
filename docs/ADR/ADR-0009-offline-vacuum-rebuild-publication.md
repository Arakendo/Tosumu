# ADR-0009: Offline VACUUM Rebuild Publication

## Status

Accepted

## Context

Local page compaction and freelist reuse do not shrink a Tosumu database.
In-place global page relocation would enlarge the WAL and crash-recovery
protocol. Rebuilding through an ordinary new database would lose durable
generation and protector continuity unless core explicitly preserves them.

## Decision

`VACUUM` is an offline, path-based core operation implemented as a verified
same-directory sibling rebuild followed by atomic source replacement.

The operation acquires the source's persistent writer sidecar before opening or
checkpointing the source and retains that exact guard through replacement and
directory durability. It is not callable on a live `SharedKvStore`. Read-only
handles remain ungated and may retain the old open file; new opens after
publication observe the rebuilt file.

Every live logical key/value pair is copied without consumer interpretation.
The rebuild preserves the current physical format, encryption mode, active DEK,
protector slots, and monotonic committed-generation history. Rewritten pages use
fresh random nonces and fresh authentication tags. `VACUUM` does not rotate the
DEK and does not alter SQL catalogs or secondary-index meaning.

Before publication, core requires an empty/checkpointed source WAL, a complete
staged logical scan, and successful structured verification. Publication must
use a same-directory platform primitive with an atomic replacement guarantee.
Unsupported platforms refuse before mutating the source. The writer lock
sidecar persists and is never copied or replaced.

Cancellation and ordinary failure are supported only before publication and
leave the source authoritative. The publication point has an old-or-new crash
contract. A failure after atomic replacement that cannot prove directory
durability is reported distinctly and never triggers automatic rollback over
the new source.

The result reports source path, logical record count, bytes before and after,
pages before and after, and whether publication is durably confirmed. These are
observations, not performance guarantees.

## Consequences

- Global reclamation reuses ordinary B+ tree construction rather than adding a
  page-relocation recovery protocol.
- Temporary free space approximately equal to the rebuilt database is required.
- Core needs private guarded-open and rebuild constructors; they must not become
  general writer-gate bypass APIs.
- Encrypted vacuum preserves protector usability but is not key rotation.
- Atomic replacement and parent-directory synchronization require explicit
  native-platform evidence before implementation is admitted.

## Alternatives Considered

- In-place relocation and truncation: rejected because partially moved parent,
  freelist, and file-length state creates a substantially larger recovery model.
- `VACUUM INTO` only: retained as a possible additive export feature, but it
  does not satisfy source reclamation and cannot delegate safe replacement.
- Unencrypted-only vacuum: rejected because silently narrowing maintenance by
  security mode would make encrypted databases second-class; unsupported crypto
  preservation must block the whole initial feature instead.

## Reopening Triggers

Revisit for online or incremental vacuum, post-publication cancellation, DEK
rotation, physical-format migration, or platforms without an admissible atomic
replace and directory-durability mechanism.

## References

- `ADR-0002-authenticated-pager-trust-boundary.md`
- `ADR-0004-cooperative-single-writer-admission.md`
- `ADR-0005-committed-generation-and-retained-wal-snapshots.md`
- `../Architectural Reviews/AR-0014-vacuum-rebuild-and-publication.md`
- `../Plans/mvp-10-vacuum.md`
