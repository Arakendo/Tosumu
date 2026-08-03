# TOKIMU-001 Implementation Plan

Status: Proposed implementation plan
Source CR: [`tokimu-001-tasset-storage-provider-boundary.md`](tokimu-001-tasset-storage-provider-boundary.md)
Target: `tosumu-core` provider boundary, supporting CLI adapters, diagnostics, and reproducible consumer evidence
Updated: 2026-08-03

## 1. Objective

Deliver a documented Rust library boundary that lets Tokimu atomically store,
recover, inspect, back up, and export a multi-record `.tasset` corpus without
depending on Tosumu pager, B+ tree, WAL, crypto-frame, SQL, or CLI internals.

This plan does not add Tokimu asset semantics to Tosumu. Tokimu owns logical
keys, values, schema versions, migrations, provenance, and diagnostics. Tosumu
owns physical storage, transactions, recovery, integrity, backup, export, and
physical format compatibility.

## 2. Current Facts and Blocking Gaps

- `tosumu_core::page_store::PageStore` is public and already provides create,
  open, read-only open, get, put, delete, scan, scan-range, and transaction.
- `PageStore::transaction` provides atomic commit/rollback and WAL-backed crash
  recovery.
- Core errors are structured through `TosumuError` and `ErrorReport`.
- Core inspection already exposes header and page verification structures.
- Stable backup logic exists only in the CLI-private `cmd_backup` path.
- CLI inspection adds useful report shaping that is not yet a public core
  consumer contract.
- Keys and values are currently rejected above `u16::MAX` bytes. The requested
  1 MiB, 16 MiB, and 64 MiB fixtures therefore require a deliberate record and
  overflow-format change before the large-value acceptance criteria can pass.
- The current concurrency posture is single-process, single-writer, with
  explicit busy behavior. This CR must document that honestly rather than
  promising multi-process writes or MVCC.
- SQL is not part of this provider boundary.

## 3. Delivery Rules

- [ ] Preserve the dependency direction: consumers -> `tosumu-core` public API
      -> storage internals.
- [ ] Do not expose `Pager`, `BTree`, WAL frame, page, or encryption-frame types
      through the admitted provider API.
- [ ] Keep consumer reports and failures structured; no message parsing.
- [ ] Add no Tokimu-specific concepts to core APIs or physical records.
- [ ] Keep source backup, portable export, and verification as separate
      operations with distinct guarantees.
- [ ] Run the narrowest affected crate tests after each non-trivial edit.
- [ ] Update the CR evidence matrix as slices complete.

## 4. Slice Overview

| Slice | Deliverable | Gate |
| --- | --- | --- |
| 0 | Contract decisions and baseline evidence | Required before API edits |
| 1 | Admitted embeddable KV provider boundary | Tokimu adapter can begin |
| 2 | Large binary value contract and format support | Tokimu corpus can begin |
| 3 | Consumer-shaped atomicity and recovery corpus | Storage semantics proven |
| 4 | Library-level stable backup | Safe working-copy capture |
| 5 | Portable single-artifact export | `.tasset` may be called portable |
| 6 | Embedded inspection and verification boundary | Diagnostics integration |
| 7 | Physical/application version separation | Compatibility contract |
| 8 | Shared Tokimu-shaped fixture and evidence matrix | CR completion |

Slices 1 and 2 unblock initial Tokimu adapter work. Slice 5 is required before
Tokimu distributes a Tosumu-backed `.tasset` as a self-contained artifact.

## 5. Slice 0: Contract Decisions and Baseline

### Checklist

- [ ] Record `PageStore` as the candidate admitted provider entry point.
- [ ] Inventory every public type reachable from its method signatures.
- [ ] Document current create/open/read-only, close-on-drop, write
      serialization, process, and thread limitations.
- [ ] Record current key/value limits and the exact error/status returned when
      they are exceeded.
- [ ] Record current WAL recovery, checkpoint, backup, and verify behavior.
- [ ] Decide whether the provider remains `PageStore` directly or becomes a
      small role-focused facade that owns only admitted consumer operations.
- [ ] Decide the supported maximum value size and on-disk length encoding for
      Slice 2 before changing format bytes.
- [ ] Decide whether the large-value format change increments physical
      `format_version`; do not add automatic migration.
- [ ] Add durable decisions to the nearest architecture/format documents.

### Acceptance Criteria

- [ ] Every later slice has a named owning module and public API target.
- [ ] The 65,535-byte current value limit is explicitly acknowledged.
- [ ] No plan item relies on SQL, CLI-private types, or Tokimu code.
- [ ] Physical-format compatibility implications are decided before wire-format
      implementation begins.

## 6. Slice 1: Embeddable KV Provider Boundary

### Implementation

- [ ] Document the supported import path and lifecycle for create/open/open
      read-only/get/put/delete/scan/transaction.
- [ ] Add rustdoc examples that use only admitted core types.
- [ ] Make key/value size constants public if consumers need preflight checks.
- [ ] Ensure read-only mutation attempts return a stable structured error.
- [ ] Document that dropping the handle closes owned resources; add an explicit
      close/flush API only if a consumer-visible failure can otherwise be lost.
- [ ] Document `Send`/`Sync` reality, write serialization, file locking, and
      same-process/multi-process limitations.
- [ ] Add an integration test under `crates/tosumu-core/tests/` so it compiles
      against the crate's public surface only.
- [ ] In that test, atomically write metadata and multiple binary records, then
      reopen and verify exact values.
- [ ] Add rollback coverage proving a failed transaction exposes no partial
      logical asset.

### Validation

```text
cargo test -p tosumu-core --test provider_boundary
cargo test -p tosumu-core page_store
cargo clippy -p tosumu-core --all-targets -- -D warnings
```

### Acceptance Criteria

- [ ] An external Rust crate can implement an adapter using only documented
      `tosumu-core` provider and error types.
- [ ] No physical page, WAL, B+ tree, crypto-frame, SQL, or CLI type appears in
      the consumer-facing example.
- [ ] Multi-key commit is atomic.
- [ ] Closure failure rolls back all writes in that transaction.
- [ ] Read-only, busy, invalid-argument, corruption, and wrong-key states are
      machine-classifiable.

## 7. Slice 2: Large Binary Value Contract

This is a format-bearing slice. Current `u16` value lengths cannot represent
the CR corpus and must not be worked around by application-level silent
chunking unless that is chosen and documented as the provider contract.

### Design Gate

- [ ] Compare at least these designs:
  - widened logical value length plus Tosumu-owned overflow chain;
  - Tosumu-owned chunk manifest and chunk records hidden behind `PageStore`;
  - explicitly deferred streaming API over the same logical value contract.
- [ ] Choose one canonical owner for chunk/overflow reconstruction.
- [ ] Specify corruption checks for missing, duplicate, cyclic, truncated, and
      oversized overflow segments.
- [ ] Specify a practical enforced maximum value size and allocation checks.
- [ ] Specify old/new physical format open behavior and fixture expectations.
- [ ] Add a stable structured error code for values beyond the enforced limit.

### Implementation

- [ ] Replace the `u16::MAX` value ceiling with the chosen bounded contract.
- [ ] Keep key limits separately documented and enforced.
- [ ] Implement checked length arithmetic before allocation or page traversal.
- [ ] Cover put/get/overwrite/delete/reopen/scan for overflow-backed values.
- [ ] Reclaim overwritten/deleted overflow storage without leaving reachable
      stale records or violating freelist/B+ tree invariants.
- [ ] Measure peak resident allocation and copy count for 1 MiB, 16 MiB, and
      64 MiB operations in a repeatable benchmark or diagnostic test.
- [ ] Defer streaming unless measurements show whole-value buffering blocks
      realistic Tokimu assets.

### Test Matrix

- [ ] Empty value.
- [ ] Inline-boundary values immediately below/at/above the threshold.
- [ ] 1 MiB payload.
- [ ] 16 MiB payload.
- [ ] 64 MiB payload.
- [ ] Maximum accepted payload.
- [ ] One byte above the enforced maximum.
- [ ] Large-to-small and small-to-large overwrite.
- [ ] Delete and reinsert after reopen.
- [ ] Scan returns exact reconstructed values.
- [ ] Corrupt and truncated overflow chains produce structured findings.

### Acceptance Criteria

- [ ] 1 MiB, 16 MiB, and 64 MiB payloads round-trip byte-for-byte after close
      and reopen.
- [ ] Exact hashes remain stable across put, overwrite, scan, and recovery.
- [ ] Delete and overwrite preserve B+ tree and overflow invariants.
- [ ] Over-limit input fails before unbounded allocation or partial mutation.
- [ ] Allocation/copy measurements are recorded for Tokimu's streaming decision.

## 8. Slice 3: Consumer Atomicity and Recovery Corpus

### Fixture Shape

Use application-defined keys for:

- manifest and fixture schema version;
- provenance;
- dependency table;
- diagnostics;
- one small binary payload;
- one overflow-backed binary payload.

### Checklist

- [ ] Add a reusable core test fixture builder with deterministic bytes/hashes.
- [ ] Commit the full logical asset in one transaction.
- [ ] Add forced closure-error rollback coverage.
- [ ] Crash before the commit record and prove the prior state is restored.
- [ ] Crash after the commit record and prove the committed state is replayed.
- [ ] Cover interrupted multi-key create, overwrite, and delete.
- [ ] Assert each reopen yields exactly the prior asset or exactly the new
      asset, never a mixture.
- [ ] Return structured recovery observations for replayed committed work,
      discarded uncommitted work, busy state, and unrecoverable corruption.

### Acceptance Criteria

- [ ] The fixture passes all selected crash-injection sites.
- [ ] No outcome contains a mixed logical asset generation.
- [ ] Recovery outcomes can be classified without parsing display text.
- [ ] Large-value overflow remains valid after replay and rollback.

## 9. Slice 4: Library-Level Stable Backup

### Implementation

- [ ] Move stable-copy ownership from CLI `cmd_backup` into a focused
      `tosumu-core` backup module.
- [ ] Keep CLI backup as a thin renderer/adapter over the core operation.
- [ ] Define input/output types without CLI dependencies.
- [ ] Return a structured `BackupReport` containing source, main artifact,
      optional WAL artifact, attempts, and captured WAL/checkpoint state.
- [ ] Preserve bounded retry behavior and return structured `Busy` when source
      stability cannot be established.
- [ ] Reject existing destination main/WAL paths without partial replacement.
- [ ] Clean staged files on all failure paths.
- [ ] Document behavior while another handle is open and the exact consistency
      guarantee of the captured pair.

### Validation

```text
cargo test -p tosumu-core backup
cargo test -p tosumu-cli backup
```

### Acceptance Criteria

- [ ] An embedded consumer can request a stable backup without shelling out or
      copying Tosumu files itself.
- [ ] Success returns a complete committed main/WAL pair and structured report.
- [ ] Instability returns `Busy`; it never returns a silently inconsistent pair.
- [ ] Opening the backup reproduces the source's committed state.
- [ ] CLI behavior remains compatible while delegating storage semantics to core.

## 10. Slice 5: Portable Single-Artifact Export

### Design Gate

- [ ] Define whether export requires a closed source, obtains an exclusive
      lock, or operates from a stable backup copy.
- [ ] Define how committed WAL frames are reconciled without ambiguously
      mutating the source.
- [ ] Define destination replacement, fsync, rename, and directory durability
      guarantees for supported filesystems.

### Implementation

- [ ] Add a library-level export operation returning a structured
      `PortableExportReport`.
- [ ] Reconcile all committed WAL frames into the destination main file.
- [ ] Ensure the successful destination requires no WAL sidecar.
- [ ] Verify the destination header, pages, overflow chains, and B+ tree before
      publishing the final path.
- [ ] Report blocking readers/writers or unreconciled WAL as structured failure.
- [ ] Preserve the source database and sidecar state unless the API explicitly
      documents a source checkpoint operation.
- [ ] Test failure cleanup and destination non-replacement.

### Acceptance Criteria

- [ ] The exported file can be copied alone to a new directory and opened.
- [ ] All committed keys and hashes match the source logical state.
- [ ] Verification succeeds with every source-side sidecar hidden or removed.
- [ ] Success never requires an undocumented companion file.
- [ ] WAL reconciliation failure is explicit and leaves no published partial
      artifact.

## 11. Slice 6: Embedded Inspection and Verification

### Implementation

- [ ] Inventory public core inspection reports versus CLI-only payload shaping.
- [ ] Define one stable incubation-level Rust report for header, WAL, page,
      overflow, and B+ tree verification observations.
- [ ] Keep reportable findings separate from fatal inspection failures.
- [ ] Give findings stable codes/categories; descriptions remain supplemental.
- [ ] Preserve distinctions for corrupt page, corrupt overflow chain,
      unsupported format, wrong key, file busy, and incomplete B+ tree checks.
- [ ] Refactor CLI JSON rendering to consume the same core observations where
      practical rather than rebuilding storage meaning.
- [ ] Verify both working databases and portable exports through the API.

### Acceptance Criteria

- [ ] Tokimu can render storage diagnostics from structured core reports.
- [ ] No consumer must import `Pager` or scrape CLI/human-readable output.
- [ ] Findings and fatal failures are distinct in types and status.
- [ ] Portable artifact verification exercises physical format, pages,
      overflow, and B+ tree validity.

## 12. Slice 7: Physical Format vs Application Schema

### Checklist

- [ ] Document that Tosumu `format_version` governs physical storage only.
- [ ] Document that Tokimu owns `.tasset` schema and adapter codec versions.
- [ ] Preserve explicit unsupported physical-version rejection.
- [ ] Add no automatic physical migration to open paths.
- [ ] Expose validated physical version/header information through the admitted
      inspection boundary.
- [ ] Demonstrate transactionally storing and changing application-owned
      schema/version records as ordinary values.
- [ ] Add adapter guidance for mapping physical incompatibility separately from
      application-schema incompatibility.

### Acceptance Criteria

- [ ] Physical and application schema incompatibilities are distinct failures.
- [ ] Tosumu assigns no semantic meaning to `.tasset` schema records.
- [ ] A newer unsupported physical format remains a structured open/inspect
      error, not an attempted migration.

## 13. Slice 8: Shared Consumer Fixture and Evidence

### Checklist

- [ ] Add a deterministic fixture specification under `docs/CRs/Tokimu/`.
- [ ] Record Tosumu crate version, physical format version, fixture schema
      version, keys, payload sizes, and expected hashes.
- [ ] Add a core integration test that builds the fixture without Tokimu code.
- [ ] Round-trip create, commit, reopen, verify, stable backup, portable export,
      and independent reopen.
- [ ] Exercise corruption, wrong-key, newer-format, busy, and identity-isolation
      cases from the CR evidence matrix.
- [ ] Publish machine-readable fixture metadata if both projects need automated
      comparison.
- [ ] Update the CR evidence matrix with commands, results, and artifact hashes.

### Acceptance Criteria

- [ ] Tosumu runs the fixture using only core public APIs.
- [ ] Tokimu can reproduce equivalent logical observations through its adapter.
- [ ] Matching keys, values, versions, and hashes establish the shared boundary.
- [ ] The fixture proves backup and portable export are different guarantees.
- [ ] Every CR evidence-matrix row is passed or explicitly deferred with owner
      and rationale.

## 14. Cross-Cutting Error Contract

- [ ] Reuse existing `TosumuError` variants and public codes where phenomena
      already match.
- [ ] Add new variants/codes only for stable cross-boundary phenomena such as
      value-too-large, backup instability, export reconciliation failure, or
      overflow-chain corruption.
- [ ] Create errors where the failure is understood and preserve source errors.
- [ ] Keep CLI exit codes and human text at the CLI boundary.
- [ ] Add code-catalog tests for every newly public structured code.
- [ ] Keep aspirational codes out of `ERRORS.md` until code emits them.

## 15. Final Validation

Run focused tests after each slice, then complete:

```text
cargo test -p tosumu-core
cargo test -p tosumu-cli
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

For large-value and crash tests, document any ignored/manual cases and provide
their exact commands. Do not count an ignored 64 MiB or crash test as evidence
until its explicit run passes.

## 16. CR Completion Criteria

TOKIMU-001 is complete only when all of the following are true:

- [ ] The admitted provider API is documented and tested as an external crate.
- [ ] A multi-record logical asset commits or rolls back atomically.
- [ ] 1 MiB, 16 MiB, and 64 MiB values pass reopen and recovery tests.
- [ ] Stable backup is available from `tosumu-core` with a structured report.
- [ ] Portable export produces one independently verifiable file.
- [ ] Embedded verification returns structured observations and failures.
- [ ] Physical format and `.tasset` schema versions remain separate concerns.
- [ ] The shared fixture passes the requested evidence matrix.
- [ ] Workspace tests and Clippy pass.
- [ ] Tokimu can implement its adapter without importing Tosumu internals.

## 17. Explicitly Deferred

- SQL-backed asset schemas or queries.
- Generic application schema migration framework.
- Async I/O and streaming blobs without measurement evidence.
- Multi-process writes or MVCC work.
- Network, browser, or WASM storage providers.
- Tokimu-specific storage types, runtime cooking, or inspection panes.
- Automatic physical-format migration.