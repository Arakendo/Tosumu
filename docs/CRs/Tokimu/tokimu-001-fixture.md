# TOKIMU-001 Shared Consumer Fixture

This fixture is intentionally application-shaped but Tosumu-semantic-free. It
uses only the public `tosumu_core::{KvStore, backup, export, inspect}` APIs.

## Contract

- Fixture schema: `fixture-schema-v1`
- Tosumu crate version: `0.0.0`
- Physical format: `2`
- Hash algorithm: SHA-256 of each value, represented as lowercase hexadecimal
- Key ordering for comparisons: ascending bytewise key order, matching `scan()`
- Large payload: 1 MiB overflow-backed value
- Source state: one committed transaction containing all records

The versioned machine-readable manifest is
[`tokimu-001-fixture.json`](tokimu-001-fixture.json). Consumers must construct
each value from its declared encoding and compare SHA-256 hashes, rather than
interpreting the metadata as a storage-format artifact.

## Records

| Key | Value | Size | SHA-256 |
| --- | --- | ---: | --- |
| `asset/manifest` | `fixture-schema-v1` | 17 | `812018d3e17a8663f453a933c919466361ce1306b0b9a815f4528e24bf6bdf86` |
| `asset/provenance` | `source:tokimu-test\\nrevision:0001` | 32 | `01de4a1f4d75760c842b6c9418f31af03ea99fcc694c9ec53a0b8beac44b9034` |
| `asset/dependencies` | `base-material\\nshared-mesh` | 25 | `1ea88b449d14b15c0ec299a43d7cdd41fd92a1e0d44813ea5430b9a07385240b` |
| `asset/diagnostics` | `warning:fixture-only\\nstatus:clean` | 33 | `767e326f59387bdfe0a3df510c3d51e8650a52dd762155cd8653d2698596b584` |
| `asset/payload-small` | `00 01 fe ff` | 4 | `c5dbae22661af6db18a1f676db82a7ef7de46d27c3a263a872f00478b0d99fc4` |
| `asset/payload-large` | bytes `00..ff` repeated | 1,048,576 | `fbbab289f7f94b25736c58be46a994c441fd02552cc6022352e3d86d2fab7c83` |

## Evidence Test

The repeatable implementation is
`external_consumer_fixture_round_trips_backup_export_and_verification` in
`crates/tosumu-core/tests/provider_boundary.rs`.

It proves:

1. all records commit atomically through one transaction;
2. source reopen preserves exact value hashes;
3. stable backup preserves exact value hashes and the WAL pair;
4. portable export preserves exact value hashes and has no WAL sidecar; and
5. core verification reports all pages valid and the B-tree valid.

The test does not claim 16 MiB or 64 MiB fixture coverage; those payload sizes
remain covered by the separate large-value tests and measurements remain open.
