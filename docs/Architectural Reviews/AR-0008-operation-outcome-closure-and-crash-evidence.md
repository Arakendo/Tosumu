# AR-0008: Operation Outcome Closure And Crash Evidence

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-27 |
| Last reviewed | 2026-08-27 |
| Scope | Core operations / recovery tests / CLI and hosted boundaries |
| Trigger | Tosumu distinguishes outcomes from errors and injects crashes, but does not yet define one cross-boundary rule for unexplained operation disappearance |
| Related ADRs | ADR-0001, ADR-0002 |
| Related evidence | Error Design Document, SDD crash simulation, WAL recovery tests, CLI and WASM boundaries |

## Architectural Question

Should every started bounded Tosumu operation close with an observed success,
structured failure, independently observed termination, or explicit unresolved
disappearance, and which operations and observers are in scope?

## Context

The error specification preserves original causes, distinguishes completed
findings from boundary failures, and discourages process exit below the CLI
boundary. The SDD also treats crash injection and recovery as first-class
evidence. These rules cover returned behavior well but do not completely define
what evidence is required when the process, worker, browser page, or external
provider disappears before returning a report.

This matters especially for WAL and recovery tests, backup and export
publication, browser/WASM inspection, future hosted operation, and any claim
that failure was contained. Missing in-process evidence cannot establish that
an operation failed safely, nor can it identify an out-of-memory condition,
panic, host kill, or provider fault.

Tosumu deliberately tests crash behavior, so an outcome-closure rule must not
misclassify an injected crash as an unexpected implementation failure. The test
harness and the operation contract must identify the expected terminal domain.

## Evidence

- Tests or fuzzing: crash simulation injects failures and verifies reopen
  behavior; malformed inputs are expected to return typed failures rather than
  panic.
- Independent consumers: CLI and WASM boundaries can observe returned reports,
  but they share different failure domains.
- Diagnostics or audits: the durable `ErrorReport` shape preserves code,
  status, message, details, and an optional source.
- Repeated implementation friction: plans distinguish executed validation,
  unavailable environments, and inferred success, but no common terminal
  classification exists.
- Missing evidence: no retained subprocess/browser harness demonstrates a
  common classification across normal completion, structured rejection,
  expected injected crash, unexpected termination, timeout, and observer
  unavailability.

## Ownership And Dependency Analysis

- Core operations return typed outcomes or errors while core remains able to
  execute; core does not own process or browser supervision.
- WAL and recovery tests define which injected termination points are expected
  and verify the post-reopen invariant.
- CLI, service, browser, and test harnesses observe lifecycle events available
  to their host and preserve operation identity.
- An observer used to support a survival or containment claim must live outside
  the failure domain whose loss is being tested.
- Presentation and logging must not replace the first trustworthy cause or
  turn missing evidence into a guessed diagnosis.

## Alternatives Considered

### Alternative A: Returned errors are sufficient

- Benefits: matches ordinary library control flow.
- Costs: says nothing when the process or host disappears.
- Failure mode: the most severe failures become the least observable while
  still being reported as ordinary test failures or timeouts.

### Alternative B: Require external supervision for every operation

- Benefits: strongest lifecycle observation.
- Costs: disproportionate for ordinary in-process calls and unit tests.
- Failure mode: expensive ceremony obscures the boundaries that actually need
  an independent observer.

### Alternative C: Scope outcome closure to bounded operations and claims

- Benefits: ordinary calls retain typed returns, while crash, survival,
  containment, and hosted-operation claims add external observation where
  needed.
- Costs: requires precise definitions of operation identity, timeout, and
  expected injected termination.
- Failure mode: vague scoping lets a serious disappearance be relabeled
  `unavailable` or `skipped`.

## Findings

- An unresolved disappearance must never count as success, recovery, or safe
  containment.
- Unknown termination remains unknown; absence of a callback or report does not
  identify a cause.
- First-cause preservation and bounded evidence are already consistent with the
  Error Design Document.
- The exact public vocabulary and mandatory operation scope need executable
  subprocess and browser evidence before acceptance.

## Disposition

Incubating. Use the following provisional terminal categories in new crash or
host-lifecycle evidence: `completed`, `structured_failure`,
`expected_injected_termination`, `externally_observed_termination`, and
`unresolved_disappearance`. These are review vocabulary, not yet a public enum
or stable serialized contract.

## Required Follow-Up

- [ ] Build a subprocess fixture covering normal completion, structured error,
      controlled termination, timeout, and observer failure.
- [ ] Demonstrate that an injected crash remains distinguishable from an
      unexpected process disappearance.
- [ ] Apply the classification to one WAL/recovery case and one CLI or WASM
      boundary case.
- [ ] Define first-cause retention, operation identity, evidence bounds, and
      privacy requirements.
- [ ] Decide whether accepted meaning belongs in an ADR, the Error Design
      Document, or both.

## Reopening Triggers

- A crash or timeout is reported as success, skipped, or unavailable without
  retained terminal evidence.
- A recovery or containment claim depends only on an in-process observer.
- A hosted or browser operation introduces a new failure domain.
- The provisional categories cannot distinguish expected crash injection from
  implementation failure.

## Review History

### Cycle 1 -- 2026-08-27

- Status entering review: Proposed
- New evidence: error-boundary rules and current crash-test claims were
  compared.
- Findings: the invariant is useful, but public scope and supervision evidence
  remain incomplete.
- Disposition: Incubating
- Resulting ADR or documentation change: none

