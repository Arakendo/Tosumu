# AR-0004: Semantic Change History And Sync

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-03 |
| Last reviewed | 2026-08-03 |
| Scope | Storage core / semantic adapters / sync consumers |
| Trigger | Future sync design requires durable semantic changes that the physical WAL does not provide |
| Related ADRs | ADR-0001 |
| Related evidence | `docs/Specifications/Tosumu Software Design Document.md` offline-first and semantic-change sections; Tokimu consumer pressure |

## Architectural Question

Where should durable semantic change identity, history, watermarks, tombstones,
and conflict evidence live without turning the physical WAL or B+ tree into an
application replication protocol?

## Context

Tosumu's WAL records physical transaction and page-recovery mechanics. Future
offline sync and collaborative consumers need changes such as stable object or
row identity, actor identity, before/after hashes, watermarks, and conflict
evidence. Those are semantic records and cannot be reconstructed reliably from
arbitrary page writes.

The design discusses this direction, but no implemented semantic change log or
independent syncing consumer establishes the correct boundary.

## Evidence

- Tests or fuzzing: WAL and recovery tests prove physical durability only.
- Independent consumers: Tokimu anticipates live and portable asset modes, but
  does not yet synchronize Tosumu semantic changes.
- Diagnostics or audits: current inspect output exposes physical WAL records.
- Repeated implementation friction: none from a real sync cycle yet.
- Missing evidence: stable semantic IDs, two writers or replicas, conflict
  cases, tombstone retention, compaction, and watermark semantics.

## Ownership And Dependency Analysis

- Core owns atomic storage transactions and physical recovery evidence.
- A relational or consumer adapter owns what a row, asset, node, cue, or
  application mutation means.
- A future change-history capability may own durable ordering and generic
  provenance without understanding consumer payload semantics.
- Sync transport owns movement, not semantic conflict policy.
- WAL pages must not become public replication units.

## Alternatives Considered

### Alternative A: Replicate the physical WAL

- Benefits: reuses an existing durable log.
- Costs: couples replicas to page layout and recovery internals.
- Failure mode: physical writes are mistaken for semantic intent.

### Alternative B: Put every semantic change in core

- Benefits: one central history.
- Costs: core must understand arbitrary consumer schemas.
- Failure mode: application meaning leaks into storage mechanics.

### Alternative C: Incubate an explicit semantic change boundary

- Benefits: separates recovery from synchronization and preserves ownership.
- Costs: consumers must define their own change vocabulary initially.
- Failure mode: generic fields may be stabilized before conflict evidence exists.

## Findings

- Physical WAL and semantic change history are distinct.
- Tosumu may eventually provide durable generic change evidence, but consumers
  continue to own mutation meaning and conflict policy.
- No permanent sync or change-log contract is admitted yet.

## Disposition

Incubating. Preserve the distinction and wait for an end-to-end offline sync
consumer before selecting a durable contract.

## Required Follow-Up

- [ ] Implement one consumer-owned semantic change vocabulary.
- [ ] Exercise offline edits, replay, tombstones, and one real conflict.
- [ ] Compare portable checkpoint and live working-store requirements.
- [ ] Keep physical WAL records private to recovery.

## Reopening Triggers

- A consumer needs offline synchronization or collaborative editing.
- Two adapters independently create equivalent change IDs and watermarks.
- Physical WAL details begin leaking into a public sync surface.

## Review History

### Cycle 1 -- 2026-08-03

- Status entering review: Proposed
- New evidence: future design was separated from implemented WAL guarantees.
- Findings: the semantic/physical distinction is strong; the contract is not.
- Disposition: Incubating
- Resulting ADR or documentation change: none

