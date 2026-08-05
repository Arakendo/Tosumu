//! Fuzz target: fuzz_tql_render
//!
//! Exercises the pure TQL outcome renderers with bounded synthetic results.
//! It deliberately avoids store opening and provider inspection so this target
//! checks the presentation boundary without making physical storage a fuzzer
//! dependency.

#![no_main]
#![allow(dead_code)] // Path-included CLI modules intentionally expose more than this target uses.

use libfuzzer_sys::fuzz_target;

#[path = "../../crates/tosumu-cli/src/tql.rs"]
mod tql;
#[path = "../../crates/tosumu-cli/src/tql_dispatch.rs"]
mod tql_dispatch;
#[path = "../../crates/tosumu-cli/src/tql_render.rs"]
mod tql_render;

use tql_dispatch::{
    CheckOutcome, CheckState, DescriptionOutcome, StatusOutcome, TqlOutcome, WalStatusOutcome,
};

fn bounded_utf8(data: &[u8]) -> String {
    let text = String::from_utf8_lossy(data);
    let mut end = 0;
    for (index, character) in text.char_indices() {
        let next = index + character.len_utf8();
        if next > tql::MAX_KEY_BYTES {
            break;
        }
        end = next;
    }
    text[..end].to_owned()
}

fn word(data: &[u8], offset: usize) -> u64 {
    let mut bytes = [0_u8; 8];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = data.get(offset + index).copied().unwrap_or_default();
    }
    u64::from_le_bytes(bytes)
}

fuzz_target!(|data: &[u8]| {
    let selector = data.first().copied().unwrap_or_default() % 5;
    let key = bounded_utf8(data);
    let outcome = match selector {
        0 => TqlOutcome::Status(StatusOutcome {
            page_count: word(data, 1),
            data_pages: word(data, 9),
            tree_height: (word(data, 17) as usize).min(usize::MAX),
        }),
        1 => TqlOutcome::Check(CheckOutcome {
            page_integrity: [CheckState::Passed, CheckState::Failed, CheckState::NotChecked]
                [(data.get(1).copied().unwrap_or_default() % 3) as usize],
            pages_checked: word(data, 2),
            pages_ok: word(data, 10),
            page_issue_count: word(data, 18) as usize,
            tree_integrity: [CheckState::Passed, CheckState::Failed, CheckState::NotChecked]
                [(data.get(26).copied().unwrap_or_default() % 3) as usize],
        }),
        2 => TqlOutcome::Description(DescriptionOutcome::Found {
            key,
            value_bytes: word(data, 1) as usize,
        }),
        3 => TqlOutcome::Description(DescriptionOutcome::Missing { key }),
        _ => TqlOutcome::WalStatus(WalStatusOutcome {
            wal_exists: data.get(1).copied().unwrap_or_default() & 1 == 1,
            record_count: word(data, 2) as usize,
        }),
    };

    let human = tql_render::render_human(&outcome);
    let first_json = tql_render::render_json(&outcome).expect("TQL outcome serializes");
    let second_json = tql_render::render_json(&outcome).expect("TQL outcome serializes twice");
    assert_eq!(first_json, second_json, "TQL rendering must be deterministic");
    let value: serde_json::Value = serde_json::from_str(&first_json).expect("valid TQL JSON");
    assert_eq!(value["schema_version"], 1);
    assert!(human.len() <= tql::MAX_KEY_BYTES.saturating_mul(8) + 1024);
    assert!(first_json.len() <= tql::MAX_KEY_BYTES.saturating_mul(8) + 1024);
});
