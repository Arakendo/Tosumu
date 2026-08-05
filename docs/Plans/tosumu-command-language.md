# Tosumu Command Language Implementation Plan

| Field | Value |
| --- | --- |
| Status | In progress |
| Opened | 2026-08-03 |
| Last updated | 2026-08-04 |
| Owner | Tosumu maintainers |
| Target | TQL parser, structured command surface, SQL lowering, and operator integration |
| Related ADRs | [ADR-0001: Storage Engine Layer Boundaries](../ADR/ADR-0001-storage-engine-layer-boundaries.md) |
| Related reviews | [AR-0001: TQL Command Language Boundary](../Architectural%20Reviews/AR-0001-tql-command-language-boundary.md) |
| Related CRs | None |
| Depends on | Existing inspect/verify APIs, initial SQL layer, later virtual views and sync semantics |

## Status

TQL now has a bounded, read-only CLI incubation surface for `STATUS`, `CHECK`,
`DESCRIBE <key>`, and `WAL STATUS`. Its parser, typed commands, structured outcomes, and
CLI renderers remain private to `tosumu-cli`; virtual views, SQL lowering,
trust, freshness, sync, mutation, and a reusable public API remain deferred.
The ownership boundary is still under review in AR-0001.

The local timing channel is documented in
[TQL Runtime Observation v1](../Notes/TQL%20Runtime%20Observation%20v1.md).

The currently actionable CLI-local slices are complete enough to pause:
remaining work is either platform validation (Linux fuzz and a fresh strict
MkDocs CI run) or requires an independently owned capability or caller. The
repository documentation workflow already runs `mkdocs build --strict` on
Python 3.12 before Pages deployment; this shell simply lacks a local MkDocs
installation. This plan remains **In progress** rather than completed because
those are evidence gaps, not waived requirements.

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
- `tosumu-cli` now privately contains a bounded TQL parser, typed commands,
  structured outcomes, and separate text/JSON renderers for the initial
  read-only surface.

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

- [x] AR-0001 records ownership alternatives and implementation evidence.
- [x] Bounded TQL lexer/parser and typed command AST.
- [x] Structured result and typed error model independent of CLI rendering.
- [x] Read-only `STATUS`, `CHECK`, `DESCRIBE <key>`, and `WAL STATUS` vertical slice.
- [x] CLI execution with human-readable and JSON evidence.
- [ ] At least one honest SQL-lowered command after required virtual-view
      semantics exist.
- [ ] Explicit capability gating for trust, freshness, conflict, witness, and
      sync commands.
- [~] Parser/property/fuzz coverage and resource limits. Property coverage,
      bounded limits, parser fuzzing, and outcome-rendering fuzzing are present;
      sustained Linux fuzz execution remains open.
- [x] Non-CLI consumer or documented evidence substitution. Tokimu's external
      process consumer validates the provisional JSON contract without linking
      Tosumu implementation crates.
- [x] Operator command reference and updated design/governance records.

## Implementation Slices

Each slice must compile independently and must not claim facts unavailable from
its source capability.

### Slice 0: Baseline, Grammar Boundary, And Review

**Objective:** Freeze a small test corpus, confirm dependencies, and prevent
implementation from silently settling AR-0001.

#### Deliverables

- [x] Read ADR-0001, governance records, the SQL plan, and the TQL design.
- [x] Open AR-0001 for TQL ownership and mixed lowering.
- [x] Record a command corpus containing accepted, malformed, trailing-token,
      whitespace, case, missing-argument, and oversized-input examples.
- [x] Inventory the existing public facts that can back `STATUS`, `CHECK`,
      `DESCRIBE`, and `WAL STATUS` without CLI parsing or physical-type leakage.
- [x] Define initial limits for command bytes, token count, key bytes, and
      diagnostic count.
- [x] Record commands blocked by missing source semantics.

#### Acceptance Criteria

- [x] Every first-slice command maps to an existing public source of truth.
- [x] Grammar examples distinguish accepted syntax from aspirational syntax.
- [x] No sync, witness, conflict, or freshness guarantee is implied by the
      parser corpus.
- [x] Physical format impact is recorded as none.
- [x] AR-0001 remains the authority for unresolved crate ownership.

#### Validation

```text
Review command corpus and source-capability inventory.
cargo test -p tosumu-sql
cargo test -p tosumu-cli
```

#### Exit State

A bounded grammar baseline and source-fact inventory permit parser work without
committing to unsupported command semantics.

**Recorded evidence:** [TQL Command Corpus v1](../Notes/TQL%20Command%20Corpus%20v1.md)
defines the initial accepted syntax, typed failures, source-fact inventory,
limits, and explicitly blocked command families.

### Slice 1: Bounded Parser And Typed Command Model

**Objective:** Parse the smallest read-only grammar into typed commands without
opening a database or performing work.

Initial grammar:

```text
STATUS
CHECK
DESCRIBE <key>
WAL STATUS
```

#### Deliverables

- [x] Add a lexer/parser with explicit input and token limits.
- [x] Add typed `TqlCommand` values for the initial grammar.
- [x] Reject unknown commands, missing arguments, invalid trailing tokens,
      invalid key encoding, and oversized input with typed errors.
- [x] Keep parsing side-effect free and independent of database handles.
- [x] Add table-driven parser tests from the Slice 0 corpus.
- [x] Add property tests asserting deterministic parse results and no panics.

#### Acceptance Criteria

- [x] Supported commands parse case-insensitively with documented whitespace
      behavior.
- [x] One input produces one command; command chaining is rejected.
- [x] Keys are preserved exactly according to the documented key syntax.
- [x] Invalid or unsupported syntax fails before any storage operation.
- [x] The parser imports no core physical-storage or CLI presentation types.
- [x] Parsing time and allocation remain bounded by declared limits.

#### Validation

```text
cargo test -p <tql-owning-crate> parser
cargo test -p <tql-owning-crate> --test command_corpus
```

#### Exit State

Tosumu can parse a deliberately tiny TQL surface into typed, inert commands.
The parser remains incubated in `tosumu-cli` until a second consumer establishes
that a dedicated `tosumu-tql` crate is warranted.

### Slice 2: Structured Outcomes And Read-Only Dispatch

**Objective:** Execute `STATUS`, `CHECK`, `DESCRIBE`, and `WAL STATUS` through
existing public capabilities and return structured outcomes.

#### Deliverables

- [x] Define structured status, check, description, and WAL-status outcomes.
- [x] Add a dispatcher that receives explicit database/inspection capability
      inputs rather than opening hidden global state.
- [x] Adapt existing inspect and verify facts without copying their meaning.
- [ ] Represent unavailable, not-checked, failed, and unanchored states
      distinctly.
- [x] Exercise temporary fixtures for healthy, missing-key, integrity-failure,
      missing-verification, and WAL-status observations. The read-only dispatch
      tests retain byte-for-byte database and WAL comparisons around the
      healthy command set.
- [x] Prove inspection commands do not mutate database or WAL state.

#### Acceptance Criteria

- [x] `STATUS` returns a structured summary backed by existing source facts.
- [x] `CHECK` identifies exactly what was checked, failed, and not checked.
- [x] `DESCRIBE <key>` returns value metadata and an explicit missing-key result.
- [x] `WAL STATUS` returns only public sidecar existence and decoded record
      count, without recovery, checkpoint, durability, freshness, trust, or
      synchronization claims.
- [x] No outcome claims freshness, witnesses, conflicts, or truth without
      supporting evidence. The four admitted outcome families carry no fields
      for those deferred dimensions; their explicit non-guarantees are retained
      in the operator reference and disclosure audit.
- [x] Expected failures are typed and do not rely only on logs or prose.
- [x] Repeating any initial command leaves database bytes and logical contents
      unchanged.

#### Validation

```text
cargo test -p <tql-owning-crate> dispatch
cargo test -p <tql-owning-crate> --test read_only_commands
```

#### Exit State

The first useful TQL behavior exists as provider-neutral structured evidence,
without presentation or mutation ownership.

**Progress:** `tosumu-cli` now contains an incubating read-only dispatch adapter
that accepts an explicit `KvStore`, optional `VerificationReport`, and optional
public `WalSummary`. It reports unprovided verification as `NotChecked`, maps
supplied verification facts without reinterpreting them, returns value presence
plus byte length rather than value contents, and limits WAL observation to
sidecar existence plus decoded record count. Unavailable and unanchored states remain deferred because the
initial command set has no source capability that can establish them. A
temporary-store test proves the first command set leaves the database bytes
and WAL unchanged. The dispatch tests now also inject one authenticated-page
failure through the existing inspection fixture pattern, proving `CHECK`
reports page failure while leaving the unavailable B-tree result as
`NotChecked`. `DESCRIBE` currently uses the public `KvStore::get()` result only
to report value length; it must not be treated as a durable metadata-only
lookup contract. Bounded inspection remains the material Slice 2 evidence gap:
a future provider-level metadata lookup, if independently needed, should prove
that presence and byte length can be observed without loading the value.

### Slice 3: CLI Integration And Output Contracts

**Objective:** Make the structured command surface useful to operators while
keeping rendering in `tosumu-cli`.

#### Deliverables

- [x] Add one-shot CLI invocation for TQL input.
- [x] Render human-readable output from structured outcomes.
- [x] Emit provisional versioned JSON output suitable for scripts and retained
      fixtures.
- [x] Keep TQL JSON errors in the same provisional versioned schema family as
      successful TQL outcomes.
- [x] Return stable process-level success/failure classification through the
      existing CLI error boundary.
- [x] Add focused output tests for text and JSON rendering.
- [x] Document quoting and shell-escaping behavior at the CLI boundary.
- [x] Make `tql --help` state the admitted commands and read-only exclusions.

#### Acceptance Criteria

- [x] CLI text and JSON are renderings of the same structured outcome.
- [x] Changing text layout does not change command semantics.
- [ ] JSON distinguishes unsupported, unavailable, unanchored, failed, and
      successful states.
- [x] Secrets, protector material, and unauthenticated values are never emitted.
- [x] Exit status is deterministic and documented for parse, command, integrity,
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

- [x] Inventory evidence needed for compact trust and explanatory outcomes.
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

- [x] Add fuzz targets for lexer/parser and structured rendering boundaries.
      `fuzz_tql_parse` covers arbitrary UTF-8 syntax and `fuzz_tql_render`
      covers bounded synthetic outcomes without opening a store.
- [ ] Add malformed UTF-8 boundary tests where byte input is accepted.
- [x] Add oversized command, token, and key cases plus whitespace and
      trailing-input rejection. The initial grammar has no nested syntax, so
      nesting cannot enter its parser state.
- [ ] Add command-family authorization hooks only if a real mutating command is
      admitted later.
- [x] Audit the implemented read-only renderers for secret or unauthenticated
      data disclosure. The retained audit names the public output facts and
      reopening triggers for any secret-bearing command family.
- [~] Add opt-in parser and representative command timing observations. `tql
      --timings` writes bounded stage durations to stderr while preserving the
      human and JSON outcome schemas. Allocation measurement remains deferred
      until a profiling mechanism is admitted.

#### Acceptance Criteria

- [x] Arbitrary UTF-8 input is property-tested for deterministic, non-panicking
      parsing; the grammar has no recursion and declares command, token, and
      key limits.
- [x] Parse failures perform no database work.
- [x] Maximum-size unknown tokens are reported once in structured details;
      human-facing messages remain bounded and do not duplicate the token.
- [ ] Inspection never bypasses authentication or integrity checks.
- [~] Diagnostics are bounded and the implemented renderers omit protected
      values; provider-error redaction needs encrypted/provider evidence.
- [x] Performance findings remain evidence unless promoted to a contract. The
      opt-in timing stream is explicitly diagnostic-only and excludes those
      observations from the result schema.

#### Validation

```text
cargo test --workspace --all-targets
cargo fuzz run <tql-parser-or-render-target>
```

`fuzz_tql_parse` and `fuzz_tql_render` compile under the repository's installed nightly toolchain.
On the current Windows development environment, libFuzzer execution is blocked
by a missing sanitizer runtime DLL (`STATUS_DLL_NOT_FOUND`); run the target on
a sanitizer-capable Windows setup or Linux CI before treating fuzz execution as
completed evidence. The separate weekly/manual Linux `Fuzz` workflow now runs
both bounded targets for `10,000` iterations and deliberately does not alter
stable build CI.

**Recorded evidence:** [TQL Disclosure Audit v1](../Notes/TQL%20Disclosure%20Audit%20v1.md)
records the output facts permitted for the implemented command set and its
explicit limits.

#### Exit State

The admitted grammar and command paths are robust against malformed and hostile
input within documented resource limits.

### Slice 8: Independent Consumer And Compatibility Evidence

**Objective:** Determine whether TQL has earned a reusable subsystem boundary.

#### Deliverables

- [x] Add a non-CLI Rust, .NET, or consumer-corpus caller using structured TQL
      commands and outcomes. Tokimu's Tier 3
      `corpus/consumers/tosumu-tql-cli-consumer` invokes the executable as an
      external process and consumes only the versioned JSON envelope.
- [x] Compare parser and outcome behavior across both callers. The consumer
      executes the admitted success corpus plus a typed invalid command, then
      asserts command names, outcome state, error code, JSON schema version,
      and process classification without parsing TQL itself.
- [x] Introduce explicit schema/version metadata if outcomes are serialized.
      The provisional envelope carries `schema_version: 1`; the independent
      consumer rejects an unknown version rather than guessing compatibility.
- [x] Classify every divergence as presentation-only, contract refinement, or
      rejected consumer behavior. The admitted fixture corpus observed no
      divergence; a future difference must be recorded in one of those three
      classes before its expectation changes.
- [ ] Decide whether a dedicated `tosumu-tql` crate is justified.

#### Acceptance Criteria

- [x] The second caller does not parse CLI text or duplicate command semantics.
- [x] Provider/frontend concerns remain outside TQL outcomes. The corpus
      consumer invokes an executable and reads JSON only; it neither opens a
      store nor receives provider-native records.
- [x] Serialized compatibility, if offered, has an explicit version policy.
      The current policy is provisional schema version `1`: consumers reject
      unknown versions, and no stable public ABI is claimed.
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

**Current evidence:** The first independent caller is intentionally an
external-process corpus tool rather than an in-process crate user. It exercises
the JSON compatibility boundary that a .NET, Rust, or automation host would
actually receive, while leaving parser, storage opening, inspection, dispatch,
and semantic interpretation inside Tosumu. This supports continued CLI-local
incubation; it does not by itself justify extracting `tosumu-tql`.

### 2026-08-04 -- External-Process Consumer Evidence

- **Consumer:** Tokimu's Tier 3 `tosumu-tql-cli-consumer` creates a temporary
  fixture only through public `tosumu init` and `tosumu put` commands, then
  invokes `tosumu tql` as a separate process.
- **Observed contract:** The consumer accepts only JSON envelope schema version
  `1`, rejects unknown versions, and verifies `STATUS`, `CHECK`, both
  `DESCRIBE` outcomes, `WAL STATUS`, and a typed invalid-command rejection.
- **Boundary result:** No admitted fixture divergence was observed. The
  consumer neither parses TQL, opens Tosumu storage, links Tosumu crates, nor
  receives provider-native records.
- **Conclusion:** This is evidence for the executable and serialized
  compatibility boundary. It is not evidence that parser, outcome, or dispatch
  types are ready for a stable in-process `tosumu-tql` crate.

### Slice 9: Admission, Documentation, Or Parking

**Objective:** Close the plan honestly and reconcile architecture with the
implemented surface.

#### Deliverables

- [x] Update the TQL design to distinguish implemented, deferred, and rejected
      commands. The bounded initial surface is now named explicitly; trust,
      SQL-view, sync, mutation, shell, and byte-input work remains deferred
      until its owning capability exists.
- [x] Add an operator command reference for every admitted command. The
      reference names source facts, outcome shape, successful missing-key/WAL
      observations, and non-guarantees for `STATUS`, `CHECK`, `DESCRIBE`, and
      `WAL STATUS`.
- [x] Reconcile CLI, SQL, inspect, error, safety, and design documentation.
      The SDD now names the exact CLI-local subset alongside the long-range
      TQL direction; the command design, error design, review, and plan retain
      the same deferred boundaries.
- [ ] Resolve AR-0001 based on retained implementation evidence.
- [ ] Create ADR-0002 if, and only if, a durable ownership decision is accepted.
- [x] Record parked commands with their missing capability and reopening
      trigger.

#### Acceptance Criteria

- [x] Documentation never labels design-only commands as implemented.
- [x] Every admitted command names its source facts, output shape, and failure
      semantics.
- [ ] All validation gates pass.
- [x] The plan status is In progress, Completed, Parked, Superseded, or Blocked
      accurately. It remains In progress because external validation and
      independent-consumer evidence are still material.
- [x] No unresolved architecture question is disguised as a local code choice.
      Trust, SQL views, sync, protected metadata, byte input, and reusable
      crate ownership remain named parking items rather than CLI-local
      implementations.

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
- [x] parser, dispatch, and output contracts are bounded and tested;
- [x] at least one useful read-only vertical slice is available;
- [x] SQL lowering is proven or explicitly parked for missing view semantics;
- [x] unsupported trust/sync behavior remains explicit;
- [ ] AR-0001 has a recorded disposition;
- [ ] ADR-0002 exists only if a durable architecture was accepted;
- [x] public docs distinguish implemented and design-only commands;
- [x] deferred work has an owner and reopening trigger.

## Parking Or Reopening Criteria

Park SQL-lowered commands until a canonical virtual view and logical scan can
support them. Park sync commands until semantic change history, conflicts,
scope, peer identity, and preview APIs exist independently of TQL.

| Parked command family | Missing owning capability | Reopen only when |
| --- | --- | --- |
| `TRUST`, `WHY`, `EVIDENCE`, freshness and witness queries | Key-scoped evidence, provenance, witness, and freshness facts | A provider-neutral evidence source can name supported and unavailable dimensions without fabricated verdicts |
| `STALE`, `CONFLICTS`, and other query sugar | Canonical SQL virtual views plus logical scan semantics | A view has an accepted contract and TQL/SQL equivalence can be tested |
| `SYNC PREVIEW` and `SYNC APPLY` | Semantic change history, peer/scope identity, conflict policy, and preview/apply APIs | The sync owner exposes a bounded plan that TQL can observe without interpreting WAL |
| Mutations, unlock/protector, or byte-input commands | Command-family authorization, protected metadata, and byte-input contracts | The owning API defines authorization, disclosure, failure, and resource-limit semantics |
| Metadata-only `DESCRIBE` refinement | Provider-neutral record metadata observation | An independent consumer proves presence/length metadata is needed without materializing values |

Reopen parked slices when:

- a virtual view exposes stale, conflicted, unwitnessed, or sync-candidate
  records;
- witness or observer evidence anchors freshness;
- a semantic sync planner exposes bounded preview results;
- a non-CLI consumer requires the structured TQL surface;
- implementation reveals repeated duplicated lowering logic.

## Waiting Register

Unchecked work in this plan is not a generic backlog. Each item remains open
because its source capability, independent evidence, or architectural decision
does not yet exist. TQL must not manufacture that missing layer merely to make
a checklist green.

| Slice | What is waiting | Why this plan cannot close it locally | Reopen or completion event |
| --- | --- | --- | --- |
| Slice 2: outcomes | `Unavailable` and `Unanchored` outcome variants | The four admitted commands have no source that can establish provider availability or anchor freshness. `NotChecked` and explicit command non-guarantees are the only honest current facts. | A provider-neutral availability or evidence/freshness API emits the corresponding typed fact. |
| Slice 3: JSON | Unsupported, unavailable, and unanchored result serialization | JSON cannot honestly serialize states the structured outcome does not yet produce. Adding placeholder variants would imply a contract without source evidence. | Slice 2 or Slice 4 admits those outcome states from a real owner. |
| Slice 4: trust and explanation | `TRUST`, `WHY`, evidence citations, recommendations, and freshness | These require key-scoped provenance, integrity, witness, and freshness facts. Database-wide verification does not become a per-key trust verdict. | An evidence-owning capability exposes bounded, provider-neutral findings and their unavailable dimensions. |
| Slice 5: SQL lowering | First virtual view and one SQL-backed TQL command | SQL has no accepted canonical view or logical scan contract for these operational facts. A CLI scan would duplicate or invent SQL semantics. | `tosumu-sql` admits a view with defined columns, ordering, bounds, and equivalence fixtures. |
| Slice 6: sync | Peer/scope inputs, `SYNC PLAN`/`SYNC PREVIEW`, conflicts, and apply policy | WAL records are physical recovery data, not semantic changes. TQL cannot infer peers, conflict policy, or safe mutations from them. | A sync owner exposes semantic change history and bounded preview facts; mutation work receives its own safety review. |
| Slice 7: hardening | Byte-input tests, authorization hooks, encrypted-provider disclosure, sustained fuzz execution, and allocation measurement | No byte-input or mutating TQL command exists, and no protected-provider fixture/profiling contract has been admitted. Windows cannot execute the sanitizer fuzz runtime. | Admit the relevant command/provider/profiling capability, and observe successful Linux `Fuzz` workflow execution for both existing targets. |
| Slice 8: independent use | Second Rust, .NET, or corpus consumer and compatibility comparison | A second consumer must genuinely need the structured outcome rather than be fabricated solely to force crate extraction. | An independent caller consumes the outcome without parsing CLI text, then classifies any divergence. |
| Slice 9: admission | AR-0001 disposition and possible ADR-0002 | Crate ownership depends on Slice 8 and the unresolved evidence above. An ADR now would freeze an unproven boundary. | AR-0001 reviews independent-consumer and validation evidence, then accepts continued CLI locality or durable extraction. |
| Cross-cutting validation | Fresh strict MkDocs and Linux fuzz CI evidence; full workspace validation | The repository already defines the CI routes. This local shell lacks MkDocs/Python 3.12 and Windows cannot execute the sanitizer runtime; the broad workspace command previously exceeded the local timeout without a test failure. | Observe successful `Docs`, `Fuzz`, and normal CI workflow runs for the committed change set. |

This register is deliberately conservative. A source capability may reduce one
row without authorizing unrelated commands or a public `tosumu-tql` crate.

## Progress Log

### 2026-08-03

- **Work completed:** Reviewed TQL design, SQL/core/CLI boundaries, Tosumu
  governance, and opened AR-0001.
- **Validation:** Documentation and source inventory only.
- **Findings:** TQL is plausibly a thin mixed-lowering surface; dedicated crate
  ownership and virtual views remain unproven.
- **Plan changes:** Split useful inspection, explanation, SQL lowering, sync,
  hardening, and admission into independently parkable slices.

### 2026-08-03 -- Slice 0 and Slice 1

- **Work completed:** Added `TQL Command Corpus v1` and an inert parser module
  under `tosumu-cli`. The parser accepts only `STATUS`, `CHECK`,
  `DESCRIBE <key>`, and `WAL STATUS` and emits typed commands or typed syntax
  errors.
- **Source-fact inventory:** `KvStore::stat()` can back `STATUS`, public
  inspection verification reports can back `CHECK`, `KvStore::get()` can back
  a deliberately minimal `DESCRIBE` result, and public `inspect_wal()` facts
  can back a deliberately small `WAL STATUS` result.
- **Limits:** Command bytes `4096`, tokens `16`, key bytes `1024`, and future
  command diagnostics `32`.
- **Property evidence:** The parser now uses `proptest` over arbitrary UTF-8
  input to prove repeated parses are deterministic and remain panic-free;
  table-driven cases retain the specific grammar and limit assertions.
- **Validation:** `cargo fmt --all -- --check`; `cargo test -p tosumu-cli tql`;
  `cargo clippy -p tosumu-cli --all-targets -- -D warnings`.
- **Finding:** The parser can remain inside `tosumu-cli` without CLI rendering,
  database handles, physical storage types, or a public ABI. That preserves
  AR-0001's deferred crate decision.

### 2026-08-03 -- Slice 2 and Slice 3

- **Work completed:** Added read-only dispatch for `STATUS`, `CHECK`,
  `DESCRIBE`, and `WAL STATUS`, plus the one-shot `tosumu tql <database> "<statement>"` CLI
  command. The adapter opens a store only after parsing, requests a public
  verification snapshot only for `CHECK`, and renders the resulting typed
  outcome as either human text or provisional JSON schema version `1`.
- **Read-only evidence:** Dispatch tests compare database and WAL bytes before
  and after representative commands. A tampered-page fixture reports failed
  page integrity without claiming tree verification passed.
- **Disclosure boundary:** `DESCRIBE` reports only key presence and value byte
  count; it does not emit stored value content. The first CLI surface accepts
  neither unlock material nor trust, freshness, witness, or sync commands.
- **Exit boundary:** `STATUS`, `WAL STATUS`, and `DESCRIBE` complete successfully when their
  read-only observation succeeds, including a missing key. `CHECK` returns the
  existing nonzero reported-issues process status when page or B-tree integrity
  reports a failure; parse, open, and I/O failures remain structured errors.
- **Validation:** `cargo fmt --all -- --check`; `cargo test -p tosumu-cli tql`;
  `cargo clippy -p tosumu-cli --all-targets -- -D warnings`.
- **Finding:** The same structured outcome can serve text and JSON rendering
  without making terminal formatting part of TQL semantics. `Unavailable` and
  `Unanchored` remain deferred evidence states because the initial commands do
  not own the capabilities that would produce them.
- **Next slice:** Inventory the evidence required for `TRUST <key>` and
  `WHY <key>` before admitting explanation commands.

### 2026-08-03 -- Slice 4 Evidence Inventory

- **Work completed:** Added `TQL Trust And Explanation Evidence Inventory v1`.
  It separates database-wide verification and key presence from the absent
  key-scoped integrity citation, freshness, witness, provenance, conflict, and
  recommendation capabilities.
- **Finding:** Current facts cannot honestly support `TRUST <key>` or
  `WHY <key>`. In particular, a successful database `CHECK` is not a per-key
  trust verdict, and no current source can ground a freshness claim.
- **Plan change:** Marked only the evidence-inventory deliverable complete.
  Explanation commands remain deferred until their source capabilities exist.
- **Next slice:** Wait for a public evidence-bearing capability or proceed to
  another independently supported plan slice; do not create shell-owned trust
  or recommendation semantics.

### 2026-08-03 -- Read-Only Exit Classification

- **Work completed:** Classified completed `CHECK` results separately from
  command failures. A page or B-tree integrity failure now returns the
  existing nonzero reported-issues process status after rendering its typed
  outcome. `STATUS`, found `DESCRIBE`, and missing `DESCRIBE` remain successful
  read-only observations.
- **Validation:** Focused TQL tests prove that only a typed failed integrity
  dimension produces `ReportedIssues`; parser, open, and I/O failures remain
  structured CLI errors.
- **Finding:** Process status can communicate an observed integrity problem
  without turning a missing key into an error or making text output the only
  machine-readable contract.
- **Next slice:** Remain parked on trust/explanation and SQL-lowering work
  until their public evidence or virtual-view sources exist.

### 2026-08-03 -- TQL JSON Error Boundary

- **Work completed:** Kept `tql --json` failures in TQL's provisional schema
  version `1` rather than reusing the older inspect envelope. The TQL error
  envelope carries the typed error code, status, message, and available parser
  details, including a rejected command or missing argument where applicable.
- **Validation:** A focused renderer test proves a missing `DESCRIBE` key
  argument produces valid TQL JSON with its typed `command` and `argument`
  details preserved.
- **Finding:** TQL success and failure output can share one bounded schema
  family without making inspect's historical output shape a hidden dependency.

### 2026-08-03 -- Typed TQL Parse Codes

- **Work completed:** Replaced the generic `CLI_ARGUMENT_INVALID` boundary
  code for TQL parser failures with stable TQL-specific codes for empty input,
  command and key limits, unknown commands, missing arguments, unexpected
  tokens, and invalid keys.
- **Validation:** Table-driven boundary tests cover every `TqlParseError`
  variant. The public-code documentation check and the provisional JSON error
  renderer test confirm the machine-readable contract stays synchronized.
- **Finding:** TQL can expose actionable parser diagnostics without making
  parser details a storage or trust contract.

### 2026-08-03 -- WAL Status Observation

- **Work completed:** Added `WAL STATUS` as a bounded operational inspection
  command backed only by `tosumu_core::inspect::inspect_wal()`. It reports
  sidecar existence and decoded record count, without exposing the sidecar
  path.
- **Read-only evidence:** The existing temporary-store proof now executes
  `WAL STATUS` and retains byte-for-byte equality for both the database and
  WAL. A CLI JSON test confirms the physical WAL path is not serialized.
- **Boundary:** This does not claim recovery success, checkpoint posture,
  durability, freshness, trust, or synchronization state.
- **Validation:** `cargo fmt --all -- --check`; `cargo test -p tosumu-cli tql`;
  `cargo clippy -p tosumu-cli --all-targets -- -D warnings`.

### 2026-08-03 -- CLI Scope Disclosure

- **Work completed:** Added TQL-specific long help that names the three
  admitted commands and explicitly excludes mutations, unlock material, shell
  pipelines, trust/freshness claims, and sync commands.
- **Validation:** A CLI-help test checks that the disclosed command set and
  read-only boundary stay present as the command surface evolves.
- **Finding:** The CLI can make its provisional scope visible at invocation
  time instead of requiring an operator to infer it from documentation.

### 2026-08-04 -- Operator Contract Reconciliation

- **Work completed:** Updated the TQL design to distinguish its implemented
  four-command read-only surface from deferred trust, SQL-view, sync, mutation,
  shell, and byte-input candidates. Added an operator reference that names the
  source facts, JSON/outcome boundary, successful missing-key and missing-WAL
  observations, process-status behavior, and non-guarantees for every admitted
  command.
- **Correction:** `STATUS` is documented as the existing `KvStore::stat()`
  count summary only. It does not claim store identity or format metadata that
  the current structured outcome does not contain.
- **Validation:** `cargo test -p tosumu-cli -q` passed (`112` unit and `1`
  integration test). Strict MkDocs validation remains blocked locally because
  the current Python environment has no `mkdocs` module and no `mkdocs`
  executable on `PATH`.
- **Finding:** The initial surface is now operator-documentable without
  overstating inspection facts. It remains CLI-local until an independent
  consumer establishes a stable reusable boundary.

### 2026-08-04 -- Bounded Description Observation Gap

- **Finding:** The current `DESCRIBE <key>` implementation calls public
  `KvStore::get()` and discards the returned bytes after observing their length.
  This is safe for the current bounded store API and does not disclose value
  content, but it is not evidence for a scalable metadata-only inspection
  contract.
- **Boundary:** Do not add a CLI-local page scan, physical record parser, or
  artificial output cap to conceal this behavior. If another consumer requires
  presence and length without materializing a value, design a provider-neutral
  metadata observation at that time and validate it independently.
- **Validation:** `cargo fmt --all -- --check` and
  `cargo clippy --workspace --all-targets -- -D warnings` completed. The
  combined `cargo test --workspace --all-targets` attempt reached the benchmark
  binary and exceeded the local command timeout; focused
  `cargo test -p tosumu-cli -q` remains passing (`112` unit and `1`
  integration test).

### 2026-08-04 -- Structured Rendering Fuzz Boundary

- **Work completed:** Moved TQL success rendering into a pure CLI-local
  `tql_render` module and added `fuzz_tql_render`. The target generates every
  admitted outcome variant with bounded synthetic facts, checks deterministic
  human/JSON rendering, validates the JSON schema marker, and asserts bounded
  output relative to the documented key limit. It never opens a store or
  invokes provider inspection.
- **Validation:** `cargo fmt --all -- --check`; `cargo test -p tosumu-cli -q`
  passed (`116` unit and `1` integration test); `cargo check --offline
  --manifest-path fuzz/Cargo.toml --bin fuzz_tql_render` passed.
- **Boundary:** This is compile and property-shape evidence only. Sustained
  libFuzzer execution remains pending the documented sanitizer-capable Linux
  workflow; the current Windows host cannot execute the existing targets.
- **Finding:** Output safety can be stress-tested without coupling the fuzzer
  to database opening, storage internals, or a public TQL crate.

### 2026-08-04 -- Validation Reconciliation

- **Work completed:** Re-ran the focused CLI suite, TQL-specific lint gate,
  diff check, and the offline structured-renderer fuzz build after the
  renderer extraction.
- **Validation:** `cargo test -p tosumu-cli -q` passed (`116` unit and `1`
  integration test); `cargo clippy -p tosumu-cli --all-targets -- -D warnings`
  passed; `cargo check --offline --manifest-path fuzz/Cargo.toml --bin
  fuzz_tql_render` passed; `git diff --check` passed. `cargo test --workspace
  --all-targets` exceeded the local `120` second command cap in broader
  targets without reporting a test failure. `mkdocs build --strict` remains
  unavailable in this shell: neither `mkdocs` on `PATH` nor
  `C:\Python314\python -m mkdocs` is installed.
- **Next evidence:** Run the existing Linux fuzz workflow and a strict MkDocs
  environment before using either validation result as completion evidence.

### 2026-08-04 -- Strict Documentation Validation Boundary

- **Work completed:** Reviewed the repository documentation workflow. Its
  `build` job installs `requirements-docs.txt` with Python 3.12 and executes
  `mkdocs build --strict` before uploading the Pages artifact.
- **Finding:** Strict documentation validation is a reproducible CI gate, not
  an absent TQL requirement. The local shell's missing `mkdocs` executable is
  an environment limitation only.
- **Remaining evidence:** A fresh successful workflow run is still required
  before strict MkDocs validation can count as evidence for this TQL change
  set. It remains separate from Linux sanitizer-backed fuzz execution.

### 2026-08-04 -- Linux Fuzz Workflow Completeness

- **Work completed:** Extended the existing weekly/manual Linux `Fuzz`
  workflow with a separate `tql-renderer` job. It installs `cargo-fuzz` under
  nightly and runs `fuzz_tql_render -- -runs=10000`; the existing parser job
  continues to run the same bounded count for `fuzz_tql_parse`.
- **Finding:** Both implemented untrusted-input boundaries now have a
  sanitizer-capable CI execution route. The pure renderer target remains
  isolated from database opening, while the parser target retains arbitrary
  UTF-8 grammar pressure.
- **Remaining evidence:** A successful run of the workflow is still pending.
  Adding the route is not equivalent to a completed fuzz campaign.

### 2026-08-04 -- Local Fuzz Target Reconciliation

- **Validation:** `cargo check --offline --manifest-path fuzz/Cargo.toml --bin
  fuzz_tql_parse --bin fuzz_tql_render` passed. `cargo test -p tosumu-cli -q`
  passed (`116` unit and `1` integration test), and `git diff --check` passed.
- **Environment boundary:** Windows' Python launcher is present, but no Python
  3.12 runtime is installed; local strict MkDocs validation therefore remains
  unavailable. The CI documentation job provides the supported Python 3.12
  validation route.
- **Next evidence:** Trigger or observe a successful `Docs` workflow and the
  weekly/manual Linux `Fuzz` workflow after this change set is committed.

### 2026-08-04 -- Current Slice Closure

- **Work completed:** Reconciled the Software Design Document with the exact
  four-command CLI-local implementation and added a parked-command register
  naming the absent capability and reopening trigger for every deferred command
  family.
- **Finding:** The initial outcome vocabulary does not contain trust,
  freshness, witness, conflict, or truth fields, so the corresponding
  no-fabricated-claim acceptance criterion is now supported by both code shape
  and the disclosure audit. That does not establish the absent capabilities.
- **Paused boundary:** Remaining work is intentionally external to the current
  CLI-local slice: sustained Linux fuzz execution, a fresh strict MkDocs CI
  run, virtual views, sync/evidence APIs, protected-provider disclosure
  evidence, or an independent consumer. No new command or public crate is
  justified until one of those inputs arrives.

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
