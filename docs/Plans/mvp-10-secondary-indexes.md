# MVP+10 Secondary Indexes

## Purpose

Deliver the first SQL-owned, non-unique, single-column secondary index without
changing Tosumu's physical storage format or weakening snapshot and recovery
claims.

ADR-0008 owns representation and subsystem boundaries. This plan sequences the
implementation; it is not architectural authority.

## Scope

- `CREATE INDEX <name> ON <table> ( <column> )`.
- Equality lookup on one indexed non-primary-key column.
- Duplicate secondary values represented by distinct primary-key-suffixed
  entries.
- Atomic create/backfill, insert replacement, and delete maintenance.
- Snapshot-consistent index entry and primary-row reads.

Unique, composite, covering, partial, expression, range-planning, foreign-key,
full-text, fuzzy, vector, and spatial indexes are deferred.

## Ordered Slices

### 1. Codec And Catalog Foundation

- [ ] Add a versioned binary index-entry codec with unambiguous tuple framing.
- [ ] Preserve type identity for INTEGER, TEXT, and BLOB values.
- [ ] Add prefix-bound helpers for exact-secondary-value scans.
- [ ] Add a separate versioned index-definition catalog record.
- [ ] Prove collisions, embedded zero bytes, prefix values, signed integer
      boundaries, malformed records, and deterministic ordering.

### 2. Shared Transaction Integration

- [ ] Add a provider-neutral ordered range read to `KvWriteTransaction` so
      backfill observes the same staged transaction it publishes.
- [ ] Move `SqlDatabase` from direct `PageStore` ownership to `SharedKvStore`.
- [ ] Retain existing SQL behavior and error mapping through focused regression
      tests before adding DDL semantics.

### 3. DDL And Atomic Backfill

- [ ] Parse and semantically validate `CREATE INDEX`.
- [ ] Reject duplicate index names, missing tables, missing columns, and primary-
      key targets with typed SQL diagnostics.
- [ ] Scan existing table rows and publish the definition plus all entries in
      one transaction.
- [ ] Prove a failed backfill publishes neither catalog nor entries.

### 4. Mutation Maintenance

- [ ] On INSERT, read any old row in the write transaction, remove stale entries,
      add new entries, and replace the row atomically.
- [ ] On DELETE, remove index entries and the primary row atomically.
- [ ] Cover repeated secondary values and unchanged indexed values.

### 5. Planning And Snapshot Execution

- [ ] Select an equality index only when the predicate and catalog match.
- [ ] Scan the exact secondary-value key interval and fetch primary rows through
      the same `KvReadTransaction`.
- [ ] Expose the selected index name in explain output.
- [ ] Preserve explicit unsupported diagnostics for unadmitted query shapes.

### 6. Recovery And Closure

- [ ] Reopen after committed DDL and mutations and verify catalog, entries, and
      query results.
- [ ] Add injected failure evidence around backfill and row/index maintenance.
- [ ] Run workspace formatting, strict clippy, all-target tests, and strict docs.
- [ ] Update ADR/AR review history and roadmap status with measured results.

## Acceptance

- Row and secondary-index mutations are one committed generation.
- An active query snapshot never combines index entries and rows from different
  generations.
- Recovery exposes either the complete old state or complete new state, never a
  partially published index.
- The physical page, record, WAL, and encryption formats remain unchanged.
