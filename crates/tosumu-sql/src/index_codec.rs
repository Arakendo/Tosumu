//! Logical secondary-index key encoding owned by the SQL layer.
//!
//! ADR-0008 keeps these entries in a reserved ordered keyspace in the shared
//! KV tree. The encoding is versioned and prefix-free so one secondary value
//! can be scanned without admitting adjacent values that share its bytes.

use crate::ast::Value;
use crate::error::{SqlError, SqlResult};
use tosumu_core::MAX_KEY_SIZE;

/// Reserved key prefix for secondary-index entries.
pub const INDEX_ENTRY_PREFIX: &[u8] = b"__sql_index__/entry/";

const INDEX_ENTRY_VERSION: u8 = 1;
const INTEGER_TAG: u8 = 1;
const TEXT_TAG: u8 = 2;
const BLOB_TAG: u8 = 3;

/// Build one secondary-index entry key.
pub(crate) fn index_entry_key(
    table: &str,
    index: &str,
    secondary: &Value,
    primary: &Value,
) -> SqlResult<Vec<u8>> {
    let mut key = index_value_prefix(table, index, secondary);
    encode_value(&mut key, primary);
    validate_key_size(key)
}

/// Return inclusive bounds covering exactly one indexed secondary value.
pub(crate) fn index_entry_bounds(
    table: &str,
    index: &str,
    secondary: &Value,
) -> SqlResult<(Vec<u8>, Vec<u8>)> {
    let prefix = index_value_prefix(table, index, secondary);
    let start = validate_key_size(prefix.clone())?;
    let mut end = prefix;
    end.push(u8::MAX);
    Ok((start, validate_key_size(end)?))
}

/// Decode and validate one complete index entry.
pub(crate) fn decode_index_entry(key: &[u8]) -> SqlResult<(String, String, Value, Value)> {
    let mut cursor = Cursor::new(key);
    cursor.expect_bytes(INDEX_ENTRY_PREFIX, "secondary-index prefix")?;
    let version = cursor.byte("secondary-index version")?;
    if version != INDEX_ENTRY_VERSION {
        return Err(codec_error(format!(
            "unsupported secondary-index key version: expected {INDEX_ENTRY_VERSION}, got {version}"
        )));
    }

    let table = cursor.escaped_string("table name")?;
    let index = cursor.escaped_string("index name")?;
    let secondary = cursor.value("secondary value")?;
    let primary = cursor.value("primary value")?;
    if !cursor.is_finished() {
        return Err(codec_error("trailing bytes in secondary-index key"));
    }
    Ok((table, index, secondary, primary))
}

fn index_value_prefix(table: &str, index: &str, secondary: &Value) -> Vec<u8> {
    let mut key = Vec::with_capacity(
        INDEX_ENTRY_PREFIX.len() + table.len() + index.len() + value_size_hint(secondary) + 8,
    );
    key.extend_from_slice(INDEX_ENTRY_PREFIX);
    key.push(INDEX_ENTRY_VERSION);
    encode_escaped(&mut key, table.as_bytes());
    encode_escaped(&mut key, index.as_bytes());
    encode_value(&mut key, secondary);
    key
}

fn encode_value(output: &mut Vec<u8>, value: &Value) {
    match value {
        Value::Integer(number) => {
            output.push(INTEGER_TAG);
            let mut ordered = number.to_be_bytes();
            ordered[0] ^= 0x80;
            output.extend_from_slice(&ordered);
        }
        Value::Text(text) => {
            output.push(TEXT_TAG);
            encode_escaped(output, text.as_bytes());
        }
        Value::Blob(bytes) => {
            output.push(BLOB_TAG);
            encode_escaped(output, bytes);
        }
    }
}

fn encode_escaped(output: &mut Vec<u8>, bytes: &[u8]) {
    for &byte in bytes {
        if byte == 0 {
            output.extend_from_slice(&[0, u8::MAX]);
        } else {
            output.push(byte);
        }
    }
    output.extend_from_slice(&[0, 0]);
}

fn value_size_hint(value: &Value) -> usize {
    match value {
        Value::Integer(_) => 9,
        Value::Text(text) => text.len() + 3,
        Value::Blob(bytes) => bytes.len() + 3,
    }
}

fn validate_key_size(key: Vec<u8>) -> SqlResult<Vec<u8>> {
    if key.len() > MAX_KEY_SIZE {
        Err(codec_error(format!(
            "secondary-index key is {} bytes; maximum is {MAX_KEY_SIZE}",
            key.len()
        )))
    } else {
        Ok(key)
    }
}

fn codec_error(message: impl Into<String>) -> SqlError {
    SqlError::RowEncoding(message.into())
}

struct Cursor<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> Cursor<'a> {
    fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, position: 0 }
    }

    fn is_finished(&self) -> bool {
        self.position == self.bytes.len()
    }

    fn expect_bytes(&mut self, expected: &[u8], field: &str) -> SqlResult<()> {
        let end = self
            .position
            .checked_add(expected.len())
            .ok_or_else(|| codec_error(format!("invalid {field}")))?;
        if self.bytes.get(self.position..end) != Some(expected) {
            return Err(codec_error(format!("invalid {field}")));
        }
        self.position = end;
        Ok(())
    }

    fn byte(&mut self, field: &str) -> SqlResult<u8> {
        let byte = self
            .bytes
            .get(self.position)
            .copied()
            .ok_or_else(|| codec_error(format!("truncated {field}")))?;
        self.position += 1;
        Ok(byte)
    }

    fn escaped_string(&mut self, field: &str) -> SqlResult<String> {
        let bytes = self.escaped_bytes(field)?;
        String::from_utf8(bytes).map_err(|error| codec_error(format!("invalid {field}: {error}")))
    }

    fn escaped_bytes(&mut self, field: &str) -> SqlResult<Vec<u8>> {
        let mut decoded = Vec::new();
        loop {
            let byte = self.byte(field)?;
            if byte != 0 {
                decoded.push(byte);
                continue;
            }

            match self.byte(field)? {
                0 => return Ok(decoded),
                u8::MAX => decoded.push(0),
                escape => {
                    return Err(codec_error(format!(
                        "invalid escape 0x{escape:02x} in {field}"
                    )))
                }
            }
        }
    }

    fn value(&mut self, field: &str) -> SqlResult<Value> {
        match self.byte(field)? {
            INTEGER_TAG => {
                let end = self
                    .position
                    .checked_add(8)
                    .ok_or_else(|| codec_error(format!("invalid {field}")))?;
                let bytes: [u8; 8] = self
                    .bytes
                    .get(self.position..end)
                    .ok_or_else(|| codec_error(format!("truncated INTEGER {field}")))?
                    .try_into()
                    .map_err(|_| codec_error(format!("invalid INTEGER {field}")))?;
                self.position = end;
                let mut signed = bytes;
                signed[0] ^= 0x80;
                Ok(Value::Integer(i64::from_be_bytes(signed)))
            }
            TEXT_TAG => Ok(Value::Text(self.escaped_string(field)?)),
            BLOB_TAG => Ok(Value::Blob(self.escaped_bytes(field)?)),
            tag => Err(codec_error(format!(
                "unsupported type tag {tag} in {field}"
            ))),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    #[test]
    fn entry_round_trips_all_value_types_and_embedded_zeroes() {
        let cases = [
            (Value::Integer(i64::MIN), Value::Integer(i64::MAX)),
            (
                Value::Text("a\0b".to_string()),
                Value::Text("pk\0tail".to_string()),
            ),
            (Value::Blob(vec![0, 1, 0, 255]), Value::Blob(vec![0])),
        ];

        for (secondary, primary) in cases {
            let key = index_entry_key("users", "by_value", &secondary, &primary).unwrap();
            assert_eq!(
                decode_index_entry(&key).unwrap(),
                (
                    "users".to_string(),
                    "by_value".to_string(),
                    secondary,
                    primary
                )
            );
        }
    }

    #[test]
    fn exact_value_bounds_include_duplicates_but_not_prefix_neighbors() {
        let (start, end) =
            index_entry_bounds("users", "by_name", &Value::Text("ann".to_string())).unwrap();
        let duplicate_a = index_entry_key(
            "users",
            "by_name",
            &Value::Text("ann".to_string()),
            &Value::Integer(1),
        )
        .unwrap();
        let duplicate_b = index_entry_key(
            "users",
            "by_name",
            &Value::Text("ann".to_string()),
            &Value::Integer(2),
        )
        .unwrap();
        let prefix_neighbor = index_entry_key(
            "users",
            "by_name",
            &Value::Text("anna".to_string()),
            &Value::Integer(1),
        )
        .unwrap();

        assert!(start <= duplicate_a && duplicate_a <= end);
        assert!(start <= duplicate_b && duplicate_b <= end);
        assert!(!(start <= prefix_neighbor && prefix_neighbor <= end));
    }

    #[test]
    fn signed_integer_encoding_follows_numeric_order() {
        let mut keys = [i64::MAX, 1, 0, -1, i64::MIN].map(|number| {
            index_entry_key(
                "numbers",
                "by_number",
                &Value::Integer(number),
                &Value::Integer(0),
            )
            .unwrap()
        });
        keys.sort();
        let decoded: Vec<i64> = keys
            .iter()
            .map(|key| match decode_index_entry(key).unwrap().2 {
                Value::Integer(number) => number,
                _ => unreachable!(),
            })
            .collect();
        assert_eq!(decoded, [i64::MIN, -1, 0, 1, i64::MAX]);
    }

    #[test]
    fn text_and_blob_encoding_follows_byte_order() {
        for values in [
            vec![
                Value::Text(String::new()),
                Value::Text("a".to_string()),
                Value::Text("a\0".to_string()),
                Value::Text("aa".to_string()),
            ],
            vec![
                Value::Blob(vec![]),
                Value::Blob(vec![0]),
                Value::Blob(vec![0, 0]),
                Value::Blob(vec![1]),
            ],
        ] {
            let mut keys: Vec<_> = values
                .iter()
                .rev()
                .map(|value| index_entry_key("t", "i", value, &Value::Integer(0)).unwrap())
                .collect();
            keys.sort();
            let decoded: Vec<_> = keys
                .iter()
                .map(|key| decode_index_entry(key).unwrap().2)
                .collect();
            assert_eq!(decoded, values);
        }
    }

    #[test]
    fn type_and_component_boundaries_do_not_collide() {
        let candidates = [
            index_entry_key("a", "bc", &Value::Text("x".to_string()), &Value::Integer(1)).unwrap(),
            index_entry_key("ab", "c", &Value::Text("x".to_string()), &Value::Integer(1)).unwrap(),
            index_entry_key("a", "bc", &Value::Blob(b"x".to_vec()), &Value::Integer(1)).unwrap(),
            index_entry_key(
                "a",
                "bc",
                &Value::Text("x".to_string()),
                &Value::Text("1".to_string()),
            )
            .unwrap(),
        ];

        for left in 0..candidates.len() {
            for right in left + 1..candidates.len() {
                assert_ne!(candidates[left], candidates[right]);
            }
        }
    }

    #[test]
    fn malformed_and_oversized_keys_are_rejected() {
        let valid = index_entry_key("t", "i", &Value::Integer(1), &Value::Integer(2)).unwrap();
        for end in 0..valid.len() {
            assert!(decode_index_entry(&valid[..end]).is_err());
        }

        let oversized = Value::Blob(vec![1; MAX_KEY_SIZE]);
        assert!(index_entry_key("t", "i", &oversized, &Value::Integer(1)).is_err());
    }

    proptest! {
        #[test]
        fn generated_entries_round_trip(
            table in "[a-z][a-z0-9_]{0,16}",
            index in "[a-z][a-z0-9_]{0,16}",
            secondary in arbitrary_value(),
            primary in arbitrary_value(),
        ) {
            let key = index_entry_key(&table, &index, &secondary, &primary).unwrap();
            let decoded = decode_index_entry(&key).unwrap();
            prop_assert_eq!(decoded, (table, index, secondary, primary));
        }
    }

    fn arbitrary_value() -> impl Strategy<Value = Value> {
        prop_oneof![
            any::<i64>().prop_map(Value::Integer),
            prop::collection::vec(any::<char>(), 0..32)
                .prop_map(|chars| Value::Text(chars.into_iter().collect())),
            prop::collection::vec(any::<u8>(), 0..32).prop_map(Value::Blob),
        ]
    }
}
