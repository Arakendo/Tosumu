# AR-0013: Secondary Index Representation And Ownership

| Field | Value |
| --- | --- |
| Status | Accepted |
| Opened | 2026-09-02 |
| Last reviewed | 2026-09-02 |
| Scope | Relational layer / logical key encoding / atomic mutation |
| Trigger | MVP+10 next requires single-column secondary indexes |
| Related ADRs | ADR-0001, ADR-0006, ADR-0008 |
| Related evidence | SDD §§12 and 18, SQL row/catalog codecs, shared KV transaction and snapshot tests |

## Architectural Question

Should the first secondary indexes require independently rooted physical B+
trees in `tosumu-core`, or should `tosumu-sql` represent them as ordered,
reserved logical keyspaces in the existing authenticated KV tree?

## Context

The SDD described secondary indexes as additional B+ trees mapping
`(secondary_key, primary_key)`. The current physical format persists exactly
one root page. Root splits update that one page-zero field, and neither core nor
the SQL catalog has a transactionally maintained catalog of physical roots.

The SQL layer already stores catalog and row records in reserved logical
keyspaces within that tree. ADR-0006 now supplies generation-pinned ordered
reads and atomic multi-key writes, which are the mechanisms needed to maintain
a secondary mapping without teaching core about tables, columns, or indexes.

## Evidence

- Tests or fuzzing: shared KV tests prove atomic multi-key commit, rollback,
  recovery, and generation-pinned range reads over the existing tree.
- Independent consumers: `tosumu-sql` owns table definitions, row encoding,
  predicate planning, and the primary-row keyspace.
- Diagnostics or audits: page zero exposes one root and no physical-tree root
  directory or independent-root recovery protocol.
- Repeated implementation friction: a second physical root would require root
  allocation, root-split publication, catalog recovery, inspection, and format
  migration before it can answer one SQL equality lookup.
- Missing evidence: no admitted caller requires independent-tree compaction,
  per-index page statistics, covering data, uniqueness, or range planning.

## Ownership And Dependency Analysis

`tosumu-sql` owns index definitions, typed tuple encoding, maintenance rules,
and planner selection. `tosumu-core` owns only ordered byte keys, snapshot range
reads, and atomic byte mutations. Core must not learn SQL table, column, index,
or uniqueness semantics.

The SQL database handle should consume `SharedKvStore` so one statement can
read and mutate through the supported snapshot/write boundary. Index catalog
records remain separate from version-1 table records, avoiding reinterpretation
of existing table payloads.

## Alternatives Considered

### Alternative A: Add independently rooted physical B+ trees now

- Benefits: physical isolation and a literal implementation of the earlier SDD
  shorthand.
- Costs: adds a durable root directory and atomic root-split publication to the
  storage format before any caller demonstrates that physical isolation matters.
- Failure mode: a committed index root and catalog entry can disagree after a
  crash unless a new multi-root recovery contract is designed and verified.

### Alternative B: Use reserved ordered SQL keyspaces

- Benefits: reuses proven atomic commit, recovery, authentication, and snapshot
  range traversal; preserves core/SQL ownership boundaries; changes no physical
  format byte.
- Costs: primary rows and indexes share tree height, page locality, and write
  amplification.
- Failure mode: malformed or ambiguous tuple encoding can cause collisions or
  incorrect range bounds, so the codec needs focused property and boundary tests.

### Alternative C: Defer indexes until multi-root storage exists

- Benefits: avoids choosing a logical representation now.
- Costs: blocks an admitted SQL feature on an unproven physical optimization.
- Failure mode: storage structure, rather than caller semantics, dictates the
  roadmap indefinitely.

## Findings

The logical-keyspace representation is the smallest architecture supported by
current evidence. One index entry key contains a versioned, prefix-free tuple
of index identity, typed secondary value, and typed primary key; the value is
empty. Repeated secondary values therefore occupy distinct keys. The first
slice supports plain, non-unique, single-column indexes and equality lookup.

Index definition publication and backfill must be one atomic write transaction.
INSERT replacement and DELETE must read the transaction-local old row and
update every affected index entry in the same transaction as the primary row.
Queries must resolve index entries and primary rows from one read snapshot.

This is an SQL logical-format addition, not a Tosumu page/record/WAL format
change. Independent physical trees remain a possible later optimization and
must not alter SQL results or the logical index contract.

## Disposition

Accepted through ADR-0008. Revise the SDD's “additional B+ trees” shorthand to
describe ordered logical secondary keyspaces for the initial implementation.

## Required Follow-Up

- [x] Record representation and ownership in ADR-0008.
- [ ] Implement and property-test the versioned index/catalog codecs.
- [ ] Move `SqlDatabase` onto the shared KV owner and add transactional range
      reads needed for atomic backfill.
- [ ] Implement DDL, maintenance, equality planning, snapshot execution, and
      recovery evidence.
- [x] Require no physical-format migration for the first index representation.

## Reopening Triggers

Reopen if benchmarks show unacceptable shared-tree amplification or locality,
if independent index lifecycle requires physical reclamation, or if a durable
multi-root directory is admitted for another core storage requirement.

## Review History

### Cycle 1 -- 2026-09-02

- Status entering review: Proposed
- New evidence: the supported shared KV surface now provides atomic multi-key
  writes and snapshot range reads, while the physical format still owns one
  root and no root-directory recovery protocol.
- Findings: SQL-owned ordered keyspaces satisfy the first index semantics with
  no core semantic leak or physical-format change.
- Disposition: Accepted through ADR-0008.
- Resulting ADR or documentation change: replace the physical-tree shorthand
  with a representation-neutral secondary-index contract and record the first
  logical representation.
