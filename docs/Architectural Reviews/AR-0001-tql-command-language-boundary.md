# AR-0001: TQL Command Language Boundary

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-03 |
| Last reviewed | 2026-08-03 |
| Scope | Relational layer / operator surface / CLI / cross-cutting |
| Trigger | The TQL design proposes one command surface over SQL queries, inspection, verification, and future sync operations |
| Related ADRs | [ADR-0001: Storage Engine Layer Boundaries](../ADR/ADR-0001-storage-engine-layer-boundaries.md) |
| Related evidence | [TQL design](../Tosumu%20Command%20Language.md), [implementation plan](../Plans/tosumu-command-language.md), initial SQL implementation |

## Architectural Question

Where should Tosumu Command Language syntax, command meaning, SQL lowering,
operational dispatch, and result shaping live so TQL remains a thin,
embeddable operator surface without becoming a second execution engine or
teaching `tosumu-core` command-language semantics?

## Context

TQL is intended to explain and operate on Tosumu's evidence-bearing state.
Some proposed commands, such as `STALE`, are naturally query sugar over future
SQL virtual views. Others, such as `CHECK` and `SYNC PREVIEW`, are operational
requests over inspection, verification, or sync APIs.

The current workspace already establishes two important boundaries:

- `tosumu-core` owns physical storage, integrity, recovery, and bounded
  inspection without SQL or CLI meaning;
- `tosumu-sql` owns the initial relational parser, semantic checker, planner,
  and executor over public storage behavior.

The CLI currently depends on both crates and renders operator-facing output.
Putting TQL directly into any one of those layers could accidentally make that
layer own the whole command surface.

## Evidence

- **Tests or fuzzing:** The SQL lexer/parser/planner and CLI dispatch paths are
  tested independently. No TQL parser or execution evidence exists yet.
- **Independent consumers:** The CLI is the first intended caller. A reusable
  Rust caller, .NET adapter, or another consumer has not yet exercised TQL.
- **Diagnostics or audits:** Existing inspect and verify surfaces already
  produce some of the facts needed by `STATUS`, `CHECK`, and `DESCRIBE`.
- **Repeated implementation friction:** Not yet observed. The design document
  predicts mixed SQL and operational lowering, but implementation must verify
  that split.
- **Missing evidence:** SQL virtual views, semantic change history, conflict
  metadata, witness-backed freshness, and sync preview/apply services are not
  all implemented. TQL must not synthesize those guarantees.

## Ownership And Dependency Analysis

The provisional ownership model is:

```text
TQL text
    ↓
TQL syntax and command classification
    ├── honest SQL lowering → tosumu-sql
    └── operational request → public inspect / verify / future sync API
                                      ↓
                                  tosumu-core

CLI / Rust / future adapter
    ↑
structured TQL outcome
```

- TQL owns its grammar, command AST, lowering classification, and structured
  command outcome.
- SQL owns SQL parsing, relational semantics, planning, and execution.
- Core inspection, verification, and future sync capabilities own the facts
  and operations TQL exposes.
- CLI and other frontends own text, JSON, TUI, or platform-specific rendering.
- TQL must not own pager, B+ tree, WAL, cryptographic, or physical-format
  mechanics.
- `tosumu-core` and `tosumu-sql` must not depend upward on TQL or CLI.

Whether TQL should graduate into a dedicated `tosumu-tql` crate remains under
review. The crate boundary is plausible because TQL is intended to be
embeddable, but it has not yet earned a stable public API.

## Alternatives Considered

### Alternative A: Dedicated Thin TQL Crate

- **Benefits:** Reusable outside the CLI; structurally prevents CLI rendering
  from becoming language semantics; can depend downward on SQL and public
  operational APIs.
- **Costs:** Adds a crate and public boundary before a second caller exists.
- **Failure mode:** The crate becomes a second planner/executor or a bag of
  forwarding abstractions with no independent value.

### Alternative B: Incubate TQL Inside `tosumu-cli`

- **Benefits:** Smallest first implementation; one concrete caller; easy to
  revise while grammar and output are unsettled.
- **Costs:** Encourages parser, command semantics, and presentation to become
  entangled; later extraction may expose accidental CLI assumptions.
- **Failure mode:** TQL becomes impossible to use from Rust, .NET, or another
  operator frontend without invoking CLI-shaped behavior.

### Alternative C: Put TQL In `tosumu-sql`

- **Benefits:** Direct access to SQL AST and execution; natural for commands
  that lower to virtual views.
- **Costs:** Makes SQL own operational inspection, verification, and sync
  commands that are not relational queries.
- **Failure mode:** Operational workflows are distorted into fake SQL, or the
  SQL crate becomes a general Tosumu shell runtime.

### Alternative D: Put TQL In `tosumu-core`

- **Benefits:** Direct access to every fact and operation.
- **Costs:** Violates ADR-0001 and introduces command-language semantics into
  the physical storage layer.
- **Failure mode:** Storage mechanics become coupled to operator vocabulary and
  higher-level policies.

### Alternative E: Continue Design-Only Incubation

- **Benefits:** Avoids premature API stabilization while required trust and
  sync facts remain unavailable.
- **Costs:** Delays useful operator access to inspection behavior that already
  exists.
- **Failure mode:** The design grows without implementation evidence and
  accumulates commands Tosumu cannot honestly support.

## Findings

- TQL must remain above `tosumu-core` and beside, not inside, SQL semantics.
- SQL lowering and operational dispatch are distinct paths that may share one
  TQL surface but must preserve their owning subsystem's meaning.
- Structured results must be separate from CLI rendering so the language can
  remain embeddable.
- The first implementation should expose only evidence Tosumu can currently
  produce. Missing witness, conflict, or sync facts must yield explicit
  unsupported or unanchored states.
- A dedicated crate is a strong candidate but lacks independent-consumer
  evidence.

## Disposition

**Incubating.** Proceed with the bounded parser and read-only inspection slices
in the related implementation plan. Do not stabilize a universal command
execution trait, virtual-view ABI, or mutation surface yet. Revisit crate
ownership after the first CLI caller and one non-CLI caller exercise the same
structured command contract.

## Required Follow-Up

- [ ] Implement and test a bounded TQL parser and command AST.
- [ ] Exercise `STATUS`, `CHECK`, and `DESCRIBE` through existing public
      inspection and verification behavior.
- [ ] Preserve structured outcomes separately from CLI rendering.
- [ ] Add one non-CLI caller or document an evidence substitution.
- [ ] Prove at least one honest SQL lowering after virtual-view semantics exist.
- [ ] Revisit the dedicated-crate boundary.
- [ ] Create ADR-0002 only if the review accepts a durable TQL ownership and
      dependency decision.

## Reopening Triggers

- A second frontend needs reusable TQL parsing or execution.
- SQL virtual views provide the first query-sugar lowering target.
- A command requires logic duplicated between SQL and an operational API.
- Sync, witness, conflict, or freshness capabilities expose stable public
  semantics.
- The first implementation reveals that structured outcomes cannot remain
  independent of CLI rendering.

## Review History

### Cycle 1 -- 2026-08-03

- **Status entering review:** Proposed
- **New evidence:** Existing SQL and inspect boundaries were compared with the
  TQL design; no TQL implementation exists yet.
- **Findings:** A thin mixed-lowering surface is plausible, core ownership is
  rejected, and dedicated-crate graduation needs implementation pressure.
- **Disposition:** Incubating
- **Resulting ADR or documentation change:** No ADR yet. Implementation is
  governed by `docs/Plans/tosumu-command-language.md`.

