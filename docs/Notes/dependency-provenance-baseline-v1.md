# Dependency Provenance Baseline v1

| Field | Value |
| --- | --- |
| Authority | Evidence note under AR-0010; not an accepted dependency policy |
| Subject | The exact `Cargo.lock` identity recorded in the adjacent JSON artifact |
| Observation state | `observed_finding` |
| Captured | 2026-09-03 |
| Generator | `scripts/dependency-provenance.ps1` |
| Retained artifact | `dependency-provenance-baseline-v1.json` |

## Purpose

This note is the first machine-derived repository-wide dependency closure for
Assurance Slice 1. It answers which packages Cargo resolves, how they
participate in the workspace, which features are selected, and which packages
declare build-script or procedural-macro targets. It deliberately does not
claim that the dependency implementations have been audited or that any target
has been qualified.

The JSON subject is the SHA-256 identity of `Cargo.lock`. A baseline generated
for a different lockfile is evidence about a different subject and must not be
composed as if it described the current closure.

## Reproduction And Staleness Check

Generate the artifact from the repository root:

```powershell
pwsh -NoProfile -File scripts/dependency-provenance.ps1
```

Check the retained artifact without replacing it:

```powershell
pwsh -NoProfile -File scripts/dependency-provenance.ps1 -Check
```

The check fails if the file is absent or differs byte-for-byte from normalized
Cargo metadata and the current lockfile. The generated JSON contains no time,
host path, username, or local registry-cache path.

## Observed Closure

The first baseline contains 226 packages and five resolution profiles:

| Profile | Target filter | Packages | Normal-role packages | Build-role packages | Development-role packages |
| --- | --- | ---: | ---: | ---: | ---: |
| Workspace, unfiltered | None | 226 | 146 | 7 | 159 |
| Linux x86-64 | `x86_64-unknown-linux-gnu` | 174 | 120 | 7 | 111 |
| Windows x86-64 | `x86_64-pc-windows-msvc` | 173 | 118 | 7 | 112 |
| macOS x86-64 | `x86_64-apple-darwin` | 172 | 118 | 7 | 110 |
| Browser WASM | `wasm32-unknown-unknown` | 167 | 113 | 7 | 117 |

Roles overlap. For example, a package may be reachable through both normal and
development paths. `workspace` identifies the six workspace members and is not
another dependency kind.

All 220 non-workspace packages in this resolution have a retained Cargo
lockfile checksum. That establishes exact registry archive resolution. It does
not establish source review, repository ownership, absence of malicious code,
or offline availability.

The catalog identifies 46 packages declaring a Cargo `custom-build` target and
12 declaring a `proc-macro` target. These are candidate review surfaces, not a
finding that all 58 execute in every profile or that their behavior is unsafe.
The profile membership and enabled features in the JSON provide the narrower
context needed for review.

## Initial Critical Boundary

`tosumu-core` directly resolves these normal dependencies in the unfiltered
profile:

- authentication, encryption, and key derivation: `argon2 0.5.3`,
  `chacha20poly1305 0.10.1`, `hkdf 0.12.4`, `hmac 0.12.1`, and
  `sha2 0.10.9`;
- randomness and secret lifecycle: `getrandom 0.2.17` and `zeroize 1.8.2`;
- physical representation and integrity mechanics: `crc32fast 1.5.0` and
  `data-encoding 2.11.0`;
- native writer admission: target-gated `fs4 1.1.0`; and
- private error implementation: `thiserror 2.0.18`.

This is an initial ownership boundary, not the completed risk classification.
Transitive packages inherit review relevance from the path by which they enter
this set. Parser, public-vocabulary, unsafe, and update-owner classifications
still require retained human judgment. The generator therefore records
`unsafe_review_state: not_assessed` for every package instead of inferring
safety from metadata.

## Explicit Limitations

- Target-filtered `cargo metadata` is resolution evidence, not compilation or
  platform-qualification evidence.
- Package license strings are upstream metadata observations, not an
  independent legal conclusion.
- A declared build script or procedural macro says that executable build-time
  code exists; metadata does not explain what that code does.
- Advisory status, maintainer identity, upstream repository revision, unsafe
  implementation review, and update ownership are not yet represented.
- The unfiltered workspace includes benchmark and development closures. It is
  not a release-artifact SBOM.
- The WASM profile describes workspace resolution under a target filter. It
  does not claim that every workspace member is intended or able to compile for
  WASM.
- Lockfile checksums do not provide offline source custody or reproducible
  binary artifacts.

## Next Review Work

1. Add retained risk classifications and rationale without allowing the
   generator to manufacture human judgments.
2. Audit the transitive closure of the initial `tosumu-core` critical boundary,
   starting with build scripts, procedural macros, unsafe boundaries, and
   enabled features.
3. Separate release, fuzz, and supported-target profiles from the broad
   workspace closure.
4. Assign dependency-addition, update, advisory-response, and exception owners.
5. Return the evidence to AR-0010 to decide whether a risk-tiered policy is
   ready for an ADR.

## References

- `Cargo.toml`
- `Cargo.lock`
- `.github/workflows/ci.yml`
- `docs/Architectural Reviews/AR-0010-dependency-trust-and-source-provenance.md`
- `docs/Notes/assurance-claim-inventory-v1.md`
- `docs/Plans/high-assurance-engineering-and-evidence-export.md`
