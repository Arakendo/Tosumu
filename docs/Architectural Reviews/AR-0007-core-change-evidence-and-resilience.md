# AR-0007: Core Change Evidence And Resilience Discipline

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-27 |
| Last reviewed | 2026-08-27 |
| Scope | Core storage / adapters / cross-cutting engineering evidence |
| Trigger | Storage, recovery, integrity, and adapter changes currently rely on several distributed validation rules without one proportional admission boundary |
| Related ADRs | ADR-0001, ADR-0002, ADR-0003 |
| Related evidence | SDD §11, `SECURITY.md`, workspace tests, fuzz targets, crash simulation, provider and fixture evidence |

## Architectural Question

What proportional evidence must accompany changes to Tosumu's core storage
contracts, on-disk behavior, recovery, authenticated storage, and outer
adapters, and which parts belong in an ADR rather than a testing or contribution
guide?

## Context

The Software Design Document already makes testing normative and distinguishes
unit, property, fuzz, integration, crash, known-answer, fixture, and stage
acceptance evidence. The repository also requires formatting, strict Clippy,
workspace tests, and documentation validation where applicable.

What remains distributed is the decision rule for applying those forms of
evidence proportionally. A physical-format or recovery change needs stronger
failure and compatibility evidence than a CLI presentation adjustment, while
an adapter that parses hostile bytes or publishes persistent state cannot avoid
core-strength review merely because it lives outside `tosumu-core`.

This review does not assume that one large checklist should become permanent
architecture. It asks which invariants need a durable decision and which checks
should remain maintainable engineering guidance.

## Evidence

- Tests or fuzzing: core has focused unit and property tests, workspace
  integration tests, fault and crash tests, known-answer tests, and byte-facing
  fuzz targets.
- Independent consumers: the CLI, SQL layer, inspection adapters, WASM boundary,
  and Tokimu provider exercise different portions of the public surface.
- Diagnostics or audits: boundary errors are structured; verification reports
  distinguish reportable findings from failures that prevent a meaningful
  report.
- Repeated implementation friction: plans repeatedly need to explain which
  validation ran, which environment evidence is missing, and why one local
  result does not establish a broader guarantee.
- Missing evidence: there is no retained cross-project sample showing that one
  proportional gate improves decisions without producing ritual `N/A`
  responses; performance and allocation evidence is uneven across paths.

## Ownership And Dependency Analysis

- Core storage owners define invariants for pages, WAL, recovery, B+ trees,
  transactions, integrity, and bounded inspection.
- Adapter owners validate their inputs, preserve core error meaning, and prove
  the boundary they introduce without redefining storage semantics.
- Tests and fixtures prove claims at the narrowest honest boundary; corpus or
  independent consumers prove composition rather than private implementation.
- Maintainers decide whether missing evidence is acceptable and retain the
  reason. Tool output and test count do not substitute for that decision.
- Performance measurements remain observations tied to a workload, target, and
  build profile unless a separate public budget is explicitly accepted.

## Alternatives Considered

### Alternative A: Keep the current distributed rules

- Benefits: no new review ceremony; existing specifications remain sufficient
  for experienced maintainers.
- Costs: plans and changes continue reconstructing the applicable evidence set.
- Failure mode: a high-risk adapter or core crossing receives only convenient
  local validation.

### Alternative B: Accept one exhaustive core-change checklist now

- Benefits: one visible gate for ownership, performance, testing, containment,
  and recovery.
- Costs: duplicates normative specifications and may turn irrelevant checks
  into ritual.
- Failure mode: checkbox completion is mistaken for evidence of correctness.

### Alternative C: Incubate a proportional two-level discipline

- Benefits: tests a stronger gate for core contracts and risky crossings while
  retaining a smaller adapter gate.
- Costs: requires several real change records before the useful stable subset
  becomes clear.
- Failure mode: classification by directory hides an outer change that alters a
  core contract.

## Findings

- Tosumu already has enough evidence to require explicit invariants, typed
  failures, focused rejection tests, and retained validation results for core
  changes.
- Persistent commit, reopen, migration, recovery, unsafe code, hostile binary
  parsing, and cross-layer contract changes require stronger evidence wherever
  their implementation lives.
- The distinction among prevention, containment, capture, recovery,
  degradation, and fatal failure is useful and consistent with Tosumu's error
  and recovery specifications.
- There is not yet enough Tosumu-specific review history to accept a permanent
  exhaustive checklist or a new architectural classification vocabulary.

## Disposition

Incubating. Apply a provisional proportional evidence section in focused plans
or review records, then retain which questions materially changed a decision.
Do not describe checklist completion as a correctness, durability, security, or
recovery guarantee.

## Required Follow-Up

- [ ] Pilot the full gate on one substantive `tosumu-core` change.
- [ ] Pilot the smaller gate on one adapter or presentation change.
- [ ] Record which items affected a decision and which produced ritual
      not-applicable answers.
- [ ] Decide whether the stable result belongs in an ADR, the SDD testing
      strategy, a contribution guide, or a combination of those locations.
- [ ] Keep validation commands and unavailable environment evidence distinct
      from successful results.

## Reopening Triggers

- A core defect escapes because the relevant failure or compatibility boundary
  was not tested.
- An adapter corrupts or widens core state despite living outside
  `tosumu-core`.
- Review records repeatedly reconstruct the same evidence checklist.
- The provisional gate becomes ceremonial or materially slows low-risk work
  without improving decisions.

## Review History

### Cycle 1 -- 2026-08-27

- Status entering review: Proposed
- New evidence: existing SDD testing rules, current validation practices, and
  cross-cutting plan evidence were compared.
- Findings: a proportional discipline is justified, but its permanent form and
  location are not yet proven.
- Disposition: Incubating
- Resulting ADR or documentation change: none

