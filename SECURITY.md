# Security Policy

| Field | Value |
| --- | --- |
| Authority | Normative security posture and disclosure policy |
| Lifecycle | Current, pre-audit |
| Scope | Threat model, explicit limitations, and vulnerability reporting |

## Pre-audit status

`tosumu` is an **early-stage database project**. It implements authenticated encryption, envelope key management, and a write-ahead log, but these mechanisms have not been independently reviewed or audited.

> **Do not use `tosumu` to protect real secrets.**
>
> Use a mature, reviewed storage engine (e.g. SQLCipher, age-encrypted storage, or a purpose-built vault) if you need production-grade confidentiality or integrity.

## Threat model

See `docs/Specifications/Tosumu Software Design Document.md §8.1` (in scope) and `docs/Specifications/Tosumu Software Design Document.md §8.10–8.11` (explicit non-goals and known limitations). In brief:

**In scope**

- Attacker with read/write access to the database file at rest.
- Page swap, page rollback (single-page), page reorder, truncation, bit-flipping.
- Wrong-passphrase / wrong-protector rejection.

**Out of scope**

- Attacker with memory access to the running process.
- Side channels (cache timing, power, microarchitectural).
- Traffic analysis of file-modification patterns.
- Plaintext recovery from OS swap, hibernation, or crash dumps.
- Consistent multi-page rollback (acknowledged limitation; see `docs/Specifications/Tosumu Software Design Document.md §5.3`).
- Remote attestation, network key escrow, KMS integration.

## Authority and input boundaries

Security includes integrity and availability at storage boundaries, not only
secret handling. Implemented and future adapters follow these rules:

- possession or discovery of a database, provider, command, or host capability
  does not grant unrelated observation or mutation authority;
- successful parsing and authentication do not establish authorization,
  freshness, semantic validity, or trust in application meaning;
- files, uploaded bytes, serialized state, provider responses, host messages,
  and remote input remain untrusted until they are structurally validated and
  resource-bounded for the operation being attempted;
- adapters receive the narrowest storage operations and data projections needed
  for their responsibility rather than pager, key, filesystem, or process
  authority by default;
- missing, stale, revoked, unknown, or expanded authority fails explicitly at
  the protected boundary; and
- diagnostics must preserve useful bounded provenance without exposing keys,
  passphrases, decrypted values, unnecessary paths, or arbitrary payloads.

These rules do not claim process isolation, multi-tenant security, a stable
authorization API, or an implemented remote-service policy. Those questions
remain under `docs/Architectural Reviews/AR-0003-service-authority-and-host-modes.md`.

## Reporting a vulnerability

If you believe you have found a cryptographic or integrity-affecting flaw in the design or implementation, please report it privately rather than by public issue.

- **Preferred:** use GitHub's [private vulnerability reporting](https://github.com/Arakendo/tosumu/security/advisories/new) if enabled on the repository.
- **Fallback:** open an issue titled `SECURITY: <short summary>` and immediately email the maintainer without including exploit details.

Because this is a pre-stability project maintained on a best-effort basis, please understand:

- There is **no SLA** for response or fix.
- There is **no coordinated-disclosure pipeline** and no CVE assignment process.
- Fixes land in `main`; there are no backport branches.

## Scope of "security" fixes

Issues that will be taken seriously:

- Bypass of AEAD verification.
- Key leakage (in-memory or on-disk) outside the documented threat model.
- AAD construction flaws that allow page swap / rollback / reorder beyond what `docs/Specifications/Tosumu Software Design Document.md` already calls out as known limitations.
- Any cryptographic primitive misused against its documented constraints.

Issues that are **expected behavior** and not bugs:

- The attacker can tell how big the database file is.
- The attacker can tell which pages changed between snapshots.
- Keys in process memory are readable by other code in that process.
- Losing all protectors makes the database unrecoverable (that is the point).
- Downgrading to an older `format_version` is not supported.
- A committed generation or reader snapshot is local visibility evidence. It
  does not prove freshness against a consistent rollback of the database and
  WAL by an attacker.

## Dependencies

`tosumu` uses audited primitives from the [RustCrypto](https://github.com/RustCrypto) ecosystem (`chacha20poly1305`, `hmac`, `sha2`, `hkdf`, `argon2`) and avoids hand-rolled cryptographic primitives. The *composition* of those primitives is original and is the part most likely to contain flaws.
