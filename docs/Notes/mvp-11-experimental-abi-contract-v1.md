# MVP+11 Experimental ABI Contract v1

| Field | Value |
| --- | --- |
| Status | Proposed experiment schema; no exported symbols or compatibility promise |
| Observed | 2026-09-03 |
| Owner | AR-0017 / MVP+11 Slice 1 |
| Depends on | Foreign contract inventory, `SharedKvStore`, `ErrorReport`, ADR-0001, AR-0017 |

## Purpose

Define the behavior that a minimal independently compiled C harness should
pressure before Tosumu chooses concrete exported layouts. This record does not
reserve symbol names, numeric discriminants, struct layout, calling convention,
or ABI version. Those become compatibility commitments only after the hostile
corpus and independent caller justify stabilization.

## Boundary Shape

The experiment has five subjects:

| Subject | Capability | Explicit exclusion |
| --- | --- | --- |
| Database handle | create/open, single put/delete/get, snapshot creation, bounded connection observations, close | No pager/WAL access, protector mutation, callback, or borrowed transaction |
| Snapshot handle | generation, get, inclusive ordered range read, close | No mutation or live/latest read |
| Immutable byte-result handle | length, bounded copy/view, close | No allocator mixing or mutation |
| Structured error handle | code, status, message, typed details, close | No global/thread-local last error and no Rust source object |
| Call result | success/absence/not-applied/error plus exactly the payload allowed by that outcome | No null-as-error convention |

Every non-zero handle is opaque. Callers cannot derive address, kind, slot, or
generation from its bits. Zero is always invalid and is never a live object.

## Provisional Operation Set

```text
abi_version() -> version

database_create(path_bytes, open_options) -> database | error
database_open(path_bytes, open_options) -> database | error
database_close(database) -> success | error
database_put(database, key, value) -> success | error
database_delete(database, key) -> success | error
database_get(database, key) -> bytes | absent | error
database_snapshot(database) -> snapshot | error
database_connection_info(database) -> observation | error

snapshot_generation(snapshot) -> u64 | error
snapshot_get(snapshot, key) -> bytes | absent | error
snapshot_scan(snapshot, inclusive_start, inclusive_end) -> encoded_pairs | error
snapshot_close(snapshot) -> success | error

bytes_length(bytes) -> length | error
bytes_copy(bytes, destination, capacity) -> written_or_required | error
bytes_close(bytes) -> success | error

error_code(error) -> borrowed_ascii_view | boundary_failure
error_status(error) -> status | boundary_failure
error_message(error) -> bytes | boundary_failure
error_detail_count(error) -> count | boundary_failure
error_detail(error, index) -> typed_detail | boundary_failure
error_close(error) -> success | boundary_failure
```

Names are descriptive pseudocode. There is no raw-pointer-returning `get`, no
caller use of Rust layouts, and no API callback. Range results use one bounded,
versioned encoding or iterator-like pull handle selected during implementation;
the experiment must not return an unbounded graph of allocations.

Create/open options initially admit only the protector modes already exposed by
the selected provider-neutral owner. A future platform protector is not encoded
as a passphrase variant and cannot trigger passphrase fallback.

## Call Outcome Algebra

Each call produces exactly one top-level outcome:

```text
success(payload?)
absent
not_applied(version?)       # reserved for later conditional operations
error(error_handle)
boundary_failure(code)      # only when no error object can safely be allocated
```

`absent` is valid only for lookup-like calls. `not_applied` is not an error and
must not be collapsed into `success`. An error outcome owns exactly one error
handle. Payload outputs remain zeroed/invalid on every non-success outcome.

Boundary failures are a tiny FFI-owned vocabulary for conditions such as
unsupported ABI version, invalid output pointer, invalid/stale/wrong-kind
handle, wrong thread, allocation failure while constructing an error, or a
contained panic. These are not silently added to the core public error-code
registry. Stabilizing them requires AR-0017 disposition and an Error Design
Document update.

## Buffer Contract

- Input `(pointer, length)` is borrowed for the call and copied only where the
  operation explicitly requires retention.
- Null with non-zero length is invalid. Null with zero length represents an
  empty slice, never absence.
- Lengths are checked before pointer construction or arithmetic.
- Success outputs are immutable ABI-owned byte handles. Callers release them
  only through the matching ABI close operation.
- The initial experiment prefers bounded copy-out: query/copy reports the exact
  required length when capacity is insufficient and writes nothing partial.
- Any temporary direct view, if tested, remains valid only while its byte handle
  is live and no close races it; it is not the initial portability contract.
- Secret inputs and error material are never returned through ordinary value
  buffers. Zeroization remains unclaimed unless separately implemented and
  evidenced.

## Structured Error Projection

Core failures are first converted to `ErrorReport`. The ABI copies its stable
code, coarse status, message, and ordered typed details into an independently
owned error object. Detail values preserve `bool`, UTF-8 string, `u16`, and
`u64` distinctions. Unknown future detail types fail as unsupported in the
experimental ABI rather than being formatted into strings.

The error object contains no Rust backtrace, dynamic source error, OS object,
secret, provider handle, or borrowed pointer into a database. It remains
readable after the originating database is closed.

## Provisional Handle State Machines

### Database

```text
           create/open success
Invalid --------------------------> Active
                                      |  \
                  integrity/panic     |   \ close wins registry removal
                                      v    v
                                  Poisoned  Closed/Stale
                                      |
                                      +------ close ------> Closed/Stale
```

- Failed create/open produces no handle.
- `Active` permits the bounded operation set.
- A contained panic or core poison outcome marks the handle `Poisoned`; only
  error observation and close remain valid.
- Close atomically removes the live generation before destruction. Later use
  and double close receive invalid/stale-handle boundary failure.
- The experiment begins thread-affine. Cross-thread use returns `wrong_thread`
  until independent mobile caller evidence justifies a wider rule.

### Snapshot

```text
Invalid -- snapshot success --> Active -- close --> Closed/Stale
                                  |
                                  +-- parent database close: remains Active
```

A snapshot owns its generation pin and may outlive the database handle that
created it, matching the underlying owned Rust snapshot behavior. It is
read-only, thread-affine in the initial experiment, and never silently changes
to a latest read. Snapshot-limit failure produces no handle.

### Bytes and error objects

```text
Invalid -- successful allocation --> Active -- close --> Closed/Stale
```

They are immutable and independent of the originating database lifetime. The
experiment may permit close/read from any thread only if the registry proves
concurrent close/read cannot free storage while borrowed; otherwise it applies
the same conservative thread-affinity rule.

## Registry And Concurrency Hypothesis

The experiment should use kind- and generation-checked integer handles backed
by an adapter-owned registry. Lookup obtains a temporary strong owner before
releasing the registry lock, so concurrent close cannot free an object during a
call. Closing removes the entry first; a generation prevents slot reuse from
reviving stale handles.

This is a hypothesis to test, not an accepted public representation. The
registry must be bounded, must return explicit exhaustion, and must not hold its
global lock while performing storage I/O or Argon2 work. Fork/process cloning,
dynamic unload, and calls from signal handlers are unsupported.

## Panic And Reentrancy Policy

Every exported entry point is wrapped at the adapter boundary. A panic becomes
a boundary failure or owned structured error, publishes no fabricated success
payload, and marks an affected database handle poisoned when its state may be
uncertain. Panic text is not a stable error and is not exposed by default.

The ABI invokes no application function pointer. Progress, cancellation, and
asynchronous platform authorization are absent from v1. If later evidence
requires them, AR-0017 must define thread, reentrancy, lifetime, shutdown, and
late-completion behavior first.

## Multi-Mutation Exclusion

No `transaction_begin` operation exists in this schema. The current Rust write
transaction is a closure-scoped borrow and cannot remain live across calls.
MVP+11 Slice 3 must admit either a named copied mutation batch or an owned core
transaction contract before adding atomic multi-call behavior.

## Required Falsification Before Implementation Graduation

- stale generation never resolves after registry slot reuse;
- wrong-kind handles cannot dispatch to another object's operation;
- double close and close/use races never double free;
- null/length and capacity failures write no partial output;
- panic injection never unwinds into C or returns success;
- database close does not invalidate a live snapshot;
- error and byte objects remain valid after database close;
- thread-affinity rejection is deterministic; and
- allocation/registry exhaustion is explicit and leak-free.

## Disposition

Suitable as the contract for a private C experiment. It does not yet justify
exported stable symbols, headers distributed to consumers, a public ABI version,
unsafe-code exceptions beyond a narrowly reviewed adapter, or mobile support.
