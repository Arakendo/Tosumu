# Tosumu Architectural Review Records

Architectural Review Records preserve unresolved questions, implementation or
consumer pressure, evidence, alternatives, findings, dispositions, and
reopening criteria. They sit between informal notes and binding ADRs.

```text
Observation or pressure
    ↓
Architectural Review
    ├── incubate / defer / reject / confirm current architecture
    └── accept architectural change
            ↓
           ADR
```

## Open A Review When

- ownership between storage core, relational layers, adapters, and consumers is
  unclear;
- a file-format or compatibility change is proposed;
- a new stable public contract or cross-layer dependency is proposed;
- repeated implementation friction suggests an existing boundary is wrong;
- corpus or independent-consumer evidence may reopen an accepted decision;
- a proposal is deferred or rejected and its reasoning should remain durable.

Use `AR-NNNN-short-title.md`. Review numbering is independent from ADR
numbering. Copy `TEMPLATE.md` and add the record to this index.

## Statuses

- **Proposed**
- **Under Review**
- **Incubating**
- **Accepted** -- requires an ADR or deliberate ADR revision
- **Deferred**
- **Rejected**
- **No Change**
- **Superseded**
- **Reopened**

## Index

- [AR-0001: TQL Command Language Boundary](AR-0001-tql-command-language-boundary.md)
  -- incubating ownership, lowering, and structured-outcome boundaries for the
  Tosumu operator command surface.
- [AR-0002: Structured Inspection Contract Boundary](AR-0002-structured-inspection-contract-boundary.md)
  -- incubating the provider-neutral inspection snapshot boundary below CLI
  serialization.
- [AR-0003: Service Authority And Host Modes](AR-0003-service-authority-and-host-modes.md)
  -- incubating embedded, daemon, and remote host ownership without admitting
  distributed storage.
- [AR-0004: Semantic Change History And Sync](AR-0004-semantic-change-history-and-sync.md)
  -- separating application mutations and sync evidence from the physical WAL.
- [AR-0005: Witness, Observer, And Freshness](AR-0005-witness-observer-and-freshness.md)
  -- preserving the distinction between authenticated local integrity and
  externally anchored freshness.
- [AR-0006: Format Evolution And Migration Boundary](AR-0006-format-evolution-and-migration-boundary.md)
  -- holding pre-stability format evolution and migration policy open until a
  concrete incompatible change exists.
- [AR-0007: Core Change Evidence And Resilience Discipline](AR-0007-core-change-evidence-and-resilience.md)
  -- incubating proportional validation and failure-evidence gates for core
  contracts and risky adapter crossings.
- [AR-0008: Operation Outcome Closure And Crash Evidence](AR-0008-operation-outcome-closure-and-crash-evidence.md)
  -- defining honest terminal evidence for crash, survival, and hosted-operation
  claims without guessing causes from missing reports.
- [AR-0009: Multiple-Reader Execution And Coordination](AR-0009-multiple-reader-execution-and-coordination.md)
  -- owning the MVP+10 locking, visibility, checkpoint, and mechanism baseline.
- [AR-0010: Dependency Trust And Source Provenance](AR-0010-dependency-trust-and-source-provenance.md)
  -- reviewing the source identity and audit burden of format- and
  authentication-critical dependencies.

All indexed reviews are currently **Incubating**. There are no accepted,
rejected, deferred, or superseded reviews.
