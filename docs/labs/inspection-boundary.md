# Inspection Boundary Lab

This lab is a bounded browser consumer of the Tosumu inspection contract. It
does not parse pages in JavaScript, unlock stores, or read paths from the host.
It sends either a reviewed bundled byte fixture or an uploaded byte buffer to
the Tosumu WASM adapter and presents the versioned observation returned by that
adapter. JavaScript transports bytes only; it does not parse the database.

Without JavaScript or WASM, this page remains a useful disclosure: the current
browser proof reads only the plaintext page-zero header. Page verification,
tree traversal, WAL inspection, keyslots, protectors, and decrypted content
are intentionally unavailable in this mode.

<section class="tosumu-inspection-lab" data-tosumu-inspection-lab>
  <header>
    <p class="tosumu-kicker">TOSUMU / BROWSER INSPECTION BOUNDARY</p>
    <h2>Header-only byte observation</h2>
    <p>The reviewed fresh-store fixture loads automatically, so this boundary
    remains useful without asking visitors to bring a database. The reviewed
    cases exercise the facts this boundary can
    establish and the errors it must report. Uploads are capped at 16 MiB,
    remain in this browser session, and are never assigned a host path by the
    contract.</p>
  </header>
  <div class="tosumu-inspection-fixtures" role="group" aria-label="Reviewed browser fixtures">
    <p>Reviewed boundary cases</p>
    <button type="button" data-tosumu-inspection-fixture data-tosumu-inspection-fixture-label="Fresh compatible store" data-tosumu-inspection-fixture-expectation="header observation" data-tosumu-inspection-fixture-url="../../overrides/fixtures/inspection-header-fixture-v1.tosumu" disabled>Fresh store</button>
    <button type="button" data-tosumu-inspection-fixture data-tosumu-inspection-fixture-label="Known populated store" data-tosumu-inspection-fixture-expectation="header observation only" data-tosumu-inspection-fixture-url="../../overrides/fixtures/inspection-populated-fixture-v1.tosumu" disabled>Populated store</button>
    <button type="button" data-tosumu-inspection-fixture data-tosumu-inspection-fixture-label="Invalid magic bytes" data-tosumu-inspection-fixture-expectation="explicit rejection" data-tosumu-inspection-fixture-url="../../overrides/fixtures/inspection-invalid-magic-v1.bin" disabled>Invalid magic</button>
    <button type="button" data-tosumu-inspection-fixture data-tosumu-inspection-fixture-label="Truncated page zero" data-tosumu-inspection-fixture-expectation="explicit rejection" data-tosumu-inspection-fixture-url="../../overrides/fixtures/inspection-truncated-v1.bin" disabled>Truncated</button>
    <button type="button" data-tosumu-inspection-fixture data-tosumu-inspection-fixture-label="Newer format header" data-tosumu-inspection-fixture-expectation="explicit rejection" data-tosumu-inspection-fixture-url="../../overrides/fixtures/inspection-newer-format-v1.bin" disabled>Newer format</button>
  </div>
  <details class="tosumu-inspection-upload">
    <summary>Inspect an uploaded database instead</summary>
    <label for="tosumu-inspection-file">Database file</label>
    <input id="tosumu-inspection-file" data-tosumu-inspection-file type="file" accept=".tosumu,.tsm,application/octet-stream">
    <button type="button" data-tosumu-inspection-run disabled>Inspect selected bytes</button>
  </details>
  <p data-tosumu-inspection-status role="status">WASM inspection is loading. Static disclosure remains available if it cannot start.</p>
  <pre data-tosumu-inspection-output aria-live="polite">Choose a reviewed fixture to see a bounded Tosumu observation or explicit rejection.

Unavailable by design in this browser proof:
- protected-store unlocking
- page and tree verification
- WAL and keyslot inspection
- decrypted content preview</pre>
  <section class="tosumu-inspection-interface-guide" aria-labelledby="tosumu-inspection-interface-guide-title">
    <h3 id="tosumu-inspection-interface-guide-title">Two embedded interfaces</h3>
    <p><strong>TypeScript semantic interface:</strong> browser controls select
    reviewed or uploaded bytes; the Tosumu Rust/WASM adapter returns a
    versioned observation; TypeScript renders that observation panel. The
    browser transports bytes and presents facts, but never parses database
    pages or invents storage meaning.</p>
    <p><strong>Ratatui terminal interface:</strong> the same bounded result
    initializes one interactive Rust/WASM-owned Ratatui session. Ratatui
    produces normalized terminal cells, the browser canvas draws those cells,
    and focused browser input is forwarded as bounded events. Rust/WASM owns
    the prompt, transcript, history, and command outcomes.</p>
    <p><strong>Why both exist:</strong> the TypeScript interface is the
    accessible semantic browser presentation. The Ratatui interface is the
    terminal-provider proof. They share an observation boundary but do not
    share UI state or make the browser a terminal emulator.</p>
  </section>
  <section class="tosumu-ratatui-terminal" aria-labelledby="tosumu-ratatui-terminal-title">
    <h3 id="tosumu-ratatui-terminal-title">Browser-safe Ratatui command session</h3>
    <p>Click the terminal to claim keyboard focus, then type <code>HELP</code>,
    <code>STATUS</code>, or <code>CLEAR</code> and press Enter. The browser
    releases terminal focus when you click elsewhere. TQL and native storage
    commands remain deliberately unavailable because raw uploaded bytes are
    not a reviewed native store session.</p>
    <canvas data-tosumu-ratatui-terminal-canvas width="768" height="352" tabindex="0" role="application" aria-label="Interactive browser-safe Ratatui command session will appear after a fixture is inspected"></canvas>
    <textarea data-tosumu-ratatui-terminal-input class="tosumu-ratatui-terminal-input" rows="1" aria-label="Browser-safe Ratatui terminal input" autocomplete="off" autocapitalize="off" spellcheck="false"></textarea>
    <p data-tosumu-ratatui-terminal-status>Awaiting a reviewed fixture or uploaded byte buffer.</p>
  </section>
</section>
