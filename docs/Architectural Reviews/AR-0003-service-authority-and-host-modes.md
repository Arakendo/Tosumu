# AR-0003: Service Authority And Host Modes

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-03 |
| Last reviewed | 2026-08-03 |
| Scope | Core storage / service adapter / deployment hosts |
| Trigger | Architecture notes describe embedded, daemon, and remote hosts without an implemented service boundary |
| Related ADRs | ADR-0001 |
| Related evidence | `docs/architecture.md`, `docs/Specifications/Tosumu Software Design Document.md`, current embedded CLI and provider consumers |

## Architectural Question

Does Tosumu need one provider-neutral authority/service contract shared by
embedded, local-daemon, and remote administration hosts, and if so which
lifecycle and serialization decisions belong there rather than in core?

## Context

Tosumu is implemented and described as embedded-first, single-process, and
single-writer. Architecture notes also sketch a future `tosumu-service` that
would coordinate open/close lifecycle, unlock state, write serialization,
inspection shaping, and boundary error mapping across several host modes.

No service, daemon, or server crate currently proves that contract. The main
design also explicitly rejects distributed storage as a core identity. The
review prevents a future deployment sketch from becoming accepted architecture
without implementation evidence.

## Evidence

- Tests or fuzzing: current tests validate embedded storage, not host parity.
- Independent consumers: CLI and Tokimu's provider use embedded APIs.
- Diagnostics or audits: busy and structured-error behavior demonstrate some
  authority concerns, but not a service contract.
- Repeated implementation friction: none yet across distinct host mechanisms.
- Missing evidence: one local IPC host, lifecycle parity tests, authentication
  policy, cancellation, and multi-database authority behavior.

## Ownership And Dependency Analysis

- Core owns storage semantics, transactions, recovery, and single-writer rules.
- A future authority layer may own connection lifecycle and boundary policy.
- Host adapters own IPC, HTTP, process management, and platform mechanisms.
- Hosts must not reimplement storage semantics or make remote deployment imply
  distributed storage.

## Alternatives Considered

### Alternative A: Embedded APIs only

- Benefits: smallest system and strongest current evidence.
- Costs: every daemon or UI host may duplicate lifecycle policy.
- Failure mode: divergent authority semantics across consumers.

### Alternative B: Admit `tosumu-service` now

- Benefits: establishes a common boundary early.
- Costs: freezes a contract with no non-embedded implementation.
- Failure mode: speculative service abstractions leak into core.

### Alternative C: Incubate the host model

- Benefits: preserves the intended ownership direction while waiting for a
  concrete second host.
- Costs: current consumers continue using embedded APIs directly.
- Failure mode: architecture prose may drift unless this review remains linked.

## Findings

- Embedded operation remains the only established host mode.
- A service boundary is plausible but not admitted.
- Remote administration and distributed storage are different concepts.

## Disposition

Incubating. Treat service and host modes as a reviewed hypothesis, not as an
implemented or accepted subsystem.

## Required Follow-Up

- [ ] Build one bounded local-host experiment without changing core semantics.
- [ ] Compare embedded and hosted lifecycle, errors, and inspect outcomes.
- [ ] Record authority, authentication, cancellation, and shutdown behavior.
- [ ] Accept, defer, or reject a service layer through a later review cycle.

## Reopening Triggers

- A consumer requires process isolation or shared local authority.
- Two host adapters duplicate lifecycle or write-serialization policy.
- Hosted operation pressures the current single-process guarantee.

## Review History

### Cycle 1 -- 2026-08-03

- Status entering review: Proposed
- New evidence: architecture prose was compared with the actual workspace.
- Findings: the host model is coherent but currently speculative.
- Disposition: Incubating
- Resulting ADR or documentation change: none

