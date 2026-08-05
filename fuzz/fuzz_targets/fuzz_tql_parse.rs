//! Fuzz target: fuzz_tql_parse
//!
//! Feed arbitrary bytes through the bounded TQL syntax boundary. Invalid UTF-8
//! is intentionally rejected before the string-only grammar runs; valid UTF-8
//! must parse deterministically and never panic. This target includes the
//! parser source directly so TQL remains CLI-local while it is incubating.

#![no_main]

use libfuzzer_sys::fuzz_target;

#[path = "../../crates/tosumu-cli/src/tql.rs"]
mod tql;

fuzz_target!(|data: &[u8]| {
    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    let first = tql::parse(input);
    let second = tql::parse(input);
    assert_eq!(first, second, "TQL parsing must be deterministic");
});
