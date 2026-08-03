# Tosumu Architectural Review Records

Architectural Review Records preserve unresolved questions, implementation or
consumer pressure, evidence, alternatives, findings, dispositions, and
reopening criteria. They sit between informal notes and binding ADRs.

```text
Observation or pressure
    ↓
Architectural Review
    ├── incubate / defer / reject / confirm current architecture
    └── accept architectural change
            ↓
           ADR
```

## Open A Review When

- ownership between storage core, relational layers, adapters, and consumers is
  unclear;
- a file-format or compatibility change is proposed;
- a new stable public contract or cross-layer dependency is proposed;
- repeated implementation friction suggests an existing boundary is wrong;
- corpus or independent-consumer evidence may reopen an accepted decision;
- a proposal is deferred or rejected and its reasoning should remain durable.

Use `AR-NNNN-short-title.md`. Review numbering is independent from ADR
numbering. Copy `TEMPLATE.md` and add the record to this index.

## Statuses

- **Proposed**
- **Under Review**
- **Incubating**
- **Accepted** -- requires an ADR or deliberate ADR revision
- **Deferred**
- **Rejected**
- **No Change**
- **Superseded**
- **Reopened**

## Index

No Tosumu Architectural Review Records have been opened yet. The Tokimu
storage-provider CR remains an incoming consumer request until Tosumu evidence
raises an unresolved ownership question.

