# Tosumu Specifications

This collection contains Tosumu's durable engineering specifications. These
documents define current contracts; accepted ADRs may refine or supersede
their architectural decisions, and implementation plans may not override
them.

| Specification | Authority | Scope |
| --- | --- | --- |
| [Tosumu Software Design Document](Tosumu%20Software%20Design%20Document.md) | Normative | Architecture, format, goals, and staged design |
| [Tosumu Error Design Document](Tosumu%20Error%20Design%20Document.md) | Normative | Public error identity, categories, context, and boundary translation |
| [Tosumu Inspect API Specification](Tosumu%20Inspect%20API%20Specification.md) | Normative | Machine-readable inspection envelopes, payloads, errors, and compatibility |
| [Tosumu Reference Implementations](Tosumu%20Reference%20Implementations.md) | Informative | External implementations and resources that inform Tosumu work |

The repository-root `SECURITY.md` remains the conventional security policy and
disclosure entry point. The published
[Safety and Limits](../safety-and-limits.md) page summarizes that posture for
the documentation site.

## Maintenance Rule

- Keep one authoritative copy of each specification.
- Update code, tests, contributor guidance, and public summaries when a
  specification path or contract changes.
- Use an ADR for accepted architectural decisions that alter an established
  boundary.
- Use the [Document Status](../document-status.md) dashboard to distinguish
  authority from lifecycle.
