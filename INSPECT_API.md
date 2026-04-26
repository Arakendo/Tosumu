# Inspect API

`tosumu-cli` exposes a machine-readable inspection contract for downstream tools such as the TUI, the WPF harness, and future companion tooling.

The inspect JSON contract currently has one baseline schema. The CLI emits that structured schema by default and does not expose a schema selector.

## Common Envelope

Every `tosumu inspect ... --json` command returns the same top-level envelope:

```json
{
  "command": "inspect.header",
  "ok": true,
  "payload": {},
  "error": null
}
```

Fields:

- `command`: stable command identifier such as `inspect.header` or `inspect.verify`.
- `ok`: `true` on success, `false` when the command failed or inspection found a failing status.
- `payload`: command-specific payload. Omitted or `null` on error.
- `error`: structured error payload. Omitted or `null` on success.

Current error payload shape:

```json
{
  "code": "ARGUMENT_INVALID",
  "status": "invalid_input",
  "message": "invalid argument: page number out of range",
  "details": {
    "reason": "page number out of range"
  },
  "pgno": null
}
```

## Current Commands

### `inspect.header`

Returns file-header fields plus slot-0 keyslot metadata.

Important payload fields:

- `format_version`
- `page_size`
- `min_reader_version`
- `flags`
- `page_count`
- `freelist_head`
- `root_page`
- `wal_checkpoint_lsn`
- `dek_id`
- `keyslot_count`
- `keyslot_region_pages`
- `slot0.kind`
- `slot0.kind_byte`
- `slot0.version`

### `inspect.verify`

Returns per-page integrity results plus the B-tree invariant result.

Verification findings and partial verification states remain in the payload. The top-level error envelope is reserved for failures that prevent the command from producing any verify snapshot.

Incomplete verify states should remain in the payload when inspect can still produce a meaningful partial report. Promote them to the top-level error envelope only when the command cannot produce a reliable report envelope at all.

Verify payload findings add stable payload codes for machine handling. These payload codes classify reportable verify states without promoting them to top-level inspect errors.

Important payload fields:

- `pages_checked`
- `pages_ok`
- `issue_count`
- `issues[]`
- `issues[].code`
- `page_results[]`
- `page_results[].issue_code`
- `btree.checked`
- `btree.ok`
- `btree.code`
- `btree.message`

### `inspect.pages`

Returns a lightweight page summary for every data page.

Important payload fields:

- `pages[].pgno`
- `pages[].page_version`
- `pages[].page_type`
- `pages[].page_type_name`
- `pages[].slot_count`
- `pages[].state`
- `pages[].issue`

Page states currently emitted:

- `ok`
- `auth_failed`
- `corrupt`
- `io`

### `inspect.page`

Returns one decoded page and its records.

Important payload fields:

- `pgno`
- `page_version`
- `page_type`
- `page_type_name`
- `slot_count`
- `free_start`
- `free_end`
- `records[]`

Record kinds currently emitted:

- `Live`
- `Tombstone`
- `Unknown`

### `inspect.wal`

Returns the presence and decoded summary of the WAL sidecar.

Important payload fields:

- `wal_exists`
- `wal_path`
- `record_count`
- `records[]`

WAL record kinds currently emitted:

- `begin`
- `page_write`
- `commit`
- `checkpoint`

### `inspect.tree`

Returns the current B-tree root and a recursive tree summary.

Important payload fields:

- `root_pgno`
- `root`
- `root.children[]`
- `root.children[].relation`
- `root.children[].separator_key_hex`

Tree child relations currently emitted:

- `leftmost`
- `separator`

### `inspect.protectors`

Returns configured keyslot / protector summaries.

Important payload fields:

- `slot_count`
- `slots[].slot`
- `slots[].kind`
- `slots[].kind_byte`

## Compatibility Rules

- This contract is the current inspect baseline; do not add version selectors until a real incompatible change exists.
- Prefer one canonical field per concept over compatibility aliases.
- Command identifiers should remain stable once published.
- UI shells should not infer extra meaning beyond what the contract states; Rust remains the source of truth for file semantics.

The canonical Rust definition for the current envelope and payloads lives in [crates/tosumu-cli/src/inspect_contract.rs](crates/tosumu-cli/src/inspect_contract.rs).