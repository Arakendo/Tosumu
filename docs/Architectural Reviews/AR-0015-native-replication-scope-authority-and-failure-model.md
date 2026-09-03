# AR-0015: Native Replication Scope, Authority, And Failure Model

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-09-03 |
| Last reviewed | 2026-09-03 |
| Scope | Storage core / service authority / replication / K3s deployment |
| Trigger | The long-term roadmap now targets async standby, fenced automatic authority transfer, and conditional synchronous quorum durability |
| Related ADRs | ADR-0001, ADR-0004, ADR-0005, ADR-0006, ADR-0007 |
| Related evidence | Format-3 committed generations and snapshots, stable backup/export, AR-0003 through AR-0008, `docs/Plans/cluster-fault-tolerance-and-replication.md`, K3s/Kubernetes storage and coordination contracts |

## Architectural Question

Should Tosumu admit native single-leader data replication and eventual fenced
authority transfer above its embedded core, and which failure model, identity
domains, ownership boundaries, and claim limits must be accepted before a
replication protocol is selected?

## Context

Tosumu currently owns local embedded storage. Format 3 provides a durable
monotonic committed generation, authenticated pages, a physical recovery WAL,
process-local snapshots, one cooperative cross-process writer, stable backup,
portable export, and structured verification. These mechanisms provide useful
inputs to replication, but none is a replication or distributed-authority
protocol.

The normative Software Design Document contains deliberate tension that must be
resolved rather than interpreted opportunistically:

- §18.4 says distributed or replicated storage will never be added and directs
  those workloads to established distributed databases;
- §22 defines a possible service authority but explicitly says it is not a
  distributed database and owns no replication;
- §23 and MVP+12 define witnesses and observers on K3s while explicitly
  excluding writable replicas and automatic failover; and
- §30 describes future semantic change history for offline or multi-device
  synchronization.

The new cluster plan targets harder long-term capabilities without treating
that target as an accepted change. Its first admission gate is this review.
Operational recovery and freshness work can proceed within current boundaries;
native replication cannot become supported architecture until the conflict is
resolved through an ADR and corresponding normative updates.

This review is scope and authority admission. It does not choose the final
replication representation. A later protocol-admission cycle or separate
review must use executable bootstrap and catch-up prototypes to decide that
question.

## Terms And Claim Boundaries

The review uses the following terms narrowly:

- **Operational fault tolerance:** one Tosumu authority recovers using the same
  retained volume, storage-provider replication, or an independent backup.
- **Freshness evidence:** an observer or witness can detect that authenticated
  local state disagrees with previously observed state.
- **Native replica:** Tosumu-owned protocol state continuously maintains
  another database copy from complete committed units.
- **Standby:** a native replica that cannot accept client writes and may become
  eligible for promotion.
- **Promotion:** an explicit transition granting a standby write authority only
  after the prior authority is fenced.
- **Automatic authority transfer:** the system performs eligibility, fencing,
  promotion, and routing without an operator issuing each step.
- **Synchronous quorum durability:** a write is acknowledged in a quorum class
  only after the configured data quorum has durably accepted it.
- **Active-active:** more than one authority accepts writes concurrently. This
  remains outside the proposed target.

These claims form a strict ladder:

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

No rung implies the next one.

## Three Independent Sequence Domains

Replication work must not collapse three different meanings into one integer:

| Domain | Meaning | Current state | Must not mean |
| --- | --- | --- | --- |
| `CommittedGeneration` | Local atomic publication of one database state | Format 3 uses the durable commit-record LSN as the generation | Network receipt, replica progress, or permission to write |
| `ReplicationPosition` | Ordered progress in the admitted replication representation | Unimplemented | Physical WAL byte offset, page number, or authority grant |
| `AuthorityEpoch` | Which authority generation may publish writes | Unimplemented | Database content version or proof that a replica is caught up |

The protocol may define checked relationships among these domains. For example,
one replication unit may describe the effects of one committed generation, and
every unit may be scoped by an authority epoch. The types remain distinct even
if an early implementation happens to advance them together. Serialization,
comparison, rollover, restore, and migration rules must reject accidental
cross-domain use.

## Current Evidence

### Storage And Recovery Evidence

- ADR-0005 establishes monotonic committed generations, complete WAL-published
  mutations, retained authenticated versions, finite retention, and explicit
  checkpoint/recovery ordering.
- ADR-0006 exposes one shared owner with stable process-local snapshots and one
  atomic multi-mutation callback.
- ADR-0007 exposes conservative conditional writes, but its version tokens are
  owner-local and deliberately invalid after reopen. They are not durable
  replica or authority tokens.
- Stable backup captures a stable main/WAL pair; portable export stages,
  checkpoints, and verifies an independent artifact.
- Current fault evidence exercises local crash and recovery boundaries, not
  network partitions, replica divergence, or authority transfer.

### Authority And Coordination Evidence

- ADR-0004's persistent writer sidecar rejects cooperating local writers. It is
  advisory, path/file-identity based, and does not fence a remote or isolated
  former leader.
- AR-0003 identifies a plausible service authority but has no implemented
  local IPC host, remote host, authentication policy, cancellation contract, or
  lifecycle-parity evidence.
- Kubernetes Lease objects provide coordination and leader-election mechanics.
  They do not by themselves prove that an isolated process has lost access to
  its local storage or that a former leader cannot publish.
- A K3s local-path volume is bound to its node. It is useful for single-node
  development but is not evidence that data survives node or disk loss.

### Replication And Sync Evidence

- AR-0004 correctly prevents the physical recovery WAL from becoming an
  application synchronization protocol.
- AR-0004 does not settle the representation for a byte-identical passive
  standby under one authority. That problem has different conflict, identity,
  and compatibility requirements from offline multi-writer semantic sync.
- Format-3 generations and retained authenticated versions are newer evidence
  than AR-0004's first review cycle. They justify evaluating a dedicated
  physical committed-generation representation alongside normalized committed
  KV effects.
- No implementation currently emits stable change identities, replica
  positions, tombstones, replica acknowledgements, or promotion evidence.

### Freshness And Security Evidence

- AR-0005 keeps freshness unanchored. No signed receipt format, trust-root
  lifecycle, outage policy, or witness implementation exists.
- `SECURITY.md` is pre-audit and excludes consistent multi-page rollback,
  remote attestation, network key escrow, and KMS integration.
- Replication would add hostile network input, long-lived credentials, more
  protected copies, new key-distribution questions, and new availability
  claims.

### Consumer And Operational Evidence

- No retained consumer currently specifies a required RPO, RTO, promotion
  bound, read consistency, replica count, or failure-domain model.
- No multi-node K3s fixture demonstrates node loss, volume reattachment,
  network partition, stale-PVC restoration, or split-brain resistance.
- The current roadmap establishes ambition and work sequencing, not proof that
  the resulting operational burden is justified.

## Candidate Failure Model

These are hypotheses for Slice 0 evidence, not supported guarantees:

| Failure or condition | Candidate initial treatment | Open decision |
| --- | --- | --- |
| Process or pod crash-stop | Recover/restart from local durable state | Detection and readiness bounds |
| Graceful shutdown | Drain bounded work and close authority explicitly | Cancellation and timeout contract |
| Node crash or loss | Recover from named storage provider or native standby | Required failure-domain independence |
| Disk or volume loss | Restore verified backup or promote eligible standby | RPO/RTO and identity after restore |
| Message loss, duplication, reorder, and delay | Retry/idempotence or explicit gap/reseed | Sequence and retention representation |
| Network partition | Minority or ambiguous authority refuses writes | Fencing mechanism and availability tradeoff |
| Paused or delayed former leader | Treat as live until positively fenced | Storage-enforced versus service-only epoch |
| K3s control-plane outage | Preserve existing safe authority if possible; do not infer a new one | Lease expiry and cached-authority rules |
| Stale but authentic PVC or backup | Detect through witness/observer evidence | Enforcement policy and recovery selection |
| Replica state divergence | Refuse eligibility; diagnose and reseed | Comparison proof and repair prohibition |
| Local data corruption | Existing authentication/verification plus explicit replica policy | Whether healthy peers may repair automatically |
| Clock skew or rollback | Clocks do not establish ordering or authority correctness | Diagnostic-only time semantics |
| Byzantine or compromised replica | Outside initial crash-fault hypothesis unless security review admits it | Authentication and blast-radius requirements |

The first target should assume crash faults, omission/delay/reordering, and
detectable storage corruption. It must not imply tolerance of arbitrary
Byzantine behavior. Authentication is still required because an unauthenticated
peer or message is untrusted input even in a non-Byzantine availability model.

## Authority And Fencing Question

The strongest unresolved question is where an authority epoch becomes
enforceable.

### Service-Only Epoch Check

The authority layer checks `AuthorityEpoch` before calling unchanged local
storage.

- Benefits: preserves core format and keeps cluster policy above core.
- Costs: an isolated old process may still hold a valid local writer guard and
  enough storage authority to publish.
- Failure mode: a newer service epoch exists elsewhere, but the former leader
  continues committing locally because it cannot observe that fact.

### Storage-Enforced Epoch

Local publication includes or validates a durable authority epoch below the
service boundary.

- Benefits: stale authority rejection becomes structural at publication.
- Costs: changes local transaction, recovery, format, bootstrap, and standalone
  semantics; the storage file still cannot learn remote liveness by itself.
- Failure mode: two partitions each believe they own a newer epoch unless one
  external mechanism serializes epoch grants.

### Externally Fenced Storage Or Host

An independent system revokes the former leader's volume, process, credentials,
or hardware access before a new leader writes.

- Benefits: removes publication capability outside the failed process's
  cooperation.
- Costs: provider-specific behavior and operational coupling; fencing outcome
  must itself be trustworthy and observable.
- Failure mode: the controller mistakes requested fencing for completed
  fencing.

### Candidate Direction

Do not accept service-only epoch checking as sufficient fencing. The eventual
design likely needs both typed authority epochs and positive external or
storage-enforced revocation. This is a safety constraint for prototypes, not a
selection of a particular K3s, CSI, consensus, or hardware mechanism.

## Ownership And Dependency Analysis

### Core Storage

`tosumu-core` continues to own local transaction atomicity, committed-
generation meaning, recovery, authenticated pages, bounded snapshots, backup,
export, verification, and any minimal publication hook proven necessary by the
selected protocol. It must not import Kubernetes, transport, membership,
election, retry, or topology types.

### Service Authority

A reviewed authority layer owns database lifecycle, request admission, current
role, write serialization, authority-epoch observation, cancellation, and
mapping storage results into host-safe outcomes. It does not gain permission to
bypass core transaction, verification, or error contracts.

### Replication Coordinator

A future provider-neutral replication layer may own replica identities,
replication positions, snapshot installation, ordered delivery, durable/applied
watermarks, acknowledgement classes, lag, retention, reseeding, divergence,
and promotion eligibility. Whether this is part of `tosumu-service` or a
separate crate remains open.

### Hosts And K3s

Hosts own IPC/HTTP/gRPC, authentication, authorization, TLS, rate and resource
limits, process supervision, and untrusted decoding. K3s resources own desired
placement and routing. Neither Kubernetes pod phase nor Lease state alone is a
database correctness proof.

### Witnesses And Observers

Witnesses and observers own external freshness evidence. They are not database
replicas, data-quorum voters, or automatic-promotion authorities in the MVP+12
design. Witness quorum, data quorum, and Kubernetes control-plane quorum remain
separate domains.

### Consumers

Consumers own business RPO/RTO, workload and topology evidence, semantic
conflict policy, and the decision that a harder operational level is worth its
cost. Tosumu must not invent a zero-RPO requirement merely to justify
consensus.

## Alternatives Considered

### Alternative A: Retain Operational Recovery Only

Use one authority with a reattachable or storage-provider-replicated block
volume, verified offsite backups, restore drills, and witnesses.

- Benefits: preserves the embedded storage identity and obtains substantial
  operational value with the smallest correctness surface.
- Costs: no Tosumu-maintained warm copy; node/storage recovery depends on the
  named provider; RTO may be minutes or longer.
- Failure mode: provider replication or backup frequency is mistaken for
  engine-level RPO and availability guarantees.

### Alternative B: Admit Single-Leader Asynchronous Replication

Maintain one or more passive standbys through a dedicated protocol, begin with
manual fenced promotion, then evaluate automatic authority transfer.

- Benefits: separates data-copy evidence from consensus, permits measured
  bounded-RPO recovery, and teaches identity/fencing before automation.
- Costs: introduces service, protocol, retention, bootstrap, key, compatibility,
  and operational state.
- Failure mode: lag or incomplete fencing is hidden behind an "HA" label.

### Alternative C: Begin With Synchronous Consensus

Make a replicated log or consensus system the first native replication model.

- Benefits: can define quorum durability and authority together.
- Costs: largest implementation, dependency, testing, upgrade, and operational
  burden before a consumer has established the requirement.
- Failure mode: Tosumu becomes an unaudited distributed state machine whose
  complexity outruns its local storage evidence.

### Alternative D: Active-Active Semantic Replication

Allow several writable authorities and resolve application conflicts.

- Benefits: disconnected writes and multi-site availability.
- Costs: requires stable semantic identity, conflict policy, tombstones,
  causality, and consumer ownership far beyond passive standby replication.
- Failure mode: storage core silently acquires application meaning or conflicts
  are resolved without authority.

### Alternative E: Replicate The Existing WAL Directly

Stream current WAL records and offsets as the public replica protocol.

- Benefits: apparently reuses durable physical records.
- Costs: exposes page layout, checkpoint horizon, retention, encryption frame,
  and recovery internals as distributed compatibility contracts.
- Failure mode: local checkpoint or format evolution invalidates remote
  ordering and silently drops required history.

## Preliminary Findings

- Operational K3s recovery and external freshness evidence are compatible with
  current architecture and may proceed without admitting native replication.
- Native replication is a plausible long-term Tosumu capability only as an
  explicit single-leader layer above the embedded core. It is not a hidden
  behavior of network storage or the local writer sidecar.
- AR-0004 excludes public application sync over the physical WAL but does not
  answer the passive-standby representation question. The protocol admission
  must compare normalized committed effects with a dedicated physical
  committed-generation representation.
- `CommittedGeneration`, `ReplicationPosition`, and `AuthorityEpoch` require
  separate types and durable meanings.
- Manual promotion must precede automatic authority transfer. Positive fencing
  is required for both; service-only observation of a newer epoch is not enough
  to disable an isolated old authority.
- Automatic async failover and synchronous acknowledged-write survival are
  separate capabilities. Consensus is conditional on retained evidence that
  measured async RPO is insufficient.
- Active-active writes, application conflict resolution, and general-purpose
  distributed SQL remain outside this review.
- Current evidence is insufficient to accept native replication, choose a
  protocol representation, add a consensus dependency, change the format, or
  claim a bounded RPO/RTO.

## Disposition

Incubating. Continue Slice 0 by collecting a concrete consumer recovery
objective and exercising the service and operational K3s boundaries. Native
replication implementation is not admitted. A bounded representation prototype
may later gather protocol evidence, but it must not stabilize public APIs,
durable bytes, or dependencies before scope admission through an ADR.

## Required Follow-Up

- [ ] Retain one representative consumer's required and preferred RPO, RTO,
      write availability, read consistency, failure domains, and operational
      constraints.
- [ ] Complete AR-0003's bounded service-host experiment and lifecycle parity
      evidence.
- [ ] Establish the Level 1 K3s baseline for one local-path development profile
      and one explicitly named multi-node storage profile.
- [ ] Complete AR-0005 receipt, trust-root, outage, and stale-state prototypes.
- [ ] Define the minimum external fencing evidence accepted for manual
      promotion; distinguish request, completion, and verification.
- [ ] Model restore and promotion identity transitions, including whether a
      restored database retains or changes database/replica identity.
- [ ] Review shared versus independent replica encryption keys and protector
      ownership.
- [ ] Decide whether to admit native single-leader replication in a scope ADR
      or park it at operational recovery/freshness.
- [ ] After scope admission, open the protocol-admission cycle comparing
      normalized committed effects and dedicated physical committed-generation
      records through bootstrap/catch-up prototypes.
- [ ] Do not review consensus dependencies until a retained requirement shows
      the async automatic-transfer profile is insufficient.

## Scope-Admission Gate

An ADR admitting native replication must state at least:

- Tosumu remains an embedded core with an explicit authority/replication layer
  above it;
- one active leader is the maximum supported write-authority count;
- the supported failure model and explicitly excluded Byzantine behavior;
- the separation among local generation, replication position, and authority
  epoch;
- the boundary at which stale authority is rejected and the independent
  mechanism that positively fences prior authority;
- why replication is justified beyond provider-replicated storage plus backup;
- that witnesses are not data replicas or data-quorum voters;
- that the recovery WAL is not the public replication protocol;
- that backup remains required after replication; and
- which protocol, format, dependency, and security questions remain for the
  later protocol-admission gate.

## Reopening Triggers

- A consumer supplies concrete RPO/RTO or node-loss requirements.
- A bounded service host proves or falsifies the proposed authority ownership.
- A K3s recovery exercise shows provider storage satisfies or fails the target.
- A bootstrap/catch-up prototype demonstrates a representation that preserves
  identity, atomicity, and bounded retention.
- Fencing evidence cannot prevent a paused or partitioned old leader from
  publishing.
- A proposed format or dependency change crosses an accepted boundary.
- A use case requires active-active writes, Byzantine tolerance, follower
  reads, cross-region latency, or zero-RPO acknowledgement.

## Review History

### Cycle 1 -- 2026-09-03

- Status entering review: Proposed
- New evidence: the long-term cluster plan separates operational recovery,
  freshness, async standby, fenced automatic transfer, and conditional quorum
  durability; format-3 generations now provide stronger internal publication
  evidence than existed when AR-0004 first reviewed semantic sync.
- Findings: the hard target is coherent as a gated single-leader direction, but
  the normative distributed-storage exclusion, absent service host, missing
  consumer recovery objective, unresolved fencing boundary, and untested
  replication representation prevent acceptance.
- Disposition: Incubating; proceed with scope and authority evidence only.
- Resulting ADR or documentation change: none. The plan now distinguishes scope
  admission from later protocol admission and preserves three independent
  sequence domains.

## References

- `docs/Plans/cluster-fault-tolerance-and-replication.md`
- `docs/Plans/high-assurance-engineering-and-evidence-export.md`
- `docs/Plans/main-feature-roadmap.md`
- `docs/Specifications/Tosumu Software Design Document.md`
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
- [Kubernetes StatefulSets](https://kubernetes.io/docs/concepts/workloads/controllers/statefulset/)
- [Kubernetes Leases](https://kubernetes.io/docs/concepts/architecture/leases/)
- [K3s Volumes And Storage](https://docs.k3s.io/add-ons/storage)
