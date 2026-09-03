# AR-0014: VACUUM Rebuild And Publication

| Field | Value |
| --- | --- |
| Status | Accepted |
| Opened | 2026-09-02 |
| Last reviewed | 2026-09-02 |
| Scope | Core storage / file lifecycle / recovery / encryption |
| Trigger | MVP+10 next requires explicit space reclamation |
| Related ADRs | ADR-0002, ADR-0004, ADR-0005, ADR-0009 |
| Related evidence | Pager freelist behavior, stable backup, portable export, writer-gate and crash tests |

## Architectural Question

Should `VACUUM` compact pages in place or rebuild a verified sibling artifact,
and how can publication preserve writer exclusion, generations, protectors, and
old-or-new crash semantics?

## Context

Deletes already release unused overflow pages to a persistent freelist and page
rewrites compact fragmented heaps locally. They do not shrink the database file
or globally repack the B+ tree. Portable export proves staging-copy validation,
but it intentionally publishes to a different path and does not rebuild live
records.

A naive new-database rebuild is not equivalent to maintenance. It resets the
durable committed generation, changes or drops protector configuration, and can
release the source writer gate before replacement. Copying encrypted page images
would retain fragmentation; rewriting them must use fresh page nonces.

## Evidence

- Tests or fuzzing: transaction recovery proves old-or-new logical commit;
  export proves verify-before-rename staging; writer tests prove one persistent
  sidecar gate coordinates mutation paths.
- Independent consumers: core and SQL callers treat all reserved catalog, row,
  and secondary-index records as ordinary logical KV pairs.
- Diagnostics or audits: header inspection exposes page count and freelist head,
  sufficient to measure before/after reclamation without claiming performance.
- Repeated implementation friction: the original constructors acquired their own
  writer guard, so an offline operation cannot retain one source guard across
  both open/checkpoint and replacement without a guarded-open refactor.
- Retained-admission implementation evidence: the writer guard is now a
  cloneable, path-bound capability over one shared OS file handle. A private
  pager open accepts only a guard for the same database path, and tests prove
  the gate remains held when either the maintenance owner or pager owner is
  dropped before the other.
- Publication implementation evidence: the private Unix helper validates a
  same-directory pair of regular files, synchronizes staging, opens the parent
  before the publication point, performs one `rename()`, then synchronizes the
  directory. Its typed internal result distinguishes a rename failure from a
  post-rename durability failure; unsupported targets return before inspecting
  or changing either path.
- Rebuild-state implementation evidence: a private pager context captures the
  authenticated page-zero image and active derived keys after writable recovery
  has checkpointed. The staging constructor copies format/protector identity,
  resets only allocation fields, retains the checkpoint-generation lower bound,
  and uses the ordinary encrypted page writer. A focused encrypted test proves
  the passphrase still unlocks, keyslot bytes remain identical, generation does
  not move backward, and identical page plaintext receives a different frame.
- Copy/verification implementation evidence: the rebuild copies each logical
  record in its own bounded transaction, then drops and reopens staging through
  the source unlock path. It requires an empty staging WAL, matching logical
  count and length-framed SHA-256 digest, and a clean structured page/B-tree
  verification report. Tests cover reclaimed sentinel data and every supported
  unlock path (passphrase, recovery key, and keyfile), plus refusal to overwrite
  staging and writer exclusion throughout rebuild work.
- Orchestration implementation evidence: public core entry points cover
  sentinel, passphrase, recovery-key, and keyfile unlock paths, with a real CLI
  caller at `tosumu vacuum`. Capability checking is the first operation.
  Supported execution retains the source writer guard through source close,
  staging-sidecar removal, atomic replacement, and directory synchronization.
  The typed report returns record/page/byte observations and durable
  confirmation; post-replacement sync failure maps to
  `VACUUM_DURABILITY_UNCERTAIN` without rollback. The Windows test proves
  `VACUUM_PLATFORM_UNSUPPORTED` leaves source, WAL, and lock bytes unchanged
  and creates no staging artifact.
- Failure-boundary implementation evidence: an injected replacement failure
  observes the source gate still held, leaves the old logical source
  authoritative, and removes only owned staging artifacts. An injected
  directory-sync failure first replaces the source, returns
  `VACUUM_DURABILITY_UNCERTAIN`, and proves cleanup does not restore the old
  file. Staging cleanup is armed only after create-new succeeds, so a name
  collision is refused without deleting the pre-existing path.
- The injected matrix now also stops after gate acquisition but before source
  open/checkpoint, during a selected record copy, and immediately before staged
  verification. The first leaves source and WAL bytes unchanged; the latter two
  preserve the source's logical record set and remove the owned staging main,
  WAL, and writer-lock paths. Together with the replacement and directory-sync
  cases, every boundary named by the initial failure plan is exercised without
  using platform emulation as evidence for atomicity.
- After recovery/checkpoint and while retaining the writer gate, VACUUM now
  runs complete page authentication and B+ tree invariant verification against
  the source before creating staging. A source with an authenticated-page or
  structural finding is rejected as `FILE_CORRUPT`; a focused corruption test
  proves the source bytes remain untouched and no staging artifact appears.
- Before staging creation, VACUUM queries caller-available space on the sibling
  filesystem and requires at least the current source main-file length. A failed
  preflight returns `STORAGE_OUT_OF_SPACE` without creating staging. This is a
  conservative admission observation, not a promise that later allocation
  cannot fail if filesystem availability changes during the rebuild.
- Platform publication evidence: POSIX `rename()` requires atomic replacement
  of an existing non-directory entry, and POSIX explicitly prescribes syncing
  the containing directory when the new name must be durably confirmed.
- Windows publication evidence: `ReplaceFileW` documents failure outcomes in
  which the replaced path can be absent or the old file can have moved to a
  different name; its `REPLACEFILE_WRITE_THROUGH` flag is unsupported.
  `MoveFileExW` documents replacement, but its write-through guarantee applies
  specifically to moves implemented as copy-and-delete and does not state the
  required atomic old-or-new crash contract.
- Rust portability evidence: `std::fs::rename` currently maps to POSIX
  `rename()` on Unix and one of multiple Win32 mechanisms on Windows. Its API
  does not strengthen the Windows mechanisms into the guarantee required here.
- Missing evidence: no encrypted rebuild constructor or interruption matrix
  exists yet.

## Ownership And Dependency Analysis

Core owns physical rebuild, encryption material, generation continuity,
verification, writer admission, and publication. SQL and other consumers own no
special participation: every live logical KV pair, including reserved keys, is
copied byte-for-byte by key and value.

The CLI may expose progress and confirmation later, but it must not implement
copy, replacement, cleanup, or recovery policy. `VACUUM` is not a method on a
live `SharedKvStore`; replacing a file behind an open owner would split path and
handle identity.

## Alternatives Considered

### Alternative A: Compact pages in place

- Benefits: retains the same open file and protector header naturally.
- Costs: requires page relocation, pointer rewriting, truncation, and recovery
  of partially moved trees inside the normal WAL protocol.
- Failure mode: a crash can leave parent pointers, freelist, and file length in
  mutually inconsistent states.

### Alternative B: Rebuild and atomically replace while retaining the gate

- Benefits: constructs a dense tree using ordinary insertion, verifies it
  before publication, and admits a simple old-or-new source outcome.
- Costs: requires temporary space near the source, guarded open/checkpoint,
  crypto-state transfer, generation continuity, and platform replacement code.
- Failure mode: a non-atomic replacement or premature gate release creates a
  missing source or permits a writer to mutate the old file during publication.

### Alternative C: Publish only `VACUUM INTO` and require manual replacement

- Benefits: reuses export-style destination publication.
- Costs: does not satisfy the admitted source-reclamation command and delegates
  the most dangerous step to callers.
- Failure mode: manual replacement can omit the WAL, lock sidecar, durability
  barriers, or protector-preservation checks.

## Findings

Use a verified sibling rebuild and atomic source replacement. The operation is
offline and path-based. It acquires `<database>.writer.lock` before opening the
source and retains that same guard until publication and directory durability
complete. Read-only handles remain ungated; they may continue reading their old
open file, while new opens after publication observe the rebuilt file.

The source is checkpointed under the retained guard. Every live logical key and
value is copied to a sibling staging database. The rebuild preserves format,
encryption mode, active DEK, keyslots, and durable generation continuity, but
rewrites every page with fresh random nonces and recomputes authentication.
`VACUUM` does not imply DEK rotation.

The staged database must pass logical scan and structured verification before
publication. Publication uses a same-directory platform primitive that replaces
the source atomically. If that guarantee is unavailable, the operation refuses
before source mutation. The source WAL must be empty before replacement; the
persistent writer-lock sidecar is neither replaced nor removed.

The initial publication capability is admitted on Unix: synchronize the staged
file, open the containing directory, call same-directory `rename()`, then
`fsync()` the already-open directory descriptor. A rename failure is
pre-publication. An `fsync()` failure after rename is publication-complete but
durability-uncertain and must be reported as such.

Windows is not admitted for initial publication. Neither the documented
`ReplaceFileW` nor `MoveFileExW` contract proves both atomic old-or-new
replacement and durable namespace confirmation, so Windows must reject VACUUM
before checkpointing, creating staging artifacts, or otherwise mutating the
source. This is a platform limitation, not permission to emulate replacement
with delete/rename steps. Reopen this finding if Microsoft documents a suitable
primitive or a narrowly reviewed dependency supplies a stronger contract with
auditable evidence.

Failure or cancellation before publication leaves the source authoritative and
removes only recognized staging artifacts. After the publication point the
operation reports success or a specific durability-uncertain outcome; it must
not restore an older source over a possibly durable new one. Recovery must see
either the complete old database or the complete rebuilt database, never a
partially copied source.

## Disposition

Accepted through ADR-0009.

## Required Follow-Up

- [x] Record offline rebuild and publication invariants in ADR-0009.
- [x] Admit POSIX `rename()` plus containing-directory `fsync()` on Unix; reject
      Windows before mutation pending a documented atomic/durable mechanism.
- [x] Refactor guarded open/checkpoint without permitting public gate bypass.
- [ ] Implement crypto/generation-preserving staging rebuild and verification.
- [ ] Add interruption tests before, during, and after publication.

## Reopening Triggers

Reopen if a portable atomic-replacement primitive cannot be supported, if
protector preservation requires a format change, or if callers require online
vacuum, incremental vacuum, cancellation after publication, or DEK rotation.

## Review History

### Cycle 1 -- 2026-09-02

- Status entering review: Proposed
- New evidence: existing freelist reuse does not shrink the file; export staging
  does not preserve an in-place maintenance identity; current constructors
  cannot retain one writer guard across source close and replacement.
- Findings: only an offline verified sibling rebuild with retained admission and
  atomic replacement supports the claimed old-or-new outcome.
- Disposition: Accepted through ADR-0009.
- Resulting ADR or documentation change: open a dedicated implementation plan
  with platform publication and encrypted-state preservation as early gates.

### Cycle 2 -- 2026-09-02

- Status entering review: Accepted, publication mechanism unresolved.
- New evidence: POSIX specifies atomic replacement and directory `fsync()`;
  Microsoft documents partial `ReplaceFileW` failure states, an unsupported
  write-through flag, and no equivalent old-or-new durability guarantee for
  `MoveFileExW`.
- Findings: Unix publication is implementable without a new dependency;
  Windows must refuse before source mutation rather than inherit an unproven
  filesystem claim.
- Disposition: Accepted with Unix as the first supported publication target and
  Windows explicitly unsupported until stronger evidence is available.
- Sources:
  [POSIX `rename()`](https://pubs.opengroup.org/onlinepubs/9799919799/functions/rename.html),
  [POSIX directory durability rationale](https://pubs.opengroup.org/onlinepubs/9799919799/xrat/V4_xbd_chap01.html),
  [Microsoft `ReplaceFileW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-replacefilew),
  [Microsoft `MoveFileExW`](https://learn.microsoft.com/en-us/windows/win32/api/winbase/nf-winbase-movefileexw),
  [Rust `std::fs::rename`](https://doc.rust-lang.org/std/fs/fn.rename.html).
