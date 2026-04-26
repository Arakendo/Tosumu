# Safety and Limits

This is the page to read before treating Tosumu like a mature database.

## Early development status

Tosumu is an academic and learning project. It has real storage logic, real crash-recovery work, and real authenticated page encryption, but it has not been independently audited or hardened for production use.

## Durability the project currently aims for

Current implemented direction includes:

- write-ahead logging
- crash recovery on open
- explicit verification tooling
- tests focused on torn writes, crash boundaries, and recovery behavior

That means the project is trying to make durability and corruption behavior visible and testable.

## Durability the project does not claim yet

Tosumu does not currently claim:

- production-grade durability validation across all filesystems and deployment topologies
- mature multi-process coordination
- audited storage semantics
- a stable compatibility story for all future format changes

The design document is explicit that some guarantees are only as strong as the OS and filesystem contract underneath them.

## Confidentiality warning

Authenticated encryption is built into the storage model, but the project is still not suitable for protecting real secrets. The implementation is original and unaudited.

## Backup and import/export status

Current surface:

- `backup` exists to copy a database and WAL sidecar together
- inspection and verification tooling exist

What does not exist yet as a polished product surface:

- a mature import/export story
- long-term migration guarantees
- operational tooling for production backup rotation and restore drills

## Concurrency limits

Today the design is intentionally conservative:

- single process
- single writer
- no general multi-process access model
- broader multi-reader concurrency is later roadmap work

If you need server-style concurrent access today, Tosumu is the wrong tool.

## Threat-model limits

The design explicitly does not try to solve everything. Examples outside the intended scope include:

- memory compromise in the running process
- side-channel resistance
- traffic-analysis resistance
- consistent multi-page rollback protection

For the full threat-model and scope statement, use the repository `SECURITY.md` and `DESIGN.md` documents.