# AR-0002: Structured Inspection Contract Boundary

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-03 |
| Last reviewed | 2026-08-03 |
| Scope | Core storage / CLI adapter / tooling consumers |
| Trigger | The inspect JSON schema has multiple consumers, while its envelope and payload types remain CLI-owned |
| Related ADRs | ADR-0001 |
| Related evidence | `docs/Specifications/Tosumu Error Design Document.md`, `docs/Specifications/Tosumu Inspect API Specification.md`, CLI/TUI/WPF consumers, `tosumu-core::error` and `tosumu-core::inspect` |

## Architectural Question

Which structured inspection facts and outcome contracts belong in a reusable
Tosumu capability, and which JSON serialization and command-shell concerns
must remain owned by `tosumu-cli`?

## Context

Tosumu already treats machine-readable inspection as a first-class surface.
Core owns structured errors and several inspection facts, while the common
JSON envelope and most serialized payload types currently live in
`tosumu-cli`. The TUI, WPF harness, and future tools are expected to consume
the contract rather than reinterpret database bytes or scrape human prose.

This is real multi-consumer pressure, but it does not yet prove that the JSON
wire shape itself belongs in core or that a new shared crate is necessary.

## Evidence

- Tests or fuzzing: core verification and structured-error tests exercise
  engine facts; CLI tests exercise envelope translation.
- Independent consumers: CLI/TUI and the WPF harness consume inspection
  results through different presentation mechanisms.
- Diagnostics or audits: `docs/Specifications/Tosumu Inspect API Specification.md` defines stable command IDs and a
  common structured outcome envelope.
- Repeated implementation friction: reusable facts and CLI serialization are
  adjacent in `inspect_contract.rs`, making ownership easy to blur.
- Missing evidence: a non-CLI Rust consumer of the complete inspection
  contract and an incompatible schema change requiring version policy.

## Ownership And Dependency Analysis

- Core owns storage facts, integrity findings, and domain error identity.
- An inspection capability may own provider-neutral snapshots and outcomes.
- CLI owns command parsing, JSON serialization, exit codes, and terminal prose.
- UI shells own presentation and must not decode physical storage independently.
- Core must not depend on CLI or on a specific JSON envelope.

## Alternatives Considered

### Alternative A: Keep the entire contract in the CLI

- Benefits: no new abstraction or crate.
- Costs: non-CLI tools either depend on CLI internals or duplicate translation.
- Failure mode: CLI serialization becomes accidental engine semantics.

### Alternative B: Move JSON payloads into core

- Benefits: one immediately reusable schema.
- Costs: core would own transport and serialization choices.
- Failure mode: a presentation contract becomes a storage-engine dependency.

### Alternative C: Incubate provider-neutral snapshots below CLI serialization

- Benefits: preserves core facts and permits multiple boundary encodings.
- Costs: temporary translation remains in the CLI.
- Failure mode: extraction may be premature without another direct consumer.

## Findings

- Structured inspection is a durable Tosumu capability, not formatted CLI
  output.
- Core-owned facts and CLI-owned JSON are distinct responsibilities.
- Current evidence does not justify moving the wire envelope into core or
  creating a new crate immediately.

## Disposition

Incubating. Preserve the current structured contract, keep JSON translation in
the CLI, and gather a direct non-CLI consumer before extracting shared snapshot
types.

## Required Follow-Up

- [ ] Exercise core inspection snapshots from one direct non-CLI Rust consumer.
- [ ] Inventory payload types that duplicate core facts versus boundary-only
      serialization.
- [ ] Define schema-version policy only when an incompatible change is real.
- [ ] Open an ADR if ownership or dependency direction changes.

## Reopening Triggers

- A second boundary needs the same complete inspection outcome.
- CLI-owned payload types must be imported by another crate.
- A schema change requires compatibility or version negotiation.

## Review History

### Cycle 1 -- 2026-08-03

- Status entering review: Proposed
- New evidence: normative error/inspect specifications and current crate
  ownership were compared.
- Findings: semantics are reusable; JSON and command mechanics remain CLI-owned.
- Disposition: Incubating
- Resulting ADR or documentation change: none

