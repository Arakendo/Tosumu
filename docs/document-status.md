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

## Architectural Reviews

All current reviews are **Incubating**. They preserve unresolved evidence and
do not change accepted architecture by themselves.

| Record | Question |
| --- | --- |
| [AR-0001](Architectural%20Reviews/AR-0001-tql-command-language-boundary.md) | TQL ownership and lowering |
| [AR-0002](Architectural%20Reviews/AR-0002-structured-inspection-contract-boundary.md) | Reusable inspection facts versus CLI serialization |
| [AR-0003](Architectural%20Reviews/AR-0003-service-authority-and-host-modes.md) | Embedded, daemon, and remote authority boundaries |
| [AR-0004](Architectural%20Reviews/AR-0004-semantic-change-history-and-sync.md) | Semantic history and sync ownership |
| [AR-0005](Architectural%20Reviews/AR-0005-witness-observer-and-freshness.md) | Witness, observer, and freshness evidence |
| [AR-0006](Architectural%20Reviews/AR-0006-format-evolution-and-migration-boundary.md) | Format evolution and migration ownership |

## Implementation Plans

| Plan | Lifecycle | Next action |
| --- | --- | --- |
| [Main Feature Roadmap](Plans/main-feature-roadmap.md) | Active | Close MVP+9 follow-up and define the MVP+10 gate |
| [Initial SQL Layer](Plans/initial-sql-layer.md) | Completed baseline; retained | Resolve separately listed deferred SQL scope only through new evidence or a follow-up plan |
| [Tosumu Command Language](Plans/tosumu-command-language.md) | Proposed | Complete Slice 0 evidence and retain AR-0001 ownership review |
| [Documentation Lifecycle And Design Decomposition](Plans/documentation-lifecycle-and-design-decomposition.md) | Active | Apply metadata and separate current design from future proposals incrementally |

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
