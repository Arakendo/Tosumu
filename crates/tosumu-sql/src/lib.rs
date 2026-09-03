//! `tosumu-sql` — initial SQL query layer for the tosumu embedded database (MVP+9).
//!
//! This crate implements a minimal SQL surface over `tosumu_core::SharedKvStore`.
//! It does not depend on CLI, TUI, or physical storage internals.
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
pub mod catalog;
pub mod error;
pub mod executor;
mod index_codec;
pub mod lexer;
pub mod parser;
pub mod planner;
pub mod row_codec;
pub mod semantic;

/// SQL value representation. Re-exported from `ast` for convenience.
pub use ast::{DataType, Expr, Projection, Stmt, Value};
pub use executor::{ExecutionOutcome, QueryResult};
pub use planner::PlanWarning;

/// SQL-layer error type. Re-exported from `error` for convenience.
pub use error::SqlError;

/// Result type alias for SQL operations.
pub type SqlResult<T> = std::result::Result<T, SqlError>;

/// Result of planning a statement without executing it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExplainOutcome {
    pub plan: String,
    pub warnings: Vec<PlanWarning>,
}

// ── Public API (wired over the shared KV owner) ───────────────────────────────

use crate::executor::Executor;
use crate::planner::Planner;
use crate::semantic::SemanticChecker;
use tosumu_core::SharedKvStore;

/// Opaque database handle for SQL operations.
pub struct SqlDatabase {
    store: SharedKvStore,
}

impl SqlDatabase {
    /// Open an existing database file at the given path.
    pub fn open(path: &std::path::Path) -> SqlResult<Self> {
        let store = SharedKvStore::open(path).map_err(SqlError::CatalogStorage)?;
        Ok(SqlDatabase { store })
    }

    /// Create a new database file at the given path (fails if exists).
    pub fn create(path: &std::path::Path) -> SqlResult<Self> {
        let store = SharedKvStore::create(path).map_err(SqlError::CatalogStorage)?;
        Ok(SqlDatabase { store })
    }

    /// Prepare a SQL statement for execution.
    ///
    /// Parses and counts parameters without requiring a mutable database borrow.
    pub fn prepare(&self, sql: &str) -> SqlResult<PreparedStatement> {
        let stmt = parser::parse(sql)?;
        let parameter_count = stmt.parameter_count();
        Ok(PreparedStatement {
            stmt,
            parameter_count,
        })
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

        let table_catalog = self.catalog_for_statement(&stmt.stmt)?;
        let indexes = match table_catalog.as_ref() {
            Some(table) => self.load_indexes_for_table(&table.name)?,
            None => vec![],
        };

        // Plan (with catalog context for PK-aware predicate validation)
        let plan_output = Self::plan_statement(&stmt.stmt, table_catalog.as_ref(), &indexes)?;

        // Execute (executor handles catalog write for CreateTable)
        let executor = Executor::new();
        let mut outcome = executor.execute(
            plan_output.plan,
            bindings,
            &self.store,
            table_catalog.as_ref(),
        )?;
        outcome.warnings = plan_output.warnings;
        Ok(outcome)
    }

    /// Plan a SQL statement without modifying the database.
    pub fn explain(&self, sql: &str) -> SqlResult<ExplainOutcome> {
        let stmt = self.prepare(sql)?;
        let table_catalog = self.catalog_for_statement(&stmt.stmt)?;
        let indexes = match table_catalog.as_ref() {
            Some(table) => self.load_indexes_for_table(&table.name)?,
            None => vec![],
        };
        let plan_output = Self::plan_statement(&stmt.stmt, table_catalog.as_ref(), &indexes)?;

        Ok(ExplainOutcome {
            plan: plan_output.plan.describe(),
            warnings: plan_output.warnings,
        })
    }

    fn catalog_for_statement(&self, stmt: &Stmt) -> SqlResult<Option<catalog::TableDef>> {
        let checker = SemanticChecker::new(EmptyCatalogForExec);
        match stmt {
            Stmt::CreateTable { name, columns } => {
                checker.check_create_table(stmt)?;
                if self.load_catalog_entry(name).is_some() {
                    return Err(SqlError::TableAlreadyExists {
                        table: name.clone(),
                    });
                }
                // Build and store catalog entry so planner can use it
                let pk_index = columns.iter().position(|c| c.is_primary_key).unwrap_or(0);
                let table_def = catalog::TableDef {
                    name: name.clone(),
                    columns: columns.clone(),
                    primary_key_index: pk_index,
                    root_page: None,
                };
                Ok(Some(table_def))
            }
            Stmt::CreateIndex { name, table, .. } => {
                if self.load_index_entry(name).is_some() {
                    return Err(SqlError::IndexAlreadyExists {
                        index: name.clone(),
                    });
                }
                let table_def = self
                    .load_catalog_entry(table)
                    .ok_or_else(|| SqlError::table_not_found(table))?;
                checker.check_create_index(stmt, &table_def)?;
                Ok(Some(table_def))
            }
            Stmt::Insert { table, .. } => {
                checker.check_insert(stmt)?;
                let table_def = self
                    .load_catalog_entry(table)
                    .ok_or_else(|| SqlError::table_not_found(table))?;
                checker.check_insert_against_schema(stmt, &table_def)?;
                Ok(Some(table_def))
            }
            Stmt::Select { table, .. } => {
                checker.check_select(stmt)?;
                let table_def = self
                    .load_catalog_entry(table)
                    .ok_or_else(|| SqlError::table_not_found(table))?;
                Ok(Some(table_def))
            }
            Stmt::Delete { table, .. } => {
                checker.check_delete(stmt)?;
                let table_def = self
                    .load_catalog_entry(table)
                    .ok_or_else(|| SqlError::table_not_found(table))?;
                Ok(Some(table_def))
            }
        }
    }

    fn plan_statement(
        stmt: &Stmt,
        table_catalog: Option<&catalog::TableDef>,
        indexes: &[catalog::IndexDef],
    ) -> SqlResult<crate::planner::PlanOutput> {
        let planner = Planner::new();
        planner.plan_with_catalog_and_indexes(stmt, table_catalog, indexes)
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

    fn load_index_entry(&self, index_name: &str) -> Option<catalog::IndexDef> {
        let key = catalog::index_key(index_name);
        self.store
            .get(key.as_bytes())
            .ok()
            .flatten()
            .and_then(|data| catalog::deserialize_index_def(&data).ok())
    }

    fn load_indexes_for_table(&self, table_name: &str) -> SqlResult<Vec<catalog::IndexDef>> {
        let (start, end) = catalog::index_catalog_bounds();
        self.store
            .snapshot()?
            .scan(&start, &end)?
            .into_iter()
            .map(|(_, payload)| catalog::deserialize_index_def(&payload))
            .filter_map(|result| match result {
                Ok(index) if index.table == table_name => Some(Ok(index)),
                Ok(_) => None,
                Err(error) => Some(Err(error)),
            })
            .collect()
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
    fn get_table(&self, _name: &str) -> Option<catalog::TableDef> {
        None
    }
    fn table_exists(&self, _name: &str) -> bool {
        false
    }
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
    use crate::index_codec::{decode_index_entry, index_entry_bounds};

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
        if let ExecutionOutcome {
            result: QueryResult::Affected { rows },
            ..
        } = result.unwrap()
        {
            assert_eq!(rows, 1);
        } else {
            panic!("expected Affected result");
        }

        // Select the row back
        let result = db.execute("SELECT * FROM users WHERE id = 1");
        assert!(result.is_ok());
        if let ExecutionOutcome {
            result: QueryResult::Select { columns, rows },
            ..
        } = result.unwrap()
        {
            // SELECT * returns actual column names from catalog (id, name)
            assert_eq!(columns.len(), 2);
            assert_eq!(columns[0], "id");
            assert_eq!(columns[1], "name");
            assert_eq!(rows.len(), 1);
            assert_eq!(
                rows[0],
                vec![Value::Integer(1), Value::Text("alice".to_string())]
            );
        } else {
            panic!("expected Select result");
        }
    }

    #[test]
    fn shared_owner_sql_database_reopens_existing_rows() {
        let (path, _dir) = test_db_path();
        {
            let mut db = SqlDatabase::create(&path).unwrap();
            db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
                .unwrap();
            db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
                .unwrap();
        }

        let mut reopened = SqlDatabase::open(&path).unwrap();
        let outcome = reopened
            .execute("SELECT * FROM users WHERE id = 1")
            .unwrap();
        assert!(matches!(
            outcome.result,
            QueryResult::Select { rows, .. }
                if rows == vec![vec![Value::Integer(1), Value::Text("alice".to_string())]]
        ));
    }

    #[test]
    fn create_index_atomically_backfills_duplicate_secondary_values() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();
        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 2, 'alice' )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 3, 'bob' )").unwrap();

        db.execute("CREATE INDEX users_by_name ON users ( name )")
            .unwrap();

        let snapshot = db.store.snapshot().unwrap();
        let (start, end) =
            index_entry_bounds("users", "users_by_name", &Value::Text("alice".to_string()))
                .unwrap();
        let entries = snapshot.scan(&start, &end).unwrap();
        let primary_keys: Vec<_> = entries
            .iter()
            .map(|(key, value)| {
                assert!(value.is_empty());
                decode_index_entry(key).unwrap().3
            })
            .collect();
        assert_eq!(primary_keys, [Value::Integer(1), Value::Integer(2)]);
        assert!(snapshot
            .get(catalog::index_key("users_by_name").as_bytes())
            .unwrap()
            .is_some());
    }

    #[test]
    fn create_index_validates_name_table_column_and_primary_key() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();
        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        assert!(matches!(
            db.execute("CREATE INDEX missing_table ON ghosts ( name )"),
            Err(SqlError::TableNotFound { .. })
        ));
        assert!(matches!(
            db.execute("CREATE INDEX missing_column ON users ( email )"),
            Err(SqlError::ColumnNotFound { .. })
        ));
        assert!(matches!(
            db.execute("CREATE INDEX redundant_pk ON users ( id )"),
            Err(SqlError::IndexOnPrimaryKey { .. })
        ));
        db.execute("CREATE INDEX users_by_name ON users ( name )")
            .unwrap();
        assert!(matches!(
            db.execute("CREATE INDEX users_by_name ON users ( name )"),
            Err(SqlError::IndexAlreadyExists { .. })
        ));
    }

    #[test]
    fn failed_index_backfill_publishes_neither_catalog_nor_entries() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();
        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        let key = row_codec::row_key("users", &Value::Integer(1));
        db.store.put(key.as_bytes(), b"malformed-row").unwrap();

        assert!(matches!(
            db.execute("CREATE INDEX users_by_name ON users ( name )"),
            Err(SqlError::RowEncoding(_))
        ));
        let snapshot = db.store.snapshot().unwrap();
        assert!(snapshot
            .get(catalog::index_key("users_by_name").as_bytes())
            .unwrap()
            .is_none());
        let (start, end) =
            index_entry_bounds("users", "users_by_name", &Value::Text("alice".to_string()))
                .unwrap();
        assert!(snapshot.scan(&start, &end).unwrap().is_empty());
    }

    #[test]
    fn insert_replacement_and_delete_maintain_secondary_entries() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();
        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 2, 'alice' )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 3, 'bob' )").unwrap();
        db.execute("CREATE INDEX users_by_name ON users ( name )")
            .unwrap();

        db.execute("INSERT INTO users VALUES ( 1, 'bob' )").unwrap();
        db.execute("INSERT INTO users VALUES ( 3, 'bob' )").unwrap();
        assert_eq!(indexed_primary_keys(&db, "alice"), vec![Value::Integer(2)]);
        assert_eq!(
            indexed_primary_keys(&db, "bob"),
            vec![Value::Integer(1), Value::Integer(3)]
        );

        db.execute("DELETE FROM users WHERE id = 2").unwrap();
        assert!(indexed_primary_keys(&db, "alice").is_empty());
        db.execute("DELETE FROM users WHERE id = 1 OR id = 3")
            .unwrap();
        assert!(indexed_primary_keys(&db, "bob").is_empty());
    }

    #[test]
    fn secondary_equality_lookup_returns_duplicates_and_names_index_in_explain() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();
        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 2, 'alice' )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 3, 'bob' )").unwrap();
        db.execute("CREATE INDEX users_by_name ON users ( name )")
            .unwrap();

        let explain = db
            .explain("SELECT id FROM users WHERE name = 'alice'")
            .unwrap();
        assert_eq!(
            explain.plan,
            "SECONDARY_INDEX_LOOKUP table=users index=users_by_name"
        );
        let prepared = db
            .prepare("SELECT id, name FROM users WHERE name = ?")
            .unwrap();
        let outcome = db
            .execute_prepared(&prepared, &[Value::Text("alice".to_string())])
            .unwrap();
        assert_eq!(
            outcome.result,
            QueryResult::Select {
                columns: vec!["id".to_string(), "name".to_string()],
                rows: vec![
                    vec![Value::Integer(1), Value::Text("alice".to_string())],
                    vec![Value::Integer(2), Value::Text("alice".to_string())],
                ],
            }
        );

        db.execute("INSERT INTO users VALUES ( 2, 'bob' )").unwrap();
        let outcome = db
            .execute("SELECT id FROM users WHERE name = 'alice'")
            .unwrap();
        assert_eq!(
            outcome.result,
            QueryResult::Select {
                columns: vec!["id".to_string()],
                rows: vec![vec![Value::Integer(1)]],
            }
        );
    }

    fn indexed_primary_keys(db: &SqlDatabase, secondary: &str) -> Vec<Value> {
        let (start, end) = index_entry_bounds(
            "users",
            "users_by_name",
            &Value::Text(secondary.to_string()),
        )
        .unwrap();
        db.store
            .snapshot()
            .unwrap()
            .scan(&start, &end)
            .unwrap()
            .into_iter()
            .map(|(key, _)| decode_index_entry(&key).unwrap().3)
            .collect()
    }

    #[test]
    fn duplicate_table_creation_is_rejected() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY )")
            .unwrap();
        let error = db
            .execute("CREATE TABLE users ( id INTEGER PRIMARY KEY )")
            .unwrap_err();

        assert!(matches!(
            error,
            SqlError::TableAlreadyExists { table } if table == "users"
        ));
    }

    #[test]
    fn prepared_statements_use_bound_primary_keys() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();

        let insert = db.prepare("INSERT INTO users VALUES ( ?, ? )").unwrap();
        db.execute_prepared(
            &insert,
            &[Value::Integer(7), Value::Text("alice".to_string())],
        )
        .unwrap();

        let select = db.prepare("SELECT * FROM users WHERE id = ?").unwrap();
        let result = db.execute_prepared(&select, &[Value::Integer(7)]).unwrap();

        if let ExecutionOutcome {
            result: QueryResult::Select { rows, .. },
            ..
        } = result
        {
            assert_eq!(
                rows,
                vec![vec![Value::Integer(7), Value::Text("alice".to_string())]]
            );
        } else {
            panic!("expected Select result");
        }
    }

    #[test]
    fn insert_into_missing_table_is_rejected() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        let error = db
            .execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap_err();
        assert!(matches!(error, SqlError::TableNotFound { table } if table == "users"));
    }

    #[test]
    fn named_projection_returns_only_requested_columns() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();

        let result = db.execute("SELECT name FROM users WHERE id = 1").unwrap();

        if let ExecutionOutcome {
            result: QueryResult::Select { columns, rows },
            ..
        } = result
        {
            assert_eq!(columns, vec!["name".to_string()]);
            assert_eq!(rows, vec![vec![Value::Text("alice".to_string())]]);
        } else {
            panic!("expected Select result");
        }
    }

    #[test]
    fn select_supports_pk_and_column_equality_filter() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();

        let result = db
            .execute("SELECT name FROM users WHERE id = 1 AND name = 'alice'")
            .unwrap();

        assert!(matches!(
            result.result,
            QueryResult::Select { rows, .. }
                if rows == vec![vec![Value::Text("alice".to_string())]]
        ));
    }

    #[test]
    fn select_and_filter_mismatch_returns_no_rows() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();

        let result = db
            .execute("SELECT name FROM users WHERE id = 1 AND name = 'bob'")
            .unwrap();

        assert!(matches!(
            result.result,
            QueryResult::Select { rows, .. } if rows.is_empty()
        ));
    }

    #[test]
    fn select_supports_not_equal_residual_filter() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();

        let result = db
            .execute("SELECT name FROM users WHERE id = 1 AND name != 'bob'")
            .unwrap();

        assert!(matches!(
            result.result,
            QueryResult::Select { rows, .. }
                if rows == vec![vec![Value::Text("alice".to_string())]]
        ));
    }

    #[test]
    fn not_equal_residual_filter_can_reject_row() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();

        let result = db
            .execute("SELECT name FROM users WHERE id = 1 AND name != 'alice'")
            .unwrap();

        assert!(matches!(
            result.result,
            QueryResult::Select { rows, .. } if rows.is_empty()
        ));
    }

    #[test]
    fn select_supports_ordered_residual_filters() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, age INTEGER )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 42 )").unwrap();

        for (operator, expected_rows) in [("<", 0), ("<=", 1), (">", 0), (">=", 1)] {
            let result = db
                .execute(&format!(
                    "SELECT age FROM users WHERE id = 1 AND age {operator} 42"
                ))
                .unwrap();
            let QueryResult::Select { rows, .. } = result.result else {
                panic!("expected Select result");
            };
            assert_eq!(rows.len(), expected_rows, "operator {operator}");
        }
    }

    #[test]
    fn ordered_filter_rejects_mixed_value_types() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, age INTEGER )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 42 )").unwrap();

        let error = db
            .execute("SELECT age FROM users WHERE id = 1 AND age < '42'")
            .unwrap_err();

        assert!(matches!(error, SqlError::UnsupportedQueryShape { .. }));
    }

    #[test]
    fn select_supports_or_primary_key_lookups() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 2, 'bob' )").unwrap();

        let result = db
            .execute("SELECT name FROM users WHERE id = 1 OR id = 2")
            .unwrap();

        assert!(matches!(
            result.result,
            QueryResult::Select { rows, .. }
                if rows == vec![
                    vec![Value::Text("alice".to_string())],
                    vec![Value::Text("bob".to_string())],
                ]
        ));
    }

    #[test]
    fn prepared_or_lookup_binds_each_primary_key() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 2, 'bob' )").unwrap();

        let statement = db
            .prepare("SELECT name FROM users WHERE id = ? OR id = ?")
            .unwrap();
        let result = db
            .execute_prepared(&statement, &[Value::Integer(2), Value::Integer(1)])
            .unwrap();

        assert!(matches!(
            result.result,
            QueryResult::Select { rows, .. }
                if rows == vec![
                    vec![Value::Text("bob".to_string())],
                    vec![Value::Text("alice".to_string())],
                ]
        ));
    }

    #[test]
    fn delete_supports_or_primary_key_lookups() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 2, 'bob' )").unwrap();

        let result = db
            .execute("DELETE FROM users WHERE id = 1 OR id = 2")
            .unwrap();

        assert!(matches!(result.result, QueryResult::Affected { rows: 2 }));
        let remaining = db.execute("SELECT * FROM users WHERE id = 1").unwrap();
        assert!(matches!(remaining.result, QueryResult::Select { rows, .. } if rows.is_empty()));
    }

    #[test]
    fn named_projection_preserves_requested_order() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT, note TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice', 'admin' )")
            .unwrap();

        let result = db
            .execute("SELECT note, id FROM users WHERE id = 1")
            .unwrap();

        assert!(matches!(
            result.result,
            QueryResult::Select { columns, rows }
                if columns == vec!["note", "id"]
                    && rows == vec![vec![Value::Text("admin".to_string()), Value::Integer(1)]]
        ));
    }

    #[test]
    fn unknown_projected_column_is_rejected() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();
        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY )")
            .unwrap();

        let error = db
            .execute("SELECT missing FROM users WHERE id = 1")
            .unwrap_err();

        assert!(matches!(
            error,
            SqlError::ColumnNotFound { table, column }
                if table == "users" && column == "missing"
        ));
    }

    #[test]
    fn duplicate_primary_key_overwrites_existing_row() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();
        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'bob' )").unwrap();

        let result = db.execute("SELECT name FROM users WHERE id = 1").unwrap();

        assert!(matches!(
            result.result,
            QueryResult::Select { rows, .. }
                if rows == vec![vec![Value::Text("bob".to_string())]]
        ));
    }

    #[test]
    fn select_star_surfaces_planner_warnings() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();
        db.execute("INSERT INTO users VALUES ( 1, 'alice' )")
            .unwrap();

        let result = db.execute("SELECT * FROM users WHERE id = 1").unwrap();
        assert_eq!(result.warnings.len(), 1);
    }

    #[test]
    fn explain_returns_plan_without_executing() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();
        db.execute("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )")
            .unwrap();

        let explained = db.explain("SELECT name FROM users WHERE id = ?").unwrap();

        assert_eq!(explained.plan, "PK_LOOKUP table=users");
        assert!(explained.warnings.is_empty());
        assert!(matches!(
            db.execute("SELECT name FROM users WHERE id = 1").unwrap().result,
            QueryResult::Select { rows, .. } if rows.is_empty()
        ));
    }

    #[test]
    fn select_nonexistent_key_returns_empty() {
        let (path, _dir) = test_db_path();
        let mut db = SqlDatabase::create(&path).unwrap();

        db.execute("CREATE TABLE t ( id INTEGER PRIMARY KEY )")
            .unwrap();

        let result = db.execute("SELECT * FROM t WHERE id = 999");
        assert!(result.is_ok());
        if let ExecutionOutcome {
            result: QueryResult::Select { rows, .. },
            ..
        } = result.unwrap()
        {
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
