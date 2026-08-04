# AR-0005: Witness, Observer, And Freshness Boundary

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-03 |
| Last reviewed | 2026-08-03 |
| Scope | Integrity / deployment / external observation |
| Trigger | Authenticated local storage cannot by itself prove freshness or prevent consistent rollback |
| Related ADRs | ADR-0002 |
| Related evidence | `docs/Specifications/Tosumu Software Design Document.md` witness/observer sections, `SECURITY.md` threat-model exclusions |

## Architectural Question

How should Tosumu represent and obtain evidence that authenticated local state
is current, without claiming that page authentication alone prevents a
consistent rollback or provides external truth?

## Context

Authenticated pages prove that bytes match a valid protected state. They do not
prove that the state is the newest valid state. The design sketches witnesses,
observers, signed receipts, audit heads, and deployment topologies to provide a
freshness anchor. None is implemented, audited, or part of the current threat
model.

## Evidence

- Tests or fuzzing: page tamper tests prove local integrity, not freshness.
- Independent consumers: no deployed witness or observer consumer exists.
- Diagnostics or audits: security documentation explicitly excludes
  multi-page rollback prevention and remote attestation.
- Repeated implementation friction: freshness claims appear in future design
  language but have no executable boundary.
- Missing evidence: receipt format, trust roots, outage policy, quorum rules,
  clock assumptions, rollback simulation, and key rotation.

## Ownership And Dependency Analysis

- Pager authentication owns local byte integrity.
- A witness mechanism would own external freshness evidence.
- An observer may collect and report evidence but must not silently become the
  storage authority.
- Applications own policy for refusing writes or accepting stale state.
- Core must not report "current" without an external anchor capable of proving it.

## Alternatives Considered

### Alternative A: Treat authenticated storage as fresh

- Benefits: simple status model.
- Costs: makes a false security claim.
- Failure mode: a valid older snapshot is accepted as current.

### Alternative B: Admit the proposed witness architecture now

- Benefits: records an ambitious target.
- Costs: freezes protocol and deployment semantics without evidence.
- Failure mode: security guarantees exist only in prose.

### Alternative C: Preserve an explicit freshness gap

- Benefits: honest guarantees and room for experiments.
- Costs: current deployments cannot prove recency externally.
- Failure mode: consumers may ignore the distinction unless diagnostics expose it.

## Findings

- Integrity and freshness are separate guarantees.
- ADR-0002 does not establish rollback prevention, witness trust, or recency.
- Witness and observer concepts remain research candidates.

## Disposition

Incubating. Keep freshness explicitly unanchored until a tested external
evidence protocol exists.

## Required Follow-Up

- [ ] Define one bounded freshness claim and its failure semantics.
- [ ] Build rollback and stale-snapshot corpus cases.
- [ ] Prototype receipts without changing the storage format prematurely.
- [ ] Review security claims before any witness feature is described as supported.

## Reopening Triggers

- A real deployment must distinguish current from valid-but-stale state.
- A witness or observer prototype produces signed evidence.
- Tosumu documentation or APIs begin making freshness claims.

## Review History

### Cycle 1 -- 2026-08-03

- Status entering review: Proposed
- New evidence: security exclusions were reconciled with future witness prose.
- Findings: the problem is real; the proposed architecture remains unproven.
- Disposition: Incubating
- Resulting ADR or documentation change: none

