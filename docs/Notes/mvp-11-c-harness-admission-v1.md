# MVP+11 C Harness And Unsafe-Boundary Admission v1

| Field | Value |
| --- | --- |
| Status | Implementation admitted for a private Linux experiment; evidence not yet obtained |
| Reviewed | 2026-09-03 |
| Owner | AR-0017 / MVP+11 Slice 1 |
| Depends on | Experimental ABI contract v1, ADR-0003, AR-0010, `SharedKvStore`, `ErrorReport` |

## Admission Question

Can Tosumu implement and independently call the smallest useful C boundary
without changing core behavior, importing a binding dependency, or permitting
unsafe code to spread beyond pointer validation and copy-out?

## Crate And Dependency Boundary

Admit one workspace crate named `tosumu-ffi-experimental` with these properties:

- it depends only on `tosumu-core` through a path dependency;
- it builds `rlib` for Rust-side contract tests and `cdylib` for the independent
  C caller; `staticlib` and mobile packaging are deferred;
- it is private and unpublished, and every symbol contains `experimental_v1`;
- it owns foreign representation, registry, panic containment, and projection
  of `ErrorReport`; and
- `tosumu-core` retains `#![forbid(unsafe_code)]` and gains no C types, exported
  symbols, registry state, or mobile policy.

No new Cargo package, build script, procedural macro, binding generator, or
native library is admitted. The C compiler and linker are evidence-producing
toolchain inputs, not release dependencies. Their identities must be recorded
when the harness runs.

## First Evidence Profile

The first profile is intentionally only a hosted Linux desktop experiment:

1. build the Rust `cdylib` with the repository's selected Rust toolchain;
2. compile a C11 harness with the runner's C compiler against a hand-maintained
   experimental header;
3. link/load the produced shared library and run the harness as a separate
   process;
4. inspect dynamic exports against the explicit experimental-symbol allowlist;
5. record Rust, Cargo, C compiler, linker, host, and artifact identity; and
6. report configuration separately from an observed passing run.

The current Windows workstation exposes no `cc`, `clang`, `gcc`, or `cl` on
`PATH`, so no local independent-C result is claimed by this review. Windows,
macOS, iOS, and Android are separate future profiles. Linux success will prove
neither mobile qualification nor cross-platform ABI stability.

## Provisional Representation

The hand-maintained header defines only C fixed-width integers, `size_t`, and
raw byte pointers. It does not expose Rust enums, structs, strings, allocators,
paths, or object addresses.

- `uint64_t` is the opaque handle carrier; zero is invalid.
- a small `repr(C)` outcome returned by value contains a numeric outcome tag, a
  numeric boundary status, and one `uint64_t` payload.
- success payloads are handles or scalar observations as documented per call;
  error payloads are owned error handles; absent has no payload.
- input buffers are `(const uint8_t *, size_t)` pairs borrowed for one call.
- output bytes are owned handles copied through a bounded copy operation and
  released only by the matching close function.
- discriminants and symbol spellings are retained beside the header and tested
  for exact agreement, but remain explicitly experimental.

The initial harness exercises ABI version, unencrypted create/open, close,
single-key put/delete/get, snapshot create/generation/get/close, byte
length/copy/close, and full error code/status/message/detail projection. Range
encoding and connection observations may follow in the same slice only after
their bounded C representation is reviewed; their omission is explicit rather
than filled with JSON or callbacks.

## Unsafe-Code Budget

The adapter crate uses `#![deny(unsafe_code)]`. One private `raw` module receives
a narrow `#[allow(unsafe_code)]` exception and `#![deny(unsafe_op_in_unsafe_fn)]`.
Unsafe operations are limited to:

1. constructing a borrowed input slice after checking pointer/length rules; and
2. copying an already-owned immutable byte result into a validated caller
   destination with sufficient capacity.

Export attributes and C entry points live with this reviewed raw boundary when
the selected Rust version's lint model requires it. Registry lookup, handle
state, UTF-8 validation, storage calls, result construction, error projection,
and panic policy remain safe Rust in sibling modules. No unsafe block performs
I/O, allocation, locking, handle dispatch, UTF-8 conversion, or Tosumu core
operations.

Each unsafe helper states its preconditions directly above the block. Null with
zero length is handled without constructing a Rust slice from null. Pointer
addition occurs only after capacity checks. Source storage is never exposed, so
a valid caller destination cannot alias it. An arbitrary dangling or forged
non-null pointer cannot be validated portably and remains caller memory
unsafety, not a promised boundary error.

## Registry And Thread Rules

The registry issues non-zero opaque identifiers whose numeric structure is not
public. Entries record kind and a non-reusable process generation; lookup must
match both. It returns a temporary strong owner before releasing the global
registry lock, and no global registry lock remains held during storage I/O,
Argon2 work, or caller-buffer access.

Database and snapshot reads/mutations are initially restricted to their
creating thread. All close functions are thread-independent so a foreign
runtime can release resources from a finalizer thread. Byte and error reads may
be thread-independent only after their close/read race uses the same temporary-
owner rule.

A close/use race linearizes at lookup: an operation that already owns a strong
reference may finish, while later lookup receives stale-handle failure. Closing
the database handle does not close snapshots, byte results, or errors derived
from it.

## Panic And Build Policy

Every export delegates through one panic-containment wrapper. It constructs the
complete return value before the entry point returns, never unwinds into C, and
returns a fixed boundary failure if an owned error cannot be produced. A panic
during an operation associated with a database marks its adapter object
poisoned before later operations are admitted; close remains available.

Recoverable registry/capacity exhaustion uses checked allocation where the
adapter controls it. Allocator-level out-of-memory may abort under Rust's
runtime and is not described as a contained panic or typed failure.

The experimental library must be built with an unwinding panic strategy.
`panic = "abort"` makes containment impossible and is therefore rejected for
this profile rather than described as degraded support. A later consumer build
system must preserve or explicitly renegotiate this constraint.

## Conservation And Hostile Corpus

Before the experiment can graduate, retain tests showing:

- direct `SharedKvStore` and C-harness operations produce the same committed
  values, absence, snapshot generations, and structured core errors;
- create/open/put/delete/get/snapshot behavior does not change in
  `tosumu-core`;
- wrong-kind, stale, random, zero, and double-closed handles fail without core
  dispatch;
- null/non-zero input, invalid UTF-8, embedded-NUL paths, size overflow, and
  insufficient output capacity write no partial output;
- database close leaves an existing snapshot and owned error/byte result live;
- panic injection does not cross C or fabricate success;
- the global registry lock is absent while core work executes; and
- symbol inspection finds no undeclared Tosumu exports.

Existing workspace format, crypto, recovery, concurrency, and documentation
checks remain the broad conservation baseline. A green C harness is evidence
about this adapter profile only.

## Disposition

Admit the bounded private implementation above. Do not admit a stable ABI,
published crate, generated binding, static library, callback, asynchronous
operation, multi-call transaction, platform protector, mobile target, or
cross-platform compatibility claim. Reopen AR-0017 after the first independent
C run and hostile-handle results.
