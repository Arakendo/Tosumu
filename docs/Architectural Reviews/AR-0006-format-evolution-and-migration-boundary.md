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
- Current header validation rejects only versions greater than the engine's
  `FORMAT_VERSION`; it does not consult `min_reader_version` or reject an older
  incompatible floor. That is sufficient only while supported formats share
  one interpretation. A v3-only binary cannot reuse `NewerFormat` semantics to
  refuse v2 honestly.

## Preferred First Incompatible Change

Format v3 is a clean pre-stability break for snapshot-capable databases. The
physical page-frame and keyslot layouts may remain byte-identical, but these
behavioral fields and rules change incompatibly:

- page-zero `wal_checkpoint_lsn` becomes the durable main-file generation and
  the lower bound for post-checkpoint WAL record LSNs;
- every logical mutation publishes through a structurally framed WAL
  transaction; direct main-file auto-commit is no longer a valid publication
  path;
- commit-record LSN is the atomic visible generation, while page-write LSNs
  remain physical record identities;
- committed versions newer than the checkpoint may remain in the persistent
  WAL and cannot be truncated by a writer unaware of reader pins; and
- the database-owned WAL opener validates monotonic post-horizon LSNs and seeds
  an empty sidecar from `wal_checkpoint_lsn + 1`.

Ordinary v3 open accepts only the explicitly supported v3 interval. A v2 file
returns `FORMAT_VERSION_UNSUPPORTED` with found, supported-minimum, and
supported-maximum details before WAL recovery or mutation. The implementation
must replace or generalize the direction-specific `NewerFormat` variant; it
must not describe an older v2 file as "newer." A v2 binary already rejects v3
as newer, excluding cooperating old writers from the new protocol.

No automatic, in-place, or open-time migration is admitted. If preservation
pressure later justifies tooling, the candidate is an explicit offline logical
rewrite:

1. open the v2 source read-only with a deliberately retained legacy reader;
2. create a distinct new v3 destination and protector state;
3. scan logical key/value records into bounded v3 transactions;
4. checkpoint, reopen, and verify the destination; and
5. leave source replacement or archival as a separate operator action.

The rewrite must never reinterpret v2 WAL records as v3 retained history, must
not overwrite the source, and must emit a receipt before it can be called a
migration. No such tool is required for MVP+10: current Tokimu evidence is a
regenerable pre-release fixture and has not established durable user datasets
that need preservation.

## Disposition

Incubating. Keep one current baseline and explicit incompatibility errors. Use
the first real incompatible change to decide clean break versus migration.

## Required Follow-Up

- [ ] Preserve representative format fixtures and exact version diagnostics.
- [x] Record the first incompatible format delta and affected real datasets.
- [ ] Decide compatibility horizon before publishing a stable format promise.
- [ ] Open an ADR before admitting migration or long-term compatibility policy.
- [x] Describe the exact v2-to-snapshot-format delta, including WAL generation,
      page-zero checkpoint meaning, old-writer exclusion, and crash ordering.
- [x] Prefer a clean v3 pre-stability break; defer an explicit offline logical
      rewrite until non-regenerable v2 data supplies preservation pressure.
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

### Cycle 3 -- 2026-08-27

- Status entering review: Incubating
- New evidence: AR-0011 now defines the v3 generation, retained-WAL, epoch, and
  crash-ordering candidate. Current validation only rejects versions above the
  engine maximum, while Tokimu's checked-in format-2 fixture is explicitly
  regenerable and pre-stability.
- Findings: v3 must be an exact supported interval rather than silently opening
  v2 with v3 semantics. No real dataset currently pays for migration machinery.
  If that changes, a separate-destination logical rewrite is safer and more
  inspectable than in-place physical conversion.
- Disposition: remain Incubating; prefer a clean v3 break and explicit ordinary
  open refusal. Defer offline rewrite implementation until preservation demand.
- Resulting ADR or documentation change: none until AR-0011 is promoted and the
  format change is authorized.
