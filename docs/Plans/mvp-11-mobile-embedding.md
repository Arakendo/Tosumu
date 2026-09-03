# MVP+11 Mobile Embedding

| Field | Value |
| --- | --- |
| Status | Proposed; Slice 0 complete and experimental ABI schema retained; implementation not admitted |
| Opened | 2026-09-03 |
| Owner | Mobile adapters above `tosumu-core` |
| Target | Callback-free C ABI, independent Swift/Kotlin callers, and named iOS/Android qualification profiles |
| Depends on | MVP+10 closure, ADR-0001/0002/0003/0004/0010, AR-0010/0016/0017, structured core errors, shared KV boundary |

## Objective

Deliver a memory-safe, versioned foreign embedding boundary and independently
exercised mobile wrappers without exposing Rust storage internals, weakening
protector policy, or turning cross-compilation into a mobile-support claim.

## Explicit Non-Goals

- No SQL, CLI, pager, WAL, B+ tree, page, or raw crypto ABI.
- No callbacks, reentrant application code, or asynchronous ABI in the first
  contract.
- No silent protector fallback or universal raw-key-export requirement.
- No stable Swift/Kotlin API before the C ownership contract survives an
  independent caller and hostile corpus.
- No “iOS supported,” “Android supported,” “hardware-backed,” or compliance
  label based on build success alone.
- No UniFFI, JNI helper, Apple framework, or Android crate before AR-0010 review.

## Slice 0: Contract And Target Baseline

- [x] Open AR-0017 and record governing ownership/failure invariants.
- [x] Inventory existing KV, snapshot, diagnostic, inspection, and error
      operations against foreign-call needs.
- [x] Record the closure-borrowed multi-mutation gap rather than pretending it
      is an FFI transaction handle.
- [x] Record handle subjects, result/buffer ownership questions, hostile inputs,
      and a callback-free initial policy.
- [x] Separate protector capabilities and mobile qualification evidence levels.
- [x] Reconcile normative SDD section 19 using this evidence without replacing
      unresolved questions with implementation promises.
- [x] Define the experimental callback-free request/result algebra and handle
      state machine without reserving ABI layouts, symbols, or compatibility.

**Exit:** the first experiment has a bounded question set and no stable ABI is
claimed.

## Slice 1: Private Callback-Free C Experiment

- [ ] Add a dedicated adapter crate; keep all necessary `unsafe` localized and
      documented there while `tosumu-core` remains `forbid(unsafe_code)`.
- [ ] Define provisional fixed-width ABI scalars, explicit byte slices, owned
      result/error objects, ABI version query, and kind-specific opaque handles.
- [ ] Expose create/open/close, put/delete/get, snapshot begin/get/range/close,
      and bounded connection observations only.
- [ ] Contain panics at every exported entry point and prove no unwind crosses C.
- [ ] Use no callbacks. Do not expose multi-call writes until Slice 3.
- [ ] Build an independently compiled C harness rather than a Rust test calling
      `extern "C"` functions directly.

**Exit:** the C harness can exercise the minimal lifecycle and retrieve full
structured errors, but all symbols remain explicitly experimental.

## Slice 2: Hostile ABI Corpus And Ownership Closure

- [ ] Exercise zero/random/forged/wrong-kind/stale/double-closed handles.
- [ ] Exercise null/length mismatch, length overflow, zero-length versus absent,
      oversized input, allocation failure, and output aliasing policy.
- [ ] Exercise database/snapshot close ordering and concurrent close/use.
- [ ] Verify chosen thread rules for every handle kind.
- [ ] Inject panics before and after internal state acquisition and verify
      containment, cleanup, and handle usability/poisoning.
- [ ] Run leak, address, undefined-behavior, and symbol/export checks on each
      admitted desktop harness target where tooling supports them.

**Exit:** ownership and invalid-use behavior are executable, not comments.

## Slice 3: Atomic Multi-Mutation Admission

- [ ] Compare an adapter-owned copied mutation batch with a core-owned
      transaction capability.
- [ ] Specify staged-read, duplicate-key, conditional-write, abort, commit,
      poison, memory-bound, and generation-result semantics.
- [ ] Require an independent Rust caller if core gains a new owned transaction
      contract; require a C caller for either option.
- [ ] Accept the narrower command-batch name if it cannot provide interactive
      transaction semantics.
- [ ] Update or add an ADR before changing the core public contract.

**Exit:** foreign callers can perform named atomic behavior without retaining a
borrow across calls or using callbacks.

## Slice 4: Mobile Target Build Admission

- [ ] Select minimum iOS/Android OS versions and exact Rust/SDK/NDK targets.
- [ ] Generate target-specific dependency and build-script closures under
      AR-0010 before adding packaging dependencies.
- [ ] Compile minimal C ABI artifacts for device and simulator/emulator targets.
- [ ] Verify exported symbols, architecture slices, loader behavior, panic mode,
      and reproducible build inputs.
- [ ] Record unsupported targets explicitly, including the disposition of
      32-bit Android.

**Exit:** named artifacts compile and load; no device-runtime claim yet.

## Slice 5: Independent Language Consumers

- [ ] Build a Swift package and independent Swift caller using only the C
      contract and generated/retained header.
- [ ] Build an Android library and independent Kotlin caller through a narrow
      JNI adapter using only the C contract.
- [ ] Keep language wrappers thin: lifetime ergonomics and platform policy,
      not duplicated storage semantics.
- [ ] Exercise binary values, absence, typed errors, snapshots, close ordering,
      and process restart from both consumers.
- [ ] Stabilize neither wrapper until caller feedback is reconciled in AR-0017.

**Exit:** two language ecosystems pressure the same ABI without reaching into
Rust internals.

## Slice 6: One Real Platform Protector Prototype

- [ ] Choose Keychain or Android Keystore based on available device evidence,
      not API familiarity.
- [ ] Record exportability, operation shape, authorization/session behavior,
      device binding, backup/migration behavior, invalidation, cancellation,
      rate limit, and error provenance independently.
- [ ] Reopen AR-0016 Gate C3 with the real opaque-handle and failure contract.
- [ ] Keep provider-specific objects and failures out of durable format meaning
      and normalize outcomes without erasing actionable distinctions.
- [ ] Prove unavailable/denied/revoked state does not silently fall back.

**Exit:** C3 is accepted, revised, or parked from real provider evidence; the
prototype alone creates no general provider SPI or hardware claim.

## Slice 7: Lifecycle And Storage Qualification

- [ ] Define named simulator/emulator and physical-device profiles.
- [ ] Exercise clean close, suspend/resume, forced process death, restart/WAL
      recovery, low-storage, memory pressure, file protection/lock state, and
      application upgrade.
- [ ] Verify database, WAL, writer-lock, backup, and export artifact handling for
      each platform container and backup policy.
- [ ] Separate filesystem durability observation from logical recovery success.
- [ ] Measure Argon2 latency, memory, thermal/battery pressure, and cancellation
      needs before changing KDF policy or format parameters.

**Exit:** every support statement names the profile and failure modes actually
exercised.

## Slice 8: Packaging And Acceptance Review

- [ ] Pin packaging/build toolchains and retain artifact provenance.
- [ ] Define ABI compatibility, symbol version, wrapper version, upgrade, and
      deprecation policy.
- [ ] Complete independent C, Swift, and Kotlin consumer suites plus device
      evidence for the claimed profiles.
- [ ] Reconcile AR-0017 and create/revise ADRs for any stable ABI, owned
      transaction, or protector boundary.
- [ ] Update the SDD, security model, error specification, public roadmap, and
      distribution documentation to exactly match the evidence.

**Exit:** MVP+11 is complete only for explicitly named platform profiles; every
other target remains unsupported or unqualified.

## Required Evidence Ladder

```text
Rust contract inventory
    -> independently compiled C harness
    -> hostile ownership and panic corpus
    -> target artifact compiles and loads
    -> Swift/Kotlin simulator or emulator caller
    -> physical-device lifecycle evidence
    -> separately evidenced hardware-protector capabilities
    -> named supported mobile profile
```

No rung implies the next.

## Risks

| Risk | Failure | Control |
| --- | --- | --- |
| Raw pointers become identity | stale/use-after-free behavior is undefined | kind- and generation-checked opaque-handle experiment |
| Error flattening | callers retry corruption or hide committed outcomes | preserve structured code/status/details |
| Callback creep | reentrancy and shutdown become unbounded | callback-free first ABI; separate admission |
| Fake transaction handle | borrowed Rust state crosses calls | explicit Slice 3 architecture gate |
| Protector boolean | distinct platform properties become one misleading claim | decomposed capability observations |
| Cross-build marketing | unrun artifacts are called supported | evidence ladder and named profiles |
| Wrapper semantic drift | Swift and Kotlin promise different databases | one C contract and shared conformance corpus |

## Immediate Next Slice

Implement the admitted private Linux C experiment with no new third-party
dependency. Keep unsafe operations within the reviewed input-slice and copy-out
helpers, then obtain an independently compiled C run before crediting Slice 1.

## References

- `docs/Architectural Reviews/AR-0017-mobile-embedding-abi-and-hardware-protector-boundary.md`
- `docs/Architectural Reviews/AR-0016-cryptographic-provider-seam-and-suite-identity.md`
- `docs/Notes/mvp-11-foreign-contract-and-target-inventory-v1.md`
- `docs/Notes/mvp-11-experimental-abi-contract-v1.md`
- `docs/Notes/mvp-11-c-harness-admission-v1.md`
- `docs/Specifications/Tosumu Error Design Document.md`
- `docs/Specifications/Tosumu Inspect API Specification.md`
- `docs/Specifications/Tosumu Software Design Document.md`
