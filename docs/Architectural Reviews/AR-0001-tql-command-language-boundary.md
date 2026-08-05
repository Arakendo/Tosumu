# AR-0001: TQL Command Language Boundary

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-03 |
| Last reviewed | 2026-08-04 |
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
  tested independently. TQL now has bounded parser tests, read-only dispatch
  tests, and text/JSON rendering tests for `STATUS`, `CHECK`, `DESCRIBE`, and
  `WAL STATUS`.
- **Independent consumers:** The CLI is the first intended caller. A reusable
  Rust caller, .NET adapter, or another consumer has not yet exercised TQL.
- **Diagnostics or audits:** Existing inspect and verify surfaces already
  produce some of the facts needed by `STATUS`, `CHECK`, `DESCRIBE`, and
  `WAL STATUS`.
- **Repeated implementation friction:** The initial operational commands can
  reuse public storage and verification facts without pretending to be SQL.
  No honest SQL lowering target exists yet because virtual views are absent.
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

- [x] Implement and test a bounded TQL parser and command AST.
- [x] Exercise `STATUS`, `CHECK`, `DESCRIBE`, and `WAL STATUS` through existing public
      inspection and verification behavior.
- [x] Preserve structured outcomes separately from CLI rendering.
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

### Cycle 2 -- 2026-08-03

- **Status entering review:** Incubating.
- **New evidence:** `tosumu-cli` now parses bounded `STATUS`, `CHECK`, and
  `DESCRIBE <key>` statements into typed commands, dispatches them through
  public read-only storage and verification APIs, and renders one structured
  outcome as human text or provisional JSON schema version `1`. Tests retain
  database/WAL byte equality across read-only dispatch and exercise a tampered
  page result.
- **Findings:** CLI file opening and rendering can remain outside parser and
  dispatcher semantics. `DESCRIBE` can report presence and byte length without
  disclosing value contents. The first operational commands do not prove an
  SQL-lowering path, reusable public API, or a dedicated crate.
- **Disposition:** Continue incubation in `tosumu-cli`.
- **Resulting ADR or documentation change:** Updated the TQL design and plan
  to distinguish the implemented read-only subset from trust, freshness, sync,
  SQL-view, and mutation work that remains deferred.

### Cycle 3 -- 2026-08-03

- **Status entering review:** Incubating.
- **New evidence:** The trust and explanation inventory compared the public
  verification report and key lookup with the design's integrity, freshness,
  provenance, witness, conflict, and recommendation dimensions.
- **Findings:** Database-wide page verification and key presence are real but
  insufficient for a key-scoped trust verdict. No public evidence source yet
  grounds freshness, witnesses, provenance, conflicts, or typed next actions.
- **Disposition:** Keep `TRUST <key>` and `WHY <key>` deferred; do not make
  unavailable fields look like a completed trust model.
- **Resulting ADR or documentation change:** Added `TQL Trust And Explanation
  Evidence Inventory v1` and recorded its reopening triggers in the plan.

### Cycle 4 -- 2026-08-03

- **Status entering review:** Incubating.
- **New evidence:** The read-only CLI now distinguishes a completed `CHECK`
  with reported integrity failures from successful observations through the
  existing reported-issues process status. A focused unit test proves that
  only a failed page or B-tree dimension takes that path.
- **Findings:** Process classification can remain an adapter concern layered
  over a provider-neutral typed outcome. Missing-key `DESCRIBE` remains a
  successful observation, while parser, storage-open, and I/O failures remain
  structured CLI errors.
- **Disposition:** Continue incubation in `tosumu-cli`; no public TQL ABI or
  dedicated crate is implied.
- **Resulting ADR or documentation change:** Updated the command plan and TQL
  design to document the read-only exit classification.

### Cycle 5 -- 2026-08-03

- **Status entering review:** Incubating.
- **New evidence:** `WAL STATUS` now consumes the public `inspect_wal()`
  summary and emits only WAL sidecar existence plus decoded record count. The
  read-only dispatch test retains database and WAL byte equality before and
  after the command; CLI JSON output omits physical sidecar paths.
- **Findings:** WAL observation is an operational inspection command, not an
  honest SQL-lowering target. Public record-count facts do not justify claims
  about recovery success, checkpoint posture, durability, freshness, trust, or
  synchronization.
- **Disposition:** Continue incubation in `tosumu-cli`.
- **Resulting ADR or documentation change:** Updated the command corpus, TQL
  design, and command plan to name the bounded WAL observation explicitly.

### Cycle 6 -- 2026-08-03

- **Status entering review:** Incubating.
- **New evidence:** Arbitrary UTF-8 input is property-tested for deterministic,
  non-panicking parse behavior under declared command, token, and key limits.
  Focused CLI tests prove an invalid statement fails before the database path is
  opened and that a maximum-size unknown token is emitted once in structured
  JSON details rather than duplicated in the human-facing message.
- **Findings:** Input boundedness is a parser and CLI-boundary concern; it does
  not require TQL to own storage authentication, recovery, or mutation policy.
  A common CLI error payload can serve both inspect and TQL envelopes without
  making their command schemas the same contract.
- **Disposition:** Continue CLI-local incubation. The remaining hardening work
  is fuzz-target tooling, malformed byte-input evidence if a byte API is ever
  admitted, and performance observations rather than another command family.
- **Resulting ADR or documentation change:** Updated the plan's Slice 7
  checklist and documented shared error-payload ownership in the CLI adapter.

### Cycle 7 -- 2026-08-04

- **Status entering review:** Incubating.
- **New evidence:** The repository now has a parser-only `cargo-fuzz` target
  that includes the CLI-local grammar without opening a store or invoking
  dispatch. Standard-toolchain property tests also exercise bounded
  `DESCRIBE` JSON rendering. The target compiles under nightly locally; its
  Windows execution is blocked by the unavailable sanitizer runtime, so a
  bounded weekly/manual Linux workflow owns actual libFuzzer execution.
- **Findings:** Fuzz execution belongs to a dedicated nightly Linux validation
  boundary, not the stable cross-platform build matrix. TQL can receive
  malformed-input evidence without making its parser public or admitting a
  general byte-input API.
- **Disposition:** Continue CLI-local incubation. Treat successful Linux fuzz
  runs as parser evidence only; structured-rendering fuzz and a second
  independent consumer remain open evidence gaps.
- **Resulting ADR or documentation change:** Added `fuzz_tql_parse`, the
  Linux fuzz workflow, and explicit platform-limit documentation in the TQL
  command corpus and implementation plan.

### Cycle 8 -- 2026-08-04

- **Status entering review:** Incubating.
- **New evidence:** The operator reference was reconciled with the actual
  CLI-local dispatch surface. `STATUS` is now documented as a direct
  observation of `KvStore::stat()` only: page count, data-page count, and tree
  height. `DESCRIBE <key>` currently obtains public value length by loading
  the public value through `KvStore::get()` and discarding its contents.
  `WAL STATUS` remains limited to sidecar existence and decoded record count.
- **Findings:** Store identity, format metadata, durable metadata-only lookup,
  recovery, checkpoint posture, freshness, trust, synchronization, and
  semantic explanation are not established by the current commands. A
  CLI-local physical-page scanner would bypass the provider boundary, so a
  provider-neutral metadata lookup remains deferred until an independent
  consumer proves that contract is needed.
- **Validation:** Formatting and workspace clippy completed successfully;
  focused `tosumu-cli` tests passed (112 unit tests plus 1 integration test).
  A full workspace test attempt progressed through the complete CLI suite and
  substantial core coverage, but exceeded the local time budget in existing
  expensive crypto/property coverage; this is a validation gap, not a test
  failure. Strict MkDocs validation is also unavailable in the local Python
  environment because the `mkdocs` module is not installed there.
- **Disposition:** Continue CLI-local incubation. Do not extract a public TQL
  crate or add a new command family until an independent caller creates
  evidence for a stable semantic boundary.
- **Resulting ADR or documentation change:** Added the admitted operator
  reference and bounded-description observation note to the TQL design and
  implementation plan.

### Cycle 9 -- 2026-08-04

- **Status entering review:** Incubating.
- **New evidence:** A disclosure audit now records the permitted output facts
  for every implemented command. A renderer test uses a stored-value sentinel
  and proves that neither human nor JSON `DESCRIBE` output can contain it;
  existing WAL output coverage retains the physical-path exclusion.
- **Findings:** The initial TQL syntax accepts no unlock material, protector
  data, or binary payload. Requested keys and rejected command tokens remain
  caller-provided diagnostic text, not protected TQL fields. This is a bounded
  renderer property only; it does not prove provider errors from future
  encrypted or protected storage are safe to serialize.
- **Disposition:** Keep the read-only disclosure boundary CLI-local. Require a
  fresh disclosure review before any secret-bearing, mutation, byte-input, or
  protected-metadata command is admitted.
- **Resulting ADR or documentation change:** Added `TQL Disclosure Audit v1`
  and marked the implemented-renderer audit complete in the TQL plan.

### Cycle 10 -- 2026-08-04

- **Status entering review:** Incubating.
- **New evidence:** Successful outcome rendering now lives in a pure
  CLI-local module, separately from storage opening, dispatch, and the common
  CLI error envelope. `fuzz_tql_render` synthesizes every admitted outcome
  family with bounded facts and checks deterministic human/JSON rendering,
  schema marking, and bounded output. The parser and renderer fuzz targets
  compile under the nightly toolchain; sustained libFuzzer execution remains
  pending the sanitizer-capable Linux workflow.
- **Findings:** The current four-command outcome vocabulary is useful enough
  to test independently of terminal formatting, but no non-CLI caller has yet
  shown that it is a stable reusable API. Timing observations are diagnostic
  stderr evidence, not TQL result fields or performance guarantees.
- **Disposition:** Continue CLI-local incubation. Do not extract
  `tosumu-tql`, add a public byte-input interface, or admit trust, SQL-view,
  sync, or mutation commands from this evidence.
- **Resulting ADR or documentation change:** Reconciled the SDD with the
  implemented subset and added the plan's explicit parked-command register.

### Cycle 11 -- 2026-08-04

- **Status entering review:** Incubating.
- **New evidence:** Tokimu's Tier 3 `tosumu-tql-cli-consumer` created a
  temporary database through public CLI commands, then invoked the TQL CLI as
  an external process. It consumed only schema-versioned JSON for `STATUS`,
  `CHECK`, present and missing `DESCRIBE`, `WAL STATUS`, and a typed invalid
  command. The consumer rejects an unknown schema version and does not parse
  TQL, link Tosumu crates, or inspect storage.
- **Findings:** The provisional executable/JSON contract is independently
  consumable without provider-native leakage. No divergence appeared in the
  admitted fixture corpus. This proves compatibility at the process boundary,
  not a reusable in-process parser, dispatcher, or outcome API.
- **Disposition:** Continue CLI-local incubation. Keep schema version `1`
  provisional, classify future consumer differences as presentation-only,
  contract refinement, or rejected behavior, and defer `tosumu-tql` extraction
  until independent in-process reuse creates stronger evidence.
- **Resulting ADR or documentation change:** Updated Slice 8 of the TQL plan
  with the completed compatibility evidence and its version policy. No ADR or
  public crate was admitted.
