use super::*;
use crate::commands::inspect::{
    cmd_inspect_header_json,
    cmd_inspect_page_json,
    cmd_inspect_pages_json,
    cmd_inspect_protectors_json,
    cmd_inspect_tree_json,
    cmd_inspect_verify_json,
    cmd_inspect_wal_json,
};
use crate::unlock::UnlockSecret;

#[test]
fn inspect_header_json_uses_structured_success_envelope() {
    let path = temp_path("inspect_header_json_success");
    let _ = std::fs::remove_file(&path);
    tosumu_core::page_store::PageStore::create(&path).unwrap();

    let rendered = cmd_inspect_header_json(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "inspect.header");
    assert_eq!(json["ok"], true);
    assert_eq!(json["payload"]["page_size"], 4096);
    assert_eq!(json["payload"]["slot0"]["kind"], "Sentinel");
    assert_eq!(json["payload"]["slot0"]["kind_byte"], 1);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_error_json_uses_structured_error_envelope() {
    let rendered = render_inspect_error_json(
        "inspect.header",
        &TosumuError::InvalidArgument("page number out of range"),
    );
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "inspect.header");
    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["kind"], "invalid_argument");
    assert_eq!(json["error"]["message"], "invalid argument: page number out of range");
    assert!(json["payload"].is_null());
}

#[test]
fn inspect_verify_json_uses_structured_success_envelope() {
    let path = temp_path("inspect_verify_json_success");
    let _ = std::fs::remove_file(&path);
    tosumu_core::page_store::PageStore::create(&path).unwrap();

    let rendered = cmd_inspect_verify_json(&path, None, false).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "inspect.verify");
    assert_eq!(json["ok"], true);
    assert_eq!(json["payload"]["issues"].as_array().unwrap().len(), 0);
    assert_eq!(json["payload"]["btree"]["checked"], true);
    assert_eq!(json["payload"]["btree"]["ok"], true);
    assert!(json["payload"]["btree"]["message"].is_null());

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_page_json_uses_structured_success_envelope() {
    let path = temp_path("inspect_page_json_success");
    let _ = std::fs::remove_file(&path);
    let mut store = tosumu_core::page_store::PageStore::create(&path).unwrap();
    store.put(b"alpha", b"one").unwrap();

    let rendered = cmd_inspect_page_json(&path, 1, None, false).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "inspect.page");
    assert_eq!(json["ok"], true);
    assert_eq!(json["payload"]["pgno"], 1);
    assert_eq!(json["payload"]["page_type_name"], "Leaf");
    assert!(json["payload"]["records"]
        .as_array()
        .unwrap()
        .iter()
        .any(|record| record["kind"] == "Live"
            && record["key_hex"] == "616c706861"
            && record["value_hex"] == "6f6e65"));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_pages_json_uses_structured_success_envelope() {
    let path = temp_path("inspect_pages_json_success");
    let _ = std::fs::remove_file(&path);
    let mut store = tosumu_core::page_store::PageStore::create(&path).unwrap();
    store.put(b"alpha", b"one").unwrap();

    let rendered = cmd_inspect_pages_json(&path, None, false).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "inspect.pages");
    assert_eq!(json["ok"], true);
    assert!(json["payload"]["pages"].as_array().unwrap().len() >= 1);
    assert_eq!(json["payload"]["pages"][0]["pgno"], 1);
    assert_eq!(json["payload"]["pages"][0]["page_type_name"], "Leaf");
    assert_eq!(json["payload"]["pages"][0]["state"], "ok");

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_wal_json_uses_structured_success_envelope() {
    let path = temp_path("inspect_wal_json_success");
    let wal_path = tosumu_core::wal::wal_path(&path);
    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal_path);
    tosumu_core::page_store::PageStore::create(&path).unwrap();
    let _ = std::fs::remove_file(&wal_path);

    {
        let mut writer = tosumu_core::wal::WalWriter::create(&wal_path).unwrap();
        writer.append(&tosumu_core::wal::WalRecord::Begin { txn_id: 9 }).unwrap();
        writer.append(&tosumu_core::wal::WalRecord::PageWrite {
            pgno: 1,
            page_version: 7,
            frame: Box::new([0u8; tosumu_core::format::PAGE_SIZE]),
        }).unwrap();
        writer.append(&tosumu_core::wal::WalRecord::Commit { txn_id: 9 }).unwrap();
        writer.sync().unwrap();
    }

    let rendered = cmd_inspect_wal_json(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "inspect.wal");
    assert_eq!(json["ok"], true);
    assert_eq!(json["payload"]["wal_exists"], true);
    assert_eq!(json["payload"]["record_count"], 3);
    assert_eq!(json["payload"]["records"][0]["kind"], "begin");
    assert_eq!(json["payload"]["records"][0]["txn_id"], 9);
    assert_eq!(json["payload"]["records"][1]["kind"], "page_write");
    assert_eq!(json["payload"]["records"][1]["pgno"], 1);
    assert_eq!(json["payload"]["records"][1]["page_version"], 7);
    assert_eq!(json["payload"]["records"][2]["kind"], "commit");
    assert_eq!(json["payload"]["records"][2]["txn_id"], 9);

    let _ = std::fs::remove_file(&path);
    let _ = std::fs::remove_file(&wal_path);
}

#[test]
fn inspect_tree_json_uses_structured_success_envelope() {
    let path = temp_path("inspect_tree_json_success");
    let _ = std::fs::remove_file(&path);

    let mut store = tosumu_core::page_store::PageStore::create(&path).unwrap();
    for i in 0u32..500 {
        store.put(
            format!("tree-key-{i:05}").as_bytes(),
            format!("tree-val-{i:05}").as_bytes(),
        ).unwrap();
    }
    assert!(
        store.stat().unwrap().tree_height >= 2,
        "expected test fixture to force a root split"
    );

    let rendered = cmd_inspect_tree_json(&path, None, false).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "inspect.tree");
    assert_eq!(json["ok"], true);
    assert_eq!(json["payload"]["root"]["page_type_name"], "Internal");
    assert!(json["payload"]["root"]["children"].as_array().unwrap().len() >= 2);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_protectors_json_uses_structured_success_envelope() {
    let path = temp_path("inspect_protectors_json_success");
    let _ = std::fs::remove_file(&path);
    tosumu_core::page_store::PageStore::create(&path).unwrap();

    let rendered = cmd_inspect_protectors_json(&path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["command"], "inspect.protectors");
    assert_eq!(json["ok"], true);
    assert_eq!(json["payload"]["slot_count"], 0);
    assert_eq!(json["payload"]["slots"].as_array().unwrap().len(), 0);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_verify_json_accepts_explicit_passphrase_unlock() {
    let path = temp_path("inspect_verify_json_passphrase_unlock");
    let _ = std::fs::remove_file(&path);
    tosumu_core::page_store::PageStore::create_encrypted(&path, "correct-horse").unwrap();

    let rendered = cmd_inspect_verify_json(&path, Some(UnlockSecret::Passphrase("correct-horse".to_string())), false).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(json["ok"], true);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_page_json_accepts_explicit_passphrase_unlock() {
    let path = temp_path("inspect_page_json_passphrase_unlock");
    let _ = std::fs::remove_file(&path);
    let mut store = tosumu_core::page_store::PageStore::create_encrypted(&path, "correct-horse").unwrap();
    store.put(b"alpha", b"one").unwrap();

    let rendered = cmd_inspect_page_json(&path, 1, Some(UnlockSecret::Passphrase("correct-horse".to_string())), false).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(json["ok"], true);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_pages_json_accepts_explicit_passphrase_unlock() {
    let path = temp_path("inspect_pages_json_passphrase_unlock");
    let _ = std::fs::remove_file(&path);
    let mut store = tosumu_core::page_store::PageStore::create_encrypted(&path, "correct-horse").unwrap();
    store.put(b"alpha", b"one").unwrap();

    let rendered = cmd_inspect_pages_json(&path, Some(UnlockSecret::Passphrase("correct-horse".to_string())), false).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(json["ok"], true);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_tree_json_accepts_explicit_passphrase_unlock() {
    let path = temp_path("inspect_tree_json_passphrase_unlock");
    let _ = std::fs::remove_file(&path);

    let mut store = tosumu_core::page_store::PageStore::create_encrypted(&path, "correct-horse").unwrap();
    for i in 0u32..500 {
        store.put(
            format!("tree-key-{i:05}").as_bytes(),
            format!("tree-val-{i:05}").as_bytes(),
        ).unwrap();
    }

    let rendered = cmd_inspect_tree_json(&path, Some(UnlockSecret::Passphrase("correct-horse".to_string())), false).unwrap();
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();
    assert_eq!(json["ok"], true);

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_verify_json_no_prompt_returns_wrong_key_for_encrypted_db() {
    let path = temp_path("inspect_verify_json_no_prompt_wrong_key");
    let _ = std::fs::remove_file(&path);
    tosumu_core::page_store::PageStore::create_encrypted(&path, "correct-horse").unwrap();

    let err = cmd_inspect_verify_json(&path, None, true).err().unwrap();
    assert!(matches!(err, TosumuError::WrongKey));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_tree_json_no_prompt_returns_wrong_key_for_encrypted_db() {
    let path = temp_path("inspect_tree_json_no_prompt_wrong_key");
    let _ = std::fs::remove_file(&path);
    tosumu_core::page_store::PageStore::create_encrypted(&path, "correct-horse").unwrap();

    let err = cmd_inspect_tree_json(&path, None, true).err().unwrap();
    assert!(matches!(err, TosumuError::WrongKey));

    let _ = std::fs::remove_file(&path);
}

#[test]
fn inspect_tree_wrong_key_uses_structured_error_envelope() {
    let path = temp_path("inspect_tree_json_wrong_key_envelope");
    let _ = std::fs::remove_file(&path);
    tosumu_core::page_store::PageStore::create_encrypted(&path, "correct-horse").unwrap();

    let err = cmd_inspect_tree_json(&path, None, true).err().unwrap();
    let rendered = render_inspect_error_json("inspect.tree", &err);
    let json: serde_json::Value = serde_json::from_str(&rendered).unwrap();

    assert_eq!(json["ok"], false);
    assert_eq!(json["error"]["kind"], "wrong_key");
    assert_eq!(
        json["error"]["message"],
        "wrong passphrase or key — could not unlock any keyslot"
    );

    let _ = std::fs::remove_file(&path);
}

fn temp_path(tag: &str) -> PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("tosumu_cli_{tag}_{nanos}.tsm"))
}