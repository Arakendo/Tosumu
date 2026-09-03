# MVP+10 VACUUM

## Purpose

Implement ADR-0009's offline verified rebuild without weakening writer,
recovery, generation, or encrypted-protector guarantees.

## Ordered Slices

### 1. Platform Publication Evidence

- [ ] Define native Windows and Unix atomic replacement helpers.
- [ ] Prove same-directory replacement, source-old-or-new interruption states,
      and parent-directory durability behavior.
- [ ] Refuse unsupported targets before creating or mutating source artifacts.

### 2. Retained Writer Admission

- [ ] Add a private guarded source-open/checkpoint path that consumes or borrows
      an already acquired `WriterGuard`.
- [ ] Retain the source sidecar gate across source close, staging verification,
      atomic replacement, and directory synchronization.
- [ ] Prove a competing writer receives `FileBusy` throughout the operation.

### 3. Rebuild State Transfer

- [ ] Capture format, active crypto material, protector slots, and committed
      generation through a private typed rebuild context.
- [ ] Create a sibling staging database with the preserved context and fresh
      page nonces/authentication.
- [ ] Copy every live logical key/value pair in bounded transactions while
      preserving generation monotonicity.
- [ ] Reject invalid source structure or insufficient staging space explicitly.

### 4. Verification And Publication

- [ ] Compare source and staging logical counts/digests without exposing values.
- [ ] Require complete structured verification and no staging WAL.
- [ ] Atomically replace the source; never replace the writer sidecar.
- [ ] Return a typed report with byte/page/count observations and durability
      confirmation.

### 5. Failure Matrix And Closure

- [ ] Inject failures before checkpoint, during copy, during verification, at
      replacement, and during directory synchronization.
- [ ] Prove pre-publication failures retain the old source and clean recognized
      staging files.
- [ ] Prove post-publication uncertainty never restores an older source.
- [ ] Cover unencrypted and every supported protector unlock path.
- [ ] Run formatting, strict clippy, all-target tests, and strict docs.
- [ ] Update ADR/AR history and advance MVP+10 to benchmark closure.

## Acceptance

- All committed logical records survive, including SQL catalog and index keys.
- The rebuilt source is smaller when reclaimable pages exist.
- Existing protectors still unlock encrypted databases.
- The durable committed generation never moves backward.
- Crash/interruption exposes the complete old or complete rebuilt database.
