//! `tosumu-sql` — toy SQL query layer for the tosumu embedded database (MVP+9).
//!
//! This crate implements a minimal SQL surface over `tosumu_core::page_store::PageStore`.
//! It does not depend on CLI, TUI, or any storage internals below `PageStore`.
//!
//! # Supported statements (baseline)
//!
//! - `CREATE TABLE <name> ( <pk_name> TYPE PRIMARY KEY, <col> TYPE, ... )`
//! - `INSERT INTO <table> VALUES ( ... )`
//! - `SELECT <projection> FROM <table> WHERE <pk> = ?`
//!
//! # Public API
//!
//! ```rust,no_run
//! use tosumu_sql::{SqlDatabase, Value};
//! use std::path::Path;
//! use std::str::FromStr;
//!
//! let mut db = SqlDatabase::open(Path::new("test.tsm")).unwrap();
//! db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )").unwrap();
//! db.execute("INSERT INTO users VALUES ( 1, 'alice' )").unwrap();
//! ```

#![forbid(unsafe_code)]

pub mod ast;
pub mod error;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod planner;
pub mod catalog;
pub mod row_codec;
pub mod executor;

/// SQL value representation. Re-exported from `ast` for convenience.
pub use ast::{DataType, Expr, Projection, Stmt, Value};

/// SQL-layer error type. Re-exported from `error` for convenience.
pub use error::SqlError;

/// Result type alias for SQL operations.
pub type SqlResult<T> = std::result::Result<T, SqlError>;

// ── Public API (Phase 5/6 — wired over PageStore) ─────────────────────────────

use crate::executor::{ExecutionOutcome, Executor};
use crate::planner::Planner;
use crate::semantic::SemanticChecker;
use tosumu_core::page_store::PageStore;

/// Opaque database handle for SQL operations.
pub struct SqlDatabase {
    store: PageStore,
}

impl SqlDatabase {
    /// Open an existing database file at the given path.
    pub fn open(path: &std::path::Path) -> SqlResult<Self> {
        let store = PageStore::open(path)
            .map_err(SqlError::CatalogStorage)?;
        Ok(SqlDatabase { store })
    }

    /// Create a new database file at the given path (fails if exists).
    pub fn create(path: &std::path::Path) -> SqlResult<Self> {
        let store = PageStore::create(path)
            .map_err(SqlError::CatalogStorage)?;
        Ok(SqlDatabase { store })
    }

    /// Prepare a SQL statement for execution.
    ///
    /// Parses and counts parameters without requiring a mutable database borrow.
    pub fn prepare(&self, sql: &str) -> SqlResult<PreparedStatement> {
        let stmt = parser::parse(sql)?;
        let parameter_count = stmt.parameter_count();
        Ok(PreparedStatement { stmt, parameter_count })
    }

    /// Execute a prepared statement with the given bindings.
    pub fn execute_prepared(
        &mut self,
        stmt: &PreparedStatement,
        bindings: &[Value],
    ) -> SqlResult<ExecutionOutcome> {
        // Validate binding count
        if bindings.len() != stmt.parameter_count {
            return Err(SqlError::BindingCountMismatch {
                expected: stmt.parameter_count,
                got: bindings.len(),
            });
        }

        let checker = SemanticChecker::new(EmptyCatalogForExec);
        let table_catalog = match &stmt.stmt {
            Stmt::CreateTable { name, columns } => {
                checker.check_create_table(&stmt.stmt)?;
                // Build and store catalog entry so planner can use it
                let pk_index = columns.iter().position(|c| c.is_primary_key).unwrap_or(0);
                let table_def = catalog::TableDef {
                    name: name.clone(),
                    columns: columns.clone(),
                    primary_key_index: pk_index,
                    root_page: None,
                };
                Some(table_def)
            }
            Stmt::Insert { table, .. } => {
                checker.check_insert(&stmt.stmt)?;
                let table_def = self
                    .load_catalog_entry(table)
                    .ok_or_else(|| SqlError::table_not_found(table))?;
                checker.check_insert_against_schema(&stmt.stmt, &table_def)?;
                Some(table_def)
            }
            Stmt::Select { table, .. } => {
                checker.check_select(&stmt.stmt)?;
                let table_def = self
                    .load_catalog_entry(table)
                    .ok_or_else(|| SqlError::table_not_found(table))?;
                Some(table_def)
            }
            Stmt::Delete { table, .. } => {
                checker.check_delete(&stmt.stmt)?;
                let table_def = self
                    .load_catalog_entry(table)
                    .ok_or_else(|| SqlError::table_not_found(table))?;
                Some(table_def)
            }
        };

        // Plan (with catalog context for PK-aware predicate validation)
        let planner = Planner::new();
        let plan_output = match &stmt.stmt {
            Stmt::CreateTable { .. } => planner.plan(&stmt.stmt)?,
            _ => planner.plan_with_catalog(&stmt.stmt, table_catalog.as_ref())?,
        };

        // Execute (executor handles catalog write for CreateTable)
        let executor = Executor::new();
        let mut outcome = executor.execute(
            plan_output.plan,
            bindings,
            &mut self.store,
            table_catalog.as_ref(),
        )?;
        outcome.warnings = plan_output.warnings;
        Ok(outcome)
    }

    /// Load a catalog entry from the store.
    fn load_catalog_entry(&self, table_name: &str) -> Option<catalog::TableDef> {
        let key = catalog::table_key(table_name);
        if let Ok(Some(data)) = self.store.get(key.as_bytes()) {
            catalog::deserialize_table_def(&data).ok()
        } else {
            None
        }
    }

    /// Execute a SQL statement directly (parse + plan + execute).
    pub fn execute(&mut self, sql: &str) -> SqlResult<ExecutionOutcome> {
        let stmt = self.prepare(sql)?;
        self.execute_prepared(&stmt, &[])
    }
}

/// Empty catalog for execution-time semantic checking.
struct EmptyCatalogForExec;

impl crate::semantic::Catalog for EmptyCatalogForExec {
    fn get_table(&self, _name: &str) -> Option<catalog::TableDef> { None }
    fn table_exists(&self, _name: &str) -> bool { false }
}

/// A prepared SQL statement.
///
/// In the final design, `prepare()` parses and counts parameters without
/// requiring a mutable database borrow. The statement can then be reused
/// with different bindings.
#[derive(Debug, Clone)]
pub struct PreparedStatement {
    stmt: Stmt,
    parameter_count: usize,
}

impl PreparedStatement {
    /// Return the underlying statement AST.
    pub fn stmt(&self) -> &Stmt {
        &self.stmt
    }

    /// Return the number of `?` parameters in this statement.
    pub fn parameter_count(&self) -> usize {
        self.parameter_count
    }
}

// ── Unit tests (Phase 5 — integration) ────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::QueryResult;

    fn test_db_path() -> (std::path::PathBuf, tempfile::TempDir) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("test.tsm");
        (path, dir)
    }

    #[test]
    fn create_table_and_insert_and_select() {
        let (path, _dir) = test_db_path();
        
        // Create a new database
        let mut db = SqlDatabase::create(&path).unwrap();
        
        // Create table
        let result = db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )");
        assert!(result.is_ok());
        
        // Insert a row
        let result = db.execute("INSERT INTO users VALUES ( 1, 'alice' )");
        assert!(result.is_ok());
        if let ExecutionOutcome { result: QueryResult::Affected { rows }, .. } = result.unwrap() {
            assert_eq!(rows, 1);
        } else {
            panic!("expected Affected result");
        }
        
        // Select the row back
        let result = db.execute("SELECT * FROM users WHERE id = 1");
        assert!(result.is_ok());
        if let ExecutionOutcome { result: QueryResult::Select { columns, rows }, .. } = result.unwrap() {
            // SELECT * returns actual column names from catalog (id, name)
            assert_eq!(columns.len(), 2);
            assert_eq!(columns[0], "id");
            assert_eq!(columns[1], "name");
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0], vec![Value::Integer(1), Value::Text("alice".to_string())]);
        } else {
            panic!("expected Select result");
        }
    }

    #[test]
    fn prepared_statements_use_bound_primary_keys() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )").unwrap();

        let insert = db.prepare("INSERT INTO users VALUES ( ?, ? )").unwrap();
        db.execute_prepared(
            &insert,
            &[Value::Integer(7), Value::Text("alice".to_string())],
        )
        .unwrap();

        let select = db.prepare("SELECT * FROM users WHERE id = ?").unwrap();
        let result = db.execute_prepared(&select, &[Value::Integer(7)]).unwrap();

        if let ExecutionOutcome { result: QueryResult::Select { rows, .. }, .. } = result {
            assert_eq!(rows, vec![vec![Value::Integer(7), Value::Text("alice".to_string())]]);
        } else {
            panic!("expected Select result");
        }
    }

    #[test]
    fn insert_into_missing_table_is_rejected() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        let error = db.execute("INSERT INTO users VALUES ( 1, 'alice' )").unwrap_err();
        assert!(matches!(error, SqlError::TableNotFound { table } if table == "users"));
    }

    #[test]
    fn named_projection_returns_only_requested_columns() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )").unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )").unwrap();

        let result = db.execute("SELECT name FROM users WHERE id = 1").unwrap();

        if let ExecutionOutcome { result: QueryResult::Select { columns, rows }, .. } = result {
            assert_eq!(columns, vec!["name".to_string()]);
            assert_eq!(rows, vec![vec![Value::Text("alice".to_string())]]);
        } else {
            panic!("expected Select result");
        }
    }

    #[test]
    fn select_star_surfaces_planner_warnings() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )").unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )").unwrap();

        let result = db.execute("SELECT * FROM users WHERE id = 1").unwrap();
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn select_nonexistent_key_returns_empty() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();
        
        db.execute("CREATE TABLE t ( id INTEGER PRIMARY KEY )").unwrap();
        
        let result = db.execute("SELECT * FROM t WHERE id = 999");
        assert!(result.is_ok());
        if let ExecutionOutcome { result: QueryResult::Select { rows, .. }, .. } = result.unwrap() {
            assert!(rows.is_empty());
        } else {
            panic!("expected Select result");
        }
    }

    #[test]
    fn unsupported_query_shape_rejected() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();
        
        // SELECT without WHERE should be rejected
        let result = db.execute("SELECT * FROM users");
        assert!(result.is_err());
    }

    #[test]
    fn ast_parameter_count_create_table() {
        let stmt = Stmt::CreateTable {
            name: "users".to_string(),
            columns: vec![ast::ColumnDef {
                name: "id".to_string(),
                data_type: DataType::Integer,
                is_primary_key: true,
            }],
        };
        assert_eq!(stmt.parameter_count(), 0);
    }

    #[test]
    fn ast_parameter_count_insert_with_params() {
        let stmt = Stmt::Insert {
            table: "users".to_string(),
            values: vec![
                Expr::Parameter(1),
                Expr::Literal(Value::Text("alice".to_string())),
            ],
        };
        assert_eq!(stmt.parameter_count(), 1);
    }

    #[test]
    fn ast_parameter_count_select_with_params() {
        let stmt = Stmt::Select {
            table: "users".to_string(),
            columns: Projection::All,
            predicate: Some(Expr::Eq(
                Box::new(Expr::Column("id".to_string())),
                Box::new(Expr::Parameter(1)),
            )),
        };
        assert_eq!(stmt.parameter_count(), 1);
    }

    #[test]
    fn value_to_sql_literal_integer() {
        let v = Value::Integer(42);
        assert_eq!(v.to_sql_literal(), "42");
    }

    #[test]
    fn value_to_sql_literal_text() {
        let v = Value::Text("alice".to_string());
        assert_eq!(v.to_sql_literal(), "'alice'");
    }

    #[test]
    fn value_to_sql_literal_text_with_quotes() {
        let v = Value::Text("o'reilly".to_string());
        assert_eq!(v.to_sql_literal(), "'o''reilly'");
    }

    #[test]
    fn catalog_key_building() {
        assert_eq!(catalog::table_key("users"), "__sql_catalog__/table/users");
        assert_eq!(catalog::meta_key("version"), "__sql_catalog__/meta/version");
    }

    #[test]
    fn data_type_from_str() {
        use std::str::FromStr;

        assert_eq!(DataType::from_str("INTEGER"), Ok(DataType::Integer));
        assert_eq!(DataType::from_str("TEXT"), Ok(DataType::Text));
        assert_eq!(DataType::from_str("BLOB"), Ok(DataType::Blob));
        assert_eq!(DataType::from_str("REAL"), Err(()));
    }

    #[test]
    fn data_type_name() {
        assert_eq!(DataType::Integer.name(), "INTEGER");
        assert_eq!(DataType::Text.name(), "TEXT");
        assert_eq!(DataType::Blob.name(), "BLOB");
    }
}