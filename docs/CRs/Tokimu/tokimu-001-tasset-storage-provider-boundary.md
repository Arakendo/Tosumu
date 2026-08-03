# TOKIMU-001: Tasset Storage Provider Boundary

| Field | Value |
| --- | --- |
| Status | Proposed |
| Requested by | Tokimu |
| Opened | 2026-08-03 |
| Target | `tosumu-core` library boundary and supporting diagnostics/tooling |
| Related Tokimu review | `AR-0011: Tosumu-Backed Tasset Canonical Asset Output` |
| Priority | Incubation blocker for the first Tosumu-backed `.tasset` corpus |

Implementation plan: [`tokimu-001-implementation-plan.md`](tokimu-001-implementation-plan.md)

## Summary

Tokimu wants to use Tosumu as the first durable storage provider for its
canonical editable asset output, currently called `.tasset`.

This is intentionally a real consumer relationship rather than a request for
Tosumu to implement Tokimu asset semantics. Tokimu will own asset identity,
schema, provenance, dependencies, diagnostics, and migration policy. Tosumu
should continue to own storage, transactions, recovery, integrity, physical
format compatibility, and storage inspection.

The initial Tokimu adapter should use Tosumu's smallest stable key/value
surface. It should not depend on the developing SQL layer merely to create
useful corpus pressure.

## Consumer Pipeline

```text
source asset
    ↓
Tokimu importer and canonical asset model
    ↓
Tokimu `.tasset` logical schema
    ↓
Tokimu Tosumu adapter
    ↓
Tosumu key/value, transaction, WAL, and recovery semantics
    ↓
portable or working storage artifact
```

Tokimu needs to be able to reverse this pipeline without exposing Tosumu page,
B+ tree, WAL, encryption, or SQL implementation objects through the Tokimu
public API.

## Ownership Boundary

### Tokimu owns

- `.tasset` logical identity and semantic version;
- canonical asset records, dependencies, provenance, and diagnostics;
- the mapping from asset concepts to provider keys and values;
- application-level schema migration and compatibility policy;
- source import and runtime cooking;
- deciding when a working asset becomes a portable artifact.

### Tosumu owns

- atomic commit and rollback;
- key/value storage and large-value overflow behavior;
- WAL creation, replay, checkpointing, and recovery;
- physical format version validation;
- corruption and integrity detection;
- stable storage-facing diagnostics and error classification;
- safe snapshot, backup, and portable-export mechanics;
- encryption and protector behavior when enabled.

### Tosumu must not need to understand

- Tokimu assets, scenes, materials, meshes, images, or shaders;
- `.tasset` record names or schema versions;
- Tokimu runtime resource handles;
- source formats such as GLB, SVG, CGM, FBX, PNG, JPEG, or BMP.

## Existing Tosumu Evidence

The current Tosumu checkout already provides useful parts of this boundary:

- page-based key/value storage with overflow chains for large values;
- transaction closure commit and rollback;
- WAL-backed crash recovery;
- explicit physical `format_version` validation;
- structured errors and inspection payloads;
- verification and corruption evidence;
- a CLI backup command that retries until it captures stable copies of the
  main database and optional WAL sidecar.

This CR does not ask Tosumu to replace those mechanisms. It asks for the
remaining consumer-facing contracts needed to use them safely from another
Rust project.

## Required Deliverables

### 1. Bounded Embeddable Provider Surface

- [ ] Identify and document the supported Rust library entry point for a
      consumer that needs create/open, get, put, delete, scan, and transaction.
- [ ] Keep the first admitted surface key/value based and independent of SQL.
- [ ] Document handle lifetime, close behavior, write serialization, read-only
      behavior, and thread/process limitations.
- [ ] Return structured Tosumu errors rather than requiring Tokimu to parse
      messages.
- [ ] Add at least one integration test that consumes the boundary as an
      external crate would, without reaching into pager or B+ tree internals.

Acceptance criteria:

- A consumer can implement an adapter without importing physical page, WAL,
  encryption-frame, or CLI types.
- A transaction can write metadata and multiple binary values atomically.
- A failed transaction leaves no partially visible asset state.

### 2. Large Binary Value Contract

- [ ] Document the practical and enforced maximum key and value sizes.
- [ ] Exercise values representative of editable assets, including at least
      1 MiB, 16 MiB, and 64 MiB payloads if current limits permit them.
- [ ] Verify put, get, overwrite, delete, reopen, recovery, and scan behavior
      for overflow-backed values.
- [ ] Record allocation or copy behavior sufficiently for Tokimu to decide
      whether a later streaming API is justified.
- [ ] Return a specific structured error when a value exceeds a real limit.

Acceptance criteria:

- Binary payloads round-trip byte-for-byte after close and reopen.
- Replacing or deleting a large value does not leak reachable records or break
  B+ tree invariants.
- The initial contract may buffer complete values; streaming is not required
  unless measurements demonstrate that buffering blocks realistic assets.

### 3. Library-Level Stable Snapshot / Backup

The current CLI backup logic is useful evidence, but Tokimu cannot safely
depend on a CLI-private function.

- [ ] Expose or factor a library-level stable snapshot/backup operation.
- [ ] Return a structured report describing the main file, WAL presence,
      checkpoint state, and produced artifact paths.
- [ ] Define whether the operation is valid while another Tosumu handle is
      open and what consistency guarantee it provides.
- [ ] Preserve the current bounded retry or locking behavior and report
      `Busy` explicitly when stability cannot be established.
- [ ] Verify that opening the snapshot reproduces the committed source state.

Acceptance criteria:

- Tokimu can request a stable backup without copying files itself.
- The backup is either a complete committed snapshot or a structured failure;
  it is never a silently inconsistent file pair.

### 4. Portable Single-Artifact Export

A working Tosumu database may legitimately use a WAL sidecar. A distributed
`.tasset` should not silently depend on an omitted sidecar.

- [ ] Define a closed or explicitly checkpointed export operation that
      produces one self-contained database file with no required WAL sidecar.
- [ ] The operation must not mutate the source into an ambiguous state.
- [ ] Report whether all committed frames were checkpointed and whether any
      readers or writers prevented export.
- [ ] Verify the exported file independently after temporarily removing or
      hiding every source-side sidecar.
- [ ] Document the filesystem durability assumptions behind successful export.

Acceptance criteria:

- A successful portable export can be copied by itself to another directory,
  opened, verified, and read with identical committed values.
- Failure to reconcile the WAL is explicit; success never means "remember to
  copy another file."

### 5. Recovery and Atomicity Corpus

- [ ] Add a consumer-shaped fixture that writes one logical asset across
      multiple keys and overflow values in one transaction.
- [ ] Crash at representative write sites before and after commit.
- [ ] Reopen and prove the result is exactly the prior state or exactly the
      committed state, never a mixture.
- [ ] Include interrupted overwrite and delete cases.
- [ ] Expose enough structured recovery evidence to distinguish replay,
      discarded uncommitted work, and unrecoverable corruption.

Acceptance criteria:

- The fixture passes the existing crash harness across the selected write
  sites.
- Recovery outcomes are machine-classifiable without parsing log text.

### 6. Inspection and Verification Boundary

- [ ] Document the supported Rust inspection/verification entry points for an
      embedded consumer, or explicitly document that only the CLI JSON
      contract is stable during incubation.
- [ ] Provide structured observations for physical format version, WAL state,
      page/record integrity, overflow integrity, and B+ tree validity.
- [ ] Keep reportable findings separate from failures that prevent inspection,
      consistent with Tosumu's existing error model.
- [ ] Ensure verification can run against a portable exported artifact.

Acceptance criteria:

- Tokimu can present storage diagnostics without interpreting Tosumu internals
  or scraping human-readable output.
- Corrupt page, corrupt overflow chain, unsupported physical format, wrong key,
  and busy-file states remain distinguishable.

### 7. Physical Format and Application Schema Separation

- [ ] Document that Tosumu `format_version` describes physical storage, not
      Tokimu's `.tasset` schema.
- [ ] Preserve explicit rejection for unsupported physical versions.
- [ ] Do not add speculative automatic physical migrations for this CR.
- [ ] Provide enough header/inspection information for Tokimu to explain that
      a file needs a different Tosumu reader or a future explicit rewrite.
- [ ] Confirm that application-owned schema/version records can be stored and
      changed transactionally without Tosumu assigning domain meaning to them.

Acceptance criteria:

- Tokimu can independently version its logical asset schema and adapter codec.
- An incompatible Tosumu physical format and an incompatible `.tasset` schema
  are reported as different failures by the adapter.

### 8. Consumer Integration Fixture

- [ ] Add a small `docs/CRs/Tokimu` fixture or test description that both
      projects can reproduce.
- [ ] Store a manifest, provenance record, dependency table, diagnostic list,
      and one or more binary payloads under application-defined keys.
- [ ] Round-trip the fixture through create, commit, reopen, verify, backup,
      portable export, and corruption tests.
- [ ] Record Tosumu version, physical format version, fixture schema version,
      payload sizes, and expected hashes.

Acceptance criteria:

- Tosumu can run the fixture without depending on Tokimu code.
- Tokimu can run an equivalent fixture through its adapter without depending
  on Tosumu internals.
- Matching logical observations and hashes establish the shared boundary.

## Requested Evidence Matrix

| Case | Tosumu pressure | Expected result |
| --- | --- | --- |
| Small manifest | Ordinary KV | Exact reopen round-trip |
| Multiple related records | Transaction | All committed or none visible |
| 1/16/64 MiB payloads | Overflow pages | Exact hashes after reopen |
| Failed adapter write | Rollback | Prior asset remains intact |
| Crash before commit | WAL recovery | Prior state |
| Crash after commit record | WAL recovery | Committed state after replay |
| Stable backup | Snapshot boundary | Complete main/WAL pair or `Busy` |
| Portable export | Checkpoint boundary | One independently openable file |
| Corrupt page | Integrity | Structured corruption report |
| Corrupt overflow chain | Verification | Structured overflow finding |
| Newer physical version | Compatibility | Explicit unsupported-version error |
| Wrong encryption key | Protector boundary | Explicit wrong-key error |
| Same logical keys in two databases | Identity isolation | No cross-database coupling |

## Deferred Work

The following are valuable but do not block the first Tokimu corpus:

- SQL schema or query support;
- a generic schema-migration framework;
- async I/O;
- multi-process writes;
- network storage;
- browser persistence or WASM filesystem integration;
- zero-copy or streaming blob APIs without measurement evidence;
- automatic runtime resource cooking;
- Tokimu-specific inspection panes in Tosumu tools.

## Explicit Non-Goals

- Making Tosumu the owner of `.tasset` semantics.
- Renaming Tosumu files or magic values to Tokimu concepts.
- Making Tosumu mandatory for every Tokimu build or platform.
- Replacing source/interchange files with `.tasset`.
- Storing live Tokimu world state or runtime GPU resources in the first slice.
- Treating the pre-stability Tosumu physical format as a permanent public file
  format merely because Tokimu begins consuming it.

## Proposed Delivery Order

1. Document and test the embeddable KV boundary.
2. Harden large-value and transactional consumer fixtures.
3. Factor the stable backup operation into a library boundary.
4. Add portable one-file export/checkpoint behavior.
5. Expose structured embedded verification evidence.
6. Run the shared Tokimu-shaped fixture and publish results.

Tokimu can begin its adapter after steps 1 and 2. Tokimu should not describe a
`.tasset` as portable or distributable until step 4 passes.

## Completion Condition

This CR is complete when Tosumu can demonstrate, through a documented
library-level API and repeatable tests, that an external Rust consumer can
atomically store a multi-record asset with large binary values, recover it,
inspect it, back it up, and export it as a verified self-contained artifact
without learning Tosumu's physical internals.

Completion does not stabilize the `.tasset` schema or automatically make
Tosumu Tokimu's permanent storage provider. It supplies the evidence required
for Tokimu AR-0011 to make that later decision honestly.
