use super::*;
use crate::test_helpers::{CrashFile, CrashPhase};
use std::path::PathBuf;

fn tmp(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tosumu_wal_{name}_{}.wal", std::process::id()))
}

fn tmp_db(name: &str) -> PathBuf {
    std::env::temp_dir().join(format!("tosumu_wal_{name}_{}.tsm", std::process::id()))
}

mod crash_preservation;
mod locking;
mod record_io;
mod recovery;
