# Inspection Header Fixture v1

`inspection-header-fixture-v1.tosumu` is a reviewed browser-lab fixture for
the Tosumu inspection island. It is a fresh, unencrypted Tosumu store created
with the public `tosumu init` command.

- Format version: `3`
- Size: `8192` bytes
- Page size: `4096` bytes
- Root page: `1`
- Protector: sentinel authentication only
- User records: none
- Credentials: none
- SHA-256: `60f92ff42944f907884a23dd20455929e501f5c635d2af4b513a669b060428fa`

The island fetches this binary as an opaque byte buffer and passes it to the
same Rust/WASM `inspect_uploaded_bytes` boundary used for browser uploads.
JavaScript does not parse, unlock, or mutate the fixture. It exists only to
make the header-only browser proof immediately runnable.
