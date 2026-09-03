# Cluster Fault Tolerance And Replication

| Field | Value |
| --- | --- |
| Status | Proposed; architectural admission required |
| Opened | 2026-09-03 |
| Last updated | 2026-09-03 |
| Owner | Tosumu maintainers |
| Target | MVP+12 and long-term MVP+15 through MVP+17 cluster track |
| Related ADRs | ADR-0001, ADR-0004, ADR-0005, ADR-0006, ADR-0007 |
| Related reviews | AR-0003, AR-0004, AR-0005, AR-0006, AR-0007, AR-0008, AR-0015 |
| Related CRs | None yet |
| Depends on | Format-3 recovery and snapshots, stable backup/export, a reviewed service authority, explicit freshness evidence, a supported K3s storage topology, and the cross-cutting assurance/evidence track |

## Status

Tosumu currently provides local crash recovery, one cooperative writer,
process-local committed-generation snapshots, stable backup, portable export,
and structured verification. It does not provide a server authority, replicated
data, leader election, fencing, replica promotion, or automatic failover.

This plan records a long-term direction toward the harder end state: one active
leader, replicated data, fenced automatic authority transfer, and, if evidence
requires near-zero RPO, quorum-defined durability on K3s. That direction is not
yet accepted architecture. The normative
design currently both excludes distributed/replicated storage and describes a
future semantic-change stream. Slice 0 therefore reconciles the scope through
Architectural Review and, if admitted, an ADR and SDD revision before native
replication implementation begins.

## Purpose

Tosumu should be able to improve availability in deliberate increments without
turning local WAL details, Kubernetes scheduling, or a shared filesystem into
an accidental distributed protocol. The near-term value is recoverability from
pod, node, and volume failure. The long-term target is a credible replicated
single-leader system whose acknowledged durability, promotion, and partition
behavior are explicit and testable.

This plan separates the claims that are often collapsed into "high
availability":

1. local recovery after a process crash;
2. replacement of a failed process or node while retaining one storage copy;
3. restoration from an independently retained copy;
4. detection that a locally valid copy is stale;
5. continuous maintenance of one or more replica copies;
6. safe authority transfer without two writable leaders; and
7. survival of acknowledged writes through synchronous replica quorum.

Each level must earn its own claim. A Kubernetes pod restart is not data
replication. A witness is not a replica. A replica is not automatically safe to
promote. Leader election is not storage fencing. A successful local `fsync`
does not prove that another failure domain has accepted the commit.

The claim ladder is therefore:

```text
pod restart
    -> node-loss recovery
    -> verified restore
    -> stale-state detection
    -> replicated standby
    -> bounded-RPO promotion
    -> automatic failover
    -> synchronous acknowledged-write survival
```

## Trigger And Evidence

- The current K3s milestone deploys witnesses and observers but explicitly does
  not provide replicas, consensus, or automatic failover.
- Format 3 provides durable monotonic committed generations and stable
  snapshots, which are useful replication anchors but are not a replication
  protocol.
- Stable backup and portable export provide safe bootstrap candidates but no
  scheduling, remote retention, restore orchestration, or incremental catch-up.
- The persistent writer sidecar excludes cooperating local writers but is not a
  distributed authority lease and cannot fence an isolated prior leader.
- AR-0003 has not yet admitted a service authority, remote host, authentication
  policy, cancellation contract, or multi-database lifecycle.
- AR-0004 requires physical WAL and semantic change history to remain distinct;
  no durable change-stream contract exists.
- AR-0005 keeps freshness unanchored until a tested witness or observer protocol
  exists.
- `SECURITY.md` still excludes consistent multi-page rollback protection,
  remote attestation, network key escrow, and KMS integration.
- K3s local-path persistent volumes remain node-local. A multi-node fault-
  tolerance claim therefore requires a reviewed CSI/block-storage provider or
  Tosumu-owned replica copies in separate failure domains.

## Current State And Gap Summary

| Capability | Current evidence | Missing before support can be claimed |
| --- | --- | --- |
| Process crash on one disk | WAL recovery, crash fixtures, reopen verification | Retain and extend operational restart evidence |
| Pod restart on the same PVC | Storage mechanisms are compatible with one owner reopening | Host lifecycle, probes, manifests, termination and startup bounds |
| Node failure with externally replicated storage | No retained deployment evidence | Storage-class contract, attach/fence behavior, node-loss test, restore runbook |
| Volume loss and cold restore | Stable backup and portable export exist | Scheduling, remote publication, retention, identity metadata, restore verification |
| Freshness and rollback detection | Design and AR only | Signed receipts, trust roots, quorum/outage policy, stale-volume corpus |
| Async warm standby | None | Replication identity, ordered stream, snapshot install, catch-up, idempotent apply, lag bounds |
| Manual promotion | None | Authority epochs, fencing proof, recovery-point selection, operator workflow |
| Automatic failover | None | Safe election, fencing, promotion state machine, split-brain and partition evidence |
| Synchronous quorum durability | None | Acknowledgement contract, replicated commit index, quorum-loss behavior |
| Active-active multi-writer operation | Not an intended target | Conflict/consensus model would require a separate architecture and plan |

## Guarantee Levels

The levels are cumulative claims, not product editions. A level may be useful
and releasable without beginning the next one.

### Level 0: Local Durable Authority

One writable process owns one database on one local storage authority. WAL,
recovery, snapshots, backup, and verification behave according to the current
format-3 contracts.

**Current disposition:** substantially implemented. Native Unix VACUUM CI and
ongoing crash/recovery evidence remain part of current maintenance.

**Claim:** committed state can recover on supported tested local storage after
the tested process and I/O failure boundaries.

**Does not claim:** survival of disk or node loss, external freshness,
replication, or failover.

### Level 1: Single-Authority Operational Fault Tolerance

One writable Tosumu host runs in K3s with an exclusively mounted PVC. The
storage provider may replicate blocks underneath Tosumu, but Tosumu still owns
exactly one active database authority. Verified backups are published to a
separate failure domain and exercised through restore drills.

**Target claim:** a failed pod can restart against retained storage, and a lost
volume can be restored through a bounded, operator-visible procedure.

**Required distinctions:**

- local-path storage supports single-node development only;
- a multi-node claim names and tests the actual CSI/block-storage provider;
- storage-provider replication is not advertised as Tosumu replication;
- RPO and RTO are measured for each retained topology;
- backup success includes reopen and verification, not merely object upload.

### Level 2: Externally Anchored Freshness

The MVP+12 observer and witness system stores signed evidence of database
identity, committed generation, manifest or state hash, audit head, and
observation time outside the primary failure domain.

**Target claim:** a valid-but-stale restore or rollback can be distinguished
from the latest independently observed state, within the documented witness
availability and trust assumptions.

**Does not claim:** witnesses can serve database data, that a newer restorable
copy exists, or that automatic failover is safe.

### Level 3: Replication Architecture Admission

The project reconciles the current distributed-storage non-goal and decides
whether the first high-availability replica should use authenticated committed-
generation effects, a dedicated physical replication representation, or
another bounded mechanism.

AR-0004 does not pre-decide this choice. It establishes that the recovery WAL
must not become an application synchronization protocol and that offline or
multi-device semantic synchronization needs semantic history. A byte-identical
passive standby under one authority is a different problem. Format-3 committed
generations and retained authenticated versions provide new evidence that the
replication review must evaluate without exposing WAL layout as public meaning.

**Target claim:** none at runtime. This is an admission level. It produces an
accepted architecture, explicit parked decision, or bounded prototype agenda.

**Does not claim:** replicated data, reduced RPO, promotion, or failover.

### Level 4: Asynchronous Warm Replica

One leader publishes an ordered durable replication stream. A standby begins
from a verified snapshot, applies complete committed units idempotently, and
reports its applied and durable watermarks. Promotion is manual and requires an
independent fencing decision.

**Target claim:** the standby can be promoted after declared evidence checks,
with potential data loss bounded by measured replication lag.

**Does not claim:** zero-RPO failover, automatic promotion, stale-follower
reads, or continued writes during loss of authority evidence.

### Level 5: Fenced Automatic Authority Transfer

A controller or service cohort elects and promotes a replacement leader only
after the previous authority is fenced. Every mutation is bound to a current
authority epoch so a delayed or partitioned former leader cannot publish state.
The first automatic-transfer design may still use asynchronous replication and
therefore retain a measured non-zero RPO.

**Target claim:** the supported partition and failure model preserves one
writable authority and automatically restores service when fencing succeeds
and an eligible replica exists.

**Does not claim:** survival of every acknowledged write, multi-leader writes,
or correctness outside the explicitly tested storage, network, and clock
assumptions.

### Level 6: Synchronous Failover And Consensus

One leader considers a configured durability class satisfied only after the
required replica quorum has durably accepted the committed unit. The protocol
has a replicated commit index or equivalent authority and explicit behavior
when quorum is unavailable.

**Target claim:** an acknowledgement carrying a quorum durability class
survives loss of any failure set allowed by the documented quorum model.

**Does not claim:** active-active writers or availability when quorum is lost.

This level begins only when retained consumer evidence requires zero or near-
zero RPO for acknowledged writes and accepts the operational cost of a
distributed state machine. Consensus must not enter merely because automatic
failover exists; fenced promotion of an asynchronous standby is independently
useful and independently risky.

## Decisions To Preserve Now

These constraints already follow from accepted Tosumu boundaries or are safety
conditions that must hold while the larger architecture remains under review:

- `tosumu-core` remains independent of Kubernetes, transport, membership, and
  deployment-provider types.
- The database file is never made multi-writer by placing it on NFS or another
  shared network filesystem.
- The physical recovery WAL does not become a public sync or replication API.
- Witnesses and observers remain freshness evidence holders, not hidden data
  replicas or election voters, unless a future ADR deliberately changes their
  role and threat model.
- Backup remains an independent recovery layer even after replication exists;
  replicas can faithfully replicate corruption, deletion, or operator error.
- The hardest planned topology remains single-leader. Active-active
  multi-writer replication requires a separate review and is not authorized by
  this plan.
- Ambiguous authority fails closed for mutation. No component guesses that a
  lease holder is dead, deletes a lock, or promotes solely because a pod is not
  reachable.
- Every write acknowledgement names its durability class: local, asynchronously
  queued, one remote durable copy, or quorum durable. No stronger class is
  inferred from a successful local commit.
- `CommittedGeneration`, `ReplicationPosition`, and `AuthorityEpoch` remain
  distinct domains. They may have explicit checked relationships, but none is
  an alias for another merely because all three are monotonic values.
- RPO, RTO, replication lag, and failover time remain measured observations
  until a reviewed contract establishes a supported bound.

## Architectural Decisions Required

During Level 3 and before Level 4 implementation, a new replication-focused Architectural Review
must reconcile and either preserve, narrow, or supersede the SDD statement that
distributed/replicated storage will never be added. That review must also
revisit AR-0003, AR-0004, and AR-0005 with executable evidence.

The review must decide at least:

1. whether replica transport belongs to `tosumu-service`, a new provider-
   neutral replication crate, or a host-owned adapter;
2. whether the first stream contains normalized KV transaction effects, a
   dedicated physical replica record, or another representation;
3. how a replication sequence relates to database committed generation without
   exporting WAL offsets or page numbers as public meaning;
4. how snapshot bootstrap and incremental catch-up form one gap-free history;
5. how database identity, replica identity, authority epoch, and restored-copy
   identity differ;
6. whether replicas share encryption material or maintain independently
   protected logical copies;
7. which component durably records acknowledgement and promotion evidence;
8. which consistency model applies to reads from leaders and replicas;
9. how retention pressure, lagging replicas, and forced reseeding are bounded;
10. whether a stable format revision is required and what migration policy
    protects existing databases; and
11. which consensus or coordination dependency, if any, is admitted after
    source, build-script, license, MSRV, and platform review.

If the retained evidence admits native replication, promote the stable result
to an ADR and update the SDD, Error Design Document, Inspect API Specification,
and `SECURITY.md` before claiming Level 4 support.

## Ownership And Dependency Boundary

```text
clients and operators
        |
host API, authentication, authorization, and rate limits
        |
single database authority / replication coordinator
        |---------------- witness and observer evidence
        |---------------- backup and restore publisher
        |---------------- transport-neutral replica messages
        |
tosumu-core public KV, transaction, snapshot, verify, and recovery contracts
        |
pager, B+ tree, private WAL, authenticated pages, local files
```

### `tosumu-core` Owns

- Atomic local storage transactions and committed-generation meaning.
- Local WAL durability, recovery, checkpointing, and authenticated page access.
- Provider-neutral snapshot, backup, export, verification, and bounded
  diagnostics needed by an upper replication layer.
- Any minimal new atomic hook proven necessary to bind one committed unit to
  replication evidence, without learning network or cluster policy.

### Authority Or Replication Layer Owns

- Database and replica identities above path identity.
- Leader lifecycle, authority epochs, admission, and write serialization.
- Replication message ordering, acknowledgement classes, lag, retention, and
  reseed policy.
- Snapshot installation and promotion eligibility.
- Mapping cluster outcomes into stable service errors and inspect observations.

### Host And Deployment Layers Own

- HTTP, gRPC, local IPC, TLS, authentication, authorization, and untrusted-input
  bounds.
- K3s resources, Services, StatefulSets, probes, disruption budgets, topology,
  secrets, PVC selection, and provider-specific recovery procedures.
- Process supervision and external observation of termination.

### Witness And Observer Layers Own

- Signed freshness evidence, trust roots, gaps, disagreement, and audit-head
  comparison.
- No database mutation, data serving, or promotion authority in the initial
  design.

### Consumer Layers Own

- Table, row, asset, and application mutation meaning.
- Conflict policy for offline or multi-writer synchronization.
- Business-level recovery priorities and acceptable RPO/RTO.

## Public Contract Impact

Levels 1 and 2 should add deployment and service contracts without changing
authenticated database bytes. Levels 4 through 6 may require new durable
identity, replication, acknowledgement, or authority-epoch state. Any such
format change requires AR-0006 reopening, an explicit format decision, upgrade
fixtures, and refusal behavior for unsupported versions.

Likely public concepts, all provisional until admitted, include:

- `DatabaseIdentity`, `ReplicaIdentity`, and `AuthorityEpoch`;
- `ReplicationPosition` distinct from physical WAL record LSN;
- `DurabilityClass` and `CommitReceipt`;
- `ReplicaRole`, `ReplicaHealth`, `ReplicationLag`, and `PromotionEligibility`;
- snapshot manifest and installation receipt types; and
- structured failures for gaps, divergence, stale authority, unavailable
  quorum, rejected promotion, and failed snapshot installation.

Names, serialized fields, retryability, and stable error codes belong in the
Error Design Document and Inspect API Specification only after executable
callers show the contract.

## Development Gates And Implementation Slices

Each slice must compile, retain bounded evidence, and leave stronger levels
explicitly unsupported. Later implementation may begin only when the named
exit gate is satisfied. Within each capability level, work follows the same
evidence sequence where applicable: contract, executable baseline, fault
corpus, implementation, independent consumer or deployment, and acceptance
review.

### Slice 0: Native Replication Scope And Authority Admission

**Objective:** Decide what replicated Tosumu is allowed to mean before choosing
a protocol or dependency.

#### Deliverables

- [x] Open a replication and failover Architectural Review linked to this plan.
- [ ] Reconcile the SDD distributed-storage non-goal with its future semantic-
      change and K3s sections.
- [ ] Define supported failure domains: process, pod, node, disk, volume,
      network partition, delayed process, and stale restore.
- [ ] Define initial RPO/RTO targets as test hypotheses, not guarantees.
- [ ] Define write acknowledgement and read-consistency classes.
- [ ] Decide the single-leader invariant and keep active-active writes deferred.
- [ ] Record the authority-epoch and fencing requirements for any promotion.
- [ ] Inventory candidate coordination, transport, TLS, and K3s dependencies
      through AR-0010's provenance gate.

#### Acceptance Criteria

- [ ] Existing local guarantees and new distributed hypotheses are visibly
      distinct.
- [ ] No plan relies on shared-file locking or Kubernetes pod state as the sole
      split-brain prevention mechanism.
- [ ] Every stronger level has explicit unavailable-quorum and partition
      behavior.
- [ ] An accepted scope ADR or explicit parked disposition exists before Slice
      4 prototypes graduate into supported implementation.

#### Exit Gate

The service experiment may begin once ownership is narrow enough to implement
without importing cluster policy into core. Native replication may not begin
until its architecture is accepted.

### Slice 1: One Bounded Service Authority

**Objective:** Produce the first non-embedded authority boundary without
changing local storage semantics.

#### Deliverables

- [ ] Implement one local IPC or loopback-only service experiment over
      `SharedKvStore`.
- [ ] Preserve transaction, snapshot, conditional-write, error, and inspect
      semantics across the boundary.
- [ ] Add bounded request size, concurrency, cancellation, shutdown, and
      authentication/authorization experiments.
- [ ] Give each hosted operation an identity and externally observable terminal
      outcome consistent with AR-0008.
- [ ] Compare embedded and hosted lifecycle behavior with the same fixture.

#### Acceptance Criteria

- [ ] Exactly one authority owns the writer gate for a hosted database.
- [ ] A host cannot bypass public storage APIs or expose pager/WAL objects.
- [ ] Malformed, oversized, unauthorized, cancelled, and interrupted requests
      fail explicitly.
- [ ] AR-0003 has enough evidence to accept, revise, or reject the service
      boundary.

#### Exit Gate

One supported service contract owns database lifecycle and can be hosted in a
container without claiming replication.

### Slice 2: K3s Single-Authority Recovery Baseline

**Objective:** Establish Level 1 with one writable pod and independent recovery
copies.

#### Deliverables

- [ ] Add reproducible K3s manifests or a chart for one Tosumu host.
- [ ] Use an exclusive PVC; document local-path as single-node development only.
- [ ] Select one multi-node CSI/block-storage topology and retain its attach,
      detach, fencing, snapshot, and failure assumptions.
- [ ] Add startup, readiness, and liveness probes whose meanings do not exceed
      Tosumu's verified state.
- [ ] Publish stable backups to a separate failure domain with database
      identity, committed generation, hash, and verification result metadata.
- [ ] Exercise pod kill, graceful termination, node loss, unavailable volume,
      stale snapshot, backup loss, and cold restore.
- [ ] Record measured RPO, RTO, startup recovery time, and backup/restore cost.

#### Acceptance Criteria

- [ ] Pod replacement against the same retained PVC reopens and verifies the
      last supported committed state.
- [ ] Node recovery succeeds only on the named storage topology and does not
      imply support for arbitrary PVC providers.
- [ ] Loss of the primary volume has a verified operator-driven restore path.
- [ ] Two writable pods cannot reach the database through the supported
      deployment.
- [ ] Storage, host, and Tosumu failures remain distinguishable.

#### Exit Gate

Level 1 is supportable for the tested K3s topology. Failure to restore or prove
exclusive authority remains a readiness failure and operator action, not an
automatic promotion.

### Slice 3: Witness And Observer Freshness Evidence

**Objective:** Complete the existing MVP+12 scope and establish Level 2.

#### Deliverables

- [ ] Define and authenticate transport-neutral freshness receipts.
- [ ] Prototype the local observer and three independently placed witnesses.
- [ ] Define trust roots, rotation, quorum, gaps, clock assumptions, and outage
      policy.
- [ ] Bind backup metadata and restore decisions to witnessed identity and
      committed generation.
- [ ] Add stale-PVC, audit truncation, witness disagreement, and missing-receipt
      fixtures.

#### Acceptance Criteria

- [ ] A stale but cryptographically valid database is not reported as current.
- [ ] Witness unavailability, disagreement, and definite rollback are distinct
      outcomes.
- [ ] Witnesses cannot mutate or serve the database.
- [ ] Readiness can refuse unsafe writes without initiating failover.
- [ ] AR-0005 and security claims are updated from executable evidence.

#### Exit Gate

Level 2 is supportable. Operators can identify the last independently observed
state, but still require storage or backup data to restore it.

### Slice 4: Replication Protocol Admission And Snapshot Bootstrap

**Objective:** Complete Level 3 by using bounded executable prototypes to admit
one transport-neutral, gap-free replication representation without stabilizing
the physical WAL as public meaning.

#### Deliverables

- [ ] Implement two competing bounded prototypes if the architecture review
      cannot yet choose between normalized KV effects and dedicated physical
      replica records.
- [ ] Assign stable database, replica, transaction, and replication identities.
- [ ] Capture a verified snapshot and its replication position atomically or
      through a proven retry/stability protocol.
- [ ] Install the snapshot into an isolated standby destination and verify it
      before incremental apply.
- [ ] Detect duplicate, reordered, missing, corrupt, wrong-database, wrong-
      epoch, and unsupported-version messages.
- [ ] Bound batch bytes, transaction bytes, in-flight messages, retained
      history, and bootstrap resource use.
- [ ] Decide encryption-key and protector behavior through a security review.

#### Acceptance Criteria

- [ ] Snapshot plus increments reproduces the leader's committed logical state
      for the tested history.
- [ ] A gap cannot be skipped silently; an unrecoverable lag requests explicit
      reseeding.
- [ ] Reapplication is idempotent and never publishes a partial transaction.
- [ ] No public contract requires page numbers, WAL byte offsets, or checkpoint
      truncation behavior.
- [ ] Format and migration impact is accepted before durable bytes stabilize.

#### Exit Gate

One representation has evidence from bootstrap, catch-up, corruption, and
compatibility tests and is accepted through an ADR. The alternative is removed
or retained only as documented experimental evidence.

### Slice 5: Asynchronous Standby And Manual Promotion

**Objective:** Establish Level 4 with one leader and at least one warm standby.

#### Deliverables

- [ ] Stream committed units to a standby with durable received/applied
      watermarks and bounded backpressure.
- [ ] Expose lag, retained-history pressure, reseed requirement, divergence,
      and promotion eligibility through structured inspection.
- [ ] Define an operator promotion command that requires fencing evidence and a
      chosen recovery point.
- [ ] Reject writes on replicas and on stale authority epochs.
- [ ] Exercise disconnect/reconnect, slow replica, leader crash, standby crash,
      duplicate delivery, delayed delivery, and manual promotion.
- [ ] Measure steady-state latency, throughput, storage amplification, catch-up
      rate, RPO, and promotion time.

#### Acceptance Criteria

- [ ] The standby never exposes a partial committed unit.
- [ ] Lag is observable and a retained-history overrun requires explicit
      reseeding.
- [ ] Promotion cannot proceed without positive fencing evidence.
- [ ] The old leader rejects later writes after observing a newer epoch.
- [ ] Documentation states the remaining partition case in which external
      fencing must stop an isolated old leader.

#### Exit Gate

Level 4 is supportable with manual operations and a measured non-zero data-loss
window. Automatic failover remains disabled.

### Slice 6: Fenced Automatic Authority Transfer On K3s

**Objective:** Establish Level 5 without treating Kubernetes scheduling as the
source of database correctness.

#### Deliverables

- [ ] Implement or integrate leader election with monotonic authority epochs.
- [ ] Establish positive fencing for the old leader before replacement writes.
- [ ] Automate eligibility checks, promotion, service routing, and readiness.
- [ ] Preserve operator-visible reasons when failover is withheld.
- [ ] Define failback, demotion, and rejoin semantics for the former leader.
- [ ] Exercise control-plane outage, API latency, network partitions, paused
      processes, node loss, volume reattachment delay, stale PVC restoration,
      rolling upgrades, and simultaneous restart.
- [ ] Retain a manual recovery path when automation cannot prove safety.

#### Acceptance Criteria

- [ ] At most one eligible leader can acknowledge writes in every supported
      fault-injection schedule.
- [ ] An isolated old leader cannot publish under a superseded epoch.
- [ ] Automatic transfer occurs only with fenced prior authority and an
      eligible replica.
- [ ] Ambiguous cases remain unavailable with structured diagnostics.
- [ ] K3s manifests spread replicas across real failure domains and do not
      place all recoverability behind one node or storage controller.
- [ ] The measured async replication lag remains the explicit potential data-
      loss window during promotion.

#### Exit Gate

Level 5 is supportable only for the named K3s, storage, network, fencing, and
replica topologies backed by retained fault evidence.

### Slice 7: Synchronous Failover And Consensus

**Objective:** Establish Level 6 only if measured consumer requirements justify
zero or near-zero RPO and the project explicitly accepts distributed-state-
machine responsibilities.

#### Deliverables

- [ ] Retain a consumer requirement that async Level 5 cannot satisfy.
- [ ] Choose and document the replicated-log or consensus mechanism through
      dependency and protocol review.
- [ ] Bind commit acknowledgement to a durable quorum and authority epoch.
- [ ] Define membership, quorum changes, learner/bootstrap roles, and removal.
- [ ] Define behavior for lost quorum, minority partitions, slow disks, and
      rejoining divergent nodes.
- [ ] Add deterministic protocol simulation or model checking for election,
      replication, commit, and membership invariants.
- [ ] Add black-box multi-process and multi-node fault tests.

#### Acceptance Criteria

- [ ] A quorum-class acknowledged commit survives every failure set promised by
      the configured quorum model.
- [ ] A minority cannot acknowledge writes or advance committed authority.
- [ ] Rejoining nodes verify identity and history before serving or voting.
- [ ] Membership changes cannot accidentally create two valid quorums.
- [ ] Local-only, async, and quorum write results remain distinct at every API
      boundary.
- [ ] Automatic failover still requires fencing; quorum does not silently
      replace the Level 5 authority rules.

#### Exit Gate

Level 6 is supportable under the named quorum and failure model, or is parked
with the async Level 5 profile retained as the supported ceiling.

### Slice 8: Compatibility, Security, And Operational Hardening

**Objective:** Decide whether the feature remains experimental, graduates to a
supported deployment profile, or is parked.

#### Deliverables

- [ ] Run mixed-version upgrade, downgrade refusal, snapshot-install, and
      rolling-restart matrices.
- [ ] Complete key rotation, replica addition/removal, credential compromise,
      and restore-after-compromise exercises.
- [ ] Fuzz every untrusted replication and witness decoder.
- [ ] Run sustained lag, disk-full, WAL/replication retention, backup, and
      failover soak tests.
- [ ] Reconcile ADRs, specifications, public docs, charts, runbooks, and
      security limitations.
- [ ] Obtain independent design/security review before making production-grade
      confidentiality, integrity, or availability claims.

#### Acceptance Criteria

- [ ] Compatibility and migration behavior is explicit for every durable
      version participating in a cluster.
- [ ] Resource exhaustion produces bounded backpressure or typed refusal rather
      than silent history loss.
- [ ] Operator procedures are reproducible by someone other than the author.
- [ ] Remaining unsupported topologies and threat assumptions are prominent.

#### Exit Gate

The plan closes with an honestly named experimental or supported profile, or a
parked decision explaining why established replicated databases should be used
instead.

## Assurance And Evidence Export Dependency

Cluster capability cannot graduate solely from successful failover. The
[High-Assurance Engineering And Evidence Export](high-assurance-engineering-and-evidence-export.md)
plan owns the cross-cutting build, provenance, evidence-composition,
qualification, key-lifecycle, and independent-review gates.

Every supported cluster profile must make these observations available through
bounded machine-readable evidence when their owning subsystems exist:

- database, replica, build, and deployment-profile identity;
- committed generation, replication position, and authority epoch as separate
  domains;
- performed integrity and structural-verification dimensions;
- checkpoint and startup-recovery outcome;
- backup/export identity, generation, verification, and publication outcome;
- freshness anchor and witness agreement, gaps, or unavailability;
- replica role, received/durable/applied lag, and promotion eligibility;
- durability class and its supporting local, remote-copy, or quorum receipt;
  and
- explicit unsupported, unconfigured, incomplete, stale, or unverifiable
  dimensions.

Witness, data-quorum, build, backup, and authority evidence retain separate
signing purposes and trust roots. A cluster level may be functionally complete
while remaining experimental because its release provenance, platform
qualification, key boundary, or independent review has not reached the
required assurance profile.

## Validation Matrix

| Concern | Evidence | Required Result |
| --- | --- | --- |
| Local transaction and recovery | Existing and extended WAL/crash fixtures | Prior or complete committed state, never mixed state |
| Service parity | Embedded-versus-hosted fixture | Equivalent storage outcomes and structured errors |
| Snapshot bootstrap | Export/install/catch-up integration fixture | Gap-free verified replica state |
| Message safety | Unit, property, and fuzz tests | Typed bounded rejection; no panic or partial apply |
| Replica ordering | Duplicate/reorder/gap/delay simulation | Idempotent apply or explicit reseed |
| Authority safety | Deterministic protocol simulation/model | No two committed leaders in supported schedules |
| Process failure | Externally supervised subprocess fixtures | Terminal outcome classified per AR-0008 |
| K3s failure | Multi-node pod/node/network/storage injection | Only documented recovery or unavailable outcomes |
| Freshness | Stale PVC and witness disagreement corpus | No stale state reported as current |
| Security | Trust/key/credential and parser review | Claims remain within the reviewed threat model |
| Compatibility | Mixed-version and format fixtures | Supported interoperability or explicit refusal |
| Performance | Baseline, replication, catch-up, backup, failover benchmarks | Results retained as observations with topology |
| Documentation | `mkdocs build --strict` | Pass |
| Workspace | format, Clippy, and all-target tests | Pass |

## Failure And Diagnostic Semantics

The final names remain provisional, but the following meanings must remain
distinct:

- local storage failure versus transport failure;
- replica unavailable versus replica divergent;
- temporary lag versus history no longer retained;
- invalid message versus authenticated message from the wrong database or
  authority epoch;
- quorum unavailable versus quorum rejected the mutation;
- leader unknown versus known stale leader;
- promotion ineligible versus promotion blocked awaiting fencing;
- snapshot transfer failure versus snapshot verification failure;
- freshness unanchored versus witness unavailable versus rollback suspected;
- commit durable locally versus durable on one replica versus durable on a
  quorum; and
- externally observed termination versus unresolved disappearance.

No expected cluster failure may be represented only by a log line, indefinite
wait, pod restart loop, or generic I/O error when the responsible boundary can
classify it more precisely.

## Compatibility And Migration

- Levels 1 and 2 should avoid a database-format change unless receipt binding
  evidence proves one necessary.
- Level 4 may add durable replication identities or positions. If it does,
  ordinary open must reject incompatible formats before mutation and no
  automatic in-place migration is implied.
- Snapshot bootstrap never copies a live main file without its required WAL and
  stability evidence.
- A restored copy does not silently inherit authority to write. Restore and
  promotion are distinct operations.
- Mixed-version cohorts require an explicit compatibility interval. Rolling
  upgrade is unsupported until the matrix is executable.
- Downgrade must refuse rather than let an older writer erase replication or
  authority history it does not understand.

## Security And Trust

Replication adds network input, long-lived credentials, remote copies of
protected data, and new availability claims. Before Level 4 support:

- define mutual authentication and per-database authorization;
- bind every message to protocol version, database identity, sender identity,
  authority epoch, sequence position, transaction identity, and payload hash;
- cap decoding, decompression, batching, buffering, retries, and retained
  history before allocating or mutating;
- define replay, downgrade, cross-database splice, stale-leader, and compromised
  replica behavior;
- prevent diagnostics from exposing keys, credentials, plaintext values, or
  unrestricted paths;
- decide how protector and data-encryption keys reach or do not reach replicas;
  and
- keep freshness, durability, and availability claims separate.

Witness quorum, data quorum, and Kubernetes control-plane quorum are different
trust domains. Sharing the word "quorum" must not merge their authority.

## Performance And Resource Bounds

Each topology records at least:

- local and acknowledged commit latency by durability class;
- steady-state write throughput and replication amplification;
- received, durable, and applied replica lag;
- maximum in-flight and retained replication bytes;
- snapshot size, generation time, transfer time, verification time, and install
  time;
- catch-up rate relative to incoming write rate;
- backup publication and cold-restore time;
- failover detection, fencing, election, promotion, and readiness time; and
- memory/file-descriptor/task bounds per database, client, replica, and
  witness.

Backpressure must be decided before the stream is unbounded. A slow replica may
be disconnected and forced to reseed, but retained history must never disappear
silently while the replica is still reported as recoverable.

## Risks And Mitigations

| Risk | Impact | Mitigation Or Evidence |
| --- | --- | --- |
| Kubernetes restart is mistaken for data HA | Node or disk loss destroys the only copy | Name storage topology and test actual node/volume loss |
| Shared storage admits two writers | Corruption or divergent authority | Exclusive mount plus application authority and fencing; never shared NFS |
| Public WAL becomes replication protocol | Format internals become permanent and checkpointing constrains peers | Separate replication representation and keep WAL private |
| Async lag is advertised as zero RPO | Acknowledged recent commits disappear after promotion | Expose durable/applied watermarks and acknowledgement class |
| Election occurs without fencing | Split brain and divergent commits | Positive fencing and authority epochs before promotion |
| Witnesses are treated as replicas | Detection is mistaken for recoverability | Keep evidence and data roles distinct in APIs and deployment |
| Replicas copy corruption or deletion | All live copies become unusable | Independent versioned backups and restore verification |
| Slow replica causes unbounded retention | Primary disk exhaustion and write outage | Hard bounds, backpressure, lag diagnostics, explicit reseed |
| Mixed versions interpret history differently | Divergence during upgrade | Compatibility matrix and explicit refusal before rolling upgrades |
| Key distribution expands compromise radius | One replica compromises every copy | Dedicated security review and explicit shared-versus-independent key design |
| Consensus dependency exceeds project capacity | Large unaudited correctness surface | Proven dependency review, model tests, or park Level 6 |

## Completion Criteria

This long-term plan is complete when:

- [ ] the normative scope either admits the selected replication target or
      explicitly parks it;
- [ ] each claimed level passes its development and evidence gate;
- [ ] acknowledgement, RPO, RTO, consistency, promotion, and partition behavior
      are explicit;
- [ ] format, API, error, inspection, security, and deployment contracts agree;
- [ ] retained multi-process and multi-node fault evidence covers the supported
      topology; and
- [ ] stronger unimplemented levels remain visibly unsupported.

The plan may close successfully at Level 2, Level 4, or Level 5 if evidence
shows that quorum replication or automatic failover would harm Tosumu's
inspectability, security posture, or maintenance capacity. Parking a harder
level is a decision, not an implementation failure.

## Parking Or Reopening Criteria

Park native replication if no independent consumer needs a warm standby, if an
external storage provider satisfies the measured RPO/RTO, or if safe fencing
cannot be established for the target environment. Reopen when a consumer
supplies concrete recovery objectives, a second failure domain, representative
write workloads, and capacity to operate the resulting system.

Park automatic failover if manual promotion meets the operational objective or
if deterministic and black-box fault evidence cannot exclude split brain under
the supported model. Recommend an established replicated database when a user
needs stronger guarantees than Tosumu has earned.

## Progress Log

### 2026-09-03

- Work completed: recorded the capability and claim ladders and eight gated
  development slices; linked it to existing service, sync, freshness, format,
  evidence, and crash-outcome reviews.
- Validation: `git diff --check` and `mkdocs build --strict` pass on the initial
  planning and roadmap changes.
- Findings: useful K3s fault tolerance can precede native replication, but the
  intended harder target requires a normative scope decision before protocol
  work.
- Plan changes: added long-term single-leader async, fenced automatic-transfer,
  and final synchronous-consensus targets without admitting active-active
  writes.
- Next slice: Slice 0, beginning with the replication/failover Architectural
  Review and explicit failure/RPO/RTO model.

### 2026-09-03 -- Slice 0 Opened

- Work completed: opened AR-0015 for native replication scope, authority, and
  failure-model admission; separated scope admission from later protocol
  admission; made committed generation, replication position, and authority
  epoch explicitly distinct domains.
- Validation: `git diff --check` and `mkdocs build --strict` pass with AR-0015
  indexed in the review and documentation navigation.
- Findings: service-only epoch checking cannot by itself fence an isolated old
  leader; the protocol review must compare normalized committed effects with a
  dedicated physical committed-generation representation.
- Plan changes: renamed Slice 0 and Slice 4 to clarify their separate admission
  responsibilities.
- Next slice: retain one concrete consumer recovery objective and define the
  first K3s failure-domain/RPO/RTO hypotheses.

## References

- `docs/Specifications/Tosumu Software Design Document.md`, especially service,
  witness/observer, K3s, and sync-shaped design sections
- `docs/Specifications/Tosumu Error Design Document.md`
- `docs/Specifications/Tosumu Inspect API Specification.md`
- `SECURITY.md`
- `docs/ADR/ADR-0001-storage-engine-layer-boundaries.md`
- `docs/ADR/ADR-0004-cooperative-single-writer-admission.md`
- `docs/ADR/ADR-0005-committed-generation-and-retained-wal-snapshots.md`
- `docs/ADR/ADR-0006-shared-kv-store-and-snapshot-transactions.md`
- `docs/ADR/ADR-0007-database-generation-conditional-writes.md`
- `docs/Architectural Reviews/AR-0003-service-authority-and-host-modes.md`
- `docs/Architectural Reviews/AR-0004-semantic-change-history-and-sync.md`
- `docs/Architectural Reviews/AR-0005-witness-observer-and-freshness.md`
- `docs/Architectural Reviews/AR-0006-format-evolution-and-migration-boundary.md`
- `docs/Architectural Reviews/AR-0007-core-change-evidence-and-resilience.md`
- `docs/Architectural Reviews/AR-0008-operation-outcome-closure-and-crash-evidence.md`
- `docs/Architectural Reviews/AR-0015-native-replication-scope-authority-and-failure-model.md`
- `docs/Plans/main-feature-roadmap.md`
- `docs/Plans/high-assurance-engineering-and-evidence-export.md`
