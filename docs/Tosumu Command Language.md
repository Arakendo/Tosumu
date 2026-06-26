# TQL: Tosumu Command Language

## Status

Draft / exploratory long-range direction.

TQL is a Tosumu-specific command language for inspecting, explaining, and improving the epistemic state of stored data.

Naming note: the project keeps the `TQL` acronym for continuity, even though the current design is better described as a command language than a query language. Treat `TQL` as the stable subsystem name.

Long-range intent: TQL is primarily a surface layer, not a parallel execution engine.
Where possible, TQL should desugar into SQL over virtual views and explanatory query surfaces rather than introducing a separate planner or executor stack.

TQL is **not** a replacement for SQL.

TQL should feel more like a shell command language than a declarative relational language. Short forms such as `STATUS`, `STALE`, `CONFLICTS`, and `TRUST player/42` are a feature, not a concession.

SQL answers:

> What data matches this query?

TQL answers:

> Why should I trust this data, how fresh is it, who has witnessed it, and what would improve confidence?

---

## Design Goal

TQL exists to make Tosumu's native strengths accessible without forcing every application to reimplement sync, trust, freshness, and provenance logic.

Tosumu stores records.

TQL explains the database's understanding of those records.

### North Star

One practical north star for the project is that common synchronization should eventually feel as native and boring as transactions.

In the Rust API, the common case should be close to:

```rust
db.sync(peer, scope)?;
```

That line is not a promise about the final exact API shape.
It is a design target for the user experience: application code should not have to open-code sync bookkeeping, witness tracking, conflict explanation, provenance updates, and audit reasoning in every caller.
The caller may eventually supply a sync scope or policy, but should not have to own the underlying reasoning machinery.

The rich reasoning belongs on the operator side.
Programs should get a compact, structured result.
Humans should be able to ask follow-up questions in TQL such as `SYNC PREVIEW laptop`, `WHY CONFLICT player/42`, and `WHY LAST SYNC`.

---

## Non-Goals

TQL is not:

- a full SQL dialect
- a graph query language
- a stored procedure language
- an application scripting language
- a sync protocol by itself
- a replacement for the Rust API

TQL should remain small, inspectable, and boringly parseable.

Tiny parser goblin. Leashed.

---

## Relationship to SQL

Tosumu may support a toy SQL layer for ordinary relational-style access:

```sql
SELECT value FROM records WHERE key = 'player/42';
```

TQL sits beside that layer:

```tql
WHY player/42
TRUST player/42
TRACE player/42
STALE
SYNC PLAN peer:laptop
```

Long term, most TQL commands should desugar into SQL-facing virtual views or explanatory queries and then use the existing SQL pipeline:

```text
TQL surface
    ↓
desugar where possible
    ↓
SQL surface
    ↓
SQL AST
      ↓
Semantic check
      ↓
Plan
      ↓
Execution
```

The storage engine must not know or care whether a request began as raw SQL or TQL sugar.

Some TQL commands may remain operational verbs over inspect, verify, or sync APIs rather than lowering to SQL. Those should stay thin and should reuse the same underlying metadata and explanation logic rather than inventing separate semantics.

### Rule of Honest Lowering

A TQL command should lower to SQL if doing so preserves its meaning without distortion.
Otherwise it should remain a thin operational command.

This rule exists to prevent two common failures:

- forcing operational workflows into awkward fake-relational shapes
- inventing a second execution model for questions that SQL views can already answer cleanly

---

## Core Concepts

### Integrity

Whether stored bytes cryptographically verify.

Examples:

```tql
TRUST player/42
INTEGRITY FAILURES
```

Integrity answers:

> Has this data been tampered with or corrupted?

---

### Freshness

Whether the value is current relative to known witnesses or sync anchors.

Examples:

```tql
STALE
WHY STALE player/42
```

Freshness answers:

> Is this value probably current, or merely locally valid?

---

### Witnesses

Other devices, peers, logs, or anchors that have observed a state.

Examples:

```tql
WITNESSES player/42
UNWITNESSED
```

Witnesses answer:

> Who else has seen this state?

---

### Provenance

Where a value came from and how it changed.

Examples:

```tql
TRACE player/42
HISTORY player/42
```

Provenance answers:

> How did this record get here?

---

### Sync Need

Whether synchronization would improve confidence, freshness, or conflict resolution.

Examples:

```tql
NEEDS SYNC
SYNC PLAN peer:laptop
SYNC PREVIEW peer:laptop
```

Sync need answers:

> What would become more trustworthy if I synchronized now?

---

### Assurance

Whether the database can justify its current security, integrity, and operational posture.

Examples:

```tql
AUDIT
VERIFY
PROTECTORS
WAL STATUS
```

Assurance answers:

> What evidence do I have that this database is healthy, authenticated, and being operated safely?

---

## Command Families

TQL has two intended command families:

- Query sugar over SQL-facing virtual views and explanatory queries
- Thin operational verbs over inspect, verify, and sync surfaces when a command is not naturally relational

The first family is the preferred default. The second exists only where SQL would be an awkward or misleading surface.

The surface syntax should stay conversational and command-like. TQL should prefer direct forms such as `STALE`, `CONFLICTS`, `STATUS`, and `WHY player/42` over SQL-shaped ceremony when the shorter form stays clear.

### Inspection Commands

```tql
STATUS
CHECK
DESCRIBE player/42
```

`STATUS` summarizes database health.

`CHECK` verifies structural and cryptographic integrity.

`DESCRIBE` shows the value plus its metadata summary.

---

### Trust Commands

```tql
TRUST player/42
WHY player/42
WHY STALE player/42
WHY CONFLICT player/42
```

`TRUST` gives a compact trust summary.

`WHY` gives a human-readable explanation.

Example output:

```text
Record: player/42

Integrity:
    verified

Freshness:
    stale

Witnesses:
    local only

Reason:
    Record was modified locally after last sync.
    No remote witness has confirmed this version.

Recommended action:
    SYNC PREVIEW peer:laptop
```

---

### Sync Commands

```tql
NEEDS SYNC
STALE
CONFLICTS
SYNC PLAN peer:laptop
SYNC PREVIEW peer:laptop
SYNC APPLY peer:laptop
```

`SYNC PLAN` computes what would happen.

`SYNC PREVIEW` displays proposed send/receive/conflict effects.

`SYNC APPLY` performs the operation, subject to safety checks.

Example:

```text
Sync preview: peer:laptop

Send:
    18 records

Receive:
    7 records

Conflicts:
    2 records

Expected confidence gain:
    25 records gain an additional witness

Rollback risk:
    none detected
```

---

### Assurance Commands

```tql
AUDIT
VERIFY
PROTECTORS
REKEY STATUS
WAL STATUS
EVIDENCE player/42
WHY NOT FRESH player/42
```

`AUDIT` should summarize structural, cryptographic, and operational findings in a form suitable for review.

`VERIFY` should perform integrity and invariant checks and report exactly what passed, what failed, and what was not checked.

`PROTECTORS` should explain the active protector posture, including the difference between integrity-only sentinel protection and confidentiality from local readers.

`REKEY STATUS` should explain whether protector rotation or DEK rotation is advisable, overdue, or recently completed.

`WAL STATUS` should explain recovery and checkpoint posture in operator language rather than raw engine internals alone.

`EVIDENCE <key>` should show the supporting evidence behind a record's current trust and freshness standing.

`WHY NOT ...` should explain a missing property such as freshness, witness coverage, or sync readiness by naming the missing evidence and recommended next action.

These commands are especially important for security-sensitive and audit-heavy environments.
They should help an operator answer questions such as:

- What exactly was verified?
- Which protector model is active right now?
- Does this database currently provide integrity only, or both integrity and confidentiality against local file readers?
- Is the WAL clean, replayable, and checkpointed as expected?
- What evidence is missing before I can treat this record as fresh or witnessed?

---

### Provenance Commands

```tql
TRACE player/42
HISTORY player/42
RECENT
UNWITNESSED
```

`TRACE` should behave like record-level `git log` plus trust metadata.

Example:

```text
Record: player/42

Created:
    desktop @ LSN 104

Modified:
    mac-mini @ LSN 188

Witnessed:
    laptop @ LSN 190

Current standing:
    authenticated, witnessed, fresh
```

---

### Explain Commands

```tql
EXPLAIN SELECT * FROM records WHERE key = 'player/42';
WHY PLAN SELECT * FROM records WHERE key = 'player/42';
```

`EXPLAIN` describes execution.

`WHY PLAN` explains planner choice.

Example:

```text
Plan:
    primary key lookup

Reason:
    key predicate is exact
    no scan required

Estimated pages:
    1
```

---

## Virtual Views

TQL should eventually expose its concepts through virtual SQL views and related explanatory query surfaces. This is the canonical long-range design, not an optional convenience.

```sql
SELECT * FROM stale_records;
SELECT * FROM conflicted_records;
SELECT * FROM unwitnessed_records;
SELECT * FROM sync_candidates;
```

TQL sugar may lower into these views:

```tql
STALE
```

equivalent to:

```sql
SELECT * FROM stale_records;
```

This keeps TQL small and prevents duplicate semantics.

Recommended rule: if a TQL command can be expressed honestly as a SQL query over a virtual view, prefer that over adding a new execution path.

Corollary: if forcing a command through SQL would distort its meaning, lifecycle, or user expectations, keep it as a thin operational verb instead.

One brain for the parser goblin. Two hats maximum.

---

## MVP Scope

Initial implementable TQL should be tiny.

Long-range TQL can be broader than the first implementation, but the surface should grow by adding sugar over well-defined metadata and views, not by creating a second general-purpose query engine.

Recommended MVP:

```tql
STATUS
CHECK
DESCRIBE <key>
TRUST <key>
WHY <key>
STALE
CONFLICTS
NEEDS SYNC
SYNC PREVIEW <peer>
```

No mutation except possibly `SYNC APPLY`, and even that can wait.

First implementation should prove explanation, not power.

Important scope note:

- Query-like commands such as `STALE` should eventually lower to SQL-facing virtual views.
- Operational commands such as `STATUS`, `CHECK`, `SYNC PREVIEW`, and `SYNC APPLY` may remain thin wrappers over non-SQL engine surfaces.
- The existence of operational commands does not change the main design goal: TQL should stay mostly sugar, not a peer language runtime.

### Future Shell Commands

Long term, TQL will likely benefit from a few explicitly shell-like commands that focus on guidance rather than retrieval.

Examples worth preserving as future direction:

- `HELP` for command explanations, examples, and related commands
- `DOCTOR` for high-level diagnostic summaries and suggested next actions
- `WATCH` for continuously refreshed status-style views during sync or repair work

These are not part of the initial implementation plan.
They are listed here to capture the intended shell identity of TQL: a surface that helps users interrogate and understand the database, not just extract data from it.

### Future Command Reference

This section is a placeholder for a later operator-facing reference page.
The parser does not care about these categories. Humans will.

Recommended top-level groupings:

- Inspect: `STATUS`, `CHECK`, `DESCRIBE`, `RECENT`, `WITNESSES`
- Explain: `WHY`, `TRUST`, `TRACE`, `HISTORY`, `EVIDENCE`, `WHY NOT ...`
- Operate: `SYNC PLAN`, `SYNC PREVIEW`, `SYNC APPLY`, `WATCH`, `DOCTOR`
- Assure: `AUDIT`, `VERIFY`, `PROTECTORS`, `REKEY STATUS`, `WAL STATUS`

Likely reference fields for each command:

- Purpose
- Input shape
- Output shape
- Evidence sources consulted
- Recommended next actions
- Whether the command lowers to SQL, uses inspect/verify surfaces, or uses sync/operator surfaces

For security-sensitive deployments, the command reference should also identify which commands are suitable for:

- routine health checks
- incident response and tamper review
- key-management review
- pre-sync and post-sync audit trails
- operator evidence collection in regulated or high-assurance environments

The goal is not to claim certification or compliance by naming a command.
The goal is to make it easier for an operator or auditor to collect and explain the evidence Tosumu can actually provide.

Guardrail: TQL should remain the primary interactive command surface for operating and understanding a Tosumu database.
Features that naturally belong in an operator shell may be added over time.
Features whose primary purpose is relational data access belong in SQL.
Features whose primary purpose is embedding or low-level application integration belong in the Rust API.

---

## Safety Rules

TQL must not bypass storage invariants.

TQL must not:

* read unauthenticated pages
* mutate records during inspection commands
* silently ignore failed integrity checks
* hide conflict state
* treat stale-but-valid as fresh
* claim truth, only evidence

The database may say:

> This value is authenticated and witnessed.

It must not say:

> This value is true.

That distinction is the whole little philosophical raccoon driving the forklift.

---

## Example Session

```tql
STATUS
```

```text
Integrity:
    healthy

Freshness:
    93% fresh
    7% stale

Sync:
    14 records need sync

Conflicts:
    0
```

```tql
WHY player/42
```

```text
Record:
    player/42

Integrity:
    verified

Freshness:
    stale

Reason:
    local version has not been witnessed by any peer

Recommended action:
    SYNC PREVIEW peer:laptop
```

```tql
SYNC PREVIEW peer:laptop
```

```text
Send:
    player/42
    settings/theme

Receive:
    inventory/7

Conflicts:
    none

Expected result:
    2 local records gain witness
    1 remote record imported
```

---

## Design Principle

TQL exists because Tosumu treats stored data as evidence-bearing state.

SQL asks:

> What rows exist?

TQL asks:

> What does Tosumu know about those rows?

TQL is best understood as an evidence-oriented command language.
Its purpose is not merely to retrieve stored values, but to explain what the database believes, why it believes it, what evidence supports that belief, and what an operator should do next.
It is also the natural interactive shell surface for inspection, explanation, safety checks, synchronization, and operator-facing maintenance.

More concretely:

- SQL retrieves.
- TQL explains and operates.
- TQL should expose the database's internal reasoning, not just its final answer.
- TQL may grow ordinary operator-shell commands over time, so long as SQL remains the home for relational access and the Rust API remains the home for embedding.

In practice, that usually means:

- SQL remains the main structured query language.
- TQL provides a friendlier surface for trust, provenance, freshness, and sync-oriented questions.
- When possible, TQL phrases should desugar to SQL over virtual views instead of introducing separate execution semantics.

### Explanation First

TQL should optimize for understanding, not brevity.

If two commands can expose the same underlying fact, prefer the form that better explains the database's current knowledge, supporting evidence, and recommended next action.

That means TQL output should often do more than report state. It should help the user decide what to do next.

That is the whole point.