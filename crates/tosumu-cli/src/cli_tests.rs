use super::*;
use crate::commands::protector::{
    confirm_recovery_words,
    format_recovery_key_for_display,
    recovery_words,
};

#[test]
fn recovery_words_rechunk_into_eight_groups_of_four() {
    let secret = "ABCDEFGH-IJKLMNOP-QRSTUVWX-YZ234567";
    let words = recovery_words(secret);
    assert_eq!(words, vec![
        "ABCD", "EFGH", "IJKL", "MNOP", "QRST", "UVWX", "YZ23", "4567",
    ]);
}

#[test]
fn recovery_display_uses_eight_groups_of_four() {
    let secret = "ABCDEFGH-IJKLMNOP-QRSTUVWX-YZ234567";
    assert_eq!(
        format_recovery_key_for_display(secret),
        "ABCD-EFGH-IJKL-MNOP-QRST-UVWX-YZ23-4567"
    );
}

#[test]
fn recovery_confirmation_accepts_correct_words() {
    let secret = "ABCDEFGH-IJKLMNOP-QRSTUVWX-YZ234567";
    confirm_recovery_words(secret, "ijkl", "yz23").unwrap();
}

#[test]
fn recovery_confirmation_rejects_wrong_words() {
    let secret = "ABCDEFGH-IJKLMNOP-QRSTUVWX-YZ234567";
    let err = confirm_recovery_words(secret, "WRONG", "YZ23").unwrap_err();
    assert!(matches!(err, TosumuError::InvalidArgument("recovery key confirmation failed")));
}

#[test]
fn cli_parses_add_keyfile_subcommand() {
    let cli = Cli::try_parse_from([
        "tosumu",
        "protector",
        "add-keyfile",
        "db.tsm",
        "db.key",
    ]).unwrap();

    match cli.command {
        Command::Protector { action: ProtectorAction::AddKeyfile { path, keyfile } } => {
            assert_eq!(path, PathBuf::from("db.tsm"));
            assert_eq!(keyfile, PathBuf::from("db.key"));
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn cli_parses_add_recovery_key_subcommand() {
    let cli = Cli::try_parse_from([
        "tosumu",
        "protector",
        "add-recovery-key",
        "db.tsm",
    ]).unwrap();

    match cli.command {
        Command::Protector { action: ProtectorAction::AddRecoveryKey { path } } => {
            assert_eq!(path, PathBuf::from("db.tsm"));
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn cli_parses_inspect_header_json_subcommand() {
    let cli = Cli::try_parse_from([
        "tosumu",
        "inspect",
        "header",
        "--json",
        "db.tsm",
    ]).unwrap();

    match cli.command {
        Command::Inspect {
            action: InspectAction::Header { path, json },
        } => {
            assert_eq!(path, PathBuf::from("db.tsm"));
            assert!(json);
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn cli_parses_inspect_verify_json_subcommand() {
    let cli = Cli::try_parse_from([
        "tosumu",
        "inspect",
        "verify",
        "--json",
        "db.tsm",
    ]).unwrap();

    match cli.command {
        Command::Inspect {
            action: InspectAction::Verify { path, json, unlock },
        } => {
            assert_eq!(path, PathBuf::from("db.tsm"));
            assert!(json);
            assert!(!unlock.no_prompt);
            assert!(!unlock.stdin_passphrase);
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn cli_parses_inspect_pages_json_subcommand() {
    let cli = Cli::try_parse_from([
        "tosumu",
        "inspect",
        "pages",
        "--json",
        "db.tsm",
    ]).unwrap();

    match cli.command {
        Command::Inspect {
            action: InspectAction::Pages { path, json, unlock },
        } => {
            assert_eq!(path, PathBuf::from("db.tsm"));
            assert!(json);
            assert!(!unlock.no_prompt);
            assert!(!unlock.stdin_passphrase);
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn cli_parses_inspect_wal_json_subcommand() {
    let cli = Cli::try_parse_from([
        "tosumu",
        "inspect",
        "wal",
        "--json",
        "db.tsm",
    ]).unwrap();

    match cli.command {
        Command::Inspect {
            action: InspectAction::Wal { path, json },
        } => {
            assert_eq!(path, PathBuf::from("db.tsm"));
            assert!(json);
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn cli_parses_inspect_page_json_subcommand() {
    let cli = Cli::try_parse_from([
        "tosumu",
        "inspect",
        "page",
        "--page",
        "1",
        "--json",
        "db.tsm",
    ]).unwrap();

    match cli.command {
        Command::Inspect {
            action: InspectAction::Page { path, page, json, unlock },
        } => {
            assert_eq!(path, PathBuf::from("db.tsm"));
            assert_eq!(page, 1);
            assert!(json);
            assert!(!unlock.no_prompt);
            assert!(!unlock.stdin_passphrase);
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn cli_parses_inspect_protectors_json_subcommand() {
    let cli = Cli::try_parse_from([
        "tosumu",
        "inspect",
        "protectors",
        "--json",
        "db.tsm",
    ]).unwrap();

    match cli.command {
        Command::Inspect {
            action: InspectAction::Protectors { path, json },
        } => {
            assert_eq!(path, PathBuf::from("db.tsm"));
            assert!(json);
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn cli_parses_inspect_verify_with_stdin_passphrase() {
    let cli = Cli::try_parse_from([
        "tosumu",
        "inspect",
        "verify",
        "--json",
        "--stdin-passphrase",
        "db.tsm",
    ]).unwrap();

    match cli.command {
        Command::Inspect {
            action: InspectAction::Verify { path, json, unlock },
        } => {
            assert_eq!(path, PathBuf::from("db.tsm"));
            assert!(json);
            assert!(unlock.stdin_passphrase);
            assert!(!unlock.no_prompt);
            assert!(!unlock.stdin_recovery_key);
            assert!(unlock.keyfile.is_none());
        }
        _ => panic!("unexpected command variant"),
    }
}

#[test]
fn cli_parses_inspect_verify_with_no_prompt() {
    let cli = Cli::try_parse_from([
        "tosumu",
        "inspect",
        "verify",
        "--json",
        "--no-prompt",
        "db.tsm",
    ]).unwrap();

    match cli.command {
        Command::Inspect {
            action: InspectAction::Verify { unlock, .. },
        } => {
            assert!(unlock.no_prompt);
            assert!(!unlock.stdin_passphrase);
            assert!(!unlock.stdin_recovery_key);
            assert!(unlock.keyfile.is_none());
        }
        _ => panic!("unexpected command variant"),
    }
}
