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

## Future service boundary and host modes

Tosumu is still embedded-first, but the planned deployment story is better described as a stable authority boundary with different hosts around it.

```txt
Application / CLI / UI
		  ↓
	  Host adapter
		  ↓
	tosumu-service
		  ↓
	  tosumu-core
		  ↓
	 Database file
```

The important constraint is that the host changes, but storage semantics do not. `tosumu-core` remains the canonical engine. A future `tosumu-service` layer would own open or close lifecycle, unlock state, write serialization, inspect shaping, and boundary-level error mapping. Hosts wrap that service boundary; they do not reimplement it.

### Planned host modes

- Embedded host: the caller and service live in the same process. This stays the default deployment model.
- Local daemon host: a future `tosumu-daemon` exposes the same service contract over local IPC such as named pipes, Unix sockets, or stdio.
- Remote or admin host: a future `tosumu-server` or `tosumu-admin` exposes the same contract over HTTP or another operator-facing surface.

This is not a shift to a different database personality. It is one authority model with multiple deployment shells.

### Platform guidance

| Platform | Embedded host | Local daemon host | Remote or admin host |
| --- | --- | --- | --- |
| Windows | Primary | Strong option | Supported |
| Linux | Primary | Strong option | Primary server target |
| macOS | Primary | Good option | Supported |
| iOS | Primary | Generally unavailable | Client only |
| Android | Primary | App-scoped only | Client only |

Platform notes:

- Windows is a natural fit for either in-process desktop usage or a long-lived local authority using named pipes and, when needed, a Windows service.
- Linux is the broadest target because it fits embedded tools, local daemons managed by `systemd`, and server or container deployment.
- macOS is best treated as a strong local desktop and development platform, with `launchd` as the natural daemon manager when a background authority is needed.
- iOS should be treated as embedded-only in practice; sandbox and lifecycle constraints make a general daemon host the wrong model.
- Android allows app-local service patterns, but it should still be treated as embedded-first rather than as a machine-wide daemon host.

For the deeper design sketch, including service-layer responsibilities and multi-database contexts, see the main design document.