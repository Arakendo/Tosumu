# ADR-0006: Shared KV Store And Snapshot Transactions

## Status

Accepted

## Context

ADR-0004 admits one cooperating writer, and ADR-0005 defines format-3
committed generations, reader-pinned retained WAL, checkpoint suppression, and
finite pressure bounds. AR-0009 then exercised that storage contract through a
private shared owner, an opt-in logical prototype, and the separate
`tosumu-sql` crate using real SQL row encodings.

The evidence supports a small embedded KV contract. It does not support the
SDD's complete future typestate `Database`/`Session` model, writer queues,
waiting policies, asynchronous execution, session identities, or partial
checkpoints. Promoting those concepts together would freeze policy that no
caller currently needs.

## Decision

`tosumu-core` exposes these supported provider-neutral types at its crate root:

- `SharedKvStore`: a cloneable `Send + Sync` owner for one writable database;
- `KvReadTransaction`: a generation-pinned `Send + !Sync` logical reader;
- `KvScanPage`: one owned, bounded logical range result with an inclusive
  continuation key;
- `KvWriteTransaction<'_>`: a borrowed `!Send + !Sync` logical writer supplied
  only to an atomic write callback; and
- `KvConnectionInfo`: bounded process-local reader, generation, WAL-retention,
  and checkpoint-blocking facts.

The names deliberately extend the existing `KvStore` and `KvTransaction`
provider vocabulary. They do not reserve the generic `Database` and `Session`
names needed by a later locked/unlocked typestate or host composition.

### Owner and opening contract

- `SharedKvStore::create` and `open` support unencrypted format-3 databases.
- `create_encrypted` and `open_with_passphrase` support the current passphrase
  protector path. Additional protector-specific constructors may be added
  without changing snapshot meaning.
- The owner retains the ADR-0004 writer guard for its complete shared lifetime.
  The last database or read-transaction reference releases it on drop.
- Independent `KvStore::open_readonly` handles remain live views and do not join
  this process-local snapshot registry.

### Read contract

- `snapshot` captures the latest durable committed generation while serialized
  with commit publication and retains one bounded registry pin until drop.
- `KvReadTransaction::get` and inclusive ordered `scan` resolve only versions no
  newer than that generation through the authenticated pager boundary.
- `KvReadTransaction::scan_page` applies positive pair and logical-payload-byte
  limits during traversal. Its owned `KvScanPage` contains admitted pairs, the
  first unconsumed key as an inclusive continuation, and the blocked entry's
  full logical size when the byte budget prevents admission. Logical payload is
  `key.len() + value.len()`.
- An excluded overflow value is not read or allocated merely to discover its
  continuation and declared size. The continuation key is one explicit additive
  allocation outside the payload budget, independently bounded by
  `MAX_KEY_SIZE`. Invalid limits and inverted bounds retain typed
  `InvalidArgument`; admitted data retains existing corruption/authentication
  behavior.
- The reader is movable to another thread but cannot be shared concurrently.
- Reader drop only unregisters the pin. It performs no checkpoint, I/O, wait,
  or fallible cleanup.

### Write contract

- `put` and `delete` publish one logical mutation through the common format-3
  transaction mechanism.
- `write` holds the process-local owner for the callback. Returning `Ok`
  publishes all staged mutations as one committed generation; returning `Err`
  rolls them back and preserves the caller error.
- The borrowed writer exposes logical `put`, `delete`, and staged `get`. It
  cannot escape the callback or move/share across threads.
- A callback must use its supplied transaction. Same-thread re-entry through a
  captured clone or snapshot fails before mutex acquisition with structured
  `InvalidArgument`, rolls back, and does not advance the generation.
- A callback panic publishes no staged WAL bytes and poisons the process-local
  owner. Drop plus validated reopen recovers the prior committed state. Tosumu
  does not catch the panic or perform fallible commit/rollback work in `Drop`.

### Diagnostics and execution policy

`KvConnectionInfo` reports active and maximum readers, oldest reader
generation, checkpoint and latest generations, retained WAL bytes, retained
frame versions, and whether process-local readers block checkpointing.

Operations are synchronous. Cross-process writer admission remains fail-fast
through ADR-0004. This decision adds no queue depth, session identity, reader
age, timeout, retry, cancellation, background executor, or passive/prefix
checkpoint contract. Such policy requires new caller evidence and deliberate
review.

The previous `experimental-shared-readers` feature and `experimental` module
are removed when this contract is implemented. Keeping two names for the same
mechanism would make the compatibility boundary ambiguous.

## Consequences

- Embedded callers can share one writer owner across threads while retaining
  coherent historical logical reads.
- The initial owner mutex serializes each logical read operation. Multiple
  snapshots may coexist and a writer may commit between their calls, but this
  decision does not claim parallel read execution or scaling.
- `tosumu-sql` has a real lower-layer snapshot contract for later scan work
  without teaching core about tables or SQL.
- Bounded consumers can stop before excluded overflow materialization and resume
  without receiving a physical page, slot, WAL position, or mutable cursor.
- Long-lived readers can defer checkpoints and cause bounded write rejection;
  callers can observe the pressure but cannot configure the private defaults
  through this initial API.
- The richer SDD `Database`/`Session` design remains a compatible future wrapper
  rather than an alias for this narrower KV store.
- This decision changes no page/WAL bytes, generation meaning, recovery order,
  security/freshness claim, or dependency closure beyond ADR-0004/0005.

## Alternatives Considered

- **Promote the experimental names unchanged.** Rejected because
  `SharedKvDatabase` claims a broader abstraction than the logical KV surface
  actually provides, and an `experimental` module cannot be the supported
  compatibility boundary.
- **Implement `Database` and `Session` now.** Rejected because no current caller
  needs session identity, waiting policy, locked typestate, or host scheduling.
- **Add snapshots directly to cloneable `KvStore`.** Rejected because its
  existing mutable transaction contract and independent-handle live-view
  behavior would become ambiguous.
- **Keep the feature experimental indefinitely.** Rejected because private,
  feature-gated, encrypted, failure-path, and downstream SQL evidence now cover
  the admitted minimum contract.

## Reopening Triggers

Revisit this decision if a consumer needs cross-process pinned readers,
read-only shared owners, recovery-key/keyfile constructors, configurable
retention limits, partial checkpoints, session identity/age, bounded waiting or
cancellation, async integration, parallel read throughput, or a typestate
locked/unlocked database. Revisit bounded pagination if measured repeated root
descent requires an owned cursor, callers require continuation across
close/reopen, reverse traversal is admitted, or logical pair/payload limits do
not predict consumer resource use adequately.

## 2026-09-03 Amendment: Bounded Snapshot Pagination

AR-0018 established the bounded page contract through a private traversal,
leaf-boundary and overflow-corruption falsifications, a complete-scan property,
an integration caller using only public Rust exports, and an independently
compiled C caller. The amendment admits the provider-neutral Rust contract. It
does not admit the experimental C symbols as stable ABI, make the physical WAL
an application protocol, or add cursor, mobile, SQL, or service semantics to
core.

## References

- `ADR-0001-storage-engine-layer-boundaries.md`
- `ADR-0002-authenticated-pager-trust-boundary.md`
- `ADR-0003-source-unit-cohesion-size-pressure-and-decomposition.md`
- `ADR-0004-cooperative-single-writer-admission.md`
- `ADR-0005-committed-generation-and-retained-wal-snapshots.md`
- `../Architectural Reviews/AR-0009-multiple-reader-execution-and-coordination.md`
- `../Architectural Reviews/AR-0018-bounded-snapshot-range-pagination.md`
- `../Plans/mvp-10-multiple-readers.md`
- `../Specifications/Tosumu Software Design Document.md`
- `../Specifications/Tosumu Error Design Document.md`
