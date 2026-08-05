<section class="tosumu-home-intro">
  <div class="tosumu-home-copy">
    <p class="tosumu-eyebrow">Local storage / explicit evidence</p>
    <h1>Understand what the database believes.</h1>
    <p class="tosumu-lede">
      Tosumu is an experimental, single-file database engine for embedders who
      need durable local state without accepting an opaque storage boundary.
    </p>
    <div class="tosumu-actions">
      <a class="tosumu-button tosumu-button--primary" href="getting-started/">Start locally</a>
      <a class="tosumu-button" href="architecture/">Inspect the architecture</a>
    </div>
  </div>

  <aside class="tosumu-status-card" aria-label="Tosumu maturity and scope">
    <div class="tosumu-status-card__header">
      <span>Project observation</span>
      <strong>Pre-stability</strong>
    </div>
    <dl>
      <div><dt>Shape</dt><dd>Single file / single process</dd></div>
      <div><dt>Core</dt><dd>Pager / B+ tree / WAL</dd></div>
      <div><dt>Trust</dt><dd>Authenticated pages</dd></div>
      <div><dt>Tools</dt><dd>Inspect API / TUI / TQL experimental</dd></div>
    </dl>
    <p>Not production-ready. Do not use Tosumu for real secrets or irreplaceable data.</p>
  </aside>
</section>

<section class="tosumu-principles" aria-label="Tosumu design priorities">
  <article><span>01</span><strong>Inspectable</strong><p>State and recovery evidence remain available to tools.</p></article>
  <article><span>02</span><strong>Explicit</strong><p>Failures are structured instead of hidden behind success-shaped APIs.</p></article>
  <article><span>03</span><strong>Bounded</strong><p>Small embedded use cases outrank server-database ambition.</p></article>
</section>

## Choose a path

<div class="tosumu-paths">
  <article>
    <p class="tosumu-eyebrow">Use</p>
    <h3>Build, open, inspect</h3>
    <p>Take the shortest path from a clean checkout to a database you can examine.</p>
    <a href="getting-started/">Getting started &rarr;</a>
  </article>
  <article>
    <p class="tosumu-eyebrow">Evaluate</p>
    <h3>Read guarantees first</h3>
    <p>Separate implemented behavior from intentional limits and future work.</p>
    <a href="safety-and-limits/">Safety and limits &rarr;</a>
  </article>
  <article>
    <p class="tosumu-eyebrow">Integrate</p>
    <h3>Consume observations</h3>
    <p>Use the machine-readable inspection contract without parsing storage internals.</p>
    <a href="inspect-api/">Inspect API &rarr;</a>
  </article>
</div>

## Current capability map

| Area | Maturity | Current claim |
| --- | --- | --- |
| Storage engine | Implemented | Page-based key/value storage with B+ tree lookup |
| Recovery | Implemented | WAL-backed recovery with explicit diagnostics |
| Authenticated storage | Experimental | Authenticated pages and key protection; not audited |
| Inspection | Implemented | Read-only TUI and structured inspect output |
| Tosumu Command Language | Experimental | Bounded command/query surface under active validation |
| Sync and collaboration | Deferred | Reviews exist; no admitted synchronization contract |

Tosumu uses maturity words deliberately. **Implemented** describes observed
behavior, not production readiness. **Experimental** identifies an active
boundary that may change. **Deferred** is not a silent fallback.

## How the pieces fit

```text
application meaning
        |
        v
public storage and inspection contracts
        |
        v
pager -> B+ tree -> WAL -> authenticated pages
        |
        v
explicit observations and failures
```

The architecture is intentionally small enough to follow end-to-end. Start
with [Concepts](concepts.md) for the model, then use
[Architecture](architecture.md) for ownership and crate boundaries.

## Engineering record

This site explains Tosumu's current meaning. GitHub preserves the complete path
by which that meaning was discovered.

- [Specifications](Specifications/README.md) define normative and current contracts.
- [Accepted architecture decisions](ADR/README.md) record binding design constraints.
- [Project governance](project-governance.md) explains document authority.
- [Document status](document-status.md) separates current guidance, active
  work, speculation, and historical records.

Implementation plans, unresolved reviews, change-request internals, notes, and
conversations remain engineering records rather than ordinary user guidance.
