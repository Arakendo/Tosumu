# Inspection Header Fixture v1

`inspection-header-fixture-v1.tosumu` is a reviewed browser-lab fixture for
the Tosumu inspection island. It is a fresh, unencrypted Tosumu store created
with the public `tosumu init` command.

- Format version: `2`
- Size: `8192` bytes
- Page size: `4096` bytes
- Root page: `1`
- Protector: sentinel authentication only
- User records: none
- Credentials: none
- SHA-256: `5c1f3f353907d2dcc0f9fb22b87e64a5c2b174b8a75b8b535d5831e4a383d88f`

The island fetches this binary as an opaque byte buffer and passes it to the
same Rust/WASM `inspect_uploaded_bytes` boundary used for browser uploads.
JavaScript does not parse, unlock, or mutate the fixture. It exists only to
make the header-only browser proof immediately runnable.
