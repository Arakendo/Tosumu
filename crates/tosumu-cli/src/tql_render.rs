//! Pure renderers for the provisional, CLI-local TQL outcome schema.
//!
//! This module deliberately knows only about typed outcomes. Opening stores,
//! collecting inspection facts, terminal emission, and CLI error envelopes
//! remain in their respective adapters.

use serde_json::json;

use crate::tql_dispatch::{
    CheckOutcome, CheckState, DescriptionOutcome, StatusOutcome, TqlOutcome, WalStatusOutcome,
};

pub(crate) fn render_human(outcome: &TqlOutcome) -> String {
    match outcome {
        TqlOutcome::Status(status) => format!(
            "status:\n  page_count: {}\n  data_pages: {}\n  tree_height: {}",
            status.page_count, status.data_pages, status.tree_height
        ),
        TqlOutcome::Check(check) => format!(
            "check:\n  page_integrity: {}\n  pages_checked: {}\n  pages_ok: {}\n  page_issue_count: {}\n  tree_integrity: {}",
            check_state_name(check.page_integrity),
            check.pages_checked,
            check.pages_ok,
            check.page_issue_count,
            check_state_name(check.tree_integrity)
        ),
        TqlOutcome::Description(DescriptionOutcome::Found { key, value_bytes }) => {
            format!("describe:\n  key: {key}\n  state: found\n  value_bytes: {value_bytes}")
        }
        TqlOutcome::Description(DescriptionOutcome::Missing { key }) => {
            format!("describe:\n  key: {key}\n  state: missing")
        }
        TqlOutcome::WalStatus(WalStatusOutcome {
            wal_exists,
            record_count,
        }) => format!("wal status:\n  wal_exists: {wal_exists}\n  record_count: {record_count}"),
    }
}

pub(crate) fn render_json(outcome: &TqlOutcome) -> Result<String, serde_json::Error> {
    let value = match outcome {
        TqlOutcome::Status(StatusOutcome {
            page_count,
            data_pages,
            tree_height,
        }) => json!({
            "schema_version": 1,
            "command": "STATUS",
            "outcome": {
                "page_count": page_count,
                "data_pages": data_pages,
                "tree_height": tree_height,
            },
        }),
        TqlOutcome::Check(CheckOutcome {
            page_integrity,
            pages_checked,
            pages_ok,
            page_issue_count,
            tree_integrity,
        }) => json!({
            "schema_version": 1,
            "command": "CHECK",
            "outcome": {
                "page_integrity": check_state_name(*page_integrity),
                "pages_checked": pages_checked,
                "pages_ok": pages_ok,
                "page_issue_count": page_issue_count,
                "tree_integrity": check_state_name(*tree_integrity),
            },
        }),
        TqlOutcome::Description(DescriptionOutcome::Found { key, value_bytes }) => json!({
            "schema_version": 1,
            "command": "DESCRIBE",
            "outcome": { "key": key, "state": "found", "value_bytes": value_bytes },
        }),
        TqlOutcome::Description(DescriptionOutcome::Missing { key }) => json!({
            "schema_version": 1,
            "command": "DESCRIBE",
            "outcome": { "key": key, "state": "missing" },
        }),
        TqlOutcome::WalStatus(WalStatusOutcome {
            wal_exists,
            record_count,
        }) => json!({
            "schema_version": 1,
            "command": "WAL STATUS",
            "outcome": { "wal_exists": wal_exists, "record_count": record_count },
        }),
    };
    serde_json::to_string_pretty(&value)
}

pub(crate) fn check_state_name(state: CheckState) -> &'static str {
    match state {
        CheckState::Passed => "passed",
        CheckState::Failed => "failed",
        CheckState::NotChecked => "not_checked",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_outcome_variant_has_a_stable_machine_command_name() {
        let cases = [
            (
                TqlOutcome::Status(StatusOutcome {
                    page_count: 1,
                    data_pages: 0,
                    tree_height: 1,
                }),
                "STATUS",
            ),
            (
                TqlOutcome::Check(CheckOutcome {
                    page_integrity: CheckState::NotChecked,
                    pages_checked: 0,
                    pages_ok: 0,
                    page_issue_count: 0,
                    tree_integrity: CheckState::NotChecked,
                }),
                "CHECK",
            ),
            (
                TqlOutcome::Description(DescriptionOutcome::Missing {
                    key: "asset/name".to_owned(),
                }),
                "DESCRIBE",
            ),
            (
                TqlOutcome::WalStatus(WalStatusOutcome {
                    wal_exists: false,
                    record_count: 0,
                }),
                "WAL STATUS",
            ),
        ];

        for (outcome, command) in cases {
            let rendered = render_json(&outcome).expect("serialize TQL outcome");
            let value: serde_json::Value = serde_json::from_str(&rendered).expect("valid JSON");
            assert_eq!(value["command"], command);
        }
    }
}
