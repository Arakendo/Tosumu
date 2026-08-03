# TOKIMU-001 Implementation Plan

Status: Proposed implementation plan
Source CR: [`tokimu-001-tasset-storage-provider-boundary.md`](tokimu-001-tasset-storage-provider-boundary.md)
Slice 0 baseline: [`tokimu-001-provider-baseline.md`](tokimu-001-provider-baseline.md)
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
- Keys remain limited by the current `u16` key encoding. Values now have a
      bounded version-2 overflow path with a 64 MiB logical maximum; the remaining
      work is broad test coverage, corruption verification, and allocation/copy
      measurement.
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
- [x] Update the CR evidence matrix as slices complete.

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

- [x] Record `PageStore` as the candidate admitted provider entry point.
- [x] Inventory every public type reachable from its method signatures.
- [x] Document current create/open/read-only, close-on-drop, write
      serialization, process, and thread limitations.
- [x] Record current key/value limits and the exact error/status returned when
      they are exceeded.
- [x] Record current WAL recovery, checkpoint, backup, and verify behavior.
- [x] Decide whether the provider remains `PageStore` directly or becomes a
      small role-focused facade that owns only admitted consumer operations.
- [x] Decide the supported maximum value size and on-disk length encoding for
      Slice 2 before changing format bytes.
- [x] Decide whether the large-value format change increments physical
      `format_version`; do not add automatic migration.
- [x] Add durable decisions to the nearest architecture/format documents.

### Acceptance Criteria

- [x] Every later slice has a named owning module and public API target.
- [x] The 65,535-byte current value limit is explicitly acknowledged.
- [x] No plan item relies on SQL, CLI-private types, or Tokimu code.
- [x] Physical-format compatibility implications are decided before wire-format
      implementation begins.

## 6. Slice 1: Embeddable KV Provider Boundary

### Implementation

- [x] Document the supported import path and lifecycle for create/open/open
      read-only/get/put/delete/scan/transaction.
- [x] Add rustdoc examples that use only admitted core types.
- [x] Make key/value size constants public if consumers need preflight checks.
- [x] Ensure read-only mutation attempts return a stable structured error.
- [x] Document that dropping the handle closes owned resources; add an explicit
      close/flush API only if a consumer-visible failure can otherwise be lost.
- [x] Document `Send`/`Sync` reality, write serialization, file locking, and
      same-process/multi-process limitations.
- [x] Add an integration test under `crates/tosumu-core/tests/` so it compiles
      against the crate's public surface only.
- [x] In that test, atomically write metadata and multiple binary records, then
      reopen and verify exact values.
- [x] Add rollback coverage proving a failed transaction exposes no partial
      logical asset.

### Validation

```text
cargo test -p tosumu-core --test provider_boundary
cargo test -p tosumu-core page_store
cargo clippy -p tosumu-core --all-targets -- -D warnings
```

### Acceptance Criteria

- [x] An external Rust crate can implement an adapter using only documented
      `tosumu-core` provider and error types.
- [x] No physical page, WAL, B+ tree, crypto-frame, SQL, or CLI type appears in
      the consumer-facing example.
- [x] Multi-key commit is atomic.
- [x] Closure failure rolls back all writes in that transaction.
- [x] Read-only, busy, invalid-argument, corruption, and wrong-key states are
      machine-classifiable.

## 7. Slice 2: Large Binary Value Contract

This is a format-bearing slice. Current `u16` value lengths cannot represent
the CR corpus and must not be worked around by application-level silent
chunking unless that is chosen and documented as the provider contract.

### Design Gate

- [x] Compare at least these designs:
  - widened logical value length plus Tosumu-owned overflow chain;
  - Tosumu-owned chunk manifest and chunk records hidden behind `PageStore`;
  - explicitly deferred streaming API over the same logical value contract.
- [x] Choose one canonical owner for chunk/overflow reconstruction.
- [x] Specify corruption checks for missing, duplicate, cyclic, truncated, and
      oversized overflow segments.
- [x] Specify a practical enforced maximum value size and allocation checks.
- [x] Specify old/new physical format open behavior and fixture expectations.
- [x] Add a stable structured error code for values beyond the enforced limit.

### Implementation

- [x] Replace the `u16::MAX` value ceiling with the chosen bounded contract.
- [x] Keep key limits separately documented and enforced.
- [x] Implement checked length arithmetic before allocation or page traversal.
- [x] Cover put/get/overwrite/delete/reopen/scan for overflow-backed values.
- [x] Reclaim overwritten/deleted overflow storage without leaving reachable
      stale records or violating freelist/B+ tree invariants.
- [x] Measure one overwrite and logical copy volume for 1 MiB, 16 MiB, and
      the repeatable diagnostic tests
      `large_value_write_measurement_one_megabyte` and
      `large_value_write_measurement_sixteen_megabytes`, and
      `large_value_write_measurement_maximum_value`.
- [ ] Defer streaming unless measurements show whole-value buffering blocks
      realistic Tokimu assets.

### Test Matrix

- [x] Empty value.
- [x] Inline-boundary values immediately below/at/above the threshold.
- [x] 1 MiB payload.
- [x] 16 MiB payload.
- [x] 64 MiB payload.
- [x] Maximum accepted payload.
- [x] One byte above the enforced maximum.
- [x] Large-to-small and small-to-large overwrite.
- [x] Delete and reinsert after reopen.
- [x] Scan returns exact reconstructed values.
- [x] Corrupt and truncated overflow chains produce structured findings.

### Acceptance Criteria

- [x] 1 MiB, 16 MiB, and 64 MiB payloads round-trip byte-for-byte after close
      and reopen.
- [x] Exact hashes remain stable across put, overwrite, scan, and recovery.
- [x] Delete and overwrite preserve B+ tree and overflow invariants.
- [x] Over-limit input fails before unbounded allocation or partial mutation.
- [ ] Logical copy-volume and timing measurements are recorded for Tokimu's
      streaming decision: 1 MiB (1,048,576 bytes; 3,287.9 ms), 16 MiB
      (16,777,216 bytes; 49,666.2 ms), and 64 MiB (67,108,864 bytes;
      203,219.2 ms). Exact allocator/peak-RSS counts remain unclaimed, so the
      streaming decision remains open.

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

- [x] Add a reusable core test fixture builder with deterministic bytes/hashes.
- [x] Commit the full logical asset in one transaction.
- [x] Add forced closure-error rollback coverage.
- [x] Crash before the commit record and prove the prior state is restored.
- [x] Crash after the commit record and prove the committed state is replayed.
- [x] Cover interrupted multi-key create, overwrite, and delete.
- [x] Assert each reopen yields exactly the prior asset or exactly the new
      asset, never a mixture.
- [x] Return structured recovery observations for replayed committed work,
      discarded uncommitted work, busy state, and unrecoverable corruption.

### Acceptance Criteria

- [x] The fixture passes all selected before- and after-commit crash-injection
      sites.
- [x] No outcome contains a mixed logical asset generation for the selected
      create, overwrite, and delete cases.
- [x] Recovery outcomes can be classified without parsing display text;
      successful WAL transactions use `inspect_recovery`, while busy and
      unrecoverable states remain structured `TosumuError` results.
- [x] Large-value overflow remains valid after replay and rollback.

## 9. Slice 4: Library-Level Stable Backup

### Implementation

- [x] Move stable-copy ownership from CLI `cmd_backup` into a focused
      `tosumu-core` backup module.
- [x] Keep CLI backup as a thin renderer/adapter over the core operation.
- [x] Define input/output types without CLI dependencies.
- [x] Return a structured `BackupReport` containing source, destination,
      optional WAL artifact, and attempts.
- [x] Preserve bounded retry behavior and return structured `Busy` when source
      stability cannot be established.
- [x] Reject existing destination main/WAL paths without partial replacement.
- [x] Clean staged files on all failure paths.
- [x] Document behavior while another handle is open and the exact consistency
      guarantee of the captured pair.

### Validation

```text
cargo test -p tosumu-core backup
cargo test -p tosumu-cli backup
```

### Acceptance Criteria

- [x] An embedded consumer can request a stable backup without shelling out or
      copying Tosumu files itself.
- [x] Success returns a complete committed main/WAL pair and structured report.
- [x] Instability returns structured `FileBusy`; it never returns a silently
      inconsistent pair.
- [x] Opening the backup reproduces the source's committed state.
- [x] CLI behavior remains compatible while delegating storage semantics to core.

## 10. Slice 5: Portable Single-Artifact Export

### Design Gate

- [x] Define export as a stable backup copied to a private staging path,
      checkpointed there, and published without mutating the source.
- [x] Define committed WAL reconciliation as recovery plus WAL truncation on
      the staging copy only.
- [ ] Define destination replacement, fsync, rename, and directory durability
      guarantees for supported filesystems.

The current API refuses an existing destination, publishes the validated
single file with rename, and does not promise directory fsync durability. It
does not require the source handle to be closed; source changes that prevent a
stable pair are reported through the existing structured `FileBusy` error.

### Implementation

- [x] Add a library-level export operation returning a structured
      `PortableExportReport`.
- [x] Reconcile all committed WAL frames into the staging main file.
- [x] Ensure the successful destination requires no WAL sidecar.
- [x] Verify the destination header, pages, overflow chains, and B+ tree before
      publishing the final path.
- [x] Report source instability or unreconciled WAL as structured failure.
- [x] Preserve the source database and sidecar state; export never checkpoints
      the source.
- [x] Test failure cleanup and destination non-replacement.

### Acceptance Criteria

- [x] The exported file can be copied alone to a new directory and opened.
- [x] All committed keys and hashes match the source logical state.
- [x] Verification succeeds with every source-side sidecar hidden or removed.
- [x] Success never requires an undocumented companion file.
- [ ] WAL reconciliation failure is explicit and leaves no published partial
      artifact.

## 11. Slice 6: Embedded Inspection and Verification

### Implementation

- [x] Inventory public core inspection reports versus CLI-only payload shaping.
- [x] Define one stable incubation-level Rust report for header, WAL, page, and
      B+ tree verification observations.
- [x] Keep reportable findings separate from fatal inspection failures.
- [x] Give page and B+ tree findings stable typed categories; descriptions
      remain supplemental.
- [x] Preserve distinctions for corrupt page, corrupt overflow chain,
      unsupported format, wrong key, and incomplete B+ tree checks.
- [x] Preserve a distinct file-busy failure through embedded verification as
      structured `TosumuError::FileBusy` / `FILE_OPEN_BUSY`.
- [ ] Refactor CLI JSON rendering to consume the same core observations where
      practical rather than rebuilding storage meaning.
- [x] Verify both working databases and portable exports through the API.

The embedded verification boundary supports sentinel stores through
`inspect_verification` and passphrase-protected stores through
`inspect_verification_with_passphrase`. Recovery-key and keyfile inspection
remain a follow-up extension of the same opener pattern.

### Acceptance Criteria

- [x] Tokimu can render storage diagnostics from structured core reports.
- [x] No consumer must import `Pager` or scrape CLI/human-readable output.
- [x] Findings and fatal failures are distinct in types and status.
- [x] Portable artifact verification exercises physical format, pages,
      overflow, and B+ tree validity.

## 12. Slice 7: Physical Format vs Application Schema

### Checklist

- [x] Document that Tosumu `format_version` governs physical storage only.
- [x] Document that Tokimu owns `.tasset` schema and adapter codec versions.
- [x] Preserve explicit unsupported physical-version rejection.
- [x] Add no automatic physical migration to open paths.
- [x] Expose validated physical version/header information through the admitted
      inspection boundary.
- [x] Demonstrate transactionally storing and changing application-owned
      schema/version records as ordinary values.
- [x] Add adapter guidance for mapping physical incompatibility separately from
      application-schema incompatibility.

### Acceptance Criteria

- [x] Physical and application schema incompatibilities are distinct failures.
- [x] Tosumu assigns no semantic meaning to `.tasset` schema records.
- [x] A newer unsupported physical format remains a structured open/inspect
      error, not an attempted migration.

## 13. Slice 8: Shared Consumer Fixture and Evidence

### Checklist

- [x] Add a deterministic fixture specification under `docs/CRs/Tokimu/`.
- [x] Record Tosumu crate version, physical format version, fixture schema
      version, keys, payload sizes, and expected hashes.
- [x] Add a core integration test that builds the fixture without Tokimu code.
- [x] Round-trip create, commit, reopen, verify, stable backup, portable export,
      and independent reopen.
- [x] Exercise corruption, wrong-key, newer-format, busy, and identity-isolation
      cases from the CR evidence matrix.
- [ ] Publish machine-readable fixture metadata if both projects need automated
      comparison.
- [x] Update the CR evidence matrix with commands, results, and artifact hashes.

Current Slice 8 evidence is recorded in
`docs/CRs/Tokimu/tokimu-001-fixture.md`. The focused command
`cargo test -p tosumu-core --test provider_boundary
external_consumer_fixture_round_trips_backup_export_and_verification` passed;
the exact value hashes are listed in that fixture document. The 64 MiB reopen,
recovery, and independent measurement probes were also run explicitly; their
commands and results are recorded in the provider baseline.

Additional public-boundary tests now verify newer physical-version rejection,
wrong-key rejection, and identity isolation:
`external_consumer_gets_structured_error_for_newer_physical_format`,
`external_consumer_gets_wrong_key_for_encrypted_store`, and
`external_consumer_keeps_database_identities_isolated`.
`external_consumer_gets_structured_finding_for_corrupt_page` also verifies a
typed page-auth finding through embedded verification. The focused
`inspect_verification_preserves_busy_as_structured_error` test verifies that a
busy WAL is returned as `FILE_OPEN_BUSY`, separate from reportable findings.
`inspect_verification_supports_passphrase_protected_store` verifies encrypted
verification and typed wrong-key rejection.

### Acceptance Criteria

- [x] Tosumu runs the fixture using only core public APIs.
- [ ] Tokimu can reproduce equivalent logical observations through its adapter.
- [x] Matching keys, values, versions, and hashes establish the shared boundary.
- [x] The fixture proves backup and portable export are different guarantees.
- [x] Every CR evidence-matrix row is passed or explicitly deferred with owner
      and rationale.

### Remaining Gate Disposition

| Gate | Owner | Disposition | Follow-up validation |
| --- | --- | --- | --- |
| Tokimu adapter reproduces the fixture | Tokimu consumer project | Deferred until the consumer-side adapter exists; Tosumu has validated the admitted public boundary without importing consumer semantics | Run the equivalent Tokimu adapter fixture and compare keys, schema/version records, payload hashes, and reopen observations |
| Peak allocation and streaming policy | Tosumu + Tokimu | Deferred; elapsed time and logical copy volume are measured for 1, 16, and 64 MiB, but allocator/peak-RSS evidence is not claimed | Repeat the three ignored measurement tests with an agreed peak-allocation/RSS recorder, then record the buffering decision |
| Machine-readable fixture metadata | Tosumu + Tokimu | Optional until both projects require automated comparison; the Markdown fixture remains reproducible and authoritative for incubation | Publish versioned metadata alongside `tokimu-001-fixture.md` and compare it in both projects |
| Unlock-aware embedded verification | Tosumu | Passphrase verification is implemented and tested; recovery-key and keyfile variants remain deferred | Add recovery-key and keyfile overloads and run encrypted corrupt-page/overflow cases for each unlock mode |

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

- [x] The admitted provider API is documented and tested as an external crate.
- [x] A multi-record logical asset commits or rolls back atomically.
- [x] 1 MiB, 16 MiB, and 64 MiB values pass reopen and recovery tests.
- [x] Stable backup is available from `tosumu-core` with a structured report.
- [x] Portable export produces one independently verifiable file.
- [x] Embedded verification returns structured observations and failures.
- [x] Physical format and `.tasset` schema versions remain separate concerns.
- [x] The shared fixture passes the requested evidence matrix.
- [ ] Workspace tests and Clippy pass.
- [ ] Tokimu can implement its adapter without importing Tosumu internals; the
      Tosumu side is validated, but the consumer-side adapter remains deferred
      to the Tokimu project.

## 17. Explicitly Deferred

- SQL-backed asset schemas or queries.
- Generic application schema migration framework.
- Async I/O and streaming blobs without measurement evidence.
- Multi-process writes or MVCC work.
- Network, browser, or WASM storage providers.
- Tokimu-specific storage types, runtime cooking, or inspection panes.
- Automatic physical-format migration.