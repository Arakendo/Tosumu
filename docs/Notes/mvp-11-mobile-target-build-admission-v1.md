# MVP+11 Mobile Target Build Admission v1

| Field | Value |
| --- | --- |
| Status | Cross-build profiles observed; simulator/emulator loading not yet observed |
| Observed | 2026-09-03 |
| Owner | AR-0017 / MVP+11 Slice 4 |
| Depends on | AR-0010 dependency evidence, AR-0017, private experimental C adapter |
| Construction Rust | Rust/Cargo 1.95.0; target inspection uses matching `llvm-tools-preview` |

## Purpose

Name the first mobile compile and artifact-inspection profiles before adding
tooling or allowing a successful cross-build to become a runtime claim. These
profiles are intentionally 64-bit and callback-free. They do not qualify a
simulator, emulator, physical device, filesystem, lifecycle, packaging format,
or platform protector.

## Candidate Profiles

| Profile | Rust target | Build host and native tools | Provisional deployment floor | Artifact question |
| --- | --- | --- | --- | --- |
| iOS device ARM64 | `aarch64-apple-ios` | GitHub `macos-15`; Xcode 16.4 (`16F6`); iPhoneOS 18.5 SDK | iOS 13.0 | Can the experimental C surface link as an ARM64 static archive with only the declared symbols? |
| iOS simulator ARM64 | `aarch64-apple-ios-sim` | GitHub `macos-15`; Xcode 16.4 (`16F6`); iPhoneSimulator 18.5 SDK | iOS 13.0 | Can the same surface link for the Apple-silicon simulator without being called runtime-tested? |
| Android device ARM64 | `aarch64-linux-android` | GitHub `ubuntu-24.04`; NDK r27d `27.3.13750724`; Clang target driver | API 24 | Can the adapter link as an ARM64 `.so` with a bounded exported-symbol set? |
| Android emulator x86-64 | `x86_64-linux-android` | GitHub `ubuntu-24.04`; NDK r27d `27.3.13750724`; Clang target driver | API 24 | Can the adapter link as an x86-64 `.so` without implying an emulator run? |

The Rust iOS targets are Tier 2 with `std` and require the matching SDK from
Xcode. Rust documents iOS 10 as its toolchain floor; this experiment raises the
deployment target to 13.0 as a provisional Tosumu maintenance boundary. Android
uses the LTS NDK and an explicit API suffix on the Clang driver. Android's
`minSdkVersion` affects native link/load compatibility even when Tosumu does not
directly call a newer API, so API 24 is part of the artifact subject rather than
wrapper metadata to add later.

The initial profile excludes 32-bit Android, Intel iOS simulator, Mac Catalyst,
bitcode, dynamic iOS libraries, XCFramework/AAR packaging, signing, and symbol
stripping. Those are not silently inherited from the host runner.

## Artifact Shape

The experimental crate currently emits an `rlib` and desktop/Android-style
`cdylib`. Slice 4 may add `staticlib` solely to produce the private iOS archive;
this is not permission to publish that archive or stabilize its symbols.

Each build must retain at least:

- the exact Rust compiler identity and installed target;
- runner OS/image observations;
- `xcodebuild -version` plus selected SDK identity, or NDK revision plus Clang
  identity;
- deployment target/API level;
- file type and architecture reported from the linked artifact;
- the normalized Tosumu export set compared with the retained allowlist; and
- confirmation that test-hook symbols are absent from the non-test artifact.

The construction compiler is pinned independently of the repository's moving
`stable` compatibility jobs. Apple `nm` from Xcode 16.4 cannot inspect every
LLVM attribute emitted by newer Rust LLVM releases, so archive symbol evidence
uses the `llvm-nm` shipped with the pinned Rust toolchain. Xcode still owns the
Apple SDK, linker, archive type, and architecture observations; the matching
LLVM reader owns only the normalized symbol observation.

An archive member list or shared-object dependency list is useful artifact
inspection, not loader evidence. A later simulator/emulator consumer must load
and invoke the library before that stronger statement is made.

## Target-Specific Dependency Closure

The dependency provenance generator now resolves all four mobile targets. The
workspace profiles contain 173 packages for each iOS target and 175 for each
Android target. The narrower `tosumu-core` normal/build closures contain 41
packages on iOS and 42 on Android, with seven build-script and one proc-macro
candidate in each profile.

The two iOS closures match the current macOS core package set. Android adds
`linux-raw-sys 0.12.1` relative to iOS through the native file-locking closure.
That is resolution/reachability evidence, not proof that every package builds,
links, runs, or behaves correctly on the target. Existing build-script source
reviews remain hash-bound; the new profile membership does not upgrade their
overall `attempted_incomplete` state.

## Admission Sequence

1. Check regenerated target closure and human-reviewed executable inputs.
2. Add `staticlib` as a private iOS artifact shape and preserve the existing
   Linux `.so` symbol baseline.
3. Link all four release artifacts with exact host tools and deployment floors.
4. Inspect architecture, file kind, dependencies, and exported Tosumu symbols.
5. Retain the hosted run and failures without claiming load or runtime evidence.
6. Only then design the first independent simulator/emulator loader fixture.

## Open Questions

- Whether the provisional iOS 13/API 24 floors match the first real customer's
  device fleet.
- Whether one archive allowlist can exclude Rust/runtime symbols meaningfully,
  or whether iOS must inspect only global Tosumu-prefixed symbols.
- Whether `fs4`/`rustix` file-lock behavior needs a mobile-specific mechanism or
  only runtime filesystem qualification.
- Whether the eventual iOS package should be a static XCFramework and Android
  package an AAR with one or more ABI slices.
- Whether reproducible artifact identity is feasible with the selected Apple
  and Android linkers before packaging metadata is added.

## Evidence Limits

GitHub Actions run `33825144893` observed the four named artifacts at commit
`f63383fa7710d94a33940cb974b4c8b4be58f68d`. Job `100876092477` linked and
inspected the two arm64 Apple archives with Rust 1.95.0, Xcode 16.4 (`16F6`),
and SDK 18.5. Job `100876092434` linked and inspected the arm64 and x86-64
Android shared objects with Rust 1.95.0, NDK r27d, and API 24 Clang drivers.
All four normalized production symbol sets matched the retained allowlist.
Dependency provenance job `100876092292` independently regenerated the
target-filtered closure under the pinned Rust/Cargo release. The overall run
was cancelled after these and the relevant FFI evidence jobs passed, so no
unfinished matrix work is credited.

Run `33825979582`, iOS job `100878618289`, then compiled a separate C11
executable, linked it with the arm64 simulator archive, cold-booted the admitted
iOS 18.5 runtime, and invoked it through `simctl spawn`. The process called the
ABI-version export and returned the expected marker. The job passed from
01:29:54 through 01:33:19 UTC on 2026-09-04 at cumulative commit
`6c35f028830c91d1c0f971191614e80abc219d98`. A preceding app-bundle attempt
was deliberately stopped after its packaging path stalled; it is retained as a
failed experiment, not loader evidence.

Official target documentation and runner inventories informed these candidate
inputs. The retained run now records Tosumu's compiler, SDK/NDK, linker, and
artifact observations. The iOS simulator archive now has one loader
observation; Android still does not. Runner image inventories can change
beneath a label. The jobs fail if the explicitly selected Xcode or NDK path
disappears rather than silently choosing another toolchain.
