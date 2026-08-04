# Architecture Governance Extraction Audit

## Purpose

This audit separates durable Tosumu architecture from unresolved design
directions that had accumulated in `docs/Specifications/Tosumu Software Design Document.md`, `docs/architecture.md`, and
their peer specifications.

```text
implemented and accepted boundary -> ADR
unresolved ownership question     -> Architectural Review
approved implementation work      -> Plan
future possibility                 -> design or roadmap evidence
```

## Extracted Records

| Record | Result | Primary evidence |
| --- | --- | --- |
| ADR-0002 Authenticated Pager Trust Boundary | Accepted existing architecture | pager implementation, security model, corruption tests, design sections 4-8 |
| AR-0002 Structured Inspection Contract Boundary | Incubating | core facts and errors; CLI-owned JSON envelope; TUI/WPF consumers |
| AR-0003 Service Authority And Host Modes | Incubating | embedded implementation and future host sketches |
| AR-0004 Semantic Change History And Sync | Incubating | physical WAL plus future offline-sync requirements |
| AR-0005 Witness, Observer, And Freshness | Incubating | authenticated storage and explicit rollback/freshness exclusions |
| AR-0006 Format Evolution And Migration | Incubating | implemented pre-stability format and deferred migration policy |

## Existing Records Retained

- ADR-0001 continues to define storage-engine layer boundaries.
- AR-0001 continues to incubate the TQL command-language boundary.
- The TQL implementation plan remains active evidence rather than an accepted
  command-language architecture.

## Findings Not Promoted

The following topics remain design or roadmap material. They do not yet have
enough independent pressure for a separate review:

- MVCC and multiple-reader mechanics;
- repair and salvage services beyond the current rollback-first posture;
- telemetry, entropy scoring, and operational fingerprinting;
- compliance reports and remote attestation;
- advanced indexes and query extensions;
- cluster deployment details and concrete K3s topology;
- generic migration registries, receipt stores, and automatic migration APIs.

Promoting every future heading would make review records restate speculation
rather than preserve decision pressure.

## Contradictions Made Explicit

The design discusses future daemon, server, witness, observer, and sync roles
while also describing Tosumu as single-file, embedded, and not a distributed
storage engine. These statements can coexist only if hosts and movement remain
adapters around one canonical embedded storage engine. AR-0003 through AR-0005
hold that question open rather than resolving it by implication.

## Next Audit Trigger

Repeat this extraction audit when one of these occurs:

- a new workspace crate changes ownership or dependency direction;
- a consumer depends on a future service, sync, witness, or migration contract;
- the on-disk format reaches a declared stability milestone;
- normative specifications gain a durable guarantee not represented by an ADR;
- an incubating review obtains enough evidence for acceptance, rejection, or
  deliberate deferral.

