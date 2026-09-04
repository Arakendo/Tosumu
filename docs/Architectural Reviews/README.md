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
- [AR-0011: Committed Generation And Version Residence](AR-0011-committed-generation-and-version-residence.md)
  -- accepted through ADR-0005; retains the MVP+10 commit-LSN, retained-WAL,
  reader-horizon, checkpoint, limit, and format-v3 evidence.
- [AR-0012: Conditional Write And Version Token Semantics](AR-0012-conditional-write-and-version-token-semantics.md)
  -- accepted through ADR-0007; defines the database-generation token and
  conditional-write outcome boundary.
- [AR-0013: Secondary Index Representation And Ownership](AR-0013-secondary-index-representation-and-ownership.md)
  -- accepted through ADR-0008; assigns logical index representation and
  maintenance to SQL without adding physical B+ tree roots.
- [AR-0014: VACUUM Rebuild And Publication](AR-0014-vacuum-rebuild-and-publication.md)
  -- accepted through ADR-0009; defines offline rebuild, retained writer
  admission, protector continuity, verification, and atomic publication.
- [AR-0015: Native Replication Scope, Authority, And Failure Model](AR-0015-native-replication-scope-authority-and-failure-model.md)
  -- incubating whether Tosumu should admit single-leader native replication,
  and defining the scope, identity, failure, and fencing gates that precede a
  protocol decision.
- [AR-0016: Cryptographic Provider Seam And Suite Identity](AR-0016-cryptographic-provider-seam-and-suite-identity.md)
  -- accepted through ADR-0010 for private format-v3 and entropy facades;
  protectors, public provider identity, suite agility, and deployment claims
  remain incubating.
- [AR-0017: Mobile Embedding, ABI, And Hardware-Protector Boundary](AR-0017-mobile-embedding-abi-and-hardware-protector-boundary.md)
  -- incubating foreign-ABI ownership, mobile lifecycle and qualification, and
  real Keychain/Keystore pressure for opaque provider-owned keys.
- [AR-0018: Bounded Snapshot Range Pagination](AR-0018-bounded-snapshot-range-pagination.md)
  -- accepted through ADR-0006; defines a generation-pinned range page whose
  pair and payload bounds apply during traversal rather than after full
  materialization.
- [AR-0019: Foreign Atomic Mutation Batch Boundary](AR-0019-foreign-atomic-mutation-batch-boundary.md)
  -- incubating a bounded, copied, one-shot foreign command batch without
  replacing the callback-scoped core write transaction.

AR-0009, AR-0011, AR-0012, AR-0013, AR-0014, AR-0018, and the Gate C1 portion
of AR-0016 are **Accepted** through their related ADRs. Other indexed reviews
and unresolved portions are currently **Incubating**; none are rejected,
deferred, or superseded.
