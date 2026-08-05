export const INSPECTION_BYTE_LIMIT = 16 * 1024 * 1024;

export const BUNDLED_FIXTURE_NAME = "inspection-header-fixture-v1.tosumu";

export function fixtureButtonState(button) {
  return {
    label: button.dataset.tosumuInspectionFixtureLabel,
    expectation: button.dataset.tosumuInspectionFixtureExpectation,
    url: button.dataset.tosumuInspectionFixtureUrl,
  };
}

export function selectedFileState(file) {
  if (!file) {
    return {
      canInspect: false,
      message: "Choose a database file to inspect its bounded header observation.",
    };
  }

  if (file.size > INSPECTION_BYTE_LIMIT) {
    return {
      canInspect: false,
      message: `Selected file is ${file.size} bytes; the browser boundary accepts at most ${INSPECTION_BYTE_LIMIT} bytes.`,
    };
  }

  return {
    canInspect: true,
    message: `Selected ${file.name} (${file.size} bytes). Ready for header-only inspection.`,
  };
}

export function bundledFixtureState() {
  return {
    canInspect: true,
    message: `Bundled ${BUNDLED_FIXTURE_NAME} is ready for header-only inspection.`,
  };
}

export function renderInspectionResponse(json) {
  const response = JSON.parse(json);
  if (response.outcome === "failure") {
    return [
      "BOUNDARY RESULT / REJECTED",
      `Code: ${response.error.code}`,
      `Status: ${response.error.status}`,
      `Reason: ${response.error.message}`,
      "",
      "The browser submitted opaque bytes. Tosumu rejected them before claiming a storage observation.",
      "",
      "RAW DTO",
      JSON.stringify(response, null, 2),
    ].join("\n");
  }

  const observation = response.observation;
  return [
    "BOUNDARY RESULT / HEADER OBSERVATION",
    `Input: ${observation.source.byte_count} bytes / ${observation.source.unlock_state.replaceAll("_", " ")}`,
    `Format: v${observation.header.format_version} / ${observation.header.page_size} byte pages`,
    `Container: ${observation.header.page_count} declared pages / root page ${observation.header.root_page}`,
    `Header: flags 0x${observation.header.flags.toString(16).padStart(4, "0")} / ${observation.header.keyslot_count} declared keyslots`,
    "",
    "NOT OBSERVED IN THIS BROWSER MODE",
    "- protected page integrity or decrypted records",
    "- B-tree contents or record count",
    "- WAL sidecars and provider-owned filesystem state",
    "- keyslot enumeration or protector unlock state",
    "",
    "RAW DTO",
    JSON.stringify(response, null, 2),
  ].join("\n");
}
