//! SQL-layer error types.
//!
//! All errors describe phenomena at the SQL level (e.g., "table not found")
//! rather than mechanisms (e.g., "key not in catalog"). They map into
//! `tosumu_core::error::TosumuError` where appropriate.

use thiserror::Error;

/// SQL-layer error kinds.
#[derive(Debug, Error)]
pub enum SqlError {
    /// The requested table does not exist in the catalog.
    #[error("table '{table}' not found")]
    TableNotFound { table: String },

    /// The requested column does not exist in the table's schema.
    #[error("column '{column}' not found in table '{table}'")]
    ColumnNotFound { table: String, column: String },

    /// A column name appears more than once in a CREATE TABLE definition.
    #[error("duplicate column '{column}' in table '{table}'")]
    DuplicateColumn { table: String, column: String },

    /// No primary key was declared in the CREATE TABLE definition.
    #[error("missing primary key in table '{table}'")]
    MissingPrimaryKey { table: String },

    /// A column type is not supported by the baseline SQL layer.
    #[error("unsupported type '{ty}' for column '{column}' in table '{table}'")]
    UnsupportedType { table: String, column: String, ty: String },

    /// The query shape is not supported by the MVP+9 baseline.
    #[error("{0}")]
    UnsupportedQueryShape(String),

    /// The number of bound parameters does not match the number of `?` placeholders.
    #[error("binding count mismatch: expected {expected}, got {got}")]
    BindingCountMismatch { expected: usize, got: usize },

    /// A value's SQL type does not match the column's declared type.
    #[error("type mismatch in table '{table}', column '{column}': expected {expected}, got {got}")]
    TypeMismatch {
        table: String,
        column: String,
        expected: String,
        got: String,
    },

    /// A catalog operation failed due to a core storage error.
    #[error("catalog storage error: {0}")]
    CatalogStorage(#[from] tosumu_core::error::TosumuError),

    /// A row encoding or decoding error.
    #[error("row encoding error: {0}")]
    RowEncoding(String),

    /// The SQL input could not be tokenized or parsed.
    #[error("parse error at line {line}, column {col}: {message}")]
    ParseError { message: String, line: usize, col: usize },
}

impl SqlError {
    /// Create a new `UnsupportedQueryShape` error with a descriptive message.
    pub fn unsupported_query_shape(msg: impl Into<String>) -> Self {
        SqlError::UnsupportedQueryShape(msg.into())
    }

    /// Create a new `TableNotFound` error.
    pub fn table_not_found(table: impl Into<String>) -> Self {
        SqlError::TableNotFound { table: table.into() }
    }

    /// Create a new `ColumnNotFound` error.
    pub fn column_not_found(table: impl Into<String>, column: impl Into<String>) -> Self {
        SqlError::ColumnNotFound {
            table: table.into(),
            column: column.into(),
        }
    }

    /// Create a new `DuplicateColumn` error.
    pub fn duplicate_column(table: impl Into<String>, column: impl Into<String>) -> Self {
        SqlError::DuplicateColumn {
            table: table.into(),
            column: column.into(),
        }
    }

    /// Create a new `MissingPrimaryKey` error.
    pub fn missing_primary_key(table: impl Into<String>) -> Self {
        SqlError::MissingPrimaryKey { table: table.into() }
    }

    /// Create a new `ParseError`.
    pub fn parse_error(message: impl Into<String>, line: usize, col: usize) -> Self {
        SqlError::ParseError {
            message: message.into(),
            line,
            col,
        }
    }
}

/// Result type alias for SQL operations.
pub type SqlResult<T> = std::result::Result<T, SqlError>;