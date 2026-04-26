# Structured Error Design

This document defines the error-reporting model for `tosumu` across `tosumu-core`, `tosumu-cli`, the inspect JSON contract, and downstream tools such as the WPF harness and the planned TUI.

It complements [DESIGN.md](DESIGN.md) and [INSPECT_API.md](INSPECT_API.md). `DESIGN.md` remains the broader architectural source of truth; this file is the durable reference for how failures should be identified, categorized, propagated, and translated at system boundaries.

## Goals

- Give diagnosable failures a stable machine-readable identity.
- Keep status categories small enough to stay consistent across crates and processes.
- Preserve the original cause while adding structured context as errors propagate.
- Translate errors to CLI output, JSON envelopes, logs, and UI messages only at system boundaries.
- Make downstream tools consume one stable error contract instead of inferring behavior from strings.

## Non-Goals

- Building an "error router" or middleware pipeline.
- Parsing arbitrary error strings after the fact.
- Replacing every local `thiserror` enum with one global mega-enum.
- Teaching domain code about CLI exit codes, HTTP statuses, or WPF dialogs.
- Introducing a new shared workspace crate before the existing crates actually need one.

## Core Model

Errors that cross module or process boundaries should carry four pieces of information:

1. `code`: stable machine-readable identifier for the specific failure.
2. `status`: small category used for broad policy and boundary translation.
3. `message`: human-readable explanation.
4. `details`: structured fields such as `path`, `pgno`, `slot`, `operation`, or `format_version`.

The original source error should also be preserved whenever there is one.

### Error Code Format

Codes should use this pattern:

```txt
<AREA>_<OPERATION>_<REASON>
```

Examples:

```txt
CLI_ARGUMENT_INVALID
FILE_OPEN_BUSY
INSPECT_PAGE_OUT_OF_RANGE
PAGE_AUTH_TAG_FAILED
PAGE_DECODE_CORRUPT
PROTECTOR_UNLOCK_WRONG_KEY
FORMAT_VERSION_UNSUPPORTED
KEYSLOT_REGION_TAMPERED
```

Codes must be stable, searchable, and safe to expose in logs and machine-readable envelopes.

## Code Ownership

Each error code should have a clear owning module or subsystem. New codes should be introduced by the module that understands the failure, not at arbitrary call sites. Avoid reusing one code across unrelated failure modes just because the surface wording looks similar.

## Code Granularity

Do not introduce a new error code unless the failure needs to be distinguished for handling, logging, or user-facing behavior. Prefer adding structured details to an existing code over creating near-duplicate codes.

### Error Statuses

Statuses stay intentionally small and boring:

```rust
pub enum ErrorStatus {
    InvalidInput,
    NotFound,
    Conflict,
    PermissionDenied,
    Busy,
    IntegrityFailure,
    ExternalFailure,
    Unsupported,
    Internal,
}
```

Repository-specific guidance:

- Use `IntegrityFailure` for authenticated-decryption failures, tampering, corruption, and structural verification failures.
- Use `Busy` for lock or concurrent-access style failures such as the current `file_busy` inspect kind.
- Use `PermissionDenied` for wrong-key / unlock-denied style failures.
- Use `ExternalFailure` for OS, file I/O, and dependency failures where the external system is the immediate cause.

## Canonical Rust Shape

Internal modules may keep focused local error enums with `thiserror`. In this repository, the durable boundary-facing shape is `ErrorReport`, produced either directly or from `TosumuError::error_report()`. The important part is the structured report shape, not a specific type name like `AppError`.

Suggested minimal shape:

```rust
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorStatus {
    InvalidInput,
    NotFound,
    Conflict,
    PermissionDenied,
    Busy,
    IntegrityFailure,
    ExternalFailure,
    Unsupported,
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ErrorValue {
    Str(Cow<'static, str>),
    U64(u64),
    Bool(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ErrorDetail {
    pub key: &'static str,
    pub value: ErrorValue,
}

#[derive(Debug)]
pub struct ErrorReport {
    pub code: &'static str,
    pub status: ErrorStatus,
    pub message: Cow<'static, str>,
    pub details: Vec<ErrorDetail>,
    pub source: Option<anyhow::Error>,
}

impl ErrorReport {
    pub fn new(
        code: &'static str,
        status: ErrorStatus,
        message: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            code,
            status,
            message: message.into(),
            details: Vec::new(),
            source: None,
        }
    }

    pub fn with_source(mut self, source: impl Into<anyhow::Error>) -> Self {
        self.source = Some(source.into());
        self
    }

    pub fn with_detail(mut self, key: &'static str, value: ErrorValue) -> Self {
        self.details.push(ErrorDetail { key, value });
        self
    }
}
```

The important design choice here is `details`. Do not collapse structured context back into the message string. A message is for humans; `details` are for logs, JSON, tests, and tooling.

## Details Usage

Details should capture stable, machine-meaningful context such as identifiers, page numbers, offsets, slot indices, or operation names. Avoid placing large or unstructured data in details, and do not duplicate message text in detail fields.

## Internal Errors vs Boundary Errors

Do not force every function in the repository to return one shared boundary type everywhere.

The preferred shape is:

- local module errors remain small and role-focused
- the boundary-facing error type is created where the failure is understood
- boundary code translates structured errors into CLI output, inspect JSON, logs, or UI messages

That keeps domain code readable and avoids a repo-wide dependency on one huge enum.

Suggested pattern:

```rust
#[derive(thiserror::Error, Debug)]
pub enum OpenPageError {
    #[error("page authentication failed")]
    AuthFailed,
    #[error("page decode failed")]
    Corrupt,
    #[error("io failed: {0}")]
    Io(#[from] std::io::Error),
}

impl OpenPageError {
    pub fn into_error_report(self, pgno: u64) -> ErrorReport {
        match self {
            OpenPageError::AuthFailed => ErrorReport::new(
                codes::PAGE_AUTH_TAG_FAILED,
                ErrorStatus::IntegrityFailure,
                "page authentication failed",
            )
            .with_detail("pgno", ErrorValue::U64(pgno)),
            OpenPageError::Corrupt => ErrorReport::new(
                codes::PAGE_DECODE_CORRUPT,
                ErrorStatus::IntegrityFailure,
                "page decode failed",
            )
            .with_detail("pgno", ErrorValue::U64(pgno)),
            OpenPageError::Io(err) => ErrorReport::new(
                codes::FILE_READ_FAILED,
                ErrorStatus::ExternalFailure,
                "page read failed",
            )
            .with_detail("pgno", ErrorValue::U64(pgno))
            .with_source(err),
        }
    }
}
```

## Error Conversion

When converting local errors into a boundary `ErrorReport`, preserve the original cause whenever possible. Do not discard underlying errors or replace them with generic failures without attaching the source unless there is genuinely no useful underlying cause to preserve.

## Source of Truth for Codes

The canonical list of public codes should live in code, not only in markdown.

Recommended approach:

- define public codes as constants in Rust
- keep this document as the human design reference
- add a small test that the documented public-code list and the exported code constants stay in sync once the list is implemented

That avoids drift between a code catalog and the actual emitted values.

## Suggested Repository Layout

Start small. Do not create a new workspace crate yet.

Suggested first implementation:

```txt
crates/tosumu-core/src/error/
    mod.rs           // ErrorReport, ErrorStatus, ErrorDetail, TosumuError::error_report()
  codes.rs         // shared engine and inspect-facing codes

crates/tosumu-cli/src/error_boundary.rs
  // CLI exit-code mapping
  // inspect JSON error-envelope mapping
  // human-readable rendering
```

Why this shape:

- `tosumu-cli` already depends on `tosumu-core`
- core and inspect-facing failures should not be redefined in two places
- CLI-only codes can stay local to `tosumu-cli` until another crate genuinely needs them

Do not introduce `crates/tosumu-errors` unless at least one additional Rust crate besides `tosumu-cli` needs to share the same boundary-facing error types and code catalog.

## Inspect JSON Compatibility Plan

The current inspect contract in [INSPECT_API.md](INSPECT_API.md) should stay on one baseline shape with the structured fields that already carry the meaning.

Current shape:

```json
{
    "code": "ARGUMENT_INVALID",
    "status": "invalid_input",
  "message": "invalid argument: page number out of range",
  "pgno": null
}
```

Recommended baseline:

1. Use one structured inspect schema as the current baseline.
2. Internally map failures to a structured `ErrorReport` first.
3. Emit one canonical error shape with `code`, `status`, `message`, and relevant details.
4. Introduce a new schema version only when a real incompatibility is necessary.

## Inspect Verify Errors

Verification findings should remain in the inspect payload when the command can complete and report them. Use the top-level error envelope only when inspect cannot produce a meaningful result. For machine-stable handling, add structured issue codes inside the verify payload before promoting findings to boundary errors.

## Inspect Verify Incomplete States

Incomplete verify states should remain in the inspect payload when verify can still return a meaningful partial report. Promote them to a top-level inspect error only when the command cannot produce a reliable report envelope. Prefer structured payload issue codes for machine handling before changing the top-level error contract.

## Inspect Payload Issue Codes

When inspect returns a meaningful payload with findings, prefer stable payload issue codes over promoting those states to the top-level error envelope. Payload issue codes should be owned by the reporting payload that understands the state being classified.

For `inspect verify`, use payload codes to classify page findings and B-tree follow-up states. Keep these codes small and state-oriented.

Suggested current shape:

```txt
VERIFY_PAGE_AUTH_FAILED
VERIFY_PAGE_CORRUPT
VERIFY_PAGE_IO
VERIFY_BTREE_INVALID
VERIFY_BTREE_INCOMPLETE
```

Use these to represent reportable verify states such as authenticated page failure, corruption, I/O failure during page verification, invalid B-tree invariants after page verification, or incomplete B-tree verification because earlier findings or follow-up inspection could not complete.

## Plain-Text Verify Errors

Plain-text verify failures should return structured CLI boundary errors when verification cannot complete. Verification results that complete successfully but report invalid content should remain payload or output data. Avoid direct process exits inside command logic except for top-level CLI framework behavior such as help or version handling.

## Outcomes vs Errors

Use a boundary error when the command cannot complete the contract it advertises. Use a reported outcome when the command completes and the primary result is a finding, absence, or diagnostic state rather than a transport failure.

Examples:

- plain-text `verify` findings should remain reported output with exit policy decided at the top-level CLI boundary
- `inspect verify` findings and incomplete states should remain payload data while a meaningful report envelope can still be produced
- `get` for a missing key may return a structured `NotFound` boundary error because the command's contract is to retrieve one value and it cannot do so

Current inspect error shape:

```json
{
  "code": "PROTECTOR_UNLOCK_WRONG_KEY",
  "status": "permission_denied",
  "message": "database unlock failed with the provided protector material",
  "details": {
    "slot": 1,
    "operation": "inspect.header"
  }
}
```

## Status and Exit Code Mapping

Boundaries should usually translate by `status` first, with code-specific overrides when the product surface needs them.

Suggested CLI defaults:

| Status | Exit code |
|---|---:|
| `InvalidInput` | 2 |
| `NotFound` | 4 |
| `PermissionDenied` | 5 |
| `Conflict` | 6 |
| `Unsupported` | 7 |
| `Busy` | 8 |
| `ExternalFailure` | 9 |
| `IntegrityFailure` | 10 |
| `Internal` | 1 |

Examples of code-specific overrides that may still be reasonable:

- `CLI_ARGUMENT_INVALID` may print usage help in addition to the default exit code.
- `PROTECTOR_UNLOCK_WRONG_KEY` may suppress an internal-source chain in user-facing output while still logging it.
- `PAGE_AUTH_TAG_FAILED` and `PAGE_DECODE_CORRUPT` may share an exit code but should remain distinct codes for logs and tooling.

## Logging and Telemetry

When the CLI boundary logs an error, emit structured fields rather than one formatted blob.

Current CLI behavior:

- default human-facing output remains unchanged: human-readable stderr for normal CLI commands, structured JSON envelopes for `inspect ... --json`
- structured boundary error logs are opt-in via `TOSUMU_LOG_ERRORS=1` (also accepts `true`, `yes`, or `on`)
- the emitted log line is a single key-value record on stderr

Recommended log fields:

```txt
event=boundary_error
code=PAGE_AUTH_TAG_FAILED
status=integrity_failure
message="page authentication failed"
operation=inspect.page
pgno=42
source="The system cannot find the file specified."
```

The message should stay readable on its own, but `code`, `status`, `operation`, and structured detail fields are the stable fields downstream tools should rely on. `source` is optional and should be included only when there is a distinct underlying cause worth preserving.

## Logging Consistency

Structured logs should include at minimum `code`, `status`, `message`, and `operation`. When available, include relevant detail fields using their stable detail keys. Logs should remain machine-queryable without parsing free-form text.

## Suggested First Rollout

1. Introduce `ErrorStatus`, `ErrorReport`, and `codes.rs` in `tosumu-core`.
2. Convert the inspect/open/verify paths that already cross the CLI boundary.
3. Add CLI boundary mapping for:
   - human-readable stderr
   - inspect JSON envelopes
   - exit code selection
4. Lock the emitted values with focused tests.
5. Keep the WPF harness and future TUI consuming the inspect JSON contract rather than Rust internals.
6. Keep one structured inspect baseline until a real incompatible change forces a new schema version.

## Implemented Public Code Sets

The implemented code catalog is currently split between shared core codes in `tosumu-core` and a small number of CLI-local boundary codes in `tosumu-cli`.

Implemented core public codes:

<!-- BEGIN_CORE_PUBLIC_CODES -->
```txt
FILE_IO_FAILED
RECORD_CORRUPT
PAGE_DECODE_CORRUPT
PAGE_AUTH_TAG_FAILED
PAGE_ENCRYPT_FAILED
RNG_UNAVAILABLE
FILE_TRUNCATED
HANDLE_POISONED
FORMAT_NOT_TOSUMU
FORMAT_VERSION_UNSUPPORTED
PAGE_SIZE_MISMATCH
STORAGE_OUT_OF_SPACE
ARGUMENT_INVALID
INSPECT_PAGE_OUT_OF_RANGE
FILE_OPEN_BUSY
PROTECTOR_UNLOCK_WRONG_KEY
COMMITTED_FLUSH_FAILED
```
<!-- END_CORE_PUBLIC_CODES -->

Current CLI-local boundary codes:

<!-- BEGIN_CLI_PUBLIC_CODES -->
```txt
CLI_ARGUMENT_INVALID
CLI_KEY_NOT_FOUND
```
<!-- END_CLI_PUBLIC_CODES -->

Keep aspirational or not-yet-implemented codes out of these lists until they are actually emitted by code or claimed by a concrete rollout step.

## Rules to Keep This Small

- No string parsing as control flow.
- No giant post-hoc error parser file.
- No one-enum-to-rule-them-all error hierarchy.
- No boundary-specific concepts inside domain code.
- No new shared crate until an actual second caller forces it.

The goal is not a framework. The goal is a stable, boring contract for failures that matter outside the function that produced them.