# Inspect API

The inspect API is the machine-readable contract exposed by `tosumu-cli` for downstream tools.

## What it is for

It exists so tools can inspect a database without scraping human-oriented CLI output.

Current consumers include:

- the TUI viewer
- the WPF harness
- future companion tools and diagnostics surfaces

## Common envelope

Each inspect JSON response uses a common top-level shape:

- `command`
- `ok`
- `payload`
- `error`

That means downstream tools can handle success and failure consistently across commands.

## Current command families

The current inspect surface includes:

- `inspect.header`
- `inspect.verify`
- `inspect.pages`
- `inspect.page`
- `inspect.wal`
- `inspect.tree`
- `inspect.protectors`

## Design intent

- one current baseline schema
- stable command identifiers
- field additions preferred over compatibility aliases
- Rust-side inspect types remain the source of truth

## Example uses

Read a structured header:

```sh
cargo run -- inspect header demo.tsm --json
```

Read structured verification output:

```sh
cargo run -- inspect verify demo.tsm --json
```

Inspect one decoded page:

```sh
cargo run -- inspect page demo.tsm --page 1 --json
```

## Where to look for exact fields

Use the repository `INSPECT_API.md` document and the Rust definitions in `crates/tosumu-cli/src/inspect_contract.rs` when you need the exact current envelope and payload fields.