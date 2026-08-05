# Public Website And Repository Records

| Field | Value |
| --- | --- |
| Status | In progress |
| Opened | 2026-08-04 |
| Last updated | 2026-08-04 |
| Owner | Tosumu maintainers |
| Target | Tosumu MkDocs site and repository documentation lifecycle |
| Related ADRs | ADR-0001, ADR-0002 |
| Related reviews | None |
| Depends on | Documentation lifecycle and design decomposition |

## Status

Tosumu has a useful MkDocs foundation and a documented authority model, but
the current navigation exposes nearly every engineering record as though it
were part of one reader journey. Nested collection headings also appear like
links while behaving as inert labels. Indexed navigation fixes the immediate
interaction defect; a deliberate publication policy is still needed.

## Purpose

Turn the Tosumu site into a curated public explanation and evidence surface
without weakening GitHub as the complete engineering record.

The governing distinction is:

> **The website explains current Tosumu meaning and evidence. The repository
> preserves the complete path by which that meaning was discovered.**

## Audiences

| Audience | Primary questions |
| --- | --- |
| New user | What is Tosumu, what can it do, and how do I try it safely? |
| Integrator | Which APIs and formats are current, and what are their limits? |
| Evaluator | What evidence supports integrity, recovery, and inspection claims? |
| Contributor | Which specifications and accepted decisions constrain changes? |
| Maintainer or researcher | What plans, reviews, requests, and historical evidence led here? |

The first four audiences should not need to traverse working plans or raw
conversations. The final audience should retain access to every record through
GitHub.

## Publication Policy

### Public Website

Publish and place in primary navigation:

- project purpose, status, warnings, and maturity vocabulary;
- getting started and task-oriented guides;
- concepts and a readable architecture overview;
- file-format, error, inspection, CLI, safety, and compatibility references;
- the public roadmap and known limitations;
- normative specifications under a clearly labeled engineering-reference area;
- accepted ADRs because they explain binding design constraints;
- curated evidence labs, deterministic artifacts, and validation summaries;
- concise explanations of how Tosumu makes decisions.

### GitHub Engineering Record

Retain in the repository, but do not place in the ordinary public reading path:

- active or completed implementation plans;
- unresolved Architectural Reviews and raw evidence cycles;
- incoming and accepted Change Request internals;
- notes, conversations, inventories, and audits;
- templates and local workflow instructions;
- archived or superseded material;
- generated reports that have not been editorially summarized.

These records may still be built for link checking or exposed through a
clearly labeled **Engineering Record** page. They must not compete with user
guidance in the primary sidebar.

### Promote By Curation, Not Relocation

A repository record becomes website material by producing a stable public
summary:

```text
plan, review, note, or corpus result
            |
            v
reviewed current finding
            |
            v
guide, reference, status page, ADR, or evidence lab
```

The original record remains in GitHub. The public summary links back to it when
the extra history is useful.

## Information Architecture

The intended public navigation is:

```text
Home
Start
  Getting Started
  Safety And Limits
Learn
  Concepts
  Architecture
Use
  File Format
  Error Model
  Inspect API
  CLI Reference
Reference
  Specifications
  Accepted Decisions
Status
  Current Status
  Roadmap
Evidence
  Storage And Recovery
  Inspection And TQL
  Consumer Labs
Contribute
  Development
  Documentation And Decisions
Engineering Record
  GitHub indexes for reviews, plans, requests, notes, and history
```

Exact labels may change after reader testing. Ownership and lifecycle
distinctions must not.

## Goals

- Make every visible navigation item behave consistently.
- Keep the shortest useful path for users and evaluators obvious.
- Preserve specifications and accepted decisions as accessible public
  engineering reference.
- Move work tracking and exploratory history out of the primary reading path.
- Add visual evidence where it explains a bounded storage claim better than
  prose alone.
- Keep all claims useful without JavaScript or Tokimu presentation islands.

## Non-Goals

- Hiding engineering history or making Tosumu appear more mature than it is.
- Replacing GitHub issue, review, or source browsing.
- Publishing every document merely because MkDocs can render it.
- Building an interactive database administration product.
- Making the Tosumu site depend on a live Tokimu deployment.

## Deliverables

- [x] Enable indexed navigation so collection headings with index pages are
      clickable.
- [x] Give TOKIMU-001 and Supporting Records explicit landing pages.
- [x] Give the homepage a restrained Tosumu-specific evidence theme without
      adopting Tokimu's large editorial hero scale.
- [ ] Add a short public **How Tosumu Makes Decisions** page.
- [ ] Reorganize primary navigation around Start, Learn, Use, Reference,
      Status, Evidence, and Contribute.
- [ ] Move plans, unresolved reviews, CR internals, notes, conversations,
      templates, and archive history behind an Engineering Record boundary.
- [ ] Add one static-first evidence page using a deterministic Tosumu fixture.
- [ ] Add accessibility, mobile, link, and strict-build validation.

## Implementation Slices

### Slice 0: Navigation Correctness

**Deliverables**

- [x] Enable Material indexed navigation.
- [x] Make Specifications, Project Governance, ADRs, Reviews, Plans, Change
      Requests, TOKIMU-001, and Supporting Records resolve to landing pages.
- [ ] Verify desktop, mobile, keyboard, and screen-reader navigation behavior.

**Acceptance criteria**

- [ ] Every sidebar item that looks actionable is a link or an operable
      disclosure control.
- [ ] Collection headings open an explanatory index rather than a random child.
- [x] Strict MkDocs validation passes in the documentation environment.

### Slice 1: Reader-Facing Core

**Deliverables**

- [x] Rewrite the home page around one clear claim, maturity, and first action.
- [ ] Group current guidance into Start, Learn, Use, Reference, and Status.
- [ ] Add explicit maturity labels to experimental TQL and deferred features.

**Acceptance criteria**

- [x] A new reader can find build, safety, architecture, and current status in
      one navigation decision each.
- [x] Experimental behavior cannot be mistaken for supported behavior.
- [ ] Public summaries agree with specifications and accepted ADRs.

### Slice 2: Engineering Record Boundary

**Deliverables**

- [ ] Add a public engineering-record landing page.
- [ ] Remove working records from the primary navigation.
- [ ] Preserve stable GitHub links to every collection index.
- [ ] Decide whether repository-only pages are excluded from MkDocs or built
      but omitted from navigation.

**Acceptance criteria**

- [ ] Plans and reviews remain discoverable to contributors without crowding
      user documentation.
- [ ] No public page links to an excluded local route.
- [ ] GitHub remains the complete record of current and historical work.

### Slice 3: Evidence Pages

**Deliverables**

- [ ] Select one storage lifecycle or TQL fixture with known provenance.
- [ ] Publish textual facts, diagnostics, limitations, and a static artifact.
- [ ] Optionally add a Tokimu presentation island over the same versioned data.

**Acceptance criteria**

- [ ] The page communicates the complete result without JavaScript.
- [ ] Presentation invents no storage facts.
- [ ] Fixture, producer revision, schema, and limitations remain visible.

### Slice 4: Site Quality Gate

**Deliverables**

- [ ] Add strict MkDocs, internal-link, accessibility, and responsive checks.
- [ ] Review headings, code blocks, tables, search terms, and page metadata.
- [ ] Record a deployment and rollback procedure.

**Acceptance criteria**

- [ ] The site builds reproducibly from a clean checkout.
- [ ] Core guidance works at narrow and wide viewport sizes.
- [ ] Missing optional evidence produces an explicit static fallback.

## Validation

```text
python -m mkdocs build --strict
cargo test --workspace --all-targets
manual keyboard and mobile navigation review
internal-link and accessibility scan
```

## Completion Criteria

This plan completes when the public site has a clear reader journey, the full
engineering record remains available through GitHub, all visible navigation is
operable, one bounded evidence page demonstrates the site/provider model, and
site quality checks run reproducibly.
