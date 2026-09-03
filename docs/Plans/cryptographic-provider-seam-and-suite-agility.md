# Cryptographic Provider Seam And Suite Agility

| Field | Value |
| --- | --- |
| Status | Gate C1 implemented; Gate C2 oracle implemented with Linux execution pending; all public SPI and format changes remain unadmitted |
| Opened | 2026-09-03 |
| Last updated | 2026-09-03 |
| Owner | Tosumu maintainers |
| Target | `tosumu-core` crypto boundary, future format revision, protector integrations, and assurance profiles |
| Related ADRs | ADR-0001, ADR-0002, ADR-0003, ADR-0009, ADR-0010 |
| Related reviews | AR-0010 dependency trust and source provenance; AR-0016 cryptographic provider seam and suite identity |
| Related CRs | None; future regulated or internally controlled provider requirements are expected consumer pressure |
| Depends on | Existing authenticated pager, format-v3 fixtures, crypto KATs, offline rebuild publication, and assurance evidence model |

## Status

Tosumu currently has one implemented cryptographic construction backed by
RustCrypto crates. The private `FormatV3Crypto` and `SystemEntropy` facades own
the current mechanisms while existing public crypto functions remain wrappers,
but algorithms, key representations, nonce/tag sizes, KDF parameters, and
authentication domains are concrete format semantics rather than provider-
neutral contracts.

This plan admits no provider SPI, alternate suite, format revision, compliance
claim, or migration behavior. Its completed first implementation slice is
deliberately limited to a private seam that reproduces current bytes and errors.
Any public boundary or durable suite identity requires Architectural Review and
an accepted ADR before implementation treats it as settled.

## Purpose

Tosumu should be able to support customers who must use a particular
cryptographic implementation, validated module, hardware boundary, entropy
source, or key-management system without allowing process configuration to
reinterpret existing database bytes.

The plan separates two capabilities that are often incorrectly combined:

1. **backend substitution** implements Tosumu's existing construction through a
   different library while preserving exact format bytes and semantics; and
2. **suite agility** allows a database to use a different versioned
   cryptographic construction, which changes durable meaning and requires
   authenticated identity, compatibility rules, and migration.

It also separates algorithm-suite identity from provider implementation
identity. A database may durably record suite X while runtime evidence records
that provider Y, module version Z, and configuration Q implemented it. Provider
brands, library versions, validation certificates, and deployment policy do
not belong in page-format dispatch bytes unless a future review proves they
are required for interpretation.

## Governing Invariant

> Process configuration selects what may be created; it never changes how
> existing ciphertext is interpreted.

On open, durable authenticated metadata determines the suite. If no admitted
provider implements that suite, open fails explicitly. Tosumu must never try a
different suite, guess from ciphertext, downgrade to plaintext, or reinterpret
an existing file according to the process default.

## Trigger And Evidence

- The software design mentions AES-GCM as an alternative, while format v3 and
  the implementation provide no general suite identifier or dispatch contract.
- Page AEAD, DEK wrapping, KCV construction, HKDF labels, header MAC, Argon2id,
  randomness, and raw 32-byte keys are directly encoded in current code and
  format semantics.
- ADR-0002 requires review when page authentication, AAD, or trust-boundary
  placement changes.
- The assurance inventory showed that dependency presence is not a security
  guarantee; the unused direct `zeroize` declaration is the concrete example.
- AR-0010 now provides machinery for exact provider/dependency source identity,
  target closure, build-time execution review, and explicit unassessed states.
- Future TPM, KMS, HSM, hosted service, backup, and replica-key requirements
  create pressure for provider-owned key handles rather than universal raw-key
  export.
- A FIPS-oriented request would concern more than algorithm selection: module
  identity, configuration, entropy, key handling, platform, build provenance,
  operational environment, and retained evidence all participate in the
  eventual claim.

## Current State

### Implemented construction

| Concern | Current format-v3 behavior |
| --- | --- |
| Page protection | ChaCha20-Poly1305; random 12-byte nonce; 16-byte tag |
| Page AAD | page number, page version, and page type |
| DEK | Random 32-byte value |
| Subkeys | HKDF-SHA256 with fixed Tosumu v1 labels |
| Header authentication | HMAC-SHA256 |
| Passphrase protector | Argon2id with serialized m/t/p/version parameters |
| Recovery protector | Base32 secret decoded and expanded through HKDF-SHA256 |
| DEK wrapping | ChaCha20-Poly1305 with slot/dek/kind AAD |
| KCV | Fixed-input ChaCha20-Poly1305 construction |
| Entropy | Direct `getrandom` calls |
| In-memory keys | Raw `[u8; 32]` arrays stored by pager/snapshot paths |

### Current coupling

The pager and unlock paths call free crypto functions and retain raw derived
keys. Page frame offsets assume the current nonce and tag sizes. Keyslot fields
assume the current wrapped-DEK length and KDF representation. No durable field
names a general suite, and page AAD does not bind one.

This means a backend that exactly reproduces the current construction can be
introduced privately without changing bytes. A suite with different
algorithms, authentication domains, key sizes, nonce/tag layouts, or protector
parameters cannot be selected safely without explicit format work.

## Capability Model

```text
CryptoProfile (policy and admitted combination)
├── FormatCryptoSuite (durable interpretation)
│   ├── page protect / unprotect
│   ├── DEK wrap / unwrap and key check
│   ├── subkey derivation
│   └── header authentication
├── ProtectorProvider (credential and external-key lifecycle)
│   ├── passphrase / recovery / keyfile
│   └── future TPM / KMS / HSM
└── EntropyProvider (approved randomness path)
    ├── key material
    ├── salts
    └── nonces
```

`CryptoProfile` is the policy unit. It constrains which suite, protector, and
entropy combinations are admitted for a named use, but it must not force those
mechanisms into one monolithic trait.

### Suite identity versus provider identity

`CryptoSuiteId` identifies the byte-level construction necessary to interpret
the database. It is stable, versioned, and authenticated. Two implementations
may implement the same suite only if they produce and accept identical bytes
and failure semantics for every specified input.

`CryptoProviderId` identifies an implementation for diagnostics and evidence:
library/module identity, version, build, configuration, platform, and any
external validation reference. It is not used to reinterpret durable bytes.

### Key ownership

The long-term provider boundary should permit opaque provider-owned key handles.
Raw key import/export is an optional capability, not the base contract. A
provider may keep keys in a validated module, hardware device, operating-system
keystore, or isolated service while still implementing the suite.

The first private seam may temporarily wrap the current raw arrays to conserve
behavior. That temporary representation must not silently become the stable
public SPI.

## Goals

- Insert a private backend seam with zero byte, error, and public API changes.
- Prove provider independence using exact fixtures and a real second caller or
  implementation pressure before stabilizing traits.
- Support provider-owned key lifecycle without teaching the pager provider-
  specific vocabulary.
- Define authenticated, downgrade-resistant suite identity before alternate
  suite implementation.
- Make unsupported suite/provider/profile combinations fail explicitly.
- Provide full-rewrite suite migration with atomic publication and verified
  reopen rather than metadata-only reinterpretation.
- Feed provider/module/configuration identity into assurance evidence without
  converting implementation identity into file-format meaning.

## Non-Goals

- A `fips` Boolean, Cargo feature, header flag, or generic compliance badge.
- Claiming that approved algorithms alone make Tosumu or a deployment compliant.
- Runtime guessing, opportunistic fallback, or multi-suite trial decryption.
- Per-page mixtures of suites within one ordinary database generation.
- Allowing applications to bypass ADR-0002's authenticated pager boundary.
- Replacing protector policy, entropy policy, and format crypto with one giant
  provider object merely because one vendor supplies all three.
- Making raw key export mandatory.
- In-place suite conversion or merely rewrapping the DEK when page protection
  or authentication domains change.
- Adding provider-specific errors or foreign types to Tosumu's durable public
  vocabulary.
- Retrofitting format v3 with ambiguous reserved-byte interpretations.

## Ownership And Dependency Boundary

### `tosumu-core`

Owns suite semantics, authenticated storage behavior, provider-neutral key
capabilities, typed failure mapping, format dispatch, and migration mechanics.
It must not own claims about a customer's regulatory regime or deployment.

### Provider adapter

Owns calls into a specific cryptographic library, module, device, or service;
provider key handles; provider initialization; and mapping foreign failures
into bounded Tosumu details. It must not redefine suite bytes or pager trust.

### Host or consumer

Owns provider availability, credentials, module configuration, authorization,
deployment policy, and acceptance of a named profile. It may constrain database
creation but may not override the suite recorded by an existing database.

### Assurance layer

Owns evidence that a particular artifact and runtime used provider Y/module Z
under configuration Q. It keeps algorithm, implementation, build, validation,
and deployment observations separate.

## Public Contract Candidates

Names remain provisional pending independent caller evidence:

```rust
pub struct CryptoSuiteId(/* stable owned identifier */);
pub struct CryptoProviderId(/* diagnostic/evidence identity */);

pub trait FormatCryptoSuite {
    type PageKey;
    type HeaderKey;
    type WrappingKey;

    fn suite_id(&self) -> CryptoSuiteId;
    // Bounded page, wrapping, derivation, and authentication operations.
}

pub trait EntropyProvider {
    fn fill(&self, purpose: EntropyPurpose, output: &mut [u8]) -> Result<()>;
}

pub trait ProtectorProvider {
    type KeyHandle;
    // Create/open/wrap operations with explicit export capabilities.
}
```

The public design must resolve object safety, thread safety, provider lifetime,
key-handle cloning, destruction, redaction, asynchronous external services,
error causality, cancellation, and capability discovery. These sketches are not
accepted APIs.

## Development Gates And Slices

### Gate C0: Architecture Admission

**Objective:** Reconcile provider ownership with ADR-0002 and define what must
be format-stable before code creates a reusable seam.

- [x] Open an Architectural Review covering suite identity, provider identity,
      key ownership, entropy, protector separation, failure behavior, and
      format compatibility.
- [x] Inventory every current crypto operation and pager-held key lifetime.
- [x] Select a private concrete format-v3 facade and separate entropy facade;
      defer trait, enum, object-safe, and stateful-provider shape until C2.
- [x] Define exact byte- and error-conservation fixtures.
- [x] Confirm that format v3 receives no alternate interpretation.
- [x] Confirm that C1 adds no dependency; feed any later provider dependency
      through AR-0010.

**Exit gate:** an ADR admits only the private byte-preserving seam, or parks it.
No suite identifier is allocated at this gate.

### Phase 1 / Slice C1: Private Byte-Preserving Backend Seam

**Objective:** Route current operations through a private implementation seam
without changing format bytes, public APIs, or supported behavior.

- [x] Move current ChaCha/HKDF/HMAC/Argon2/getrandom mechanics behind private,
      purpose-specific interfaces.
- [x] Keep the pager as the sole authenticated crossing point.
- [x] Preserve exact frame, keyslot, header, KCV, KDF, AAD, and error behavior.
- [x] Use a default RustCrypto implementation selected structurally, not by
      mutable process configuration.
- [x] Preserve the observed raw-key lifecycle at C1 and defer reliable,
      key-free lifecycle instrumentation to C3, where provider-owned handles
      can expose creation/use/destruction events without exposing key bytes.
- [x] Keep provider types out of public storage traits and format modules.

**Acceptance:** existing databases and reviewed fixtures are byte-compatible;
fixed-input KATs, corruption, wrong-key, recovery, hostile-input, WAL, snapshot,
and rebuild tests are unchanged in meaning.

### Phase 1 / Slice C2: Provider-Independence Evidence

**Objective:** Demonstrate that the seam describes Tosumu's construction rather
than merely renaming RustCrypto calls.

- [x] Add an independent backend implementing the exact current suite, or a
      narrowly limited deterministic test backend where a second real
      implementation is unavailable.
- [x] If deterministic testing is used, forbid it from production construction
      and document why its nonce/key behavior is not a valid real suite.
- [x] Run cross-backend known-answer, encrypt/decrypt, wrap/unwrap, header-MAC,
      wrong-key, tamper, and error-equivalence tests.
- [x] Prove cross-provider interoperability for identical suite bytes.
- [ ] Record unsupported target/provider combinations explicitly.

**Exit gate:** independent pressure has revealed enough contract shape to decide
whether a public provider SPI is justified.

#### C2 admission slices

- [x] Assess real and test-only independent implementation candidates without
      adding them to the product dependency graph.
- [x] C2a: define a versioned deterministic oracle corpus covering every
      format-v3 construction and normalized negative outcome.
- [x] Admit the independent oracle toolchain and pinned dependency closure
      through AR-0010.
- [x] C2b: implement the oracle outside `tosumu-core` and compare both
      executors over the retained corpus.

### Phase 2 / Slice C3: Provider-Owned Key Lifecycle

**Objective:** Permit opaque key ownership without weakening pager trust or
requiring universal raw-key export.

- [ ] Define non-serializable key-handle capabilities and ownership semantics.
- [ ] Decide cloning, sharing, thread-safety, session, expiration, revocation,
      and destruction behavior.
- [ ] Separate create/import/export capability from ordinary encrypt/decrypt use.
- [ ] Define behavior for unavailable, locked, revoked, rate-limited, or remote
      provider state.
- [ ] Prevent debug, error, inspection, evidence, and panic paths from exposing
      keys or sensitive provider material.
- [ ] Exercise a mock opaque provider and one independent consumer crate.
- [ ] Add key-free lifecycle instrumentation that observes handle creation,
      authorized use, cloning/sharing where admitted, revocation, and
      destruction without serializing key bytes or provider secrets.

**Exit gate:** raw `[u8; 32]` is an implementation detail of providers that
permit it, not a mandatory public-provider contract.

### Gate C4: Authenticated Suite Identity And Format Revision

**Objective:** Admit suite agility as an explicit on-disk compatibility change.

- [ ] Open or update the architecture review and accept a format ADR.
- [ ] Define a stable `CryptoSuiteId` namespace and ownership process.
- [ ] Specify every algorithm, size, label, AAD field, protector encoding, and
      error-relevant behavior comprising a suite.
- [ ] Place suite identity in metadata available for dispatch before page
      decryption and authenticate it against substitution/downgrade.
- [ ] Bind suite identity into page and wrapping authentication domains where
      required by the threat model.
- [ ] Specify unknown, unavailable, forbidden, and retired suite diagnostics.
- [ ] Define backup, WAL, snapshot, inspection, replication, and recovery
      behavior for the new format.
- [ ] Preserve deterministic rejection by older readers.

**Exit gate:** a new format revision has complete fixtures and compatibility
rules before any alternate suite writes production-shaped data.

### Phase 3 / Slice C5: First Alternate Suite

**Objective:** Prove suite dispatch with one fully specified alternate suite,
not necessarily a compliance profile.

- [ ] Select the suite only after requirements and dependency review.
- [ ] Implement through an independently owned provider adapter.
- [ ] Add normative KATs and negative vectors for every construction.
- [ ] Prove existing format-v3 databases still select their original suite.
- [ ] Prove process defaults affect creation policy only.
- [ ] Expose suite identity through bounded inspection and evidence APIs.
- [ ] Reject unsupported provider, target, feature, and key capability
      combinations before mutation.

**Exit gate:** two suites coexist without ambiguity, downgrade, fallback, or
provider identity leaking into durable interpretation.

### Phase 4 / Slice C6: Explicit Suite Migration

**Objective:** Convert a database through verified full rewrite and atomic
publication.

- [ ] Build on ADR-0009's offline rebuild/publication mechanism.
- [ ] Require source-suite provider access and destination-profile admission.
- [ ] Decrypt/authenticate each source page and re-encrypt every destination
      page under a fresh destination generation and keys.
- [ ] Recreate protectors according to explicit destination policy; never assume
      an opaque key can be exported or converted.
- [ ] Verify the complete destination before publication.
- [ ] Retain source/destination identities, counts, verification results, and
      publication outcome without leaking keys.
- [ ] Define crash-before-publication, ambiguous publication, cleanup, backup,
      rollback, and downgrade behavior.

**Exit gate:** migration is a recoverable rewrite, never a metadata flag change
or partial mixed-suite state.

### Phase 5 / Slice C7: Reviewed Deployment Profile

**Objective:** Support a named customer or assurance profile with evidence
appropriate to its real requirements.

- [ ] Define required algorithms, provider/module identity, configuration,
      entropy, key handling, targets, builds, deployment boundary, and
      operational procedures.
- [ ] Bind artifact and runtime provider observations into the assurance model.
- [ ] Establish which facts are supplied by Tosumu, the provider, the host, the
      customer, and an independent assessor.
- [ ] Exercise startup self-tests, provider health, failure injection, key
      revocation, backup/restore, upgrade, and recovery behavior.
- [ ] Complete dependency/source provenance and native-boundary review.
- [ ] Obtain any required independent validation before making the named claim.

**Exit gate:** only the named artifact/provider/configuration/deployment profile
may carry the reviewed claim. Generic Tosumu remains pre-audit unless separately
reviewed.

## Failure And Diagnostic Semantics

Candidate failure phenomena include:

- durable suite unknown to this reader;
- suite known but no provider installed;
- provider installed but not admitted by creation/runtime policy;
- provider unavailable, unhealthy, locked, revoked, or misconfigured;
- required opaque-key capability unsupported;
- entropy unavailable;
- provider authentication failure;
- provider internal/external failure with bounded source detail;
- migration source or destination provider unavailable;
- provider evidence unavailable, stale, incomplete, or unverifiable.

Authentication failure remains a storage-integrity phenomenon. Provider brand
and foreign error values must not replace Tosumu-owned stable error identity.
Provider fallback after authentication failure is forbidden.

## Compatibility And Migration

### Format v3

The private seam must preserve format v3 exactly. It may not write a suite ID
into reserved bytes, change AAD, change key derivation labels, alter error
classification, or allow alternate construction.

### Future suite-aware format

Suite identity is a compatibility boundary. Unknown suites fail explicitly.
Older readers reject the new format through existing version rules. New readers
must not infer suites from nonce/tag lengths or trial decryption.

### WAL and retained generations

WAL frames remain meaningful only under their database generation and suite.
Migration cannot combine source-suite WAL with destination-suite pages. Retained
snapshots, backups, and replica bootstrap artifacts preserve suite identity.

### Migration

Changing suite is equivalent to changing the cryptographic format of every
protected page. It requires full rewrite, verification, and atomic publication.
Keyslot-only edits are insufficient.

## Validation Matrix

| Claim | Evidence | Required result |
| --- | --- | --- |
| Private seam conserves format | Existing fixtures plus byte-for-byte vectors | No byte change for fixed inputs |
| Provider interoperability | Cross-backend KAT corpus | Identical bytes and typed outcomes |
| Trust boundary | Pager/recovery/inspection tests | No plaintext bypass |
| No fallback | Unknown/unavailable/tampered fixtures | One explicit failure; no alternate trial |
| Key ownership | Opaque mock provider | No required raw export or diagnostic leak |
| Suite identity | Format fixtures and downgrade attacks | Authenticated selection; substitution rejected |
| Migration | Crash corpus and verified reopen | Source preserved until atomic publication |
| WAL/snapshot | Recovery and retained-generation fixtures | No cross-suite ambiguity |
| Provider evidence | Evidence bundle/profile tests | Suite and implementation identities separate |
| Native/WASM | Target-specific closure and builds | Unsupported combinations explicit |
| Dependency provenance | AR-0010 generator and reviews | Exact closure and review state retained |
| Documentation | `mkdocs build --strict` | Pass |

## Security And Assurance Rules

- “Implements suite X” is not “approved provider,” “validated module,” or
  “compliant deployment.”
- Provider/module evidence must name artifact, version, target, configuration,
  and observation method.
- Suite identity is authenticated data; provider identity is provenance data.
- No provider may return unauthenticated plaintext on failure.
- No automatic fallback follows authentication, initialization, or policy
  failure.
- Key and nonce purposes must be explicit at the entropy boundary.
- Key handles and sensitive buffers must have defined destruction and redaction
  behavior; dependency names such as `zeroize` are not evidence of that result.
- External KMS/HSM availability changes operational behavior and must have
  bounded timeout, retry, cancellation, and recovery semantics.
- A validated provider may still be composed incorrectly. Tosumu's construction,
  format, and integration require their own evidence.

## Dependency And Platform Policy

Every provider adapter is a distinct AR-0010 subject. Review must include:

- exact source/module identity and license;
- enabled features and native/build/proc-macro closure;
- unsafe and FFI boundary;
- dynamic/static linking and redistribution consequences;
- supported Rust, OS, architecture, and filesystem profiles;
- WASM/mobile availability or explicit exclusion;
- initialization and self-test behavior;
- entropy and key custody;
- update, vulnerability, revocation, and incident owner; and
- effect on reproducible and offline builds.

## Performance And Resource Bounds

Provider dispatch must be measured separately from algorithm cost, external
device/service latency, and migration cost. Page operations require bounded
allocations and must not introduce an unbounded remote round trip per page.
Opaque providers may require sessions or batching; those mechanisms cannot
weaken transaction, cancellation, or authentication semantics.

Migration requires free-space estimation, progress reporting, interruption
handling, and the same publication bounds as offline VACUUM. Performance
measurements do not become provider-neutral guarantees.

## Risks And Mitigations

| Risk | Impact | Mitigation or gate |
| --- | --- | --- |
| Giant provider trait combines unrelated policy | Unreviewable contract and fake portability | Separate suite, protector, entropy, and profile roles |
| Raw bytes become mandatory SPI | HSM/KMS boundary defeated | Opaque handles; raw export optional |
| Process default reinterprets files | Silent corruption or lockout | Durable authenticated suite selection |
| Suite ID is not authenticated | Downgrade/substitution attack | Bind it into header and relevant AAD |
| Provider fallback hides failure | Authentication bypass or ambiguous diagnostics | Fail closed; never trial alternatives |
| Alternate suite squeezed into v3 | Old readers misinterpret reserved bytes | New format revision and fixtures |
| Migration treated as rewrap | Pages remain under old construction | Full verified rewrite/publication |
| Deterministic test provider leaks into production | Nonce/key catastrophe | Structural test-only availability and explicit limits |
| Provider name becomes compliance claim | Misleading security posture | Separate suite, implementation, validation, and deployment evidence |
| Native provider breaks WASM/mobile/MSRV | Hidden target regression | Target-specific dependencies and CI profiles |
| External provider stalls page path | Availability and transaction failures | Sessions, bounds, cancellation, health diagnostics |
| Multiple suites multiply recovery states | Untestable failure matrix | One suite per database generation; fault corpus per suite/migration |

## Roadmap Placement

This is a cross-cutting track rather than a new sequential database milestone:

- C0-C2 may begin after the remaining MVP+10 closure because they preserve
  format v3 and reduce future coupling.
- C3 should precede public TPM/KMS/HSM protector stabilization and inform mobile
  or hosted key custody.
- C4 must be reconciled with any future format revision before MVP+12 evidence,
  backup identity, or MVP+15 replication begins depending on suite identity.
- C5-C6 follow accepted suite-format design and use offline rebuild publication.
- C7 belongs to the high-assurance profile track and cannot be inferred from
  feature completion.

Replication protocols should carry suite identity as database metadata, not
negotiate a new interpretation of already committed bytes. Backup/restore and
evidence subjects must also preserve it.

## Completion Criteria

The plan is complete only when:

- [ ] private backend substitution conserves format v3 exactly;
- [ ] public provider/key contracts, if any, are admitted by ADR and exercised
      by an independent caller;
- [ ] suite and provider identities are structurally distinct;
- [ ] opaque key lifecycle and unsupported capabilities are explicit;
- [ ] alternate suite identity is authenticated and downgrade-resistant;
- [ ] migration is a verified atomic full rewrite;
- [ ] inspection, errors, WAL, backup, replication, and evidence preserve suite
      identity correctly;
- [ ] no generic compliance claim exceeds a named reviewed profile; and
- [ ] all remaining unsupported provider/target/profile combinations are
      retained rather than implied successful.

## Parking And Reopening Criteria

Park after C0 if byte-preserving abstraction adds complexity without independent
provider pressure. Reopen or advance when:

- a customer requires a named crypto implementation or deployment profile;
- an HSM, KMS, TPM, mobile keystore, or validated module needs opaque handles;
- a current primitive or provider becomes unavailable or unsuitable;
- a format revision creates a natural suite-identity migration point;
- replication/backup design requires explicit suite portability; or
- independent review recommends provider isolation.

## Progress Log

### 2026-09-03 -- Plan Opened

- Work completed: inventoried the current construction and created the phased
  provider/suite/migration/profile plan.
- Validation: `git diff --check` and `mkdocs build --strict` pass with the plan
  integrated into both roadmaps, the plan/status indexes, navigation, and the
  cluster and assurance plans.
- Findings: a private exact-byte seam is separable from suite agility; alternate
  suites require authenticated format identity and full rewrite migration.
- Plan changes: suite identity, provider implementation identity, and deployment
  validation are separate subjects. Raw key export is optional, not foundational.
- Next slice: open the C0 Architectural Review after current MVP+10 closure and
  capture exact format/error conservation fixtures before implementation.

### 2026-09-03 -- Gate C0 Review And Vector Baseline

- Work completed: opened AR-0016; retained the operation, entropy, key-copy,
  call-path, error, and test-gap inventory; added exact fixed construction
  vectors without changing production crypto APIs or randomness.
- Validation: the focused vector test, `cargo fmt --all -- --check`,
  `cargo clippy --workspace --all-targets -- -D warnings`,
  `cargo test --workspace --tests`, `git diff --check`, and
  `mkdocs build --strict` pass. An `--all-targets` run completed the existing
  B-tree/SQLite benchmark and part of the concurrency benchmark before that
  redundant benchmark execution was stopped; it is not recorded as a pass.
- Findings: entropy is called outside `crypto.rs`; raw keys are copied through
  pager, snapshots, unlock, and rebuild; most prior `kat_*` tests were
  behavioral rather than exact vectors; recovery-secret entropy failure still
  panics and is deliberately not repaired inside the baseline.
- Plan changes: Gate C0 admits conservation work only. File-level fixtures and
  a reviewed private contract still precede an ADR or provider seam.
- Next slice: retain file-level create/mutate/recover/protector/rebuild
  conservation evidence, then propose the smallest private format-v3 contract.

### 2026-09-03 -- Gate C0 File Matrix And Contract Candidate

- Work completed: retained the named file-level conservation matrix and mapped
  its passing executable evidence across encrypted lifecycle, snapshots,
  recovery, protectors, inspection, and rebuild.
- Findings: the smallest C1 preparation is a private concrete format-v3 facade
  plus a separate entropy facade. It requires neither a public trait nor a
  provider carried in the public pager type.
- Plan changes: file-level conservation and contract-candidate prerequisites
  are complete. Public crypto-wrapper disposition and formal ADR acceptance
  remain open.
- Next slice: decide the existing public free-function disposition and prepare
  the narrow C1 ADR without repairing the recorded entropy-error inconsistency.

### 2026-09-03 -- Gate C0 Accepted

- Work completed: ADR-0010 accepts the private concrete format-v3 facade and
  separate private entropy facade; existing public crypto functions remain
  compatibility wrappers.
- Claim boundary: no provider SPI, opaque key handle, alternate suite, format
  identifier, runtime negotiation, or compliance claim was admitted.
- Next slice: implement C1 as a behavior-preserving structural change and rerun
  the exact vectors, file matrix, WASM build, and performance observation.

### 2026-09-03 -- C1 Entropy Facade

- Work completed: centralized DEK, nonce, salt, database-identifier, and
  recovery-secret random acquisition behind the purpose-named private
  `SystemEntropy` facade.
- Conservation: public APIs and random byte lengths are unchanged;
  fallible calls still return `RngFailed`, and recovery-secret generation
  deliberately retains its recorded panic on entropy failure.
- Validation: the exact construction vector, encrypted create/open fixture,
  formatting, and strict workspace Clippy pass.
- Next slice: extract the private concrete format-v3 cryptographic facade and
  retain existing public free functions as wrappers.

### 2026-09-03 -- C1 Format-v3 Facade

- Work completed: moved the existing HKDF-SHA256, ChaCha20-Poly1305,
  Argon2id, KCV, HMAC-SHA256, and recovery-KEK mechanics behind the private
  concrete `FormatV3Crypto` facade. Existing free functions remain wrappers,
  and the pager retains no provider object or runtime selection state.
- Conservation: the exact construction vector and all 23 focused crypto tests
  pass. The full file-level matrix is exercised through the workspace suite;
  native formatting and linting and the browser-WASM adapter build also pass.
  The WASM check additionally repaired the non-native `WriterGuard` stub so it
  preserves the clone contract already required by pager ownership.
- Performance observation: the extraction adds only static concrete calls and
  no allocation or runtime dispatch. The existing `lookup/plain/tosumu`
  Criterion path nevertheless ran because ADR-0010 requires post-extraction
  measurement: its 95% interval was 41.216-42.670 us, with Criterion reporting
  a 3.75-7.11% time reduction against the retained local baseline. This is one
  local observation, not a general throughput claim or causal attribution.
- C1 closure: reliable key-lifecycle instrumentation cannot observe freely
  copied raw arrays without changing the representation ADR-0010 conserves.
  Call counters would observe facade invocation, not key lifetime. The
  instrumentation requirement therefore moves to C3 alongside opaque handles;
  C1 is complete. C2 provider-independence evidence and every public provider,
  opaque-key, alternate-suite, or compliance claim remain unadmitted.

### 2026-09-03 -- C2 Independent Backend Candidate Assessment

- Work completed: compared OpenSSL, libsodium, a `ring` composition, a Go
  oracle, and a deterministic fixture backend against the exact format-v3
  construction and current build boundaries.
- Findings: no candidate should enter `tosumu-core` merely to provide test
  evidence. A deterministic fake can exercise interface shape but cannot prove
  independent cryptographic execution. A separately built Go oracle is the
  preferred complete candidate; it can express every current construction
  without becoming a runtime backend, but its absent toolchain and module
  closure require explicit admission.
- Gate change: split C2 into C2a, a versioned non-secret deterministic corpus,
  and C2b, its independently implemented executor. No dependency, private
  provider trait, public SPI, or format change is admitted by this assessment.
- Next slice: define the C2a request/response corpus and generate it from
  Tosumu's retained vectors before selecting or installing the executor.

### 2026-09-03 -- C2a Deterministic Oracle Corpus

- Work completed: retained schema version 1 of the language-neutral format-v3
  oracle corpus with seven positive constructions and seven negative mutations.
  It uses explicit byte encoding, endian-qualified integer fields, fixed input
  recipes, expected bytes or digests, and normalized failure categories.
- Validation: PowerShell JSON parsing confirms schema 1, format 3, seven
  positive cases, seven negative cases, and no duplicate IDs. Tosumu's focused
  `gate_c0_fixed_construction_vectors` test still passes against the source
  values represented by the corpus.
- Claim boundary: the corpus is an input to future independent evidence. Its
  agreement with Tosumu's existing vector does not itself provide independence.
- Next slice: admit a pinned Go toolchain and `golang.org/x/crypto` module
  closure through AR-0010 before implementing C2b.

### 2026-09-03 -- C2b Independent Go Executor

- Work completed: added a separately built Go command that consumes the shared
  corpus and independently implements every positive and negative case using
  standard-library SHA-256/HMAC/HKDF plus pinned `x/crypto` Argon2id and
  ChaCha20-Poly1305. It is outside the Cargo workspace and release artifacts.
- Provenance: Go 1.26.8, `x/crypto` 0.56.0, and transitive `x/sys` 0.47.0 are
  exact; module sums and the seven-package non-standard compile closure are
  retained in `crypto-c2-oracle-provenance-v1.md`.
- Validation: the official Windows amd64 Go archive matched its published
  SHA-256; `go test ./...` passed; the command independently reported seven
  positive and seven negative cases passed without emitting vector material.
  A Linux amd64 test binary cross-compiled successfully but was not executed.
- Claim boundary: this is cross-implementation corpus evidence, not a runtime
  provider, complete source audit, platform qualification, or compliance claim.
- Next slice: obtain the first Linux CI execution, record supported oracle
  targets, then use the pressure it revealed to decide whether any production
  private trait is justified. The current evidence does not require one.

## References

- `docs/ADR/ADR-0001-storage-engine-layer-boundaries.md`
- `docs/ADR/ADR-0002-authenticated-pager-trust-boundary.md`
- `docs/ADR/ADR-0009-offline-vacuum-rebuild-publication.md`
- `docs/ADR/ADR-0010-private-format-v3-cryptographic-mechanism-seams.md`
- `docs/Architectural Reviews/AR-0010-dependency-trust-and-source-provenance.md`
- `docs/Architectural Reviews/AR-0016-cryptographic-provider-seam-and-suite-identity.md`
- `docs/Notes/crypto-boundary-inventory-v1.md`
- `docs/Notes/crypto-c2-independent-backend-assessment-v1.md`
- `docs/Notes/crypto-c2-oracle-provenance-v1.md`
- `tools/crypto-oracle/testdata/format-v3-v1.json`
- `docs/Notes/crypto-file-conservation-matrix-v1.md`
- `docs/Plans/high-assurance-engineering-and-evidence-export.md`
- `docs/Plans/cluster-fault-tolerance-and-replication.md`
- `docs/Specifications/Tosumu Software Design Document.md`, sections 4-8
- `docs/Specifications/Tosumu Error Design Document.md`
- `SECURITY.md`
- `crates/tosumu-core/src/crypto.rs`
- `crates/tosumu-core/src/format.rs`
- `crates/tosumu-core/src/pager.rs`
