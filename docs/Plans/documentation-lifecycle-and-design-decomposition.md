# Documentation Lifecycle And Design Decomposition Plan

| Field | Value |
| --- | --- |
| Status | Active |
| Opened | 2026-08-03 |
| Last updated | 2026-08-03 |
| Owner | Tosumu maintainers |
| Authority | Tracking plan; engineering specifications and accepted ADRs remain authoritative |
| Target | Clear document lifecycle, stable source-of-truth entry points, and a bounded `docs/Specifications/Tosumu Software Design Document.md` |

## Purpose

Make it immediately clear which Tosumu documents are current contracts,
accepted decisions, active work, speculative proposals, informative notes, or
historical records. Reduce the mixed lifecycle content in `docs/Specifications/Tosumu Software Design Document.md` without
breaking established links or creating a second architectural source of truth.

## Current Evidence

- The engineering specifications are normative and referenced by code, tests,
  public docs, and contributor guidance.
- Governance folders distinguish document purpose but did not provide one
  lifecycle dashboard.
- `docs/Specifications/Tosumu Software Design Document.md` combines implemented architecture, roadmap tracking, deferred
  mechanisms, and long-range Stage 7+ proposals.
- Completed plans and accepted change requests can look active when their
  collection indexes are not updated.
- Moving the design, error, inspection, and reference documents into one
  specification collection required an atomic update to dozens of code, test,
  CI, contributor, and documentation references.

## Non-Goals

- Moving the repository-root `SECURITY.md`; its conventional location supports
  security tooling and responsible-disclosure discovery.
- Treating MkDocs summaries as an alternate specification.
- Converting every observation into an ADR or active plan.
- Deleting historical reasoning merely to shorten the documentation tree.
- Declaring speculative design implemented because it appears in `docs/Specifications/Tosumu Software Design Document.md`.

## Slice 0: Establish Lifecycle Vocabulary

### Deliverables

- [x] Add a documentation status dashboard.
- [x] Define authority independently from lifecycle.
- [x] Inventory normative specifications, decisions, reviews, plans, requests,
      and supporting records.
- [x] Correct collection indexes whose labels disagree with document status.

### Acceptance Criteria

- [x] A reader can identify current, active, proposed, incubating, completed,
      and historical material from one page.
- [x] Accepted architecture remains distinguishable from implementation plans.
- [x] Normative entry points remain explicit and all moved paths are updated
      atomically.

## Slice 1: Normalize Status Metadata

### Deliverables

- [x] Add an authority and lifecycle block to every normative specification
      and the informative reference index.
- [ ] Add or normalize status metadata on every public design proposal, plan,
      review, and change request.
- [ ] Require opened/updated dates and a next action for active or proposed
      work.
- [ ] Require successor links for superseded records.

### Acceptance Criteria

- [x] Specification lifecycle no longer needs to be inferred from prose.
- [ ] No plan, review, or request lifecycle must be inferred solely from
      unchecked boxes or prose.
- [ ] `Draft` is never used as a substitute for authority or implementation
      status.
- [ ] Collection indexes and individual status blocks agree.

## Slice 2: Classify `docs/Specifications/Tosumu Software Design Document.md`

### Deliverables

- [ ] Label each major section as current contract, current rationale, roadmap,
      deferred design, or speculative direction.
- [ ] Link accepted decisions to their ADRs and unresolved boundaries to their
      Architectural Reviews.
- [ ] Remove duplicated delivery tracking in favor of the main feature roadmap.
- [ ] Identify sections that can become focused normative specifications
      without changing their meaning.

### Acceptance Criteria

- [ ] A reader can distinguish implemented architecture from Stage 7+ ideas.
- [ ] No accepted guarantee is weakened or silently moved to informative prose.
- [ ] Every extracted section retains a stable link or explicit compatibility
      note from `docs/Specifications/Tosumu Software Design Document.md`.

## Slice 3: Extract Focused Specifications Incrementally

### Deliverables

- [ ] Extract only sections with proven independent ownership and repeated
      references.
- [ ] Keep `docs/Specifications/Tosumu Software Design Document.md` as the architecture map and link to focused normative
      specifications.
- [ ] Update code comments, tests, contributor guidance, and public summaries
      in the same change as each extraction.
- [ ] Add redirects or compatibility anchors where practical.
- [x] Move the design, error, inspection, and reference documents into
      `docs/Specifications/`, add a collection index, and update every known
      repository reference.
- [x] Keep `SECURITY.md` at the repository root as the conventional security
      policy and disclosure entry point.

### Acceptance Criteria

- [ ] Each fact has one normative owner.
- [ ] Public documentation does not become a parallel source of truth.
- [ ] Strict MkDocs validation and repository link checks pass after every
      extraction.

## Slice 4: Retire And Archive Deliberately

### Deliverables

- [ ] Review completed plans for retained value and archive only support
      material whose active role has ended.
- [ ] Mark completed change requests accurately and preserve consumer evidence.
- [ ] Move obsolete notes or conversations to `Archive/` with reason, date,
      and replacement.
- [ ] Keep superseded ADRs in `ADR/` with successor metadata.

### Acceptance Criteria

- [ ] Nothing historical appears active.
- [ ] Nothing binding disappears into the archive.
- [ ] Archived records explain why they remain useful.

## Validation

- `py -m mkdocs build --strict`
- repository link/reference search for every moved or renamed document
- review of `AGENTS.md`, `README.md`, public docs, and collection indexes
- `git diff --check`

## Exit State

This plan completes when document authority and lifecycle are explicit, the
current versus speculative boundary in `docs/Specifications/Tosumu Software Design Document.md` is visible, and any focused
specification extraction leaves one unambiguous normative owner per contract.

## Progress Log

- 2026-08-03: Established the lifecycle vocabulary and document status
  dashboard.
- 2026-08-03: Added authority and lifecycle metadata to the specifications
  and classified `docs/Specifications/Tosumu Reference Implementations.md` as informative rather than normative.
- 2026-08-03: Consolidated design, error, inspection, and reference documents
  under `docs/Specifications/`; updated code, tests, CI, contributor guidance,
  public summaries, and MkDocs navigation while retaining root `SECURITY.md`.
