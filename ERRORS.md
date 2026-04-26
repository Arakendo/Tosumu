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

Internal modules may keep focused local error enums with `thiserror`. The structured boundary error should be a small shared shape used when the failure is understood and needs to travel across module or process boundaries.

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
pub struct AppError {
    pub code: &'static str,
    pub status: ErrorStatus,
    pub message: Cow<'static, str>,
    pub details: Vec<ErrorDetail>,
    pub source: Option<anyhow::Error>,
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
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

## Internal Errors vs Boundary Errors

Do not force every function in the repository to return `AppError`.

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
    pub fn into_app_error(self, pgno: u64) -> AppError {
        match self {
            OpenPageError::AuthFailed => AppError::new(
                codes::PAGE_AUTH_TAG_FAILED,
                ErrorStatus::IntegrityFailure,
                "page authentication failed",
            )
            .with_detail("pgno", ErrorValue::U64(pgno)),
            OpenPageError::Corrupt => AppError::new(
                codes::PAGE_DECODE_CORRUPT,
                ErrorStatus::IntegrityFailure,
                "page decode failed",
            )
            .with_detail("pgno", ErrorValue::U64(pgno)),
            OpenPageError::Io(err) => AppError::new(
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
  mod.rs           // AppError, ErrorStatus, ErrorDetail, AppResult
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

The current inspect contract already has a structured error envelope in [INSPECT_API.md](INSPECT_API.md), but it only exposes a coarse `kind` plus a message.

Current shape:

```json
{
  "kind": "invalid_argument",
  "message": "invalid argument: page number out of range",
  "pgno": null
}
```

Recommended migration path:

1. Keep schema version `1` stable.
2. Internally map failures to `AppError` first.
3. Preserve the current `error.kind` field as a compatibility alias.
4. Add richer fields only in a deliberate schema bump.

Suggested schema version `2` shape:

```json
{
  "kind": "wrong_key",
  "code": "PROTECTOR_UNLOCK_WRONG_KEY",
  "status": "permission_denied",
  "message": "database unlock failed with the provided protector material",
  "details": {
    "slot": 1,
    "operation": "inspect.header"
  }
}
```

The legacy `kind` field should remain until all known consumers have moved to `code` and `status`.

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

When a boundary logs an error, emit structured fields rather than one formatted blob.

Recommended log fields:

```txt
code=PAGE_AUTH_TAG_FAILED
status=IntegrityFailure
message="page authentication failed"
pgno=42
operation=inspect.page
source="aead tag mismatch"
```

The message should stay readable on its own, but `code`, `status`, and `details` are the stable fields downstream tools should rely on.

## Suggested First Rollout

1. Introduce `ErrorStatus`, `AppError`, and `codes.rs` in `tosumu-core`.
2. Convert the inspect/open/verify paths that already cross the CLI boundary.
3. Add CLI boundary mapping for:
   - human-readable stderr
   - inspect JSON envelopes
   - exit code selection
4. Lock the emitted values with focused tests.
5. Keep the WPF harness and future TUI consuming the inspect JSON contract rather than Rust internals.
6. Only after that, decide whether the inspect API needs schema version `2` richer error fields.

## Initial Code Set to Introduce First

Start with the errors already visible in the current inspect contract and engine failure modes:

```txt
CLI_ARGUMENT_INVALID
FILE_OPEN_BUSY
FILE_READ_FAILED
FORMAT_VERSION_UNSUPPORTED
INSPECT_PAGE_OUT_OF_RANGE
KEYSLOT_REGION_TAMPERED
PAGE_AUTH_TAG_FAILED
PAGE_DECODE_CORRUPT
PROTECTOR_UNLOCK_WRONG_KEY
VERIFY_BTREE_INVARIANT_FAILED
```

That is enough to prove the design without trying to catalog the entire repository on day one.

## Rules to Keep This Small

- No string parsing as control flow.
- No giant post-hoc error parser file.
- No one-enum-to-rule-them-all error hierarchy.
- No boundary-specific concepts inside domain code.
- No new shared crate until an actual second caller forces it.

The goal is not a framework. The goal is a stable, boring contract for failures that matter outside the function that produced them.