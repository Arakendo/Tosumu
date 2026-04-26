# File Format

Tosumu has a documented on-disk format with a fixed page size and a separate WAL sidecar. That format is implemented, but it is still pre-stability.

## Stable enough to document, not stable enough to freeze

Today you should read the format as:

- real
- implemented
- worth documenting
- still subject to pre-stability change

This site gives the public outline. The repository design document remains the deeper reference for field-by-field detail.

## High-level layout

The main database file contains:

- page 0: file header
- one or more keyslot-region pages after page 0
- data pages after that

The WAL currently lives in a separate sidecar file.

## Current important properties

- fixed 4096-byte page size
- little-endian integers
- plaintext header fields needed for format and protector discovery
- authenticated page encryption for data pages
- keyslot region authenticated by a header MAC
- B+ tree pages, overflow pages, and free pages as distinct page types

## What the format is trying to optimize for

- inspectability
- explicit type information
- authenticated storage semantics
- predictable crash-recovery behavior

It is not trying to optimize for extreme density, compatibility with SQLite, or minimizing every byte of metadata.

## What is still explicitly unsettled

- long-term compatibility guarantees
- future format migrations after pre-stability
- later entropy-bookkeeping fields discussed in the roadmap
- any future additions needed for advanced witness or SQL-adjacent work

## Read the full spec when you need exact bytes

For exact header fields, page-frame layout, keyslot region details, and roadmap-linked format changes, use the repository design document's file-format and cryptography sections.