# AR-0006: Format Evolution And Migration Boundary

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-03 |
| Last reviewed | 2026-08-27 |
| Scope | On-disk format / compatibility / migration tooling |
| Trigger | The format is implemented and versioned but deliberately pre-stability, while future migration mechanisms remain speculative |
| Related ADRs | ADR-0001, ADR-0002 |
| Related evidence | `docs/Specifications/Tosumu Software Design Document.md` section 13, `docs/file-format.md`, format fixtures and version errors, AR-0009 snapshot admission findings |

## Architectural Question

When Tosumu first changes its on-disk baseline incompatibly, should it take a
clean pre-stability break or admit explicit migration tooling and compatibility
policy, and which layer owns that work?

## Context

The current format is real, documented, versioned, and exercised by tests. It
is not frozen. Tosumu currently refuses incompatible versions explicitly and
does not run automatic migrations during `open()`. The design contains possible
migration categories, receipts, history, and APIs, but marks them deferred
until a concrete incompatible change supplies evidence.

## Evidence

- Tests or fuzzing: current format and version rejection are tested.
- Independent consumers: Tokimu uses the current provider baseline but has not
  required migration of durable user data.
- Diagnostics or audits: format/version failures are structured and inspectable.
- Repeated implementation friction: none from a real incompatible release.
- Missing evidence: an actual old/new format pair, user data requiring
  preservation, migration duration, crash recovery, and compatibility horizon.

## Ownership And Dependency Analysis

- Core owns physical format recognition and compatibility refusal.
- Explicit physical migration tooling may depend on core format primitives.
- Schema migrations belong to semantic/query layers, not the pager.
- `open()` must not silently perform destructive or expensive migration.
- Consumers own the value of preserving their datasets; they do not own page
  rewrite mechanics.

## Alternatives Considered

### Alternative A: Freeze the current format

- Benefits: immediate compatibility promise.
- Costs: stabilizes primitives before sufficient evidence.
- Failure mode: permanent baggage from an experimental baseline.

### Alternative B: Build a general migration framework now

- Benefits: appears prepared for future changes.
- Costs: speculative APIs and crash semantics.
- Failure mode: framework shape does not fit the first real migration.

### Alternative C: One baseline with explicit refusal until concrete pressure

- Benefits: honest pre-stability and minimal compatibility machinery.
- Costs: an early incompatible change may require a clean break.
- Failure mode: valuable user data appears before migration policy is ready.

## Findings

- The current format is implemented, not stable.
- Incompatible versions must fail explicitly; automatic migration on open is
  not accepted.
- Migration architecture remains deferred until a concrete format delta and
  preservation requirement exist.
- AR-0009 now supplies the first concrete pressure: committed-LSN snapshots
  need durable generation meaning, retained WAL history, and checkpoint rules
  that format-v2 writers do not understand.
- Reusing the existing `wal_checkpoint_lsn` bytes does not make the change
  behaviorally compatible. A v2 writer can reset WAL LSNs, overwrite the main
  file, and truncate history that a snapshot-capable engine would retain.
- The SDD's former assertion that Stage 6 needed no new storage format confused
  reusable `PageWrite` frame bytes with an implemented retention protocol. The
  normative text now defers the actual compatibility decision to AR-0006 and
  AR-0009.
- The accepted Tokimu provider fixture records physical format 2 and explicitly
  does not stabilize that pre-release format permanently. It requires explicit
  unsupported-version reporting, not automatic migration on open.

## Disposition

Incubating. Keep one current baseline and explicit incompatibility errors. Use
the first real incompatible change to decide clean break versus migration.

## Required Follow-Up

- [ ] Preserve representative format fixtures and exact version diagnostics.
- [ ] Record the first incompatible format delta and affected real datasets.
- [ ] Decide compatibility horizon before publishing a stable format promise.
- [ ] Open an ADR before admitting migration or long-term compatibility policy.
- [ ] Describe the exact v2-to-snapshot-format delta, including WAL generation,
      page-zero checkpoint meaning, old-writer exclusion, and crash ordering.
- [ ] Decide whether the first snapshot format is a clean pre-stability break or
      gains an explicit offline logical rewrite from readable v2 databases.
- [ ] Update the Tokimu provider fixture deliberately if its physical-format
      evidence moves beyond version 2; do not reinterpret its schema version.

## Reopening Triggers

- A physical primitive must change incompatibly.
- A released consumer has durable data that must survive an upgrade.
- The project declares an on-disk stability milestone.

## Review History

### Cycle 1 -- 2026-08-03

- Status entering review: Proposed
- New evidence: current format docs were separated from deferred migration prose.
- Findings: refusal policy is current; migration mechanics are not.
- Disposition: Incubating
- Resulting ADR or documentation change: none

### Cycle 2 -- 2026-08-27

- Status entering review: Incubating
- New evidence: AR-0009 Cycle 5 shows that format-v2 WAL LSNs reset, the
  checkpoint field remains zero, old frames are discarded, and direct writes
  bypass a commit generation. The SDD's unsupported no-format-change assertion
  was corrected. The accepted Tokimu fixture names physical format 2 without
  claiming permanent stability.
- Findings: snapshot publication is the first concrete incompatible format
  pressure. Activating existing header bytes is insufficient because older
  writers would violate the new retained-history protocol.
- Disposition: remain Incubating until the exact snapshot representation is
  specified. Preserve explicit refusal and no automatic migration on open;
  evaluate a clean v3 break against an explicit offline logical rewrite.
- Resulting ADR or documentation change: none.
