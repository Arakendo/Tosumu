# Roadmap

This page is the public roadmap summary. It is intentionally shorter than the repository design document.

## Now

- stabilize the MVP +8 storage-and-inspection slice
- keep the CLI, inspect contract, and TUI viewer coherent
- keep crash, crypto, and verification behavior visible through tests and tooling
- improve the trust surface around docs, diagnostics, and website guidance

## Next

- the current design roadmap places a toy SQL layer after the core storage milestones
- continued work on inspection, audit, and structured diagnostics may still reshape near-term priorities while the project remains pre-stability

## Later

- MVCC-style reader work
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

The full MVP and stage breakdown lives in the repository design document. Use that when you need milestone-by-milestone detail.