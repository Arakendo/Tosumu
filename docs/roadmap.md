# Roadmap

This page is the public roadmap summary. It is intentionally shorter than the repository design document.

## Now

- maintain the ADR-0006/0007 `SharedKvStore` snapshot, atomic-write, and
  conditional-write contracts through core and SQL-layer caller tests
- maintain ADR-0008 plain single-column secondary indexes and their atomic
  row/index mutation contract
- obtain the remaining native Unix CI evidence for the completed MVP+10
  `VACUUM` implementation
- advance [AR-0015](Architectural%20Reviews/AR-0015-native-replication-scope-authority-and-failure-model.md):
  define concrete failure domains, RPO/RTO hypotheses, authority epochs, and
  fencing requirements before native replication work
- begin the cross-cutting assurance inventory and AR-0010 dependency-closure
  baseline without changing the current pre-audit security posture
- keep the CLI, inspect contract, and TUI viewer coherent
- keep crash, crypto, and verification behavior visible through tests and tooling
- improve the trust surface around docs, diagnostics, and website guidance

## Next

- prove one bounded service authority without changing embedded storage
  semantics
- deploy one writable K3s host with exclusive storage, verified offsite
  backups, restore drills, and topology-specific RPO/RTO evidence
- add observer and witness freshness evidence without treating witnesses as
  replicas or readiness as automatic failover
- define a bounded evidence-export boundary for identity, generation,
  integrity, recovery, freshness, authority, backup, durability, and build
  provenance without merging those claims
- retain fail-fast writer admission and bounded snapshot/WAL pressure while the
  hosted authority and cancellation contracts are reviewed

## Later

- logical SQL scans built on the admitted reader-visibility contract
- composite and covering secondary indexes after measured caller pressure
- mobile-facing wrappers and protector integrations
- entropy bookkeeping and richer audit reporting
- an admitted single-leader replication representation, verified snapshot
  bootstrap, and asynchronous warm standby with manual fenced promotion
- fenced automatic authority transfer with stale-primary rejection and explicit
  partition behavior
- synchronous quorum durability only if a concrete near-zero-RPO requirement
  justifies distributed-state-machine scope
- reproducible and attested release artifacts, named platform qualification,
  privilege/key-lifecycle review, and independent assurance-profile review

## Not Planned Yet

- becoming a general-purpose relational database product
- networked client/server operation as the core project shape
- shared-filesystem multi-writer operation or active-active replication
- feature parity with SQLite
- full-text search, vector search, or advanced indexing families outside the documented scope
- production-hardening promises before the design and implementation earn them
- blanket high-assurance, regulatory, certification, or defense-suitability
  claims detached from a named reviewed deployment profile

## For the full roadmap

Use the [Main Feature Roadmap](Plans/main-feature-roadmap.md) for the canonical
delivery checklist and current completion status. The full normative MVP and
stage definitions remain in `docs/Specifications/Tosumu Software Design Document.md`.
