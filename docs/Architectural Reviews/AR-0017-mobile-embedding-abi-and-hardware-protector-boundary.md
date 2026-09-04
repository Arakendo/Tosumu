# AR-0017: Mobile Embedding, ABI, And Hardware-Protector Boundary

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-09-03 |
| Last reviewed | 2026-09-03 |
| Scope | Mobile embedding adapter / C ABI / Swift and Kotlin wrappers / hardware-backed protectors |
| Trigger | MVP+11 is next in the delivery sequence, while the normative mobile design contains unverified and outdated implementation assumptions |
| Related ADRs | ADR-0001, ADR-0002, ADR-0003, ADR-0004, ADR-0006, ADR-0007, ADR-0010 |
| Related reviews | AR-0007, AR-0009, AR-0010, AR-0016, AR-0018 |
| Related evidence | SDD section 19, current `SharedKvStore` and inspection boundaries, writer-lock sidecar behavior, format-v3 crypto seam, and C2 independent oracle |

## Architectural Question

What is the smallest stable mobile embedding boundary that can preserve Tosumu's
typed storage, ownership, concurrency, authentication, and failure contracts
across C, Swift, and Kotlin—and how should mobile hardware-backed protection
exercise crypto Gate C3 without making raw key export, fallback, or a public
provider SPI mandatory?

## Context

The SDD's mobile section predates several implemented boundaries. It sketches a
pointer-returning C API, Swift and Kotlin wrappers, and Keychain/Keystore
protectors, but the sketch is not an accepted ABI or implementation plan.
MVP+10 added process-cooperative writer admission, retained WAL generations,
snapshot handles, and offline VACUUM publication. ADR-0010 and AR-0016 now
separate the durable format-v3 construction from runtime mechanism and future
opaque-key ownership.

Several SDD statements therefore require reconciliation before implementation:

- a Tosumu database is operationally a managed artifact set, not universally
  one file: WAL and the persistent writer-lock sidecar have distinct lifecycle,
  copy, backup, and exclusion rules;
- native writer admission has a target-specific dependency, so “no platform-
  specific dependencies” is no longer an evidenced statement;
- successful desktop compilation does not qualify iOS/Android filesystems,
  suspend/resume, process death, memory pressure, packaging, or device behavior;
- Keychain, Secure Enclave, Android Keystore, hardware backing, biometric
  authorization, backup eligibility, and device-only policy are different
  properties and must not be collapsed into one protector claim;
- silently falling back from an unavailable hardware authority to a passphrase
  would change policy and conflicts with fail-closed, typed failure behavior;
- the illustrative C API loses error detail, uses ambiguous null results, and
  does not define buffer provenance, handle state, double-close, thread use,
  cancellation, or panic containment; and
- the five-week estimate and binary-size figures are planning guesses rather
  than retained observations.

This review precedes a dedicated MVP+11 implementation plan. Opening it changes
neither the normative SDD nor any public API.

## Governing Invariants

1. `tosumu-core` continues to own provider-neutral storage meaning; mobile and
   language adapters translate that contract without exposing pager, WAL,
   page, B+ tree, or format internals.
2. No Rust panic or unwinding crosses a foreign ABI boundary.
3. Every foreign allocation and handle has one documented owner, destructor,
   validity state, and concurrency rule.
4. Hardware-protector unavailability, denial, revocation, cancellation, and
   authentication failure remain distinguishable from wrong credentials,
   corrupt storage, and ordinary I/O failure where the platform permits it.
5. Fallback between protector authorities is explicit caller policy; Tosumu
   never silently weakens the requested authority.
6. Runtime provider choice cannot reinterpret existing format-v3 bytes.
7. Platform support is claimed only at the level actually exercised: compile,
   simulator/emulator, device, filesystem/lifecycle, or hardware-backed.

## Evidence

### Established

- ADR-0001 places consumer and adapter semantics above `tosumu-core`.
- ADR-0002 keeps persistent ciphertext and authentication at the pager boundary.
- MVP+10 provides opaque Rust transaction/snapshot owners and one cooperating
  writable authority per database artifact set.
- The error specification and inspection boundary provide typed source
  vocabulary that an ABI can map without exposing Rust layouts.
- ADR-0010 provides private format-v3 and entropy seams. Gate C2 independently
  reproduced format-v3 operations but did not establish an opaque-key contract.

### Missing

- a complete operation and error inventory for a minimal foreign caller;
- an independently compiled C caller exercising ownership and failure paths;
- evidence for stale, forged, null, double-freed, cross-thread, and reentrant
  handles, including process shutdown and callback behavior;
- iOS and Android target builds with audited target-specific dependency closure;
- simulator/emulator and physical-device lifecycle/crash fixtures;
- filesystem durability evidence for named OS/device/storage profiles;
- a real Keychain or Keystore prototype that proves whether keys are exported,
  wrapped, operation-bound, session-bound, or asynchronously authorized; and
- packaging provenance, symbols, minimum OS versions, ABI-version negotiation,
  and upgrade compatibility evidence.

## Ownership And Dependency Analysis

The initial dependency direction under review is:

```text
Swift / Kotlin application semantics
              |
              v
language wrapper and platform lifecycle policy
              |
              v
versioned C ABI adapter with opaque handles and owned buffers
              |
              v
provider-neutral Tosumu KV / transaction / inspection contracts
              |
              v
tosumu-core storage mechanics and authenticated pager

platform Keychain / Keystore adapter
              |
              v
future protector capability boundary (AR-0016 Gate C3)
              |
              v
OS service / hardware authority
```

The C ABI owns representation, validation, panic containment, and allocation
transfer—not storage semantics. Swift/Kotlin wrappers own language lifecycle
and platform policy. A protector adapter owns interaction with an OS authority,
but Tosumu owns protector purpose, normalized outcomes, durable metadata, and
the rule that no implicit fallback occurs.

The FFI handle model and cryptographic key-handle model are related but not the
same abstraction. A database handle being opaque to C does not prove that a
provider can retain a non-exportable key through pager, snapshot, recovery,
rebuild, and close lifetimes.

## Alternatives Considered

### Alternative A: Handwritten versioned C ABI with thin language wrappers

- Benefits: explicit allocation and error contracts; independently testable
  from C; avoids making Swift/Kotlin generators part of core architecture.
- Costs: more manual bindings and compatibility discipline.
- Failure mode: an underspecified handle table recreates Rust ownership bugs at
  the ABI boundary or flattens typed failures into integers without detail.

### Alternative B: UniFFI-generated Swift and Kotlin bindings

- Benefits: less binding boilerplate and stronger generated ownership patterns.
- Costs: admits a build-time/runtime dependency and generator-defined ABI whose
  target closure, compatibility, error mapping, and customization need review.
- Failure mode: generated convenience is mistaken for a stable Tosumu contract
  or prevents the hardware-protector boundary from expressing required state.

### Alternative C: Platform-native wrappers call separate bespoke Rust APIs

- Benefits: each platform can match native lifecycle and authorization models.
- Costs: duplicated semantics and greater drift between iOS and Android.
- Failure mode: two mobile APIs make different durability, error, or fallback
  promises while both appear to represent Tosumu.

### Alternative D: Stabilize the SDD sketch directly

- Benefits: fastest apparent path to a demo.
- Costs: freezes ambiguous memory, error, thread, and protector behavior.
- Failure mode: successful happy-path calls are presented as a safe mobile ABI.

### Alternative E: Continue incubation with bounded probes

- Benefits: lets a C consumer and one platform protector reveal the actual
  contract before public stabilization.
- Costs: delays a polished SDK and may replace early prototypes.
- Failure mode: prototypes escape as de facto APIs without explicit unstable
  labeling and removal criteria.

## Findings

- A dedicated adapter crate is consistent with ADR-0001; adding FFI vocabulary
  to `tosumu-core` is not.
- The current SDD examples are useful requirements prompts, not safe API
  definitions or evidence of mobile support.
- The first implementation pressure should come from a minimal independent C
  caller before Swift/Kotlin packaging stabilizes the ABI.
- Mobile Keychain/Keystore work is credible pressure for AR-0016 Gate C3, but
  provider capabilities must be learned from an actual platform prototype.
- “Hardware-backed,” “non-exportable,” “biometric-gated,” and “device-only”
  must remain separate observations.
- Silent passphrase fallback is not admissible. A caller may explicitly reopen
  with another configured protector under separately defined policy.
- Cross-compilation is necessary evidence but insufficient platform
  qualification.

## Disposition

**Incubating.** Admit an MVP+11 contract inventory, target/dependency inventory,
and bounded private C-ABI experiment. Do not yet admit a stable ABI, UniFFI or
mobile SDK dependency, Swift/Kotlin public API, hardware-protector SPI, opaque
crypto key handle, fallback policy, supported mobile target, or compliance
claim.

## Required Follow-Up

- [x] Reconcile SDD section 19's obsolete claims and distinguish design intent,
      hypotheses, and accepted requirements.
- [x] Create a sliced MVP+11 plan: contract baseline, C harness, hostile-handle
      corpus, target builds, language consumers, platform protector prototype,
      lifecycle fault corpus, packaging, and acceptance review.
- [x] Inventory the minimum provider-neutral KV, transaction, snapshot,
      inspection, and close operations plus their complete typed outcomes.
- [ ] Decide ABI version negotiation, handle identity/state, allocation, thread,
      cancellation, callback, and panic-containment rules before stabilization.
- [ ] Review candidate binding/toolchain dependencies under AR-0010 before
      admission.
- [ ] Use one real Keychain or Keystore prototype to reopen AR-0016 Gate C3;
      keep raw-key export and synchronous operation optional until evidenced.
- [ ] Define named iOS/Android qualification profiles and retain compile,
      simulator/emulator, physical-device, filesystem, and hardware evidence as
      distinct observations.
- [ ] Require at least one independent Swift caller and one independent Kotlin
      caller before their public wrappers stabilize.

## Reopening Triggers

- the operation/error inventory and hostile-handle corpus are complete;
- an independent C caller exposes missing ownership or version semantics;
- a binding generator materially reduces risk and its dependency closure is
  reviewed;
- a real Keychain/Keystore integration reveals key export, session,
  authorization, or asynchronous constraints;
- mobile target compilation reveals unsupported dependencies; or
- simulator/emulator/device evidence contradicts desktop durability or
  lifecycle assumptions.

## Review History

### Cycle 1 -- 2026-09-03

- Status entering review: Proposed.
- New evidence: MVP+10 sidecar/snapshot behavior, ADR-0010's private crypto
  seam, C2 independent oracle results, and a line-by-line reconciliation of SDD
  section 19 against current architecture.
- Findings: the adapter layer is architecturally plausible, but the existing C
  sketch and platform/protector statements lack the ownership, failure, target,
  and device evidence required for stabilization.
- Disposition: Incubating; admit inventories and bounded experiments only.
- Resulting ADR or documentation change: AR-0017 opened; no ADR or public API.

### Cycle 2 -- 2026-09-03

- Status entering review: Incubating; baseline inventory and plan open.
- New evidence: the foreign-contract inventory maps the current shared KV,
  snapshot, diagnostic, inspection, and structured-error surfaces to candidate
  C meanings; records provisional handle/result rules and target evidence; and
  identifies that closure-borrowed multi-mutation writes cannot cross a
  callback-free multi-call ABI.
- Findings: create/open/close, atomic single mutations, latest reads, snapshots,
  and bounded observations are sufficient for a private C experiment. Atomic
  multi-call writes require a later choice between a deliberately narrower
  adapter command batch and a new core-owned transaction capability. Neither is
  admitted by this cycle.
- Disposition: remain Incubating. Proceed next with SDD reconciliation and an
  experimental request/result schema plus handle state machine; do not export
  functions yet.
- Resulting ADR or documentation change: MVP+11 plan and contract/target
  inventory retained; no ADR, dependency, stable ABI, or platform claim.

### Cycle 3 -- 2026-09-03

- Status entering review: Incubating; Slice 0 inventory complete.
- New evidence: SDD section 19 now gives current ADRs and AR-0017 precedence
  over its historical mobile sketches, removes implicit protector fallback,
  and classifies old estimates and precedents as non-evidence. The experimental
  ABI contract defines a callback-free outcome algebra, owned structured errors
  and byte results, provisional kind/generation-checked handles, explicit state
  transitions, conservative thread affinity, panic containment, and snapshot
  independence from parent-handle close.
- Findings: the schema is narrow enough to design an independent C harness
  without inventing multi-call transaction semantics. Its registry, numeric
  layout, symbols, unsafe implementation, and boundary-error vocabulary still
  require conservation and hostile-caller evidence.
- Disposition: remain Incubating. Admit review of the C harness, narrow unsafe
  boundary, and build inputs; do not implement exports until that review is
  retained.
- Resulting ADR or documentation change: normative SDD reconciliation and
  experimental ABI contract note; no ADR, dependency, or public ABI.

### Cycle 4 -- 2026-09-03

- Status entering review: Incubating; experimental behavior defined, no
  exported functions.
- New evidence: the harness admission review maps a zero-new-dependency adapter
  crate, hosted Linux C11 evidence profile, hand-maintained experimental header,
  thread-independent cleanup, UTF-8 path subset, panic-strategy constraint,
  exact unsafe-code budget, and conservation/hostile corpus. No C compiler is
  visible on the current Windows workstation, so independent execution evidence
  remains absent.
- Findings: a private `cdylib` experiment can pressure the boundary without
  changing `tosumu-core` or adding binding/build dependencies. Unsafe behavior
  can remain limited to checked input-slice construction and bounded copy-out.
  Linux success will not qualify another platform or stabilize the ABI.
- Disposition: remain Incubating. Admit the bounded private Linux
  implementation and independent C harness; do not admit publication, binding
  generation, mobile targets, callbacks, multi-call transactions, or stable
  symbols.
- Resulting ADR or documentation change: C-harness and unsafe-boundary admission
  record; no ADR, dependency, stable ABI, or platform claim.

### Cycle 5 -- 2026-09-03

- Status entering review: Incubating; private Linux implementation admitted.
- New evidence: GitHub Actions run `33817119731`, job `100851610793`, built the
  `cdylib`, compiled and dynamically linked an independent C11 process, matched
  the Tosumu export allowlist, and exercised binary values, snapshot survival,
  stale/wrong-kind/null failures, and complete string-detail error projection
  successfully on Ubuntu 24.04 with Rust 1.98.1, GCC 13.3, and GNU ld 2.42.
- Findings: the callback-free owner/result shape is viable for this one Linux
  profile, and the adapter added no third-party dependency. Bounded range and
  connection-observation representations plus deliberate panic injection are
  still missing from Slice 1. The result supplies no mobile, MSRV, cross-platform,
  arbitrary-pointer-safety, or stable-ABI evidence.
- Disposition: remain Incubating. Retain the independent result and continue the
  private Slice 1 experiment; do not stabilize or publish the boundary.
- Resulting ADR or documentation change: observed C-harness evidence and partial
  MVP+11 Slice 1 credit; no ADR, stable ABI, or mobile claim.

### Cycle 6 -- 2026-09-03

- Status entering review: Incubating; lifecycle/error C evidence obtained,
  observation and panic gates open.
- New evidence: GitHub Actions run `33817692660`, job `100853354051`, exercised
  the bounded immutable connection observation plus a feature-gated induced
  panic from the independent C process. The common containment wrapper returned
  the declared panic outcome, subsequent database use returned poisoned, close
  remained valid, and the job passed in 21 seconds.
- Findings: connection facts fit an immutable owned observation handle. Panic
  containment is credible for the exercised unwind profile, but does not cover
  allocator abort or foreign pointer faults. The current core snapshot scan
  fully materializes its range before an adapter can enforce limits, so exposing
  it as bounded would be false.
- Disposition: remain Incubating. Credit connection observation and the tested
  panic transition. Review a resumable pair/byte-bounded core scan under
  ADR-0006/0007 before adding range exports; reject adapter-side post-hoc
  truncation.
- Resulting ADR or documentation change: expanded C evidence and bounded-range
  blocker retained; no stable ABI or mobile claim.

### Cycle 7 -- 2026-09-03

- Status entering review: Incubating; bounded core range contract required
  before the private C experiment could expose pagination.
- New evidence: AR-0018 completed private traversal, fault, property, public
  Rust caller, and independent C caller gates. GitHub Actions run `33820788393`,
  job `100862796680`, compiled and ran the expanded C11 harness against commit
  `2f7969af3cf6a2adbcec6cf24c2f4739b4f2ce4b`. Immutable page handles preserved
  pair, continuation, blocked-size, and derived-byte ownership after database
  and page close without callbacks or physical cursors.
- Findings: a bounded logical range composes through the existing handle model
  without extending core with foreign policy. This closes Slice 1's remaining
  range omission but does not settle hostile-handle concurrency, multi-call
  writes, target packaging, or mobile lifecycle.
- Disposition: remain Incubating. Credit Slice 1 as complete, accept AR-0018
  through ADR-0006 for the Rust contract only, and move to Slice 2 hostile ABI
  and ownership closure. Keep every C symbol private and experimental.
- Resulting ADR or documentation change: bounded range no longer blocks the
  experiment; no stable ABI, wrapper, target, or mobile claim.

### Cycle 8 -- 2026-09-03

- Status entering review: Incubating; Slice 1 complete and hostile ownership
  evidence required before language wrappers.
- New evidence: a table-driven Rust corpus invokes 31 handle-taking exports
  against zero, `u64::MAX`, every wrong-kind handle, and the matching stale
  handle across database, snapshot, byte, error, connection, and scan-page
  kinds. It verifies all six close operations remove their entries. Separate
  cases cover every borrowed input position with null/nonzero and lengths above
  `isize::MAX`, empty bytes versus absence, undersized and null output buffers,
  malformed paths, oversized keys, invalid indices, every handle's thread
  policy, finalizer-thread close, and 64 close/use races. Eight feature-enabled
  FFI tests pass locally.
- Findings: the corpus caught two incorrect assumptions and one real boundary
  defect. Empty lookup keys preserve the existing absence result rather than a
  write-style invalid-key error. The feature-gated panic hook formerly panicked
  before validating its database handle and now performs normal kind/thread/
  poison admission first. Raw input now rejects lengths above `isize::MAX`
  before `from_raw_parts`, preventing an invalid Rust-slice construction.
- Disposition: remain Incubating. Credit the first handle matrix and every-kind
  thread policy. Do not close the broader input/concurrency checklist until
  registry exhaustion, irreducible foreign-region preconditions, database and
  snapshot close races, independent C hostile cases, and applicable sanitizer
  evidence are retained.
- Resulting ADR or documentation change: Slice 2 partial evidence only; no ABI,
  wrapper, platform, or safety-support claim.

### Cycle 9 -- 2026-09-03

- Status entering review: Incubating; handle/thread matrix retained, with
  capacity, database/snapshot races, independent C hostile cases, and sanitizer
  evidence still open.
- New evidence: the production registry insertion path now has isolated tests
  for its 4,096-entry ceiling, one-slot recovery, monotonic non-reuse, and
  counter exhaustion without insertion. Sixteen database and 64 snapshot
  close/use races add only the documented completed-or-stale outcomes. The full
  feature-enabled FFI crate now passes 13 local tests. GitHub Actions run
  `33822108887`, job `100866802176`, exercised cumulative commit
  `2ce0daa108a8d60acd0e449d0465ba5dd6fb729f` successfully in 22 seconds. Its
  independent C process covers a bounded hostile subset and runs both ordinary
  and GCC AddressSanitizer/UndefinedBehaviorSanitizer variants with leak
  detection and halt-on-error enabled; the exact symbol allowlist also passes.
- Findings: registry capacity is a recoverable boundary result, while allocator-
  level OOM remains a possible Rust process abort. Foreign region validity is
  an irreducible caller precondition; lengths above `isize::MAX` are rejected,
  immutable inputs may overlap, and the ABI exposes no private source address
  from which a conforming copy-out alias can be formed. C-side sanitizer success
  does not instrument or establish undefined-behavior freedom in Rust.
- Disposition: remain Incubating. Credit the completed handle, input, registry,
  thread, close/race, independent-C, C sanitizer, and symbol slices. Keep Slice
  2 open for the multi-point panic/cleanup matrix and evaluation of an admitted
  Rust-side UB tool.
- Resulting ADR or documentation change: ownership contract and plan narrowed;
  no stable ABI, memory-safety certification, wrapper, or mobile claim.

### Cycle 10 -- 2026-09-03

- Status entering review: Incubating; Slice 2 retained ownership and C-side
  sanitizer evidence, with multi-point panic cleanup and an applicable
  Rust-side undefined-behavior experiment still open.
- New evidence: feature-only injection now exercises panics before dispatch,
  after database lookup, and after acquiring real core write state. The latter
  stages a mutation before unwinding. Local feature-enabled adapter tests pass
  14 cases and show the common boundary contains all three panics, poisons
  later database use where association exists, permits close, and does not
  publish the staged mutation after reopen. GitHub Actions run `33822799882`
  exercised cumulative commit
  `f08b9bf4d0d1aa6bc848640d5358f09bc08f09f1`. Independent C job
  `100868927910` passed the cumulative hostile, sanitizer, symbol, and panic
  corpus. Miri job `100868928071` passed from 00:41:32 through 00:42:15 UTC on
  2026-09-04 using nightly 2026-09-03 and eight scheduler seeds.
- Findings: the post-acquisition case reaches a real core write closure rather
  than simulating adapter state, and conservation after reopen distinguishes a
  contained unwind from accidental commit. The Miri corpus is deliberately
  filesystem-free and covers raw input-slice formation, exported copy-out, and
  an immutable-result close/read race. It is useful Rust-side evidence for
  those executed paths only; it is not a soundness proof, exhaustive schedule
  exploration, foreign-pointer validation, or mobile-platform qualification.
- Disposition: remain Incubating. Credit Slice 2 as complete and move to the
  Slice 3 atomic multi-mutation admission review. Preserve allocator aborts,
  invalid foreign regions, platform exceptions, and unexecuted unsafe paths as
  outside the contained-panic/Miri observations.
- Resulting ADR or documentation change: MVP+11 plan advances to Slice 3; no
  stable ABI, memory-safety certification, wrapper, or mobile claim.
