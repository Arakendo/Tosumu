# Crypto C2 Oracle Provenance v1

| Field | Value |
| --- | --- |
| Status | Admitted evidence-only tool baseline; not a product dependency audit |
| Observed | 2026-09-03 |
| Scope | `tools/crypto-oracle` build and execution inputs |
| Owner | AR-0010 and AR-0016 Gate C2b |

The independent format-v3 oracle is deliberately outside the Cargo workspace
and Tosumu release artifacts. Its dependencies support one reproducible
evidence job; they are not admitted as `tosumu-core` providers or product
runtime dependencies.

## Pinned Inputs

| Input | Identity | Role |
| --- | --- | --- |
| Go toolchain | `go1.26.8`; Windows amd64 archive SHA-256 `b92c3b2adae85a11ba71fe7216daf0d84e82af4c8ab6c5625807f28622043a59` | Local retained execution |
| `golang.org/x/crypto` | `v0.56.0`; Go module sum `h1:GUh5Ii4J5jtcseSMiRqr1jXCNHoxjeV9Fmekc2oLy6Y=` | Argon2id and ChaCha20-Poly1305 |
| `golang.org/x/sys` | `v0.47.0`; Go module sum `h1:o7XGOvZQCADBQQ4Y7VNq2dRWQR7JmOUW8Kxx4ZsNgWs=` | Transitive CPU feature support |
| GitHub toolchain action | `actions/setup-go@v7` | CI toolchain acquisition from `go.mod` |

The exact `go.mod` and `go.sum` are retained adjacent to the tool. Go 1.26.8
is a supported Go release as observed on 2026-09-03. The selected `x/crypto`
release requires Go 1.26 and is licensed BSD-3-Clause. The indirect `x/sys`
module is also pinned rather than resolved through an open range.

## Compiled Non-Standard Package Closure

`go list -deps` reports seven non-standard dependency packages for the command:

- `golang.org/x/crypto/argon2`;
- `golang.org/x/crypto/blake2b`;
- `golang.org/x/crypto/chacha20`;
- `golang.org/x/crypto/chacha20poly1305`;
- `golang.org/x/crypto/internal/alias`;
- `golang.org/x/crypto/internal/poly1305`; and
- `golang.org/x/sys/cpu`.

Standard-library JSON, Base32, binary encoding, SHA-256, HMAC, and HKDF packages
complete the execution closure. Package reachability and module checksums do
not establish complete source review, compiler trust, side-channel properties,
or platform qualification.

## Admission Boundary

The two pinned modules and Go toolchain are admitted only for an offline,
deterministic, non-secret oracle that:

- consumes the versioned public test corpus;
- performs no network access at execution time after module acquisition;
- emits only pass/fail and case counts;
- cannot create, open, or mutate a Tosumu database;
- is not linked into Rust artifacts; and
- makes no FIPS, validation, certification, or production-suitability claim.

Changing the toolchain, module version, imported package closure, corpus schema,
or execution role reopens this evidence-only admission under AR-0010.

## Target Evidence

| Target | Observation | Qualification boundary |
| --- | --- | --- |
| Windows amd64 | Built and executed all positive, negative, fail-closed schema, and mutation tests with Go 1.26.8 | Local evidence-only oracle target |
| Linux amd64 | `go test -c` cross-compilation succeeded; GitHub Actions job is wired but has no retained result in this change | Compile evidence only until CI executes |
| Other Go targets | Not exercised | Unqualified, not silently supported |
| Browser WASM | Not required or exercised | The oracle is offline evidence tooling and is not part of Tosumu's browser artifact |

## Retained Execution

On 2026-09-03, the checksum-verified Windows amd64 Go 1.26.8 archive built the
tool with workspace-local build and module caches. `go test ./...` passed, and
the command reported seven positive and seven negative format-v3 cases passed.
The same source cross-compiled its test binary for Linux amd64; that binary was
not executed locally.
