# Tosumu Format-v3 Independent Crypto Oracle

This tool independently executes Tosumu's deterministic format-v3
cryptographic corpus using Go's standard-library HMAC/HKDF/SHA-256 and pinned
`golang.org/x/crypto` Argon2id and ChaCha20-Poly1305 implementations.

It is evidence tooling, not a runtime backend, provider API, supported database
implementation, or compliance boundary. It must not be linked into
`tosumu-core` or shipped in Tosumu release artifacts.

Run from this directory:

```text
go test ./...
go run . testdata/format-v3-v1.json
```

Successful output reports only schema case counts. The tool does not print
keys, plaintext, ciphertext, MACs, or provider internals. Unknown corpus schema
or format versions fail closed.

The module pins Go 1.26.8 and exact module versions through `go.mod` and
`go.sum`. See `docs/Notes/crypto-c2-oracle-provenance-v1.md` for the retained
dependency and claim boundary.

