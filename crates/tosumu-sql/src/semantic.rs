//! Semantic checker for the tosumu toy SQL layer (MVP+9).
//!
//! Validates statements against a catalog before execution.

use crate::error::{SqlError, SqlResult};
use crate::ast::{DataType, Stmt};
use crate::catalog::TableDef;

/// Catalog trait for semantic checking.
pub trait Catalog {
    fn get_table(&self, name: &str) -> Option<TableDef>;
    fn table_exists(&self, name: &str) -> bool;
}

/// Semantic checker.
pub struct SemanticChecker<C: Catalog> {
    catalog: C,
}

impl<C: Catalog> SemanticChecker<C> {
    /// Create a new semantic checker.
    pub fn new(catalog: C) -> Self {
        SemanticChecker { catalog }
    }

    /// Validate a CREATE TABLE statement.
    pub fn check_create_table(&self, stmt: &Stmt) -> SqlResult<()> {
        if let Stmt::CreateTable { name, columns } = stmt {
            // Check for duplicate column names
            let mut seen_names = std::collections::HashSet::new();
            let mut has_pk = false;
            for col in columns {
                if !seen_names.insert(&col.name) {
                    return Err(SqlError::DuplicateColumn {
                        table: name.clone(),
                        column: col.name.clone(),
                    });
                }
                if col.is_primary_key {
                    has_pk = true;
                }
                // Check supported types (parser already enforces this, but be defensive)
                match col.data_type {
                    DataType::Integer | DataType::Text | DataType::Blob => {}
                }
            }
            if !has_pk {
                return Err(SqlError::MissingPrimaryKey { table: name.clone() });
            }
        }
        Ok(())
    }

    /// Validate an INSERT statement.
    pub fn check_insert(&self, stmt: &Stmt) -> SqlResult<()> {
        if let Stmt::Insert { values, .. } = stmt {
            if values.is_empty() {
                return Err(SqlError::unsupported_query_shape(
                    "INSERT requires at least one value".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Validate a SELECT statement.
    pub fn check_select(&self, stmt: &Stmt) -> SqlResult<()> {
        if let Stmt::Select { predicate, .. } = stmt {
            if predicate.is_none() {
                return Err(SqlError::unsupported_query_shape(
                    "baseline SQL requires WHERE clause with primary-key equality for SELECT".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Validate a DELETE statement.
    pub fn check_delete(&self, stmt: &Stmt) -> SqlResult<()> {
        if let Stmt::Delete { predicate, .. } = stmt {
            if predicate.is_none() {
                return Err(SqlError::unsupported_query_shape(
                    "baseline SQL requires WHERE clause with primary-key equality for DELETE".to_string(),
                ));
            }
        }
        Ok(())
    }

    /// Validate that an INSERT value list matches the table schema.
    pub fn check_insert_against_schema(&self, stmt: &Stmt, table_def: &TableDef) -> SqlResult<()> {
        if let Stmt::Insert { values, .. } = stmt {
            if values.len() != table_def.columns.len() {
                return Err(SqlError::TypeMismatch {
                    table: table_def.name.clone(),
                    column: "VALUES".to_string(),
                    expected: format!("{}", table_def.columns.len()),
                    got: format!("{}", values.len()),
                });
            }
        }
        Ok(())
    }

    /// Resolve the primary key column name from a table definition.
    pub fn pk_column_name<'a>(&self, table_def: &'a TableDef) -> Option<&'a str> {
        table_def.columns.get(table_def.primary_key_index).map(|c| c.name.as_str())
    }

    /// Check that a table exists in the catalog.
    pub fn ensure_table_exists(&self, table: &str) -> SqlResult<TableDef> {
        self.catalog.get_table(table)
            .ok_or_else(|| SqlError::table_not_found(table))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Value;

    struct EmptyCatalog;

    impl Catalog for EmptyCatalog {
        fn get_table(&self, _name: &str) -> Option<TableDef> { None }
        fn table_exists(&self, _name: &str) -> bool { false }
    }

    #[test]
    fn check_create_table_always_ok() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::CreateTable {
            name: "users".to_string(),
            columns: vec![crate::ast::ColumnDef {
                name: "id".to_string(),
                data_type: DataType::Integer,
                is_primary_key: true,
            }],
        };
        assert!(checker.check_create_table(&stmt).is_ok());
    }

    #[test]
    fn check_insert_empty_values_rejected() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::Insert {
            table: "users".to_string(),
            values: vec![],
        };
        assert!(checker.check_insert(&stmt).is_err());
    }

    #[test]
    fn check_insert_nonempty_values_ok() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::Insert {
            table: "users".to_string(),
            values: vec![crate::ast::Expr::Literal(Value::Integer(1))],
        };
        assert!(checker.check_insert(&stmt).is_ok());
    }

    #[test]
    fn check_select_without_where_rejected() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::Select {
            table: "users".to_string(),
            columns: crate::ast::Projection::All,
            predicate: None,
        };
        assert!(checker.check_select(&stmt).is_err());
    }

    #[test]
    fn check_select_with_where_ok() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::Select {
            table: "users".to_string(),
            columns: crate::ast::Projection::All,
            predicate: Some(crate::ast::Expr::Eq(
                Box::new(crate::ast::Expr::Column("id".to_string())),
                Box::new(crate::ast::Expr::Literal(Value::Integer(1))),
            )),
        };
        assert!(checker.check_select(&stmt).is_ok());
    }

    #[test]
    fn check_delete_without_where_rejected() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::Delete {
            table: "users".to_string(),
            predicate: None,
        };
        assert!(checker.check_delete(&stmt).is_err());
    }

    #[test]
    fn check_delete_with_where_ok() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::Delete {
            table: "users".to_string(),
            predicate: Some(crate::ast::Expr::Eq(
                Box::new(crate::ast::Expr::Column("id".to_string())),
                Box::new(crate::ast::Expr::Literal(Value::Integer(1))),
            )),
        };
        assert!(checker.check_delete(&stmt).is_ok());
    }

    #[test]
    fn ensure_table_not_found() {
        let checker = SemanticChecker::new(EmptyCatalog);
        assert!(checker.ensure_table_exists("users").is_err());
    }

    #[test]
    fn check_type_mismatch_detection() {
        // Type checking would validate that values match column types.
        // For baseline, we just check non-empty for INSERT.
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::Insert {
            table: "users".to_string(),
            values: vec![crate::ast::Expr::Literal(Value::Integer(42))],
        };
        assert!(checker.check_insert(&stmt).is_ok());
    }

    #[test]
    fn check_create_table_duplicate_column_rejected() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::CreateTable {
            name: "users".to_string(),
            columns: vec![
                crate::ast::ColumnDef { name: "id".to_string(), data_type: DataType::Integer, is_primary_key: true },
                crate::ast::ColumnDef { name: "id".to_string(), data_type: DataType::Text, is_primary_key: false },
            ],
        };
        assert!(checker.check_create_table(&stmt).is_err());
    }

    #[test]
    fn check_create_table_no_pk_rejected() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::CreateTable {
            name: "users".to_string(),
            columns: vec![
                crate::ast::ColumnDef { name: "name".to_string(), data_type: DataType::Text, is_primary_key: false },
            ],
        };
        assert!(checker.check_create_table(&stmt).is_err());
    }

    #[test]
    fn check_insert_against_schema_wrong_count() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::Insert {
            table: "users".to_string(),
            values: vec![crate::ast::Expr::Literal(Value::Integer(1))],
        };
        let table_def = TableDef {
            name: "users".to_string(),
            columns: vec![
                crate::ast::ColumnDef { name: "id".to_string(), data_type: DataType::Integer, is_primary_key: true },
                crate::ast::ColumnDef { name: "name".to_string(), data_type: DataType::Text, is_primary_key: false },
            ],
            primary_key_index: 0,
            root_page: None,
        };
        assert!(checker.check_insert_against_schema(&stmt, &table_def).is_err());
    }

    #[test]
    fn check_insert_against_schema_correct_count() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let stmt = Stmt::Insert {
            table: "users".to_string(),
            values: vec![
                crate::ast::Expr::Literal(Value::Integer(1)),
                crate::ast::Expr::Literal(Value::Text("alice".to_string())),
            ],
        };
        let table_def = TableDef {
            name: "users".to_string(),
            columns: vec![
                crate::ast::ColumnDef { name: "id".to_string(), data_type: DataType::Integer, is_primary_key: true },
                crate::ast::ColumnDef { name: "name".to_string(), data_type: DataType::Text, is_primary_key: false },
            ],
            primary_key_index: 0,
            root_page: None,
        };
        assert!(checker.check_insert_against_schema(&stmt, &table_def).is_ok());
    }

    #[test]
    fn pk_column_name_returns_pk() {
        let checker = SemanticChecker::new(EmptyCatalog);
        let table_def = TableDef {
            name: "users".to_string(),
            columns: vec![
                crate::ast::ColumnDef { name: "id".to_string(), data_type: DataType::Integer, is_primary_key: true },
                crate::ast::ColumnDef { name: "name".to_string(), data_type: DataType::Text, is_primary_key: false },
            ],
            primary_key_index: 0,
            root_page: None,
        };
        assert_eq!(checker.pk_column_name(&table_def), Some("id"));
    }
}
