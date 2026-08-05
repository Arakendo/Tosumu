# TQL Runtime Observation v1

## Purpose

Record one reproducible, local timing observation for the bounded TQL CLI
surface. This note is diagnostic evidence, not a latency, throughput, or
allocation contract.

## Command

```text
tosumu tql target/tql-timing-observation.tsm "STATUS" --json --timings
```

The fixture was a newly initialized, unencrypted local store with two pages and
one data page. The command produced the ordinary `STATUS` JSON on standard
output and this opt-in diagnostic on standard error:

```text
tql timings: parse_us=32 open_us=452 inspection_us=1 dispatch_us=1229 render_us=174
```

## Interpretation

- `parse_us` covers the bounded UTF-8 TQL parser.
- `open_us` covers read-only store opening.
- `inspection_us` is zero-cost-in-practice for `STATUS`; it records the
  conditional verification/WAL inspection stage, which this command did not
  request.
- `dispatch_us` covers the structured outcome adapter.
- `render_us` covers JSON serialization and terminal writing setup.

The observation is affected by local filesystem state, debug-build settings,
machine load, and the empty fixture. It must not be used as a service-level
promise or compared directly with another machine.

## Contract Boundary

`--timings` writes a single bounded line to standard error. It is intentionally
outside the TQL human and JSON schemas, so scripts that consume `--json` do not
need to accept timing fields or treat timing as database truth.

Allocation counts are not recorded. Tosumu has not admitted an allocation
profiler or a general profiling service, so inventing an allocation claim here
would be less honest than leaving the measurement unavailable.

## Reopen Triggers

Revisit this note when any of the following occurs:

- a stable benchmark or profiler mechanism is admitted;
- a second TQL consumer needs structured timing data;
- a command gains paging, scanning, or large-result behavior;
- encrypted-store TQL inspection is admitted.
