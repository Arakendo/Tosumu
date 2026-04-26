# Tosumu

Tosumu is an experimental local database engine focused on inspectability, explicit failure reporting, and simple embeddable use cases.

It is a single-file, single-process, page-based key-value store written in Rust. The project is intentionally small enough to understand end-to-end: file format, pager, B+ tree, WAL, authenticated page encryption, structured errors, and inspection tooling all live in one repository.

## What problem it solves

Tosumu is aimed at the uncomfortable middle ground between "I need something real" and "I do not want a black box." It is designed so you can inspect what the database believes about its own state instead of trusting a silent success path.

Core themes:

- inspectability over magic
- explicit failure reporting over stringly-typed surprises
- authenticated storage from the first real file format onward
- small-scope embedded use cases over server-database ambition

## Current status

Tosumu is in early development and remains a learning project.

- MVP +8 is complete: storage, B+ tree lookup, WAL recovery, key management, and the read-only TUI viewer are implemented.
- The current roadmap still places a toy SQL layer after the storage milestones, but the storage engine remains the core of the project.
- The design is public and detailed, but pre-stability changes are still expected.

## Warning

!!! warning

    Tosumu is not production-ready. Do not use it to protect real secrets or irreplaceable data.

The repository documents the crypto and storage design in detail, but this is not an audited system and not a mature database product.

## Start here

- [Getting Started](getting-started.md) for the shortest path from build to inspect
- [Safety and Limits](safety-and-limits.md) for what the project currently guarantees and what it does not
- [Architecture](architecture.md) for the crate and subsystem layout
- [Inspect API](inspect-api.md) for the machine-readable contract used by tools

## Source-of-truth documents

This site is the public guide, not the canonical engineering spec.

- The full design lives in the repository `DESIGN.md`.
- Structured error behavior is defined in `ERRORS.md`.
- The current inspect JSON contract is defined in `INSPECT_API.md`.

Use this site for orientation and working guidance. Use the repository docs when you need the full design record.