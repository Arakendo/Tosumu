# ADR-0008: SQL-Owned Secondary Index Keyspaces

## Status

Accepted

## Context

MVP+10 requires plain single-column secondary indexes. Tosumu's physical format
persists one B+ tree root, while `tosumu-sql` already owns catalog, row, and
query semantics as reserved keys in that tree. Adding independent roots would
require a durable root directory and a new atomic root-publication protocol.

ADR-0006 already provides the necessary provider-neutral mechanisms: atomic
multi-key writes and generation-pinned ordered reads.

## Decision

The first secondary indexes are SQL-owned ordered logical keyspaces within the
existing authenticated KV B+ tree. `tosumu-core` remains unaware of tables,
columns, indexes, and uniqueness.

An index entry key is a versioned, prefix-free encoding of index identity,
typed secondary value, and typed primary key. Its value is empty. Including the
primary key permits duplicate secondary values without a posting-list format.
Index definitions use separate versioned catalog records; existing version-1
table catalog payloads are unchanged.

The first supported form is a named, non-unique, single-column index used for
equality lookup. Composite, unique, covering, partial, expression, full-text,
fuzzy, vector, spatial, and foreign-key-specific indexes are outside this
decision.

Creating an index over existing rows publishes its catalog record and complete
backfill in one write transaction. INSERT replacement and DELETE update the
primary row and all affected index entries in one write transaction. Indexed
queries scan matching entries and fetch primary rows from one read snapshot.
No supported operation may expose a published partial index.

`SqlDatabase` consumes the supported shared KV owner. Any additional write-
transaction range operation remains provider-neutral and returns ordered byte
pairs; it does not expose B+ tree or SQL types.

This decision changes the SQL logical key schema only. It changes no page,
record, WAL, page-zero, or encryption format and requires no database migration.
Existing databases gain index records only after explicit SQL DDL.

## Consequences

- Existing atomic commit, recovery, authentication, and snapshots cover index
  maintenance without a second root protocol.
- Primary and secondary records share B+ tree pages, height, and write
  amplification.
- The tuple codec is compatibility-sensitive SQL data and requires collision,
  prefix-boundary, ordering, malformed-input, and reopen tests.
- Independent physical trees may be introduced later as a semantics-preserving
  storage optimization through a separate format decision.

## Alternatives Considered

- Add independent physical B+ tree roots now: rejected because it expands the
  physical format and recovery protocol without evidence that physical
  isolation is required for the first SQL lookup feature.
- Store primary-key lists as index values: rejected because every duplicate
  update rewrites a shared value and complicates atomic incremental mutation.
- Defer all secondary indexes: rejected because current shared KV mechanisms
  are sufficient for a bounded implementation.

## Reopening Triggers

Revisit if measured workloads show unacceptable shared-tree amplification or
locality, independent index reclamation is required, or another admitted core
feature establishes a durable multi-root directory.

## References

- `ADR-0001-storage-engine-layer-boundaries.md`
- `ADR-0006-shared-kv-store-and-snapshot-transactions.md`
- `../Architectural Reviews/AR-0013-secondary-index-representation-and-ownership.md`
- `../Plans/mvp-10-secondary-indexes.md`
- `../Specifications/Tosumu Software Design Document.md`
