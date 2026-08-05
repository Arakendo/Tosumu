//! Incubating Tosumu Command Language (TQL) parser.
//!
//! This module owns only bounded syntax and inert command values. Dispatch,
//! storage facts, and terminal or JSON rendering remain in their owning layers.

use std::fmt;

/// Maximum UTF-8 command size accepted by the initial TQL grammar.
pub(crate) const MAX_COMMAND_BYTES: usize = 4 * 1024;
/// Maximum whitespace-delimited tokens accepted before parsing stops.
pub(crate) const MAX_TOKEN_COUNT: usize = 16;
/// Maximum UTF-8 byte length of a `DESCRIBE` key token.
pub(crate) const MAX_KEY_BYTES: usize = 1024;

/// A parsed, side-effect-free initial TQL command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TqlCommand {
    Status,
    Check,
    Describe { key: String },
    WalStatus,
}

/// Syntax failures reported before any database capability is consulted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TqlParseError {
    EmptyInput,
    InputTooLarge {
        limit: usize,
        actual: usize,
    },
    TooManyTokens {
        limit: usize,
    },
    UnknownCommand {
        command: String,
    },
    MissingArgument {
        command: &'static str,
        argument: &'static str,
    },
    UnexpectedToken {
        command: &'static str,
        token: String,
    },
    KeyTooLarge {
        limit: usize,
        actual: usize,
    },
    InvalidKey {
        reason: &'static str,
    },
}

impl fmt::Display for TqlParseError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyInput => write!(formatter, "TQL input is empty"),
            Self::InputTooLarge { limit, actual } => {
                write!(
                    formatter,
                    "TQL input is {actual} bytes; the limit is {limit}"
                )
            }
            Self::TooManyTokens { limit } => {
                write!(formatter, "TQL input has more than {limit} tokens")
            }
            Self::UnknownCommand { command } => {
                write!(formatter, "unknown TQL command `{command}`")
            }
            Self::MissingArgument { command, argument } => {
                write!(formatter, "TQL command `{command}` requires {argument}")
            }
            Self::UnexpectedToken { command, token } => {
                write!(
                    formatter,
                    "unexpected token `{token}` after TQL command `{command}`"
                )
            }
            Self::KeyTooLarge { limit, actual } => {
                write!(formatter, "TQL key is {actual} bytes; the limit is {limit}")
            }
            Self::InvalidKey { reason } => write!(formatter, "invalid TQL key: {reason}"),
        }
    }
}

impl std::error::Error for TqlParseError {}

/// Parses the initial read-only TQL grammar without opening a store.
///
/// Commands are ASCII case-insensitive. ASCII whitespace separates tokens;
/// leading and trailing ASCII whitespace is ignored. `DESCRIBE` keys are one
/// non-control UTF-8 token and are otherwise preserved exactly.
pub(crate) fn parse(input: &str) -> Result<TqlCommand, TqlParseError> {
    if input.len() > MAX_COMMAND_BYTES {
        return Err(TqlParseError::InputTooLarge {
            limit: MAX_COMMAND_BYTES,
            actual: input.len(),
        });
    }

    let tokens = tokenize(input)?;
    let Some(command) = tokens.first() else {
        return Err(TqlParseError::EmptyInput);
    };

    if command.eq_ignore_ascii_case("STATUS") {
        return parse_zero_argument("STATUS", &tokens, TqlCommand::Status);
    }
    if command.eq_ignore_ascii_case("CHECK") {
        return parse_zero_argument("CHECK", &tokens, TqlCommand::Check);
    }
    if command.eq_ignore_ascii_case("DESCRIBE") {
        return parse_describe(&tokens);
    }
    if command.eq_ignore_ascii_case("WAL") {
        return parse_wal(&tokens);
    }

    Err(TqlParseError::UnknownCommand {
        command: (*command).to_owned(),
    })
}

fn parse_wal(tokens: &[&str]) -> Result<TqlCommand, TqlParseError> {
    let Some(action) = tokens.get(1) else {
        return Err(TqlParseError::MissingArgument {
            command: "WAL",
            argument: "STATUS",
        });
    };
    if !action.eq_ignore_ascii_case("STATUS") {
        return Err(TqlParseError::UnexpectedToken {
            command: "WAL",
            token: (*action).to_owned(),
        });
    }
    if let Some(token) = tokens.get(2) {
        return Err(TqlParseError::UnexpectedToken {
            command: "WAL STATUS",
            token: (*token).to_owned(),
        });
    }
    Ok(TqlCommand::WalStatus)
}

fn tokenize(input: &str) -> Result<Vec<&str>, TqlParseError> {
    let tokens = input.split_ascii_whitespace().collect::<Vec<_>>();
    if tokens.len() > MAX_TOKEN_COUNT {
        return Err(TqlParseError::TooManyTokens {
            limit: MAX_TOKEN_COUNT,
        });
    }
    Ok(tokens)
}

fn parse_zero_argument(
    command: &'static str,
    tokens: &[&str],
    parsed: TqlCommand,
) -> Result<TqlCommand, TqlParseError> {
    if let Some(token) = tokens.get(1) {
        return Err(TqlParseError::UnexpectedToken {
            command,
            token: (*token).to_owned(),
        });
    }
    Ok(parsed)
}

fn parse_describe(tokens: &[&str]) -> Result<TqlCommand, TqlParseError> {
    let Some(key) = tokens.get(1) else {
        return Err(TqlParseError::MissingArgument {
            command: "DESCRIBE",
            argument: "a key",
        });
    };
    if let Some(token) = tokens.get(2) {
        return Err(TqlParseError::UnexpectedToken {
            command: "DESCRIBE",
            token: (*token).to_owned(),
        });
    }
    if key.len() > MAX_KEY_BYTES {
        return Err(TqlParseError::KeyTooLarge {
            limit: MAX_KEY_BYTES,
            actual: key.len(),
        });
    }
    if key.chars().any(char::is_control) {
        return Err(TqlParseError::InvalidKey {
            reason: "keys must not contain control characters",
        });
    }
    Ok(TqlCommand::Describe {
        key: (*key).to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use proptest::prelude::*;

    use super::*;

    struct Case {
        input: String,
        expected: Result<TqlCommand, TqlParseError>,
    }

    #[test]
    fn parser_corpus_is_deterministic_and_side_effect_free() {
        let cases = vec![
            Case {
                input: "STATUS".into(),
                expected: Ok(TqlCommand::Status),
            },
            Case {
                input: " \tstatus\r\n".into(),
                expected: Ok(TqlCommand::Status),
            },
            Case {
                input: "CHECK".into(),
                expected: Ok(TqlCommand::Check),
            },
            Case {
                input: "describe player/42".into(),
                expected: Ok(TqlCommand::Describe {
                    key: "player/42".into(),
                }),
            },
            Case {
                input: "wal status".into(),
                expected: Ok(TqlCommand::WalStatus),
            },
            Case {
                input: "DESCRIBE assets/uber".into(),
                expected: Ok(TqlCommand::Describe {
                    key: "assets/uber".into(),
                }),
            },
            Case {
                input: String::new(),
                expected: Err(TqlParseError::EmptyInput),
            },
            Case {
                input: "DESCRIBE".into(),
                expected: Err(TqlParseError::MissingArgument {
                    command: "DESCRIBE",
                    argument: "a key",
                }),
            },
            Case {
                input: "WAL".into(),
                expected: Err(TqlParseError::MissingArgument {
                    command: "WAL",
                    argument: "STATUS",
                }),
            },
            Case {
                input: "STATUS NOW".into(),
                expected: Err(TqlParseError::UnexpectedToken {
                    command: "STATUS",
                    token: "NOW".into(),
                }),
            },
            Case {
                input: "DESCRIBE player/42 EXTRA".into(),
                expected: Err(TqlParseError::UnexpectedToken {
                    command: "DESCRIBE",
                    token: "EXTRA".into(),
                }),
            },
            Case {
                input: "STATUS; CHECK".into(),
                expected: Err(TqlParseError::UnknownCommand {
                    command: "STATUS;".into(),
                }),
            },
        ];

        for case in cases {
            assert_eq!(parse(&case.input), case.expected, "input: {:?}", case.input);
            assert_eq!(
                parse(&case.input),
                parse(&case.input),
                "input: {:?}",
                case.input
            );
        }
    }

    #[test]
    fn parser_enforces_declared_resource_limits() {
        let oversized = "S".repeat(MAX_COMMAND_BYTES + 1);
        assert_eq!(
            parse(&oversized),
            Err(TqlParseError::InputTooLarge {
                limit: MAX_COMMAND_BYTES,
                actual: MAX_COMMAND_BYTES + 1,
            })
        );

        let too_many_tokens = std::iter::repeat("x")
            .take(MAX_TOKEN_COUNT + 1)
            .collect::<Vec<_>>()
            .join(" ");
        assert_eq!(
            parse(&too_many_tokens),
            Err(TqlParseError::TooManyTokens {
                limit: MAX_TOKEN_COUNT,
            })
        );

        let oversized_key = format!("DESCRIBE {}", "k".repeat(MAX_KEY_BYTES + 1));
        assert_eq!(
            parse(&oversized_key),
            Err(TqlParseError::KeyTooLarge {
                limit: MAX_KEY_BYTES,
                actual: MAX_KEY_BYTES + 1,
            })
        );
    }

    proptest! {
        #[test]
        fn parser_is_deterministic_and_never_panics_for_arbitrary_utf8(input in any::<String>()) {
            let first = parse(&input);
            let second = parse(&input);
            prop_assert_eq!(first, second);
        }
    }
}
