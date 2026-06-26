//! SQL abstract syntax tree (AST) types for the MVP+9 baseline.
//!
//! These types are produced by the parser and consumed by the semantic checker,
//! planner, and executor. They do not depend on any storage implementation.

use std::str::FromStr;

/// Column data type as declared in CREATE TABLE.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataType {
    /// INTEGER column type.
    Integer,
    /// TEXT column type.
    Text,
    /// BLOB column type.
    Blob,
}

impl DataType {
    /// Return the SQL name for this data type.
    pub fn name(&self) -> &'static str {
        match self {
            DataType::Integer => "INTEGER",
            DataType::Text => "TEXT",
            DataType::Blob => "BLOB",
        }
    }

    /// Parse a data type from a wire-format type tag byte.
    pub fn from_u8(tag: u8) -> Option<DataType> {
        match tag {
            1 => Some(DataType::Integer),
            2 => Some(DataType::Text),
            3 => Some(DataType::Blob),
            _ => None,
        }
    }

    /// Convert this data type to a wire-format type tag byte.
    pub fn as_u8(&self) -> u8 {
        match self {
            DataType::Integer => 1,
            DataType::Text => 2,
            DataType::Blob => 3,
        }
    }
}

impl FromStr for DataType {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "INTEGER" => Ok(DataType::Integer),
            "TEXT" => Ok(DataType::Text),
            "BLOB" => Ok(DataType::Blob),
            _ => Err(()),
        }
    }
}

/// A column definition in a CREATE TABLE statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDef {
    /// Column name.
    pub name: String,
    /// Declared data type.
    pub data_type: DataType,
    /// Whether this column is the primary key.
    pub is_primary_key: bool,
}

/// Projection specification for a SELECT statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Projection {
    /// SELECT * — all columns in declaration order.
    All,
    /// SELECT col1, col2, ... — explicitly named columns.
    Named(Vec<String>),
}

/// Top-level SQL statement types supported by the MVP+9 baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    /// CREATE TABLE <name> ( <columns> )
    CreateTable {
        /// Table name.
        name: String,
        /// Column definitions.
        columns: Vec<ColumnDef>,
    },
    /// INSERT INTO <table> VALUES ( <values> )
    Insert {
        /// Table name.
        table: String,
        /// Values to insert (one per column, in declaration order).
        values: Vec<Expr>,
    },
    /// SELECT <projection> FROM <table> WHERE <predicate>
    Select {
        /// Table name.
        table: String,
        /// Columns to project.
        columns: Projection,
        /// Optional WHERE predicate (must be pk = value for baseline).
        predicate: Option<Expr>,
    },
    /// DELETE FROM <table> WHERE <predicate>
    Delete {
        /// Table name.
        table: String,
        /// Optional WHERE predicate.
        predicate: Option<Expr>,
    },
}

impl Stmt {
    /// Return the table name referenced by this statement, if any.
    pub fn table_name(&self) -> Option<&str> {
        match self {
            Stmt::CreateTable { name, .. } => Some(name),
            Stmt::Insert { table, .. } => Some(table),
            Stmt::Select { table, .. } => Some(table),
            Stmt::Delete { table, .. } => Some(table),
        }
    }

    /// Count the number of `?` parameter placeholders in this statement.
    pub fn parameter_count(&self) -> usize {
        match self {
            Stmt::CreateTable { .. } => 0,
            Stmt::Insert { values, .. } => {
                values.iter().filter(|e| e.is_parameter()).count()
            }
            Stmt::Select { predicate, .. } => {
                predicate.as_ref().map_or(0, |e| e.parameter_count())
            }
            Stmt::Delete { predicate, .. } => {
                predicate.as_ref().map_or(0, |e| e.parameter_count())
            }
        }
    }
}

/// SQL expression tree nodes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Expr {
    /// A literal value (e.g., `123`, `'hello'`).
    Literal(Value),
    /// A column reference by name (e.g., `id` in `WHERE id = ?`).
    Column(String),
    /// Equality comparison: left = right.
    Eq(Box<Expr>, Box<Expr>),
    /// A positional parameter placeholder (`?`).
    Parameter(usize),
}

impl Expr {
    /// Count the number of `?` parameter placeholders in this expression.
    pub fn parameter_count(&self) -> usize {
        match self {
            Expr::Literal(_) => 0,
            Expr::Column(_) => 0,
            Expr::Eq(left, right) => left.parameter_count() + right.parameter_count(),
            Expr::Parameter(_) => 1,
        }
    }

    /// Return true if this expression contains at least one parameter placeholder.
    pub fn is_parameter(&self) -> bool {
        matches!(self, Expr::Parameter(_))
    }
}

/// SQL value types supported by the MVP+9 baseline.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    /// INTEGER value (signed 64-bit).
    Integer(i64),
    /// TEXT value (UTF-8 string).
    Text(String),
    /// BLOB value (arbitrary bytes).
    Blob(Vec<u8>),
}

impl Value {
    /// Return the SQL type name for this value.
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Integer(_) => "INTEGER",
            Value::Text(_) => "TEXT",
            Value::Blob(_) => "BLOB",
        }
    }

    /// Return the declared DataType that this value corresponds to.
    pub fn data_type(&self) -> DataType {
        match self {
            Value::Integer(_) => DataType::Integer,
            Value::Text(_) => DataType::Text,
            Value::Blob(_) => DataType::Blob,
        }
    }

    /// Format this value as an SQL literal string suitable for display.
    pub fn to_sql_literal(&self) -> String {
        match self {
            Value::Integer(n) => format!("{n}"),
            Value::Text(s) => {
                let escaped = s.replace('\'', "''");
                format!("'{escaped}'")
            }
            Value::Blob(bytes) => {
                let hex: String = bytes.iter().map(|b| format!("{b:02x}")).collect();
                format!("X'{hex}'")
            }
        }
    }
}