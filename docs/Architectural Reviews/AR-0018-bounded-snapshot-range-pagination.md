# AR-0018: Bounded Snapshot Range Pagination

| Field | Value |
| --- | --- |
| Status | Incubating |
| Opened | 2026-09-03 |
| Last reviewed | 2026-09-03 |
| Scope | Provider-neutral snapshot range reads / pagination / resource limits |
| Trigger | MVP+11 needs a bounded foreign range operation, while the accepted snapshot scan fully materializes its range before an adapter can impose a limit |
| Related ADRs | ADR-0002, ADR-0003, ADR-0005, ADR-0006 |
| Related reviews | AR-0007, AR-0009, AR-0017 |
| Related evidence | `btree::read::scan_from`, `KvReadTransaction::scan`, MVP+11 independent C harness |

## Architectural Question

What provider-neutral snapshot range contract can bound traversal and
allocation before values are materialized, preserve inclusive ordered scan
meaning, and support deterministic continuation without exposing B+ tree or
WAL structure?

## Context

ADR-0006 accepts `KvReadTransaction::scan(start, end)` as an inclusive ordered
read pinned to one committed generation. Its implementation descends to the
first leaf, walks the leaf chain, inserts every selected pair into a `BTreeMap`,
and finally collects the complete range into a `Vec`.

That is coherent for the accepted API, but an adapter cannot make it bounded by
truncating the returned vector: all leaf traversal, overflow reads, allocation,
and value copies have already occurred. MVP+11 therefore exposed point reads
and bounded connection observations but correctly withheld range exports.

The requirement is broader than C. Native services, SQL scans, mobile callers,
and future evidence exporters all benefit from a provider-neutral bounded
primitive. The contract must remain about logical ordered key/value data, not
physical pages or a foreign encoding.

## Governing Invariants

- Every page is read from the same `KvReadTransaction` generation pin.
- Results remain ordered by raw key bytes and contain no duplicate key.
- The lower bound of the first page and upper bound are inclusive, matching the
  accepted scan contract.
- A continuation identifies the first unconsumed logical key and resumes
  inclusively. It is not a page number, slot, WAL offset, or physical cursor.
- Pair and logical payload-byte limits are enforced during traversal, before an
  excluded overflow value is read or allocated.
- Logical payload bytes mean `key.len() + value.len()`. Adapter framing and copy
  budgets are separate, derived bounds.
- The payload limit does not bound every byte owned by the result: a blocked
  page returns one continuation key from the excluded entry. That additive
  allocation is independently bounded by Tosumu's `MAX_KEY_SIZE`; adapters must
  include one maximum-sized key plus representation overhead in their envelope.
- Zero limits and inconsistent bounds fail before traversal with a typed
  invalid-argument result.
- Corruption, authentication, and snapshot-residence failures retain their
  existing typed identity and poison behavior.
- Pagination changes no on-disk bytes, checkpoint behavior, pin lifetime, or
  generation meaning.

## Candidate Contract

The leading candidate is a stateless page operation on `KvReadTransaction`:

```text
scan_page(
    start_inclusive,
    end_inclusive,
    maximum_pairs,
    maximum_payload_bytes,
) -> KvScanPage

KvScanPage {
    pairs,
    next_start_inclusive,
    blocked_entry_payload_bytes,
}
```

`next_start_inclusive == None` means the range is exhausted. When another entry
exists but would exceed either limit, it contains that first unconsumed key.
The caller supplies it as the next page's `start_inclusive`; the snapshot pin
makes this deterministic even while writers commit later generations.

`blocked_entry_payload_bytes` is present only when the first unconsumed entry
cannot fit the remaining byte budget. This allows a caller to distinguish
ordinary pair-limit pagination from a page size too small for one entry without
reading or returning that entry's overflow value. If no pair has yet been
returned, the caller must increase its admitted byte limit or stop; retrying the
same page unchanged makes no progress.

Discovering that result may decode the containing leaf and copy the
continuation key, but it must not read, authenticate, or allocate the excluded
overflow value. The leaf page and continuation key have independent fixed
bounds; neither is charged to the admitted logical payload.

The admitted Rust experiment uses one public owned `KvScanPage` result with
public `pairs`, `next_start_inclusive`, and `blocked_entry_payload_bytes`
fields. `KvReadTransaction::scan_page` accepts `maximum_pairs: usize` and
`maximum_payload_bytes: u64` directly. A separate limits type, mutable cursor,
and accessor-only result add no demonstrated value yet. The C adapter must use
fixed-width ABI inputs and checked conversion; its representation does not
change the Rust contract.

The method returns the existing `InvalidArgument` identity when either limit is
zero or `start_inclusive > end_inclusive`. Large admitted limits do not cause
proportional preallocation. There is no separate implementation maximum beyond
the representable argument and existing key/value format bounds.

## Alternatives Considered

### Truncate the existing materialized scan in the adapter

Rejected. It limits output presentation after unbounded traversal and
allocation and would make the ABI's resource-bound claim false.

### Return one fully materialized range handle

Rejected for the same reason. Opaque ownership does not make work bounded.

### Expose physical page and slot cursors

Rejected. It leaks storage mechanics across ADR-0001/0002, couples callers to
tree layout, and makes compaction or format evolution a logical API change.

### Add an owned mutable core cursor

Plausible later, especially if measured repeated descent is material. It adds
cursor state, thread/lifetime rules, poisoning, and another owned capability
before evidence requires them. The stateless logical continuation is the
smaller first experiment.

### Use an exclusive `after` key

Plausible but more error-prone across first and later calls. Returning the first
unconsumed key and always using an inclusive lower bound keeps one operation
shape and avoids synthesizing a successor for arbitrary bytes.

## Evidence Required Before ADR Amendment

- focused leaf-boundary, empty-range, exact-limit, pair-limit, and oversized-
  inline/overflow tests;
- property tests showing concatenated pages equal the existing complete scan
  for the same snapshot and bounds;
- fault tests preserving corruption/authentication outcomes;
- an integration test outside the core crate using only the public contract;
- the independent C caller consuming at least two pages and one blocked-entry
  result without callbacks or raw physical state; and
- a conservation run covering existing snapshot, WAL-retention, recovery, and
  SQL callers.

## Disposition

Incubating. The private traversal and public result vocabulary have cleared
their gates, and `KvReadTransaction::scan_page` is admitted as a public Rust
experiment with an external integration caller. Do not add C range symbols
until that adapter preserves fixed-width limit conversion, owned-result
lifetime, multi-page continuation, and blocked-entry meaning. Amend ADR-0006
only after the independent Rust and C callers establish the contract.

## Reopening Triggers

- the private prototype cannot avoid materializing an excluded overflow value;
- leaf-boundary continuation requires physical state or ambiguous lookahead;
- repeated root descent is measured as materially harmful;
- SQL needs cancellation, predicates, projections, or reverse traversal that
  cannot compose above this primitive;
- a consumer requires a durable continuation across close/reopen; or
- pair/payload limits do not predict adapter memory closely enough.

## Review History

### Cycle 1 -- 2026-09-03

- Status entering review: Proposed by AR-0017 Cycle 6 evidence.
- New evidence: direct implementation inspection confirms the accepted scan
  fully materializes the range before return.
- Findings: post-hoc adapter truncation is inadmissible; a logical first-
  unconsumed-key continuation can remain provider-neutral and snapshot-pinned.
- Disposition: Incubating. Admit a private traversal prototype and focused
  equivalence/resource-bound tests; withhold public core and C APIs.
- Resulting ADR or documentation change: AR-0018 opened; no ADR or API change.

### Cycle 2 -- 2026-09-03

- Status entering review: Incubating; private traversal prototype admitted.
- New evidence: the private snapshot traversal enforces pair/payload limits
  before overflow materialization. Focused tests concatenate 160 rows across
  multiple leaf/page limits exactly to the complete scan, report an excluded
  multi-page overflow value by declared logical size, and exercise a 1,000-byte
  continuation key whose key alone exceeds the page budget. A 24-case property
  varies bounds and limits and reproduces the existing complete snapshot scan.
- Findings: first-unconsumed inclusive continuation is workable without
  physical state. The logical payload budget is not a total-memory budget: one
  continuation key is an explicit additive allocation bounded by
  `MAX_KEY_SIZE`. Excluded overflow bytes need not be materialized.
- Disposition: remain Incubating. Add malformed-length/corruption and exact
  leaf-boundary falsifications, then review the public result vocabulary and
  independent Rust caller before amending ADR-0006.
- Resulting ADR or documentation change: private prototype and conservation
  properties only; no public method or C range export.

### Cycle 3 -- 2026-09-03

- Status entering review: Incubating; boundary and corruption falsifications
  required before reviewing a public contract.
- New evidence: a focused test derives a real leaf boundary from the built tree
  and exhausts both limits exactly at that boundary; the continuation is the
  first key in the next leaf and the resumed page conserves the complete scan.
  Fault tests then replace an overflow chain with a cycle. When the declared
  logical size exceeds the admitted budget, the page returns the blocked key
  and size without reading the excluded chain. Raising the budget admits the
  same entry and preserves the existing `OverflowChainCorrupt` result. A
  declared length above `MAX_VALUE_SIZE` is rejected immediately even when the
  entry would otherwise be excluded.
- Findings: logical continuation needs no physical lookahead at a leaf
  boundary. Bounded exclusion may deliberately defer corruption that exists
  only inside unread overflow pages, but it does not defer validation of the
  trusted leaf metadata used to make the exclusion decision. Once an entry is
  admitted, existing overflow validation and typed failures remain conserved.
- Disposition: remain Incubating. The private traversal has cleared its
  implementation-falsification gate. Review and admit the smallest public Rust
  result vocabulary, then require an integration test outside `tosumu-core`
  before exposing the operation to C.
- Resulting ADR or documentation change: no public API or ADR amendment yet;
  the next gate changes from traversal feasibility to public-contract evidence.

### Cycle 4 -- 2026-09-03

- Status entering review: Incubating; private traversal feasibility established
  and public result vocabulary unresolved.
- New evidence: Tosumu's accepted provider boundary already uses owned result
  structs with directly readable fields, `usize` for host collection counts,
  and `u64` for byte-size observations. The prototype allocates incrementally
  and does not reserve from either supplied limit.
- Findings: one `KvScanPage` type and one `KvReadTransaction::scan_page` method
  are sufficient for an independent Rust caller. A limits object, cursor
  capability, builder, or adapter-shaped integer vocabulary would add contract
  surface without evidence. Fixed-width conversion belongs at the C boundary.
- Disposition: remain Incubating, but admit this public Rust experiment. Require
  an external integration caller to establish usability and conservation before
  adding C symbols. This admission does not yet amend ADR-0006.
- Resulting ADR or documentation change: candidate Rust names, field visibility,
  argument types, validation, and allocation behavior are fixed for the next
  experiment.

### Cycle 5 -- 2026-09-03

- Status entering review: Incubating; public Rust experiment admitted but not
  implemented or independently consumed.
- New evidence: `KvScanPage` and `KvReadTransaction::scan_page` now project the
  private bounded traversal without exposing pager or tree state. A separate
  integration-test crate surface consumes only public exports, concatenates
  more than two pages to the accepted complete scan while a writer advances,
  observes the captured value, handles and retries a 20,000-byte blocked
  overflow entry, and receives the existing typed invalid-argument outcome for
  zero limits and inverted bounds. `cargo test --workspace --all-targets`
  passed, including existing Criterion benchmark targets, recovery properties,
  maximum-value provider coverage, SQL, inspection, and experimental FFI
  consumers; tests already marked ignored remained explicitly unrun.
- Findings: the public owned-result vocabulary is sufficient for a real Rust
  caller and conserves generation pinning and existing errors. Neither the
  limits nor the continuation require a new owned capability or preallocation.
- Disposition: remain Incubating. Admit a private C adapter experiment with
  checked fixed-width conversions and owned result handles. Require the C
  harness to consume multiple pages and a blocked entry before ADR amendment.
- Resulting ADR or documentation change: public Rust experiment and independent
  integration evidence established; no C symbols or ADR amendment yet.

## References

- `../ADR/ADR-0001-storage-engine-layer-boundaries.md`
- `../ADR/ADR-0002-authenticated-pager-trust-boundary.md`
- `../ADR/ADR-0005-committed-generation-and-retained-wal-snapshots.md`
- `../ADR/ADR-0006-shared-kv-store-and-snapshot-transactions.md`
- `AR-0017-mobile-embedding-abi-and-hardware-protector-boundary.md`
- `../Plans/mvp-11-mobile-embedding.md`
