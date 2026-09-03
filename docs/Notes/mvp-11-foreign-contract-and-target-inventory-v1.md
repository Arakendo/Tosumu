# MVP+11 Foreign Contract And Target Inventory v1

| Field | Value |
| --- | --- |
| Status | Baseline observation; not a stable ABI or platform-support claim |
| Observed | 2026-09-03 |
| Owner | AR-0017 / MVP+11 planning |
| Sources | `SharedKvStore`, core error reports, inspection boundary, Cargo target configuration, ADR-0001 through ADR-0004, ADR-0010 |

## Purpose

Identify the smallest existing storage contract a foreign caller can exercise,
the semantics that do not cross C safely yet, and the target evidence required
before Tosumu calls an iOS or Android artifact supported.

## Existing Logical Operations

| Subject | Existing Rust operation | Candidate foreign meaning | Initial disposition |
| --- | --- | --- | --- |
| Writable database | create/open, including passphrase variants | Acquire one database owner or a typed failure | Include in experiment; protector-neutral construction remains future work |
| Database lifetime | Rust ownership/drop | Explicit close; invalidate exactly once | Include; stale and double-close behavior must be structural |
| Single mutation | `put`, `delete` | One committed generation or typed failure | Include |
| Latest read | `get` | Found bytes, absent, or typed failure | Include; absence must not be encoded as generic null/error |
| Conditional mutation | `put_if_absent`, compare-and-set, versioned put | Applied/not-applied plus owner-scoped version | Defer until token identity can be represented without serializing it |
| Snapshot | `snapshot`, then `get`/range scan | Generation-pinned read handle | Include after handle-parent and close ordering are explicit |
| Diagnostics | `connection_info` | Bounded scalar observations | Include after ABI struct/version encoding is selected |
| Inspection | provider-neutral inspection response | Bounded structured observation | Separate optional surface; do not expose pager or physical Rust types |
| Multi-mutation write | closure-borrowed `write`/`try_write` | Atomic staged mutation sequence | Not directly representable as a callback-free multi-call C handle |
| Backup/export/VACUUM | path-level offline operations | Managed artifact operation and structured report | Defer from first ABI; sidecar and exclusivity policy must be explicit |

The first experiment must not expose raw `BTree`, `Pager`, `WalWriter`, page,
keyslot, or Rust enum layouts.

## Multi-Mutation Gap

`SharedKvStore::write` lends a non-`Send`, non-`Sync` transaction to one Rust
closure. Holding that borrow across foreign calls is neither supported nor a
credible ABI contract. Two candidates require later evidence:

1. an adapter-owned command batch that stores copied put/delete intents and
   applies them in one existing Rust closure at commit time; or
2. a new core-owned transaction capability with explicit lifetime and staged
   read semantics.

The command batch is smaller but cannot claim arbitrary interactive transaction
semantics. The core capability is stronger but changes a storage contract and
requires independent Rust and C callers. The first C harness will use only
single-operation commits and snapshots while measuring this pressure.

## Result And Error Envelope

Every fallible call needs a result status separate from payload presence. The
minimum lossless information already exists in `ErrorReport`:

- stable string code;
- coarse status;
- human-readable message; and
- zero or more typed details (`bool`, string, `u16`, or `u64`).

The ABI must not return only `-1`, use null for both absence and failure, expose
OS errno as Tosumu identity, or require a caller-sized error buffer before its
length is known. Error data must remain valid under a documented owner until
explicit release or the next operation on the same error object—not through an
unscoped thread-local pointer.

Successful byte results need pointer, length, allocation identity, and exactly
one matching release function. Zero-length and absent values remain distinct.
Inputs are borrowed for the call only and are never retained unless an API
explicitly says it copies them.

## Handle Subjects And Provisional Rules

Distinct handle kinds are required for database owners, snapshots, returned
buffers, errors, and any later write batch. A generic pointer with caller casts
would erase the very state the boundary must validate.

The first experiment should pressure a generation-checked handle registry
rather than exporting Rust allocation addresses. This is a hypothesis, not an
accepted ABI. The hostile corpus must cover:

- zero, random, forged, wrong-kind, stale, and already-closed handles;
- parent database close while snapshots exist and both close orders;
- concurrent close/use and calls from the wrong thread under the chosen rule;
- null pointer with non-zero length, overflowed lengths, and aliased outputs;
- allocation failure and oversized input; and
- panic injection inside the boundary, proving unwind containment and a typed
  unusable/failed outcome.

The initial ABI has no application callbacks. Cancellation and progress, if
needed, use explicit operation/state polling until concrete platform evidence
justifies callback reentrancy.

## Threading And Lifecycle Questions

`SharedKvStore` is cloneable and internally coordinated; `KvReadTransaction` is
`Send` but deliberately not `Sync`; `KvWriteTransaction` is neither. A foreign
ABI must not simplify these into “all handles thread-safe.” The experiment must
choose and test one conservative rule per handle kind. Thread affinity is an
acceptable initial restriction if reported explicitly.

Process death runs no destructors. Committed state follows the existing WAL
contract; uncommitted adapter state must remain memory-only and unpublished.
Suspend/resume, OS file coordination, background-task expiration, and memory
pressure are platform lifecycle events, not synonyms for `close`.

## Protector Capability Vocabulary

No single hardware-backed boolean is sufficient. A future protector observation
must represent independent, possibly unknown properties:

- key material exportability;
- cryptographic operation versus wrap/unwrap capability;
- hardware or isolated-execution backing and its attestation source;
- device binding and migration/backup eligibility;
- user-presence and biometric authorization requirements;
- unlocked-device/session requirements;
- enrollment-change invalidation;
- rate limiting, lockout, cancellation, expiration, and revocation;
- synchronous versus asynchronous authorization; and
- evidence provenance and observation time.

Unsupported and unknown are distinct. Provider failure never triggers an
implicit passphrase fallback.

## Target Evidence Baseline

| Target/profile | Current evidence | Missing before qualification |
| --- | --- | --- |
| Windows x86_64 desktop | Workspace builds/tests; writable paths use native writer admission; VACUUM refuses before mutation | C ABI harness and sanitizer-equivalent hostile evidence |
| Linux x86_64 desktop | Cargo target closure retained; hosted stable CI in progress for current VACUUM checkpoint | C harness, ABI artifact, symbol and loader evidence |
| macOS arm64 desktop | Hosted stable workspace CI passed at commit `abdc241` | C harness; this is not iOS evidence |
| iOS device ARM64 | No build or device evidence | Dependency closure, target build, C/Swift caller, lifecycle/filesystem/device tests |
| iOS simulator ARM64/x86_64 | No evidence | Target build and simulator integration |
| Android ARM64 | No build or device evidence | NDK/toolchain closure, target build, C/Kotlin caller, lifecycle/filesystem/device tests |
| Android x86_64 emulator | No evidence | Target build and emulator integration |
| 32-bit Android ARM | Named by old SDD only | Requirements and dependency feasibility; otherwise explicitly unsupported |

Desktop host success does not qualify a mobile target. Simulator/emulator
success does not establish physical-device storage or hardware-protector
behavior.

## Baseline Conclusion

The existing shared KV and structured-error contracts are sufficient to begin a
small callback-free C experiment. They are not sufficient to stabilize an ABI:
multi-call atomic writes, handle identity, allocation transfer, thread rules,
panic containment, ABI negotiation, and platform protectors remain unresolved.
No mobile target is currently qualified.
