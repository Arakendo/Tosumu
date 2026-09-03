# Assurance Claim Inventory v1

| Field | Value |
| --- | --- |
| Authority | Evidence; does not create or strengthen Tosumu guarantees |
| Lifecycle | Current Slice 0 baseline |
| Captured | 2026-09-03 |
| Scope | Principal repository claims across storage, integrity, recovery, inspection, deployment, build, and assurance |
| Owning plan | `docs/Plans/high-assurance-engineering-and-evidence-export.md` |

## Purpose

This inventory begins Assurance Slice 0 by mapping Tosumu's principal current
and future claims to their authority, executable evidence, and unsupported
boundaries. It is intentionally narrower than an exhaustive line-by-line audit
of every public document. A later pass must expand it before Slice 0 can close.

The inventory does not treat a design description, roadmap checkbox, test name,
or successful local command as a stronger guarantee than its owning normative
contract. "Observed" below means retained implementation or executable evidence
exists at the stated boundary. It does not mean audited, exhaustive, or
production qualified.

## Baseline Method

Sources inspected:

- the Software Design, Error Design, and Inspect API specifications;
- `SECURITY.md`;
- accepted ADRs and current Architectural Reviews;
- the main feature and focused implementation plans;
- public documentation and current Cargo/workspace manifests;
- CI, documentation, and scheduled fuzz workflows;
- current core, SQL, CLI, inspection-boundary, and WASM source/tests; and
- the executable test catalog from
  `cargo test --workspace --tests -- --list`.

The test-catalog command completed successfully on Windows and listed 571 tests
across workspace unit and integration targets. It emitted incremental-cache
hard-link fallback warnings caused by the current filesystem; those warnings do
not establish a Tosumu behavior failure. An earlier
`cargo test --workspace --all-targets -- --list` attempt was stopped because
Criterion benchmark targets execute workloads instead of serving as a bounded
test enumeration. Neither command ran the test bodies.

## Disposition Vocabulary

| Disposition | Meaning in this inventory |
| --- | --- |
| Established local baseline | Implemented and covered by current tests/specification at a narrow local boundary |
| Partial | Useful implementation/evidence exists, but an advertised or implied dimension remains missing |
| Platform-limited | Implemented only for named platforms or missing required native execution evidence |
| Incubating | Architectural Review holds the question open; no supported guarantee follows |
| Planned | Sequenced by a plan or roadmap, not implemented evidence |
| Unsupported | Explicitly unavailable or outside the current contract |
| Pre-audit limitation | Implemented mechanism exists, but independent security/assurance review has not occurred |

These are inventory dispositions, not proposed `EvidenceBundle` wire states.

## Principal Claim Inventory

### Storage Identity, Format, And Logical Data

| ID | Principal claim | Authority and executable evidence | Disposition | Important limit or gap |
| --- | --- | --- | --- | --- |
| FMT-001 | Ordinary open recognizes the Tosumu format and refuses unsupported physical versions before mutation | SDD/file-format docs; ADR-0005; format and inspect version-rejection tests | Established local baseline | Format 3 is current but pre-stability; no general migration framework or stable compatibility horizon |
| FMT-002 | Format 3 maintains a durable monotonic committed generation across checkpoint, WAL truncation, recovery, and reopen | ADR-0005; pager/WAL generation tests; MVP+10 baseline tests | Established local baseline | A committed generation is local publication identity, not freshness, replica progress, or authority |
| ID-001 | Tosumu exposes a durable general database identity suitable for receipts, replicas, and restore lineage | AR-0015 and cluster plan | Unsupported | `dek_id` is cryptographic/keyslot binding evidence and must not be silently promoted into the complete database/replica identity contract |
| KV-001 | Public KV callers can atomically create/open, put/get/delete/scan, commit/rollback, and reopen supported stores | ADR-0001; `KvStore`, `SharedKvStore`; provider-boundary and storage tests | Established local baseline | Synchronous embedded API; no remote service or distributed transaction contract |
| KV-002 | Supported values round-trip through overflow storage up to the documented maximum | provider-boundary tests for inline, 1 MiB, 16 MiB, and maximum values | Established local baseline | Whole-value buffering and peak allocation remain open consumer evidence; not a streaming contract |

### Transaction, Durability, Recovery, And Concurrency

| ID | Principal claim | Authority and executable evidence | Disposition | Important limit or gap |
| --- | --- | --- | --- | --- |
| TXN-001 | A supported transaction publishes all staged logical mutations or none | ADR-0005/0006; transaction, rollback, failure, and external-caller tests | Established local baseline | Local single-authority transaction only |
| DUR-001 | WAL commit publication and checkpoint have explicit local ordering and distinguish committed-but-flush-failed outcomes | ADR-0005; pager/WAL crash and flush-failure tests; Error Design codes | Established local baseline | Bound to tested OS/filesystem behavior; does not establish remote-copy or quorum durability |
| REC-001 | Writable reopen applies complete committed WAL work and ignores incomplete work without truncating before successful application | SDD; WAL recovery/crash tests; `fuzz_wal_replay` target | Established local baseline | No independently retained startup recovery receipt; in-process tests do not cover host disappearance |
| REC-002 | Tested write failures leave the prior or complete committed state rather than a mixed transaction | MVP+5 audit; page-store/WAL crash properties and fuzz target | Established tested baseline | Applies to retained injected boundaries, not every hardware/filesystem failure schedule |
| CON-001 | One cooperating writable database path is admitted; a second cooperating writer fails fast | ADR-0004; writer-gate and MVP+10 baseline tests | Established cooperative local baseline | Advisory sidecar does not stop older/arbitrary software and is not distributed fencing |
| CON-002 | `SharedKvStore` snapshots retain a coherent committed generation while one writer advances | ADR-0005/0006; core, SQL, and external shared-store tests | Established process-local baseline | Registry/pins are process-local; read operations remain mutex-serialized; independent read-only handles are live views |
| CON-003 | Snapshot count, transaction WAL, and retained-WAL pressure are bounded and typed | ADR-0005/0006; registry and pager limit tests; public error catalog | Established local baseline | Defaults are experimental and not a distributed backpressure/retention policy |
| CAS-001 | Conditional writes reject stale local owner tokens or mismatched values atomically | ADR-0007; shared-store conditional tests | Established process-local baseline | Generation token is owner-local, unserializable, and invalid after reopen; not a durable authority token |

### Integrity, Encryption, And Key Management

| ID | Principal claim | Authority and executable evidence | Disposition | Important limit or gap |
| --- | --- | --- | --- | --- |
| INT-001 | Supported encrypted page reads authenticate page identity, version, and type before returning plaintext | ADR-0002; crypto known-answer/tamper tests; corrupt-page tests | Pre-audit mechanism with established local tests | Original composition is not independently reviewed; process-memory and side-channel threats remain out of scope |
| INT-002 | Page authentication detects single-page tamper/swap/reorder/rollback within documented AAD rules | SDD/SECURITY; AEAD tests | Pre-audit mechanism with explicit scope | Consistent multi-page rollback remains undetected without external freshness evidence |
| STR-001 | Structured verification reports page findings and B+ tree validity/incompleteness without silently converting partial evidence into success | Inspect specification; `inspect_verification` and CLI contract tests | Established local baseline | Verification does not establish application semantic truth or external freshness |
| ENC-001 | Passphrase-protected databases use Argon2id-derived protection and authenticated DEK wrapping; wrong credentials fail explicitly | SDD; crypto/pager/CLI tests; Error Design | Pre-audit mechanism with established local tests | No production confidentiality claim; keys remain accessible to a compromised running process |
| KEY-001 | Supported protector slots, passphrase, recovery-key, keyfile, add/remove, and rewrap lifecycle preserve database access constraints | MVP+7 audit; protector and provider-boundary tests | Established local baseline | No KMS, network escrow, hardware-rooted general profile, or multi-host key distribution |
| DEL-001 | Deleting Tosumu data or keys guarantees physical secure erasure | SECURITY and assurance plan | Unsupported | Filesystems, flash translation, snapshots, replicas, backups, swap, and crash dumps can retain recoverable material; any future claim must be profile-specific |

### Inspection, Errors, And Evidence Composition

| ID | Principal claim | Authority and executable evidence | Disposition | Important limit or gap |
| --- | --- | --- | --- | --- |
| ERR-001 | Boundary failures have stable codes, small statuses, structured details, and source preservation where applicable | Error Design; core/CLI code-catalog synchronization and mapping tests | Established current boundary baseline | Future service, witness, replication, and assurance failures are not yet public codes |
| INSP-001 | CLI inspect commands use one structured envelope for header, verify, pages, page, WAL, tree, and protector observations | Inspect API; 119 CLI unit tests include focused JSON contract cases | Established current CLI baseline | Current schema does not compose recovery lineage, freshness, authority, backup, durability, or build provenance |
| INSP-002 | Embedded callers can obtain a bounded structured storage observation without importing pager/B+ tree internals | AR-0002; core inspection session and inspection-boundary tests; WASM adapter tests | Partial / incubating boundary | AR-0002 remains Incubating; current byte-input boundary intentionally exposes limited header facts |
| EVD-001 | One evidence object can relate database/build identity, generation, integrity, recovery, freshness, authority, backup, and durability while naming unknown dimensions | Assurance plan only | Planned | No admitted subject model, evidence state enum, schema, canonical serialization, signing, redaction, or independent caller |
| EVD-002 | Individually valid evidence is composed only when its database, generation, epoch, artifact, build, and scope subjects match | Assurance plan candidate rule | Planned | Must be made structural through AR-0002 evidence; adjacency in JSON is not proof of subject equality |

### Backup, Export, Maintenance, And Recovery Copies

| ID | Principal claim | Authority and executable evidence | Disposition | Important limit or gap |
| --- | --- | --- | --- | --- |
| BAK-001 | Stable backup publishes a matching main/WAL pair or returns a bounded failure without a knowingly mixed destination | MVP+4; `backup` tests and provider-boundary open-handle test | Established library baseline | No scheduler, remote object publication, retention, catalog, signed receipt, or restore SLA |
| EXP-001 | Portable export stages, checkpoints, verifies, and publishes a reopenable single-file artifact without source mutation | `export` tests and provider fixture | Established library baseline | No artifact-lineage identity or long-term compatibility promise |
| VAC-001 | Offline VACUUM preserves logical records, protectors, generation continuity, verification, and old-or-new publication on admitted platforms | ADR-0009; vacuum failure matrix and rebuild tests; stable macOS arm64 CI job `100836236880` at `abdc241` | Platform-limited | Native macOS confirms the Unix implementation path; this does not qualify Linux distributions, filesystems, storage providers, or power-loss behavior; Windows refuses before mutation |
| RST-001 | An operator can select and restore the correct externally retained backup for a failed deployment | Cluster and assurance plans | Planned | No backup catalog, freshness binding, restore orchestration, RPO/RTO, or retained restore drill |

### SQL And Consumer Boundaries

| ID | Principal claim | Authority and executable evidence | Disposition | Important limit or gap |
| --- | --- | --- | --- | --- |
| SQL-001 | The separate SQL crate supports its admitted create/insert/point-select/point-delete/prepared subset over public storage behavior | MVP+9 plan/audit; SQL unit and integration tests | Established bounded baseline | Not a general SQL engine; unsupported query shapes reject explicitly |
| SQL-002 | Plain single-column SQL-owned secondary indexes backfill and mutate atomically with primary rows | ADR-0008; SQL index, failure, and reopen tests | Established bounded baseline | Composite/covering/bitmap and broader planner work remain future milestones |
| TOK-001 | A provider-neutral external consumer can exercise atomic storage, backup/export, verification, identity isolation, and large-value behavior | TOKIMU-001 fixture and 23 provider-boundary tests | Established Tosumu-side consumer evidence | Real Tokimu adapter and peak-allocation/streaming decision remain outside completed core/provider evidence |

### Freshness, Service, Replication, And Cluster Operation

| ID | Principal claim | Authority and executable evidence | Disposition | Important limit or gap |
| --- | --- | --- | --- | --- |
| FRH-001 | Tosumu can prove the currently opened state is the newest valid state | SECURITY; AR-0005 | Unsupported / freshness unanchored | No witness or observer protocol exists; authenticated older state can remain valid |
| SVC-001 | One provider-neutral authority contract preserves lifecycle, errors, authorization, cancellation, and storage semantics across embedded/local/remote hosts | AR-0003 | Incubating | No local IPC, daemon, or server implementation provides parity evidence |
| K3S-001 | Tosumu supports pod restart, node-loss recovery, and verified restore on a named K3s topology | Cluster plan MVP+12a | Planned | No manifests, service host, storage-provider profile, or measured RPO/RTO evidence |
| WIT-001 | Independent witnesses/observers provide signed freshness evidence and rollback disagreement | AR-0005; MVP+12b plan | Incubating / planned | Receipt format, trust roots, outage/quorum policy, clocks, rotation, and corpus are missing |
| REP-001 | Tosumu maintains an asynchronous warm standby with gap-free bootstrap/catch-up and bounded visible lag | AR-0015; MVP+15 | Planned; architecture not admitted | No database/replica identity, replication representation, position, apply, retention, or reseed contract |
| AUT-001 | Tosumu automatically transfers authority without permitting a stale primary to publish | AR-0015; MVP+16 | Planned; architecture not admitted | No authority epoch, positive fencing, election, promotion, rejoin, or partition evidence |
| QRM-001 | A quorum-class acknowledged write survives the failure set promised by a named quorum model | cluster plan MVP+17 | Conditional future target | No retained zero-RPO consumer requirement, consensus decision, implementation, dependency review, or model evidence |
| MWR-001 | Tosumu supports active-active multi-writer operation | cluster plan/AR-0015 | Unsupported and not authorized by current plan | Requires separate semantic/conflict/consensus architecture |

### Build, Supply Chain, Platform, And Review

| ID | Principal claim | Authority and executable evidence | Disposition | Important limit or gap |
| --- | --- | --- | --- | --- |
| DEP-001 | The complete supported dependency closure has retained source identity, features, licenses, unsafe/build-script review, target behavior, and update ownership | AR-0010 | Partial / incubating | `fs4` has focused retained review; no complete risk-tiered repository closure exists |
| MSRV-001 | Rust 1.75 is the supported minimum toolchain for the workspace | workspace manifest; focused `fs4` review | Declared; incomplete general evidence | Main CI runs stable Linux/Windows/macOS and beta Linux, not an explicit 1.75 workspace job |
| CI-001 | Normal code changes run format, strict Clippy, tests, and docs across supported primary desktop OS targets | `.github/workflows/ci.yml` and `docs.yml` | Partial automation baseline | Code CI ignores docs-only changes; docs workflow does not run Rust tests; hosted runner success is not platform qualification |
| FUZ-001 | Eight fuzz targets exist for page decoding, B+ tree behavior/crash, WAL replay, AEAD frames, keyslots, and TQL parsing/rendering | `fuzz/Cargo.toml` and target sources | Partial harness baseline | Scheduled CI currently runs only TQL parser and renderer targets; this inventory did not establish recent retained runs for the other six |
| BLD-001 | Release artifacts are reproducible and traceable to declared source, toolchain, dependency, feature, and target inputs | assurance plan A2 | Unsupported | No pinned release toolchain workflow, SBOM, signed provenance, independent builder comparison, or release signing policy |
| PLT-001 | A named OS/filesystem/storage/host profile is qualified for stated long-duration and failure behavior | assurance plan A4 | Unsupported | Unit/CI platform coverage is not qualification; no profile manifest or sustained fault campaign exists |
| AUD-001 | Tosumu's cryptography, storage, service, or replication design has independent assurance review | `SECURITY.md`; assurance plan A6 | Unsupported / pre-audit | Current tests and internal reviews do not substitute for independent review |
| VUL-001 | Tosumu has a supported vulnerability intake and response SLA/backport policy | `SECURITY.md` | Partial disclosure path | Private reporting path exists, but there is no SLA, CVE pipeline, or backport branch policy |
| MOB-001 | iOS/Android wrappers, hardware-backed protectors, and lifecycle behavior are supported | MVP+11 | Planned | No FFI, wrappers, device fixtures, packaging, or platform-specific protector implementation |

## Initial Ambiguity And Drift Findings

The first pass found these areas requiring later Slice 0 reconciliation:

1. **Database identity versus `dek_id`.** Current inspection exposes `dek_id`,
   but cluster/evidence work needs a deliberate database, replica, restored-copy,
   artifact, and build identity model. Cryptographic binding identity is not
   automatically the complete operational identity.
2. **Tested durability versus qualified durability.** WAL and crash evidence is
   strong at retained injected boundaries, but public language must remain tied
   to tested OS/filesystem contracts and must not imply remote or quorum
   durability.
3. **Current fuzz inventory versus scheduled execution.** Eight targets exist;
   only the two TQL targets appear in the scheduled fuzz workflow.
4. **Declared MSRV versus CI.** Rust 1.75 is declared, while the primary matrix
   tests moving stable and beta toolchains. A focused dependency check is not a
   general workspace MSRV gate.
5. **CI coverage versus platform qualification.** Linux, Windows, and macOS CI
   provide valuable compatibility evidence but do not define filesystem,
   hardware, duration, or fault profiles.
6. **Backup existence versus recoverability.** Stable backup/export mechanisms
   exist; cataloging, offsite retention, generation/freshness binding, restore
   drills, and RPO/RTO do not.
7. **Signed evidence versus trustworthy proposition.** Future witness, backup,
   build, and authority signatures require separate subjects, purposes, trust
   roots, and dependency provenance.
8. **High-assurance positioning versus current posture.** The evidence-oriented
   niche is a useful design target, but the present supported statement remains
   pre-audit and experimental.

## Representative Operator Assurance Questionnaire

This questionnaire is the first candidate consumer pressure for later claim and
profile review. Current answers should be evidence references, explicit
limitations, or `unknown`; prose confidence is not an answer.

### Workload And Recovery

1. What data and application semantics are stored, and which of them must remain
   consumer-owned?
2. What are the required and preferred RPO and RTO for process, node, disk,
   volume, site, and operator-error failures?
3. Must writes continue while disconnected, during control-plane loss, or when
   data quorum/freshness evidence is unavailable?
4. Is manual promotion acceptable? If so, who may approve fencing and select
   the recovery position?
5. Which backup retention, immutability, restore-frequency, and geographic
   separation requirements apply?

### Platform And Failure Domain

6. Which CPU architecture, OS version, filesystem, storage device, hypervisor,
   container runtime, Kubernetes/K3s version, CSI provider, and network topology
   form the exact profile?
7. Which components share power, node, rack, storage-controller, credential,
   administrator, and network failure domains?
8. Which crash, pause, partition, disk-full, corruption, clock, upgrade, and
   restore cases must be demonstrated, and for how long?
9. Which offline periods, bandwidth ceilings, latency distributions, and
   intermittent-connectivity patterns are representative?

### Threat, Authority, And Keys

10. Which actors may read files, alter files, inspect state, mutate data,
    administer hosts, witness freshness, restore backups, promote replicas, or
    sign releases?
11. Are compromised process memory, OS/kernel, hypervisor, storage firmware,
    build host, witness, replica, or operator credentials in scope?
12. Who creates, delivers, rotates, revokes, backs up, escrows, and destroys
    data/protector/signing keys?
13. Must replicas share data-encryption keys, and what compromise radius is
    acceptable?
14. What does "deletion" mean for live files, flash media, snapshots, replicas,
    backups, logs, crash dumps, and evidence artifacts?

### Evidence And Supply Chain

15. Which database identity, generation, integrity, recovery, freshness,
    authority, backup, durability, build, and limitation facts must be exported?
16. Who consumes the evidence, what authorization do they have, and which paths,
    topology, identities, or timing data require redaction?
17. How long must evidence remain verifiable, and which signing-key rotation,
    revocation, and historical-validation rules apply?
18. Is an SBOM required? Which source provenance, offline build, reproducibility,
    compiler, dependency, license, advisory, and vulnerability-response evidence
    is mandatory?
19. Which independent review, certification, procurement, or regulatory regime
    applies to the exact deployment rather than the market category?
20. What explicit unsupported or unknown states must cause write refusal,
    readiness failure, operator approval, or deployment rejection?

## Slice 0 Status

Completed in this baseline:

- [x] principal claims grouped across the current storage and future assurance
      surface;
- [x] current authority and executable evidence identified at a bounded level;
- [x] major unsupported and ambiguous boundaries recorded;
- [x] initial public-language drift candidates identified; and
- [x] one representative operator assurance questionnaire retained.

Still required before Slice 0 closes:

- [ ] exhaustive public/normative claim extraction with stable claim IDs;
- [ ] machine-checkable links from each accepted claim to code/tests/artifacts;
- [ ] review of every existing public page for unqualified assurance language;
- [ ] AR-0007 review of whether A0-A6 and the inventory gate improve decisions
      without becoming ceremonial; and
- [ ] one real operator or consumer response to the questionnaire.

## References

- `docs/Plans/high-assurance-engineering-and-evidence-export.md`
- `docs/Plans/cluster-fault-tolerance-and-replication.md`
- `docs/Plans/main-feature-roadmap.md`
- `docs/Specifications/Tosumu Software Design Document.md`
- `docs/Specifications/Tosumu Error Design Document.md`
- `docs/Specifications/Tosumu Inspect API Specification.md`
- `SECURITY.md`
- `docs/Architectural Reviews/AR-0002-structured-inspection-contract-boundary.md`
- `docs/Architectural Reviews/AR-0005-witness-observer-and-freshness.md`
- `docs/Architectural Reviews/AR-0007-core-change-evidence-and-resilience.md`
- `docs/Architectural Reviews/AR-0008-operation-outcome-closure-and-crash-evidence.md`
- `docs/Architectural Reviews/AR-0010-dependency-trust-and-source-provenance.md`
- `docs/Architectural Reviews/AR-0015-native-replication-scope-authority-and-failure-model.md`
- `.github/workflows/ci.yml`
- `.github/workflows/docs.yml`
- `.github/workflows/fuzz.yml`
- `fuzz/Cargo.toml`
