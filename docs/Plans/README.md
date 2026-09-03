# Implementation Plans

Plans describe concrete Tosumu implementation work: scope, ownership,
incremental slices, validation, risks, and completion criteria.

Plans do not create architectural authority. When implementation evidence
changes an accepted boundary, open an Architectural Review and update or
supersede the relevant ADR.

Copy [`TEMPLATE.md`](TEMPLATE.md) when opening a plan. Remove sections that are
genuinely irrelevant, but keep per-slice deliverables, acceptance criteria,
validation, and exit states.

## Current And Retained Plans

- [Main Feature Roadmap](main-feature-roadmap.md) -- canonical implementation
  status tracker; **Active**. MVP 0-8 and the MVP+9 baseline are complete;
  the MVP+10 baseline is now the active planning gate.
- [MVP+10 Multiple Readers And Coordination](mvp-10-multiple-readers.md) --
  **Proposed; baseline recorded** for current handle, visibility, writer, and
  checkpoint behavior under AR-0009.
- [MVP+10 Secondary Indexes](mvp-10-secondary-indexes.md) -- implements the
  ADR-0008 SQL-owned ordered index representation after reader visibility and
  conditional writes.
- [MVP+10 VACUUM](mvp-10-vacuum.md) -- implementation closure recorded for the
  ADR-0009 offline verified rebuild; native Unix CI confirmation remains open.
- [Initial SQL Layer](initial-sql-layer.md) -- MVP+9 baseline complete; retained
  as **Completed baseline** implementation history and a source for deferred
  SQL work.
- [Tosumu Command Language](tosumu-command-language.md) -- proposed sliced
  implementation; **Proposed**. Ownership remains under AR-0001.
- [Tokimu Reciprocal Website Evidence](tokimu-reciprocal-website-evidence.md)
  -- **Proposed** cross-project website evidence using versioned public
  observations and independently deployable MkDocs sites.
- [Tosumu Inspection Island And UI Providers](tosumu-inspection-island-and-ui-providers.md)
  -- **Proposed** provider-neutral inspection observations and commands shared
  by native Ratatui and TypeScript website-island providers.
- [Public Website And Repository Records](public-website-and-repository-records.md)
  -- **Proposed** public information architecture, publication policy, indexed
  navigation, and GitHub engineering-record boundary.
- [Documentation Lifecycle And Design Decomposition](documentation-lifecycle-and-design-decomposition.md)
  -- **Active** normalization of document status and the current/future design
  boundary.
- [Core Source Unit Decomposition](core-source-unit-decomposition.md) --
  **Complete** behavior-preserving test and private-module decomposition
  triggered by ADR-0003's initial core inventory.

## Plan Requirements

A useful plan should include:

- motivating evidence and current state;
- goals and explicit non-goals;
- ownership and dependency boundaries;
- small compiling implementation slices;
- acceptance criteria for each slice;
- tests, fuzzing, fixtures, or consumer evidence;
- risks, unsupported cases, and diagnostics;
- completion, graduation, or parking criteria;
- links to related specifications, ADRs, reviews, CRs, and code.
