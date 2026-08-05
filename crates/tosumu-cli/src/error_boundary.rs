use tosumu_core::error::{ErrorDetail, ErrorReport, ErrorStatus, ErrorValue, TosumuError};

use crate::tql::TqlParseError;

pub(crate) mod codes {
    pub const CLI_ARGUMENT_INVALID: &str = "CLI_ARGUMENT_INVALID";
    pub const CLI_KEY_NOT_FOUND: &str = "CLI_KEY_NOT_FOUND";
    pub const SQL_UNSUPPORTED_QUERY_SHAPE: &str = "SQL_UNSUPPORTED_QUERY_SHAPE";
    pub const TQL_EMPTY_INPUT: &str = "TQL_EMPTY_INPUT";
    pub const TQL_INPUT_TOO_LARGE: &str = "TQL_INPUT_TOO_LARGE";
    pub const TQL_TOO_MANY_TOKENS: &str = "TQL_TOO_MANY_TOKENS";
    pub const TQL_UNKNOWN_COMMAND: &str = "TQL_UNKNOWN_COMMAND";
    pub const TQL_MISSING_ARGUMENT: &str = "TQL_MISSING_ARGUMENT";
    pub const TQL_UNEXPECTED_TOKEN: &str = "TQL_UNEXPECTED_TOKEN";
    pub const TQL_KEY_TOO_LARGE: &str = "TQL_KEY_TOO_LARGE";
    pub const TQL_INVALID_KEY: &str = "TQL_INVALID_KEY";

    #[cfg(test)]
    pub const PUBLIC_CODES: &[&str] = &[
        CLI_ARGUMENT_INVALID,
        CLI_KEY_NOT_FOUND,
        SQL_UNSUPPORTED_QUERY_SHAPE,
        TQL_EMPTY_INPUT,
        TQL_INPUT_TOO_LARGE,
        TQL_TOO_MANY_TOKENS,
        TQL_UNKNOWN_COMMAND,
        TQL_MISSING_ARGUMENT,
        TQL_UNEXPECTED_TOKEN,
        TQL_KEY_TOO_LARGE,
        TQL_INVALID_KEY,
    ];
}

#[derive(Debug)]
pub(crate) enum CliError {
    Core(TosumuError),
    InspectStdinSecretEmpty {
        argument: &'static str,
        secret_kind: &'static str,
    },
    RecoveryKeyFormatInvalid,
    RecoveryKeyConfirmationFailed,
    KeyfilePathEmpty,
    PassphrasesDoNotMatch,
    KeyNotFound {
        key: String,
    },
    BackupDestinationExists {
        path: std::path::PathBuf,
    },
    Sql(tosumu_sql::SqlError),
    Tql(TqlParseError),
}

impl CliError {
    pub(crate) fn inspect_stdin_secret_empty(
        argument: &'static str,
        secret_kind: &'static str,
    ) -> Self {
        Self::InspectStdinSecretEmpty {
            argument,
            secret_kind,
        }
    }

    pub(crate) fn recovery_key_format_invalid() -> Self {
        Self::RecoveryKeyFormatInvalid
    }

    pub(crate) fn recovery_key_confirmation_failed() -> Self {
        Self::RecoveryKeyConfirmationFailed
    }

    pub(crate) fn backup_destination_exists(path: &std::path::Path) -> Self {
        Self::BackupDestinationExists {
            path: path.to_path_buf(),
        }
    }

    pub(crate) fn keyfile_path_empty() -> Self {
        Self::KeyfilePathEmpty
    }

    pub(crate) fn passphrases_do_not_match() -> Self {
        Self::PassphrasesDoNotMatch
    }

    pub(crate) fn key_not_found(key: &str) -> Self {
        Self::KeyNotFound {
            key: key.to_string(),
        }
    }

    pub(crate) fn error_report(&self) -> ErrorReport {
        match self {
            CliError::Core(error) => error.error_report(),
            CliError::InspectStdinSecretEmpty {
                argument,
                secret_kind,
            } => ErrorReport {
                code: codes::CLI_ARGUMENT_INVALID,
                status: ErrorStatus::InvalidInput,
                message: format!("stdin {secret_kind} must not be empty"),
                details: vec![
                    ErrorDetail {
                        key: "argument",
                        value: ErrorValue::Str((*argument).to_string()),
                    },
                    ErrorDetail {
                        key: "secret_kind",
                        value: ErrorValue::Str((*secret_kind).to_string()),
                    },
                    ErrorDetail {
                        key: "input_source",
                        value: ErrorValue::Str("stdin".to_string()),
                    },
                ],
            },
            CliError::RecoveryKeyFormatInvalid => ErrorReport {
                code: codes::CLI_ARGUMENT_INVALID,
                status: ErrorStatus::InvalidInput,
                message: "recovery key format is invalid".to_string(),
                details: vec![
                    ErrorDetail {
                        key: "field",
                        value: ErrorValue::Str("recovery_key".to_string()),
                    },
                    ErrorDetail {
                        key: "validation",
                        value: ErrorValue::Str("format".to_string()),
                    },
                ],
            },
            CliError::RecoveryKeyConfirmationFailed => ErrorReport {
                code: codes::CLI_ARGUMENT_INVALID,
                status: ErrorStatus::InvalidInput,
                message: "recovery key confirmation failed".to_string(),
                details: vec![
                    ErrorDetail {
                        key: "field",
                        value: ErrorValue::Str("recovery_key".to_string()),
                    },
                    ErrorDetail {
                        key: "validation",
                        value: ErrorValue::Str("confirmation".to_string()),
                    },
                ],
            },
            CliError::KeyfilePathEmpty => ErrorReport {
                code: codes::CLI_ARGUMENT_INVALID,
                status: ErrorStatus::InvalidInput,
                message: "keyfile path must not be empty".to_string(),
                details: vec![
                    ErrorDetail {
                        key: "field",
                        value: ErrorValue::Str("keyfile_path".to_string()),
                    },
                    ErrorDetail {
                        key: "validation",
                        value: ErrorValue::Str("required".to_string()),
                    },
                ],
            },
            CliError::PassphrasesDoNotMatch => ErrorReport {
                code: codes::CLI_ARGUMENT_INVALID,
                status: ErrorStatus::InvalidInput,
                message: "passphrases do not match".to_string(),
                details: vec![
                    ErrorDetail {
                        key: "field",
                        value: ErrorValue::Str("passphrase_confirmation".to_string()),
                    },
                    ErrorDetail {
                        key: "validation",
                        value: ErrorValue::Str("match".to_string()),
                    },
                ],
            },
            CliError::KeyNotFound { key } => ErrorReport {
                code: codes::CLI_KEY_NOT_FOUND,
                status: ErrorStatus::NotFound,
                message: "key not found".to_string(),
                details: vec![
                    ErrorDetail {
                        key: "key",
                        value: ErrorValue::Str(key.clone()),
                    },
                    ErrorDetail {
                        key: "operation",
                        value: ErrorValue::Str("get".to_string()),
                    },
                ],
            },
            CliError::BackupDestinationExists { path } => ErrorReport {
                code: codes::CLI_ARGUMENT_INVALID,
                status: ErrorStatus::InvalidInput,
                message: "backup destination already exists; choose a new path".to_string(),
                details: vec![
                    ErrorDetail {
                        key: "path",
                        value: ErrorValue::Str(path.display().to_string()),
                    },
                    ErrorDetail {
                        key: "operation",
                        value: ErrorValue::Str("backup".to_string()),
                    },
                ],
            },
            CliError::Sql(error) => ErrorReport {
                code: match error {
                    tosumu_sql::SqlError::UnsupportedQueryShape(_) => {
                        codes::SQL_UNSUPPORTED_QUERY_SHAPE
                    }
                    _ => codes::CLI_ARGUMENT_INVALID,
                },
                status: match error {
                    tosumu_sql::SqlError::UnsupportedQueryShape(_) => ErrorStatus::Unsupported,
                    tosumu_sql::SqlError::TableAlreadyExists { .. } => ErrorStatus::Conflict,
                    tosumu_sql::SqlError::TableNotFound { .. }
                    | tosumu_sql::SqlError::ColumnNotFound { .. } => ErrorStatus::NotFound,
                    _ => ErrorStatus::InvalidInput,
                },
                message: error.to_string(),
                details: vec![ErrorDetail {
                    key: "operation",
                    value: ErrorValue::Str("sql".to_string()),
                }],
            },
            CliError::Tql(error) => ErrorReport {
                code: tql_parse_error_code(error),
                status: ErrorStatus::InvalidInput,
                message: tql_parse_error_message(error),
                details: tql_parse_error_details(error),
            },
        }
    }
}

fn tql_parse_error_code(error: &TqlParseError) -> &'static str {
    match error {
        TqlParseError::EmptyInput => codes::TQL_EMPTY_INPUT,
        TqlParseError::InputTooLarge { .. } => codes::TQL_INPUT_TOO_LARGE,
        TqlParseError::TooManyTokens { .. } => codes::TQL_TOO_MANY_TOKENS,
        TqlParseError::UnknownCommand { .. } => codes::TQL_UNKNOWN_COMMAND,
        TqlParseError::MissingArgument { .. } => codes::TQL_MISSING_ARGUMENT,
        TqlParseError::UnexpectedToken { .. } => codes::TQL_UNEXPECTED_TOKEN,
        TqlParseError::KeyTooLarge { .. } => codes::TQL_KEY_TOO_LARGE,
        TqlParseError::InvalidKey { .. } => codes::TQL_INVALID_KEY,
    }
}

/// Keeps machine-readable token details available without repeating a bounded
/// but attacker-controlled token in both the human message and JSON details.
fn tql_parse_error_message(error: &TqlParseError) -> String {
    match error {
        TqlParseError::UnknownCommand { .. } => "unknown TQL command".to_string(),
        TqlParseError::UnexpectedToken { .. } => "unexpected TQL token".to_string(),
        _ => error.to_string(),
    }
}

fn tql_parse_error_details(error: &TqlParseError) -> Vec<ErrorDetail> {
    let mut details = vec![
        ErrorDetail {
            key: "operation",
            value: ErrorValue::Str("tql".to_string()),
        },
        ErrorDetail {
            key: "stage",
            value: ErrorValue::Str("parse".to_string()),
        },
    ];

    match error {
        TqlParseError::InputTooLarge { limit, actual }
        | TqlParseError::KeyTooLarge { limit, actual } => {
            details.push(ErrorDetail {
                key: "limit",
                value: ErrorValue::U64(*limit as u64),
            });
            details.push(ErrorDetail {
                key: "actual",
                value: ErrorValue::U64(*actual as u64),
            });
        }
        TqlParseError::TooManyTokens { limit } => details.push(ErrorDetail {
            key: "limit",
            value: ErrorValue::U64(*limit as u64),
        }),
        TqlParseError::UnknownCommand { command }
        | TqlParseError::UnexpectedToken { token: command, .. } => details.push(ErrorDetail {
            key: "token",
            value: ErrorValue::Str(command.clone()),
        }),
        TqlParseError::MissingArgument { command, argument } => {
            details.push(ErrorDetail {
                key: "command",
                value: ErrorValue::Str((*command).to_string()),
            });
            details.push(ErrorDetail {
                key: "argument",
                value: ErrorValue::Str((*argument).to_string()),
            });
        }
        TqlParseError::InvalidKey { reason } => details.push(ErrorDetail {
            key: "reason",
            value: ErrorValue::Str((*reason).to_string()),
        }),
        TqlParseError::EmptyInput => {}
    }

    details
}

impl From<TosumuError> for CliError {
    fn from(value: TosumuError) -> Self {
        CliError::Core(value)
    }
}

impl From<tosumu_sql::SqlError> for CliError {
    fn from(value: tosumu_sql::SqlError) -> Self {
        CliError::Sql(value)
    }
}

impl From<TqlParseError> for CliError {
    fn from(value: TqlParseError) -> Self {
        Self::Tql(value)
    }
}

#[cfg(test)]
mod tests {
    use super::codes::SQL_UNSUPPORTED_QUERY_SHAPE;
    use super::codes::{self, PUBLIC_CODES};
    use crate::tql::TqlParseError;

    #[test]
    fn documented_cli_public_codes_match_exported_constants() {
        let errors_md_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join("docs/Specifications/Tosumu Error Design Document.md");
        let errors_md = std::fs::read_to_string(&errors_md_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", errors_md_path.display()));

        let documented = extract_marked_code_block(
            &errors_md,
            "<!-- BEGIN_CLI_PUBLIC_CODES -->",
            "<!-- END_CLI_PUBLIC_CODES -->",
        );

        assert_eq!(documented, PUBLIC_CODES);
    }

    #[test]
    fn unsupported_sql_uses_stable_boundary_code_and_status() {
        let error = super::CliError::from(tosumu_sql::SqlError::unsupported_query_shape(
            "baseline SQL supports only primary-key equality lookups",
        ));
        let report = error.error_report();

        assert_eq!(report.code, SQL_UNSUPPORTED_QUERY_SHAPE);
        assert_eq!(report.status, tosumu_core::error::ErrorStatus::Unsupported);
        assert_eq!(
            report.message,
            "baseline SQL supports only primary-key equality lookups"
        );
    }

    #[test]
    fn tql_parse_errors_use_specific_stable_boundary_codes() {
        let cases = [
            (TqlParseError::EmptyInput, codes::TQL_EMPTY_INPUT),
            (
                TqlParseError::InputTooLarge {
                    limit: 4,
                    actual: 5,
                },
                codes::TQL_INPUT_TOO_LARGE,
            ),
            (
                TqlParseError::TooManyTokens { limit: 16 },
                codes::TQL_TOO_MANY_TOKENS,
            ),
            (
                TqlParseError::UnknownCommand {
                    command: "MYSTERY".to_string(),
                },
                codes::TQL_UNKNOWN_COMMAND,
            ),
            (
                TqlParseError::MissingArgument {
                    command: "DESCRIBE",
                    argument: "a key",
                },
                codes::TQL_MISSING_ARGUMENT,
            ),
            (
                TqlParseError::UnexpectedToken {
                    command: "STATUS",
                    token: "NOW".to_string(),
                },
                codes::TQL_UNEXPECTED_TOKEN,
            ),
            (
                TqlParseError::KeyTooLarge {
                    limit: 4,
                    actual: 5,
                },
                codes::TQL_KEY_TOO_LARGE,
            ),
            (
                TqlParseError::InvalidKey {
                    reason: "keys must not contain control characters",
                },
                codes::TQL_INVALID_KEY,
            ),
        ];

        for (error, expected_code) in cases {
            let report = super::CliError::Tql(error).error_report();
            assert_eq!(report.code, expected_code);
            assert_eq!(report.status, tosumu_core::error::ErrorStatus::InvalidInput);
        }
    }

    fn extract_marked_code_block<'a>(
        document: &'a str,
        start_marker: &str,
        end_marker: &str,
    ) -> Vec<&'a str> {
        let after_start = document
            .split_once(start_marker)
            .unwrap_or_else(|| panic!("missing start marker {start_marker}"))
            .1;
        let before_end = after_start
            .split_once(end_marker)
            .unwrap_or_else(|| panic!("missing end marker {end_marker}"))
            .0;
        let code_block = before_end
            .split_once("```txt")
            .unwrap_or_else(|| panic!("missing txt code block after {start_marker}"))
            .1
            .split_once("```")
            .unwrap_or_else(|| panic!("missing closing code fence before {end_marker}"))
            .0;

        code_block
            .lines()
            .map(str::trim)
            .filter(|line| !line.is_empty())
            .collect()
    }
}
