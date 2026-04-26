# Architecture

Tosumu is organized as a small stack of focused components rather than one giant binary.

## Main parts

### `tosumu-core`

The core crate owns the engine behavior:

- on-disk format
- pager
- B+ tree
- WAL and recovery
- crypto and keyslot handling
- structured engine-facing errors

This is the canonical owner of storage semantics.

### `tosumu-cli`

The CLI exposes the engine as a command-line tool:

- database lifecycle commands such as `init`, `put`, `get`, `scan`, and `backup`
- inspection commands such as `dump`, `hex`, `verify`, and `inspect ... --json`
- protector management commands
- the read-only TUI viewer (`tosumu view`)

### Inspect contract

Machine-readable inspection output is a first-class surface. The JSON envelope is consumed by the TUI, the WPF harness, and future tools.

### UI shells

Today there are two main inspection-facing shells:

- the cross-platform CLI and TUI in `tosumu-cli`
- the repository's Windows WPF harness under `dotnet/`

The Rust-side inspect contract is the source of truth. UI shells should consume it, not invent their own file semantics.

## Layer direction

The design intends a one-way dependency flow:

1. CLI and tooling surfaces
2. query or higher-level interpretation layers
3. B+ tree and logical storage operations
4. transaction and WAL machinery
5. pager and page cache
6. crypto boundary
7. file I/O and page bytes

The important rule is not the diagram. The important rule is that lower layers do not call upward and higher layers do not bypass the boundary below them.

## Trust boundary

The pager is the main trust boundary.

- in memory: trusted plaintext pages
- on disk: adversarial bytes

Pages are authenticated and decrypted when they enter memory, and encrypted when they leave it. Higher layers do not manipulate ciphertext directly.

## Concurrency direction

Current direction:

- single process
- single writer
- explicit locking and busy behavior

Later direction on the roadmap:

- multiple readers by snapshot/MVCC work
- witness and observer deployment work in clustered environments

What is not planned as a core identity change: a general-purpose multi-process network database.