# Concepts

Tosumu is easier to read if you start with a few core ideas.

## Embedded, not server-shaped

Tosumu is a local engine linked into an application or driven through the CLI. It is not a network service and it is not trying to become a Postgres alternative.

## Inspectability is a product goal

The project does not treat inspection as an afterthought. The CLI, JSON inspect contract, and TUI viewer are part of the design, not debugging leftovers.

## Single writer

The current model is one process and one writer. That simplifies correctness, file locking, and failure reporting. Multi-reader concurrency belongs later in the roadmap.

## Authenticated pages

Every on-disk page is protected with AEAD. Page number, version, and type are bound as additional authenticated data, so the engine can reject page swaps, some rollback classes, and type confusion attacks.

## Envelope key management

The database uses a random DEK for page encryption. Protectors such as passphrases and recovery keys derive KEKs that wrap the DEK in keyslots. That separation is what makes cheap KEK rotation possible.

## Structured failures

Errors that cross a boundary are meant to carry a code, a status, a human-readable message, and structured details. Downstream tools should not have to reverse-engineer behavior from strings.

## Public design, unstable implementation

The design is documented in unusual depth for a project at this stage. That does not mean the on-disk format or roadmap are permanently frozen. The project is still pre-stability.