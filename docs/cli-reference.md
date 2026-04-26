# CLI Reference

This is the public command map for the current CLI surface.

## Core storage commands

- `init <path>` — create a database
- `put <path> <key> <value>` — insert or update a key
- `get <path> <key>` — read a value
- `delete <path> <key>` — remove a key
- `scan <path>` — print all key-value pairs
- `stat <path>` — print database statistics

## Inspection commands

- `dump <path>` — pretty-print the header and optionally a page
- `hex <path> --page N` — raw page or header hex dump
- `verify <path> [--explain]` — authenticate and validate pages
- `view <path> [--watch]` — open the interactive read-only viewer
- `inspect ... --json` — structured machine-readable inspection output

## Inspect subcommands

- `inspect header <path>`
- `inspect verify <path>`
- `inspect pages <path>`
- `inspect page <path> --page N`
- `inspect wal <path>`
- `inspect tree <path>`
- `inspect protectors <path>`

Some inspect commands support explicit unlock options such as `--stdin-passphrase`, `--stdin-recovery-key`, and `--keyfile`.

## Backup and key management

- `backup <src> <dest>` — copy a database and its WAL sidecar
- `protector add-passphrase <path>`
- `protector add-recovery-key <path>`
- `protector add-keyfile <path> <keyfile>`
- `protector remove <path> --slot N`
- `protector list <path>`
- `rekey-kek <path> --slot N` — cheap KEK rotation for one slot

## Viewer keys

The TUI viewer currently supports:

- `/` — filter page list
- `n` / `N` — next or previous match
- `:` — jump to page
- `Tab` — switch focus
- `j` / `k` or arrow keys — move within the active pane
- `1` to `6` — switch panels
- `w` — toggle watch mode
- `q` — quit

## Exit behavior

The CLI distinguishes broad failure classes through structured statuses and exit codes. For machine-facing flows, prefer `inspect ... --json` over parsing human-readable stderr.