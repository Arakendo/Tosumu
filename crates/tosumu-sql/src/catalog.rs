//! Catalog serialization for the tosumu toy SQL layer (MVP+9).
//!
//! Handles reserved key prefixes and wire-format serialization of table definitions.

use crate::ast::{ColumnDef, DataType};
use crate::error::{SqlError, SqlResult};

/// Reserved key prefix for table catalog entries.
pub const TABLE_KEY_PREFIX: &str = "__sql_catalog__/table/";

/// Reserved key prefix for metadata entries.
pub const META_KEY_PREFIX: &str = "__sql_catalog__/meta/";

/// Current catalog format version.
pub const CATALOG_VERSION: u8 = 1;

/// Table definition stored in the catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    pub primary_key_index: usize,
    pub root_page: Option<u64>,
}

/// Build the catalog key for a table.
pub fn table_key(table: &str) -> String {
    format!("{TABLE_KEY_PREFIX}{table}")
}

/// Build the catalog key for metadata.
pub fn meta_key(key: &str) -> String {
    format!("{META_KEY_PREFIX}{key}")
}

/// Serialize a TableDef to its wire format.
pub fn serialize_table_def(table_def: &TableDef) -> Vec<u8> {
    let mut buf = Vec::new();
    
    // Version
    buf.push(CATALOG_VERSION);
    
    // Table name
    let name_bytes = table_def.name.as_bytes();
    buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
    buf.extend_from_slice(name_bytes);
    
    // Column count
    buf.extend_from_slice(&(table_def.columns.len() as u16).to_le_bytes());
    
    // Primary key index (stored as u16 for wire compatibility)
    let pk_u16 = table_def.primary_key_index as u16;
    buf.extend_from_slice(&pk_u16.to_le_bytes());
    
    // Root page present flag and value
    if let Some(root_page) = table_def.root_page {
        buf.push(1);
        buf.extend_from_slice(&root_page.to_le_bytes());
    } else {
        buf.push(0);
    }
    
    // Columns
    for col in &table_def.columns {
        let name_bytes = col.name.as_bytes();
        buf.extend_from_slice(&(name_bytes.len() as u16).to_le_bytes());
        buf.extend_from_slice(name_bytes);
        
        // Type tag
        buf.push(col.data_type.as_u8());
        
        // Is primary key
        buf.push(if col.is_primary_key { 1 } else { 0 });
    }
    
    buf
}

/// Deserialize a TableDef from its wire format.
pub fn deserialize_table_def(data: &[u8]) -> SqlResult<TableDef> {
    if data.is_empty() || data[0] != CATALOG_VERSION {
        return Err(SqlError::RowEncoding(format!(
            "unsupported catalog version: expected {}, got {}",
            CATALOG_VERSION,
            data.first().map(|v| *v as i32).unwrap_or(-1)
        )));
    }
    
    let mut pos = 1; // skip version
    
    // Table name
    let name_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    let name = String::from_utf8(data[pos..pos + name_len].to_vec())
        .map_err(|e| SqlError::RowEncoding(format!("invalid table name: {e}")))?;
    pos += name_len;
    
    // Column count
    let col_count = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    
    // Primary key index (stored as u16 for wire compatibility)
    let pk_index = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
    pos += 2;
    
    // Root page (8 bytes if present)
    let root_page = if data[pos] == 1 {
        pos += 1;
        // Ensure we have enough bytes for u64
        if pos + 8 > data.len() {
            return Err(SqlError::RowEncoding("truncated root page data".to_string()));
        }
        let rp = u64::from_le_bytes([
            data[pos], data[pos + 1], data[pos + 2], data[pos + 3],
            data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7],
        ]);
        pos += 8; // advance past the root page value
        Some(rp)
    } else {
        pos += 1;
        None
    };
    
    // Columns
    let mut columns = Vec::with_capacity(col_count);
    for _ in 0..col_count {
        let col_name_len = u16::from_le_bytes([data[pos], data[pos + 1]]) as usize;
        pos += 2;
        let col_name = String::from_utf8(data[pos..pos + col_name_len].to_vec())
            .map_err(|e| SqlError::RowEncoding(format!("invalid column name: {e}")))?;
        pos += col_name_len;
        
        let data_type = DataType::from_u8(data[pos])
            .ok_or_else(|| SqlError::RowEncoding(format!("unsupported column type: {}", data[pos])))?;
        pos += 1;
        
        let is_primary_key = data[pos] == 1;
        pos += 1;
        
        columns.push(ColumnDef {
            name: col_name,
            data_type,
            is_primary_key,
        });
    }
    
    Ok(TableDef {
        name,
        columns,
        primary_key_index: pk_index,
        root_page,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serialize_and_deserialize_simple_table() {
        let table_def = TableDef {
            name: "users".to_string(),
            columns: vec![
                ColumnDef {
                    name: "id".to_string(),
                    data_type: DataType::Integer,
                    is_primary_key: true,
                },
                ColumnDef {
                    name: "name".to_string(),
                    data_type: DataType::Text,
                    is_primary_key: false,
                },
            ],
            primary_key_index: 0,
            root_page: None,
        };
        
        let serialized = serialize_table_def(&table_def);
        let deserialized = deserialize_table_def(&serialized).unwrap();
        
        assert_eq!(deserialized.name, "users");
        assert_eq!(deserialized.columns.len(), 2);
        assert_eq!(deserialized.primary_key_index, 0);
        assert!(deserialized.root_page.is_none());
    }

    #[test]
    fn serialize_and_deserialize_with_root_page() {
        let table_def = TableDef {
            name: "t".to_string(),
            columns: vec![ColumnDef {
                name: "id".to_string(),
                data_type: DataType::Integer,
                is_primary_key: true,
            }],
            primary_key_index: 0,
            root_page: Some(42),
        };
        
        let serialized = serialize_table_def(&table_def);
        let deserialized = deserialize_table_def(&serialized).unwrap();
        
        assert_eq!(deserialized.root_page, Some(42));
    }

    #[test]
    fn round_trip_blob_type() {
        let table_def = TableDef {
            name: "blobs".to_string(),
            columns: vec![ColumnDef {
                name: "data".to_string(),
                data_type: DataType::Blob,
                is_primary_key: false,
            }],
            primary_key_index: 0,
            root_page: None,
        };
        
        let serialized = serialize_table_def(&table_def);
        let deserialized = deserialize_table_def(&serialized).unwrap();
        
        assert_eq!(deserialized.columns[0].data_type, DataType::Blob);
    }

    #[test]
    fn reject_unsupported_version() {
        let data: Vec<u8> = vec![2, 0, 1, 0, 0, 0, 0, 0, 0]; // version 2
        let result = deserialize_table_def(&data);
        assert!(result.is_err());
    }

    #[test]
    fn reject_empty_data() {
        let result = deserialize_table_def(&[]);
        assert!(result.is_err());
    }

    #[test]
    fn table_key_format() {
        assert_eq!(table_key("users"), "__sql_catalog__/table/users");
        assert_eq!(table_key("my_table"), "__sql_catalog__/table/my_table");
    }

    #[test]
    fn meta_key_format() {
        assert_eq!(meta_key("version"), "__sql_catalog__/meta/version");
    }

    #[test]
    fn serialize_empty_columns() {
        let table_def = TableDef {
            name: "empty".to_string(),
            columns: vec![],
            primary_key_index: 0,
            root_page: None,
        };
        
        let serialized = serialize_table_def(&table_def);
        let deserialized = deserialize_table_def(&serialized).unwrap();
        
        assert_eq!(deserialized.columns.len(), 0);
    }

    #[test]
    fn data_type_wire_conversion() {
        assert_eq!(DataType::Integer.as_u8(), 1);
        assert_eq!(DataType::Text.as_u8(), 2);
        assert_eq!(DataType::Blob.as_u8(), 3);
        
        assert_eq!(DataType::from_u8(1), Some(DataType::Integer));
        assert_eq!(DataType::from_u8(2), Some(DataType::Text));
        assert_eq!(DataType::from_u8(3), Some(DataType::Blob));
        assert_eq!(DataType::from_u8(99), None);
    }

    #[test]
    fn catalog_version_constant() {
        assert_eq!(CATALOG_VERSION, 1);
    }
}