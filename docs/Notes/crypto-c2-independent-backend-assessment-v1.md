# Crypto C2 Independent Backend Assessment v1

| Field | Value |
| --- | --- |
| Status | Retained candidate assessment; no dependency or provider admitted |
| Observed | 2026-09-03 |
| Scope | Independent execution of the exact format-v3 construction |
| Owner | AR-0016 Gate C2 |

This note asks how Tosumu can obtain evidence that its private format-v3 seam
describes a construction rather than merely renaming one RustCrypto call graph.
It does not select a production provider, change format bytes, establish
provider interchangeability, or support a compliance claim.

## Evidence Required

An independent implementation must consume the same explicit inputs and match
the retained outcomes for:

- HKDF-SHA256 extract-and-expand with Tosumu's existing labels;
- RFC 8439 ChaCha20-Poly1305 page and DEK-wrap ciphertext and tags;
- Argon2id version 0x13 with stored memory, iteration, and parallelism values;
- the fixed KCV construction;
- HMAC-SHA256 header authentication over the exact covered regions;
- recovery-key HKDF; and
- successful cross-opening plus wrong-key, changed-AAD, changed-nonce,
  tamper, and malformed-input rejection.

Entropy is not part of deterministic cross-provider comparison. The harness
must supply fixed nonces and salts; a production backend must never acquire
deterministic entropy from this test path.

## Candidate Assessment

| Candidate | Coverage | Boundary cost | Current disposition |
| --- | --- | --- | --- |
| OpenSSL 3.2+ | ChaCha20-Poly1305, HKDF, HMAC, and parameterized Argon2id are available through EVP APIs | Native library provisioning, version/provider availability, Rust binding coverage, build scripts, unsafe FFI, and target qualification | Credible real backend; not admitted |
| libsodium 1.0.19+ | ChaCha20-Poly1305-IETF, HKDF-SHA256, HMAC-SHA256, and Argon2id exist in one native library | The stable high-level password API does not expose Argon2 parallelism; using embedded low-level Argon2 symbols would rely on a weaker compatibility boundary. Native acquisition/build behavior also needs review | Useful partial oracle; not an exact general format-v3 backend through its stable API |
| `ring` plus a separate Argon2 implementation | ChaCha20-Poly1305, HKDF-SHA256, and HMAC-SHA256 are covered; `ring` does not expose Argon2 | Two-provider composition, native/assembly build review, a larger evidence closure, and no single module boundary | Technically plausible test oracle; not preferred |
| Go standard library plus `golang.org/x/crypto` | HMAC-SHA256 plus parameterized Argon2id, ChaCha20-Poly1305, and HKDF-SHA256 can express the complete deterministic suite | Adds a separate toolchain and pinned module closure; Go is not installed in the current development environment | Preferred independent offline oracle candidate, pending toolchain and dependency admission |
| Fixture-replay or deterministic fake backend | Can prove that a contract routes fixed operations and failures without production entropy | Does not independently compute cryptography or prove general interoperability | Permitted only as interface pressure; insufficient to complete C2 |

## Findings

1. A fake backend is not independent cryptographic evidence. It may reveal
   ownership, input, output, and failure shape, but cannot complete C2.
2. Adding OpenSSL or libsodium directly to `tosumu-core` merely for tests would
   burden native builds and blur an evidence oracle with a supported provider.
3. A separate offline oracle keeps product code Rust-native and prevents an
   evidence dependency from entering release artifacts. Its toolchain, module
   versions, source identities, and invocation still require provenance.
4. The deterministic backend contract should receive explicit nonces and
   salts. Entropy remains a separate Tosumu-owned mechanism and must not be
   hidden inside the format-suite implementation.
5. Cross-provider byte agreement is necessary but insufficient. Both
   implementations could share a mistaken interpretation, so retained
   standards vectors and negative cases remain separate evidence.

## Preferred Gate C2 Shape

Split C2 into two bounded slices:

1. **C2a -- oracle contract and corpus:** define a versioned, non-secret JSON
   request/response corpus for deterministic format-v3 operations. The corpus
   owns explicit byte encodings, fixed nonces/salts, expected results, and
   normalized failure categories. It is evidence tooling, not a public Tosumu
   API.
2. **C2b -- independent executor:** implement the corpus in a separately built
   Go oracle after its pinned toolchain and `x/crypto` closure pass AR-0010.
   Compare its generated results with Tosumu in CI or a retained reproducible
   evidence job. Do not link it into `tosumu-core` or ship it as a provider.

### C2a corpus contract

`tools/crypto-oracle/testdata/format-v3-v1.json` is the first corpus instance. It uses
lowercase hexadecimal bytes, decimal JSON integers whose field names identify
little-endian encoding, and compact deterministic recipes for the 4,056-byte
page plaintext and page-zero MAC regions. An executor must reject an unknown
schema or format version rather than guessing.

The corpus contains seven positive constructions and seven referenced negative
mutations. Positive cases cover subkeys, KCV, header MAC, Argon2id, recovery
HKDF, DEK wrapping/opening, and page protection/opening. Negative cases cover
wrong keys, wrap-AAD changes, page-AAD changes, ciphertext and keyslot tamper,
and malformed recovery text. Corpus outputs were transcribed from the retained
Tosumu Gate C0 vector, which remains the executable source observation until an
independent executor exists.

Only after C2b should AR-0016 judge whether the private production facade needs
a trait, closed enum, stateful service, or no further abstraction. A public
provider API remains outside this gate.

## Explicit Non-Claims

- OpenSSL presence would not mean its FIPS provider can execute format v3;
  format v3 uses ChaCha20-Poly1305 and this assessment makes no FIPS claim.
- A Go oracle would not become a supported runtime backend.
- Passing fixed vectors would not qualify platforms, entropy, key erasure,
  module configuration, or deployment policy.
- No candidate may reinterpret format v3 or substitute algorithms according to
  process configuration.

## Sources Consulted

- [OpenSSL EVP Argon2 documentation](https://docs.openssl.org/3.3/man7/EVP_KDF-ARGON2/)
- [OpenSSL ChaCha20-Poly1305 documentation](https://docs.openssl.org/3.4/man3/EVP_chacha20/)
- [libsodium HKDF documentation](https://doc.libsodium.org/key_derivation/hkdf)
- [libsodium password-hashing API header](https://github.com/jedisct1/libsodium/blob/master/src/libsodium/include/sodium/crypto_pwhash.h)
- [libsodium low-level Argon2 header](https://github.com/jedisct1/libsodium/blob/master/src/libsodium/crypto_pwhash/argon2/argon2.h)
- [`ring` AEAD documentation](https://docs.rs/ring/latest/ring/aead/)
- [`ring` HKDF documentation](https://docs.rs/ring/latest/ring/hkdf/)
- [Go `x/crypto` package index](https://pkg.go.dev/golang.org/x/crypto)
