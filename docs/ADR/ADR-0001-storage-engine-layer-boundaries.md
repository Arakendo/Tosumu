# ADR-0001: Storage Engine Layer Boundaries

## Status

Accepted

## Context

Tosumu is a key/value storage engine with optional higher-level relational and
consumer adapters. Its pager, physical format, WAL, recovery, B+ tree, and
integrity behavior must remain understandable without importing SQL, table, or
application-specific meaning.

This boundary is already established by `docs/Specifications/Tosumu Software Design Document.md` and the workspace shape.
This ADR records that existing decision so later consumers cannot accidentally
turn their schema into storage-engine truth.

## Decision

- `tosumu-core` owns physical storage mechanics, key/value behavior,
  transactions, recovery, integrity, bounded inspection, and provider-neutral
  storage contracts.
- SQL, table, constraint, CLI, .NET, and other adapters depend downward on the
  core storage contract.
- Consumer schemas and meanings remain consumer-owned. Tosumu may store Tokimu
  assets or another application's records without learning those semantics.
- Higher layers must not reach through stable storage APIs to make the pager,
  B+ tree, WAL, or physical pages serve as application-level contracts.
- Dependency direction remains from semantic adapters toward storage
  mechanics, never from storage mechanics toward a consumer.

```text
Consumer meaning and schema
    ↓
Optional relational or provider adapter
    ↓
Tosumu key/value and transaction contract
    ↓
Pager, B+ tree, WAL, recovery, authenticated pages
```

## Consequences

- Tosumu can serve unrelated consumers without coupling its physical format to
  one application's vocabulary.
- SQL remains optional and cannot become a prerequisite for basic storage.
- Provider APIs must expose storage behavior and diagnostics without leaking
  physical implementation objects.
- Features that require new durable semantics must update the design and, when
  architectural, this ADR or a superseding ADR.
- Some adapters may temporarily duplicate translation logic while evidence for
  a shared higher-level contract is still immature.

## Alternatives Considered

- **Teach the core relational concepts.** Rejected because it would turn the
  key/value engine into a relational database and couple pages to schemas.
- **Expose physical storage objects to consumers.** Rejected because it would
  make implementation details accidental compatibility contracts.
- **Require SQL for every consumer.** Rejected because simple providers and
  embedded applications need the smaller key/value boundary.

## References

- `docs/Specifications/Tosumu Software Design Document.md`, especially goals, non-goals, and guiding principle 7
- `docs/Plans/initial-sql-layer.md`
- `docs/CRs/Tokimu/tokimu-001-tasset-storage-provider-boundary.md`

