# MVP+9: Toy SQL Layer - Implementation Plan

Status: Draft - namespace-backed baseline selected.
Target MVP: MVP+9 (Stage 5 query layer)
Depends on: MVP+0 through MVP+8 complete and tested
Primary references: DESIGN.md Stage 5, DESIGN.md query-layer notes, current PageStore and CLI surfaces

---

## 1. Executive Summary

This plan is intentionally narrower than a "small database with SQL" rewrite.

The goal of MVP+9 is to prove three things:

1. A SQL surface can sit above the existing storage engine without violating layer boundaries.
2. Parsing, semantic checking, planning, and execution can be kept separate.
3. Prepared statements and planner output can exist without teaching `tosumu-core` about SQL concepts.

The baseline implementation should target the current stable storage boundary: `PageStore`.
It should not reach into `Pager` directly, should not use debug-only traversal as the canonical query path, and should not assume new on-disk reserved pages unless the format and core APIs are changed first.

Recommended baseline:

- New crate: `tosumu-sql`
- Executor over `tosumu_core::page_store::PageStore`
- Catalog stored in a reserved key namespace inside the existing KV surface
- Supported statements: `CREATE TABLE`, `INSERT`, `SELECT ... WHERE pk = ?`
- Prepared statements supported
- Planner implemented, but only for shapes the baseline can actually execute safely
- CLI entrypoint: `tosumu sql`

Deferred from the baseline:

- physical reserved catalog page
- per-table root pages
- direct `BTree` or `Pager` manipulation from the SQL layer
- arbitrary `WHERE` predicates
- full-table scan execution as the canonical SQL path
- `tosumu audit`
- joins, aggregates, secondary indexes, MVCC, `VACUUM`

---

## 1.1 Cline Implementation Checklist

Use this as the default execution order. Do not skip ahead unless a prior item is complete or explicitly blocked.

### Before writing code

- [ ] Re-read `DESIGN.md` Stage 5 and this plan before starting implementation work.
- [ ] Confirm the namespace-backed baseline is still the intended model.
- [ ] Create or update `.cline-worklog.md` with the task objective and starting assumptions.

### Phase 1: crate scaffold

- [ ] Add `crates/tosumu-sql/` to the workspace.
- [ ] Create `lib.rs`, `ast.rs`, `value.rs`, and `error.rs`.
- [ ] Add the smallest compiling public API skeleton first.
- [ ] Run `cargo test -p tosumu-sql` or the narrowest available compile/test check.

### Phase 2: parser pipeline

- [ ] Implement the lexer.
- [ ] Implement the parser for baseline grammar only.
- [ ] Add unit tests for supported statements.
- [ ] Add rejection tests for unsupported grammar.
- [ ] Re-run narrow validation before moving on.

### Phase 3: catalog and row encoding

- [ ] Implement reserved SQL key helpers.
- [ ] Implement catalog serialization/deserialization.
- [ ] Implement row key and row payload codecs.
- [ ] Add round-trip tests for catalog and row codecs.
- [ ] Re-run narrow validation before moving on.

### Phase 4: semantic checker and planner

- [ ] Implement semantic validation for `CREATE TABLE`, `INSERT`, and `SELECT ... WHERE pk = ?`.
- [ ] Implement the minimal planner for supported query shapes only.
- [ ] Return `UnsupportedQueryShape` for unsupported plans rather than inventing scan behavior.
- [ ] Re-run narrow validation before moving on.

### Phase 5: executor

- [ ] Implement execution over `PageStore` only.
- [ ] Use `PageStore::transaction(...)` where multi-step mutation needs atomicity.
- [ ] Do not call `Pager` directly.
- [ ] Do not use `scan_physical()` as the SQL query path.
- [ ] Add integration tests for `CREATE TABLE -> INSERT -> SELECT ... WHERE pk = ?`.
- [ ] Re-run narrow validation before moving on.

### Phase 6: prepared statements and CLI

- [ ] Implement `prepare()` without holding a long-lived mutable DB borrow.
- [ ] Implement `execute_prepared()` with bound values passed at execution time.
- [ ] Add `tosumu sql` to `tosumu-cli`.
- [ ] Add CLI tests for success and unsupported-query failures.
- [ ] Run `cargo test --workspace` and `cargo clippy --workspace --all-targets -- -D warnings`.

### Stop and escalate if any of these become necessary

- [ ] Adding a reserved physical catalog page
- [ ] Adding per-table root pages
- [ ] Adding direct `Pager` or `BTree` internal dependencies to `tosumu-sql`
- [ ] Adding general full-table scan support to make unsupported SQL "work"
- [ ] Pulling Stage 6 or Stage 7 ideas into MVP+9 baseline

### Before yielding incomplete work

- [ ] Update `.cline-worklog.md` with findings, decisions, files touched, validations, blockers, and next step.
- [ ] Note whether the current state is safe to resume from or needs cleanup first.

---

## 2. Non-Negotiable Constraints

These constraints exist to keep the SQL layer compatible with both the current codebase and the design principles.

### 2.1 Layering

Required dependency direction:

```text
tosumu-cli -> tosumu-sql -> tosumu-core
```

The SQL layer must not depend on CLI, TUI, or WPF-specific code.

### 2.2 Storage boundary

The MVP+9 executor should use `PageStore` as its storage boundary.

Do not:

- call `Pager` directly from `tosumu-sql`
- depend on `BTree` crate-private transaction internals
- treat `scan_physical()` as the canonical SQL row scan path

Reason:

- `PageStore` is the current high-level public owner of put/get/delete/scan/transaction behavior.
- `scan_physical()` is explicitly documented as a debugging and verification surface, not a relational execution primitive.

### 2.3 MVP discipline

Do not widen MVP+9 with Stage 6 or Stage 7 ideas.

Specifically out of scope for the baseline:

- service or daemon work
- witness, freshness-anchor, or network-aware SQL features
- optimizer work beyond a tiny plan classifier
- statistics, histograms, or `ANALYZE`
- schema migrations beyond what is required for the first SQL catalog entries

### 2.4 No fake format stories

Do not claim a reserved page, separate root page, or multiple logical table roots unless the plan also includes the core and format work required to make that true.

The current engine has one root page per store. Any plan that assumes more than that must either:

1. add a core prerequisite phase first, or
2. use the current single-tree KV surface honestly.

This document recommends option 2 for MVP+9 baseline.

---

## 3. Current Codebase Facts That Shape The Plan

These facts are important because they determine what is realistic without a core refactor.

### 3.1 Stable current boundary: PageStore

`PageStore` already exposes:

- `put`
- `get`
- `delete`
- `scan`
- `scan_range`
- `transaction`

That makes it the correct substrate for a first SQL layer.

### 3.2 Current root-page model

Today a new `BTree` allocates a root page and stores that root in pager metadata.
There is no public API for "open a different logical table tree by root page" from the SQL layer.

Implication:

- a per-table root-page design is not a free add-on
- it requires explicit core and format work before SQL executor coding starts

### 3.3 Current query-layer intent in DESIGN

The design already expects:

- SQL string -> lexer -> parser -> AST -> semantic checker -> planner -> executor
- prepared statements based on AST nodes with `Parameter(usize)`
- planner output before execution

The revised plan preserves that shape.

---

## 4. Selected MVP+9 Storage Model

### 4.1 Selected baseline: namespace-backed catalog and rows

For MVP+9 baseline, store SQL metadata and row data inside the existing KV tree through reserved key prefixes.

This avoids inventing a new root-page model before the query layer exists.

Recommended reserved namespaces:

```text
__sql_catalog__/table/<table_name>          -> serialized TableDef
__sql_catalog__/meta/version                -> catalog format version
__sql_row__/<table_name>/<encoded_pk>       -> serialized row payload
```

Properties:

- no new page types
- no special reserved physical page
- no new public core API required for multiple roots
- easy to inspect using existing KV and debug tooling
- easy to migrate later if the project introduces dedicated table roots

### 4.2 Why this is the recommended baseline

This approach matches the current engine better because:

- the current engine already stores opaque key-value pairs well
- table namespacing can be expressed at the SQL layer
- catalog access can be implemented entirely through `PageStore`
- it keeps `tosumu-core` free of SQL semantics during the first implementation

### 4.3 What this baseline does not claim

This baseline does not claim:

- one B+ tree per table
- a dedicated system catalog page
- direct root-page ownership by SQL tables

If the project wants those properties, that should be a separate prerequisite phase before SQL executor work begins.

### 4.4 Future-compatible catalog payload

Even though the baseline uses one KV tree, the catalog payload can still include optional future-facing fields such as `root_page`.

Recommended rule:

- baseline implementation stores `root_page: None`
- future multi-root implementation may populate it

That keeps the catalog shape forward-compatible without lying about current storage behavior.

---

## 5. Scope

### 5.1 Baseline in scope

- new `tosumu-sql` crate
- SQL lexer
- SQL parser
- AST types
- semantic checker
- small planner
- executor over `PageStore`
- prepared statements
- `CREATE TABLE`
- `INSERT`
- `SELECT ... WHERE pk = ?`
- CLI subcommand: `tosumu sql`

### 5.2 Optional follow-on inside MVP+9 only if baseline lands cleanly

- `DELETE ... WHERE pk = ?`
- `SELECT ... WHERE pk = <literal>` in addition to bound parameters
- `--explain` output for the SQL command

### 5.3 Explicitly out of scope for baseline

- arbitrary predicates over non-PK columns
- `AND`, `OR`, `<`, `>`, range predicates, `LIKE`, `IN`
- full table scans as a user-visible success path
- joins
- aggregates
- secondary indexes
- catalog-on-reserved-page format work
- `tosumu audit`
- JSON audit output
- planner row-count estimates derived from page counts

### 5.4 Error policy for unsupported SQL

Unsupported queries should fail explicitly with a stable SQL-layer error.

Do not silently degrade unsupported shapes into whole-database scans.

Examples that should return `UnsupportedQueryShape` in baseline:

- `SELECT * FROM users` without a required primary-key equality predicate
- `SELECT * FROM users WHERE email = ?`
- `SELECT * FROM users WHERE id = ? AND name = ?`
- `DELETE FROM users`

---

## 6. Crate Structure

```text
crates/
├── tosumu-core/
├── tosumu-cli/
└── tosumu-sql/
    ├── Cargo.toml
    └── src/
        ├── lib.rs
        ├── ast.rs
        ├── lexer.rs
        ├── parser.rs
        ├── semantic.rs
        ├── planner.rs
        ├── executor.rs
        ├── catalog.rs
        ├── row_codec.rs
        ├── value.rs
        └── error.rs
```

Module responsibilities:

- `ast.rs`: statement and expression types
- `lexer.rs`: SQL tokenizer
- `parser.rs`: recursive descent parser
- `semantic.rs`: schema-aware validation
- `planner.rs`: classify supported vs unsupported query shapes
- `executor.rs`: execute plans through `PageStore`
- `catalog.rs`: catalog key encoding, schema storage, lookup
- `row_codec.rs`: row serialization and projection decoding
- `value.rs`: SQL value representation and coercion helpers
- `error.rs`: SQL-layer error type with structured mapping into core errors
- `lib.rs`: public API surface

---

## 7. Public API Shape

The public API should avoid long-lived mutable borrows held inside prepared statements.

Recommended shape:

```rust
pub struct SqlDatabase {
    store: tosumu_core::page_store::PageStore,
}

pub struct PreparedStatement {
    stmt: Stmt,
    parameter_count: usize,
}

pub enum QueryResult {
    Rows {
        columns: Vec<String>,
        rows: Vec<Vec<Value>>,
    },
    Affected {
        rows: usize,
    },
}

pub struct ExecutionOutcome {
    pub result: QueryResult,
    pub warnings: Vec<PlanWarning>,
}

impl SqlDatabase {
    pub fn open(path: &Path) -> Result<Self, SqlError>;
    pub fn prepare(&self, sql: &str) -> Result<PreparedStatement, SqlError>;
    pub fn execute_prepared(
        &mut self,
        stmt: &PreparedStatement,
        bindings: &[Value],
    ) -> Result<ExecutionOutcome, SqlError>;
    pub fn execute(&mut self, sql: &str) -> Result<ExecutionOutcome, SqlError>;
}
```

Why this shape:

- `prepare(&self)` can parse and count parameters without borrowing the database mutably
- statement reuse does not pin a mutable database borrow
- semantic checking and planning can still run at execute time against the current catalog

---

## 8. SQL Surface For Baseline

### 8.1 Supported statements

Baseline statement set:

```text
CREATE TABLE <ident> (
    <pk_name> INTEGER|TEXT|BLOB PRIMARY KEY,
    <col_name> INTEGER|TEXT|BLOB,
    ...
)

INSERT INTO <ident> VALUES (...)

SELECT <projection> FROM <ident> WHERE <pk_name> = ?
SELECT <projection> FROM <ident> WHERE <pk_name> = <literal>
```

Recommended baseline restriction:

- exactly one primary key column
- no implicit rowid
- no null primary key
- no expression evaluation beyond literal and parameter substitution
- `projection` may be `*` or an explicit column list, but only for primary-key equality lookups

### 8.2 AST shape

Keep the AST narrower than the previous draft.

Recommended baseline AST:

```rust
pub enum Stmt {
    CreateTable {
        name: String,
        columns: Vec<ColumnDef>,
    },
    Insert {
        table: String,
        values: Vec<Expr>,
    },
    Select {
        table: String,
        columns: Projection,
        predicate: Option<Expr>,
    },
    Delete {
        table: String,
        predicate: Option<Expr>,
    },
}

pub enum Projection {
    All,
    Named(Vec<String>),
}

pub enum Expr {
    Literal(Value),
    Column(String),
    Eq(Box<Expr>, Box<Expr>),
    Parameter(usize),
}
```

Important note:

- `Delete` may exist in the AST now for forward compatibility
- baseline execution support for `Delete` is optional and should come after create/insert/select are stable
- do not implement `And`, `Or`, range operators, or arbitrary boolean expressions in baseline

### 8.3 Parser grammar

Keep the grammar deliberately tiny:

```text
Stmt        -> CreateTable | Insert | Select | Delete
CreateTable -> CREATE TABLE ident '(' ColumnDef (',' ColumnDef)* ')'
ColumnDef   -> ident TypeName ('PRIMARY' 'KEY')?
TypeName    -> INTEGER | TEXT | BLOB
Insert      -> INSERT INTO ident VALUES '(' Expr (',' Expr)* ')'
Select      -> SELECT Projection FROM ident WHERE EqExpr
Delete      -> DELETE FROM ident WHERE EqExpr
Projection  -> '*' | ident (',' ident)*
EqExpr      -> Operand '=' Operand
Operand     -> Literal | ident | '?'
```

Parser rules:

- accept only single-statement input
- optional trailing semicolon is fine
- reject unsupported grammar early instead of producing a huge AST for later rejection

---

## 9. Catalog Model

### 9.1 Catalog keys

Recommended keys:

```text
__sql_catalog__/meta/version
__sql_catalog__/table/<table_name>
```

### 9.2 Catalog value

Recommended `TableDef` payload:

```rust
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub primary_key_index: usize,
    pub root_page: Option<u64>,
}
```

`root_page` remains `None` in the baseline single-tree implementation.

### 9.3 Catalog serialization

Use a small explicit binary format owned by `tosumu-sql`.

Do not introduce serde as a dependency for MVP+9 unless there is a compelling reason.

Recommended wire shape:

```text
[version: u8]
[table_name_len: u16][table_name bytes]
[column_count: u16]
[pk_index: u16]
[root_page_present: u8]
[root_page: u64 if present]
repeat column_count times:
  [name_len: u16][name bytes]
  [type_tag: u8]
  [is_primary_key: u8]
```

### 9.4 Catalog lifecycle

- `CREATE TABLE` checks for existing catalog entry
- on success it writes one catalog record through `PageStore::put`
- no separate catalog bootstrap page is needed
- if no `__sql_catalog__/meta/version` key exists, initialize it lazily on first SQL write

---

## 10. Row Encoding Model

### 10.1 Row keys

Recommended key format:

```text
__sql_row__/<table_name>/<encoded_pk>
```

This keeps SQL rows out of the user-facing raw KV namespace while still using the existing storage engine honestly.

### 10.2 Row values

Store non-key column values in a compact binary row payload.

Recommended baseline row format:

```text
[version: u8]
[column_count: u16]
repeat column_count times:
  [type_tag: u8]
  [payload_len: u32]
  [payload bytes]
```

The primary key may be duplicated in the row payload for simplicity in MVP+9.
That is acceptable for the first implementation because clarity beats space efficiency here.

### 10.3 Value types

Baseline SQL types:

- `INTEGER`
- `TEXT`
- `BLOB`

Defer `REAL` and `NULL` unless the team explicitly wants them in the first cut.

Reason:

- `REAL` adds coercion and comparison edge cases
- `NULL` forces early three-valued logic questions
- neither is required to prove the parser/planner/executor pipeline

---

## 11. Semantic Checking

The semantic checker should validate everything it can before any storage mutation starts.

Recommended checks:

### 11.1 CREATE TABLE

- table name is not empty
- table name does not use reserved SQL namespaces
- column names are unique
- exactly one primary key exists
- column types are supported in baseline

### 11.2 INSERT

- table exists
- value count matches column count
- primary key value is present and type-correct
- no parameter remains unbound at execution time

### 11.3 SELECT

- table exists
- projected columns exist
- predicate shape is exactly `pk_column = <literal|parameter>`
- `SELECT *` is allowed only when that primary-key equality predicate is present

### 11.4 DELETE

- same predicate restrictions as baseline `SELECT`
- only enable once delete execution support is explicitly turned on

Recommended SQL errors:

- `TableNotFound`
- `ColumnNotFound`
- `DuplicateColumn`
- `MissingPrimaryKey`
- `UnsupportedType`
- `UnsupportedQueryShape`
- `BindingCountMismatch`
- `TypeMismatch`

---

## 12. Planner

### 12.1 Baseline planner scope

The baseline planner should classify only the shapes the executor can actually run.

Recommended plan enum:

```rust
pub enum PlanNode {
    CreateTable { table: String },
    InsertRow { table: String, pk: Value },
    PkLookup { table: String, pk: Value, projection: Projection },
    DeleteByPk { table: String, pk: Value },
}
```

Recommended warning enum:

```rust
pub enum PlanWarning {
    SelectStar { table: String },
}
```

### 12.2 Important baseline rule

If the planner cannot produce one of the plan nodes above, it should return `UnsupportedQueryShape`.

Do not manufacture `FullScan` as a success path yet.

### 12.3 Why full scans are deferred

The earlier draft proposed full scans based on `scan_physical()` and estimated row counts from page counts.
That should be removed from the baseline because:

- `scan_physical()` is a debug/verification traversal, not a canonical query primitive
- `page_count * constant` is not a trustworthy row estimate for SQL planning
- a "successful" full scan path would broaden MVP+9 considerably

If a logical SQL scan path is added later, it should be designed explicitly, not inherited accidentally from a debug API.

---

## 13. Executor

### 13.1 Baseline executor boundary

The executor owns:

- catalog lookup
- row-key encoding
- row-value serialization and decoding
- projection shaping
- mapping SQL errors to structured results

It should call `PageStore` methods only.

### 13.2 Execution strategies

| Plan | Execution |
|------|-----------|
| `CreateTable` | Write catalog entry through `PageStore::put` |
| `InsertRow` | Encode row key + row payload, then `PageStore::put` |
| `PkLookup` | Encode row key, `PageStore::get`, decode row, project columns |
| `DeleteByPk` | Encode row key, `PageStore::delete` |

### 13.3 Transaction use

Use `PageStore::transaction(...)` for multi-step mutations where needed.

Examples:

- `CREATE TABLE` may need to initialize catalog version and table entry atomically
- `INSERT` may eventually want to maintain row count metadata atomically

If row-count bookkeeping is not implemented in baseline, keep mutations even simpler.

### 13.4 What the executor must not do

- inspect page headers directly
- call `Pager::allocate`
- call `BTree` crate-private transaction helpers
- reserve physical page 1

---

## 14. Prepared Statements

### 14.1 Preparation model

`prepare()` should:

- lex and parse SQL
- count `?` parameters
- store the AST

It should not require a mutable database borrow.

### 14.2 Execution model

`execute_prepared()` should:

- validate binding count
- substitute parameter values into a transient bound AST or evaluation context
- run semantic checking against the current catalog
- plan
- execute

### 14.3 Binding rules

Recommended baseline:

- positional parameters only
- 1-based binding index at API edge if that is more SQL-like, or 0-based if consistency with Rust APIs is preferred
- document the choice explicitly and test it

Do not keep mutable binding state inside a database-borrowing statement object unless there is a strong reason to do so.

---

## 15. CLI Integration

### 15.1 Baseline CLI command

Add a new subcommand to `tosumu-cli`:

```text
tosumu sql <path> <query> [--param <value>]...
```

Recommended follow-on flag:

```text
tosumu sql --explain <path> <query> [--param <value>]...
```

### 15.2 Baseline CLI output

For successful PK lookup:

```text
$ tosumu sql db.tsm "SELECT * FROM users WHERE id = ?" --param 1
id | name
---+------
1  | alice
```

For unsupported query shape:

```text
$ tosumu sql db.tsm "SELECT * FROM users"
error[SQL_UNSUPPORTED_QUERY_SHAPE]: baseline SQL supports only primary-key equality lookups
```

### 15.3 No audit command in baseline

Do not add `tosumu audit` in MVP+9 baseline.

Reason:

- it is a separate diagnostics product surface
- it broadens scope beyond the query pipeline
- existing inspect/verify tooling already owns the low-level diagnostic story

If audit work is desired later, it should become its own milestone and design section.

---

## 16. Implementation Phases

### Phase 0: Design sync for the selected model

Do this before writing code.

- Update the nearest design notes so the docs do not keep promising a reserved catalog page for the first implementation.
- Keep the namespace-backed baseline as the default unless the user explicitly reopens the storage-model decision.
- If a future task reopens the question and prefers per-table roots, stop and write a core prerequisite plan before coding the SQL executor.

Gate: no coding until the design-doc sync for the selected namespace-backed model is landed.

### Phase 1: Crate skeleton and AST

- create `crates/tosumu-sql`
- add `lib.rs`, `ast.rs`, `value.rs`, `error.rs`
- define baseline statement, expression, projection, and value types
- add unit tests for AST helpers and parameter counting

### Phase 2: Lexer and parser

- implement tokenizer
- implement parser for baseline grammar only
- reject multi-statement input and unsupported syntax early
- add unit tests for happy-path and rejection-path parsing
- add property tests that valid baseline inputs tokenize and parse without panic

### Phase 3: Catalog and row codec

- implement catalog key helpers
- implement catalog serialization and deserialization
- implement row key encoding
- implement row payload encoding and decoding
- add unit tests for round-trip encoding

### Phase 4: Semantic checker and planner

- implement schema validation
- implement query-shape validation
- implement narrow planner for supported plans only
- add tests for each supported and rejected statement shape

### Phase 5: Executor over PageStore

- implement `SqlDatabase`
- implement `execute()` and `execute_prepared()`
- use `PageStore` only
- add integration tests for create/insert/select-by-pk round-trip

### Phase 6: CLI integration

- add `sql` subcommand to `tosumu-cli`
- render row output and structured errors cleanly
- add CLI tests for supported and unsupported SQL

### Phase 7: Optional follow-on inside MVP+9

Only after phases 1-6 pass cleanly:

- add `DELETE ... WHERE pk = ?`
- add `--explain`
- add `SELECT *` warning output if useful

---

## 17. Testing Strategy

### 17.1 Unit tests

- lexer tokenization of keywords, identifiers, literals, punctuation, and `?`
- parser success cases for create/insert/select-by-pk
- parser rejection for unsupported grammar
- catalog serialization round-trip
- row codec round-trip
- semantic checker success and failure paths
- planner classification success and failure paths

### 17.2 Property tests

- lexer/parser never panic on valid baseline grammar inputs
- catalog and row codecs round-trip for generated values within supported type bounds
- prepared statement parameter counting matches bound placeholders

### 17.3 Integration tests

- `CREATE TABLE` then `INSERT` then `SELECT ... WHERE pk = ?`
- prepared statement reuse across multiple bindings
- duplicate table creation rejected
- duplicate PK overwrite semantics match chosen baseline policy and are documented
- unsupported full-scan query rejected with the expected SQL error

### 17.4 CLI tests

- `tosumu sql` executes a baseline point lookup successfully
- unsupported shape prints stable boundary error
- explain mode, if added, prints plan before execution

### 17.5 Validation commands

Required validation once implementation begins:

```text
cargo test -p tosumu-sql
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

---

## 18. Acceptance Criteria

Baseline MVP+9 is complete when all of the following are true:

1. `tosumu-sql` exists as a separate crate with no CLI/TUI dependencies.
2. `CREATE TABLE`, `INSERT`, and `SELECT ... WHERE pk = ?` work through the SQL surface.
3. Prepared statements work without holding a long-lived mutable database borrow.
4. The executor uses `PageStore`, not `Pager`, and not `scan_physical()` as its SQL execution path.
5. Unsupported query shapes fail explicitly instead of silently degrading to scans.
6. `cargo test -p tosumu-sql` passes.
7. `cargo test --workspace` passes.
8. `cargo clippy --workspace --all-targets -- -D warnings` passes.
9. Any required design-doc sync for the chosen catalog model is landed before or alongside implementation.

---

## 19. Known Follow-On Work After MVP+9

These are not baseline tasks.

### 19.1 Table-aware storage refactor

If the project still wants per-table root pages and a physical system catalog page, that should be a dedicated follow-on design and core milestone.

That work would likely require:

- format updates
- core APIs for opening or owning multiple logical trees
- migration strategy for namespace-backed SQL rows, if baseline ships first

### 19.2 Logical scan support

If full-table scans become desirable later, they should use an explicit logical row traversal owned by a stable storage boundary.
Do not promote `scan_physical()` into that role by accident.

### 19.3 Richer SQL

Only after the baseline is stable:

- secondary indexes
- non-PK predicates
- `AND` / `OR`
- joins
- aggregates
- statistics and estimates

---

## 20. Open Questions

These should be resolved explicitly, not by code drift.

1. Should baseline SQL support `DELETE ... WHERE pk = ?`, or should delete wait until after create/insert/select are stable?
2. Should the baseline include `TEXT` and `BLOB` immediately, or land `INTEGER` first and add the others once the pipeline is proven?
3. Should `SELECT *` be supported in the baseline, or should projections require explicit column names until row decoding is settled?