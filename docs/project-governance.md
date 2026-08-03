# Tosumu Documentation Map

Tosumu separates specifications, decisions, implementation work, incoming
requests, and exploratory material so that each document has one clear job.

## Authority And Purpose

| Location | Purpose | Authority |
| --- | --- | --- |
| Repository-root specifications | Current storage, error, inspection, and security contracts | Normative |
| [`ADR/`](ADR/README.md) | Accepted architectural decisions and consequences | Binding until superseded |
| [`Architectural Reviews/`](Architectural%20Reviews/README.md) | Questions, evidence, alternatives, findings, and reopening triggers | Evidence record; not binding by itself |
| [`Plans/`](Plans/README.md) | Concrete implementation slices and validation | Work sequencing only |
| [`CRs/`](CRs/README.md) | Incoming consumer and cross-project requests | Proposed until accepted |
| [`Notes/`](Notes/README.md) | Durable observations that are not decisions | Informative |
| [`Conversations/`](Conversations/README.md) | Preserved exploratory discussions and source material | Informative and unreviewed |
| [`Archive/`](Archive/README.md) | Retired or superseded material retained for history | Historical |
| `.workbench/` | Local audits, scratch notes, and temporary working material | Non-authoritative and gitignored |

The curated public documentation pages at the top of `docs/` summarize the
current system for users. They must remain consistent with the root
specifications and accepted ADRs rather than becoming a second specification.

## Source-Of-Truth Specifications

These specifications live at the repository root and are intentionally outside
the MkDocs publication tree:

- `DESIGN.md` -- architecture, format, goals, and staged design
- `ERRORS.md` -- public error taxonomy and behavior
- `INSPECT_API.md` -- inspection contracts
- `SECURITY.md` -- security posture and limitations
- `REFERENCES.md` -- external references and influences

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
