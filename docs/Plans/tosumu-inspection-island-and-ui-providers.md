# Tosumu Inspection Island And UI Providers

| Field | Value |
| --- | --- |
| Status | Proposed |
| Opened | 2026-08-04 |
| Last updated | 2026-08-05 |
| Owner | Tosumu and Tokimu maintainers |
| Target | Structured inspection capability, Ratatui provider, and TypeScript website island |
| Related ADRs | ADR-0001 |
| Related reviews | AR-0002 |
| Related plans | `tokimu-reciprocal-website-evidence.md`, `public-website-and-repository-records.md` |
| Depends on | `tosumu-core::inspect`, current CLI inspect contract, Tosumu fixtures, Tokimu WASM island hosting |

## Status

Tosumu already exposes bounded storage inspection facts, machine-readable CLI
payloads, a native Ratatui inspector, and a WPF consumer. The Ratatui state is
currently presentation-shaped and the complete serialized payload contract is
CLI-owned. No browser island consumes the same inspection semantics directly.
This plan proposes one shared observation and command boundary, exercised by
Ratatui and TypeScript UI providers, without yet deciding that JSON or one UI
framework belongs in Tosumu core.

## Purpose

Create a bounded Tosumu database-inspection island that can run on the public
website while preserving the native Ratatui inspector as a first-class peer.
Both providers must present the same Tosumu-owned facts and invoke the same
provider-neutral commands rather than independently interpreting database
pages.

The work exists to answer a concrete architectural question raised by AR-0002:
can a direct non-CLI consumer use a complete structured inspection model
without importing CLI serialization or physical storage internals?

```text
Tosumu inspection semantics
        ↓
versioned observation and command contract
        ├── Ratatui provider
        ├── TypeScript island provider
        └── WPF consumer evidence
```

The first browser implementation has two deliberately distinct presentation
paths. The TypeScript interface renders provider-neutral inspection facts from
the Rust/WASM boundary. One separately labelled Ratatui `TestBackend` terminal
renders normalized cells for a Rust/WASM-owned, browser-safe command profile.
Browser input is forwarded as normalized events only after the terminal claims
focus. The browser neither owns prompt or transcript state, replaces the
semantic observation, parses TQL, executes native CLI commands, nor runs
Crossterm in the browser.

## Trigger And Evidence

- `tosumu view` already proves a useful interactive inspection workflow over
  header, page, verification, tree, WAL, and protector observations.
- `inspect_contract.rs` already defines machine-readable payloads, but those
  types are private to `tosumu-cli` and coupled to JSON translation.
- The WPF harness proves that a second presentation technology needs Tosumu
  inspection facts without owning Tosumu file semantics.
- The Tosumu website needs bounded executable evidence without requiring
  readers to install the native CLI.
- Tokimu already hosts progressive-enhancement WASM islands and can provide the
  browser execution and presentation mechanism without becoming the owner of
  Tosumu facts.
- AR-0002 explicitly lists a direct non-CLI consumer as missing evidence before
  the structured inspection boundary can graduate.

Observed behavior is not yet a guarantee that Ratatui state, CLI JSON, or the
proposed browser snapshot is the permanent public contract.

## Prerequisite Evidence From The Tokimu Console Corpus

`tokimu-console-command-window` completed its embedded-provider readiness
review without promoting a shell capability or Ratatui provider. The following
evidence is available to this plan:

- one retained command session can produce deterministic transcript, command
  outcome, normalized-cell, cell-layout, and CPU-raster artifacts;
- transcript, cell, cursor, and layout divergence can fail at the first owning
  boundary rather than becoming a visual guess;
- prompt editing, focus, history recall, viewport scrolling, measured wrapping,
  clipping, and resize can remain host-owned interaction state; and
- a bounded embedded host can consume Ratatui-produced cells without making
  those cells Tosumu storage or command semantics.

The evidence rejects several implementation shortcuts:

- do not export `ViewApp`, `ListState`, key mappings, panel scroll offsets, or
  other Ratatui widget state as the shared observation model;
- do not promote CLI JSON, the console corpus's `SessionEvidence`, or a
  normalized terminal grid merely because each is serializable;
- do not require the TypeScript provider to emulate terminal cells when it can
  present provider-neutral observations directly; and
- do not extract the corpus-local Ratatui adapter until a second independent
  host demonstrates reusable behavior.

This prerequisite evidence refines the ownership and acceptance constraints;
it does not complete any Tosumu implementation slice. Official Ratatui provider
admission remains parked until a standalone terminal host and an embedded host
consume the same provider-neutral session without semantic drift.

## Current State

### Storage Inspection

`tosumu-core::inspect` owns bounded storage facts and typed inspection results.
It can inspect headers, pages, verification state, tree structure, WAL records,
and protector summaries.

### CLI JSON

`crates/tosumu-cli/src/inspect_contract.rs` translates core facts into the
current JSON envelope described by the Tosumu Inspect API Specification. The
envelope and payload structs are currently CLI-private.

### Ratatui

`crates/tosumu-cli/src/view/` owns terminal navigation, focus, filtering,
scrolling, watch behavior, selected-page state, and Ratatui rendering. Its
`ViewApp` mixes provider-neutral session state with terminal-specific state
such as `ListState`, key mappings, and panel scroll positions.

### Other Consumers

The WPF harness and Tokimu Resource Workbench provide independent UI and
provider pressure. They must remain consumers of structured facts rather than
alternative decoders of Tosumu pages.

## Primary Composition Claim

An ordinary browser application can inspect a bounded Tosumu fixture through
a Rust/WASM boundary, present it through TypeScript, and preserve semantic
parity with the Ratatui provider without any browser code parsing the Tosumu
file format.

```text
browser-selected bytes
        ↓
Tosumu Rust/WASM inspection adapter
        ↓
provider-neutral observation
        ↓
TypeScript interaction and presentation
```

At no point does:

- TypeScript parse pages, WAL frames, records, keyslots, or protectors;
- Ratatui define storage truth;
- Tokimu redefine Tosumu command outcomes;
- a browser-native database library silently replace Tosumu inspection;
- a provider infer facts absent from the observation contract.

## Goals

- Define one provider-neutral inspection-session observation model.
- Define a small command vocabulary for selection, navigation, refresh, and
  view changes without embedding keyboard or DOM event semantics.
- Adapt the native Ratatui inspector to the shared semantic model.
- Add a TypeScript browser island over a Rust/WASM inspection adapter.
- Produce one browser-visible, interactive Ratatui `TestBackend` terminal
  from the same Rust/WASM response, without promoting terminal cells into
  shared inspection semantics.
- Preserve static, accessible website evidence when WASM or JavaScript is
  unavailable.
- Compare provider outputs at semantic boundaries, not by pixel identity.
- Produce the direct non-CLI evidence requested by AR-0002.
- Keep expected failures bounded, typed, and visible in every provider.

## Non-Goals

- Running Crossterm or a native terminal emulator inside the browser. The
  browser draws cells from one Rust/WASM-owned Ratatui `TestBackend` session
  and forwards bounded normalized events only after that terminal claims
  focus; it does not own terminal state, parse TQL, access host paths, or
  execute storage commands beyond the reviewed provider profile.
- Promoting CLI JSON payload structs directly into `tosumu-core`.
- Making Tokimu a Tosumu page, WAL, protector, or recovery decoder.
- Giving TypeScript authority over storage or verification meaning.
- Matching native and browser layouts pixel for pixel.
- Exposing secrets, decrypted values, key material, passphrases, or arbitrary
  host paths through the public island.
- Adding database mutation in the first island.
- Replacing the WPF harness or requiring all UI providers to share widgets.
- Generalizing this work into a universal Tokimu shell or MUD before another
  independent command-oriented consumer produces pressure.

## Ownership And Dependency Boundary

```text
Tosumu physical storage and inspection facts
        ↓
provider-neutral inspection observation and commands
        ├── Ratatui terminal adapter
        ├── Rust/WASM boundary adapter
        │       ↓
        │   TypeScript island provider
        └── WPF consumer adapter
```

### Tosumu Inspection Owns

- storage fact identity and terminology;
- typed inspection outcomes and diagnostics;
- page, tree, WAL, verification, and protector observation semantics;
- deterministic command outcomes;
- limits and disclosure policy for inspection;
- compatibility rules for any admitted observation schema.

### Ratatui Provider Owns

- terminal layout, colors, focus, scrolling, and keyboard mapping;
- rendering observations into Ratatui widgets;
- translating terminal events into provider-neutral commands;
- terminal-only watch and status affordances.

### TypeScript Provider Owns

- DOM or canvas layout, accessibility, responsive behavior, and browser input;
- translating browser events into provider-neutral commands;
- presenting Rust/WASM observations without adding storage meaning;
- progressive enhancement and explicit WASM failure presentation.

### Tokimu May Own

- the generic website-island host lifecycle;
- WASM loading, canvas or DOM presentation mechanisms, and diagnostics;
- provider-neutral visual composition of facts supplied by Tosumu.

Tokimu must not parse Tosumu pages or become the source of Tosumu truth.

### Dependency Direction

- `tosumu-core` must not depend on Ratatui, Crossterm, TypeScript, Tokimu, DOM,
  WASM hosting, or CLI JSON.
- Shared observation types may depend on core inspection facts but must not
  depend on a concrete UI provider.
- Ratatui and WASM adapters depend downward on the shared observation model.
- TypeScript depends on the versioned WASM boundary, never physical Tosumu
  structures.
- The Tosumu MkDocs build must remain useful without Tokimu or generated WASM.

## Provisional Contract Shape

Names in this section are working vocabulary, not accepted API commitments.

```rust
struct InspectionObservation {
    schema: ObservationSchema,
    source: SourceObservation,
    capabilities: InspectionCapabilities,
    active_view: InspectionView,
    page_list: BoundedPageList,
    selected_page: Option<PageObservation>,
    panel: InspectionPanel,
    diagnostics: Vec<InspectionDiagnostic>,
}

enum InspectionCommand {
    SelectPage(u64),
    SelectNext,
    SelectPrevious,
    SetView(InspectionView),
    SetFilter(String),
    Refresh,
}
```

The command model must describe user intent. It must not contain terminal key
codes, DOM events, CSS selectors, Ratatui widget state, or renderer objects.

The observation model may reuse or compose core inspection facts. It must not
duplicate physical page decoding in a new layer.

## Public Contract Impact

This plan is expected to create one provisional structured inspection-session
contract outside the CLI renderer. Its final crate and module location remains
open until implementation proves the smallest honest boundary.

The following remain deliberately separate:

- core inspection facts;
- interactive session observations;
- CLI JSON serialization;
- WASM boundary serialization;
- Ratatui, WPF, and TypeScript presentation state.

If the implementation requires a new reusable crate or changes ownership in
ADR-0001, stop and reopen AR-0002 before treating the extraction as accepted.

## Deliverables

- [ ] Inventory Ratatui state into semantic session state versus
      terminal-provider state.
- [x] Define a bounded, provider-neutral inspection observation.
- [x] Define a provider-neutral command vocabulary and deterministic reducer.
- [ ] Adapt Ratatui to consume the shared observation and commands.
- [x] Add a headless Rust consumer test that validates observations without
      serializing through the CLI JSON schema.
- [x] Add a Rust/WASM inspection adapter over the reviewed fixture matrix and
      bounded uploaded bytes.
- [x] Add a static-first TypeScript inspection island to the Tosumu site.
- [ ] Add semantic parity tests across Ratatui-facing and browser-facing
      adapters.
- [ ] Record security, resource, disclosure, and unsupported-state behavior.
- [ ] Reopen AR-0002 with the resulting independent-consumer evidence.

## Implementation Slices

### Slice 0: Baseline And State Inventory

**Objective:** Record the current native workflow and separate Tosumu meaning
from terminal mechanics before extracting any contract.

#### Deliverables

- [x] Inventory every `ViewApp` field as semantic, provider-local, cached, or
      execution-only state.
- [ ] Capture one deterministic fixture across Header, Detail, Verify, Tree,
      WAL, and Protectors views.
- [ ] Record current CLI JSON and Ratatui-visible facts for that fixture.
- [ ] Record public-island disclosure and file-size limits.
- [ ] Confirm AR-0002 remains the owning review.

#### Acceptance Criteria

- [x] Every extracted field has one documented owner.
- [x] Terminal key mappings and widget state are excluded from semantic types.
- [ ] The fixture contains no secrets or unreviewed protected material.
- [ ] No code movement silently promotes the CLI wire schema into core.

#### Validation

```text
cargo test -p tosumu-cli view
cargo run -p tosumu-cli -- view <reviewed-fixture>
cargo run -p tosumu-cli -- inspect header <reviewed-fixture> --json
```

#### Current `ViewApp` Ownership Inventory

The inventory below records the native inspector as it exists on 2026-08-05.
It is evidence for a future adapter; it is not a request to serialize or move
`ViewApp` itself.

| Current state | Classification | Owner and extraction treatment |
| --- | --- | --- |
| `path` | Execution-only | The native CLI owns the host path and file-opening policy. A public observation exposes reviewed source metadata only, never an arbitrary host path. |
| `header`, `verify` | Semantic snapshot | `tosumu-core::inspect` facts. The shared observation may compose these results without copying physical decoding. |
| `pages` | Semantic snapshot plus cached search data | Page rows are bounded inspection facts. `search_text` is provider-derived filtering support and must not become an authoritative storage fact. |
| `tree`, `wal`, `keyslots` | Semantic snapshot | Typed inspection result or explicit unavailable diagnostic. The provider presents these results but does not recreate them. |
| `mode` | Semantic session intent plus provider mapping | A provider-neutral view selection may be retained, but the current `ViewMode::from_key` and display labels remain Ratatui/Crossterm concerns. |
| `selected` | Cached provider index | The shared session uses an optional stable page number, not a `Vec` index. Each provider resolves that page number against its bounded page window. |
| `selected_detail` | Cached derived observation | Rebuild from the selected page number and pager-backed core inspection result; do not make it a separately mutable truth. |
| `filter_query` | Semantic session intent after confirmation | The committed query may be a provider-neutral command parameter. Its input editing behavior is not. |
| `pending_filter_query`, `pending_page_jump` | Provider-local | Temporary prompt buffers and cursor behavior stay in Ratatui or a browser host. Commands receive only confirmed values. |
| `focus`, `panel_scroll`, `list_state()` | Provider-local | Ratatui layout, focus, viewport, and widget selection mechanics. They are expressly excluded from the shared model. |
| `watch_enabled`, `last_refresh`, `last_watch_fingerprint` | Execution-only | Native watch policy, clock, and file-change mechanism remain owned by the host. A later shared refresh command reports an outcome, not a timer implementation. |
| `status_message` | Provider-local | Transient terminal feedback. Structured command and inspection diagnostics provide the portable equivalent. |

#### Slice 0 Finding

`ViewApp` is not a candidate shared type: it borrows a host path, retains
terminal navigation state, uses page-list indexes, and stores refresh/cache
mechanics next to inspection facts. The smallest honest next boundary is a
headless observation built from `tosumu-core::inspect` with stable page-number
selection, explicit bounded/unavailable results, and a separate provider
adapter for keyboard, scrolling, prompts, and watch behavior.

`AR-0002` remains the owning review. This work is the direct non-CLI Rust
consumer evidence it requests; it does not promote the present CLI JSON
envelope into `tosumu-core`.

#### Exit State

The existing workflow is reproducible and its ownership seams are explicit.

### Slice 1: Shared Observation Model

**Objective:** Build the smallest headless observation that can represent one
useful inspection session without UI-provider types.

#### Deliverables

- [ ] Define provisional observation, capability, view, diagnostic, and source
      metadata types.
- [x] Build the observation from existing `tosumu-core::inspect` facts.
- [x] Bound page lists, record previews, recursive tree depth, WAL records, and
      diagnostic counts.
- [x] Add deterministic headless tests for ordinary, empty, partial, corrupt,
      and unavailable observations.
- [ ] Keep CLI JSON translation as a separate adapter.

#### Acceptance Criteria

- [x] A headless Rust test can produce every supported section without Ratatui.
- [x] Unsupported and partial states remain explicit data, not empty panels.
- [x] No observation contains terminal, DOM, Tokimu renderer, or physical
      pager ownership objects.
- [x] Repeated observations of an unchanged fixture are deterministic.

#### Validation

```text
cargo test -p tosumu-core inspect
cargo test -p tosumu-cli inspection_observation
```

#### Exit State

One provider-neutral inspection observation exists and is exercised headlessly.

#### Slice 1 Progress Note

`tosumu-core::inspection_session` now provides the provisional schema-v1
observation. It composes existing core inspection facts, preserves stable page
numbers rather than provider indexes, limits every collection, and records
optional tree, WAL, and keyslot failures as typed `Unavailable` sections.

The direct core tests cover an ordinary store, zero-length output limits, a
corrupt tree root that remains a partial observation, repeated deterministic
observations, and a physically invalid file that fails before claiming an
observation exists. The current Rust values are intentionally not CLI JSON or
the future WASM wire encoding. Public-island disclosure and redaction policy
remain Slice 4 work; this provisional model must not be treated as a browser
payload yet.

### Slice 2: Command Model And Session Reducer

**Objective:** Express inspection interaction as semantic commands independent
of keyboard, mouse, terminal, or browser mechanisms.

#### Deliverables

- [x] Define the minimum command set for view selection, page selection,
      filtering, refresh, and bounded navigation.
- [x] Implement deterministic command application over session state.
- [x] Return typed command outcomes and diagnostics.
- [x] Add invalid selection, stale refresh, empty result, and limit tests.
- [x] Keep watch timing and event loops provider-owned.

#### Acceptance Criteria

- [x] Providers can translate input into commands without mutating storage
      facts directly.
- [x] Invalid commands fail explicitly and leave session identity coherent.
- [x] Applying the same command to the same observation yields the same result.
- [x] No mutation command is admitted in this slice.

#### Validation

```text
cargo test -p tosumu-core inspection_session
```

#### Exit State

The inspection workflow has a provider-neutral observation and interaction
contract suitable for multiple UIs.

#### Slice 2 Progress Note

`InspectionSession`, `InspectionCommand`, and
`apply_inspection_command` now live beside the provisional observation in
`tosumu-core::inspection_session`. The reducer carries only confirmed view,
page-selection, filter, and refresh intent. It resolves selection against the
bounded observation by stable page number and returns typed applied or rejected
outcomes without opening storage, owning a refresh loop, mutating a page, or
depending on terminal/browser input.

Focused tests cover stable-page selection, boundary-clamped navigation, invalid
page rejection without session mutation, empty filtered navigation, and stale
refresh rejection. The session revision records a host refresh request; it is
not a durable database revision or an automatic watcher policy. Ratatui and
browser adapters remain responsible for translating their own input and for
fetching the next observation after a refresh request.

### Slice 3: Ratatui Provider Migration

**Objective:** Preserve the native inspector while making Ratatui a consumer of
the shared semantics rather than their owner.

#### Deliverables

- [x] Translate committed Crossterm view, filter, refresh, and selected-page
      actions into shared commands. Prompt editing and key mapping remain local.
- [x] Build the shared observation from the already-unlocked pager used by the
      native inspector, then retain the existing Ratatui layouts as provider
      presentation over that snapshot.
- [x] Keep focus, scroll, colors, key help, and watch cadence provider-local.
- [x] Preserve search, page jump, view switching, refresh, and diagnostics.
- [x] Add regression tests for provider mode, filter, and selected-page parity.
- [x] Align the terminal's all-page navigation window with the bounded shared
      page observation window before claiming full command parity.

#### Acceptance Criteria

- [x] Existing native workflows remain available through `tosumu view`.
- [x] Ratatui no longer computes storage facts independently of the shared
      observation builder for its initial open and refresh paths.
- [x] Terminal-only state does not leak into the shared contract.
- [x] Corrupt and partial fixtures remain inspectable without panic in the
      focused view suite.
- [x] Selecting a page outside the bounded shared observation window must gain
      an explicit provider policy rather than relying on reducer rejection.

#### Validation

```text
cargo test -p tosumu-cli view
cargo run -p tosumu-cli -- view <reviewed-fixture>
```

#### Current Evidence And Remaining Gap

The native provider now constructs its provider-neutral observation through the
same unlocked `Pager` that it uses for terminal detail inspection. This avoids a
second open path that could lose protector context or independently reinterpret
storage facts. Terminal mode changes, confirmed filters, refresh revisions, and
stable page navigation are reduced through the same provider-neutral command
state before Ratatui derives its local list selection and panel details.
stable selected page numbers all pass through the shared reducer. Ratatui still
owns key bindings, prompt buffers, search-text caching, focus, scrolling,
watch timing, and widget rendering.

The shared observation intentionally bounds its retained page list. The
terminal now adapts its selectable `PageRow` list from those retained entries,
so it cannot present a page selection the shared reducer would reject. The
provider may still use its already-unlocked pager to decode details for a
selected page after that semantic selection has succeeded. This preserves the
protector-aware terminal detail path without creating a second provider-owned
selection contract. The shared header reports total and truncated page counts,
while the terminal's visible list is deliberately limited to the retained
observation window.

#### Exit State

Ratatui is the first UI provider over the shared inspection model, with an
explicit bounded-page selection policy and provider-local detail decoding.

### Slice 4: Rust/WASM Inspection Boundary

**Objective:** Prove a direct non-CLI consumer can produce the same inspection
observations from bounded browser-provided bytes.

#### Current Progress

The first direct non-CLI inspection boundary is now present:

- The raw-byte inspection entry point accepts only caller-bounded input and
  parses the plaintext page-zero header without manufacturing a host path.
- It deliberately returns no decrypted page, tree, WAL, or keyslot facts.
  Those sections carry stable `Unsupported` outcomes until a future adapter
  supplies an approved protector and pager path.
- `tosumu-inspection-boundary` owns a versioned, serializable DTO distinct
  from CLI command JSON. It retains the actual uploaded byte count rather than
  trusting header-declared page counts.
- `tosumu-inspection-wasm` now exposes that DTO through a small `cdylib`:
  browser bytes enter as a byte slice and return the same JSON response. It
  adds no page parser, unlock state, path, or storage object to the WASM API.
- The adapter has native contract tests and a successful
  `wasm32-unknown-unknown` build. Its WASM-only `getrandom` feature is an
  adapter concern required by Tosumu's authenticated-format dependency graph;
  it does not alter core or native provider policy.
- Focused tests cover header-only input, malformed input, oversized input,
  unknown boundary schema, unavailable sections, stable bounded failures, and
  absence of host or unlock fields. The static-first browser page consumer is
  the remaining Slice 5 proof.

#### Deliverables

- [x] Add a WASM-safe adapter for uploaded database bytes. The adapter exposes
      the reviewed boundary schema version and a JSON inspection response; the
      first browser page consumer remains Slice 5 work.
- [x] Define a versioned boundary encoding distinct from CLI command JSON.
      `tosumu-inspection-boundary` is a provisional provider crate; its DTOs
      remain separate from `tosumu-core` and the CLI serializer.
- [~] Enforce byte, page, tree-depth, record-preview, and diagnostic limits.
      The raw-byte entry point and WASM adapter enforce the reviewed byte
      limit. Adapter response-size and browser UI presentation limits remain
      pending.
- [~] Reject unsupported encryption or protector requirements explicitly.
      Raw-byte observations mark pager-dependent sections unavailable instead
      of implying an unlocked store.
- [~] Add unknown-schema, malformed-input, oversized-input, and unavailable-
      provider tests.
      The boundary crate covers unknown schemas, malformed and oversized input,
      and raw-byte unavailable sections. WASM-host and browser-provider cases
      remain pending.

#### Acceptance Criteria

- [x] The WASM adapter invokes Tosumu inspection code rather than parsing
      pages in TypeScript. The static-first island sends selected bytes only
      through that adapter; TypeScript only presents its versioned response.
- [x] No host path, passphrase, key material, or decrypted secret is serialized
      by the provisional boundary DTOs.
- [x] Boundary failures contain stable codes and bounded messages.
- [x] Native core, boundary, and WASM wrapper preserve the same bounded
      raw-byte header observation for the reviewed fixture. This does not
      claim parity with an already-unlocked native pager session.

#### Validation

```text
cargo test -p <wasm-adapter-crate>
cargo build -p tosumu-inspection-wasm --target wasm32-unknown-unknown
```

#### Exit State

AR-0002 has its first direct non-CLI structured-inspection adapter. A browser
page consuming the adapter remains the next proof, not an implied completion.

### Slice 5: TypeScript Website Island

**Objective:** Present the WASM observation as an accessible, static-first
Tosumu inspection lab.

#### Current Progress

The first static-first island now exists at `docs/labs/inspection-boundary.md`.
It discloses the header-only boundary, 16 MiB input limit, and unavailable
pager-dependent sections before JavaScript is needed. The generated
`tosumu-inspection-wasm` browser module is emitted by
`scripts/build-inspection-wasm.ps1` into the MkDocs static asset tree and loads
only after the page is already useful as a limitation and contract disclosure.

The interactive surface now offers a reviewed fixture matrix first: a fresh
compatible store, a known populated store, invalid magic bytes, truncated page
zero, and a newer-format header. The compatible cases demonstrate the bounded
header facts available without unlock. The populated case intentionally does
not disclose records, proving that container compatibility is not confused with
content inspection. The rejection cases demonstrate stable boundary errors.
The page fetches every fixture as opaque bytes and routes it through the same
WASM adapter used for uploads; JavaScript neither creates nor parses a store.
An optional selected-file path rejects oversized input before crossing the WASM
boundary. The page renders a concise presentation of the versioned DTO and
retains the raw DTO as supporting evidence. The island does not claim
protected-store support, page verification, tree traversal, WAL inspection,
keyslot inspection, or decrypted content. Those remain explicit unavailable
outcomes from the boundary.

The current shell did not have MkDocs installed through either `mkdocs` or
`python -m mkdocs` on 2026-08-05, so strict site-build validation is recorded
as pending environment evidence rather than inferred from the artifact layout.

#### Deliverables

- [x] Publish static fixture facts, limitations, and a fallback artifact.
      The bundled fresh-store fixture loads automatically, making both browser
      presentations runnable without requiring a visitor to possess a database
      file.
- [x] Add a reviewed fixture matrix covering compatible containers, a known
      populated container whose records remain undisclosed, malformed magic,
      truncation, and a newer-format rejection.
- [x] Add file selection with explicit size and disclosure warnings. Drag and
      drop remains a later ergonomic enhancement, not a boundary requirement.
- [ ] Add browser views for header, pages/detail, verify, tree, WAL, and
      protectors according to advertised capabilities.
- [ ] Translate DOM input into the shared command vocabulary.
- [ ] Add keyboard navigation, focus states, responsive layout, bounded panel
      scrolling, and screen-reader labels.
- [x] Keep WASM startup and failure diagnostics visible.

#### Acceptance Criteria

- [x] The page remains useful with JavaScript or WASM disabled.
- [x] The island cannot expand the whole document because one panel is verbose.
- [x] TypeScript does not infer storage facts or parse database structures.
- [x] Every unsupported view is labeled unavailable or deferred.
- [x] Compatible, populated, malformed, truncated, and newer-format fixtures
      produce visibly distinct bounded outcomes without claiming record access.
- [x] The browser visibly distinguishes a compatible header observation from
      an explicit boundary rejection without parsing storage bytes itself.
- [ ] Narrow and wide layouts preserve content order and keyboard access.

#### Validation

```text
node --test <island-test-path>
mkdocs build --strict
pwsh -NoProfile -File scripts/build-inspection-wasm.ps1 -Profile release
node --check docs/overrides/js/tosumu-inspection-island.js
```

Current automated evidence:

```text
cargo test -p tosumu-inspection-wasm
cargo clippy -p tosumu-inspection-wasm --all-targets -- -D warnings
pwsh -NoProfile -File scripts/build-inspection-wasm.ps1 -Profile release
node --test docs/overrides/js/tosumu-inspection-island-contract.test.mjs
node --check docs/overrides/js/tosumu-inspection-island.js
```

The first two adapter checks, generated WASM package, and JavaScript syntax
check pass. The isolated browser-contract tests cover missing, oversized,
accepted, and bundled fixture selection plus response rendering without
interpreting storage facts. `mkdocs build --strict` remains pending because
MkDocs build evidence is retained separately from the adapter and island
contract checks.

### Slice 5a: Headless Ratatui Provider Projection

**Objective:** Let browser visitors compare the semantic island with a
Ratatui-produced terminal-cell projection without treating the browser as a
terminal host.

**Disposition:** Superseded as a separate public viewport by Slice 5c. The
cell-projection adapter remains useful corpus and provider evidence, but the
public lab no longer renders a passive second viewport beside the interactive
terminal. That duplication added no distinct semantic boundary.

#### Deliverables

- [x] Render the existing `InspectionBytesResponse` through Ratatui's
      `TestBackend` in the Rust/WASM adapter.
- [x] Serialize normalized cells, dimensions, colors, and supported modifiers
      as provider evidence.
- [x] Retain the returned cells as adapter and corpus evidence. The public lab
      now renders them only through the Slice 5c interactive terminal.
- [ ] Add a browser-facing contract test that rejects malformed cell snapshots
      without interpreting Tosumu storage data.

#### Acceptance Criteria

- [x] Ratatui consumes the same provider-neutral boundary response as the
      TypeScript semantic view.
- [x] Browser code displays cells only; it does not parse database bytes,
      execute Crossterm, or infer inspection facts.
- [x] The retained projection is explicitly supplemental Ratatui provider
      evidence and is not a second public viewport.
- [x] A reviewed fixture visibly yields matching semantic status and the
      interactive Ratatui terminal status in the public browser lab.

#### Exit State

The retained evidence proves the same bounded response can be rendered through
Ratatui. The public lab presents its portable TypeScript semantic observation
alongside the single interactive Ratatui session introduced in Slice 5c.

### Slice 5b: Provider-Local Ratatui Viewport Interaction

**Objective:** Make the browser-visible Ratatui evidence inspectable without
turning it into a second Tosumu command API or a browser terminal emulator.

**Disposition:** Superseded as a separate public viewport by Slice 5c. Its
bounded viewport-event handling remains adapter evidence; command-session
history and transcript navigation are the browser-visible interaction surface.

#### Deliverables

- [x] Retain the already-resolved raw-byte response in a Rust/WASM
      provider-local projection session.
- [x] Normalize browser wheel and keyboard navigation into bounded viewport
      events: scroll, page, home, and end.
- [x] Re-render only the Ratatui `TestBackend` cell projection after a
      viewport event; retain the original response unchanged.
- [x] Keep all storage commands, file selection, parsing, and DTO construction
      outside the projection interaction path.
- [ ] Retain manual browser evidence for focus, wheel, keyboard, and an
      explicit no-active-session failure.

#### Acceptance Criteria

- [x] Arrow keys, page keys, home/end, and wheel movement navigate a focused
      projection viewport.
- [x] A viewport event cannot alter the inspected bytes or the resolved
      `InspectionBytesResponse`.
- [x] Unsupported events and missing sessions fail explicitly.
- [x] Browser code forwards normalized events and renders returned cells only;
      it does not interpret Tosumu storage data.
- [ ] A public-lab interaction check confirms navigation remains usable under
      normal browser zoom and narrow-width conditions.

#### Exit State

The retained headless projection remains provider-local evidence over the
semantic DTO and does not claim native-terminal parity. Slice 5c is the one
browser-visible Ratatui surface and does not widen the raw-byte inspection
contract.

Manual evidence:

- desktop keyboard and pointer interaction;
- narrow viewport and zoomed text;
- JavaScript-disabled fallback;
- malformed and oversized upload behavior.

### Slice 5c: Browser-Safe Ratatui Command Session

**Objective:** Prove that a browser can host a genuinely interactive,
Rust/WASM-owned Ratatui command session over an already resolved raw-byte
observation, without making JavaScript a terminal emulator or giving the
session native storage authority.

#### Deliverables

- [x] Keep the active inspection response, transcript, prompt, history, and
      output viewport inside the Rust/WASM adapter.
- [x] Define bounded normalized text and key events for prompt input,
      backspace, submit, history, and transcript navigation.
- [x] Support only `HELP`, `STATUS`, and `CLEAR` in the explicitly labelled
      browser-safe profile. `STATUS` may render only facts already present in
      `InspectionBytesResponse`.
- [x] Return a fresh Ratatui `TestBackend` cell projection after each accepted
      input event.
- [x] Forward browser text through a transient accessibility-safe input
      transport and normalized keys through the focused terminal boundary;
      clear the browser buffer immediately and prevent page scrolling only for
      accepted terminal navigation.
- [x] Allocate a real prompt interior row in the Ratatui layout and regress
      prompt redraw before submission with a normalized `text:` event test.
- [x] Add tests for prompt edits, submit, bounded history, clear, unsupported
      commands, and preservation of the original inspection response.

#### Acceptance Criteria

- [ ] Typing, backspace, Enter, and command history are visibly usable in the
      generated browser terminal projection. Rust-side session and normalized
      event tests pass; retain this checkbox for public-browser evidence.
- [x] JavaScript does not retain prompt, transcript, command parsing, or
      storage-command state.
- [x] The profile cannot create, open, unlock, mutate, or disclose data beyond
      the raw-byte observation boundary.
- [x] Unsupported commands produce an explicit provider-local diagnostic.
- [x] The semantic DTO remains authoritative and unchanged throughout the
      command session.

#### Deferred Evidence

This slice does **not** execute Tosumu Command Language commands. Existing TQL
dispatch requires a reviewed native `KvStore`, verification state, and WAL
context; arbitrary browser-provided bytes intentionally do not provide those
capabilities. A later provider-neutral command boundary must be proven before
TQL execution can be admitted to a browser host.

#### Exit State

The public island demonstrates an interactive Ratatui session whose state and
rendering remain Rust/WASM-owned. It is a bounded command-window proof, not a
browser-native terminal, Crossterm host, or storage console.

### Slice 6: Provider Parity And Hardening

**Objective:** Compare providers at semantic boundaries and retain failures as
actionable evidence.

#### Deliverables

- [~] Define a provider parity matrix for facts, commands, diagnostics, and
      unsupported states. The raw-byte header row is now covered; unlocked
      pager, command, and WPF comparison rows remain open.
- [ ] Compare Ratatui, TypeScript/WASM, and WPF observations for the reviewed
      fixture.
- [~] Retain additional provider-path fixture coverage. Compatible, populated,
      malformed, truncated, and newer-format raw-byte cases are reviewed;
      WAL-present, protector-present, empty-store lifecycle, and partial-
      verification cases remain native-provider evidence.
- [ ] Add fuzz or mutation coverage for boundary decoding and command input.
- [ ] Record performance budgets for observation generation and browser
      rendering separately.

#### Acceptance Criteria

- [~] Providers agree on fact identity and command outcomes. Core, boundary,
      and WASM wrapper parity is asserted for the header-only fixture; commands
      and provider session comparisons remain open.
- [ ] Layout differences are not misreported as semantic divergences.
- [ ] A divergence is classified as provider-only behavior, contract
      refinement, or rejected semantics.
- [ ] No provider silently widens the shared contract.

#### Validation

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
mkdocs build --strict
```

#### Exit State

The two primary UI providers and the independent WPF consumer provide enough
evidence to review the structured inspection boundary.

#### Current Parity Evidence

The current provider matrix deliberately distinguishes two observation
capabilities rather than treating an uploaded file as an unlocked native
store:

| Concern | Core raw-byte observation | Boundary DTO | WASM wrapper | Native Ratatui / WPF |
| --- | --- | --- | --- | --- |
| Plaintext header | Supported | Supported | Supported | Supported after native open |
| Page list | Explicitly bounded/unavailable | Explicitly bounded/unavailable | Same DTO result | Supported through approved pager |
| Tree, WAL, keyslots | Explicitly unavailable | Explicitly unavailable | Same DTO result | May be available through approved provider paths |
| Commands | Not admitted for raw bytes | Not admitted | Not admitted | Existing native session only |

Focused tests assert core-to-boundary field parity and boundary-to-WASM JSON
parity for the reviewed raw-byte fixture. The difference from a native
Ratatui/WPF session is intentional capability scope, not an unexplained
semantic divergence. Full provider parity still requires a reviewed unlocked
fixture, command-session adapter, and WPF comparison report.

### Slice 7: AR-0002 Review And Parking Decision

**Objective:** Close the plan honestly and decide whether the shared
observation model graduates, keeps incubating, or is retired.

#### Deliverables

- [ ] Update AR-0002 with direct non-CLI and provider-parity evidence.
- [ ] Decide the permanent ownership and crate/module location of shared
      observation types.
- [ ] Record whether CLI and WASM encodings remain separate adapters.
- [ ] Update the Inspect API Specification and public website documentation.
- [ ] Record deferred mutation, shell, and provider work with reopening
      triggers.

#### Acceptance Criteria

- [ ] The disposition names what is accepted and what remains provisional.
- [ ] Any ownership change is recorded in an ADR before becoming binding.
- [ ] Remaining work has an owner and explicit destination.
- [ ] Ratatui and the TypeScript island remain independently useful.

#### Validation

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
mkdocs build --strict
```

#### Exit State

AR-0002 records whether structured interactive inspection has graduated from
CLI-side incubation into a reusable Tosumu capability.

## Validation Matrix

| Concern | Evidence | Command Or Artifact | Required Result |
| --- | --- | --- | --- |
| Core facts | Existing inspection tests | `cargo test -p tosumu-core inspect` | Pass |
| Shared observation | Deterministic fixture snapshots | Focused observation tests | Stable facts and bounds |
| Commands | Reducer and invalid-input tests | Focused session tests | Deterministic typed outcomes |
| Ratatui | Provider translation and native smoke test | `cargo test -p tosumu-cli view` | Existing workflow preserved |
| WASM | Boundary and malformed-input tests | `wasm-pack test --node ...` | No panic or semantic drift |
| TypeScript | Island lifecycle and interaction tests | `node --test <island-test-path>` | Pass |
| Provider parity | Retained fixture matrix | Generated semantic report | No unexplained divergence |
| Security | Disclosure and protector fixtures | Boundary tests and review | No secret material emitted |
| Resource limits | Oversized and adversarial inputs | Limit tests | Typed bounded rejection |
| Accessibility | Keyboard, focus, zoom, and fallback | Manual plus automated checks | Complete usable path |
| Documentation | Strict site build | `mkdocs build --strict` | Pass |

## Failure And Diagnostic Semantics

Expected failure classes include:

- invalid or truncated database bytes;
- unsupported format or minimum reader version;
- authentication, corruption, and I/O findings;
- incomplete B-tree verification;
- unavailable WAL or protector details;
- unsupported protected database in browser mode;
- observation or command limits exceeded;
- unknown observation schema;
- stale selection after refresh;
- WASM initialization or provider unavailability.

Tosumu owns storage and inspection failure identity. The WASM adapter owns
boundary translation. Ratatui and TypeScript own presentation of those
failures. No provider may replace an expected failure with an empty panel,
silent fallback, or invented success.

## Security And Disclosure

- Public fixtures must be reviewed and intentionally non-secret.
- Browser upload bytes remain session-local and are not transmitted by the
  default island.
- Passphrases, recovery keys, key files, DEKs, and decrypted secret values must
  never appear in observations, logs, DOM attributes, URLs, or retained site
  artifacts.
- Record previews default to bounded metadata or reviewed redacted content.
- Recursive tree, record, WAL, and diagnostic output must have hard limits.
- Browser persistence requires a separate review and explicit user action.

## Compatibility And Versioning

- The Tosumu on-disk format is unchanged by this plan.
- CLI JSON remains governed by the Inspect API Specification.
- The WASM observation encoding starts versioned even if only schema 1 exists.
- Unknown boundary schemas fail explicitly.
- Ratatui and TypeScript may evolve independently while preserving admitted
  observation and command semantics.
- Breaking shared semantic changes require review; provider layout changes do
  not, provided they preserve the contract.

## Risks

- Extracting `ViewApp` mechanically could preserve terminal assumptions in the
  shared contract.
- Reusing CLI JSON directly could turn one transport encoding into engine
  semantics.
- A browser island could expose more record content than public evidence needs.
- Provider parity could be mistaken for pixel parity and create unnecessary UI
  coupling.
- Tokimu hosting could become confused with Tosumu semantic ownership.
- A broad command vocabulary could prematurely turn this inspector into a
  general database shell.

## Open Questions

- Should the shared observation model live in `tosumu-core`, a dedicated
  inspection crate, or remain adjacent to the CLI until the island is proven?
- Does the browser need one complete observation or demand-driven view
  observations to remain bounded?
- Which record fields are safe and useful in a public inspector?
- Should watch/refresh be represented as a command or remain provider policy?
- Should the WPF harness consume the same Rust-side observation directly or
  continue through a serialized boundary?
- Does later mutation belong in this inspector, TQL, or a separate command
  service?
- Can a future Tokimu MUD or shell reuse the observation/command pattern
  without importing Tosumu-specific vocabulary?

## Graduation Criteria

This plan may close as graduated only when:

- Ratatui and TypeScript consume one provider-neutral observation and command
  model;
- one direct non-CLI Rust/WASM consumer exercises the complete retained
  inspection workflow;
- semantic parity tests cover ordinary and failing fixtures;
- provider UI state does not leak into the shared contract;
- disclosure and resource limits are explicit and tested;
- AR-0002 records a final disposition;
- an ADR records any new accepted ownership boundary.

Otherwise the work remains incubating with explicit reopening triggers.
