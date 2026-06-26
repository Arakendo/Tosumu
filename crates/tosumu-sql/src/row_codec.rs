//! Row encoding for the tosumu toy SQL layer (MVP+9).
//!
//! Handles serialization of row payloads and building row keys.

use crate::ast::Value;
use crate::error::{SqlError, SqlResult};

/// Reserved key prefix for row data.
pub const ROW_KEY_PREFIX: &str = "__sql_row__/";

/// Build the row key for a table + primary key value.
pub fn row_key(table: &str, pk: &Value) -> String {
    format!("{}{}/{}", ROW_KEY_PREFIX, table, pk.to_sql_literal())
}

/// Encode non-PK column values into a row payload.
///
/// Wire format:
/// ```text
/// [version: u8]
/// [column_count: u16]
/// repeat column_count times:
///   [type_tag: u8]
///   [payload_len: u32]
///   [payload bytes]
/// ```
pub fn encode_row_values(columns: &[&str], types: &[u8], values: &[Value]) -> SqlResult<Vec<u8>> {
    let mut buf = Vec::new();
    
    // Version
    buf.push(1);
    
    // Column count (total columns including PK)
    let total_count = if columns.is_empty() && types.is_empty() {
        values.len()
    } else {
        columns.len().max(types.len()).max(values.len())
    };
    buf.extend_from_slice(&(total_count as u16).to_le_bytes());
    
    // Columns
    for i in 0..total_count {
        let value = if i < values.len() { &values[i] } else { &Value::Blob(vec![]) };
        
        // Type tag
        let type_tag = if i < types.len() { types[i] } else { value_type_tag(value) };
        buf.push(type_tag);
        
        // Payload
        let payload = value_payload(value, type_tag)?;
        buf.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        buf.extend_from_slice(&payload);
    }
    
    Ok(buf)
}

/// Decode a row payload into values.
pub fn decode_row_values(data: &[u8]) -> SqlResult<Vec<Value>> {
    if data.is_empty() || data[0] != 1 {
        return Err(SqlError::RowEncoding(format!(
            "unsupported row format version: expected 1, got {}",
            data.first().map(|v| *v as i32).unwrap_or(-1)
        )));
    }
    
    let mut pos = 1;
    
    // Column count
    let col_count = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    
    let mut values = Vec::with_capacity(col_count);
    
    for _ in 0..col_count {
        // Type tag
        let type_tag = data[pos];
        pos += 1;
        
        // Payload length
        let payload_len = u32::from_le_bytes([
            data[pos], data[pos + 1], data[pos + 2], data[pos + 3],
        ]) as usize;
        pos += 4;
        
        // Payload
        let payload = &data[pos..pos + payload_len];
        pos += payload_len;
        
        let value = decode_value(type_tag, payload)?;
        values.push(value);
    }
    
    Ok(values)
}

/// Get the type tag for a value.
fn value_type_tag(value: &Value) -> u8 {
    match value {
        Value::Integer(_) => 1,
        Value::Text(_) => 2,
        Value::Blob(_) => 3,
    }
}

/// Encode a single value into its wire payload.
fn value_payload(value: &Value, type_tag: u8) -> SqlResult<Vec<u8>> {
    match (value, type_tag) {
        (Value::Integer(n), 1) => Ok(n.to_le_bytes().to_vec()),
        (Value::Text(s), 2) => Ok(s.as_bytes().to_vec()),
        (Value::Blob(b), 3) => Ok(b.clone()),
        _ => Err(SqlError::RowEncoding(format!(
            "type mismatch: value {value:?} with tag {type_tag}"
        ))),
    }
}

/// Decode a single wire value into a SQL Value.
fn decode_value(type_tag: u8, payload: &[u8]) -> SqlResult<Value> {
    match type_tag {
        1 => {
            if payload.len() != 8 {
                return Err(SqlError::RowEncoding("INTEGER requires 8 bytes".to_string()));
            }
            let bytes: [u8; 8] = payload.try_into()
                .map_err(|_| SqlError::RowEncoding("invalid INTEGER bytes".to_string()))?;
            Ok(Value::Integer(i64::from_le_bytes(bytes)))
        }
        2 => {
            let s = String::from_utf8(payload.to_vec())
                .map_err(|e| SqlError::RowEncoding(format!("invalid TEXT: {e}")))?;
            Ok(Value::Text(s))
        }
        3 => Ok(Value::Blob(payload.to_vec())),
        _ => Err(SqlError::RowEncoding(format!("unsupported type tag: {type_tag}"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn encode_and_decode_empty_columns() {
        let encoded = encode_row_values(&[], &[], &[]).unwrap();
        let decoded = decode_row_values(&encoded).unwrap();
        assert!(decoded.is_empty());
    }

    #[test]
    fn encode_and_decode_single_integer() {
        let values = vec![Value::Integer(42)];
        let encoded = encode_row_values(&["pk"], &[1], &values).unwrap();
        let decoded = decode_row_values(&encoded).unwrap();
        assert_eq!(decoded, vec![Value::Integer(42)]);
    }

    #[test]
    fn encode_and_decode_text_value() {
        let values = vec![Value::Text("alice".to_string())];
        let encoded = encode_row_values(&["name"], &[2], &values).unwrap();
        let decoded = decode_row_values(&encoded).unwrap();
        assert_eq!(decoded, vec![Value::Text("alice".to_string())]);
    }

    #[test]
    fn encode_and_decode_blob_value() {
        let values = vec![Value::Blob(vec![1, 2, 3])];
        let encoded = encode_row_values(&["data"], &[3], &values).unwrap();
        let decoded = decode_row_values(&encoded).unwrap();
        assert_eq!(decoded, vec![Value::Blob(vec![1, 2, 3])]);
    }

    #[test]
    fn encode_and_decode_mixed_columns() {
        let values = vec![
            Value::Integer(1),
            Value::Text("alice".to_string()),
            Value::Blob(vec![0xFF]),
        ];
        let encoded = encode_row_values(&["id", "name", "data"], &[1, 2, 3], &values).unwrap();
        let decoded = decode_row_values(&encoded).unwrap();
        assert_eq!(decoded.len(), 3);
        assert_eq!(decoded[0], Value::Integer(1));
        assert_eq!(decoded[1], Value::Text("alice".to_string()));
        assert_eq!(decoded[2], Value::Blob(vec![0xFF]));
    }

    #[test]
    fn row_key_format() {
        let key = row_key("users", &Value::Integer(1));
        assert!(key.starts_with("__sql_row__/users/"));
    }

    #[test]
    fn reject_unsupported_version() {
        let data: Vec<u8> = vec![2, 0, 0, 0]; // version 2
        let result = decode_row_values(&data);
        assert!(result.is_err());
    }

    #[test]
    fn reject_empty_data() {
        let result = decode_row_values(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn row_key_with_text_pk() {
        let key = row_key("users", &Value::Text("alice".to_string()));
        assert!(key.contains("__sql_row__/users/"));
        assert!(key.contains("alice"));
    }

    #[test]
    fn value_payload_integer() {
        let v = Value::Integer(42);
        let payload = value_payload(&v, 1).unwrap();
        assert_eq!(payload.len(), 8);
        assert_eq!(i64::from_le_bytes(payload.try_into().unwrap()), 42);
    }

    #[test]
    fn value_payload_text() {
        let v = Value::Text("hello".to_string());
        let payload = value_payload(&v, 2).unwrap();
        assert_eq!(payload, b"hello");
    }

    #[test]
    fn value_payload_blob() {
        let v = Value::Blob(vec![1, 2, 3]);
        let payload = value_payload(&v, 3).unwrap();
        assert_eq!(payload, vec![1, 2, 3]);
    }

    #[test]
    fn value_payload_type_mismatch() {
        let v = Value::Integer(42);
        let result = value_payload(&v, 2); // INTEGER with TEXT tag
        assert!(result.is_err());
    }
}