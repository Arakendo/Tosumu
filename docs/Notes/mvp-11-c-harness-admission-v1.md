# MVP+11 C Harness And Unsafe-Boundary Admission v1

| Field | Value |
| --- | --- |
| Status | Private Linux experiment implemented and independently exercised; ABI remains unstable |
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

## Observed Evidence

GitHub Actions run `33817119731`, job `100851610793`, exercised commit
`63820bf6347370d862b67ed0bbb9b834ebcb4153` on 2026-09-03 and completed
successfully in 35 seconds. The retained job log identifies:

- GitHub-hosted Ubuntu 24.04.4, `ubuntu-24.04` image
  `20260831.293.1`;
- `rustc 1.98.1 (48a229cea 2026-09-01)`, host
  `x86_64-unknown-linux-gnu`, LLVM 22.1.8;
- `cargo 1.98.1 (797e8a9bc 2026-08-05)`;
- GCC/`cc` 13.3.0; and
- GNU ld 2.42.

The job built the Rust `cdylib`, compiled and dynamically linked the C11
harness as a separate executable, matched every `tosumu_experimental_v1_*`
dynamic export against the retained allowlist, and printed `independent C ABI
harness: ok`. The caller exercised ABI/layout constants, binary keys,
create/put/get lifecycle, snapshot stability across a later write
and database-handle close, byte copy/ownership, stale/double close, null input,
wrong-kind rejection, and owned core error code/status/message/string-detail
projection.

This is one observed Linux profile. It does not establish MSRV compatibility,
another native platform, mobile behavior, ABI stability, range/connection
representation, panic injection, leak freedom, or arbitrary-pointer safety.

GitHub Actions run `33817692660`, job `100853354051`, then exercised commit
`c17edfe7a8241f0e11df6b58bc9bb9dd6ecc96e5` successfully in 21 seconds on the
same named hosted Linux profile. The expanded independent caller retrieved a
bounded connection observation and invoked the feature-gated panic test symbol.
The panic was contained before control returned to C, produced the declared
boundary-panic outcome, poisoned later database operations, and left close
available. The test-only symbol is present only when `ffi-test-hooks` is
selected and remains in the test-profile export allowlist.

This second result establishes the tested common containment wrapper and one
database-associated panic transition. It does not prove that allocator aborts,
foreign invalid-pointer faults, platform exceptions, or every possible panic
site are recoverable.

GitHub Actions run `33820788393`, job `100862796680`, next exercised commit
`2f7969af3cf6a2adbcec6cf24c2f4739b4f2ce4b`. The expanded independent C caller
consumed thirteen captured rows through four bounded pages after database-handle
close, checked every pair and first-unconsumed inclusive continuation, handled
and retried a blocked 20,000-byte overflow entry, and verified that derived byte
handles outlive page close. The exact experimental symbol allowlist and the C
compiler's `-Wall -Wextra -Werror` checks passed. This closes the C evidence gate
for AR-0018's provider-neutral pagination contract; it does not stabilize the C
representation.

## Bounded Range Resolution

The original `KvReadTransaction::scan` materializes its complete selected range,
so adapter-side truncation was rejected as a false resource bound. AR-0018 now
admits `KvReadTransaction::scan_page`, which applies pair and logical-payload
limits while traversing the captured generation and returns the first unconsumed
key as an inclusive continuation.

The operation validates an overflow entry's declared logical length before
deciding whether it fits, but does not read or allocate excluded overflow pages.
One continuation-key allocation lies outside the logical payload budget and is
bounded separately by `MAX_KEY_SIZE`. The experimental C adapter converts its
fixed-width pair limit to Rust `usize`, owns the returned page behind an opaque
handle, and returns pair/continuation bytes through independently owned byte
handles. It exposes no page number, slot, WAL offset, or mutable cursor.

The independent Rust and C callers, leaf-boundary tests, corruption tests,
complete-scan property, and workspace conservation run justified the 2026-09-03
ADR-0006 amendment. The C projection remains private under AR-0017 and supplies
no stable-ABI, mobile, or cross-platform claim.

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

The admitted harness exercises ABI version, unencrypted create/open, close,
single-key put/delete/get, snapshot create/generation/get/close, byte
length/copy/close, full error code/status/message/detail projection, immutable
connection observations, and owned bounded scan pages. Pagination accessors
return pair count, separately owned key/value bytes, optional continuation, and
optional blocked-entry size; absence remains distinct from a zero scalar or an
empty byte string. No operation uses JSON or callbacks.

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

### Initial Slice 2 observations

The first table-driven hostile corpus covers every handle-taking export across
all six registered kinds. Zero and `u64::MAX` are invalid handles; a live handle
of any other kind is wrong-kind; after the matching close, the same identifier
is stale and invalid. The corpus counts only its own six entries, so concurrent
Rust tests cannot falsify the removal check through unrelated registry use.

All borrowed input positions reject null with nonzero length. The raw slice
helper now also rejects lengths above `isize::MAX` before calling
`from_raw_parts`; a non-null pointer does not make an unrepresentable Rust slice
valid. The caller still owns the unavoidable stronger precondition that a
non-null region is genuinely readable for the stated length. Portable Rust or C
code cannot prove that property from an address and count.

Database and snapshot operations remain creating-thread-only. Immutable byte,
error, connection, and scan-page observations are readable on another thread,
and every kind-specific close is finalizer-thread-safe. A 64-case close/read
race over owned byte results produced only the documented linearizations:
successful read after ownership acquisition or invalid-handle after close.

These are local Rust-side boundary observations for commits
`10295dd8210bd63fa91c4a46a7064959603504f0`,
`4969aac7cb2bd98963cd549645a58361eccb363c`, and
`ca8badf3226f7eb902a43b2573339d4587faf4d7`; their pushed hosted workflows were
not yet complete when this note was updated. Registry exhaustion/recovery,
database and snapshot close/use races, allocator-abort scope, independent C
hostile cases, and sanitizer evidence remain open.

## Disposition

Admit the bounded private implementation above. Do not admit a stable ABI,
published crate, generated binding, static library, callback, asynchronous
operation, multi-call transaction, platform protector, mobile target, or
cross-platform compatibility claim. Reopen AR-0017 after the first independent
C run and hostile-handle results.
