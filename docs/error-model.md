# Error Model

Tosumu is trying to make failures diagnosable instead of merely loud.

## Boundary-facing shape

When failures cross a module, process, or tool boundary, the design aims to preserve four pieces of information:

1. `code` — stable machine-readable identifier
2. `status` — small policy category such as `invalid_input` or `integrity_failure`
3. `message` — human-readable explanation
4. `details` — structured context such as page number, slot, path, or operation

This allows the CLI, inspect JSON, logs, and UI shells to respond to the same failure without parsing raw strings.

## Why this matters

Databases do not fail in one generic way. "Wrong key," "file busy," "corrupt page," and "unsupported format" are different events and should be reported as different events.

## Status categories

The status vocabulary is intentionally small. Current categories include:

- `invalid_input`
- `not_found`
- `conflict`
- `permission_denied`
- `busy`
- `integrity_failure`
- `external_failure`
- `unsupported`
- `internal`

## Example directions

- wrong key or rejected unlock: permission-style failure
- page auth failure or corruption: integrity failure
- file I/O or dependency issue: external failure
- invalid page argument from the user: invalid input

## Tooling implication

Downstream tools should key behavior off structured codes and statuses, not string matching. That is true for the TUI, the WPF harness, and any future website or API tooling.

## Source of truth

The durable design reference for structured errors lives in the repository `ERRORS.md` document. This page is the public summary.