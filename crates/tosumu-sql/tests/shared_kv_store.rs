use tosumu_core::SharedKvStore;
use tosumu_sql::row_codec::{decode_row_values, encode_row_values, row_key};
use tosumu_sql::Value;

fn encoded_name(name: &str) -> Vec<u8> {
    encode_row_values(&["name"], &[2], &[Value::Text(name.to_string())]).unwrap()
}

fn decoded_name(bytes: &[u8]) -> String {
    match decode_row_values(bytes).unwrap().as_slice() {
        [Value::Text(name)] => name.clone(),
        values => panic!("expected one text column, got {values:?}"),
    }
}

#[test]
fn sql_row_encoding_consumes_a_coherent_shared_snapshot() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("sql-shared-kv-store.tsm");
    let database = SharedKvStore::create(&path).unwrap();
    let first = row_key("users", &Value::Integer(1));
    let second = row_key("users", &Value::Integer(2));
    let third = row_key("users", &Value::Integer(3));

    database
        .write(|transaction| {
            transaction.put(first.as_bytes(), &encoded_name("alice"))?;
            transaction.put(second.as_bytes(), &encoded_name("bob"))?;
            Ok(())
        })
        .unwrap();
    let snapshot = database.snapshot().unwrap();

    database
        .write(|transaction| {
            transaction.put(first.as_bytes(), &encoded_name("alice-new"))?;
            transaction.delete(second.as_bytes())?;
            transaction.put(third.as_bytes(), &encoded_name("cara"))?;
            Ok(())
        })
        .unwrap();

    let captured = snapshot
        .scan(
            row_key("users", &Value::Integer(0)).as_bytes(),
            row_key("users", &Value::Integer(9)).as_bytes(),
        )
        .unwrap();
    assert_eq!(captured.len(), 2);
    assert_eq!(decoded_name(&captured[0].1), "alice");
    assert_eq!(decoded_name(&captured[1].1), "bob");

    assert_eq!(
        decoded_name(&database.get(first.as_bytes()).unwrap().unwrap()),
        "alice-new"
    );
    assert_eq!(database.get(second.as_bytes()).unwrap(), None);
    assert_eq!(
        decoded_name(&database.get(third.as_bytes()).unwrap().unwrap()),
        "cara"
    );
}
