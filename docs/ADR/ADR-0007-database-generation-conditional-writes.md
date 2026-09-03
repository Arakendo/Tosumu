# ADR-0007: Database-Generation Conditional Writes

## Status

Accepted

## Context

MVP+10 calls for version-observing reads and conditional writes. ADR-0005
provides one durable monotonic database commit generation, and ADR-0006 exposes
the shared KV owner that can observe and mutate atomically. Tosumu does not
store durable per-key revisions, and physical page versions do not represent
logical key history.

## Decision

`tosumu-core` extends `SharedKvStore` with these provider-neutral types and
operations:

- `KvVersion`: an opaque token binding one database committed generation to
  the live shared-owner identity that observed it;
- `KvVersionedValue`: one value-or-absence observed atomically with its token;
- `KvConditionalResult`: either `Applied` or `NotApplied`, carrying the
  generation observed after the operation;
- `get_with_version`;
- `put_if_absent`;
- `compare_and_set`; and
- `put_if_version`.

The token is database-wide and valid only for clones of the shared owner that
created it. Any intervening committed mutation invalidates it, even when
another key changed. Drop plus reopen creates a new owner identity and
invalidates old tokens. This conservative behavior prevents stale and ABA
updates without claiming a per-key revision or durable database identity Tosumu
does not persist for every database mode.

The value and version returned by `get_with_version` are captured under the
same shared-owner critical section. Conditional methods check their
precondition, stage any mutation, and publish its commit before releasing that
same owner. A successful result reports the new committed generation; a failed
precondition reports the unchanged current generation.

`put_if_absent` applies only when the current value is absent.
`compare_and_set` applies only when the current value exactly equals the
supplied expected bytes. `put_if_version` applies only when the supplied token
equals the current database generation; it may insert or replace the named key.

An unmet precondition is a normal typed outcome, not `TosumuError` and not a
new error code. Storage, integrity, limit, busy, and durability failures remain
errors. Retry policy remains with the caller.

The first slice does not add conditional delete, multi-key preconditions,
transaction-local conditional helpers, per-key revisions, waiting, automatic
retry, token serialization, or cross-owner token acceptance. Implementations
must reject a token from a different or reopened owner before mutation;
generation equality alone is insufficient across database files.

This decision changes no page, record, WAL, or key format.

## Consequences

- Callers can express common optimistic mutations without a read/then-write
  race.
- Unrelated commits may cause conservative `NotApplied` outcomes for
  `put_if_version`.
- Value compare-and-set remains key-specific but intentionally treats a value
  that changes away and back as equal; callers requiring ABA protection use
  `KvVersion`.
- A version token must bind owner identity in memory without retaining the
  database pager/writer gate or exposing that identity as application data.

## Alternatives Considered

- Use page versions as key versions: rejected because physical rewrite identity
  is not logical record history.
- Add durable per-key revisions now: rejected because no caller evidence
  justifies the format and recovery expansion.
- Return only `bool`: rejected because the observed generation is useful for
  bounded retry and diagnostics without another racy observation.
- Return precondition misses as `Conflict` errors: rejected because the
  operation completed and reported an expected conditional outcome.

## Reopening Triggers

Revisit if measured contention requires per-key durable revisions, callers need
conditional delete or atomic multi-key preconditions, or tokens must survive
reopen, be serialized, or be compared across processes.

## References

- `ADR-0005-committed-generation-and-retained-wal-snapshots.md`
- `ADR-0006-shared-kv-store-and-snapshot-transactions.md`
- `../Architectural Reviews/AR-0012-conditional-write-and-version-token-semantics.md`
- `../Plans/mvp-10-multiple-readers.md`
- `../Specifications/Tosumu Software Design Document.md`
- `../Specifications/Tosumu Error Design Document.md`
