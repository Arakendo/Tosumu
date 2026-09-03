# MVP+10 Benchmark Closure

| Field | Value |
| --- | --- |
| Status | Active |
| Opened | 2026-09-02 |
| Last updated | 2026-09-02 |
| Owner | Tosumu maintainers |
| Target | MVP+10 concurrency and comparison evidence |
| Related ADRs | ADR-0005, ADR-0006, ADR-0008, ADR-0009 |
| Related reviews | AR-0009, AR-0014 |
| Depends on | `SharedKvStore`, retained snapshots, existing SQLite benchmark harness |

## Status

The existing Criterion suite compares plain `PageStore` and SQLite WAL-mode
insert, point lookup, range scan, and full scan workloads. It does not exercise
the public MVP+10 shared owner, reader scaling, or a writer committing while
snapshots remain pinned. This plan adds that missing evidence without turning
measurements into performance guarantees.

## Purpose

Close MVP+10 with repeatable measurements for its defining concurrency path and
an honest SQLite comparison. The benchmark must also assert the snapshot result
it times, so a fast visibility regression cannot be recorded as improvement.

## Method And Boundaries

- Use deterministic 8-byte ordered keys, 128-byte values, and 10,000 preloaded
  records, matching the retained B+ tree comparison where practical.
- Measure 1-, 4-, and 8-reader point-lookup fanout through public
  `SharedKvStore` snapshots and SQLite WAL read transactions.
- Measure one writer commit while four readers retain a pre-commit snapshot;
  each reader must observe the old value before and after the commit, and a
  fresh read must observe the new value.
- Report logical operations as Criterion throughput and retain environment and
  command details with any published result.
- SQLite uses one connection per reader thread, while Tosumu uses clones of its
  intended shared owner. That API/lifecycle difference is part of the observed
  workload and must be disclosed with results.

These measurements do not establish latency, fairness, scaling, durability, or
performance guarantees on other hardware, filesystems, builds, or datasets.

## Non-Goals

- Changing storage, MVCC, checkpoint, SQL-index, or file-format behavior.
- Adding an async executor, connection pool, waiting policy, or cross-process
  reader protocol.
- Treating SQLite configuration as a universal external baseline.
- Checking generated Criterion reports into source control.

## Compatibility And Security

The work is confined to `tosumu-bench` and documentation. It adds no public API,
dependency, on-disk change, security claim, or production failure behavior.
Synthetic values contain no secrets.

## Deliverables

- [x] Inventory the retained benchmark and identify the missing MVP+10 paths.
- [ ] Add public shared-snapshot reader-scaling comparisons against SQLite WAL.
- [ ] Add reader/writer overlap comparisons with assertions inside the workload.
- [ ] Run benchmark smoke tests, strict Clippy, and release-mode measurements.
- [ ] Record environment, commands, observations, and limitations.
- [ ] Reconcile the roadmap and mark MVP+10 complete only after native Unix
      VACUUM CI and benchmark evidence both pass.

## Validation Matrix

| Concern | Evidence | Required result |
| --- | --- | --- |
| Benchmark compilation | `cargo test -p tosumu-bench --all-targets` | Pass |
| Lint boundary | `cargo clippy -p tosumu-bench --all-targets -- -D warnings` | Pass |
| Reader fanout | `cargo bench -p tosumu-bench --bench mvp10_concurrency -- concurrent_readers` | Results for both engines at 1/4/8 readers |
| Reader/writer overlap | `cargo bench -p tosumu-bench --bench mvp10_concurrency -- reader_writer` | Both engines retain old snapshots and publish new value |
| Existing comparison | `cargo bench -p tosumu-bench --bench btree_vs_sqlite` | Existing plain workloads still run |
| Documentation | `mkdocs build --strict` | Pass |

## Risks And Mitigations

| Risk | Impact | Mitigation Or Evidence |
| --- | --- | --- |
| Thread creation dominates small operations | Results describe fanout workload rather than isolated lookup latency | Use a fixed multi-operation batch and disclose included lifecycle cost |
| SQLite/Tosumu APIs are not identical | Numbers can be overgeneralized | Match snapshot semantics and disclose connection ownership difference |
| Assertions perturb timing | Absolute time includes validation work | Keep equivalent correctness checks on both sides and prioritize trustworthy evidence |
| CI benchmark smoke runs become slow | Feedback degrades | Keep data and operation counts bounded; full sampling remains an explicit command |

## Progress Log

### 2026-09-02

- Work completed: inventoried the existing four plain and optional encrypted
  SQLite comparison groups and confirmed they use `PageStore`, not MVP+10's
  shared snapshot API.
- Validation: the pre-slice workspace all-target gate and current benchmark
  smoke workloads pass.
- Findings: no architecture decision or new dependency is required; benchmark
  construction belongs in the existing non-published benchmark crate.
- Next slice: implement deterministic reader fanout and reader/writer overlap.

## References

- `crates/tosumu-bench/benches/btree_vs_sqlite.rs`
- `docs/Plans/mvp-10-multiple-readers.md`
- `docs/Plans/main-feature-roadmap.md`
- `docs/ADR/ADR-0005-committed-generation-and-retained-wal-snapshots.md`
- `docs/ADR/ADR-0006-shared-kv-store-and-snapshot-transactions.md`
