# Main Feature Roadmap

| Field | Value |
| --- | --- |
| Status | Active |
| Opened | 2026-08-03 |
| Last updated | 2026-09-03 (cluster, assurance, and crypto-agility tracks opened) |
| Owner | Tosumu maintainers |
| Authority | Tracking plan; `docs/Specifications/Tosumu Software Design Document.md` remains normative |
| Current milestone | MVP+11 private C harness and unsafe-boundary admission review |

## Purpose

This is the canonical implementation-status checklist for Tosumu's main feature
set. It answers what is complete, what remains within an existing milestone,
and what should be planned next.

`docs/Specifications/Tosumu Software Design Document.md` owns feature meaning, architecture, and the detailed MVP/stage
roadmap. This file tracks delivery. `docs/roadmap.md` remains the shorter public
summary. Feature-specific plans retain their detailed evidence and acceptance
criteria.

A checked item means executable behavior and retained evidence exist. It does
not strengthen security, durability, compatibility, or performance guarantees
beyond the normative specifications.

## Current Direction

- [x] Establish the storage, integrity, encryption, key-management, inspection,
      TUI, and initial SQL foundations through MVP+9.
- [x] Expose a provider-neutral embedded KV boundary, stable backup, portable
      export, and structured embedded verification for independent consumers.
- [x] Close remaining MVP+9 audit and logical-scan decisions explicitly.
- [x] Open a focused MVP+10 plan before implementing MVCC, conditional writes,
      secondary indexes, or `VACUUM`.
- [x] Open a long-term cluster fault-tolerance and replication plan with
      separate recovery, freshness, standby, failover, and consensus gates.
- [x] Open a cross-cutting high-assurance engineering and evidence-export plan
      spanning provenance, inspection, qualification, keys, and review.
- [x] Open a cross-cutting cryptographic provider and suite-agility plan that
      separates a format-v3-preserving seam from future authenticated suite
      identity, migration, and deployment-profile evidence.
- [ ] Reconcile the normative distributed-storage non-goal through
      [AR-0015](../Architectural%20Reviews/AR-0015-native-replication-scope-authority-and-failure-model.md)
      before implementing native replication.

## MVP Delivery And Acceptance Checklist

The statuses below are the starting claims for the planned audit. A checked
historical item means the repository currently treats that capability as
delivered. The audit may uncheck it if code, tests, documentation, or retained
validation do not support the claim.

### MVP 0: It Stores Bytes

**Build**

- [x] Append-only key/value log with replay into an in-memory index.
- [x] Runnable `put`, `get`, and `scan` CLI path.
- [x] Clean-close synchronization and reopen behavior.
- [x] Round-trip, reopen, and empty-store tests.

**Acceptance Criteria**

- [x] A user can write and retrieve bytes through the binary.
- [x] Data survives a clean close and reopen.
- [x] Empty storage opens without panic or fabricated records.
- [x] This historical increment is allowed to be superseded by the real page
      format; completion does not require retaining the append-log engine.

### MVP+1: It Has A Real Format

**Build**

- [x] Versioned file header with Tosumu magic and reader compatibility fields.
- [x] Fixed-size pages with slotted leaf layout and bounded record decoding.
- [x] Page allocation and freelist-backed page reuse.
- [x] KV CLI for init, put, get, scan, stat, and delete.
- [x] Page/record codec round-trip and malformed-input tests.

**Acceptance Criteria**

- [x] A newly initialized file reopens deterministically.
- [x] Header and page bounds are validated before higher layers consume data.
- [x] Insert, read, scan, delete, and page reuse preserve logical KV state.
- [x] Unsupported physical format versions fail explicitly.

### MVP+2: It Is Inspectable

**Build**

- [x] Human-readable header/page dump command.
- [x] Raw page hex/ASCII inspection command.
- [x] Whole-file page verification with non-success policy for findings.
- [x] Basic KV `get --explain` cost counters.
- [x] Arbitrary-page decoder fuzz target or equivalent retained fuzz harness.

**Acceptance Criteria**

- [x] Operators can inspect headers and individual pages without custom code.
- [x] Corrupt or unauthentic pages are reported rather than silently accepted.
- [x] Point reads can explain basic I/O/search work.
- [x] Arbitrary page bytes do not cause an unexpected panic in the decoder.

### MVP+3: It Scales Past Linear Scan

**Build**

- [x] B+ tree leaf/internal pages, routing, root growth, and node splitting.
- [x] Sorted key iteration and bounded key-range scans.
- [x] Overflow chains for values that do not fit inline.
- [x] Lazy delete behavior with explicit space-reclamation limitations.
- [x] Random-operation property tests, invariant checks, and B+ tree fuzzing.

**Acceptance Criteria**

- [x] Point lookup follows tree routing rather than a full physical scan.
- [x] Sorted and range scans return the same logical ordering as a reference
      ordered map.
- [x] Splits preserve every committed key/value and valid leaf traversal.
- [x] Overflow-backed values survive insert, overwrite, delete, split, and
      reopen within documented size limits.
- [x] Tree height and structural invariants remain bounded under tested random
      operation sequences.

### MVP+4: It Survives A Crash

**Build**

- [x] Atomic transaction API with commit and rollback.
- [x] Physical/full-page WAL with begin, page-write, and commit records.
- [x] Recovery on writable open with committed replay and uncommitted discard.
- [x] Retry-on-lock behavior with structured busy failure after a bounded limit.
- [x] Library-level stable backup of the main/WAL pair.

**Acceptance Criteria**

- [x] A successful multi-key transaction is fully visible after reopen.
- [x] A failed transaction exposes none of its partial writes.
- [x] Recovery applies committed WAL work and ignores incomplete transactions.
- [x] Recovery never truncates the WAL unless application succeeded.
- [x] Stable backup yields a reopenable committed pair or an explicit failure;
      it does not publish a knowingly mixed snapshot.

### MVP+5: It Cannot Be Lied To

**Build**

- [x] Reusable phase-based crash writer/fault-injection harness.
- [x] B+ tree structural invariant checker.
- [x] WAL append/checkpoint crash tests at meaningful write boundaries.
- [x] Property tests comparing random operations with a reference model.
- [x] Crash-boundary fuzz target that reopens and verifies recovered state.
- [x] Verification path that includes B+ tree structure after page integrity.

**Acceptance Criteria**

- [x] Tested write failures leave either the prior state or the committed new
      state, never a mixed transaction.
- [x] Recovered trees pass structural invariants after tested crash boundaries.
- [x] Integrity, I/O, busy, and unsupported states remain distinguishable.
- [x] Fault-injection failures do not get mislabeled as corruption.

### MVP+6: It Is Encrypted

**Build**

- [x] Per-page AEAD with page identity/version/type bound as AAD.
- [x] Random database DEK and domain-separated derived keys.
- [x] Argon2id passphrase protector and authenticated DEK wrapping.
- [x] Header MAC covering protector/keyslot metadata.
- [x] Known-answer tests for AEAD, KDF/derivation, wrapping, and header MAC.
- [x] Crypto-frame and keyslot parser fuzz targets.

**Acceptance Criteria**

- [x] Correct credentials reopen and read encrypted data.
- [x] Wrong credentials return a structured wrong-key failure.
- [x] Authenticated page corruption returns an integrity/authentication failure.
- [x] Plaintext user values are not present in encrypted page frames.
- [x] Security claims remain limited to the threat model in `SECURITY.md`.

### MVP+7: Key Management Works

**Build**

- [x] Up to eight independently usable protector slots.
- [x] Recovery-key and keyfile protector flows.
- [x] Protector add, remove, and list CLI operations.
- [x] KEK rotation that rewraps the DEK without rewriting data pages.
- [x] Protector-slot binding and cross-database/swap attack tests.
- [x] Recovery and alternate-protector lifecycle tests.

**Acceptance Criteria**

- [x] Any active valid protector can unlock the same database identity.
- [x] Removed or obsolete credentials no longer unlock the database.
- [x] At least one valid protector must remain after supported mutations.
- [x] Protector metadata tampering or cross-database splicing is rejected.
- [x] Key-management failure paths preserve existing valid access when the
      mutation does not commit.

### MVP+8: It Is Interactively Inspectable

**Build**

- [x] Cross-platform read-only `tosumu view` TUI.
- [x] Header, page-list, and page-detail views.
- [x] B+ tree, WAL, protector, and verification views.
- [x] Keyboard navigation, scrolling, color/state indicators, and watch mode.
- [x] Encrypted-database unlock flow without making the TUI own crypto meaning.

**Acceptance Criteria**

- [x] The viewer opens ordinary and encrypted databases through core contracts.
- [x] The viewer performs no storage mutation.
- [x] Corrupt/auth-failed pages and incomplete tree trust are visibly distinct.
- [x] TUI rendering consumes structured observations rather than parsing CLI
      prose or reaching around the storage boundary.

### MVP+9: It Speaks SQL

The retained evidence is in [Initial SQL Layer](initial-sql-layer.md).

**Build**

- [x] Separate `tosumu-sql` crate with no dependency on CLI/TUI layers.
- [x] Lexer, parser, AST, semantic checker, planner, and executor pipeline.
- [x] Namespace-backed catalog and typed row codecs over the KV boundary.
- [x] `CREATE TABLE`, `INSERT`, point `SELECT`, and point `DELETE`.
- [x] Prepared statements and structurally bound parameters.
- [x] Explicit projections, constrained predicates, and primary-key OR
      multi-point operations.
- [x] `tosumu sql` execution and `--explain` output.
- [x] Typed rejection for unsupported syntax, semantics, and query shapes.
- [x] Stable logical full-table scans are explicitly outside MVP+9 and require
      a dedicated post-MVP+10 SQL plan built on admitted reader visibility.
- [x] `tosumu audit` and structured audit findings are explicitly outside
      MVP+9 and require a separate future diagnostics/audit plan.

**Acceptance Criteria**

- [x] The SQL crate depends downward on public storage behavior, not Pager or
      CLI/TUI internals.
- [x] Supported statements validate before mutation and execute end to end.
- [x] Prepared values are AST leaves and are never reparsed as SQL grammar.
- [x] Unsupported shapes fail explicitly instead of silently becoming physical
      scans.
- [x] Focused SQL/CLI tests, workspace tests, and strict workspace Clippy are
      recorded by the retained plan.
- [x] Audit and logical-scan scope is moved to named follow-on planning gates;
      the initial SQL plan and SDD already identify both as future work.

**Status:** MVP+9 baseline complete. Logical scans move to a dedicated
post-MVP+10 SQL plan; audit moves to a separate future diagnostics/audit plan.

### MVP+10: Multiple Readers

**Build**

- [x] Dedicated plan with an executable baseline of current lock/read behavior.
- [x] Accepted format-v3 generation, retained-WAL, limit, checkpoint, and
      compatibility architecture in ADR-0005.
- [x] Read transactions pinned to a stable committed-generation snapshot,
      exposed through the ADR-0006 `SharedKvStore` contract.
- [x] Single-writer/multiple-reader coordination without readers observing
      partial commits, including one atomic multi-mutation write closure.
- [x] Version-observing reads and conditional-write helpers (`put_if_absent`
      and compare-and-set/version operations).
- [x] Plain single-column SQL-owned ordered secondary indexes (ADR-0008).
- [x] `VACUUM` with explicit reclamation, interruption, and publication rules.
- [x] Representative concurrency and SQLite comparison benchmarks.

**Acceptance Criteria**

- [x] Concurrent readers each observe a coherent snapshot.
- [x] A writer can commit without invalidating or partially changing an active
      reader's snapshot.
- [x] Conditional writes reject stale preconditions atomically.
- [x] Secondary-index mutation is atomic with primary-row mutation and remains
      correct through recovery.
- [x] `VACUUM` preserves all committed logical rows and does not replace the
      source with an incomplete artifact.
- [x] Concurrency limits and unsupported multi-writer behavior are explicit.

### MVP+11: It Runs On Mobile

**Build**

- [x] Open
      [AR-0017](../Architectural%20Reviews/AR-0017-mobile-embedding-abi-and-hardware-protector-boundary.md)
      to reconcile ABI ownership, artifact-set lifecycle, target qualification,
      fail-closed protector policy, and crypto Gate C3 before implementation.
- [x] Complete AR-0017's contract and target inventories and open a dedicated
      sliced MVP+11 plan before stabilizing an ABI or adding binding tooling.
- [x] Reconcile SDD section 19 and retain an experimental callback-free result,
      buffer, error, and handle-state contract without reserving ABI symbols.
- [ ] C ABI/FFI crate with opaque handles and explicit ownership rules.
- [ ] Stable cross-language error and byte-buffer contracts.
- [ ] Swift/iOS wrapper and Keychain/Secure Enclave protector integration.
- [ ] Kotlin/Android wrapper and Keystore protector integration.
- [ ] Mobile packaging, lifecycle, cancellation, and resource-bound guidance.
- [ ] Device/emulator integration fixtures for encrypted databases.

**Acceptance Criteria**

- [ ] Swift and Kotlin callers can create, open, transact, close, and inspect a
      database without importing Rust internals.
- [ ] FFI calls do not unwind across the ABI or leak owned buffers/handles.
- [ ] Hardware-backed protector failures remain distinguishable from page
      corruption and ordinary wrong credentials.
- [ ] App suspend/resume and process restart preserve committed state.
- [ ] iOS and Android device-level encrypted round trips pass.

### MVP+12: It Runs With Recovery Evidence, Witnesses, And Observers

The staged work and stronger-level exclusions are retained in
[Cluster Fault Tolerance And Replication](cluster-fault-tolerance-and-replication.md).

**Build**

- [ ] Close the first service-authority review cycle with one bounded host
      experiment before stabilizing a remote contract.
- [ ] **MVP+12a:** one writable K3s host with an exclusive PVC, bounded probes,
      verified offsite backup publication, and a rehearsed cold restore.
- [ ] Record pod-restart, node-loss, volume-loss, and restore RPO/RTO evidence
      for each named storage topology.
- [ ] **MVP+12b:** Architectural Review closure for witness, observer, and
      freshness ownership before stabilizing their contracts.
- [ ] Transport-neutral signed receipt and observer contracts above core.
- [ ] `tosumu-server`, witness quorum service, and local observer process.
- [ ] Reproducible K3s manifests or chart that keep witnesses in independent
      failure domains and never share one writable database file across pods.
- [ ] Rollback/freshness disagreement injection and operational diagnostics.

**Acceptance Criteria**

- [ ] Pod replacement and operator-driven restore produce a verified database
      or a structured unavailable outcome; storage-provider replication is not
      mislabeled as Tosumu replication.
- [ ] Node-loss support is claimed only for an explicitly tested CSI or block-
      storage provider; local-path remains a single-node development profile.
- [ ] Restoring a stale database snapshot produces a structured rollback or
      freshness warning backed by witness evidence.
- [ ] Witnesses audit identity/freshness and do not become database replicas or
      a hidden multi-writer consensus system.
- [ ] Observer/server communication failure is explicit and bounded.
- [ ] Readiness reflects unhealthy trust state without claiming automatic
      failover.
- [ ] Core storage remains independent of Kubernetes and transport concerns.

### MVP+13: Entropy Bookkeeping

**Build**

- [ ] **MVP+13a:** structural metrics for freelist ratio, fragmentation, leaf
      fill, tombstones, height excess, and overflow ratio.
- [ ] **MVP+13b:** operational counters/timestamps through one explicit format
      revision.
- [ ] **MVP+13c:** protector/KDF age, recovery-key consumption, startup crypto
      KAT, and nonce-ceiling bookkeeping.
- [ ] Additive structured `inspect.audit` entropy payload and findings.
- [ ] Document thresholds, update rules, overflow behavior, and reset events.

**Acceptance Criteria**

- [ ] Structural metrics are reproducible from validated storage observations.
- [ ] Header bookkeeping survives recovery and cannot silently wrap.
- [ ] Verification/recovery/rekey events update only their documented fields.
- [ ] Nonce usage warns at the documented threshold and refuses before the
      safety ceiling.
- [ ] Audit reports observations and recommendations but performs no automatic
      vacuum, rekey, or repair.
- [ ] Format compatibility and migration behavior are explicit and tested.

### MVP+14: Secondary Structures For Expensive Queries

**Build**

- [ ] **MVP+14a:** page Bloom filters and planner rewrite for indexed `IN`
      predicates.
- [ ] **MVP+14b:** per-page zone maps for range skipping.
- [ ] **MVP+14c:** composite indexes and optional covering columns.
- [ ] **MVP+14d:** explicitly gated low-cardinality bitmap indexes with stable
      row identity.
- [ ] Planner diagnostics and `inspect.audit` effectiveness observations for
      every retained structure.
- [ ] Explicit refusal of hash, trie, inverted/full-text, fuzzy, vector, and
      spatial indexes unless the normative scope changes.

**Acceptance Criteria**

- [ ] Every structure has deterministic maintenance and crash-recovery tests.
- [ ] Planner selection is semantics-preserving and visible through explain
      output.
- [ ] Bloom-filter false positives affect performance only, never correctness.
- [ ] Zone maps never skip a page that can satisfy the predicate.
- [ ] Composite/covering indexes preserve key order and index-only results.
- [ ] Bitmap indexes enforce the configured cardinality gate and remain atomic
      with row mutation.
- [ ] Benchmarks demonstrate the intended query-pattern benefit and report
      storage/write amplification without converting observations into
      unsupported guarantees.

### MVP+15: It Maintains An Asynchronous Warm Standby

This milestone covers Levels 3 and 4 of the cluster plan. Native replication
does not begin until its architecture is accepted.

**Build**

- [ ] Reconcile the SDD distributed-storage non-goal and admit or park a
      single-leader replication architecture through an ADR.
- [ ] Decide byte-identical standby replication versus normalized committed-
      effect replication without exposing the recovery WAL as public meaning.
- [ ] Define database, replica, transaction, replication-position, and
      authority-epoch identities.
- [ ] Verified snapshot bootstrap, gap-free incremental catch-up, durable
      received/applied watermarks, and explicit reseeding.
- [ ] Bounded async stream with duplicate, reorder, gap, corruption, lag,
      retention, and wrong-identity handling.
- [ ] Passive standby that rejects writes and exposes structured replication
      health and promotion eligibility.
- [ ] Manual promotion requiring positive external fencing evidence.

**Acceptance Criteria**

- [ ] Snapshot plus increments reproduces the leader's committed logical state
      without publishing a partial transaction.
- [ ] Replication gaps and retained-history overruns never skip silently.
- [ ] Lag and the potential promotion data-loss window are measurable and
      visible.
- [ ] Manual promotion cannot proceed without fencing evidence and a selected
      recovery position.
- [ ] Physical WAL offsets, page numbers, and checkpoint truncation do not enter
      the stable replication contract.
- [ ] Format, key/protector, compatibility, and security consequences are
      accepted and tested before support is claimed.

### MVP+16: It Transfers Authority Automatically

**Build**

- [ ] Monotonic authority epochs and stale-primary write rejection.
- [ ] Positive fencing independent of pod reachability or process-liveness
      guesses.
- [ ] Automated replica eligibility, promotion, service routing, and readiness.
- [ ] Explicit demotion, failback, rejoin, and divergent-history behavior.
- [ ] Multi-process and multi-node partition, pause, crash, stale-PVC, rolling-
      upgrade, and simultaneous-restart fault corpus.
- [ ] Retained manual recovery path for every case automation cannot prove safe.

**Acceptance Criteria**

- [ ] At most one eligible authority acknowledges writes in every supported
      fault schedule.
- [ ] A partitioned or delayed former leader cannot publish under a superseded
      epoch.
- [ ] Failover occurs only after fencing and only to an eligible replica.
- [ ] Ambiguous authority remains unavailable with structured diagnostics.
- [ ] The async replication lag remains an explicit possible data-loss window;
      automatic failover does not imply zero RPO.

### MVP+17: It Survives Acknowledged Writes Through Quorum

This milestone begins only when retained consumer evidence shows that the
MVP+16 bounded-RPO profile is insufficient and the project explicitly accepts
distributed-state-machine responsibilities.

**Build**

- [ ] Accepted replicated-log or consensus design and fully reviewed dependency
      closure.
- [ ] Synchronous durability class bound to a durable data quorum and authority
      epoch.
- [ ] Membership changes, learner/bootstrap, removal, quorum-loss, and
      divergent-rejoin semantics.
- [ ] Deterministic protocol simulation or model checking for election,
      replication, commit, fencing, and membership invariants.
- [ ] Black-box multi-node network, process, disk, and control-plane fault
      campaign.
- [ ] Mixed-version and rolling-upgrade compatibility matrix.

**Acceptance Criteria**

- [ ] A quorum-class acknowledged commit survives every failure set promised by
      the named quorum model.
- [ ] A minority cannot acknowledge writes or advance committed authority.
- [ ] Membership changes cannot create two valid write quorums.
- [ ] Rejoining nodes verify identity and history before serving or voting.
- [ ] Local, async, one-remote-copy, and quorum durability outcomes remain
      distinguishable at every public boundary.
- [ ] Fencing remains required; consensus does not silently replace the MVP+16
      authority-transfer rules.

## Cross-Cutting Cryptographic Provider And Suite-Agility Track

[Cryptographic Provider Seam And Suite Agility](cryptographic-provider-seam-and-suite-agility.md)
tracks provider substitution and durable suite evolution across milestones.
It does not change format v3 or establish a FIPS or other compliance claim.

**Build**

- [x] Open the gated plan and record current algorithm, key-lifecycle, format,
      and pager coupling.
- [x] Accept the private C1 boundary through
      [ADR-0010](../ADR/ADR-0010-private-format-v3-cryptographic-mechanism-seams.md)
      after AR-0016 and its conservation baseline.
- [ ] Insert a private backend seam that preserves exact format-v3 bytes,
      errors, public APIs, and authenticated-pager behavior.
- [ ] Demonstrate provider independence before stabilizing a public SPI.
- [ ] Admit opaque provider-owned key handles before public HSM/KMS/TPM
      integration makes raw-key export an accidental requirement.
- [ ] Introduce authenticated suite identity only through an accepted format
      revision with downgrade and compatibility fixtures.
- [ ] Implement suite conversion only as verified full rewrite and atomic
      publication.
- [ ] Admit a named validated deployment profile only through the assurance
      track with provider/module/configuration evidence.

**Acceptance Criteria**

- [ ] Process configuration selects creation policy but never reinterprets
      existing ciphertext.
- [ ] Suite identity and provider implementation identity remain separate.
- [ ] Unknown, unavailable, forbidden, and retired suites/providers fail
      explicitly without fallback or trial decryption.
- [ ] The pager remains the sole authenticated plaintext/ciphertext boundary.
- [ ] Raw key export is optional rather than required by the provider SPI.
- [ ] Backup, WAL, retained generations, inspection, replication, and evidence
      preserve suite identity without inventing provider guarantees.
- [ ] No generic compliance label is inferred from algorithms, dependencies,
      feature flags, or provider presence.

## Cross-Cutting Long-Term Assurance Track

The feature milestones are necessary but insufficient for a high-assurance
deployment claim. [High-Assurance Engineering And Evidence Export](high-assurance-engineering-and-evidence-export.md)
tracks evidence maturity across every milestone rather than pretending it is a
feature added after MVP+17.

**Build**

- [x] Open the assurance-level and evidence-export plan without changing the
      repository's current pre-audit security posture.
- [x] Publish the bounded v1 inventory of principal integrity, durability,
      recovery, freshness, authority, provenance, platform, and unsupported
      claims without presenting it as complete coverage.
- [ ] Expand the inventory across every public claim and close remaining
      unassessed assurance-critical dependency/source boundaries.
- [ ] Advance AR-0010 into a risk-tiered dependency/source provenance policy
      backed by generated closure inventories.
- [ ] Produce pinned, SBOM-described, checksummed, and provenance-attested
      release artifacts; test independent-build reproducibility.
- [ ] Admit a bounded machine-readable evidence bundle only after AR-0002
      reviews composition, redaction, and compatibility.
- [ ] Qualify named operating-system, filesystem, storage, host, and cluster
      profiles with long-duration, fault, restore, and upgrade evidence.
- [ ] Define privilege, secret, key/protector, revocation, backup, replica, and
      secure-deletion limitations per profile.
- [ ] Obtain independent cryptographic, storage, protocol, and operational
      review before graduating any assurance claim.

**Acceptance Criteria**

- [ ] Every supported claim names its owner, evidence, profile, version, and
      unsupported boundary.
- [ ] An authorized operator can obtain bounded evidence for implemented
      identity, generation, integrity, recovery, freshness, authority, backup,
      durability, and build-provenance dimensions.
- [ ] Missing, stale, incomplete, unconfigured, unsupported, and unverifiable
      evidence never defaults to `ok`.
- [ ] Build provenance, test success, external freshness, and independent review
      remain separate claims.
- [ ] Assurance language applies only to the named reviewed deployment profile;
      it does not become a generic certification or defense-suitability claim.
- [ ] `SECURITY.md`, the specifications, profile manifests, and public guidance
      agree on the strongest supported posture.

## Criterion-Level Audit

This audit was performed against the implementation and retained evidence in
the repository on 2026-08-03. `PASS` means the criterion has an owning
implementation and executable or retained evidence. `OPEN` means the
criterion is intentionally not claimed. A passing historical criterion is not
a claim that untested inputs, unsupported limits, or future design scope are
covered.

### MVP 0 Audit

| Criterion | Disposition | Evidence |
| --- | --- | --- |
| Write and retrieve bytes through the binary | PASS | `tosumu-cli` store command tests and `log_store` tests |
| Survive clean close and reopen | PASS | `log_store` round-trip/reopen tests |
| Empty storage opens without fabricated records | PASS | empty-store tests in `log_store` |
| Historical append-log increment may be superseded | PASS | superseded by `page_store`; no compatibility claim retained |

### MVP+1 Audit

| Criterion | Disposition | Evidence |
| --- | --- | --- |
| Newly initialized file reopens deterministically | PASS | `page_store` initialization and reopen tests |
| Header and page bounds validate before consumption | PASS | `format`, page codec, and malformed-input tests |
| Insert/read/scan/delete/reuse preserve logical KV state | PASS | `page_store` operation and reuse tests |
| Unsupported physical versions fail explicitly | PASS | format/version rejection tests and typed errors |

### MVP+2 Audit

| Criterion | Disposition | Evidence |
| --- | --- | --- |
| Operators can inspect headers and individual pages | PASS | `tosumu-cli` inspect commands and CLI contract tests |
| Corrupt or unauthentic pages are reported | PASS | inspect verification tests and structured issue payloads |
| Point reads explain basic I/O/search work | PASS | CLI explain path and page-store cost counters |
| Arbitrary page bytes do not panic the decoder | PASS | `fuzz_page_decode` target and bounded page decoding tests |

### MVP+3 Audit

| Criterion | Disposition | Evidence |
| --- | --- | --- |
| Point lookup follows tree routing | PASS | B+ tree lookup tests and page-store routing implementation |
| Sorted/range scans match ordered logical ordering | PASS | B+ tree iteration/range tests and property coverage |
| Splits preserve committed values and leaf traversal | PASS | split, root-growth, and invariant tests |
| Overflow values survive lifecycle operations and reopen | PASS | overflow insert/overwrite/delete/reopen tests |
| Random sequences preserve bounded height and invariants | PASS | differential property tests, invariant checks, and `fuzz_btree_operations` |

### MVP+4 Audit

| Criterion | Disposition | Evidence |
| --- | --- | --- |
| Successful multi-key transaction is visible after reopen | PASS | transaction commit/reopen tests in `page_store` and `wal` |
| Failed transaction exposes no partial writes | PASS | rollback and failed-transaction tests |
| Recovery replays committed and discards incomplete WAL work | PASS | WAL recovery tests and `fuzz_wal_replay` |
| Recovery never truncates WAL before application succeeds | PASS | recovery failure-path tests |
| Stable backup publishes a committed, reopenable pair or fails | PASS | `backup` tests, including open-handle and pair validation |

### MVP+5 Audit

| Criterion | Disposition | Evidence |
| --- | --- | --- |
| Tested failures leave prior or committed state, never mixed state | PASS | crash-boundary property tests and `fuzz_btree_crash_boundaries` |
| Recovered trees pass structural invariants | PASS | B+ tree invariant checks after recovery |
| Integrity, I/O, busy, and unsupported states stay distinguishable | PASS | typed error codes and CLI/core error tests |
| Fault-injection failures are not mislabeled corruption | PASS | fault-injection error classification tests |

### MVP+6 Audit

| Criterion | Disposition | Evidence |
| --- | --- | --- |
| Correct credentials reopen and read encrypted data | PASS | crypto/page-store encrypted round-trip tests |
| Wrong credentials return structured wrong-key failure | PASS | crypto and CLI unlock/error contract tests |
| Authenticated corruption returns integrity/authentication failure | PASS | page authentication and inspect corruption tests |
| Plaintext values are absent from encrypted page frames | PASS | encrypted-frame assertions in crypto tests |
| Security claims remain within `SECURITY.md` threat model | PASS | crypto implementation and security specification reviewed together |

### MVP+7 Audit

| Criterion | Disposition | Evidence |
| --- | --- | --- |
| Any active valid protector unlocks the same identity | PASS | protector lifecycle and alternate-protector tests |
| Removed or obsolete credentials no longer unlock | PASS | remove/rotation lifecycle tests |
| At least one valid protector remains | PASS | protector mutation validation tests |
| Metadata tampering or cross-database splicing is rejected | PASS | slot-binding and swap-attack tests |
| Failed key-management mutation preserves valid access | PASS | rollback/failure-path protector tests |

### MVP+8 Audit

| Criterion | Disposition | Evidence |
| --- | --- | --- |
| Viewer opens ordinary and encrypted databases through core contracts | PASS | `view` unlock flow and view tests |
| Viewer performs no storage mutation | PASS | read-only `view` path; no write/transaction API is exposed |
| Corrupt/auth-failed pages and incomplete tree trust are distinct | PASS | structured inspect state and CLI/view rendering tests |
| TUI consumes structured observations rather than CLI prose | PASS | `inspect_contract`, `view`, and render/state modules |

### MVP+9 Audit

| Criterion | Disposition | Evidence |
| --- | --- | --- |
| SQL depends downward on public storage behavior | PASS | `tosumu-sql` dependency boundary and provider API usage |
| Supported statements validate before mutation and execute end to end | PASS | SQL integration tests in `tosumu-sql` and CLI SQL tests |
| Prepared values remain AST leaves and are not reparsed | PASS | lexer/parser parameter tests and prepared execution tests |
| Unsupported shapes fail explicitly instead of scanning | PASS | semantic/planner rejection tests |
| Focused SQL/CLI/workspace tests and strict Clippy are recorded | PASS | `initial-sql-layer.md`; workspace tests pass. Clippy command must be rerun with valid arguments |
| Audit and logical-scan scope is implemented or moved to a reconciled plan | DEFERRED / RESOLVED | Initial SQL §15.3 and Phase 8: logical scans follow MVP+10 visibility; audit remains a separate future diagnostics milestone |

### MVP+10 Audit

ADR-0006, ADR-0007, ADR-0008, and their supported core and SQL-layer callers prove coherent pinned
point/range reads, writer commits that preserve active snapshots, finite
registration/retained-WAL limits, diagnostics, last-reader recovery, and atomic
conditional writes. Plain single-column secondary indexes now have atomic
backfill and mutation, snapshot lookup, failure rollback, and reopen evidence.
Offline `VACUUM` now has retained writer admission, protector/generation
preservation, verified staging, explicit space and platform refusal, and a full
injected failure matrix. Benchmark closure records reader fanout,
reader/writer overlap, and the retained single-thread SQLite comparison; the
observations show coherent but mutex-serialized Tosumu reads. The stable macOS
arm64 job in CI run `33812169906` passed formatting, strict Clippy, the complete
workspace all-target test suite (including Unix-only VACUUM publication), and
documentation at commit `abdc241`. This closes the native Unix gate without
claiming that the still-running Linux job has passed.

### MVP+11 Audit

All five criteria are `OPEN / NOT STARTED`: Swift/Kotlin lifecycle access;
FFI unwind and ownership safety; distinguishable hardware-protector failures;
suspend/resume and restart durability; and device-level encrypted round trips.
No implementation or executable evidence was found.

### MVP+12 Audit

All seven criteria are `OPEN / NOT STARTED`: verified pod replacement and cold
restore; provider-scoped node-loss evidence; witness-backed rollback/freshness
warnings; witness role boundaries; bounded observer/server failures; readiness
for unhealthy trust state without automatic failover; and core independence
from Kubernetes/transport. No implementation or executable evidence was found.

### MVP+13 Audit

All six criteria are `OPEN / NOT STARTED`: reproducible structural metrics;
non-wrapping header bookkeeping; event-specific updates; nonce warning and
refusal thresholds; observation-only audit behavior; and explicit tested format
compatibility/migration. No implementation or executable evidence was found.

### MVP+14 Audit

All six criteria are `OPEN / NOT STARTED`: deterministic maintenance and
recovery; semantics-preserving explain-visible planning; Bloom-filter safety;
zone-map safety; composite/covering index correctness; bitmap cardinality and
atomicity; and query-benefit/amplification benchmarks. No implementation or
executable evidence was found.

### MVP+15 Audit

All six criteria are `OPEN / NOT STARTED`: accepted replication architecture;
gap-free bootstrap and catch-up; atomic replica apply; bounded and visible lag;
manual promotion with fencing evidence; and explicit format, protector,
compatibility, and security behavior. Format-3 generations, backup, export, and
verification are foundations, not replication evidence.

### MVP+16 Audit

All five criteria are `OPEN / NOT STARTED`: single eligible authority under
fault injection; stale-primary rejection; fenced eligible promotion; explicit
unavailability under ambiguity; and honest non-zero-RPO reporting for async
failover. No election, fencing, authority-epoch, or promotion implementation
exists.

### MVP+17 Audit

All six criteria are `OPEN / NOT STARTED`: evidence requiring synchronous
quorum; acknowledged-write survival; minority write rejection; safe membership
changes; verified replica rejoin; and distinct durability classes. No consensus
or replicated-state-machine implementation exists.

### Long-Term Assurance Track Audit

The planning document is present; all capability criteria are `OPEN / NOT
STARTED`: repository-wide claim inventory; generated critical dependency
closure; reproducible and attested artifacts; admitted bounded evidence export;
qualified platform profiles; reviewed privilege/key/destruction boundaries;
and independent profile review. Existing tests and AR-0010's focused `fs4`
review are useful inputs but do not establish an assurance level beyond the
current experimental baseline.

## Cross-Cutting Delivered Work

These capabilities were delivered outside the original linear MVP checklist and
must remain visible when planning later milestones.

- [x] Public `KvStore` / transaction provider boundary with external-crate tests.
- [x] Values up to the documented 64 MiB limit through overflow storage.
- [x] Library-level stable backup with structured report and open-handle test.
- [x] Portable single-file export with staged page/B+ tree verification.
- [x] Embedded structured verification for header, WAL, pages, and B+ tree.
- [x] Physical-format and consumer-schema version separation.
- [x] Deterministic Tokimu-shaped fixture with exact hashes and independent
      reopen/backup/export evidence.
- [x] Distinct embedded overflow-chain finding category.
- [x] Unlock-aware embedded verification for sentinel, passphrase, recovery-key,
      and keyfile-protected stores.
- [ ] Complete the remaining TOKIMU-001 consumer-adapter and streaming-policy
      rows; the current evidence matrix and deferred-row ownership are complete.

### Tokimu CR Completion Gates

The Tokimu CR pack is a consumer-boundary evidence track, not a new storage
layer. The following rows make its remaining completion conditions explicit
without moving Tokimu-specific schema or runtime meaning into `tosumu-core`.

- [x] Public provider boundary is exercised from an external-crate test without
      importing Pager, B+ tree, WAL, crypto-frame, SQL, or CLI types.
- [x] Deterministic fixture proves atomic commit, reopen, exact hashes, stable
      backup, portable export, and structured verification.
- [x] 1 MiB and 16 MiB overflow values have focused byte-for-byte reopen
      evidence; the 64 MiB maximum-value test is retained separately.
- [x] 1 MiB, 16 MiB, and 64 MiB overflow values have focused recovery evidence,
      not only reopen evidence.
- [ ] Repeatable copy-volume and timing evidence now exists for 1 MiB, 16 MiB,
      and 64 MiB overwrites; peak-allocation evidence and the decision whether
      whole-value buffering is acceptable or streaming is required remain open.
- [x] Newer-format, wrong-key, identity-isolation, and corrupt-page boundary
      cases have focused structured-error tests.
- [x] Stable-backup instability returns bounded structured `FILE_OPEN_BUSY`
      without publishing a destination pair or leaving staging files.
- [x] Busy classification is exposed through the stable embedded verification
      boundary as structured `FILE_OPEN_BUSY`, separate from reportable
      findings.
- [x] Overflow-chain corruption and portable-artifact verification are exposed
      or exercised through the stable embedded report.
- [ ] A real Tokimu adapter reproduces equivalent logical observations while
      importing only the admitted provider contract.
- [x] The CR evidence matrix is complete, with every deferred row naming an
      owner, rationale, and follow-on validation command.

See `docs/CRs/Tokimu/` for the request, detailed implementation plan, provider
baseline, and fixture evidence.

## Audit Evidence Ledger

Use this table during the roadmap audit. A milestone remains checked only when
its implementation, acceptance criteria, and retained evidence agree. Record
partial or stale milestones directly rather than forcing a pass/fail result.

| MVP | Audit disposition | Evidence or command | Follow-up owner/plan |
| --- | --- | --- | --- |
| 0 | Verified | `log_store` and CLI tests; criterion audit above | Maintain historical evidence |
| +1 | Verified | `page_store`/format tests; criterion audit above | Maintain format evidence |
| +2 | Verified | Inspect CLI contracts, verification, and `fuzz_page_decode` | Maintain inspection evidence |
| +3 | Verified | B+ tree properties/invariants and fuzz targets | Maintain tree evidence |
| +4 | Verified | WAL/recovery/transaction/backup tests | Maintain recovery evidence |
| +5 | Verified | Crash-boundary properties, invariants, and fault tests | Maintain crash evidence |
| +6 | Verified | Crypto tests, unlock tests, and `SECURITY.md` review | Maintain crypto evidence |
| +7 | Verified | Protector lifecycle and attack tests | Maintain key-management evidence |
| +8 | Verified | TUI/view tests and structured inspect contracts | Maintain viewer evidence |
| +9 | Verified baseline; deferred scope named | `initial-sql-layer.md`; audit and logical scans explicitly moved out | Post-MVP+10 SQL and future audit plans |
| +10 | Complete | ADR-0005 through ADR-0009; focused MVP+10 plans; shared KV, SQL, VACUUM, Criterion, and native macOS arm64 CI evidence | Maintain contracts and platform evidence |
| +11 | Slice 0 complete; implementation unadmitted | AR-0017, foreign contract/target inventory, experimental ABI contract, and dedicated mobile plan | Review the private C harness, unsafe boundary, build inputs, and conservation corpus |
| +12 | Not started | Architectural Review required | Unassigned |
| +13 | Not started | Future dedicated plan and format decision | Unassigned |
| +14 | Not started | Future dedicated plan and benchmarks | Unassigned |
| +15 | Planned; architecture not admitted | Cluster fault-tolerance and replication plan | Replication AR, then MVP+15 slices |
| +16 | Planned; depends on +15 | Cluster fault-tolerance and replication plan | Fencing and automatic-transfer slices |
| +17 | Conditional long-term target | Consumer near-zero-RPO evidence required | Consensus admission and quorum slices |
| Assurance | Proposed cross-cutting track; bounded inventory and generated closure baseline retained | High-assurance engineering and evidence-export plan | AR-0010 policy closure and named release profiles |
| Crypto agility | Proposed; architecture not admitted | Cryptographic provider seam and suite-agility plan | Gate C0 review and exact format-v3 conservation baseline |

## Next Planning Gate

Before implementation moves beyond the completed MVP+9 baseline:

- [x] Resolve whether audit/logical scans close MVP+9 or move into a separate
      focused plan.
- [x] Create `docs/Plans/mvp-10-multiple-readers.md` from `TEMPLATE.md`.
- [x] Record current locking, LSN visibility, and reader/writer behavior with a
      focused executable baseline.
- [x] Open an Architectural Review for multiple-reader visibility, coordination,
      and execution ownership. See [AR-0009](../Architectural%20Reviews/AR-0009-multiple-reader-execution-and-coordination.md).
- [x] Update the reviews and create ADR-0005 for the accepted format-v3
      ownership, generation, retained-WAL, and compatibility boundary.
- [x] Admit and implement the ADR-0006 shared KV owner, read transaction,
      atomic write callback, and bounded diagnostics contract.
- [x] Keep secondary indexes subordinate to the MVCC/storage plan rather than
      teaching `tosumu-core` SQL semantics.
- [x] Create the cluster fault-tolerance and replication plan with a claim
      ladder from pod restart through synchronous acknowledged-write survival.
- [x] Open the replication/failover Architectural Review. Continue
      [AR-0015](../Architectural%20Reviews/AR-0015-native-replication-scope-authority-and-failure-model.md)
      until it accepts or parks the SDD scope change before native replication
      implementation.
- [ ] Define the initial K3s failure-domain, RPO/RTO, storage-provider, and
      fencing hypotheses before stabilizing deployment claims.
- [ ] Complete the public assurance-claim inventory and remaining critical
      dependency/source review under AR-0010; do not confuse the retained
      generated closure baseline with completed assessment.
- [x] Create the cryptographic provider and suite-agility plan without changing
      format v3 or making a provider/compliance claim.
- [x] Open Gate C0 [AR-0016](../Architectural%20Reviews/AR-0016-cryptographic-provider-seam-and-suite-identity.md)
      and inventory current crypto operations, entropy calls, key lifetimes,
      and error behavior.
- [x] Add exact deterministic and fixed-nonce vectors plus the named file-level
      conservation matrix before implementing the private seam.
- [x] Implement the ADR-0010 private format-v3 and entropy facades without
      changing format bytes, public APIs, errors, or supported behavior.
- [x] Complete Gate C2 with a versioned deterministic format-v3 corpus and an
      independently implemented offline oracle; keep it outside release
      artifacts and admit its toolchain and dependency closure through AR-0010.

## Completion Rules

- Check an item only when implementation and executable evidence exist.
- Link detailed evidence from the owning plan instead of duplicating it here.
- Update this tracker whenever a feature plan is opened, completed, parked, or
  superseded.
- Update `docs/roadmap.md` when public Now/Next/Later priorities change.
- Update `docs/Specifications/Tosumu Software Design Document.md` or an ADR when feature meaning or architecture changes.
