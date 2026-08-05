# TQL Trust And Explanation Evidence Inventory v1

## Purpose

This inventory records which evidence is available before `TRUST <key>` or
`WHY <key>` enter the bounded TQL implementation. It is an admission record,
not a promise that the command surface exists.

## Evidence Matrix

| Dimension | Current source | Scope | TQL status | Honest result today |
| --- | --- | --- | --- | --- |
| Stored-page integrity | `tosumu_core::inspect::inspect_verification` | Database and page report | Available to `CHECK` | Passed, failed, or not checked; never truth |
| Key presence | `KvStore::get` | One requested key | Available to `DESCRIBE` | Found or missing, with byte count only |
| Key-specific integrity explanation | No public citation surface | One requested key | Missing | Unavailable; do not infer it from a database-wide result |
| Freshness | No witness or observer anchor | Record and database | Missing | Unanchored, not fresh or stale |
| Witnesses | No peer/witness model | Record and database | Missing | Unavailable |
| Provenance/history | No stable semantic change history | Record | Missing | Unavailable |
| Conflict state | No canonical conflict metadata | Record and scope | Missing | Unavailable |
| Recommended action | No typed explanation policy | Record and database | Missing | Do not render invented advice |

## Findings

- Authentication and a successful verification report are integrity evidence;
  neither establishes truth, provenance, freshness, or witness coverage.
- `CHECK` is intentionally database-scoped. Reusing its successful result as a
  per-key trust verdict would overstate what the current inspection API knows.
- `DESCRIBE` can establish presence, not semantic meaning or historical origin.
- `unanchored` is the correct future freshness state when a freshness question
  is asked without an external trusted anchor. It is not evidence that a record
  is stale, and it must not be reported by a command that has not requested
  freshness evidence.

## Slice 4 Decision

`TRUST <key>` and `WHY <key>` remain deferred. Adding them now would create a
shell-owned trust model with mostly unavailable fields and hardcoded advice.
The next admissible implementation requires a public, provider-neutral evidence
surface that can cite the source of each integrity, freshness, provenance, and
witness finding.

## Reopening Triggers

- A public key-scoped verification or evidence API exists.
- Semantic change history exposes stable provenance facts.
- A witness or observer capability supplies a freshness anchor.
- Conflict metadata has an owner and a bounded query surface.
- A caller needs typed recommendations backed by those findings.
