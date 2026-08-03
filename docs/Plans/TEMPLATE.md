# Plan Title

| Field | Value |
| --- | --- |
| Status | Proposed |
| Opened | YYYY-MM-DD |
| Last updated | YYYY-MM-DD |
| Owner | Maintainer or working group |
| Target | Milestone, stage, crate, or consumer |
| Related ADRs | None |
| Related reviews | None |
| Related CRs | None |
| Depends on | Existing capabilities, plans, or external work |

## Status

Summarize the present implementation state in one short paragraph. Distinguish
completed evidence, active work, deferred work, and blockers.

## Purpose

State the concrete outcome this plan exists to produce and why Tosumu needs it
now. Describe the user, consumer, compatibility, correctness, or maintainability
pressure rather than merely naming a feature.

## Trigger And Evidence

Record the evidence that justifies implementation:

- failing or missing behavior;
- tests, fuzz findings, diagnostics, or benchmarks;
- independent consumer requests;
- repeated implementation friction;
- relevant format, recovery, integrity, or security constraints.

Separate observed behavior from architectural guarantees.

## Current State

Describe what exists today, including public APIs, crates, file-format behavior,
tests, diagnostics, and known limitations. Link the owning source files and
specifications.

## Goals

- Goal one.
- Goal two.
- Goal three.

## Non-Goals

- Explicitly deferred behavior.
- Attractive adjacent work that this plan must not absorb.
- Guarantees that current evidence does not support.

## Ownership And Dependency Boundary

Describe who owns meaning, who owns storage mechanics, and which dependencies
are permitted.

```text
Consumer meaning
    ↓
Adapter or relational semantics
    ↓
Tosumu public storage contract
    ↓
Pager, B+ tree, WAL, recovery, and physical format
```

### This Work Owns

- Semantics and decisions introduced by this plan.

### This Work Must Not Own

- Consumer-specific meaning.
- Unrelated storage or platform policy.
- Guarantees that belong to another layer.

### Dependency Direction

State current and intended dependency direction. Identify forbidden upward,
cyclic, consumer-specific, or physical-format dependencies.

## Public Contract Impact

Identify proposed public types, functions, errors, diagnostics, format changes,
or compatibility behavior. State which contracts remain provisional.

If the plan changes architecture, stop and open or update an Architectural
Review and ADR before treating the new boundary as accepted.

## Deliverables

- [ ] Deliverable one.
- [ ] Deliverable two.
- [ ] Documentation and diagnostics updated.
- [ ] Focused tests and consumer evidence added.

## Implementation Slices

Each slice must compile and leave the repository in a coherent state. Add,
remove, or split slices as evidence requires; do not mark a slice complete
until all of its acceptance criteria and validation are satisfied.

### Slice 0: Baseline And Boundary Confirmation

**Objective:** Capture current behavior and confirm that the proposed work fits
accepted Tosumu ownership and compatibility boundaries.

#### Deliverables

- [ ] Read the relevant root specifications, ADRs, Reviews, and CRs.
- [ ] Record a focused baseline test, fixture, diagnostic, or measurement.
- [ ] Identify public API and on-disk compatibility impact.
- [ ] Open an Architectural Review if ownership remains unresolved.

#### Acceptance Criteria

- [ ] The problem is reproducible or otherwise supported by concrete evidence.
- [ ] Observation and guarantee are distinguished explicitly.
- [ ] Dependencies and non-goals are documented.
- [ ] No implementation work silently settles an open architecture question.

#### Validation

```text
Commands, fixtures, reports, or manual evidence used for the baseline.
```

#### Exit State

Describe the stable state that permits Slice 1 to begin.

### Slice 1: Smallest Useful Vertical Behavior

**Objective:** Implement the narrowest end-to-end behavior that proves the
contract without speculative generalization.

#### Deliverables

- [ ] Add the smallest compiling implementation.
- [ ] Add typed errors and bounded diagnostics for expected failures.
- [ ] Add focused unit and integration tests.
- [ ] Exercise the behavior through one real caller.

#### Acceptance Criteria

- [ ] Supported behavior succeeds deterministically.
- [ ] Unsupported or invalid behavior fails explicitly.
- [ ] No lower layer learns higher-level consumer semantics.
- [ ] Existing format and recovery guarantees remain intact or are migrated
      deliberately.

#### Validation

```text
cargo test -p owning-crate focused_test_name
```

#### Exit State

Describe the useful capability now available and the limitations intentionally
left for later slices.

### Slice 2: Hardening And Independent Pressure

**Objective:** Expand correctness evidence and test the boundary against another
caller, failure mode, or compatibility case.

#### Deliverables

- [ ] Add malformed, limit, rollback, recovery, or compatibility cases relevant
      to the feature.
- [ ] Add a second independent caller or explain the evidence substitution.
- [ ] Record divergences as provider-only behavior, contract refinements, or
      rejected semantics.
- [ ] Update specifications and public documentation.

#### Acceptance Criteria

- [ ] Failures remain typed, bounded, and inspectable.
- [ ] Repeated operations do not corrupt identity, ordering, or durability.
- [ ] Compatibility behavior is explicit and tested.
- [ ] The public contract contains no accidental physical implementation leak.

#### Validation

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

#### Exit State

Describe whether the work is complete, remains incubating, or has exposed a new
architectural question.

### Slice N: Completion, Migration, Or Parking

**Objective:** Close the plan honestly rather than leaving ambiguous residual
work.

#### Deliverables

- [ ] Reconcile code, tests, root specifications, public docs, ADRs, and Reviews.
- [ ] Record deferred work and unsupported behavior explicitly.
- [ ] Remove superseded compatibility paths or document their lifetime.
- [ ] Record final validation evidence.

#### Acceptance Criteria

- [ ] All in-scope acceptance criteria pass.
- [ ] Remaining work has an owner, destination, and reopening trigger.
- [ ] The plan status accurately says completed, parked, superseded, or blocked.
- [ ] No checklist item is marked complete solely because work moved out of
      sight.

#### Validation

```text
Final commands and retained artifacts.
```

#### Exit State

State the final outcome in one direct paragraph.

## Validation Matrix

| Concern | Evidence | Command Or Artifact | Required Result |
| --- | --- | --- | --- |
| Unit behavior | Focused unit tests | `cargo test -p ...` | Pass |
| Cross-layer behavior | Integration or consumer test | Name | Pass |
| Invalid input | Rejection and limit tests | Name | Typed failure |
| Recovery and durability | Crash/reopen fixture where relevant | Name | Documented guarantee |
| Compatibility | Format/API fixture where relevant | Name | Explicit result |
| Fuzzing | Decoder or mutation target where relevant | Name | No unexpected panic/corruption |
| Documentation | Strict MkDocs build | `mkdocs build --strict` | Pass |

Remove rows that genuinely do not apply and explain why. Add security,
performance, WASM, .NET, or cross-version rows when the plan affects them.

## Failure And Diagnostic Semantics

List expected failure classes and who owns each decision. Include limits,
corruption, unsupported versions, incomplete writes, invalid consumer input,
and unavailable providers where relevant.

No expected failure should rely only on a log message or silent fallback.

## Compatibility And Migration

State effects on:

- on-disk format and versioning;
- WAL, checkpoint, backup, and recovery;
- public Rust, CLI, and .NET APIs;
- existing fixtures and consumers;
- downgrade, rollback, and partial-migration behavior.

Write `No impact` only after checking each applicable boundary.

## Security And Trust

State whether the work changes authenticated data, key/protector behavior,
resource limits, untrusted-input parsing, integrity claims, or freshness
evidence. Keep claims consistent with `SECURITY.md`.

## Performance And Resource Bounds

Record relevant time, memory, file-size, page-count, allocation, or iteration
bounds. Performance observations are evidence, not guarantees, unless an
accepted contract says otherwise.

## Risks And Mitigations

| Risk | Impact | Mitigation Or Evidence |
| --- | --- | --- |
| Example risk | Consequence | Test, bound, diagnostic, or design response |

## Completion Criteria

The plan is complete when:

- [ ] all in-scope slices and acceptance criteria pass;
- [ ] public behavior and unsupported cases are documented;
- [ ] validation is repeatable;
- [ ] architectural consequences are reflected in Reviews and ADRs;
- [ ] deferred work is named rather than implied.

## Parking Or Reopening Criteria

If the plan is parked, state why useful work should stop now. Name observable
events that justify reopening, such as an independent consumer, a format
failure, a recovery divergence, or new compatibility evidence.

## Progress Log

### YYYY-MM-DD

- Work completed:
- Validation:
- Findings:
- Plan changes:
- Next slice:

Append progress entries; do not rewrite history to make the path look linear.

## References

- `DESIGN.md`
- Relevant ADRs and Architectural Reviews
- Relevant CRs, specifications, tests, fixtures, and source files

