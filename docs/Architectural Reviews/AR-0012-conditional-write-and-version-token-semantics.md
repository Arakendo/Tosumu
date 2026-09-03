# AR-0012: Conditional Write And Version Token Semantics

| Field | Value |
| --- | --- |
| Status | Accepted |
| Opened | 2026-09-02 |
| Last reviewed | 2026-09-02 |
| Scope | Core storage / public KV contract / optimistic concurrency |
| Trigger | MVP+10 next requires version-observing reads and atomic conditional writes |
| Related ADRs | ADR-0005, ADR-0006, ADR-0007 |
| Related evidence | SDD §§12 and 20.2.5, Error Design outcomes rule, shared KV caller tests |

## Architectural Question

What does a KV version token identify, which conditional mutations are in the
first supported surface, and is an unmet precondition an error or a result?

## Context

ADR-0005 gives the database a durable monotonic committed generation, and
ADR-0006 serializes current reads and writes through one shared owner. Tosumu
does not store a durable per-key revision. Page versions cannot substitute:
one logical mutation may rewrite several pages, and a page rewrite may move or
rewrite keys that did not logically change.

The current public `get` followed by `put` is intentionally not atomic across
calls. `KvConnectionInfo::latest_generation` also cannot be paired safely with
a separate `get`, because another clone may commit between those observations.

## Evidence

- Tests or fuzzing: shared-owner tests prove snapshot capture and commit
  publication serialize through one mutex and generations survive reopen.
- Independent consumers: `tosumu-sql` already consumes the shared logical KV
  boundary and supplies realistic keys and row encodings for conditional-write
  caller evidence.
- Diagnostics or audits: the current latest generation is observable, but no
  API atomically couples it to a logical value.
- Repeated implementation friction: none yet; this review precedes the public
  API so page identity does not accidentally become record identity.
- Missing evidence: durable per-key revision metadata, conditional delete, and
  cross-process optimistic tokens are not required by an admitted caller.

## Ownership And Dependency Analysis

Core owns the atomic observation, precondition check, mutation, and commit
generation. Callers own retry policy and the meaning of a conflict. The API
must remain logical KV vocabulary and must not expose pages, WAL records, SQL
rows, mutexes, or host scheduling.

## Alternatives Considered

### Alternative A: Use physical page versions as key versions

- Benefits: an existing field appears reusable.
- Costs: page rewrites and B+ tree movement are not logical key mutations.
- Failure mode: tokens change for unrelated physical work or fail to identify
  the logical history callers believe they are comparing.

### Alternative B: Add durable per-key revision metadata now

- Benefits: unrelated commits would not reject a key-specific update.
- Costs: changes record/format representation and every mutation path before a
  caller proves the extra precision is necessary.
- Failure mode: premature format coupling enlarges recovery and migration work.

### Alternative C: Use the database committed generation

- Benefits: already durable, monotonic, authenticated at the storage boundary,
  and sufficient to reject stale and ABA updates.
- Costs: any intervening commit, including to another key, invalidates a token.
- Failure mode: conservative false conflicts increase retries but cannot admit
  a stale write.

### Alternative D: Report unmet preconditions as errors

- Benefits: maps directly to the existing broad `Conflict` status.
- Costs: expected compare outcomes become exceptional control flow and require
  a new stable error code despite successful contract completion.
- Failure mode: callers conflate ordinary contention with operation failure.

## Findings

The database generation is the only currently truthful stable version. The
public token binds that generation to one live shared-owner identity because
unencrypted format 3 has no unique durable database ID. It is a database-wide,
owner-lifetime optimistic token, not a per-key revision or freshness witness.
An unmet condition is a normal result. The smallest useful surface is an atomic
versioned read plus `put_if_absent`, value compare-and-set, and
generation-checked put. Conditional delete and transaction-local convenience
methods can wait for caller evidence.

## Disposition

Accepted through ADR-0007.

## Required Follow-Up

- [x] Record the public token, observation, and outcome contract in ADR-0007.
- [x] Implement the focused shared KV slice without format changes.
- [x] Retain core and separate SQL-layer caller evidence.
- [x] Require no compatibility or migration work for this API-only change.

## Reopening Triggers

Reopen if measured unrelated-write conflicts require durable per-key revisions,
if callers need conditional delete or multi-key preconditions, or if a token
must survive owner reopen or become serializable.

## Review History

### Cycle 1 -- 2026-09-02

- Status entering review: Proposed
- New evidence: format 3 already supplies a durable database generation, while
  records have no durable logical revision and physical page versions do not
  identify key history.
- Findings: database-wide tokens are conservative but correct; precondition
  misses are outcomes; no format change is justified.
- Disposition: Accepted through ADR-0007.
- Resulting ADR or documentation change: admit the smallest conditional-write
  extension to ADR-0006's supported shared KV surface.

### Cycle 2 -- 2026-09-02

- Status entering review: Accepted
- New evidence: `SharedKvStore` now exposes owner-scoped `KvVersion`, atomic
  versioned reads, and three conditional put forms. Focused tests prove stale
  generation and wrong-value rejection, no generation advance on misses or
  invalid staged work, exactly one concurrent put-if-absent winner, successful
  token chaining, and rejection by different and reopened owners. The separate
  SQL crate exercises the API with real row encodings.
- Findings: the owner identity can remain an in-memory capability that does not
  retain the pager or writer gate. No new error code, dependency, or format byte
  is required.
- Disposition: remain Accepted through ADR-0007.
- Resulting ADR or documentation change: mark the conditional-write slice
  complete and advance MVP+10 planning to secondary indexes.
