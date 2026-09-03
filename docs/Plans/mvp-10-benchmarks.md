# MVP+10 Benchmark Closure

| Field | Value |
| --- | --- |
| Status | Complete |
| Opened | 2026-09-02 |
| Last updated | 2026-09-02 |
| Owner | Tosumu maintainers |
| Target | MVP+10 concurrency and comparison evidence |
| Related ADRs | ADR-0005, ADR-0006, ADR-0008, ADR-0009 |
| Related reviews | AR-0009, AR-0014 |
| Depends on | `SharedKvStore`, retained snapshots, existing SQLite benchmark harness |

## Status

The Criterion suite now covers the public MVP+10 shared owner at 1, 4, and 8
reader threads and a four-reader/one-writer overlap, alongside the retained
plain SQLite comparisons. The overlap workload uncovered and drove a fix for a
duplicate-heavy leaf split bug before results were accepted. Native Unix VACUUM
CI remains the separate final MVP+10 milestone gate.

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
- [x] Add public shared-snapshot reader-scaling comparisons against SQLite WAL.
- [x] Add reader/writer overlap comparisons with assertions inside the workload.
- [x] Run benchmark smoke tests, strict Clippy, and release-mode measurements.
- [x] Record environment, commands, observations, and limitations.
- [x] Reconcile the roadmap without marking MVP+10 complete before native Unix
      VACUUM CI passes.

## Retained Observations

Measured 2026-09-02 on x86_64 Windows 10.0.26200, AMD Family 25 Model 97, with
Rust 1.95.0 and Criterion's release profile. Values below are Criterion point
estimates; generated reports remain under ignored `target/criterion/`.

| Workload | Tosumu | SQLite |
| --- | ---: | ---: |
| 1 reader, 128 lookups | 8.130 ms / 15.745 Kelem/s | 1.902 ms / 67.302 Kelem/s |
| 4 readers, 512 lookups | 33.895 ms / 15.105 Kelem/s | 4.646 ms / 110.21 Kelem/s |
| 8 readers, 1,024 lookups | 66.669 ms / 15.359 Kelem/s | 7.746 ms / 132.20 Kelem/s |
| 4 pinned readers + 1 writer | 1.984 ms / 4.536 Kelem/s | 6.401 ms / 1.406 Kelem/s |

Tosumu's reader-only throughput is effectively flat as reader count grows,
which matches the current `Arc<Mutex<BTree>>` owner: snapshots are coherent and
movable between threads, but individual reads do not execute in parallel. The
overlap result includes thread creation, snapshot/connection lifecycle,
correctness assertions, and one durable writer operation; it must not be read as
an isolated write-latency comparison.

The retained single-thread suite observed Tosumu/SQLite point estimates of
57.189/12.370 ms for 1,000 inserts, 44.927/4.073 us for point lookup,
108.29/18.755 us for a 100-row range scan, and 6.678/1.406 ms for a 10,000-row
full scan. Dataset, API, durability configuration, and machine constraints make
these comparison evidence rather than a general ranking.

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

### 2026-09-02 -- implementation and measurement

- Work completed: added both benchmark groups through public `SharedKvStore`
  and SQLite WAL transactions, with snapshot assertions in the timed overlap.
- Validation: bench all-target smoke and strict Clippy passed; both release
  benchmark commands and the retained plain comparison completed.
- Findings: the first repeated overlap run found a fresh-read disappearance on
  a duplicate-heavy hot leaf. Regression reduction failed at revision 56;
  commit `3e3a06f` compacts the deduplicated live set before splitting and the
  160-cycle public regression now passes.
- Plan changes: benchmark work is complete. The later stable macOS arm64 job in
  CI run `33812169906` executed the complete workspace suite and closed the
  native Unix VACUUM gate; MVP+10 is complete.

## References

- `crates/tosumu-bench/benches/btree_vs_sqlite.rs`
- `docs/Plans/mvp-10-multiple-readers.md`
- `docs/Plans/main-feature-roadmap.md`
- `docs/ADR/ADR-0005-committed-generation-and-retained-wal-snapshots.md`
- `docs/ADR/ADR-0006-shared-kv-store-and-snapshot-transactions.md`
