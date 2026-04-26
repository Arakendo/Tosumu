use tosumu_core::error::TosumuError;

use super::inspect::{
    cmd_inspect_header_json,
    cmd_inspect_page_json,
    cmd_inspect_pages_json,
    cmd_inspect_protectors_json,
    cmd_inspect_tree_json,
    cmd_inspect_verify_json,
    cmd_inspect_wal_json,
};
use super::protector::{run_protector_action, run_rekey_kek};
use super::store::{run_delete, run_get, run_init, run_put, run_scan, run_stat};
use super::text::{cmd_backup, cmd_dump, cmd_hex, cmd_verify};
use crate::unlock::UnlockSecret;
use crate::{Cli, Command, InspectAction, InspectUnlockArgs};

pub(crate) fn run(cli: Cli) -> Result<(), TosumuError> {
    match cli.command {
        Command::Init { path, encrypt } => run_init(&path, encrypt)?,
        Command::Put { path, key, value } => run_put(&path, &key, &value)?,
        Command::Get { path, key } => run_get(&path, &key)?,
        Command::Delete { path, key } => run_delete(&path, &key)?,
        Command::Scan { path } => run_scan(&path)?,
        Command::Stat { path } => run_stat(&path)?,
        Command::Dump { path, page } => cmd_dump(&path, page, None, false)?,
        Command::Hex { path, page } => cmd_hex(&path, page)?,
        Command::Verify { path, explain } => cmd_verify(&path, explain, None, false)?,
        Command::View { path } => crate::view::run(&path)?,
        Command::Inspect { action } => run_inspect_action(action)?,
        Command::Backup { src, dest } => cmd_backup(&src, &dest)?,
        Command::Protector { action } => run_protector_action(action)?,
        Command::RekeyKek { path, slot } => run_rekey_kek(&path, slot)?,
    }
    Ok(())
}

fn run_inspect_action(action: InspectAction) -> Result<(), TosumuError> {
    match action {
        InspectAction::Header { path, json } => {
            if json {
                println!("{}", cmd_inspect_header_json(&path)?);
            } else {
                cmd_dump(&path, None, None, false)?;
            }
        }
        InspectAction::Verify { path, json, unlock } => {
            let (unlock, no_prompt) = resolve_inspect_unlock(unlock)?;
            if json {
                println!("{}", cmd_inspect_verify_json(&path, unlock, no_prompt)?);
            } else {
                cmd_verify(&path, false, unlock, no_prompt)?;
            }
        }
        InspectAction::Pages { path, unlock, .. } => {
            let (unlock, no_prompt) = resolve_inspect_unlock(unlock)?;
            let pages_json = cmd_inspect_pages_json(&path, unlock, no_prompt)?;
            println!("{pages_json}");
        }
        InspectAction::Page { path, page, json, unlock } => {
            let (unlock, no_prompt) = resolve_inspect_unlock(unlock)?;
            if json {
                println!("{}", cmd_inspect_page_json(&path, page, unlock, no_prompt)?);
            } else {
                cmd_dump(&path, Some(page), unlock, no_prompt)?;
            }
        }
        InspectAction::Wal { path, .. } => {
            let wal_json = cmd_inspect_wal_json(&path)?;
            println!("{wal_json}");
        }
        InspectAction::Tree { path, unlock, .. } => {
            let (unlock, no_prompt) = resolve_inspect_unlock(unlock)?;
            let tree_json = cmd_inspect_tree_json(&path, unlock, no_prompt)?;
            println!("{tree_json}");
        }
        InspectAction::Protectors { path, json } => {
            if json {
                println!("{}", cmd_inspect_protectors_json(&path)?);
            } else {
                cmd_dump(&path, None, None, false)?;
            }
        }
    }

    Ok(())
}

fn read_secret_from_stdin(empty_message: &'static str) -> Result<String, TosumuError> {
    use std::io::Read as _;

    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).map_err(TosumuError::Io)?;
    let secret = input.trim_end_matches(&['\r', '\n'][..]).to_string();
    if secret.is_empty() {
        return Err(TosumuError::InvalidArgument(empty_message));
    }
    Ok(secret)
}

fn resolve_inspect_unlock(unlock: InspectUnlockArgs) -> Result<(Option<UnlockSecret>, bool), TosumuError> {
    let no_prompt = unlock.no_prompt;

    if unlock.stdin_passphrase {
        return Ok((Some(UnlockSecret::Passphrase(read_secret_from_stdin(
            "stdin passphrase must not be empty",
        )?)), no_prompt));
    }

    if unlock.stdin_recovery_key {
        return Ok((Some(UnlockSecret::RecoveryKey(read_secret_from_stdin(
            "stdin recovery key must not be empty",
        )?)), no_prompt));
    }

    if let Some(keyfile) = unlock.keyfile {
        return Ok((Some(UnlockSecret::Keyfile(keyfile)), no_prompt));
    }

    Ok((None, no_prompt))
}
