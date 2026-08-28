# Roadmap

This page is the public roadmap summary. It is intentionally shorter than the repository design document.

## Now

- use the completed MVP+10 private storage and shared-owner evidence to admit a
  public shared-handle/read-transaction contract under AR-0009
- keep the CLI, inspect contract, and TUI viewer coherent
- keep crash, crypto, and verification behavior visible through tests and tooling
- improve the trust surface around docs, diagnostics, and website guidance

## Next

- expose the smallest shared-handle and committed-generation read contract only
  after its names and lifecycle survive independent caller review
- retain fail-fast writer admission and bounded snapshot/WAL pressure while
  keeping cancellation, timeout, and background execution out of the initial API
- continued work on inspection, audit, and structured diagnostics may still reshape near-term priorities while the project remains pre-stability

## Later

- logical SQL scans built on the admitted reader-visibility contract
- secondary indexes and `VACUUM`
- mobile-facing wrappers and protector integrations
- witness, observer, and deployment work for clustered scenarios
- entropy bookkeeping and richer audit reporting

## Not Planned Yet

- becoming a general-purpose relational database product
- networked client/server operation as the core project shape
- feature parity with SQLite
- full-text search, vector search, or advanced indexing families outside the documented scope
- production-hardening promises before the design and implementation earn them

## For the full roadmap

Use the [Main Feature Roadmap](Plans/main-feature-roadmap.md) for the canonical
delivery checklist and current completion status. The full normative MVP and
stage definitions remain in `docs/Specifications/Tosumu Software Design Document.md`.
