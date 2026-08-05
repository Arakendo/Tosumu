# TQL Command Corpus v1

| Field | Value |
| --- | --- |
| Status | Incubating parser corpus |
| Opened | 2026-08-03 |
| Scope | Read-only TQL syntax and source-fact mapping; parsing itself performs no dispatch or persistence work |
| Governing review | [AR-0001: TQL Command Language Boundary](../Architectural%20Reviews/AR-0001-tql-command-language-boundary.md) |
| Implementation plan | [Tosumu Command Language](../Plans/tosumu-command-language.md) |

## Purpose

This corpus fixes the first bounded syntax boundary for Tosumu Command Language
(TQL). It distinguishes syntax Tosumu can parse today from command families
whose backing semantics remain deferred.

Parsing is deliberately inert. It opens no database, reads no WAL, performs no
SQL lowering, and produces no claims about trust, freshness, conflicts, or
sync.

## Initial Grammar

```text
STATUS
CHECK
DESCRIBE <key>
WAL STATUS
```

Commands are ASCII case-insensitive. Leading, trailing, and separating ASCII
whitespace is ignored. A `DESCRIBE` key is one non-control UTF-8 token, is
preserved exactly, and is not a quoted-string grammar in v1.

One input produces exactly one command. Semicolons and trailing tokens are not
command separators and are rejected as syntax.

## Limits

| Limit | Value | Reason |
| --- | ---: | --- |
| Command bytes | 4,096 | Bounds parser work before allocation or dispatch. |
| Tokens | 16 | Prevents a small grammar from accepting an unbounded token stream. |
| Key bytes | 1,024 | Keeps the initial text-key syntax below storage-format key limits. |
| Diagnostics | 32 | Reserved bound for the future dispatcher. The parser returns one typed error. |

## Accepted Cases

| Input | Typed command |
| --- | --- |
| `STATUS` | `Status` |
| ` status ` | `Status` |
| `CHECK` | `Check` |
| `DESCRIBE player/42` | `Describe { key: "player/42" }` |
| `describe assets/uber` | `Describe { key: "assets/uber" }` |
| `wal status` | `WalStatus` |

## Rejected Cases

| Input class | Result |
| --- | --- |
| Empty input | `EmptyInput` |
| Unknown command | `UnknownCommand` |
| `DESCRIBE` without key | `MissingArgument` |
| `WAL` without `STATUS` | `MissingArgument` |
| Extra token after any initial command | `UnexpectedToken` |
| `STATUS; CHECK` | `UnknownCommand`, not command chaining |
| More than 4,096 bytes | `InputTooLarge` |
| More than 16 tokens | `TooManyTokens` |
| Key over 1,024 bytes or containing controls | Typed key error |

## Source-Fact Inventory

| Command | Existing source of truth | Initial dispatch scope |
| --- | --- | --- |
| `STATUS` | `KvStore::stat()` and public inspection summaries | Store size and structural summary only. |
| `CHECK` | `tosumu_core::inspect::inspect_verification*` | Integrity/verification result only. |
| `DESCRIBE <key>` | `KvStore::get()` | Presence and safe value metadata only; no secrets or invented provenance. |
| `WAL STATUS` | `tosumu_core::inspect::inspect_wal()` | Sidecar existence and decoded record count only; no WAL path or recovery claims. |

`STATUS` does not imply current sync state. `CHECK` does not imply truth,
freshness, or a witness. `DESCRIBE` does not imply provenance, ownership, or
conflict history. `WAL STATUS` does not imply recovery success, checkpoint
health, durability, freshness, trust, or synchronization state.

## Explicitly Blocked Commands

The following remain grammar-design ideas until a source capability can provide
their facts:

- trust, witness, and anchored freshness commands;
- conflict and semantic sync commands;
- virtual-view query sugar such as `STALE` and `CONFLICTS`;
- mutation, watch, doctor, and shell-history commands.

## Validation

The executable parser corpus is in `crates/tosumu-cli/src/tql.rs`. Table-driven
tests prove accepted/rejected behavior and declared limits. A `proptest`
property test supplies arbitrary UTF-8 input and asserts deterministic,
panic-free parsing without a database handle.

The repository fuzz target `fuzz/fuzz_targets/fuzz_tql_parse.rs` supplies the
same parser with arbitrary bytes. Non-UTF-8 input is rejected before the
string-only grammar; valid UTF-8 must remain deterministic and panic-free. It
does not open a store or exercise TQL dispatch.

The target compiles with the repository's nightly toolchain. On the current
Windows development environment its libFuzzer execution is blocked by the
missing sanitizer runtime DLL; this is an environment limitation, not passing
fuzz evidence. The standard test suite separately property-tests bounded JSON
rendering for all initial `DESCRIBE` key lengths.

### 2026-08-03 -- WAL Status Observation

- **Work completed:** Added bounded `WAL STATUS` parsing and read-only dispatch
  through the public `inspect_wal()` summary. The structured result exposes
  only sidecar existence and decoded record count.
- **Validation:** The dispatch proof compares both database and WAL bytes
  before and after the command, and the CLI JSON test proves no filesystem
  path leaks into the result.
- **Finding:** A public physical-inspection fact can be operator-visible
  without promoting recovery, checkpoint, durability, freshness, trust, or
  sync conclusions into TQL.
