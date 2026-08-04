# Tosumu Documentation Map

Tosumu separates specifications, decisions, implementation work, incoming
requests, and exploratory material so that each document has one clear job.

Use the [Document Status](document-status.md) dashboard to see the authority,
lifecycle, and next action for current records.

## Required Status Metadata

New durable documents should identify both dimensions below near the top:

- **Authority** -- normative, binding, tracking, evidence, or informative.
- **Lifecycle** -- current, active, proposed, incubating, deferred, completed,
  historical, or superseded.

Authority answers whether a document can define Tosumu behavior. Lifecycle
answers whether it describes present state, active work, future work, or
history. `Draft` may describe editorial stability, but it does not replace
either dimension.

## Authority And Purpose

| Location | Purpose | Authority |
| --- | --- | --- |
| [`Specifications/`](Specifications/README.md) | Current design, error, and inspection contracts | Normative, except the informative reference index |
| Repository-root `SECURITY.md` | Security posture, limitations, and disclosure policy | Normative |
| [`ADR/`](ADR/README.md) | Accepted architectural decisions and consequences | Binding until superseded |
| [`Architectural Reviews/`](Architectural%20Reviews/README.md) | Questions, evidence, alternatives, findings, and reopening triggers | Evidence record; not binding by itself |
| [`Plans/`](Plans/README.md) | Concrete implementation slices and validation | Work sequencing only |
| [`CRs/`](CRs/README.md) | Incoming consumer and cross-project requests | Proposed until accepted |
| [`Notes/`](Notes/README.md) | Durable observations that are not decisions | Informative |
| [`Conversations/`](Conversations/README.md) | Preserved exploratory discussions and source material | Informative and unreviewed |
| [`Archive/`](Archive/README.md) | Retired or superseded material retained for history | Historical |
| `.workbench/` | Local audits, scratch notes, and temporary working material | Non-authoritative and gitignored |

The curated public documentation pages at the top of `docs/` summarize the
current system for users. They must remain consistent with the engineering
specifications and accepted ADRs rather than becoming a second specification.

## Source-Of-Truth Specifications

The design, error, inspection, and reference documents live in the published
[`Specifications/`](Specifications/README.md) collection. The security policy
retains its conventional repository-root location:

- `docs/Specifications/Tosumu Software Design Document.md` -- architecture, format, goals, and staged design
- `docs/Specifications/Tosumu Error Design Document.md` -- public error taxonomy and behavior
- `docs/Specifications/Tosumu Inspect API Specification.md` -- inspection contracts
- `SECURITY.md` -- security posture and limitations
- `docs/Specifications/Tosumu Reference Implementations.md` -- informative external references and influences

## Change Flow

```text
Observation, consumer request, or implementation pressure
    ↓
Note, Conversation, CR, or local workbench evidence
    ↓
Architectural Review when ownership or boundaries are unresolved
    ↓
ADR when an architectural decision is accepted
    ↓
Implementation Plan
    ↓
Code, tests, and updated public documentation
```

Small bug fixes and boundary-preserving refactors do not require an
Architectural Review. They still require tests and documentation updates when
public behavior changes.
