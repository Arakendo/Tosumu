# High-Assurance Engineering And Evidence Export

| Field | Value |
| --- | --- |
| Status | Proposed; assurance claims not yet admitted |
| Opened | 2026-09-03 |
| Last updated | 2026-09-03 |
| Owner | Tosumu maintainers |
| Target | Cross-cutting long-term assurance and evidence track |
| Related ADRs | ADR-0001, ADR-0002, ADR-0003 |
| Related reviews | AR-0002, AR-0005, AR-0007, AR-0008, AR-0010, AR-0015 |
| Related CRs | None yet |
| Depends on | Current inspect/error contracts, retained recovery evidence, explicit threat model, and independently reviewable build/dependency inputs |

## Status

Tosumu has unusually explicit local storage, recovery, integrity, inspection,
and failure contracts for an early-stage project. It is nevertheless pre-audit,
pre-stability, and unsuitable for protecting real secrets under the current
`SECURITY.md` posture. The repository has no accepted general dependency-
provenance policy, reproducible-release profile, software bill of materials,
platform qualification matrix, signed evidence bundle, independent security
review, or certification claim.

This plan defines how those gaps may close incrementally. It does not describe
Tosumu as production hardened, defense ready, certified, audited, tamper-proof,
or suitable for a regulated workload today.

## Purpose

The long-term design target is a compact embedded or edge database for systems
that value authenticated state, intermittent-connectivity resilience,
inspectable recovery, externally anchored freshness, and optional resilient
single-leader operation.

The differentiating goal is not feature count. It is to make fewer important
properties implicit:

- state identity is explicit;
- integrity is explicit;
- freshness is explicit;
- authority is explicit;
- durability class is explicit;
- recovery is explicit;
- provenance is explicit; and
- unsupported guarantees are explicit.

This direction may fit industrial, field, robotic, remote-infrastructure,
aerospace, maritime, critical-infrastructure, secure-appliance, regulated,
disconnected-enterprise, or defense-adjacent environments. Those are candidate
pressure sources, not supported-use or market claims. Requirements must be
derived from concrete deployments and assurance profiles rather than from the
word "defense."

## Trigger And Evidence

- Authenticated pages and structured integrity failures already make local
  byte trust more inspectable than an unverified storage file.
- Format-3 committed generations identify which durable local state a snapshot
  observes, while explicitly not proving external freshness.
- AR-0005 and the witness plan distinguish valid state from current state.
- The cluster plan now separates local, replica, authority, and quorum claims,
  increasing the need for machine-readable evidence that survives boundaries.
- The Inspect API provides a common envelope and structured payload findings,
  but no single evidence export relates identity, generation, integrity,
  recovery, freshness, authority, backup, durability, and build provenance.
- AR-0010 records that Cargo lockfile resolution is useful but not a complete
  source-audit, offline-build, update, or release-provenance policy.
- AR-0007 and AR-0008 identify proportional evidence and externally observed
  terminal outcomes as unresolved cross-cutting disciplines.
- `SECURITY.md` is explicit that the cryptographic composition is unaudited and
  that several freshness, host, key-management, and process threats remain out
  of scope.

## Positioning Guardrail

The following is a design hypothesis suitable for planning:

> A compact, embeddable, authenticated database for high-assurance and
> intermittently connected systems, with inspectable integrity, explicit
> recovery semantics, externally anchored freshness, and optional resilient
> single-leader operation.

It is not acceptable as an unqualified present-tense product claim. Public
language must name the achieved assurance profile and its limitations. Until
independent review and profile graduation occur, prefer precise capability
statements such as "page authentication is checked on this path" or "this
fixture recovered the committed generation after the injected crash" over the
label "high assurance."

## Assurance Capability Levels

These levels describe evidence maturity. They complement feature milestones and
do not replace them.

### Assurance Level A0: Honest Experimental Baseline

Normative specifications, ADRs, security limits, typed errors, tests, and
inspection distinguish implemented behavior from future design.

**Current disposition:** partially established.

**Does not claim:** audited cryptography, reproducible releases, complete source
provenance, platform qualification, or production suitability.

### Assurance Level A1: Identified Build And Dependency Closure

Every supported artifact has a machine-generated runtime, build, procedural-
macro, native, and tooling dependency inventory. Critical dependencies have
retained source identity, version/checksum, features, license, unsafe boundary,
build-script behavior, target closure, update ownership, and advisory status.

**Target claim:** reviewers can identify the source and enabled dependency
closure intended to produce a supported artifact.

**Does not claim:** byte-for-byte reproducibility, absence of vulnerabilities,
or audit of every dependency implementation.

### Assurance Level A2: Reproducible And Attested Artifacts

Supported release artifacts have pinned toolchain and build inputs, offline or
controlled-source build instructions, an SBOM, checksums, signed provenance,
and reproducibility comparison across independent builders where technically
possible.

**Target claim:** an operator can relate a distributed artifact to declared
source, toolchain, dependency, feature, and target inputs.

**Does not claim:** source correctness or that identical vulnerable source
becomes safe because it reproduces.

### Assurance Level A3: Bounded Operational Evidence Export

One versioned, bounded, machine-readable evidence bundle composes existing
inspection facts with generation, recovery, backup, freshness, authority,
durability, and build provenance observations. Missing or unestablished
dimensions remain explicit rather than omitted or inferred.

**Target claim:** independent tooling can determine what was observed, by which
component, under which schema/profile, and which trust dimensions remain
unproven.

**Does not claim:** semantic truth of application data, external freshness
without witnesses, or authenticity of an unsigned/unanchored bundle.

### Assurance Level A4: Qualified Failure And Platform Profiles

Named operating-system, filesystem, architecture, storage-provider, and host
profiles have retained long-duration, resource-pressure, crash, upgrade,
restore, and recovery evidence. Claims attach to a profile and version rather
than to "Tosumu" in the abstract.

**Target claim:** a supported profile has reproducible evidence for its stated
failure and resource model.

**Does not claim:** correctness on untested platforms, arbitrary hardware, all
fault schedules, or provider behavior stronger than its documented contract.

### Assurance Level A5: Reviewed Isolation And Key Lifecycle

Service and deployment profiles define privilege separation, secret delivery,
key/protector rotation, credential revocation, backup-key ownership, crash-dump
and swap exposure, diagnostic redaction, and secure-deletion limitations.

**Target claim:** operators can understand which principals and failure domains
can read, mutate, recover, witness, replicate, or destroy protected state.

**Does not claim:** resistance to a compromised process or hardware, guaranteed
physical erasure on opaque flash/storage layers, or remote attestation unless a
later review explicitly admits those mechanisms.

### Assurance Level A6: Independently Reviewed Deployment Profile

A named, narrow deployment profile has independent cryptographic, storage,
protocol, and operational review; findings and remediation are retained; the
public claim vocabulary is revised to match the resulting evidence.

**Target claim:** only the exact reviewed profile and version may carry the
accepted assurance statement.

**Does not claim:** general certification, indefinite future-version coverage,
or suitability for every high-assurance, regulated, or defense environment.

## Evidence Questions

The evidence-export track should eventually let an authorized operator or tool
ask, within explicit collection and trust bounds:

1. What database identity is this?
2. Which physical format and software/build identity produced the observation?
3. Which committed generation is visible?
4. Were all inspected pages authenticated, and which dimensions were not
   inspected?
5. Is the B+ tree or higher storage structure internally consistent?
6. What checkpoint and WAL/recovery state was observed?
7. Did startup recovery run, what outcome did it report, and what generation
   followed it?
8. Is this state externally freshness-anchored, and which witnesses agree,
   disagree, are missing, or are unavailable?
9. Which authority epoch and replica role produced or currently owns the state?
10. Which backup or export corresponds to this generation and identity?
11. Which durability class did a referenced commit receive?
12. What evidence is missing, stale, incomplete, unauthenticated, or outside the
    selected assurance profile?

The first evidence bundle need not answer questions for unimplemented systems.
It must answer those dimensions as `unsupported`, `unconfigured`,
`unobserved`, `incomplete`, or another reviewed state instead of fabricating an
`ok` result.

## Evidence Subjects And Observation States

Every evidence component must identify the precise subject of its proposition.
The minimum conceptual shape is:

```text
subject:
  database_identity
  committed_generation
  replica_identity
  authority_epoch
  backup_or_export_identity
  artifact_identity
  build_identity

observation:
  kind
  result
  scope
  observed_at

provenance:
  producer
  method
  evidence_identity
```

An evidence component may use only the subject fields relevant to its meaning,
but it cannot leave the subject ambiguous. A valid backup receipt for generation
418 and a valid witness receipt for generation 421 do not collectively prove
that the backup contains generation 421. Composition must compare subject
identity and scope structurally rather than relying on nearby placement in one
JSON object.

The candidate observation-state vocabulary is:

| State | Meaning |
| --- | --- |
| `not_applicable` | The dimension does not apply to this explicitly named subject/profile |
| `unsupported` | The implementation or profile cannot establish this dimension |
| `supported_unconfigured` | The capability exists but required configuration is absent |
| `configured_unobserved` | Configuration exists but no relevant observation was obtained |
| `attempted_incomplete` | Collection began but could not produce a complete observation |
| `observed_pass` | The named check ran over the stated scope and reported no finding |
| `observed_finding` | The named check ran and reported one or more findings |
| `stale` | Evidence was valid for an earlier subject state or accepted time/policy window |
| `unverifiable` | Evidence bytes exist but their authenticity, identity, method, or dependency cannot currently be verified |

These are planning terms, not yet public enum or wire values. Feature lifecycle
(`planned`, `implemented`, `accepted`) remains separate from observation state.
In particular, `not_applicable` requires an explicit profile rule; it is not a
convenient substitute for missing evidence.

Evidence components may depend on other identified evidence. Preserve stable
evidence identity, producer, method, and dependency references now without
building a general evidence graph. Revoking build provenance may reduce policy
trust in a collector's observation without retroactively changing the
mathematical result of a page-authentication check. The evidence layer reports
both facts; it does not collapse them into one truth bit.

## Ownership And Dependency Boundary

```text
build and release provenance          witness / observer receipts
              \                              /
               bounded evidence composition
                         |
              inspect and error contracts
                         |
       public storage / service / replication facts
                         |
       pager, recovery, authenticated pages, files
```

### Core And Owning Subsystems

Each subsystem owns the facts it can establish:

- core owns local format, generation, recovery, page integrity, and structural
  verification observations;
- backup/export owns artifact capture, verification, identity, and publication
  observations;
- witness/observer owns external freshness evidence;
- service/replication owns authority, role, lag, promotion, and durability-
  class evidence;
- build/release tooling owns source, toolchain, dependency, feature, target,
  SBOM, checksum, and artifact provenance; and
- the evidence layer composes typed projections without redefining or
  strengthening their meaning.

### Consumer And Policy Ownership

Consumers decide whether a bundle satisfies their policy. Tosumu may report
that an authenticated page set, witness quorum, or build provenance statement
was observed. It does not infer that a mission, regulation, procurement rule,
or application-semantic requirement is satisfied.

### Dependency Direction

Evidence composition depends on public provider-neutral facts. It cannot hand
inspection or build tooling pager handles, decrypted values, keys, arbitrary
filesystem authority, or consumer schemas. `tosumu-core` must not depend on a
release service, SBOM format, remote witness, Kubernetes type, or policy engine.

## Evidence Bundle Candidate Boundary

The eventual public name and serialization are provisional. The conceptual
shape is:

```text
EvidenceBundle
  envelope: schema, collection identity, collector/build identity
  database: identity, format, committed generation
  integrity: performed dimensions, results, findings, incomplete reasons
  recovery: checkpoint, WAL, startup-recovery observation
  freshness: anchor state, witness observations, gaps
  authority: role, authority epoch, replication position
  backup: related artifact identity, generation, verification
  durability: declared class and supporting receipt when applicable
  provenance: source, toolchain, dependency/SBOM, artifact checksums
  limitations: unsupported, unconfigured, unobserved, stale dimensions
```

Design requirements:

- bounded cardinality and encoded size;
- deterministic field meaning and one canonical field per concept;
- versioned schema only when a real compatibility need exists;
- clear distinction between collected observation and verified guarantee;
- stable issue codes for reportable findings;
- top-level errors only when no meaningful bundle can be produced;
- redaction and authorization before paths, identities, topology, or operational
  details cross a boundary;
- canonical serialization before signing is considered;
- collection time is diagnostic metadata, never freshness or ordering proof;
  and
- explicit provenance for every externally supplied evidence component.

The bundle should compose existing inspect reports rather than replace their
focused commands. AR-0002 must review the stable composition boundary before a
new command or Rust API is admitted.

## Development Gates And Implementation Slices

### Slice 0: Assurance Claims And Evidence Inventory

**Objective:** Establish the vocabulary and inventory before adding release or
evidence frameworks.

#### Deliverables

- [x] Publish a bounded v1 inventory of the principal current and future claims,
      gaps, and executable evidence.
- [ ] Inventory every current public integrity, durability, recovery,
      freshness, authority, provenance, and platform claim.
- [ ] Map each claim to its normative owner, implementation, tests, retained
      evidence, and unsupported boundary.
- [x] Define the provisional A0-A6 profile vocabulary and review its first use
      through AR-0007; permanent acceptance remains open.
- [ ] Identify stale, duplicated, ambiguous, or aspirational public language.
- [x] Record one representative operator/consumer assurance questionnaire.

#### Acceptance Criteria

- [ ] No capability is marked established solely because a design section or
      unchecked roadmap item describes it.
- [ ] Every current claim has an evidence owner or is demoted to an explicit
      hypothesis/limitation.
- [ ] Profile names cannot be mistaken for certification levels.
- [ ] The inventory identifies the minimum useful evidence bundle.

#### Exit Gate

The project has one reviewable assurance baseline and knows which claims are
current, future, unsupported, or externally owned.

### Slice 1: Dependency And Source Provenance Baseline

**Objective:** Advance AR-0010 from one focused dependency review to a
repository-wide risk-tiered policy candidate.

#### Deliverables

- [ ] Generate runtime, build, development, procedural-macro, native, fuzz,
      WASM, and release dependency closures. The aggregate workspace, three
      native target, and WASM closures are retained; separate fuzz and release
      profiles remain open.
- [ ] Classify authentication-, format-, unsafe-, parser-, randomness-, and
      public-vocabulary-critical dependencies.
- [ ] Retain source identity, checksums, features, license, build scripts,
      unsafe boundaries, supported targets, advisories, and update owners for
      the critical closure.
- [ ] Define dependency-addition and update evidence gates.
- [ ] Decide lockfile-only, vendored, controlled registry, exact-Git, or hybrid
      source policy for supported release profiles.

#### Acceptance Criteria

- [x] The initial workspace inventory is machine-generated and reviewable
      rather than manually guessed.
- [ ] Risk classification cannot be lowered without retained rationale.
- [x] Initial Linux, Windows, macOS, and WASM target closures and selected
      features are visible; supported release-profile admission remains open.
- [ ] AR-0010 can accept, revise, or park a general provenance policy.

#### Exit Gate

Assurance Level A1 is supportable for named artifacts and targets.

### Slice 2: Reproducible Build And Release Provenance

**Objective:** Relate distributed artifacts to controlled source and build
inputs.

#### Deliverables

- [ ] Pin and publish the supported Rust toolchain and target inputs.
- [ ] Add controlled-source or offline build instructions and verification.
- [ ] Generate an SBOM and artifact checksums for each release profile.
- [ ] Emit signed build provenance with repository revision, dirty-state policy,
      dependency lock identity, feature set, target, and builder identity.
- [ ] Compare artifacts from at least two independent builders and document
      deterministic and nondeterministic fields.
- [ ] Define signing-key custody, rotation, revocation, and verification.

#### Acceptance Criteria

- [ ] An artifact can be traced to declared source and build inputs.
- [ ] A dirty or mismatched input cannot silently receive normal release
      provenance.
- [ ] Reproducibility failures report exact differences or remain explicit
      unresolved evidence.
- [ ] Provenance never substitutes for correctness or vulnerability review.

#### Exit Gate

Assurance Level A2 is supportable for the named release artifacts.

### Slice 3: Bounded Evidence Export

**Objective:** Prove one composed machine-readable evidence boundary through an
independent operator or consumer.

#### Deliverables

- [ ] Reopen AR-0002 for evidence composition and schema ownership.
- [ ] Build one private or explicitly experimental `EvidenceBundle` prototype
      from public inspect/verification facts.
- [ ] Include database/build identity, format, generation, performed integrity
      dimensions, checkpoint/recovery observations, and explicit limitations.
- [ ] Add optional freshness, authority, backup, replication, and durability
      projections only as their owning subsystems become real.
- [ ] Define collection bounds, redaction, authorization, canonical encoding,
      signing candidates, and schema compatibility.
- [ ] Exercise the bundle through one caller that does not import pager, WAL,
      crypto-frame, or host internals.

#### Acceptance Criteria

- [ ] Missing evidence cannot become `ok` through omission or a default value.
- [ ] A completed partial report remains distinct from failure to construct a
      trustworthy envelope.
- [ ] Every composed fact retains its owning subsystem and evidence provenance.
- [ ] Bundle contents are bounded and expose no decrypted user values or keys.
- [ ] Existing focused inspect commands remain usable and semantically
      consistent.

#### Exit Gate

AR-0002 either admits a minimal evidence-export contract or records why focused
reports should remain separate. If admitted, update the Inspect API and Error
Design specifications before stabilization.

### Slice 4: Recovery, Backup, Freshness, And Authority Receipts

**Objective:** Make operational state transitions independently inspectable
without merging their trust domains.

#### Deliverables

- [ ] Retain startup-recovery operation identity, prior/after generation,
      performed actions, and terminal outcome.
- [ ] Bind backup/export manifests to database identity, committed generation,
      verification result, artifact hash, and publication outcome.
- [ ] Add witness/observer receipt projections once AR-0005 admits them.
- [ ] Add authority epoch, replica position/role, promotion, and durability
      receipt projections only after AR-0015 and cluster protocol admission.
- [ ] Define expiry/staleness and verification for every externally supplied
      receipt.
- [ ] Ensure witness, data-quorum, build, backup, and authority signatures use
      distinct purposes and trust roots.

#### Acceptance Criteria

- [ ] An operator can relate a recovered state to its generation and recovery
      evidence.
- [ ] A backup can be selected and verified without treating its existence as
      freshness proof.
- [ ] Freshness, authority, durability, and provenance remain separate fields
      and failure classes.
- [ ] Unknown or unavailable external evidence is never synthesized locally.

#### Exit Gate

Assurance Level A3 is supportable for the implemented dimensions of one named
deployment profile.

### Slice 5: Platform Qualification And Long-Duration Failure Evidence

**Objective:** Attach claims to named operating profiles and sustained evidence.

#### Deliverables

- [ ] Define supported OS, architecture, filesystem, storage, host, container,
      and deployment profiles.
- [ ] Run long-duration reopen, mutation, snapshot, backup, restore, rekey,
      VACUUM, and replication workloads applicable to each profile.
- [ ] Exercise disk-full, descriptor exhaustion, memory pressure, process kill,
      abrupt node loss, filesystem errors, clock anomalies, upgrade, downgrade,
      rollback, and corrupted artifact cases.
- [ ] Record expected injected termination separately from unresolved process
      disappearance under AR-0008.
- [ ] Measure resource ceilings, degradation, recovery time, and evidence-
      collection overhead.
- [ ] Publish a machine-readable profile manifest tying claims to evidence.

#### Acceptance Criteria

- [ ] Every supported profile has a repeatable validation matrix and retained
      result identity.
- [ ] An unavailable platform or fault environment is not reported as a pass.
- [ ] Long-duration tests have explicit operation counts, durations, seeds, and
      terminal classifications.
- [ ] Claims do not silently transfer between filesystems, CSI providers,
      architectures, or host modes.

#### Exit Gate

Assurance Level A4 is supportable for the named profiles that pass; others
remain experimental or unsupported.

### Slice 6: Privilege, Secret, And Destruction Boundaries

**Objective:** Define who can observe, mutate, recover, replicate, or destroy
state and what revocation can actually guarantee.

#### Deliverables

- [ ] Threat-model embedded, daemon, remote, observer, witness, backup, replica,
      and release-signing principals separately.
- [ ] Prototype least-authority process/service boundaries where independent
      failure domains justify them.
- [ ] Define key/protector creation, delivery, rotation, revocation, escrow,
      backup, replica, and destruction workflows.
- [ ] Document OS swap, hibernation, crash dump, memory, container-secret, and
      diagnostic exposure.
- [ ] Test redaction and authority failure at every external boundary.
- [ ] State secure-deletion limits for filesystem, SSD/flash translation,
      snapshots, replicas, backups, and remote providers; do not claim physical
      erasure without platform evidence.

#### Acceptance Criteria

- [ ] Possession of one capability does not imply unrelated database, key,
      witness, backup, or administrative authority.
- [ ] Revoked or stale authority fails explicitly at the protected boundary.
- [ ] Key rotation and deletion claims name which retained copies remain able to
      recover data.
- [ ] Unsupported erasure and attestation guarantees are prominent.

#### Exit Gate

Assurance Level A5 is supportable for one narrow deployment profile, or the
remaining isolation limitations are explicitly retained.

### Slice 7: Independent Review And Profile Graduation

**Objective:** Let evidence from outside the implementation team determine the
strongest honest supported claim.

#### Deliverables

- [ ] Commission independent review of cryptographic composition, storage and
      recovery invariants, hostile-input boundaries, and any admitted
      replication/fencing protocol.
- [ ] Retain findings, severity, affected profiles, remediation, retest, and
      accepted residual risk without publishing sensitive exploit details.
- [ ] Exercise vulnerability intake, dependency advisory, signing-key
      compromise, artifact revocation, and update communication procedures.
- [ ] Define release support and backport policy before promising one.
- [ ] Review applicable certification or procurement requirements only for a
      concrete customer/profile; do not invent blanket compliance.
- [ ] Revise public positioning, `SECURITY.md`, specifications, and profile
      manifests to exactly match the accepted evidence.

#### Acceptance Criteria

- [ ] No unresolved critical finding is hidden by a broad assurance label.
- [ ] The supported statement identifies the reviewed version, build, target,
      configuration, topology, and limitations.
- [ ] Later versions do not inherit the reviewed status automatically.
- [ ] Incident and revocation paths are executable rather than prose-only.

#### Exit Gate

Assurance Level A6 applies only to the explicitly reviewed deployment profile,
or the project remains honestly at its prior level.

## Validation Matrix

| Concern | Evidence | Required Result |
| --- | --- | --- |
| Claim inventory | Normative/public statement mapping | Every claim owned, evidenced, or demoted |
| Dependency closure | Generated Cargo/target graph | Complete for named artifact and feature set |
| Critical dependency review | Source/checksum/feature/license/unsafe/build-script record | Reviewable and update-owned |
| Artifact provenance | SBOM, checksums, signed build statement | Artifact matches declared inputs |
| Reproducibility | Independent builder comparison | Identical or explained bounded differences |
| Evidence schema | Contract and compatibility fixtures | Bounded canonical meaning; explicit missing states |
| Integrity/recovery composition | Known valid, corrupt, incomplete, and recovery fixtures | No stronger claim than source reports |
| Authority/redaction | Negative permission and disclosure tests | Least authority and no secret/value leakage |
| Platform qualification | Named matrix and retained run identity | Pass only on executed profile |
| Long-duration behavior | Seeds, counts, duration, termination evidence | No unresolved disappearance counted as success |
| Upgrade/revocation | Mixed-version and compromise drills | Explicit recovery or refusal |
| Documentation | `mkdocs build --strict` | Pass |

## Failure And Diagnostic Semantics

Assurance failures must distinguish:

- evidence absent, unsupported, unconfigured, stale, incomplete, invalid, or
  unverifiable;
- build inputs undeclared versus artifact mismatch;
- dependency advisory present versus review not performed;
- inspection finding versus inability to construct a meaningful report;
- local integrity versus external freshness;
- recovery performed versus recovery outcome unresolved;
- authority unknown versus known stale authority;
- backup published versus backup verified versus backup freshness anchored;
- local, replica, and quorum durability evidence; and
- independent review not performed versus performed with unresolved findings.

These should initially remain owning-subsystem report states. Stable public
codes or statuses are added only when a caller needs machine handling and the
Error Design Document is updated.

## Compatibility And Migration

- Evidence schemas evolve independently from the database format and must not
  force a page-format revision merely to add an observation.
- Build/profile manifests identify exactly which database and evidence schema
  versions they can interpret.
- Unknown evidence fields may be ignored only under an accepted compatibility
  rule; unknown critical claims must not default to satisfied.
- Signed evidence requires canonical bytes, algorithm/key identity, rotation,
  revocation, and downgrade behavior before stabilization.
- Historical evidence remains interpretable or explicitly unsupported after an
  upgrade. Re-signing old evidence must not make it contemporaneous.

## Security And Privacy

Evidence can itself be sensitive. Database identity, paths, page counts,
generation rates, witness topology, backup locations, replica lag, actor
identity, dependency versions, and failure history may expose operational
information. Collection and export require authorization, minimization,
redaction, size limits, and explicit retention.

Signatures establish that a holder of a particular signing capability produced
the signed bytes. They do not establish semantic truth, current authority,
freshness, or policy compliance without the corresponding trust context.

## Performance And Resource Bounds

Record evidence-generation CPU, I/O, memory, output size, lock duration, page
coverage, and elapsed time. A bounded summary may reference separately retained
detailed evidence, but the reference requires identity, integrity, availability,
and retention semantics. Evidence collection must not silently trigger recovery,
checkpoint, rekey, VACUUM, replica promotion, or another mutation.

## Risks And Mitigations

| Risk | Impact | Mitigation Or Evidence |
| --- | --- | --- |
| "High assurance" becomes a marketing shortcut | Users infer unearned security or availability | Profile-scoped claims and prominent pre-audit posture |
| Evidence bundle becomes a second truth source | Inspect and storage meanings diverge | Compose owning reports; never redefine facts |
| Missing evidence defaults to success | Unknown state appears trusted | Explicit absence/incomplete/unsupported states |
| Signing is mistaken for truth | Authenticated false or stale statements gain authority | Preserve provenance, freshness, and policy distinctions |
| Dependency inventory becomes stale | Release closure differs from reviewed closure | Generate per artifact and fail mismatched release gates |
| Reproducibility is mistaken for correctness | Repeatable flawed artifact appears approved | Separate provenance, testing, audit, and vulnerability claims |
| Evidence leaks operational or secret data | Assurance tooling expands attack surface | Least authority, redaction, bounds, negative tests |
| Qualification becomes a generic project badge | Claims transfer to untested platforms | Named versioned profile manifests |
| Secure deletion is overclaimed | Old copies survive in flash, snapshots, replicas, or backups | Explicit media/provider limits and cryptographic-erasure scope |
| Assurance process becomes ceremony | Checklists grow without changing decisions | AR-0007 records which evidence materially affects admission |

## Completion Criteria

This long-term plan is complete when:

- [ ] assurance levels and public claims have accepted owners and evidence;
- [ ] named release artifacts have source/build provenance appropriate to their
      accepted profile;
- [ ] authorized tools can export bounded evidence without strengthening source
      observations;
- [ ] supported platform profiles have reproducible fault, upgrade, and
      long-duration evidence;
- [ ] key, privilege, revocation, backup, and destruction limits are explicit;
- [ ] independent review determines the strongest honest supported profile; and
- [ ] unimplemented or unreviewed assurance dimensions remain visible.

The plan may close below A6. A narrow, accurate A3 or A4 profile is preferable
to a broad assurance label unsupported by evidence.

## Parking Or Reopening Criteria

Park a level when there is no concrete consumer, deployment profile, release
artifact, or independent review capacity to exercise it. Reopen when a
dependency incident, customer questionnaire, new host/deployment, release
process, witness/replication feature, platform failure, or audit finding creates
specific pressure.

## Progress Log

### 2026-09-03

- Work completed: opened the dedicated cross-cutting assurance and evidence-
  export plan; separated engineering targets from present product claims.
- Validation: `git diff --check` and `mkdocs build --strict` pass with the plan
  integrated into the cluster plan, roadmaps, status index, and navigation.
- Findings: AR-0010 provenance and bounded evidence export are strategic
  prerequisites, not release polish; evidence composition must preserve the
  independent meanings of integrity, freshness, authority, durability,
  recovery, and provenance.
- Plan changes: added assurance levels A0-A6 and eight gated slices.
- Next slice: inventory current public claims and generate the first repository-
  wide dependency-closure baseline under AR-0010.

### 2026-09-03 -- Slice 0 Baseline Started

- Work completed: defined explicit evidence subjects, nine candidate negative
  and observed states, evidence identity/provenance requirements, and a bounded
  principal-claim inventory with an operator questionnaire.
- Validation: `cargo test --workspace --tests -- --list` completed and listed
  571 tests without executing them; `git diff --check` and
  `mkdocs build --strict` completed successfully.
- Findings: the current repository has strong local executable evidence but no
  general database identity, freshness anchor, hosted authority, native
  replication, release attestation, platform qualification, or independent
  audit. Existing fuzz targets exceed the subset currently scheduled in CI.
- Plan changes: the evidence bundle must structurally match subjects before
  composing individually valid observations.
- Next slice: expand the inventory to every public claim and generate the first
  machine-derived dependency closure under AR-0010.

### 2026-09-03 -- Slice 1 Machine-Derived Baseline

- Work completed: added a deterministic Cargo metadata/lockfile generator, a
  byte-for-byte staleness check, five target-resolution profiles, retained
  package checksums/features/licenses/build-time target flags, a CI caller, and
  a bounded evidence note.
- Validation: `scripts/dependency-provenance.ps1 -Check` matched a 226-package
  baseline. The artifact records 46 packages with build-script targets and 12
  procedural-macro packages without claiming that every target executes in
  every profile.
- Findings: all 220 resolved non-workspace packages have lockfile checksums;
  unsafe review, advisory state, upstream ownership, update ownership, release
  closure, fuzz closure, and platform qualification remain unestablished.
- Plan changes: Slice 1 now separates machine-derived closure facts from human
  risk classification. Aggregate workspace/native/WASM closure generation is
  present; critical transitive review and policy admission remain open.
- Next slice: audit the initial `tosumu-core` critical boundary's transitive
  build/unsafe surface and expand classifications only with retained rationale.

### 2026-09-03 -- Slice 1 Direct-Core Risk Classification

- Work completed: classified all 11 direct normal `tosumu-core` dependencies in
  a human-owned input: nine critical and two elevated. Each entry retains a
  tier floor, concerns, update owner, and rationale; the generated artifact is
  bound to the classification file's SHA-256 identity.
- Validation: the generator rejects unknown or duplicate package identities,
  unknown tiers, assignments below retained floors, and missing rationale,
  owner, or concerns. A below-floor fixture was rejected, the byte-for-byte
  retained check passed after regeneration, and `mkdocs build --strict`
  completed successfully.
- Findings: the direct `zeroize` declaration enables `default` and `alloc`, but
  no direct Tosumu source use was observed. Its presence must not be presented
  as evidence that Tosumu-owned secret buffers are erased. Unfiltered exposure
  reaches 57 packages, including 10 build-script and three procedural-macro
  targets, but workspace feature unification makes that broader than a named
  native core artifact.
- Plan changes: generated resolution facts and human risk judgments remain
  separate inputs. The other 215 packages remain unclassified rather than
  receiving inferred low-risk labels.
- Next slice: trace the nine critical and two elevated dependencies through
  transitive packages, build scripts, procedural macros, features, unsafe
  boundaries, and target-specific participation.

### 2026-09-03 -- Slice 1 Core Target Separation

- Work completed: added package-specific `tosumu-core` resolution profiles for
  Linux, Windows, macOS, and browser WASM using normal/build edges and retained
  enabled features.
- Validation: deterministic regeneration and byte-for-byte checking pass with
  41 Linux, 39 Windows, 41 macOS, and 35 WASM packages. The profiles narrow the
  build-script candidate sets to seven on Linux/macOS and five on Windows/WASM;
  each has one procedural-macro candidate.
- Findings: package/target reachability is stronger than workspace presence but
  still does not prove build-time execution, artifact inclusion, runtime
  reachability, or assurance-critical participation.
- Plan changes: the evidence model now retains those five statements
  separately. Source-level build-script review is next; artifact claims remain
  unavailable.
- Next slice: review the seven narrowed build-script candidates by exact source
  identity and behavior, then review the single proc-macro path.

### 2026-09-03 -- Slice 1 Build-Script Source Review

- Work completed: bound human findings for all seven core target build-script
  candidates to their exact `build.rs` SHA-256 identities and recorded observed
  environment, subprocess, filesystem, cfg, probe, and generation capabilities.
- Validation: the generator requires exactly the current candidate set and
  rejects missing, duplicate, unexpected, incomplete, or hash-mismatched
  reviews. An intentionally replaced script hash was rejected; deterministic
  regeneration, retained checking, `git diff --check`, and
  `mkdocs build --strict` pass.
- Findings: no network or non-rustc native compiler invocation was observed in
  the seven scripts. The review is `attempted_incomplete` because
  `version_check`, referenced compiler-probe inputs, and `thiserror-impl` remain
  separate executable-source subjects.
- Plan changes: source review and controlled-build execution evidence remain
  distinct. A reviewed script is not yet evidence of which branch executed in
  a named build.
- Next slice: close the `version_check` and referenced probe inputs, then review
  the `thiserror-impl` procedural-macro path.

### 2026-09-03 -- Slice 1 Helper And Macro Source Review

- Work completed: hash-bound and reviewed the four-file `version_check` source
  tree, three `proc-macro2` compiler probes, the `thiserror` compiler probe, and
  the 11-file `thiserror-impl` procedural-macro source tree.
- Validation: canonical relative-path/file-hash identities and exact file counts
  are regenerated; missing subjects, path escape, file-set drift, or content
  changes fail the retained check.
- Findings: the helper queries and parses compiler identity, the probes exercise
  compiler features, and the macro parses derive input and generates Rust error
  implementations. No direct network operation was observed in these reviewed
  inputs.
- Plan changes: the named build-script gaps are closed at source level, but the
  macro's `proc-macro2`, `quote`, `syn`, and `unicode-ident` execution closure is
  still unreviewed; state remains `attempted_incomplete`.
- Next slice: review that four-package proc-macro runtime closure, then capture
  controlled-build execution evidence separately from source findings.

## References

- `SECURITY.md`
- `docs/Specifications/Tosumu Software Design Document.md`
- `docs/Specifications/Tosumu Error Design Document.md`
- `docs/Specifications/Tosumu Inspect API Specification.md`
- `docs/ADR/ADR-0001-storage-engine-layer-boundaries.md`
- `docs/ADR/ADR-0002-authenticated-pager-trust-boundary.md`
- `docs/ADR/ADR-0003-source-unit-cohesion-size-pressure-and-decomposition.md`
- `docs/Architectural Reviews/AR-0002-structured-inspection-contract-boundary.md`
- `docs/Architectural Reviews/AR-0005-witness-observer-and-freshness.md`
- `docs/Architectural Reviews/AR-0007-core-change-evidence-and-resilience.md`
- `docs/Architectural Reviews/AR-0008-operation-outcome-closure-and-crash-evidence.md`
- `docs/Architectural Reviews/AR-0010-dependency-trust-and-source-provenance.md`
- `docs/Architectural Reviews/AR-0015-native-replication-scope-authority-and-failure-model.md`
- `docs/Plans/cluster-fault-tolerance-and-replication.md`
- `docs/Plans/main-feature-roadmap.md`
- `docs/Notes/assurance-claim-inventory-v1.md`
- `docs/Notes/dependency-provenance-baseline-v1.md`
