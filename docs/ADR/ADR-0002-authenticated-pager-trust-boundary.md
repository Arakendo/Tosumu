# ADR-0002: Authenticated Pager Trust Boundary

## Status

Accepted

## Context

Tosumu stores pages in an adversarial persistence domain while the B+ tree,
transaction, recovery, and inspection layers operate on trusted plaintext page
structures. Authentication and encryption cannot be optional decorations on
file I/O: the location where bytes cross between those domains determines
which layers may trust them.

The implemented pager, cryptography design, security policy, corruption tests,
and inspect diagnostics already enforce this boundary. This ADR records that
existing architecture without expanding Tosumu's security claims beyond the
documented threat model.

## Decision

- The pager is the authenticated storage trust boundary.
- Data pages crossing from persistent storage into trusted memory are
  authenticated before their plaintext is exposed to higher layers.
- Data pages crossing from trusted memory to persistent storage are encrypted
  and authenticated before lower file-I/O mechanisms receive them.
- Layers above the pager operate on validated plaintext page structures and do
  not handle ciphertext.
- File-I/O mechanisms below the pager operate on encoded page bytes and do not
  receive plaintext page structures.
- Page identity, format version, and page type remain bound into page
  authentication metadata so valid ciphertext cannot be silently reassigned
  to another page role.
- Tosumu does not provide an unauthenticated data-page mode. A sentinel or
  development protector may provide integrity without meaningful secrecy, but
  it does not remove authentication.
- Header and keyslot discovery remain explicit format exceptions with their
  own authentication rules; they do not weaken the data-page boundary.

```text
B+ tree / transactions / recovery / inspection
                    ↓ trusted plaintext
                 Pager
          authenticate / decrypt
          encrypt / authenticate
                    ↓ adversarial bytes
               File I/O / disk
```

## Consequences

- Authentication failure is a structured integrity failure, not a request to
  continue with partially trusted page contents.
- New page types and recovery paths must cross the same pager boundary rather
  than adding alternate plaintext file paths.
- Storage adapters can rely on the pager's validation contract without owning
  cryptographic mechanisms.
- This decision does not claim audited confidentiality, protection from a
  compromised process, multi-page rollback prevention, remote attestation, or
  freshness against an external witness.
- Changes to page authentication, associated data, or trust-boundary placement
  require architectural review and an explicit ADR revision or supersession.

## Alternatives Considered

- **Optional encryption above the pager.** Rejected because higher layers
  could accidentally bypass authentication and would need to understand page
  encoding.
- **Encryption in raw file I/O.** Rejected because file mechanisms do not own
  page identity, type, version, or structured integrity errors.
- **Allow plaintext data pages for development.** Rejected because it creates
  two storage personalities and makes authentication an accidental runtime
  option.

## References

- `docs/Specifications/Tosumu Software Design Document.md`, sections 4, 5, 6, and 8
- `SECURITY.md`
- `docs/architecture.md`
- `docs/file-format.md`
- `docs/error-model.md`
- `ADR-0001-storage-engine-layer-boundaries.md`

