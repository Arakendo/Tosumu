# AR-0010: Dependency Trust And Source Provenance

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-08-27 |
| Last reviewed | 2026-08-27 |
| Scope | Core dependency closure / build provenance / release process |
| Trigger | Tosumu relies on third-party cryptographic and storage-support crates but has no accepted policy for exact source identity beyond normal Cargo resolution |
| Related ADRs | ADR-0001, ADR-0002 |
| Related evidence | `Cargo.toml`, `Cargo.lock`, `SECURITY.md`, RustCrypto dependency choices, workspace target builds |

## Architectural Question

What source identity, audit, feature-minimization, update, and release evidence
is required for third-party code in Tosumu's authenticated storage and core
format closure?

## Context

Tosumu deliberately uses established RustCrypto primitives rather than
implementing cryptography itself. Cargo manifests and the lockfile provide
repeatable package resolution for the workspace, but they do not by themselves
define Tosumu's complete source-review, update, publication, or offline-build
policy.

A database engine also depends on non-cryptographic parsing, serialization,
randomness, synchronization, and platform crates. Build scripts and procedural
macros can participate in produced code even when they are not runtime
dependencies. Treating every dependency identically would be expensive, while
treating the whole closure as ordinary registry plumbing would understate the
trust placed in format- and authentication-critical code.

This review evaluates the boundary. It does not assume that Git submodules,
vendoring, registry checksums, or any other one mechanism is the correct answer.

## Evidence

- Tests or fuzzing: cryptographic known-answer tests, malformed-input tests,
  target builds, and fuzzing exercise behavior but do not establish source
  provenance.
- Independent consumers: published-library and external-provider workflows
  make downstream reproducibility relevant.
- Diagnostics or audits: `SECURITY.md` names the RustCrypto dependency and the
  unaudited composition risk; no retained recursive dependency audit exists.
- Repeated implementation friction: dependency and feature changes affect
  native, WASM, fuzz, and generated browser artifacts through different
  closures.
- Missing evidence: complete runtime/build/proc-macro closure inventory,
  selected-feature rationale, unsafe/build-script review, advisory process,
  offline-build requirements, update procedure, and publication consequences.

## Ownership And Dependency Analysis

- Tosumu owns the storage, format, authentication-boundary, and error semantics
  promised through its public APIs even when third-party code supplies
  mechanics.
- Third-party types and errors must not become durable Tosumu vocabulary merely
  because a crate is convenient to use internally.
- Core and authentication-critical dependencies carry a higher review burden
  than dev-only tools or presentation adapters.
- Cargo, the Rust toolchain, operating systems, hardware, and registries remain
  trust inputs requiring separate policy; this review must state rather than
  hide those limits.
- A dependency update is evidence-changing work even when upstream labels it a
  patch release, but the proportional update process remains unresolved.

## Alternatives Considered

### Alternative A: Rely on manifests, `Cargo.lock`, and registry checksums

- Benefits: standard Rust workflow and straightforward publication.
- Costs: limited retained source-review and offline provenance policy.
- Failure mode: a critical closure or feature change is reviewed only through
  compilation and tests.

### Alternative B: Require pinned repository submodules for the full core closure

- Benefits: exact source is rooted in the Tosumu repository graph and available
  for direct inspection.
- Costs: substantial transitive, procedural-macro, publication, and update
  complexity.
- Failure mode: policy cost drives stale dependencies or creates a source path
  downstream consumers cannot reproduce.

### Alternative C: Vendor the selected closure

- Benefits: offline builds and repository-local source inspection.
- Costs: copied source requires provenance, update, and generated-artifact
  discipline.
- Failure mode: the vendor tree becomes an opaque generated snapshot whose
  upstream identity is poorly retained.

### Alternative D: Use a risk-tiered provenance policy

- Benefits: focuses stronger evidence on authenticated storage, format, unsafe,
  build-time, and public-vocabulary dependencies.
- Costs: requires an explicit classification and machine-checkable inventory.
- Failure mode: convenient dependencies are classified downward without a
  defensible trust analysis.

## Findings

- Lockfile pinning is useful resolution evidence but is not a complete source
  audit or release policy.
- Mandatory submodules for the entire closure are not justified without a
  Tosumu-specific migration and publication study.
- Cryptographic primitives, randomness, byte parsing, unsafe code, build
  scripts, procedural macros, and public foreign types deserve explicit
  inventory.
- Correctness tests and vulnerability scanners support provenance review but do
  not replace architectural judgment about dependency ownership and exposure.

## Disposition

Incubating. Begin with a generated dependency-closure inventory and a focused
audit of authentication- and format-critical dependencies. Do not claim that
the current build is fully source-audited, vendored, or reproducible offline.

## Required Follow-Up

- [ ] Derive runtime, build, development, and procedural-macro closures from
      Cargo metadata.
- [ ] Identify dependencies that affect authentication, format parsing,
      randomness, unsafe boundaries, or public API vocabulary.
- [ ] Record selected features, licenses, source identity, build scripts,
      unsafe code, native/WASM implications, and update ownership for the first
      critical subset.
- [ ] Compare lockfile-only, vendored, exact-Git, submodule, and risk-tiered
      policies against publication and offline-build needs.
- [ ] Decide whether the result becomes an ADR, dependency-audit policy, CI
      check, or a combination of those artifacts.

The first concrete addition candidate is `fs4` with only its synchronous
feature for AR-0009's writer gate. Before admission, retain its exact resolved
version and checksum, transitive Unix/Windows closure, unsafe/platform boundary,
licenses, build scripts, selected features, Rust 1.75 result, native target
matrix, and WASM exclusion or behavior. Do not enable async runtime features.

### Focused Candidate Inventory: `fs4` 1.1.0

| Concern | Retained observation |
| --- | --- |
| Source identity | crates.io `fs4` 1.1.0; archive SHA-256 `7e72ed92b67c146290f88e9c89d60ca163ea417a446f61ffd7b72df3e7f1dfd5` |
| License | `MIT OR Apache-2.0`, matching Tosumu's license expression |
| Selected features | `default-features = false`, `features = ["sync"]`; no async runtime or `fs-err` adapter |
| Declared MSRV | Rust 1.75.0 |
| Windows normal closure | `fs4 1.1.0 -> windows-sys 0.61.2 -> windows-link 0.2.1` |
| Linux normal closure | `fs4 1.1.0 -> rustix 1.1.4 -> bitflags 2.13.1 + linux-raw-sys 0.12.1` |
| Build/proc-macro closure | No build dependency or procedural macro in the resolved normal/build tree; `rustix` does contain its own `build.rs`, which remains part of the source review |
| Unsafe/platform boundary | Direct raw-handle Windows locking calls and Unix borrowed-fd conversion; platform calls are supplied by `windows-sys` or `rustix` |
| Public vocabulary | Candidate remains private; `fs4` types/errors must map into Tosumu-owned guard and typed error details |
| Native check | Sync-only candidate compiled on `x86_64-pc-windows-msvc` in the isolated audit manifest |
| WASM check | An unconditional dependency fails `wasm32-unknown-unknown` through `rustix/errno`; a `cfg(any(unix, windows))` target dependency compiles while excluding the closure on WASM |
| Build scripts | No `fs4` build script observed; transitive source/build-script review remains required before final admission |

The focused review additionally confirmed that the enabled path has no
procedural macro or native-code compilation. `rustix/build.rs` probes compiler
and target capabilities and emits cfg selections; it does not fetch sources.
The actual locking boundary remains `fs4`'s small raw-handle/borrowed-fd layer
over the selected `windows-sys` or `rustix` calls. This does not constitute a
general audit of those ecosystems, but it is proportionate to admitting this
exact private, sync-only mechanism with lockfile checksums and target tests.

The candidate is admitted only for ADR-0004 under the constraints in the table.
AR-0010 remains Incubating for the repository's broader dependency policy.

## Reopening Triggers

- A new dependency enters `tosumu-core` or the authenticated format path.
- A dependency exposes foreign types through a durable public Tosumu API.
- A build script, procedural macro, unsafe implementation, or generated source
  materially changes the trusted closure.
- Release or incident response requires an offline or repository-rooted source
  build.
- A vulnerability or upstream ownership change affects a critical dependency.

## Review History

### Cycle 1 -- 2026-08-27

- Status entering review: Proposed
- New evidence: current dependency claims and Cargo-based resolution were
  compared with Tosumu's authenticated-storage boundary.
- Findings: provenance deserves focused evidence, but one mandatory source
  mechanism is not yet justified.
- Disposition: Incubating
- Resulting ADR or documentation change: none

### Cycle 2 -- 2026-08-27

- Status entering review: Incubating
- New evidence: MVP+10 needs cooperative cross-process writer admission, while
  standard Rust file locking starts after Tosumu's declared MSRV. `fs4` 1.1.0
  documents Rust 1.75 support for its sync feature and uses platform-specific
  locking beneath a sealed extension trait.
- Findings: this is a format-adjacent core dependency even though it changes no
  authenticated bytes. Its platform and transitive closure must be retained
  before use; async features are unrelated and excluded.
- Disposition: remain Incubating; admit no dependency until the focused
  inventory and target checks are recorded.
- Resulting ADR or documentation change: AR-0009 names `fs4` only as a candidate.

### Cycle 3 -- 2026-08-27

- Status entering review: Incubating
- New evidence: exact archive identity, sync-only native closures, direct unsafe
  boundaries, and target checks were retained in the focused candidate table.
- Findings: the candidate fits Rust 1.75 and the project license on native
  targets; unconditional use breaks `wasm32-unknown-unknown`, while a native-
  target-only dependency leaves the WASM closure empty.
- Disposition: remain Incubating pending transitive source/build-script review
  and the AR-0009 decision about the public raw-WAL bypass.
- Resulting ADR or documentation change: any future manifest entry must be
  target-specific to Unix/Windows and sync-only.

### Cycle 4 -- 2026-08-27

- Status entering review: Incubating
- New evidence: the enabled platform locking source and `rustix` build script
  were inspected; raw WAL scope and sidecar semantics were settled in AR-0009.
- Findings: exact private use has a bounded foreign/unsafe surface, preserves
  Rust 1.75 and WASM compilation when target-gated, and adds no async, proc-
  macro, or native compilation baggage.
- Disposition: admit `fs4` 1.1.0 only for ADR-0004's native sync-only mechanism;
  retain Incubating status for general provenance policy.
- Resulting ADR or documentation change: ADR-0004.
