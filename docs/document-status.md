# Document Status

This page is the front door for determining what Tosumu documentation means,
how much authority it carries, and whether its work is current, pending, or
historical.

## Status Vocabulary

Authority and lifecycle are separate:

| Dimension | Value | Meaning |
| --- | --- | --- |
| Authority | Normative | Defines a current Tosumu contract. Code and public docs must agree with it. |
| Authority | Binding | Records an accepted architectural decision until it is superseded. |
| Authority | Tracking | Sequences or reports work but does not create architecture. |
| Authority | Evidence | Preserves findings and alternatives without deciding them alone. |
| Authority | Informative | Provides explanation, context, or research only. |
| Lifecycle | Current | Describes the present supported or governing state. |
| Lifecycle | Active | Work is approved and currently being advanced. |
| Lifecycle | Proposed | Work is defined but has not been admitted for implementation. |
| Lifecycle | Incubating | Evidence is being gathered before a durable decision. |
| Lifecycle | Deferred | Intentionally waiting for a named trigger or dependency. |
| Lifecycle | Completed | In-scope work and acceptance criteria are complete. |
| Lifecycle | Historical | Retained for context; it does not describe current work. |
| Lifecycle | Superseded | Replaced by a named successor. |

`Draft` describes editorial stability, not authority. A normative pre-stability
specification may still be a draft while remaining the current contract.

## Engineering Specifications

Durable design, error, and inspection contracts live together under
[`Specifications/`](Specifications/README.md). The repository-root
`SECURITY.md` remains in its conventional location for GitHub security tooling
and responsible-disclosure discovery.

| Document | Authority | Lifecycle | Scope |
| --- | --- | --- | --- |
| [Tosumu Software Design Document](Specifications/Tosumu%20Software%20Design%20Document.md) | Normative | Current, pre-stability | Architecture, format, goals, and staged design |
| [Tosumu Error Design Document](Specifications/Tosumu%20Error%20Design%20Document.md) | Normative | Current | Public structured error contract |
| [Tosumu Inspect API Specification](Specifications/Tosumu%20Inspect%20API%20Specification.md) | Normative | Current | Machine-readable inspection contract |
| Repository root: `SECURITY.md` | Normative | Current, pre-audit | Threat model, limitations, and disclosure policy |
| [Tosumu Reference Implementations](Specifications/Tosumu%20Reference%20Implementations.md) | Informative | Current | External references and implementation influences |

The Tosumu Software Design Document contains both implemented architecture and clearly labeled future
or deferred material. The current separation work is tracked by the
[Documentation Lifecycle And Design Decomposition Plan](Plans/documentation-lifecycle-and-design-decomposition.md).

## Curated Public Documentation

The top-level pages under `docs/` are current user-facing explanations. They
are informative summaries and must agree with the normative root
specifications and accepted ADRs.

| Area | Lifecycle |
| --- | --- |
| Getting started, concepts, architecture, format, errors, inspection, CLI, safety | Current |
| [Public roadmap](roadmap.md) | Current summary; implementation status is tracked by the main feature roadmap |
| [Tosumu Command Language](Tosumu%20Command%20Language.md) | Proposed / exploratory; not a supported command surface |

## Accepted Decisions

| Record | Status |
| --- | --- |
| [ADR-0001: Storage Engine Layer Boundaries](ADR/ADR-0001-storage-engine-layer-boundaries.md) | Accepted / binding |
| [ADR-0002: Authenticated Pager Trust Boundary](ADR/ADR-0002-authenticated-pager-trust-boundary.md) | Accepted / binding |
| [ADR-0003: Source Unit Cohesion, Size Pressure, And Decomposition](ADR/ADR-0003-source-unit-cohesion-size-pressure-and-decomposition.md) | Accepted / binding |
| [ADR-0004: Cooperative Single-Writer Admission](ADR/ADR-0004-cooperative-single-writer-admission.md) | Accepted / binding |
| [ADR-0005: Committed Generation And Retained-WAL Snapshots](ADR/ADR-0005-committed-generation-and-retained-wal-snapshots.md) | Accepted / binding |
| [ADR-0006: Shared KV Store And Snapshot Transactions](ADR/ADR-0006-shared-kv-store-and-snapshot-transactions.md) | Accepted / binding |
| [ADR-0007: Database-Generation Conditional Writes](ADR/ADR-0007-database-generation-conditional-writes.md) | Accepted / binding |

## Architectural Reviews

AR-0009 is **Accepted** through ADR-0004, ADR-0005, and ADR-0006. AR-0011 is
**Accepted** through ADR-0005, and AR-0012 is **Accepted** through ADR-0007.
Other current reviews are **Incubating** and do not change accepted architecture
by themselves.

| Record | Question |
| --- | --- |
| [AR-0001](Architectural%20Reviews/AR-0001-tql-command-language-boundary.md) | TQL ownership and lowering |
| [AR-0002](Architectural%20Reviews/AR-0002-structured-inspection-contract-boundary.md) | Reusable inspection facts versus CLI serialization |
| [AR-0003](Architectural%20Reviews/AR-0003-service-authority-and-host-modes.md) | Embedded, daemon, and remote authority boundaries |
| [AR-0004](Architectural%20Reviews/AR-0004-semantic-change-history-and-sync.md) | Semantic history and sync ownership |
| [AR-0005](Architectural%20Reviews/AR-0005-witness-observer-and-freshness.md) | Witness, observer, and freshness evidence |
| [AR-0006](Architectural%20Reviews/AR-0006-format-evolution-and-migration-boundary.md) | Format evolution and migration ownership |
| [AR-0007](Architectural%20Reviews/AR-0007-core-change-evidence-and-resilience.md) | Proportional evidence and resilience gates for core changes and risky crossings |
| [AR-0008](Architectural%20Reviews/AR-0008-operation-outcome-closure-and-crash-evidence.md) | Terminal outcome and crash-observation evidence |
| [AR-0009](Architectural%20Reviews/AR-0009-multiple-reader-execution-and-coordination.md) | Multiple-reader visibility, locking, and execution ownership |
| [AR-0010](Architectural%20Reviews/AR-0010-dependency-trust-and-source-provenance.md) | Dependency trust, exact source identity, and release provenance |
| [AR-0011](Architectural%20Reviews/AR-0011-committed-generation-and-version-residence.md) | Accepted committed-generation, retained-WAL, checkpoint, and format-v3 evidence |
| [AR-0012](Architectural%20Reviews/AR-0012-conditional-write-and-version-token-semantics.md) | Database-generation tokens and conditional-write outcomes |

## Implementation Plans

| Plan | Lifecycle | Next action |
| --- | --- | --- |
| [Main Feature Roadmap](Plans/main-feature-roadmap.md) | Active | Plan plain single-column secondary indexes with atomic mutation and recovery evidence |
| [Initial SQL Layer](Plans/initial-sql-layer.md) | Completed baseline; retained | Resolve separately listed deferred SQL scope only through new evidence or a follow-up plan |
| [Tosumu Command Language](Plans/tosumu-command-language.md) | Proposed | Complete Slice 0 evidence and retain AR-0001 ownership review |
| [Documentation Lifecycle And Design Decomposition](Plans/documentation-lifecycle-and-design-decomposition.md) | Active | Apply metadata and separate current design from future proposals incrementally |
| [Public Website And Repository Records](Plans/public-website-and-repository-records.md) | Proposed | Validate indexed navigation, then curate the public reader path separately from GitHub work records |
| [Core Source Unit Decomposition](Plans/core-source-unit-decomposition.md) | Proposed | Capture conservation baselines, then split page-store/WAL test families and validated pager-private responsibilities |

## Change Requests

TOKIMU-001 is **accepted with Tosumu provider scope complete**. The Tokimu-side
adapter remains the consumer's work and does not keep the Tosumu request active.
See the [Change Request index](CRs/README.md).

## Supporting Material

| Collection | Lifecycle rule |
| --- | --- |
| [Notes](Notes/README.md) | Informative observations; promote when they become decisions or work |
| [Conversations](Conversations/README.md) | Unreviewed source material |
| [Archive](Archive/README.md) | Historical or superseded support material |
| `.workbench/` | Local scratch material; never authoritative |

## Maintenance Rule

When a document changes lifecycle, update this page and its collection index in
the same change. A document is not completed merely because implementation
moved elsewhere; completed work must name its validation evidence and any
remaining deferred scope.
