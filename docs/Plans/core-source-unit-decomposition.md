# Core Source Unit Decomposition

| Field | Value |
| --- | --- |
| Status | Proposed |
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

- [ ] Record the current focused test names and counts for page store, pager,
      and WAL.
- [ ] Run focused package tests and the full workspace tests.
- [ ] Retain representative fixture hashes or output artifacts where tests
      depend on committed fixtures.
- [ ] Record any existing ignored, environment-dependent, or flaky evidence
      before moving code.

Exit state: the structural campaign has a reproducible baseline and does not
mistake an existing failure for decomposition drift.

### Slice 1: Page Store Test Families

- [ ] Move shared test-only construction into a narrow private support module.
- [ ] Move basic operations, transaction/recovery, consumer recovery,
      protector lifecycle, hostile-input, differential, and failure-preservation
      tests into invariant-named private modules.
- [ ] Preserve test assertions, fixtures, feature gates, and production
      visibility.
- [ ] Confirm the implementation remains a coherent store composition unit.

Exit state: `page_store.rs` communicates the store responsibility without a
2,600-line anonymous test body.

### Slice 2: WAL Test Families

- [ ] Move record I/O, recovery, checkpoint, locking, and crash-preservation
      evidence into invariant-named private modules.
- [ ] Keep framing, decoding, retry, and recovery semantics solely in the
      production WAL implementation.
- [ ] Preserve fault-injection cleanup and serialized test behavior.

Exit state: WAL tests are navigable by invariant while the cohesive production
pipeline remains intact.

### Slice 3: Pager Private Responsibilities

- [ ] Extract page-zero validation and atomic editing behind private inputs and
      typed results.
- [ ] Extract protector-specific unlock mechanics without moving authority or
      key material outside the pager boundary.
- [ ] Review the resulting dependency direction and state access before moving
      open or transaction orchestration.
- [ ] Extract another seam only if it reduces coupling and preserves a thin,
      understandable pager composition root.
- [ ] Retain a failed extraction as evidence rather than hiding broad shared
      state behind pass-through functions.

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
