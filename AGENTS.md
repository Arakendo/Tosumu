# Tosumu Codex Instructions

## Project Intent

Tosumu is an experimental Rust-native embedded storage engine focused on
inspectability, explicit failure reporting, authenticated storage, and clear
separation between physical storage mechanics and higher-level data meaning.

The current source-of-truth design document is `docs/Specifications/Tosumu Software Design Document.md`. Security claims
must remain consistent with `SECURITY.md`, and public error and inspection
contracts must remain consistent with `docs/Specifications/Tosumu Error Design Document.md` and `docs/Specifications/Tosumu Inspect API Specification.md`.

## Architecture Boundaries

- Keep `tosumu-core` focused on storage mechanics and provider-neutral storage
  contracts: pages, records, B+ trees, WAL, recovery, integrity, and bounded
  inspection.
- Keep SQL, table, constraint, CLI, .NET, and consumer-specific semantics above
  `tosumu-core`.
- Do not teach the pager about tables, Tokimu assets, application schemas, or
  presentation concepts.
- Treat the on-disk format as a compatibility boundary. Format changes require
  explicit documentation, migration behavior, and focused validation.
- Prefer typed failures and explicit diagnostics over fallback, repair by
  accident, or silently accepting ambiguous storage state.
- Do not strengthen security, durability, freshness, or recovery claims beyond
  the evidence recorded in the design and security documents.

## Documentation Authority

- `docs/Specifications/Tosumu Software Design Document.md`, `docs/Specifications/Tosumu Error Design Document.md`, `docs/Specifications/Tosumu Inspect API Specification.md`, and `SECURITY.md` are normative
  engineering specifications.
- `docs/ADR/` records accepted architectural decisions. Do not work around an
  ADR locally; revise or supersede it deliberately when the decision changes.
- `docs/Architectural Reviews/` records unresolved questions, evidence,
  alternatives, dispositions, and reopening triggers. Reviews do not override
  accepted ADRs.
- `docs/Plans/` sequences implementation work. A plan is not architectural
  authority.
- `docs/CRs/` contains incoming consumer or cross-project change requests. A CR
  does not become a Tosumu commitment until it is accepted and planned.
- `docs/Notes/` and `docs/Conversations/` are non-binding supporting material.
- `.workbench/` is ignored local working material and must not become the only
  record of a durable decision.

Read relevant ADRs and Architectural Reviews before adding a subsystem,
changing ownership, changing the file format, or adding a dependency across an
established boundary.

## Design Habits

- Stabilize public traits only after independent callers reveal the contract.
- Separate observation from guarantee, especially for durability, freshness,
  corruption detection, and performance.
- Prefer structural impossibility over advisory comments.
- Keep early implementations small, testable, and explicit.
- Record unsupported behavior as a diagnostic rather than guessing.
- Keep consumer semantics with the consumer. Tosumu moves and protects bytes;
  it does not redefine the application meaning stored in those bytes.

## Validation

After code changes, prefer:

- `cargo fmt --all -- --check`
- `cargo clippy --workspace --all-targets -- -D warnings`
- `cargo test --workspace --all-targets`
- `mkdocs build --strict` after public documentation or navigation changes

New public APIs should have focused tests and at least one real caller.

