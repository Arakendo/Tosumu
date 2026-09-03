# AR-0016: Cryptographic Provider Seam And Suite Identity

| Field | Value |
| --- | --- |
| Status | Accepted through ADR-0010 for the private format-v3 and entropy seams only; later provider/suite phases remain incubating |
| Opened | 2026-09-03 |
| Last reviewed | 2026-09-03 |
| Scope | Authenticated pager / format-v3 cryptography / protector and entropy boundaries |
| Trigger | Customer-controlled or validated cryptographic implementations require a substitution boundary without allowing runtime configuration to reinterpret existing databases |
| Related ADRs | ADR-0001, ADR-0002, ADR-0003, ADR-0009 |
| Related evidence | `crates/tosumu-core/src/crypto.rs`, pager create/open/protector/rebuild paths, crypto and hostile-input tests, `docs/Notes/crypto-boundary-inventory-v1.md`, and the cryptographic provider and suite-agility plan |

## Architectural Question

Should Tosumu admit a private cryptographic backend seam that exactly preserves
the current format-v3 construction, and which responsibilities must remain
separate before any provider contract, opaque key handle, or alternate durable
suite can be admitted?

## Context

Tosumu has one implemented construction. RustCrypto types are localized in
`crypto.rs`, but the effective boundary is wider: pager creation obtains
entropy directly, unlock and protector mutation own raw KEKs and DEKs, the
pager stores derived raw keys for its lifetime, and VACUUM rebuild copies those
keys into a staging context.

The `crypto` module is public and exposes concrete byte-array functions. That
is an existing source API, but it is not a provider SPI: callers cannot select
an implementation, supply opaque keys, control entropy, or change the durable
suite. Adding one large public trait now would stabilize the current
incidental split and make later HSM/KMS or format work harder.

Format v3 has no general cryptographic-suite identifier. Its nonce and tag
sizes, HKDF labels, Argon2 encoding, page and wrap AAD, KCV construction,
header MAC, keyslot layout, and structured failures are already durable
meaning. Process configuration therefore cannot safely select a different
construction when opening an existing file.

This review addresses Gate C0 only. It does not approve an alternate suite,
format revision, compliance claim, public provider SPI, dynamic plugin loader,
or raw-key-export requirement.

## Governing Invariant

> Process configuration selects what may be created; it never changes how
> existing ciphertext is interpreted.

For format v3, the format version selects the one existing construction. If a
future authenticated format revision names a suite, that durable identifier
selects interpretation. Provider and deployment policy may permit or reject
the selected suite, but may not substitute another one.

## Evidence

### Current operation boundary

- `crypto.rs` retains the existing public free functions as compatibility
  wrappers. The private concrete `FormatV3Crypto` facade owns HKDF-SHA256
  subkeys, ChaCha20-Poly1305 page protection and DEK wrapping, Argon2id
  passphrase derivation, the deterministic KCV, HMAC-SHA256 header
  authentication, and recovery-secret derivation.
- The private `SystemEntropy` facade owns Tosumu's direct `getrandom` calls for
  DEKs, nonces, passphrase salts, `dek_id` seeds, and recovery secrets.
- The pager and snapshot path call page encryption/decryption and header-MAC
  functions directly. Protector editing calls derivation, KCV, wrap/unwrap,
  and header-MAC functions directly.
- WAL stores already encoded page frames. Recovery and retained snapshots use
  the pager's keys to authenticate frames; WAL does not own plaintext crypto
  semantics.
- Offline VACUUM copies the authenticated page-0 protector state plus raw page
  and header-MAC keys into a crate-private `RebuildContext`, then produces new
  page frames with fresh nonces.

### Current key and secret lifetime

- `Pager` retains `page_key: [u8; 32]` and optional
  `header_mac_key: [u8; 32]` for the handle lifetime.
- DEKs and KEKs are ordinary stack byte arrays in create, unlock, protector,
  and rekey paths. `RebuildContext` copies active derived keys.
- Passphrases and recovery strings are borrowed or owned ordinary strings;
  keyfile reads temporarily allocate a `Vec<u8>` before copying into a KEK.
- The audit subkey is derived during creation/open-related derivation but is
  not retained or used by the pager.
- The direct `zeroize` dependency is not called by Tosumu source. No current
  evidence establishes reliable erasure of Tosumu-owned secret buffers.
- Sentinel databases store the DEK in page 0 and provide authentication but no
  meaningful confidentiality. Provider abstraction cannot strengthen that
  claim.

### Current behavioral evidence

- Crypto unit tests cover round trips, AAD tampering, wrong keys, keyslot
  binding, deterministic behavior, header-MAC tampering, Argon2 salt
  separation, and recovery-secret parsing.
- Pager tests cover encrypted create/open, protector lifecycle, hostile page-0
  and page-frame changes, rekey crash behavior, recovery, snapshot use, and
  VACUUM protector continuity.
- The functions named `kat_*` mostly assert round trips, difference, or
  determinism. They are useful behavioral tests, but most do not retain exact
  expected bytes from a fixed vector.
- Random page and wrap nonces prevent byte-for-byte encryption assertions
  through the current public helpers. Fixed-nonce decryption fixtures and
  deterministic primitive vectors are still required before extraction.

### Existing error behavior

- entropy failure generally maps to `RngFailed`, but
  `generate_recovery_secret` currently panics if `getrandom` fails;
- page authentication failure maps to `AuthFailed { pgno: Some(_) }`;
- header authentication failure maps to `AuthFailed { pgno: None }`;
- wrap/KCV/recovery-secret rejection maps to `WrongKey`;
- encryption failure maps to `EncryptFailed`;
- invalid Argon2 parameters and keyfile length use `InvalidArgument`; and
- passphrase slot scanning suppresses individual derivation, KCV, and unwrap
  failures before returning `WrongKey` if no slot succeeds.

The private seam must conserve these outcomes initially, including the known
recovery-secret RNG inconsistency. Repairing that inconsistency is desirable
but is separate semantic work.

### Missing evidence

- exact fixed vectors for every deterministic construction and fixed-nonce
  page/wrap decoding;
- a complete byte comparison for database creation, page mutation, recovery,
  protector editing, and rebuild across the seam;
- an independent backend implementing the exact same format-v3 construction;
- evidence that a useful opaque-key provider can satisfy pager, snapshot,
  recovery, backup, and rebuild lifetimes;
- provider failure taxonomy and retry/cancellation behavior;
- a durable authenticated suite identifier and downgrade rules; and
- any independently reviewed provider, module configuration, or deployment
  profile.

## Ownership And Dependency Analysis

### Format crypto suite

`tosumu-core` owns the exact bytes, authentication domains, sizes, algorithms,
and error normalization required to interpret a format. A backend may execute
those operations but cannot redefine them. The pager remains the sole data-
page plaintext/ciphertext trust boundary under ADR-0002.

### Entropy provider

Entropy supplies unpredictable bytes for purposes and lengths selected by
Tosumu. It does not choose file formats, algorithms, identifiers, or fallback
policy. Moving all direct `getrandom` calls behind one private boundary is
necessary for deterministic conservation evidence and future controlled
entropy sources, but it need not be the same interface as page cryptography.

### Protector provider

Protector mechanisms obtain or use authority to unwrap the database key. A
future protector may use a passphrase, keyfile, OS keystore, TPM, HSM, KMS, or
remote service. Protector identity and lifecycle are not automatically part of
the data-page suite. A provider-owned handle must not imply that raw export is
available.

### Policy profile

A process or deployment profile says which providers and suites may create or
open data. It may refuse a known suite. It cannot supply the interpretation of
existing ciphertext or turn provider presence into a validation/compliance
claim.

### Dependency direction

The intended direction is:

```text
pager / page-0 protector orchestration
              |
              v
format-owned operation contract + error normalization
              |
              v
private backend implementation / entropy / protector adapter
              |
              v
cryptographic library, module, device, or service
```

Provider-specific errors, handles, module names, and library types must not
enter the durable format or the existing public pager/storage contracts.

## Candidate First Private Contract

The smallest Gate C1 contract is a private format-v3 facade, not a public
provider trait and not a provider stored in every `Pager`.

```text
existing public crypto free functions
                 |
                 v
private format_v3 facade
  - derive current subkeys
  - protect/open current page frames
  - derive current passphrase/recovery KEKs
  - wrap/unwrap current DEK
  - compute/verify current KCV and header MAC
                 |
                 v
RustCrypto implementation details

pager/protector orchestration
                 |
                 v
private entropy facade
  - fill purpose-sized random bytes
                 |
                 v
getrandom implementation
```

For C1:

- existing public free functions remain compatibility wrappers;
- the facade is a private concrete capability boundary selected structurally
  at compile time, so pager public types gain no generic parameter, trait
  object, provider identity, allocation, or mutable global;
- entropy is a separate private capability because salts, identifiers,
  recovery secrets, page nonces, and wrap nonces cross crypto and pager paths;
- current raw key arrays and synchronous local calls are conserved rather than
  advertised as the future public provider contract;
- all provider/library errors normalize to the existing Tosumu errors at the
  facade boundary; and
- no suite identifier, dispatch byte, alternate algorithm, or configuration
  switch is introduced.

This shape creates an owned place to replace mechanics later without deciding
opaque handles or dynamic dispatch prematurely. Gate C2 must use an independent
implementation to decide whether the facade should become a private trait,
closed enum, object-safe service, or another stateful boundary. That later
decision may revise the private facade without compatibility cost.

## Alternatives Considered

### Alternative A: One public `CryptoProvider` trait now

- Benefits: immediately visible extension point and simple marketing story.
- Costs: stabilizes raw byte-array keys, synchronous local execution, current
  protector coupling, and one construction before an independent caller has
  exercised the boundary.
- Failure mode: a nominally generic trait cannot represent real provider-owned
  keys or changes file interpretation according to runtime choice.

### Alternative B: Private exact-construction backend plus separate entropy
and protector boundaries

- Benefits: permits behavior-preserving extraction and independent backend
  evidence while keeping durable format meaning under Tosumu ownership.
- Costs: initially provides no customer-facing extension API and requires
  careful conservation fixtures.
- Failure mode: the private interface merely mirrors every free function and
  becomes a public compatibility boundary without independent pressure.

### Alternative C: Cargo feature selects the cryptographic implementation

- Benefits: small runtime surface and familiar Rust configuration.
- Costs: build configuration becomes implicit deployment policy; artifacts may
  differ without carrying usable evidence.
- Failure mode: a feature such as `fips` is mistaken for format compatibility,
  module validation, or compliant operation.

### Alternative D: Defer all seam work until an alternate provider exists

- Benefits: no speculative abstraction.
- Costs: provider requirements arrive while randomness, raw keys, protector
  policy, and pager calls remain intertwined.
- Failure mode: customer integration pressure forces a public seam before
  exact behavior has been captured.

## Findings

- Backend substitution and suite agility are separate architectural changes.
- ADR-0003 now explicitly encourages a narrow private seam when an accepted
  plan, review, consumer, or fault-injection need establishes credible future
  variation. That supports doing the format-v3 preparation before a customer
  provider arrives, but does not bypass ADR-0002 or this review's conservation
  gate.
- A private exact-construction seam is plausible without changing format v3,
  but the seam includes more than `crypto.rs` because entropy and key lifetime
  cross pager and protector paths.
- The smallest useful first boundary is private and format-v3-specific. It
  should accept or obtain typed key capabilities internally and normalize all
  results into existing Tosumu errors.
- Entropy and protector mechanisms remain separate responsibilities even if
  the initial RustCrypto implementation wires them together.
- Existing test breadth is good, but exact conservation evidence is too weak
  to approve extraction yet.
- Opaque key handles cannot be honestly designed until an independent provider
  exercises clone, thread, lifetime, recovery, rebuild, and failure behavior.
- Alternate suites require authenticated durable identity and a new format
  decision. They cannot be smuggled into format-v3 reserved bytes.

## Disposition

**Accepted through ADR-0010 for Gate C1 only.** The retained vectors, file-level
matrix, and contract analysis admit a private concrete format-v3 facade and a
separate private entropy facade. Existing public crypto functions remain
compatibility wrappers. No public provider SPI, opaque-key contract, alternate
suite, format identifier, or compliance claim is accepted.

No public provider API, alternate suite, format identifier, feature-selected
reinterpretation, or compliance label is accepted by this cycle.

## Required Follow-Up

- [x] Inventory current crypto operations, entropy call sites, key lifetimes,
      durable construction inputs, and normalized failures.
- [x] Add exact fixed vectors for deterministic constructions and fixed-nonce
      page/wrap inputs without changing production randomness.
- [x] Retain file-level conservation fixtures spanning create, mutation,
      recovery, protector editing, snapshot reads, and VACUUM rebuild.
- [x] Propose the smallest private format-v3 backend and separate entropy
      boundary against the executable baseline.
- [x] Decide whether current public free functions remain supported wrappers,
      become crate-private in a breaking pre-alpha revision, or form a distinct
      low-level API.
- [ ] Review the `generate_recovery_secret` entropy panic as separate error-
      contract work; do not repair it during mechanical extraction.
- [x] Create or revise an ADR before implementing the seam.
- [ ] Reopen separately for opaque key handles, public provider SPI, durable
      suite identity, format migration, or a named deployment profile.

## Reopening Triggers

- exact vectors and file-level conservation fixtures are retained;
- an independent exact-format backend prototype exposes a missing operation;
- an HSM, KMS, TPM, keystore, or validated module supplies a concrete opaque-
  handle and failure contract;
- a dependency or primitive becomes unavailable or unsuitable;
- a format revision creates a suite-identity opportunity; or
- replication/backup requirements make provider portability concrete.

## Review History

### Cycle 1 -- 2026-09-03

- Status entering review: Proposed
- New evidence: current operation, entropy, key-lifetime, error, and test
  inventory retained in this review and its companion note.
- Findings: a private format-v3 seam is plausible, but exact-byte conservation
  and independent-provider evidence are incomplete; entropy and protector
  ownership must remain separate.
- Disposition: Incubating; admit baseline work only.
- Resulting ADR or documentation change: none; ADR required after conservation
  evidence and private-contract review.

### Cycle 2 -- 2026-09-03

- Status entering review: Incubating
- New evidence: `gate_c0_fixed_construction_vectors` retains exact outputs for
  the three HKDF labels, KCV, header MAC, reduced-cost Argon2id fixture,
  recovery KEK, fixed-nonce DEK wrap, and a full fixed-nonce page frame hash,
  prefix, suffix, and successful decode.
- Findings: deterministic suite meaning can be pinned without injecting test
  entropy into production APIs. These are regression/conservation vectors
  generated from the current implementation, not an independent cryptographic
  validation or provider-interoperability result.
- Disposition: Incubating; file-level conservation and the proposed private
  contract remain prerequisites to an ADR.
- Resulting ADR or documentation change: none.

### Cycle 3 -- 2026-09-03

- Status entering review: Incubating
- New evidence: ADR-0003 was amended to cover preparatory private seams for
  credible variation, with capability-shaped ownership, minimum visibility,
  structural selection, conservation, and public-stability guardrails.
- Findings: the crypto-provider pressure qualifies for early private seam
  preparation, but authentication-boundary and format consequences still keep
  AR-0016 and ADR-0002 authoritative for its exact shape.
- Disposition: Incubating; proceed with file-level conservation and contract
  definition rather than waiting for a customer-specific provider.
- Resulting ADR or documentation change: ADR-0003 amended; no crypto seam is
  admitted yet.

### Cycle 4 -- 2026-09-03

- Status entering review: Incubating
- New evidence: the named file-level conservation matrix relates existing
  encrypted create/open, mutation, snapshots, recovery, protector, inspection,
  and rebuild tests to the exact construction vectors and records the passing
  workspace-test invocation.
- Findings: C1 does not require a trait object or pager-stored provider. A
  private concrete format-v3 facade plus a separate entropy facade creates the
  preparation point while conserving current public wrappers and raw-key
  behavior.
- Disposition: Incubating; the contract is concrete enough for ADR review.
  Public crypto-function disposition and the recovery-secret entropy error
  remain explicit follow-up rather than hidden seam behavior.
- Resulting ADR or documentation change: no ADR yet.

### Cycle 5 -- 2026-09-03

- Status entering review: Incubating
- New evidence: exact construction vectors, the named file-level conservation
  matrix, ADR-0003's preparatory-seam rule, and the concrete C1 contract.
- Findings: C1 can preserve public wrappers and avoid pager-stored provider
  state, runtime selection, new dependencies, or format changes.
- Disposition: Accepted through ADR-0010 for the two private C1 facades only;
  all later provider, key-handle, and suite questions remain incubating.
- Resulting ADR or documentation change: ADR-0010.

### Cycle 6 -- 2026-09-03

- Status entering review: Accepted through ADR-0010 for C1 only.
- New evidence: `SystemEntropy` and the concrete `FormatV3Crypto` facade now
  own the admitted mechanisms while the existing public crypto functions stay
  wrappers. Exact vectors, focused crypto tests, the workspace file-behavior
  suite, native lint/format checks, and the browser-WASM build pass.
- Findings: the extraction requires neither pager-stored provider state nor
  runtime dispatch or allocation and preserves the v3 interpretation. The
  required post-extraction `lookup/plain/tosumu` observation measured
  41.216-42.670 us and reported no regression against the retained local
  baseline. The WASM build exposed and prompted a narrow clone-contract repair
  in the pre-existing non-native writer-gate stub.
- Disposition: ADR-0010 is implemented for the two private facades. C1 remains
  open only for key-free lifecycle instrumentation; public provider, opaque-key,
  alternate-suite, and compliance questions remain incubating.
- Resulting ADR or documentation change: implementation and retained plan and
  inventory updates; no new architectural decision.

### Cycle 7 -- 2026-09-03

- Status entering review: ADR-0010 implemented; C1 retained one lifecycle-
  instrumentation checklist item.
- New evidence: C1 deliberately preserves freely copied raw key arrays. The
  concrete facade is stateless, so facade-call counters cannot observe key
  creation, copying, residence, revocation, or destruction.
- Findings: claiming key-lifecycle evidence from operation counters would
  strengthen observation beyond the implementation. Reliable key-free
  instrumentation belongs with C3 provider-owned handles, where lifecycle
  events have an actual owned subject.
- Disposition: C1 complete. Move lifecycle instrumentation to C3 without
  admitting opaque handles or revising ADR-0010's conservation boundary.
- Resulting ADR or documentation change: crypto plan checklist corrected; no
  new architectural decision.

### Cycle 8 -- 2026-09-03

- Status entering review: C1 complete; C2 unadmitted.
- New evidence: the retained C2 candidate assessment compares OpenSSL 3.2+,
  libsodium 1.0.19+, a `ring`/Argon2 composition, a Go oracle, and a fixture
  fake against every deterministic format-v3 operation and the repository's
  native, WASM, MSRV, and provenance constraints.
- Findings: a fake backend can pressure interface shape but cannot establish
  independent cryptographic execution. Native-library candidates create a
  product-build burden if added only for evidence. A separate Go oracle can
  express the complete suite without entering release artifacts, but its
  toolchain and pinned module closure are not yet admitted or available in the
  current environment.
- Disposition: admit C2a design work for a versioned, non-secret deterministic
  oracle corpus. Do not admit an executor dependency, production backend,
  private provider trait, public SPI, or format change yet.
- Resulting ADR or documentation change: add the C2 candidate assessment and
  split the plan into C2a corpus and C2b executor gates; no ADR.

### Cycle 9 -- 2026-09-03

- Status entering review: C2a design admitted; executor unadmitted.
- New evidence: schema version 1 of the format-v3 oracle corpus parses as seven
  positive construction cases and seven unique negative mutations. The
  existing focused Tosumu vector passes for the transcribed source values.
- Findings: compact deterministic recipes avoid retaining page-sized secrets or
  opaque generated blobs while still fixing page length, byte generation,
  layout, AAD, digest, prefix, and suffix. Normalized failures are evidence
  vocabulary and do not change Tosumu's public error contract.
- Disposition: C2a complete. The corpus remains Tosumu-derived rather than
  independent evidence until C2b executes it through a separately implemented
  oracle.
- Resulting ADR or documentation change: retain
  `tools/crypto-oracle/testdata/format-v3-v1.json`; no dependency or ADR
  admitted.

### Cycle 10 -- 2026-09-03

- Status entering review: C2a complete; C2b executor unadmitted.
- New evidence: a separately built Go executor independently matches all seven
  positive corpus cases and rejects all seven negative mutations. Its exact
  toolchain, two-module closure, module sums, and seven non-standard compiled
  packages are retained under AR-0010 Cycle 11.
- Findings: the corpus contract was sufficient without changing Tosumu's
  production facade or introducing a trait, provider state, runtime dispatch,
  or product dependency. Independent execution therefore falsifies the concern
  that the vectors only restate RustCrypto output, but does not qualify either
  implementation or prove arbitrary-provider substitutability.
- Disposition: C2b implementation and CI wiring complete; the first retained
  Linux CI result and explicit target-support record remain open. Do not add a
  production private trait merely to mirror the oracle; reopen contract shape
  only when a runtime provider or opaque-key requirement exercises it.
- Resulting ADR or documentation change: evidence-only Go oracle and provenance
  note; no public SPI, format change, or new ADR.

### Cycle 11 -- 2026-09-03

- Status entering review: C2b implemented with hosted execution pending.
- New evidence: the independent oracle's `go test ./...` passed on a GitHub-
  hosted Ubuntu runner at commit `abdc241` in CI run `33812169906`, job
  `100836236809`. The retained target record now distinguishes local Windows
  execution, hosted Linux execution, cross-compilation, and unqualified targets.
- Findings: independent Linux execution required no production provider trait,
  pager-held provider state, raw-key contract change, alternate suite, or format
  change. The current facade is therefore an adequate private preparatory seam;
  abstracting it further now would stabilize conjecture rather than evidence.
- Disposition: C2 complete. Keep C3 incubating until a concrete opaque-key
  provider or independent runtime consumer exercises lifecycle and failure
  semantics. C4 and later gates remain unadmitted.
- Resulting ADR or documentation change: target/evidence records and roadmap
  status updated; no new ADR.
