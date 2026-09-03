# AR-0017: Mobile Embedding, ABI, And Hardware-Protector Boundary

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-09-03 |
| Last reviewed | 2026-09-03 |
| Scope | Mobile embedding adapter / C ABI / Swift and Kotlin wrappers / hardware-backed protectors |
| Trigger | MVP+11 is next in the delivery sequence, while the normative mobile design contains unverified and outdated implementation assumptions |
| Related ADRs | ADR-0001, ADR-0002, ADR-0003, ADR-0004, ADR-0010 |
| Related reviews | AR-0007, AR-0009, AR-0010, AR-0016 |
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
