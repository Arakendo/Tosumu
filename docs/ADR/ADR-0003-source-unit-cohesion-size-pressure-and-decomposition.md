# ADR-0003: Source Unit Cohesion, Size Pressure, And Decomposition

## Status

Accepted

## Context

Tosumu keeps storage behavior inspectable by making ownership, failure, and
compatibility boundaries explicit. That becomes harder when one hand-maintained
source unit accumulates several independently testable responsibilities, even
if its public API remains small.

At acceptance, the workspace contains core source units large enough to require
deliberate review. `page_store.rs`, `pager.rs`, and `wal.rs` exceed 2,000
physical lines, while `btree.rs`, `inspection_session.rs`, and `inspect.rs`
exceed 1,000. Line count does not prove that any of these files is incorrectly
structured, but it is useful evidence that cohesion should be examined before
more behavior is added.

An arbitrary maximum would encourage numbered fragments, weak helper modules,
or premature public abstractions. Tosumu instead needs a proportional rule that
uses size to find pressure and responsibility to decide whether decomposition
is warranted.

## Decision

Hand-maintained source units must represent one coherent implementation
responsibility. Size and responsibility signals trigger an explicit cohesion
review. A review may retain a cohesive unit, require behavior-preserving
decomposition, or defer extraction to a named safe checkpoint.

### Size review triggers

Physical line count is an inexpensive review signal, not a quality score or an
automatic merge failure.

| Hand-maintained source size | Required treatment |
| --- | --- |
| Up to 1,000 lines | Ordinary; no size justification is required. Cohesion may still require review. |
| 1,001–2,000 lines | Inspect cohesion during a substantive modification. |
| 2,001–4,000 lines | Perform and retain an explicit decomposition review. |
| More than 4,000 lines | Presume decomposition debt; retain only with a documented cohesion or sequencing reason. |
| More than 8,000 lines | Exceptional; active work should normally include a checkpointed decomposition campaign. |

Comments, inline tests, and documentation in an implementation file count
because they contribute to the same navigation and review burden. CI may report
threshold crossings, but it must not fail solely because of line count unless a
later decision admits a demonstrated mechanical gate.

### Responsibility review triggers

A decomposition review is required regardless of line count when a source unit
shows two or more of these conditions:

- it contains multiple independently testable subsystems;
- it mixes physical storage mechanics with SQL, CLI, UI, provider, or consumer
  meaning;
- it mixes stable behavior with unresolved experiments or compatibility
  candidates;
- it contains several unrelated diagnostic or reporting paths;
- ordinary changes routinely touch distant, unrelated regions;
- tests require navigating substantial unrelated implementation;
- the filename no longer communicates a useful owner or responsibility;
- a responsibility can move behind a private module without changing a stable
  contract; or
- multiple plans, reviews, or format concerns independently modify the unit.

Crossing a size threshold and satisfying one responsibility trigger is also
sufficient to require review.

### Decomposition follows ownership seams

Every extracted unit must name its subject and implementation responsibility.
Preferred responsibility vocabulary includes `contract`, `state`,
`preparation`, `lowering`, `adapter`, `diagnostics`, `fixture`, and `tests`, but
these are descriptive terms rather than a required file template.

The smallest sufficient visibility is preferred:

1. a private child module in the same crate;
2. crate-visible code when multiple local modules require it;
3. an existing public abstraction when the responsibility already belongs
   there; and
4. a new public API or crate only after independent callers and architectural
   evidence justify it.

Moving code does not change ownership. In particular, decomposition must not
move SQL, table, CLI, UI, or consumer semantics into `tosumu-core`, expose pager
or on-disk objects to consumers, or create a new compatibility boundary merely
to shorten a file.

Before accepting a proposed seam, the review must be able to state:

- its subject and responsibility;
- the state, policy, or authority it owns;
- the inputs it consumes;
- the outputs or observations it produces;
- the responsibilities it explicitly excludes; and
- its dependency direction relative to sibling modules.

### Successful extraction reduces coupling

Smaller files are not sufficient evidence. A decomposition is unsuccessful if
the extracted modules still require most of the former unit's state, reach
through sibling internals, form conceptual cycles, or share a broad mutable
context that hides the original ownership ambiguity.

Reviews must inspect dependency direction, state access, testability through
the named responsibility, and whether the parent is being used only to disguise
a cycle.

### Conservation requirements

Behavior-preserving decomposition must retain:

- public APIs and visibility unless a separate accepted decision changes them;
- architectural ownership and dependency direction;
- on-disk bytes, format compatibility, and migration behavior;
- input, output, ordering, durability, and lifecycle behavior;
- typed errors, diagnostic identity, and source provenance;
- authentication and integrity boundaries;
- native and WASM behavior where applicable;
- focused regression fixtures, deterministic artifacts, and known
  falsifications; and
- performance characteristics that are material to the affected storage path.

Mechanical extraction must not quietly repair, suppress, or reinterpret a
known defect. Any semantic correction discovered during extraction is made as
a separate reviewable change after the conservation baseline is restored.

Tests may move into private module test files, but they remain organized around
named invariants. Splitting tests must not replace focused assertions with only
broad snapshots or duplicate production semantics in helpers.

### Active work and exceptions

When a large unit is under active semantic, recovery, or format work:

1. reach and record a coherent checkpoint;
2. retain the passing tests, fixtures, observations, and known failures;
3. perform behavior-preserving extraction separately;
4. rerun the same evidence after each coherent extraction group; and
5. resume semantic work only after the checkpoint is reproduced.

The graduated thresholds do not apply directly to generated code, vendored
source, machine-produced bindings, static lookup tables, exact corpus artifacts,
or cohesive declarative schemas. A retained exception must identify its
category and explain why the unit is more coherent intact.

An explicit decomposition review records the line count and responsibility
inventory, proposed seams, dependency direction, conservation evidence,
disposition, and reopening triggers. A multi-step campaign belongs in a plan;
the plan must distinguish mechanical moves from semantic changes.

## Consequences

- Large and contested source units receive review before ownership and
  attribution become opaque.
- Private modules can improve isolation without manufacturing public APIs or
  changing the storage-engine boundary.
- Storage, format, authentication, and recovery evidence must survive
  structural work unchanged.
- Some cohesive files will remain large with an explicit reason.
- Reviews and conservation reruns add work, and poorly chosen seams may expose
  coupling that requires a different decomposition.

This decision requires explicit review of `page_store.rs`, `pager.rs`, and
`wal.rs`. The initial dispositions and any resulting work are retained in the
core source-unit decomposition plan. The decision does not predetermine their
final module shape or require an unrelated semantic refactor.

## Alternatives Considered

- **No shared rule.** Rejected because recurring storage work can continue to
  concentrate unrelated responsibilities without a review point.
- **A hard maximum line count.** Rejected because size is not responsibility and
  a hard cap encourages arbitrary fragmentation.
- **A size report without architectural meaning.** Rejected because it locates
  pressure but cannot decide whether a unit should be retained or decomposed.
- **Graduated cohesion review.** Accepted because it uses size as evidence while
  preserving ownership, compatibility, and behavior as the deciding concerns.

## Reopening Triggers

Revisit this decision if reviews become ritual, thresholds cause arbitrary
fragmentation, generated or data-heavy units are repeatedly misclassified,
private extraction repeatedly requires accidental public APIs, or conservation
checks fail to protect storage and format behavior.

## References

- `ADR-0001-storage-engine-layer-boundaries.md`
- `ADR-0002-authenticated-pager-trust-boundary.md`
- `../Specifications/Tosumu Software Design Document.md`
- `../Specifications/Tosumu Error Design Document.md`
- `../Plans/core-source-unit-decomposition.md`
- `../Plans/documentation-lifecycle-and-design-decomposition.md`
