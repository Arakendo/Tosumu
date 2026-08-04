# Tosumu Command Language Implementation Plan

| Field | Value |
| --- | --- |
| Status | Proposed |
| Opened | 2026-08-03 |
| Last updated | 2026-08-03 |
| Owner | Tosumu maintainers |
| Target | TQL parser, structured command surface, SQL lowering, and operator integration |
| Related ADRs | [ADR-0001: Storage Engine Layer Boundaries](../ADR/ADR-0001-storage-engine-layer-boundaries.md) |
| Related reviews | [AR-0001: TQL Command Language Boundary](../Architectural%20Reviews/AR-0001-tql-command-language-boundary.md) |
| Related CRs | None |
| Depends on | Existing inspect/verify APIs, initial SQL layer, later virtual views and sync semantics |

## Status

TQL currently exists as a design document only. Tosumu has a working initial
SQL crate, public storage and inspection behavior, and CLI rendering paths, but
it has no TQL parser, command AST, structured outcome, virtual views, or sync
command implementation. The ownership boundary is under review in AR-0001.

## Purpose

Implement the smallest honest TQL surface that lets operators inspect and
understand Tosumu without creating a second query engine or embedding CLI
presentation into database semantics.

The work should prove this composition:

```text
operator command
    ↓
bounded TQL parser
    ↓
typed command
    ├── SQL lowering when relational meaning is preserved
    └── public operational API when SQL would distort meaning
    ↓
structured outcome
    ↓
CLI text / JSON / future frontend
```

The plan intentionally begins with inspection commands Tosumu can support
today. Trust, freshness, conflict, witness, and sync commands advance only when
their source capabilities can provide evidence-backed answers.

## Trigger And Evidence

- The TQL design identifies repeated operator questions around integrity,
  provenance, freshness, and sync.
- Existing CLI commands already expose some related facts through separate,
  command-specific paths.
- The initial SQL layer provides a tested parser/planner/executor but does not
  own operational inspection or sync semantics.
- Tosumu's design requires honest reporting: unanchored freshness must never be
  presented as fresh, and unsupported evidence must not be invented by a shell.
- Future CLI, Rust, .NET, and consumer tooling need structured results rather
  than screen-scraped prose.

Observed today:

- SQL can parse, plan, explain, and execute its supported subset.
- Core and CLI inspection paths can report database structure and integrity
  evidence.
- Witness-backed freshness, general conflict views, semantic sync planning,
  and all proposed virtual views are incomplete or deferred.

Not guaranteed today:

- the final TQL grammar;
- a dedicated `tosumu-tql` crate;
- stable virtual-view names or schemas;
- sync command availability;
- mutation through TQL;
- a stable serialized TQL outcome schema.

## Current State

- `docs/Tosumu Command Language.md` defines the exploratory command families,
  safety rules, SQL relationship, and suggested MVP.
- `crates/tosumu-sql` owns the initial SQL AST and execution pipeline.
- `crates/tosumu-cli` owns command dispatch and human-readable rendering.
- `tosumu-core` owns public storage, inspection, integrity, recovery, and
  provider contracts under ADR-0001.
- No command parser or reusable operator outcome sits between those layers.

## Goals

- Define a small, bounded, deterministic TQL grammar and typed command AST.
- Separate parsing, semantic dispatch, structured outcomes, and presentation.
- Implement useful read-only commands over facts Tosumu already owns.
- Lower query-shaped commands to SQL only when the lowering is semantically
  honest and observable.
- Keep operational commands thin over public inspect, verify, and future sync
  APIs.
- Provide typed errors, bounded diagnostics, and machine-readable outcomes.
- Use implementation and consumer evidence to resolve AR-0001.

## Non-Goals

- A replacement for SQL.
- A general scripting, stored-procedure, graph-query, or pipeline language.
- A second planner or executor.
- Full trust, witness, freshness, provenance, conflict, or sync implementation.
- `SYNC APPLY` in the initial read-only implementation.
- A network protocol, peer discovery mechanism, or transport.
- Interactive `WATCH`, `DOCTOR`, or shell history in the first slices.
- Stable public grammar or serialized ABI before compatibility evidence.
- Any change to Tosumu's physical file or WAL format.

## Ownership And Dependency Boundary

### This Work Owns

- TQL lexical and syntactic rules.
- A typed command model and command-family classification.
- The decision to lower a supported command to SQL or dispatch it to a public
  operational capability.
- Structured command outcomes and TQL-specific errors.
- Bounded command-level diagnostics explaining unsupported evidence.

### This Work Must Not Own

- SQL parsing, relational planning, or SQL execution.
- Pager, B+ tree, WAL, recovery, encryption, or physical inspection mechanics.
- The meaning of integrity, freshness, witnesses, conflicts, provenance, or
  sync operations.
- CLI text layout, TUI state, .NET presentation, or frontend-specific styling.
- Network transport, peer authentication, or application conflict policy.
- Claims that source capabilities cannot substantiate.

### Dependency Direction

The intended dependency direction is provisional pending AR-0001:

```text
CLI / Rust caller / future adapter
              ↓
       TQL surface contract
          ├───────────────┐
          ↓               ↓
     tosumu-sql     public inspect / verify / sync APIs
          └───────────────┘
                      ↓
                 tosumu-core
```

Forbidden directions:

- `tosumu-core` must not depend on TQL, SQL, or CLI.
- `tosumu-sql` must not depend on TQL or CLI.
- TQL must not import physical pager, WAL, B+ tree, or crypto-frame types.
- TQL outcomes must not contain CLI widgets, terminal colors, or rendered text
  as their only contract.

## Public Contract Impact

Provisional types are expected to include concepts equivalent to:

```rust
enum TqlCommand {
    Status,
    Check,
    Describe { key: Vec<u8> },
    // Later commands are admitted only with backing semantics.
}

enum TqlOutcome {
    Status(StatusOutcome),
    Check(CheckOutcome),
    Description(DescriptionOutcome),
    Query(QueryOutcome),
    Unsupported(UnsupportedOutcome),
}
```

Exact names, ownership crate, lifetimes, and serialization are not accepted
contracts yet. Outcomes should carry source facts and explicit unavailable or
unanchored states, not preformatted prose alone.

The initial work has no on-disk format impact. If a later virtual view requires
new durable metadata, that change needs its own compatibility review.

## Deliverables

- [ ] AR-0001 records ownership alternatives and implementation evidence.
- [ ] Bounded TQL lexer/parser and typed command AST.
- [ ] Structured result and typed error model independent of CLI rendering.
- [ ] Read-only `STATUS`, `CHECK`, and `DESCRIBE <key>` vertical slice.
- [ ] CLI execution with human-readable and JSON evidence.
- [ ] At least one honest SQL-lowered command after required virtual-view
      semantics exist.
- [ ] Explicit capability gating for trust, freshness, conflict, witness, and
      sync commands.
- [ ] Parser/property/fuzz coverage and resource limits.
- [ ] Non-CLI consumer or documented evidence substitution.
- [ ] Operator command reference and updated design/governance records.

## Implementation Slices

Each slice must compile independently and must not claim facts unavailable from
its source capability.

### Slice 0: Baseline, Grammar Boundary, And Review

**Objective:** Freeze a small test corpus, confirm dependencies, and prevent
implementation from silently settling AR-0001.

#### Deliverables

- [x] Read ADR-0001, governance records, the SQL plan, and the TQL design.
- [x] Open AR-0001 for TQL ownership and mixed lowering.
- [ ] Record a command corpus containing accepted, malformed, trailing-token,
      whitespace, case, missing-argument, and oversized-input examples.
- [ ] Inventory the existing public facts that can back `STATUS`, `CHECK`, and
      `DESCRIBE` without CLI parsing or physical-type leakage.
- [ ] Define initial limits for command bytes, token count, key bytes, and
      diagnostic count.
- [ ] Record commands blocked by missing source semantics.

#### Acceptance Criteria

- [ ] Every first-slice command maps to an existing public source of truth.
- [ ] Grammar examples distinguish accepted syntax from aspirational syntax.
- [ ] No sync, witness, conflict, or freshness guarantee is implied by the
      parser corpus.
- [ ] Physical format impact is recorded as none.
- [ ] AR-0001 remains the authority for unresolved crate ownership.

#### Validation

```text
Review command corpus and source-capability inventory.
cargo test -p tosumu-sql
cargo test -p tosumu-cli
```

#### Exit State

A bounded grammar baseline and source-fact inventory permit parser work without
committing to unsupported command semantics.

### Slice 1: Bounded Parser And Typed Command Model

**Objective:** Parse the smallest read-only grammar into typed commands without
opening a database or performing work.

Initial grammar:

```text
STATUS
CHECK
DESCRIBE <key>
```

#### Deliverables

- [ ] Add a lexer/parser with explicit input and token limits.
- [ ] Add typed `TqlCommand` values for the initial grammar.
- [ ] Reject unknown commands, missing arguments, invalid trailing tokens,
      invalid key encoding, and oversized input with typed errors.
- [ ] Keep parsing side-effect free and independent of database handles.
- [ ] Add table-driven parser tests from the Slice 0 corpus.
- [ ] Add property tests asserting deterministic parse results and no panics.

#### Acceptance Criteria

- [ ] Supported commands parse case-insensitively with documented whitespace
      behavior.
- [ ] One input produces one command; command chaining is rejected.
- [ ] Keys are preserved exactly according to the documented key syntax.
- [ ] Invalid or unsupported syntax fails before any storage operation.
- [ ] The parser imports no core physical-storage or CLI presentation types.
- [ ] Parsing time and allocation remain bounded by declared limits.

#### Validation

```text
cargo test -p <tql-owning-crate> parser
cargo test -p <tql-owning-crate> --test command_corpus
```

#### Exit State

Tosumu can parse a deliberately tiny TQL surface into typed, inert commands.

### Slice 2: Structured Outcomes And Read-Only Dispatch

**Objective:** Execute `STATUS`, `CHECK`, and `DESCRIBE` through existing public
capabilities and return structured outcomes.

#### Deliverables

- [ ] Define structured status, check, and description outcomes.
- [ ] Add a dispatcher that receives explicit database/inspection capability
      inputs rather than opening hidden global state.
- [ ] Adapt existing inspect and verify facts without copying their meaning.
- [ ] Represent unavailable, not-checked, failed, and unanchored states
      distinctly.
- [ ] Add fixture databases for healthy, missing-key, integrity-failure where
      safely injectable, and bounded-inspection cases.
- [ ] Prove inspection commands do not mutate database or WAL state.

#### Acceptance Criteria

- [ ] `STATUS` returns a structured summary backed by existing source facts.
- [ ] `CHECK` identifies exactly what was checked, failed, and not checked.
- [ ] `DESCRIBE <key>` returns value metadata and an explicit missing-key result.
- [ ] No outcome claims freshness, witnesses, conflicts, or truth without
      supporting evidence.
- [ ] Expected failures are typed and do not rely only on logs or prose.
- [ ] Repeating any initial command leaves database bytes and logical contents
      unchanged.

#### Validation

```text
cargo test -p <tql-owning-crate> dispatch
cargo test -p <tql-owning-crate> --test read_only_commands
```

#### Exit State

The first useful TQL behavior exists as provider-neutral structured evidence,
without presentation or mutation ownership.

### Slice 3: CLI Integration And Output Contracts

**Objective:** Make the structured command surface useful to operators while
keeping rendering in `tosumu-cli`.

#### Deliverables

- [ ] Add one-shot CLI invocation for TQL input.
- [ ] Render human-readable output from structured outcomes.
- [ ] Emit versioned JSON output suitable for scripts and retained fixtures.
- [ ] Return stable process-level success/failure classification through the
      existing CLI error boundary.
- [ ] Add snapshot or fixture tests for text and JSON output.
- [ ] Document quoting and shell-escaping behavior at the CLI boundary.

#### Acceptance Criteria

- [ ] CLI text and JSON are renderings of the same structured outcome.
- [ ] Changing text layout does not change command semantics.
- [ ] JSON distinguishes unsupported, unavailable, unanchored, failed, and
      successful states.
- [ ] Secrets, protector material, and unauthenticated values are never emitted.
- [ ] Exit status is deterministic and documented for parse, command, integrity,
      and I/O failures.

#### Validation

```text
cargo test -p tosumu-cli tql
cargo test -p tosumu-cli --test tql_cli
```

#### Exit State

Operators can run the initial TQL commands, while non-CLI callers can consume
the same underlying structured outcomes.

### Slice 4: Explanation Commands Over Existing Evidence

**Objective:** Add `TRUST <key>` and `WHY <key>` only to the extent current
integrity, provenance, and freshness APIs can answer honestly.

#### Deliverables

- [ ] Inventory evidence needed for compact trust and explanatory outcomes.
- [ ] Define structured evidence citations and recommended-action fields.
- [ ] Implement `TRUST <key>` with explicit dimensions rather than one opaque
      trusted/untrusted boolean.
- [ ] Implement `WHY <key>` as structured reasons rendered by the caller.
- [ ] Report freshness as `unanchored` while no witness or observer anchor
      exists.
- [ ] Return explicit unavailable dimensions where provenance or witness facts
      do not exist.

#### Acceptance Criteria

- [ ] Integrity, freshness, provenance, and witness dimensions remain separate.
- [ ] Authenticated data is never described as true merely because it verifies.
- [ ] Missing evidence lowers confidence or availability; it is never silently
      treated as success.
- [ ] Recommended actions derive from typed findings, not hardcoded CLI prose.
- [ ] No new durable metadata is introduced without compatibility review.

#### Validation

```text
cargo test -p <tql-owning-crate> trust
cargo test -p <tql-owning-crate> explanation
```

#### Exit State

TQL can explain existing evidence without overstating Tosumu's current
freshness, witness, or provenance capabilities.

### Slice 5: Virtual Views And Honest SQL Lowering

**Objective:** Prove that query-shaped TQL sugar can reuse SQL semantics rather
than creating a second executor.

Candidate commands:

```text
STALE
CONFLICTS
NEEDS SYNC
UNWITNESSED
```

Only commands backed by implemented canonical metadata and SQL-visible views
may enter this slice.

#### Deliverables

- [ ] Specify the first virtual view's columns, source facts, ordering, and
      unavailable-state behavior.
- [ ] Implement the view in the SQL layer without teaching core SQL concepts.
- [ ] Lower one TQL command to a parameterized SQL AST or prepared SQL request.
- [ ] Expose the lowering decision in `EXPLAIN` or command diagnostics.
- [ ] Compare raw SQL and TQL outcomes over the same fixture.
- [ ] Reject or defer candidate commands whose source metadata does not exist.

#### Acceptance Criteria

- [ ] TQL and SQL produce equivalent rows and evidence for the admitted view.
- [ ] TQL does not reimplement scan, predicate, planner, or executor logic.
- [ ] View semantics are documented independently from their TQL spelling.
- [ ] Ordering and resource bounds are deterministic or explicitly unspecified.
- [ ] Unsupported candidate commands return typed capability-unavailable
      outcomes rather than empty success.

#### Validation

```text
cargo test -p tosumu-sql virtual_view
cargo test -p <tql-owning-crate> sql_lowering
```

#### Exit State

At least one command demonstrates the preferred TQL-to-SQL path, or the slice
is explicitly parked because no honest virtual-view candidate exists yet.

### Slice 6: Sync Planning And Preview Boundary

**Objective:** Expose non-mutating sync reasoning only after Tosumu has a stable
semantic change and sync-planning capability.

#### Deliverables

- [ ] Confirm semantic sync facts are independent of physical WAL records.
- [ ] Define bounded peer identity and sync-scope inputs.
- [ ] Implement `SYNC PLAN <peer>` and/or `SYNC PREVIEW <peer>` as thin adapters
      over the owning sync capability.
- [ ] Return structured send, receive, conflict, evidence-gain, and unavailable
      information.
- [ ] Add no-network deterministic fixtures for plan/preview behavior.
- [ ] Keep `SYNC APPLY` deferred unless a separate mutation and safety review
      admits it.

#### Acceptance Criteria

- [ ] TQL does not inspect WAL to infer semantic changes.
- [ ] Preview performs no mutation and requires no live transport.
- [ ] Conflict and policy decisions remain visible and are never silently
      resolved by TQL.
- [ ] Peer, scope, record, and diagnostic counts are bounded.
- [ ] Missing sync capability is explicit rather than simulated.

#### Validation

```text
cargo test -p <sync-owning-crate> preview
cargo test -p <tql-owning-crate> sync_preview
```

#### Exit State

TQL can explain a real semantic sync plan without becoming a sync engine. This
slice may remain parked well after the initial TQL release.

### Slice 7: Parser And Safety Hardening

**Objective:** Treat TQL input as untrusted and prove bounded failure behavior.

#### Deliverables

- [ ] Add fuzz targets for lexer/parser and structured rendering boundaries.
- [ ] Add malformed UTF-8 boundary tests where byte input is accepted.
- [ ] Add oversized token, key, nesting, whitespace, and trailing-input cases.
- [ ] Add command-family authorization hooks only if a real mutating command is
      admitted later.
- [ ] Audit diagnostics for secret or unauthenticated data disclosure.
- [ ] Record parser and representative command timing/allocation observations.

#### Acceptance Criteria

- [ ] Arbitrary input cannot panic, recurse without bound, or allocate without
      declared limits.
- [ ] Parse failures perform no database work.
- [ ] Inspection never bypasses authentication or integrity checks.
- [ ] Diagnostics are bounded and redact protected material.
- [ ] Performance findings remain evidence unless promoted to a contract.

#### Validation

```text
cargo test --workspace --all-targets
cargo fuzz run <tql-parser-target>
```

#### Exit State

The admitted grammar and command paths are robust against malformed and hostile
input within documented resource limits.

### Slice 8: Independent Consumer And Compatibility Evidence

**Objective:** Determine whether TQL has earned a reusable subsystem boundary.

#### Deliverables

- [ ] Add a non-CLI Rust, .NET, or consumer-corpus caller using structured TQL
      commands and outcomes.
- [ ] Compare parser and outcome behavior across both callers.
- [ ] Introduce explicit schema/version metadata if outcomes are serialized.
- [ ] Classify every divergence as presentation-only, contract refinement, or
      rejected consumer behavior.
- [ ] Decide whether a dedicated `tosumu-tql` crate is justified.

#### Acceptance Criteria

- [ ] The second caller does not parse CLI text or duplicate command semantics.
- [ ] Provider/frontend concerns remain outside TQL outcomes.
- [ ] Serialized compatibility, if offered, has an explicit version policy.
- [ ] Crate extraction is based on independent reuse, not naming preference.
- [ ] AR-0001 records the resulting ownership evidence.

#### Validation

```text
cargo test --workspace --all-targets
Run the selected independent consumer fixture.
```

#### Exit State

The project has enough evidence to accept a durable TQL boundary, continue
incubation, or keep TQL CLI-local.

### Slice 9: Admission, Documentation, Or Parking

**Objective:** Close the plan honestly and reconcile architecture with the
implemented surface.

#### Deliverables

- [ ] Update the TQL design to distinguish implemented, deferred, and rejected
      commands.
- [ ] Add an operator command reference for every admitted command.
- [ ] Reconcile CLI, SQL, inspect, error, safety, and design documentation.
- [ ] Resolve AR-0001 based on retained implementation evidence.
- [ ] Create ADR-0002 if, and only if, a durable ownership decision is accepted.
- [ ] Record parked commands with their missing capability and reopening
      trigger.

#### Acceptance Criteria

- [ ] Documentation never labels design-only commands as implemented.
- [ ] Every admitted command names its source facts, output shape, and failure
      semantics.
- [ ] All validation gates pass.
- [ ] The plan status is Completed, Parked, Superseded, or Blocked accurately.
- [ ] No unresolved architecture question is disguised as a local code choice.

#### Validation

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
mkdocs build --strict
```

#### Exit State

TQL has an evidence-backed architectural home and bounded supported surface, or
the useful initial command set remains explicitly incubating with named gaps.

## Validation Matrix

| Concern | Evidence | Command Or Artifact | Required Result |
| --- | --- | --- | --- |
| Parser behavior | Table/property tests | TQL parser tests | Deterministic parse or typed rejection |
| Read-only dispatch | Healthy and failure fixtures | Read-only command integration tests | Structured, non-mutating outcome |
| SQL lowering | Raw SQL/TQL equivalence fixture | Virtual-view integration test | Equivalent semantic result |
| CLI boundary | Text and JSON fixtures | CLI integration tests | Same underlying outcome |
| Invalid input | Corpus and fuzzing | Parser fuzz target | No panic or unbounded work |
| Security | Integrity/redaction fixtures | TQL security tests | No unauthenticated or secret output |
| Compatibility | Versioned structured fixture | Consumer corpus | Explicit compatible or rejected result |
| Workspace | Full validation | `cargo test --workspace --all-targets` | Pass |
| Documentation | Strict site build | `mkdocs build --strict` | Pass |

## Failure And Diagnostic Semantics

Expected TQL failure classes include:

- input exceeds a declared parser or key limit;
- unknown or unsupported command;
- malformed or incomplete command;
- command recognized but source capability unavailable;
- database open, I/O, or lock failure;
- missing key or peer;
- authentication or integrity failure;
- requested evidence unavailable or freshness unanchored;
- SQL lowering, semantic check, plan, or execution failure;
- preview refused because policy, scope, or conflict facts are incomplete.

Errors must retain the owning subsystem's structured cause without exposing
physical implementation objects as TQL contracts. `Unavailable` and
`Unanchored` are evidence states, not parser errors. Human recommendations are
rendered from typed findings.

## Compatibility And Migration

- **On-disk format:** No impact for parser and initial inspection slices.
- **WAL/checkpoint/recovery:** Read-only commands must not mutate these states.
- **Rust API:** Provisional until independent-consumer evidence and AR-0001
  resolution.
- **CLI:** New additive command surface; exit and JSON behavior must be explicit.
- **.NET API:** Deferred; may become the independent consumer.
- **Fixtures:** Command and outcome fixtures require schema/version metadata
  before compatibility is promised.
- **Rollback:** Removing provisional TQL must not affect stored database bytes.

Any durable metadata added for views, provenance, conflicts, witnesses, or sync
requires its own compatibility and recovery review.

## Security And Trust

TQL operates on untrusted command input and may expose security-sensitive
evidence. It must:

- respect authenticated-read boundaries;
- avoid protector keys, decrypted secrets, and raw internal metadata leakage;
- distinguish integrity from freshness and truth;
- report what was not checked;
- bound outputs and diagnostic counts;
- avoid command mutation until authorization, atomicity, audit, and preview
  semantics are reviewed.

TQL does not make Tosumu compliant or certified. It helps operators collect and
explain evidence Tosumu can actually provide.

## Performance And Resource Bounds

Slice 0 must select initial limits for command bytes, tokens, key bytes, rows,
and diagnostics. Query and preview commands must expose truncation or paging
rather than silently allocating complete unbounded result sets.

Parser and dispatch measurements are observations. No latency guarantee is
created by this plan.

## Risks And Mitigations

| Risk | Impact | Mitigation Or Evidence |
| --- | --- | --- |
| TQL becomes a second executor | Duplicate semantics and divergent behavior | Require honest SQL lowering or thin public operational dispatch |
| Shell claims unsupported trust | Operators act on fabricated confidence | Typed unavailable/unanchored states and evidence-dimensional outcomes |
| CLI presentation becomes API | Other callers scrape prose | Structured outcomes rendered separately by each frontend |
| Dedicated crate is premature | Public abstraction churn | Keep ownership under AR-0001 until independent consumer evidence |
| SQL absorbs operational semantics | Relational layer becomes a general shell runtime | Keep SQL responsible only for relational views and execution |
| Core absorbs TQL vocabulary | Physical layer couples upward | Enforce ADR-0001 dependency direction |
| Mutating commands bypass safety | Integrity or recovery regression | Defer `SYNC APPLY`; require separate mutation review |
| Large result sets exhaust resources | Denial of service or unusable shell | Explicit row/byte/diagnostic limits and truncation metadata |

## Completion Criteria

The plan is complete when:

- [ ] all admitted slices and acceptance criteria pass;
- [ ] parser, dispatch, and output contracts are bounded and tested;
- [ ] at least one useful read-only vertical slice is available;
- [ ] SQL lowering is proven or explicitly parked for missing view semantics;
- [ ] unsupported trust/sync behavior remains explicit;
- [ ] AR-0001 has a recorded disposition;
- [ ] ADR-0002 exists only if a durable architecture was accepted;
- [ ] public docs distinguish implemented and design-only commands;
- [ ] deferred work has an owner and reopening trigger.

## Parking Or Reopening Criteria

Park SQL-lowered commands until a canonical virtual view and logical scan can
support them. Park sync commands until semantic change history, conflicts,
scope, peer identity, and preview APIs exist independently of TQL.

Reopen parked slices when:

- a virtual view exposes stale, conflicted, unwitnessed, or sync-candidate
  records;
- witness or observer evidence anchors freshness;
- a semantic sync planner exposes bounded preview results;
- a non-CLI consumer requires the structured TQL surface;
- implementation reveals repeated duplicated lowering logic.

## Progress Log

### 2026-08-03

- **Work completed:** Reviewed TQL design, SQL/core/CLI boundaries, Tosumu
  governance, and opened AR-0001.
- **Validation:** Documentation and source inventory only.
- **Findings:** TQL is plausibly a thin mixed-lowering surface; dedicated crate
  ownership and virtual views remain unproven.
- **Plan changes:** Split useful inspection, explanation, SQL lowering, sync,
  hardening, and admission into independently parkable slices.
- **Next slice:** Complete the Slice 0 command corpus and source-capability
  inventory.

## References

- `docs/Tosumu Command Language.md`
- `docs/Specifications/Tosumu Software Design Document.md`, especially the SQL/TQL sibling-surface sections
- `docs/ADR/ADR-0001-storage-engine-layer-boundaries.md`
- `docs/Architectural Reviews/AR-0001-tql-command-language-boundary.md`
- `docs/Plans/initial-sql-layer.md`
- `docs/Plans/main-feature-roadmap.md`
- `crates/tosumu-sql`
- `crates/tosumu-cli`
- `crates/tosumu-core/src/inspect.rs`

