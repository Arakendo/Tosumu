# ADR-0010: Private Format-v3 Cryptographic Mechanism Seams

## Status

Accepted

## Context

Tosumu's format-v3 cryptographic construction is implemented through public
free functions in `tosumu-core::crypto`, with additional direct entropy calls
in pager creation and protector mutation. The pager, snapshots, unlock paths,
and VACUUM rebuild retain or copy raw key arrays according to current behavior.

AR-0016 found credible future pressure for customer-controlled cryptographic
implementations, opaque provider keys, and named deployment profiles. ADR-0003
now encourages a narrow private preparation seam when such variation is
credible, but ADR-0002 requires the pager authentication boundary and exact
format meaning to remain explicit.

The retained Gate C0 evidence includes exact fixed construction vectors and a
file-level matrix spanning encrypted creation, mutation, snapshots, recovery,
protector editing, inspection, and rebuild. That evidence supports structural
preparation. It does not support a public provider API, alternate algorithm,
opaque key contract, or compliance claim.

## Decision

Tosumu will introduce two private, format-v3-preserving mechanism seams.

### Private format-v3 facade

A private concrete facade will own calls to the current RustCrypto mechanics
for current subkey and KEK derivation, page protection, DEK wrap/unwrap, KCV,
and header authentication. It implements exactly one construction selected
structurally by format v3. It is not a public trait, runtime registry, dynamic
plugin interface, or provider stored in the public `Pager` type.

Existing public `tosumu_core::crypto` free functions remain compatibility
wrappers during this change. Their signatures, results, and error behavior do
not change.

### Separate private entropy facade

All current random-byte acquisition for DEKs, page and wrap nonces, salts,
database-local identifiers, and recovery secrets will move behind a separate
private entropy facade. Tosumu continues to choose each purpose and byte
length. Entropy does not choose algorithms, formats, identifiers, or policy.

The default implementation remains `getrandom`. Production behavior does not
gain mutable global selection or a test entropy mode through public APIs.

### Conservation boundary

The extraction must preserve:

- every format-v3 size, offset, algorithm, parameter, label, AAD domain,
  endian encoding, reserved byte, keyslot rule, and MAC coverage rule;
- random-byte purposes and lengths without requiring identical random output;
- the pager as the authenticated data-page plaintext/ciphertext boundary;
- current raw-key representations, copies, and synchronous execution for this
  mechanical slice;
- all existing public APIs and visibility;
- existing structured failures and slot-scanning behavior;
- the currently documented recovery-secret entropy panic until separately
  reviewed error-contract work changes it;
- WAL, snapshot, recovery, backup, inspection, and VACUUM behavior;
- native and browser-WASM compatibility; and
- the current pre-audit security claim ceiling.

The fixed construction vectors and named file-level conservation matrix are
mandatory regression evidence for the extraction. Any semantic repair or
performance-affecting dispatch is separate work.

### Explicit exclusions

This decision does not admit a public provider SPI, dynamic loading, runtime
suite negotiation, opaque/non-exportable key handles, an alternate suite,
format-v3 suite identifiers, provider-specific storage errors, a `fips`
feature flag, or any validation, certification, audit, compliance, or
production-suitability claim.

A future alternate implementation must first exercise the private facade and
may cause it to become a private trait, closed enum, object-safe service, or
other stateful boundary. That shape is deliberately not stabilized here.

## Consequences

- Current crypto mechanics and entropy acquisition gain owned locations that
  can be revised without another repository-wide caller extraction.
- C1 remains intentionally boring: one construction, one default entropy
  source, no runtime selection, and no format change.
- Public callers retain existing low-level functions but cannot select a
  backend.
- Raw key lifetime remains a known limitation rather than becoming a public
  provider requirement.
- Hot-path cost must be measured after extraction.
- A real second implementation is still required before public provider or
  opaque-handle contracts can be judged.

## Alternatives Considered

- **Publish a `CryptoProvider` trait immediately.** Rejected because current
  raw arrays, synchronous calls, and protector coupling have not been tested
  against an independent provider.
- **Store a trait object in every pager.** Rejected for C1 because it adds
  lifetime, thread-safety, allocation, and dispatch commitments that format-v3
  conservation does not require.
- **Select providers with Cargo features.** Rejected because build selection
  must not become durable interpretation or an implied compliance claim.
- **Wait for a customer implementation.** Rejected because the credible
  variation is retained and ADR-0003 permits private preparation while the
  conservation evidence is strong.
- **Private concrete facades.** Accepted as the smallest reversible
  preparation that centralizes ownership without stabilizing provider shape.

## Reopening Triggers

Reopen when an independent backend, HSM/KMS/TPM/keystore, validated module,
asynchronous provider, or non-exportable key requirement exercises the
boundary; when a format revision needs authenticated suite identity; when
provider failures need public diagnostics; or when measured hot-path cost
shows the facade shape is material.

## References

- `ADR-0002-authenticated-pager-trust-boundary.md`
- `ADR-0003-source-unit-cohesion-size-pressure-and-decomposition.md`
- `../Architectural Reviews/AR-0016-cryptographic-provider-seam-and-suite-identity.md`
- `../Plans/cryptographic-provider-seam-and-suite-agility.md`
- `../Notes/crypto-boundary-inventory-v1.md`
- `../Notes/crypto-file-conservation-matrix-v1.md`
- `../Specifications/Tosumu Software Design Document.md`
- `../../SECURITY.md`
