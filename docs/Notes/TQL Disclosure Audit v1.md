# TQL Disclosure Audit v1

## Scope

This audit covers the currently implemented, read-only TQL commands:

```text
STATUS
CHECK
DESCRIBE <key>
WAL STATUS
```

It records the disclosure boundary of the CLI-local renderer. It is not a
general storage-security claim and does not establish encrypted-store behavior,
freshness, provenance, or trust.

## Findings

| Surface | May emit | Must not emit |
| --- | --- | --- |
| `STATUS` | Page counts and tree height | Store values, identity claims, format secrets |
| `CHECK` | Public verification states and issue counts | Protected contents or a truth verdict |
| `DESCRIBE <key>` | The requested key, found/missing state, and value byte length | Stored value contents |
| `WAL STATUS` | Sidecar existence and decoded record count | Physical sidecar path, record payloads, recovery or durability claims |
| TQL parse failure | Typed code, bounded counts, and command grammar fields | Store contents, unlock material, or protector data |

The parser accepts UTF-8 command text only. It has no password, recovery key,
protector, or binary-input argument. A rejected command token can appear in
structured parse diagnostics because it is caller-supplied command text, not a
TQL secret-bearing field. Callers must not place secrets in diagnostic command
text.

## Evidence

- `description_renderers_never_receive_or_emit_stored_value_contents` proves
  the public `DescriptionOutcome` cannot carry a stored-value sentinel into
  either human or JSON output.
- `json_wal_status_output_exposes_only_bounded_wal_facts` proves the physical
  WAL path is absent from JSON output.
- Bounded parser and JSON property tests constrain command/key input and
  output growth for the implemented surface.

## Limits And Reopening

- This audit does not prove that every future provider error is safe to render.
- A TQL command accepting unlock material, byte input, mutation payloads, or
  protected metadata requires a new disclosure review before admission.
- A future provider-neutral metadata observation may remove `DESCRIBE`'s
  current load-and-discard implementation, but must preserve this no-value
  rendering contract.
