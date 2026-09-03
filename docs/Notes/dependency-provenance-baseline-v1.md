# Dependency Provenance Baseline v1

| Field | Value |
| --- | --- |
| Authority | Evidence note under AR-0010; not an accepted dependency policy |
| Subject | Exact `Cargo.lock` and provisional risk-classification identities recorded in the generated JSON artifact |
| Observation state | `observed_finding` |
| Captured | 2026-09-03 |
| Generator | `scripts/dependency-provenance.ps1` |
| Retained artifacts | Generated baseline; human-owned risk classification and build-script review JSON |

## Purpose

This note is the first machine-derived repository-wide dependency closure for
Assurance Slice 1. It answers which packages Cargo resolves, how they
participate in the workspace, which features are selected, and which packages
declare build-script or procedural-macro targets. It deliberately does not
claim that the dependency implementations have been audited or that any target
has been qualified.

The JSON subject contains the SHA-256 identities of `Cargo.lock` and the
human-owned risk-classification input. A baseline generated for a different
lockfile or classification is evidence about a different subject and must not
be composed as if it described the current closure and review state.

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

### Package-specific target profiles

The generator also invokes `cargo tree` for `tosumu-core` alone with normal and
build edges. This avoids treating every feature unified elsewhere in the
workspace as part of the core library's selected target closure:

| Core profile | Reachable packages | Build-script candidates | Procedural-macro candidates |
| --- | ---: | ---: | ---: |
| Linux x86-64 | 41 | 7 | 1 |
| Windows x86-64 | 39 | 5 | 1 |
| macOS x86-64 | 41 | 7 | 1 |
| Browser WASM | 35 | 5 | 1 |

These profiles establish code reachable for the selected package, target, and
feature resolution. They do not collapse five different statements:

1. source is present in the dependency graph;
2. code is executed while building Tosumu;
3. code is compiled into a resulting artifact;
4. code is reachable for a selected target and feature configuration; and
5. code participates in an assurance-critical runtime path.

Cargo metadata and `cargo tree` establish the first and support the fourth.
Build-script and procedural-macro flags identify candidates for the second.
Artifact inspection and source/runtime review remain necessary for the other
claims.

### Build-script source review

All seven build-script candidates in the four core target profiles now have a
human finding bound to the SHA-256 of their exact `build.rs`. The generated
baseline rejects missing candidates, unexpected packages, duplicate reviews,
or changed script hashes.

The review remains `attempted_incomplete`, not `observed_pass`:

- `crc32fast` and `quote` query the selected rustc version and emit cfgs;
- `generic-array` delegates compiler-version detection to the still-unreviewed
  `version_check` build dependency;
- `libc` reads target/environment inputs, queries rustc, and contains external
  platform probes for `freebsd-version` and `emcc`;
- `proc-macro2` runs rustc feature probes and manages probe outputs under
  `OUT_DIR`;
- `rustix` selects platform backends using target/configuration inputs, source
  presence, and rustc compile probes; and
- `thiserror` generates Rust source under `OUT_DIR` and compiles a referenced
  probe source.

No network operation or non-rustc native compiler invocation was observed in
these seven exact scripts. That statement is limited to the reviewed
`build.rs` files. It does not cover helper libraries, referenced probe sources,
the procedural macro, or arbitrary behavior elsewhere in their packages.

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

### Provisional direct-dependency classification

The companion human-owned JSON classifies all 11 direct normal dependencies of
`tosumu-core`: nine as `critical` and two as `elevated`. Each entry has a tier
floor, named concerns, update owner, and rationale. The generator rejects an
unknown package identity, duplicate entry, unknown tier, tier below its floor,
or missing rationale/owner/concerns. Any lowering of a tier or its floor also
requires retained rationale in a new AR-0010 review cycle.

This does not classify the remaining 215 packages. In particular, it does not
automatically lower development packages, transitive dependencies, build
scripts, or procedural macros just because they are indirect.

The generator separately traces machine-derived exposure from those 11 roots
without assigning human risk tiers to their dependencies. The unfiltered
workspace graph reaches 57 packages: 48 inherit exposure from at least one
critical root and nine only from elevated roots. Ten expose build-script
targets and three expose procedural-macro targets. Every transitive exposure
remains `not_assessed`.

That unfiltered result is intentionally conservative and is not a native core
release closure. Workspace feature unification can connect the browser-enabled
`getrandom` path to `wasm-bindgen`, for example. The package-specific profiles
remove that particular ambiguity, but artifact inspection is still required
before reachable packages are described as code compiled into or executed by a
particular release.

One concrete discrepancy is now retained: `zeroize 1.8.2` participates in the
encryption closure and is declared directly by `tosumu-core`, enabling its
`default` and `alloc` features, but no direct use from Tosumu source was found.
The classification therefore treats the secret-lifecycle boundary as critical
while leaving removal, explicit use, or feature minimization for focused
review. Dependency presence is not evidence that Tosumu-owned secret buffers
are erased.

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
- `docs/Notes/dependency-risk-classification-v1.json`
- `docs/Notes/dependency-build-script-review-v1.json`
- `.github/workflows/ci.yml`
- `docs/Architectural Reviews/AR-0010-dependency-trust-and-source-provenance.md`
- `docs/Notes/assurance-claim-inventory-v1.md`
- `docs/Plans/high-assurance-engineering-and-evidence-export.md`
