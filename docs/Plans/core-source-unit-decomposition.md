# Core Source Unit Decomposition

| Field | Value |
| --- | --- |
| Status | Complete |
| Opened | 2026-08-27 |
| Last updated | 2026-08-27 |
| Authority | Tracking plan; ADR-0001, ADR-0002, and ADR-0003 remain binding |
| Target | Behavior-preserving cohesion work for threshold-crossing `tosumu-core` source units |
| Related ADRs | ADR-0001, ADR-0002, ADR-0003 |
| Related reviews | AR-0007, AR-0008 |

## Purpose

Apply ADR-0003 to the current core source tree, retain the required explicit
reviews, and sequence only the decompositions justified by demonstrated
responsibility seams.

This plan does not authorize changes to public APIs, the on-disk format,
authentication, WAL or recovery semantics, error identities, or the dependency
graph. Any such pressure stops the mechanical campaign and returns to the
owning specification or Architectural Review.

## Baseline Inventory

Physical line counts on 2026-08-27 are:

| Source unit | Physical lines | Pre-test implementation | Inline test module | ADR-0003 treatment |
| --- | ---: | ---: | ---: | --- |
| `page_store.rs` | 3,064 | 413 | 2,651 | Explicit decomposition review |
| `pager.rs` | 2,207 | 1,993 | 214 | Explicit decomposition review |
| `wal.rs` | 2,027 | 731 | 1,296 | Explicit decomposition review |
| `btree.rs` | 1,854 | 1,336 | 518 | Inspect during substantive modification |
| `inspection_session.rs` | 1,203 | 784 | 419 | Inspect during substantive modification |
| `inspect.rs` | 1,164 | 877 | 287 | Inspect during substantive modification |

The pre-test values identify the first top-level `mod tests` boundary and are
only responsibility evidence. Test-only helper implementations that appear
earlier remain part of the source unit's physical review burden.

## Review 1: `page_store.rs`

### Responsibility Inventory

The 413-line implementation is a coherent high-level key/value composition
surface over `BTree` and `Pager`. It owns:

- store creation and opening through supported protectors;
- protector-management forwarding;
- key/value validation and operations;
- store statistics; and
- transaction composition and crash-test injection entry points.

The 2,651-line inline test module contains several independently navigable
evidence families:

- basic create/open and key/value behavior;
- transaction, WAL, overflow, and root-split recovery;
- Tokimu-shaped consumer generation recovery;
- protector lifecycle and encrypted reopen behavior;
- malformed header, keyslot, ciphertext, and cross-database attacks;
- differential property and crash-model testing; and
- failure-preserves-file and read-only behavior.

### Ownership And Coupling

The implementation remains the public store composition owner. Moving its
protector forwarding into pager or crypto modules would not reduce ownership
coupling and could obscure the supported store surface.

The test families share fixtures and private access but do not need to remain
one anonymous source unit. They can be private child modules with a small
test-only support module. Helpers must not duplicate key management, recovery,
or model semantics.

### Disposition

Decompose the inline tests by invariant. Retain the production implementation
unless a later substantive change demonstrates a separate owner. This is a
test-organization change, not a public API or storage refactor.

## Review 2: `pager.rs`

### Responsibility Inventory

The approximately 1,993-line pre-test implementation currently contains:

- database creation, open, unlock, and read-only construction;
- page-zero and header validation;
- keyslot addition, removal, rotation, and enumeration;
- protector-specific unlock helpers;
- authenticated frame reading and writing;
- allocation, release, freelist, and root-page persistence;
- WAL-backed transaction begin, commit, phase-two flush, and rollback;
- committed-WAL overlay during open; and
- an atomic page-zero edit session used by protector management.

These responsibilities share the `Pager` trust boundary but are not one
implementation responsibility. In particular, page-zero/protector lifecycle,
open/unlock orchestration, steady-state frame I/O, and transaction publication
are independently reviewable paths.

### Ownership And Coupling

All extracted modules remain private children of `pager`; the pager remains the
authenticated storage trust boundary under ADR-0002. No ciphertext, plaintext
page structure, key material, raw file object, WAL writer, or protector
mechanism becomes public.

The first extraction should target helpers with already explicit inputs and
outputs rather than pass one broad mutable pager context between new modules.
Candidate seams to validate are:

```text
pager/
    page0.rs         header fields, page-zero validation, and atomic edits
    unlock.rs        protector-specific bounded unlock mechanics
    open.rs          construction and committed-WAL overlay orchestration
    transaction.rs   transaction state transitions and phase-two publication
```

These names are candidates, not pre-created template files. Frame I/O,
allocation, and header persistence may remain in the parent when extracting
them would create cycles or broad shared state.

### Disposition

Decompose at a checkpoint before the next substantive pager, protector, WAL
publication, or MVP+10 change. Start with page-zero and unlock helpers, then
review coupling before extracting transaction code. Do not combine extraction
with concurrency or storage-semantic changes.

## Review 3: `wal.rs`

### Responsibility Inventory

The 731-line implementation owns a coherent WAL pipeline:

- bounded retrying file open;
- WAL record encoding and decoding;
- append, sync, scan, and truncation;
- recovery and committed-page application;
- checkpoint publication; and
- append-state validation.

The 1,296-line inline test module separately covers:

- record encoding, reading, and LSN behavior;
- corrupt, partial, and oversized record handling;
- committed and uncommitted recovery;
- real pager and B+ tree recovery composition;
- checkpoint and truncate failure preservation;
- transient file-lock retry and structured busy diagnostics; and
- crash points around page writes and commit records.

### Ownership And Coupling

Writer, reader, recovery, and checkpoint code form a directional physical WAL
pipeline and share framing rules. Splitting the implementation now would add
module boundaries without evidence that its 731 lines are incoherent.

The test families are independent evidence groups and can move to private child
modules. Shared fixture construction must remain mechanical and must not become
a second WAL decoder or recovery implementation.

### Disposition

Decompose the inline tests by invariant. Retain the production implementation
for now. Reopen production decomposition if recovery, checkpointing, or MVP+10
retention work makes ordinary changes touch unrelated regions.

## Deferred Threshold Inspections

`btree.rs`, `inspection_session.rs`, and `inspect.rs` are in ADR-0003's
1,001–2,000-line band. They require cohesion inspection during their next
substantive modification, not immediate decomposition. The modifying plan or
change record must retain the local disposition.

## Implementation Slices

### Slice 0: Conservation Baseline

- [x] Record the current focused test names and counts for page store, pager,
      and WAL.
- [x] Run focused package tests and the full workspace tests.
- [x] Retain representative fixture hashes or output artifacts where tests
      depend on committed fixtures.
- [x] Record any existing ignored, environment-dependent, or flaky evidence
      before moving code.

Exit state: the structural campaign has a reproducible baseline and does not
mistake an existing failure for decomposition drift.

### Slice 1: Page Store Test Families

- [x] Move shared test-only construction into a narrow private support module.
- [x] Move basic operations, transaction/recovery, consumer recovery,
      protector lifecycle, hostile-input, differential, and failure-preservation
      tests into invariant-named private modules.
- [x] Preserve test assertions, fixtures, feature gates, and production
      visibility.
- [x] Confirm the implementation remains a coherent store composition unit.

Exit state: `page_store.rs` communicates the store responsibility without a
2,600-line anonymous test body.

### Slice 2: WAL Test Families

- [x] Move record I/O, recovery, checkpoint, locking, and crash-preservation
      evidence into invariant-named private modules.
- [x] Keep framing, decoding, retry, and recovery semantics solely in the
      production WAL implementation.
- [x] Preserve fault-injection cleanup and serialized test behavior.

Exit state: WAL tests are navigable by invariant while the cohesive production
pipeline remains intact.

### Slice 3: Pager Private Responsibilities

- [x] Extract page-zero validation and atomic editing behind private inputs and
      typed results. Stateless layout, validation, field access, keyslot writes,
      and header construction live in `pager/page0.rs`; the atomic edit session
      lives with its unlock dependency in `pager/unlock.rs`.
- [x] Extract protector-specific unlock mechanics without moving authority or
      key material outside the pager boundary.
- [x] Review the resulting dependency direction and state access before moving
      open or transaction orchestration.
- [x] Extract another seam only if it reduces coupling and preserves a thin,
      understandable pager composition root.
- [x] Retain a rejected extraction as evidence rather than hiding broad shared
      state behind pass-through functions. Transaction publication remains in
      the parent because it directly owns most mutable `Pager` state.

Exit state: the pager trust boundary remains structurally identical while its
demonstrated private responsibilities are easier to inspect and test.

## Conservation And Validation

Every slice must preserve:

- public API and visibility;
- exact on-disk bytes and fixture compatibility;
- AEAD, header-MAC, keyslot, and unlock behavior;
- WAL record bytes, LSN ordering, commit, checkpoint, and recovery behavior;
- typed error codes and diagnostic details;
- crash and fault-injection outcomes;
- native and WASM compilation where affected; and
- the absence of SQL, CLI, UI, or consumer semantics in core.

Validation after each coherent slice:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
```

Run applicable fuzz targets after moving byte-decoding or recovery test
boundaries. A pure test-file move does not establish new fuzz evidence.

## Completion Criteria

The plan may close when:

- page-store and WAL evidence is grouped by named invariants;
- pager extraction reduces responsibility coupling rather than only line count;
- no public, format, security, durability, or recovery contract changed;
- the three explicit ADR-0003 reviews have retained dispositions;
- deferred threshold files have named next-change triggers; and
- all applicable validation passes.

If a proposed seam requires a public API, changes the authenticated pager trust
boundary, or alters WAL/format behavior, this plan pauses and the responsible
Architectural Review or ADR is updated first.

## Progress Log

### 2026-09-03 -- Credible-Variation Seam Amendment

- ADR-0003 now treats credible future implementation variation as a valid
  decomposition trigger before a source unit becomes large or a second
  implementation is complete.
- The rule prefers narrow private capability seams with conservation evidence;
  it does not manufacture public provider APIs or let runtime selection own
  durable format meaning.
- AR-0016 is the first active application: crypto-provider pressure justifies
  early preparation, while exact format-v3 and authenticated-pager constraints
  remain separately reviewed.

### 2026-08-27 -- Conservation Baseline And First Extraction

- Baseline page-store evidence discovered 85 focused tests: 82 passed and three
  expensive Argon2/large-recovery cases remained explicitly ignored.
- Baseline WAL evidence discovered 30 focused tests: 29 passed and one manual
  Windows file-lock case remained explicitly ignored.
- The 11-test pager-focused run passed. The pre-change workspace
  `cargo test --workspace --all-targets` run also passed, including Criterion
  benchmarks, the long differential crash-recovery property, and the 64 MiB
  external-provider case. Rust reported only the known incremental-cache
  hard-link fallback warning.
- No fixture or golden file changed during the decomposition, so committed
  fixture identity remains the retained baseline.
- `page_store.rs` now retains its 415-line composition implementation and
  delegates test evidence to private `basic`, `recovery`, `protectors`,
  `hostile_input`, `storage_behavior`, `differential`, and
  `key_management_resilience` modules. All 85 test identities remain present
  under the added invariant path component.
- `wal.rs` now retains its 733-line production pipeline and delegates evidence
  to private `record_io`, `recovery`, `locking`, and `crash_preservation`
  modules. All 28 runnable and one ignored WAL-module tests remain present; the
  focused `wal` filter also executes one export test, for 30 total cases.
- `pager/page0.rs` is a 219-line private child depending only on file I/O,
  format constants, and typed core errors. The pager root consumes those
  helpers; the child does not receive a `Pager`, WAL writer, protector policy,
  or broad mutable context. `pager.rs` is now 1,998 lines.
- Moving `Page0EditSession` now would create a dependency back to pager-owned
  unlock orchestration. It remains local until the unlock seam is reviewed;
  reduced line count is not treated as sufficient evidence for that move.
- Post-change validation passed `cargo fmt --all -- --check`, strict workspace
  Clippy, focused page-store/WAL/pager tests, `cargo test --workspace
  --all-targets`, and `mkdocs build --strict`. The post-change page-store
  property test and 64 MiB external-provider case retained their baseline
  outcomes and comparable long-running behavior.

### 2026-08-27 -- Pager Unlock And Atomic Edit Session

- `pager/unlock.rs` now owns protector selection for key management, bounded
  passphrase/KEK slot scans, exact-length keyfile loading, and the atomic
  page-zero edit session. It is a private pager child; its exported surface is
  restricted to `pub(super)` inputs consumed by the pager composition root.
- The edit session moved with its unlock dependency, resolving the coupling
  noted in the first extraction without giving key material, raw file access,
  or protector mechanics public visibility. `pager.rs` is now 1,824 physical
  lines and `pager/unlock.rs` is 188 physical lines.
- The pager root depends on `page0` and `unlock`; neither child receives a
  `Pager`, WAL writer, or broad mutable context. The child dependency remains
  one-way from `unlock` to stateless `page0` helpers.
- Open/recovery orchestration remains in the root because it constructs the
  complete pager state and coordinates WAL checkpoint or read-only overlay.
  Transaction publication also remains there because extracting it would
  require sharing nearly every mutable transaction, file, header, and WAL
  field. That would obscure ownership rather than reduce coupling.
- Focused validation passed all 11 pager tests, all 12 protector lifecycle
  tests, and 25 key-management resilience tests; the existing 2,048-Argon2-call
  bit-flip test remained explicitly ignored.
- Final validation passed `cargo fmt --all -- --check`, strict workspace
  Clippy, `cargo test --workspace --all-targets`, and `mkdocs build --strict`,
  including the long differential crash-recovery property and maximum-size
  external-provider case. Rust reported only the known incremental-cache
  hard-link fallback warning.

### 2026-08-27 -- Pager Snapshot Selection

- MVP+10 generation selection first reached semantic checkpoint `480193c`,
  preserving a precise conservation baseline before structural movement.
- `pager/snapshot.rs` now owns process-local pin capture, owner-identity
  validation, page-zero selection at the captured generation, snapshot page
  bounds, retained-frame selection, and authenticated page delivery. It is a
  private child implementation over immutable pager state and exposes only the
  crate-private pin/read seam plus one parent-visible frame selector.
- The extraction moved 79 physical lines of one independently testable
  responsibility. `pager.rs` moved from 2,328 to 2,264 physical lines; the
  modest reduction is accepted because ownership, not a threshold target,
  defines the seam.
- Transaction publication remains in the pager root. It mutates the file, WAL,
  transaction metadata, header fields, health state, committed index, and
  registry-dependent checkpoint decision together; moving it behind a child
  `impl Pager` would relocate broad mutable coupling rather than reduce it.
- Focused post-extraction validation preserved all 14 pager tests, including
  retained-WAL lifetime, distinct pinned generations, page-zero bounds, foreign
  owner rejection, and both phase-two failure fixtures. No format bytes,
  errors, visibility, or public API changed.

### 2026-08-27 -- B+ Tree Read-Source Cohesion Review

- The next substantive `btree.rs` change triggered ADR-0003's deferred
  1,001-2,000-line cohesion inspection. Snapshot lookup would otherwise have
  duplicated point traversal and overflow decoding inside the root unit.
- `btree/read.rs` now owns the immutable logical-read pipeline: current and
  pinned read sources, root-to-leaf descent, point decoding, and bounded
  overflow-chain assembly. Mutation, split, freelist, invariant, and physical
  scan behavior remain in the B+ tree root.
- The dependency is one-way: the child consumes pager read contracts and
  parent-private page decoders; it cannot mutate pages, allocate, commit, or
  interpret provider/consumer meaning. Write traversal delegates to the same
  current-view descent, preventing a second separator-selection algorithm.
- `btree.rs` moved from 1,888 to 1,833 physical lines and the cohesive read
  child is 204 lines. The total implementation grew for the admitted snapshot
  behavior, while the root's responsibility count and algorithm duplication
  decreased.
- The focused logical fixture covers old-root traversal, an overwritten and
  freed overflow chain, 500 later commits, exclusion of a later key, and the
  first zero-reader checkpoint after pin drop.
- The subsequent range slice stayed within the same child responsibility:
  point and range reads now share root descent, retained leaf access, and
  overflow decoding for current and pinned sources. `btree.rs` is 1,858 lines
  and `btree/read.rs` is 260 lines after the new semantic coverage; no second
  traversal or snapshot-specific record decoder was added.
