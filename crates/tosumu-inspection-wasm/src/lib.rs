//! Browser-facing adapter for Tosumu's bounded raw-byte inspection contract.
//!
//! The adapter receives browser-provided bytes and returns the provider-neutral,
//! versioned inspection DTO as JSON. It deliberately does not expose paths,
//! keys, protectors, or decrypted storage content.

mod ratatui_projection;
mod ratatui_terminal;

use ratatui_projection::{interact, render_inspection, start_session};
use ratatui_terminal::{interact as interact_terminal, start as start_terminal};
use tosumu_inspection_boundary::{
    inspect_bytes, inspect_bytes_json, InspectionBytesRequest, INSPECTION_BOUNDARY_SCHEMA_V1,
};
use wasm_bindgen::prelude::*;

/// Returns the version expected by [`inspect_uploaded_bytes`].
#[wasm_bindgen]
pub fn inspection_boundary_schema_version() -> u16 {
    INSPECTION_BOUNDARY_SCHEMA_V1
}

/// Inspects one browser-provided byte buffer through the reviewed raw-byte
/// boundary. Input failures are represented in the returned JSON response.
#[wasm_bindgen]
pub fn inspect_uploaded_bytes(bytes: &[u8]) -> Result<String, JsValue> {
    inspect_uploaded_bytes_json(bytes).map_err(|error| JsValue::from_str(&error))
}

/// Renders the same bounded inspection response through Ratatui's headless
/// `TestBackend` and returns normalized terminal-cell evidence for a browser
/// host. This is a browser-visible provider projection, not a Crossterm
/// terminal emulator and not a second inspection parser.
#[wasm_bindgen]
pub fn render_uploaded_bytes_ratatui(
    bytes: &[u8],
    width: u16,
    height: u16,
) -> Result<String, JsValue> {
    let response = inspect_bytes(InspectionBytesRequest::with_default_limit(bytes));
    start_session(response.clone());
    render_inspection(&response, width, height)
        .and_then(|snapshot| {
            serde_json::to_string(&snapshot)
                .map_err(|error| format!("serialize Ratatui projection: {error}"))
        })
        .map_err(|error| JsValue::from_str(&error))
}

/// Applies a provider-local navigation event to the active headless Ratatui
/// session. It cannot alter the inspected bytes or Tosumu observation facts.
#[wasm_bindgen]
pub fn interact_ratatui_projection(
    event: &str,
    width: u16,
    height: u16,
) -> Result<String, JsValue> {
    interact(event, width, height)
        .and_then(|snapshot| {
            serde_json::to_string(&snapshot)
                .map_err(|error| format!("serialize Ratatui interaction projection: {error}"))
        })
        .map_err(|error| JsValue::from_str(&error))
}

/// Starts a bounded Rust/WASM-owned Ratatui command session for the current
/// raw-byte inspection profile. The browser may forward normalized input but
/// never owns terminal state or executes Tosumu storage commands.
#[wasm_bindgen]
pub fn start_ratatui_terminal(bytes: &[u8], width: u16, height: u16) -> Result<String, JsValue> {
    let response = inspect_bytes(InspectionBytesRequest::with_default_limit(bytes));
    start_terminal(response);
    interact_terminal("key:end", width, height)
        .and_then(|snapshot| {
            serde_json::to_string(&snapshot)
                .map_err(|error| format!("serialize Ratatui terminal projection: {error}"))
        })
        .map_err(|error| JsValue::from_str(&error))
}

/// Applies one normalized browser event to the active browser-safe Ratatui
/// command profile. TQL execution and native storage operations remain out of
/// scope for the raw-byte browser boundary.
#[wasm_bindgen]
pub fn interact_ratatui_terminal(event: &str, width: u16, height: u16) -> Result<String, JsValue> {
    interact_terminal(event, width, height)
        .and_then(|snapshot| {
            serde_json::to_string(&snapshot)
                .map_err(|error| format!("serialize Ratatui terminal interaction: {error}"))
        })
        .map_err(|error| JsValue::from_str(&error))
}

fn inspect_uploaded_bytes_json(bytes: &[u8]) -> Result<String, String> {
    inspect_bytes_json(InspectionBytesRequest::with_default_limit(bytes))
        .map_err(|error| format!("inspection boundary serialization failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tosumu_core::format::{
        MAGIC, OFF_FLAGS, OFF_FORMAT_VERSION, OFF_KEYSLOT_COUNT, OFF_PAGE_COUNT, OFF_PAGE_SIZE,
        OFF_ROOT_PAGE, PAGE_SIZE,
    };
    use tosumu_inspection_boundary::{
        inspect_bytes_json, InspectionBytesRequest, INSPECTION_BOUNDARY_SCHEMA_V1,
    };

    fn fixture_bytes() -> Vec<u8> {
        let mut bytes = vec![0_u8; PAGE_SIZE];
        bytes[..MAGIC.len()].copy_from_slice(MAGIC);
        bytes[OFF_FORMAT_VERSION..OFF_FORMAT_VERSION + 2].copy_from_slice(&2_u16.to_le_bytes());
        bytes[OFF_PAGE_SIZE..OFF_PAGE_SIZE + 2].copy_from_slice(&(PAGE_SIZE as u16).to_le_bytes());
        bytes[OFF_FLAGS..OFF_FLAGS + 2].copy_from_slice(&0_u16.to_le_bytes());
        bytes[OFF_PAGE_COUNT..OFF_PAGE_COUNT + 8].copy_from_slice(&3_u64.to_le_bytes());
        bytes[OFF_ROOT_PAGE..OFF_ROOT_PAGE + 8].copy_from_slice(&1_u64.to_le_bytes());
        bytes[OFF_KEYSLOT_COUNT..OFF_KEYSLOT_COUNT + 2].copy_from_slice(&0_u16.to_le_bytes());
        bytes
    }

    #[test]
    fn adapter_preserves_the_versioned_boundary_response() {
        let bytes = fixture_bytes();
        let json = inspect_uploaded_bytes_json(&bytes).unwrap();
        let boundary =
            inspect_bytes_json(InspectionBytesRequest::with_default_limit(&bytes)).unwrap();

        assert_eq!(json, boundary);
        assert!(json.contains("\"outcome\":\"observation\""));
        assert!(json.contains("\"kind\":\"uploaded_bytes\""));
        assert!(json.contains("RAW_BYTES_TREE_UNAVAILABLE"));
        assert!(!json.contains("\"path\""));
    }

    #[test]
    fn adapter_exposes_the_shared_schema_version() {
        assert_eq!(
            inspection_boundary_schema_version(),
            INSPECTION_BOUNDARY_SCHEMA_V1
        );
    }

    #[test]
    fn adapter_projects_the_same_boundary_response_through_ratatui_cells() {
        let response = inspect_bytes(InspectionBytesRequest::with_default_limit(&fixture_bytes()));
        start_session(response.clone());
        let snapshot = render_inspection(&response, 64, 16).unwrap();
        let rendered = snapshot
            .cells
            .iter()
            .map(|cell| cell.symbol.as_str())
            .collect::<String>();

        assert_eq!(snapshot.width, 64);
        assert_eq!(snapshot.height, 16);
        assert!(rendered.contains("TOSUMU INSPECTION"));
        assert!(rendered.contains("HEADER ONLY"));
    }

    #[test]
    fn ratatui_navigation_keeps_the_inspection_response_immutable() {
        let response = inspect_bytes(InspectionBytesRequest::with_default_limit(&fixture_bytes()));
        start_session(response);
        let snapshot = interact("end", 64, 10).unwrap();
        let rendered = snapshot
            .cells
            .iter()
            .map(|cell| cell.symbol.as_str())
            .collect::<String>();

        assert!(rendered.contains("viewport controls"));
    }

    #[test]
    fn browser_safe_terminal_owns_prompt_and_command_transcript() {
        let response = inspect_bytes(InspectionBytesRequest::with_default_limit(&fixture_bytes()));
        start_terminal(response);
        let prompt_snapshot = interact_terminal("text:status", 72, 18).unwrap();
        assert!(ratatui_terminal::rendered_lines(&prompt_snapshot)
            .iter()
            .any(|line| line.contains("> status_")));

        interact_terminal("key:enter", 72, 18).unwrap();
        let snapshot = interact_terminal("key:end", 72, 18).unwrap();
        let rendered = ratatui_terminal::rendered_text(&snapshot);

        assert!(rendered.contains("format version"));
        assert!(rendered.contains("TOSUMU INSPECTION"));
    }

    #[test]
    fn browser_safe_terminal_rejects_storage_commands_explicitly() {
        let response = inspect_bytes(InspectionBytesRequest::with_default_limit(&fixture_bytes()));
        start_terminal(response);
        for event in ["text:describe pages", "key:enter"] {
            interact_terminal(event, 72, 18).unwrap();
        }
        let snapshot = interact_terminal("key:end", 72, 18).unwrap();
        let rendered = ratatui_terminal::rendered_text(&snapshot);

        assert!(rendered.contains("unsupported"));
        assert!(rendered.contains("HELP, STATUS, and CLEAR"));
    }
}
