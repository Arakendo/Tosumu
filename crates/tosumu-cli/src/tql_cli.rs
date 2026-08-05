//! CLI adapter for the bounded, read-only TQL subset.
//!
//! This module owns file opening, verification snapshot selection, and terminal
//! or JSON presentation. The parser and capability adapter remain reusable
//! without CLI concerns.

use std::path::Path;
use std::time::{Duration, Instant};

use serde::Serialize;
use tosumu_core::error::ErrorReport;
use tosumu_core::inspect::{inspect_verification, inspect_wal};
use tosumu_core::KvStore;

use crate::error_boundary::CliError;
use crate::inspect_contract::{error_payload_from_report, InspectErrorPayload};
use crate::tql::{parse, TqlCommand};
use crate::tql_dispatch::{execute, CheckState, TqlOutcome};
use crate::tql_render::{render_human, render_json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TqlRunOutcome {
    Success,
    ReportedIssues,
}

/// Local execution observations emitted only when the CLI caller explicitly
/// requests `--timings`. They are not part of the TQL human or JSON schemas.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TqlTimingObservation {
    parse: Duration,
    open: Duration,
    inspection: Duration,
    dispatch: Duration,
    render: Duration,
}

impl TqlTimingObservation {
    fn render(self) -> String {
        format!(
            "tql timings: parse_us={} open_us={} inspection_us={} dispatch_us={} render_us={}",
            self.parse.as_micros(),
            self.open.as_micros(),
            self.inspection.as_micros(),
            self.dispatch.as_micros(),
            self.render.as_micros(),
        )
    }

    fn write_to_stderr(self) {
        eprintln!("{}", self.render());
    }
}

#[derive(Serialize)]
struct TqlErrorEnvelope {
    schema_version: u8,
    command: &'static str,
    outcome: Option<()>,
    error: InspectErrorPayload,
}

/// Opens an unencrypted store read-only and executes one bounded TQL command.
/// `CHECK` requests the existing public verification snapshot; other commands
/// do not pay that cost or imply that verification occurred.
pub(crate) fn run_tql(
    path: &Path,
    input: &str,
    json_output: bool,
    report_timings: bool,
) -> Result<TqlRunOutcome, CliError> {
    let parse_started = Instant::now();
    let command = parse(input)?;
    let parse = parse_started.elapsed();

    let open_started = Instant::now();
    let store = KvStore::open_readonly(path)?;
    let open = open_started.elapsed();

    let inspection_started = Instant::now();
    let verification = if matches!(command, TqlCommand::Check) {
        Some(inspect_verification(path)?)
    } else {
        None
    };
    let wal = if matches!(command, TqlCommand::WalStatus) {
        Some(inspect_wal(path)?)
    } else {
        None
    };
    let inspection = inspection_started.elapsed();

    let dispatch_started = Instant::now();
    let outcome = execute(&command, &store, verification.as_ref(), wal.as_ref())?;
    let dispatch = dispatch_started.elapsed();

    let render_started = Instant::now();
    if json_output {
        println!(
            "{}",
            render_json(&outcome).map_err(|error| {
                tosumu_core::TosumuError::Io(std::io::Error::other(error.to_string()))
            })?
        );
    } else {
        println!("{}", render_human(&outcome));
    }
    let render = render_started.elapsed();
    if report_timings {
        TqlTimingObservation {
            parse,
            open,
            inspection,
            dispatch,
            render,
        }
        .write_to_stderr();
    }
    Ok(run_outcome(&outcome))
}

fn run_outcome(outcome: &TqlOutcome) -> TqlRunOutcome {
    match outcome {
        TqlOutcome::Check(check)
            if check.page_integrity == CheckState::Failed
                || check.tree_integrity == CheckState::Failed =>
        {
            TqlRunOutcome::ReportedIssues
        }
        _ => TqlRunOutcome::Success,
    }
}

/// Renders a TQL failure in the same provisional schema family as successful
/// TQL outcomes. The payload remains structured instead of borrowing the
/// inspect command envelope merely because both are CLI diagnostics.
pub(crate) fn render_tql_error_report_json(report: &ErrorReport) -> String {
    serde_json::to_string_pretty(&TqlErrorEnvelope {
        schema_version: 1,
        command: "TQL",
        outcome: None,
        error: error_payload_from_report(report),
    })
    .unwrap_or_else(|serialization_error| {
        let message = serde_json::to_string(&serialization_error.to_string())
            .unwrap_or_else(|_| "\"failed to serialize TQL error\"".to_string());
        format!(
            "{{\"schema_version\":1,\"command\":\"TQL\",\"outcome\":null,\"error\":{{\"code\":\"TQL_ERROR_SERIALIZATION_FAILED\",\"status\":\"external_failure\",\"message\":{message}}}}}"
        )
    })
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use proptest::prelude::*;

    use super::*;
    use crate::tql_dispatch::{CheckOutcome, DescriptionOutcome, StatusOutcome, WalStatusOutcome};
    use crate::tql_render::{render_human, render_json};

    #[test]
    fn human_rendering_reports_source_facts_without_value_contents() {
        let output = render_human(&TqlOutcome::Description(DescriptionOutcome::Found {
            key: "asset/manifest".to_string(),
            value_bytes: 28,
        }));
        assert!(output.contains("asset/manifest"));
        assert!(output.contains("value_bytes"));
        assert!(!output.contains("fixture-schema"));
    }

    #[test]
    fn description_renderers_never_receive_or_emit_stored_value_contents() {
        // `DescriptionOutcome` intentionally has no value field. This sentinel
        // represents a value loaded by the current provider adapter only to
        // measure its public byte length; it must not cross into rendering.
        let stored_value = "recovery-key=do-not-render";
        let outcome = TqlOutcome::Description(DescriptionOutcome::Found {
            key: "asset/manifest".to_string(),
            value_bytes: stored_value.len(),
        });

        let human = render_human(&outcome);
        let json = render_json(&outcome).expect("serialize description JSON");

        assert!(human.contains("value_bytes"));
        assert!(json.contains("value_bytes"));
        assert!(!human.contains(stored_value));
        assert!(!json.contains(stored_value));
        assert!(!json.contains("recovery-key"));
    }

    #[test]
    fn json_outcomes_preserve_not_checked_instead_of_claiming_a_pass() {
        let output = render_json(&TqlOutcome::Check(CheckOutcome {
            page_integrity: CheckState::NotChecked,
            pages_checked: 0,
            pages_ok: 0,
            page_issue_count: 0,
            tree_integrity: CheckState::NotChecked,
        }))
        .expect("serialize JSON result");
        assert!(output.contains("not_checked"));
        assert!(output.contains("schema_version"));
    }

    #[test]
    fn json_status_output_names_the_command_and_facts() {
        let output = render_json(&TqlOutcome::Status(StatusOutcome {
            page_count: 3,
            data_pages: 2,
            tree_height: 1,
        }))
        .expect("serialize JSON result");
        assert!(output.contains("STATUS"));
        assert!(output.contains("page_count"));
    }

    #[test]
    fn timing_observation_is_bounded_and_outside_the_tql_result_schema() {
        let observation = TqlTimingObservation {
            parse: Duration::from_micros(1),
            open: Duration::from_micros(2),
            inspection: Duration::from_micros(3),
            dispatch: Duration::from_micros(4),
            render: Duration::from_micros(5),
        };

        assert_eq!(
            observation.render(),
            "tql timings: parse_us=1 open_us=2 inspection_us=3 dispatch_us=4 render_us=5"
        );
        assert!(
            !render_json(&TqlOutcome::Status(StatusOutcome {
                page_count: 1,
                data_pages: 0,
                tree_height: 0,
            }))
            .expect("serialize status JSON")
            .contains("timings"),
            "performance observations must remain outside the result schema"
        );
    }

    #[test]
    fn json_wal_status_output_exposes_only_bounded_wal_facts() {
        let output = render_json(&TqlOutcome::WalStatus(WalStatusOutcome {
            wal_exists: true,
            record_count: 4,
        }))
        .expect("serialize JSON result");
        assert!(output.contains("WAL STATUS"));
        assert!(output.contains("record_count"));
        assert!(!output.contains("wal_path"));
    }

    #[test]
    fn completed_check_uses_a_non_success_outcome_only_for_reported_failures() {
        let failed = TqlOutcome::Check(CheckOutcome {
            page_integrity: CheckState::Failed,
            pages_checked: 2,
            pages_ok: 1,
            page_issue_count: 1,
            tree_integrity: CheckState::NotChecked,
        });
        let passed = TqlOutcome::Check(CheckOutcome {
            page_integrity: CheckState::Passed,
            pages_checked: 2,
            pages_ok: 2,
            page_issue_count: 0,
            tree_integrity: CheckState::Passed,
        });

        assert_eq!(run_outcome(&failed), TqlRunOutcome::ReportedIssues);
        assert_eq!(run_outcome(&passed), TqlRunOutcome::Success);
    }

    #[test]
    fn json_errors_use_the_tql_schema_and_preserve_typed_details() {
        let rendered = render_tql_error_report_json(
            &CliError::Tql(crate::tql::TqlParseError::MissingArgument {
                command: "DESCRIBE",
                argument: "a key",
            })
            .error_report(),
        );
        let json: serde_json::Value =
            serde_json::from_str(&rendered).expect("valid TQL error JSON");

        assert_eq!(json["schema_version"], 1);
        assert_eq!(json["command"], "TQL");
        assert!(json["outcome"].is_null());
        assert_eq!(json["error"]["code"], "TQL_MISSING_ARGUMENT");
        assert_eq!(json["error"]["status"], "invalid_input");
        assert_eq!(json["error"]["details"]["command"], "DESCRIBE");
        assert_eq!(json["error"]["details"]["argument"], "a key");
    }

    #[test]
    fn parse_failure_precedes_database_opening_or_creation() {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock should be after the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!("tosumu-tql-parse-first-{nonce}.tsm"));
        assert!(!path.exists(), "test path must start absent");

        let error = run_tql(&path, "STATUS trailing", false, false)
            .expect_err("invalid TQL must fail before a database is opened");
        assert!(matches!(
            error,
            CliError::Tql(crate::tql::TqlParseError::UnexpectedToken {
                command: "STATUS",
                ..
            })
        ));
        assert!(
            !path.exists(),
            "a parse failure must not create or otherwise touch the database path"
        );
    }

    #[test]
    fn maximum_size_unknown_command_has_a_bounded_structured_error() {
        let input = "X".repeat(crate::tql::MAX_COMMAND_BYTES);
        let error = crate::tql::parse(&input).expect_err("unknown command is rejected");
        let rendered = render_tql_error_report_json(&CliError::Tql(error).error_report());
        let json: serde_json::Value =
            serde_json::from_str(&rendered).expect("bounded TQL error remains valid JSON");

        assert_eq!(json["error"]["code"], "TQL_UNKNOWN_COMMAND");
        assert_eq!(
            json["error"]["details"]["token"].as_str().map(str::len),
            Some(crate::tql::MAX_COMMAND_BYTES)
        );
        assert!(
            rendered.len() <= crate::tql::MAX_COMMAND_BYTES + 1024,
            "the bounded parser input must not produce an unbounded diagnostic"
        );
    }

    proptest! {
        #[test]
        fn bounded_description_keys_render_as_valid_json(
            key in prop::collection::vec(
                proptest::char::range('a', 'z'),
                0..=crate::tql::MAX_KEY_BYTES
            )
        ) {
            let key = key.into_iter().collect::<String>();
            let rendered = render_json(&TqlOutcome::Description(DescriptionOutcome::Missing {
                key: key.clone(),
            }))
            .expect("bounded description outcome must serialize");
            let json: serde_json::Value = serde_json::from_str(&rendered)
                .expect("TQL description JSON must remain valid");

            prop_assert_eq!(json["command"].as_str(), Some("DESCRIBE"));
            prop_assert_eq!(json["outcome"]["key"].as_str(), Some(key.as_str()));
            prop_assert_eq!(json["outcome"]["state"].as_str(), Some("missing"));
            prop_assert!(
                rendered.len() <= key.len() + 512,
                "bounded description metadata must not create unbounded JSON"
            );
        }
    }
}
