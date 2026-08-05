import assert from "node:assert/strict";
import test from "node:test";

import {
  BUNDLED_FIXTURE_NAME,
  INSPECTION_BYTE_LIMIT,
  bundledFixtureState,
  fixtureButtonState,
  renderInspectionResponse,
  selectedFileState,
} from "./tosumu-inspection-island-contract.js";

test("selected file state rejects missing and oversized input before WASM", () => {
  assert.equal(selectedFileState(undefined).canInspect, false);
  assert.equal(
    selectedFileState({ name: "too-large.tosumu", size: INSPECTION_BYTE_LIMIT + 1 }).canInspect,
    false,
  );
});

test("selected file state accepts bounded input", () => {
  const state = selectedFileState({ name: "fixture.tosumu", size: 128 });
  assert.equal(state.canInspect, true);
  assert.match(state.message, /fixture\.tosumu/);
});

test("bundled fixture remains an explicit transport-only inspection source", () => {
  const state = bundledFixtureState();
  assert.equal(state.canInspect, true);
  assert.match(state.message, new RegExp(BUNDLED_FIXTURE_NAME));
});

test("fixture button state preserves only declarative fixture transport metadata", () => {
  const fixture = fixtureButtonState({
    dataset: {
      tosumuInspectionFixtureLabel: "Invalid magic",
      tosumuInspectionFixtureExpectation: "rejected",
      tosumuInspectionFixtureUrl: "fixture.bin",
    },
  });
  assert.deepEqual(fixture, {
    label: "Invalid magic",
    expectation: "rejected",
    url: "fixture.bin",
  });
});

test("response rendering summarizes WASM facts without parsing storage bytes", () => {
  const rendered = renderInspectionResponse(JSON.stringify({
    outcome: "observation",
    observation: {
      source: { byte_count: 8192, unlock_state: "header_only" },
      header: { format_version: 2, page_size: 4096, page_count: 2, root_page: 1, flags: 3, keyslot_count: 1 },
    },
  }));
  assert.match(rendered, /HEADER OBSERVATION/);
  assert.match(rendered, /8192 bytes/);
  assert.match(rendered, /NOT OBSERVED/);
});

test("failure rendering retains the boundary's stable error identity", () => {
  const rendered = renderInspectionResponse('{"outcome":"failure","error":{"code":"FORMAT_NOT_TOSUMU","status":"unsupported","message":"not a Tosumu file"}}');
  assert.match(rendered, /REJECTED/);
  assert.match(rendered, /FORMAT_NOT_TOSUMU/);
});
