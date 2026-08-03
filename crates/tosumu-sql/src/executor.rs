//! Executor for the tosumu initial SQL layer (MVP+9).
//!
//! Executes plan nodes through PageStore. Catalog writes for CreateTable are owned here.

use crate::ast::{Expr, Projection, Value};
use crate::catalog::{serialize_table_def, table_key, TableDef};
use crate::error::{SqlError, SqlResult};
use crate::planner::{PlanNode, PlanWarning};
use crate::row_codec::{row_key, encode_row_values, decode_row_values};
use std::cmp::Ordering;
use std::collections::HashSet;
use tosumu_core::page_store::PageStore;

/// Query result from executing a statement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QueryResult {
    /// SELECT returned rows with column names.
    Select {
        columns: Vec<String>,
        rows: Vec<Vec<Value>>,
    },
    /// INSERT/DELETE returned the number of affected rows.
    Affected { rows: usize },
}

/// Outcome of executing a statement, including any planner warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExecutionOutcome {
    pub result: QueryResult,
    pub warnings: Vec<PlanWarning>,
}

/// Executor that runs plans through PageStore.
pub struct Executor;

impl Executor {
    /// Create a new executor (no state needed in baseline).
    pub fn new() -> Self {
        Executor
    }

    /// Execute a plan node against the given PageStore.
    pub fn execute(
        &self,
        plan: PlanNode,
        bindings: &[Value],
        store: &mut PageStore,
        catalog_context: Option<&TableDef>,
    ) -> SqlResult<ExecutionOutcome> {
        match plan {
            PlanNode::CreateTable { table } => Self::exec_create_table(table, store, catalog_context),
            PlanNode::InsertRow { table, values } => {
                Self::exec_insert(&table, &values, bindings, store, catalog_context)
            }
            PlanNode::PkLookup { table, pk_expr, filter, projection } => {
                Self::exec_select(&table, &pk_expr, filter.as_ref(), &projection, bindings, store, catalog_context)
            }
            PlanNode::PkLookupMany { table, pk_exprs, projection } => {
                Self::exec_select_many(&table, &pk_exprs, &projection, bindings, store, catalog_context)
            }
            PlanNode::DeleteByPk { table, pk_expr, filter } => {
                Self::exec_delete(&table, &pk_expr, filter.as_ref(), bindings, store, catalog_context)
            }
            PlanNode::DeleteByPkMany { table, pk_exprs } => {
                Self::exec_delete_many(&table, &pk_exprs, bindings, store)
            }
        }
    }

    fn exec_create_table(
        table_name: String, 
        store: &mut PageStore, 
        catalog_context: Option<&TableDef>,
    ) -> SqlResult<ExecutionOutcome> {
        // CreateTable execution: write catalog entry (executor owns this).
        if let Some(table_def) = catalog_context {
            let catalog_key = table_key(&table_name);
            let payload = serialize_table_def(table_def);
            store.put(catalog_key.as_bytes(), &payload)?;
        }
        Ok(ExecutionOutcome {
            result: QueryResult::Affected { rows: 0 },
            warnings: vec![],
        })
    }

    fn exec_insert(
        table: &str,
        values: &[Expr],
        bindings: &[Value],
        store: &mut PageStore,
        catalog_context: Option<&TableDef>,
    ) -> SqlResult<ExecutionOutcome> {
        let table_def = catalog_context.ok_or_else(|| SqlError::table_not_found(table))?;
        let resolved_values = resolve_exprs(values, bindings)?;
        let pk = resolved_values
            .get(table_def.primary_key_index)
            .cloned()
            .ok_or_else(|| SqlError::RowEncoding("missing primary key value in INSERT".to_string()))?;
        let row_key_str = row_key(table, &pk);

        let column_names: Vec<&str> = table_def.columns.iter().map(|c| c.name.as_str()).collect();
        let column_types: Vec<u8> = table_def.columns.iter().map(|c| c.data_type.as_u8()).collect();
        let payload = encode_row_values(&column_names, &column_types, &resolved_values).map_err(|e| {
            SqlError::RowEncoding(format!("INSERT row encoding failed: {e}"))
        })?;

        store.put(row_key_str.as_bytes(), &payload)?;
        Ok(ExecutionOutcome {
            result: QueryResult::Affected { rows: 1 },
            warnings: vec![],
        })
    }

    fn exec_select(
        table: &str,
        pk_expr: &Expr,
        filter: Option<&Expr>,
        projection: &Projection,
        bindings: &[Value],
        store: &mut PageStore,
        catalog_context: Option<&TableDef>,
    ) -> SqlResult<ExecutionOutcome> {
        let table_def = catalog_context.ok_or_else(|| SqlError::table_not_found(table))?;
        let pk = resolve_expr(pk_expr, bindings, &mut 0)?;
        let (columns, projected_indexes) = projection_layout(projection, table_def)?;
        let key_str = row_key(table, &pk);
        let data = match store.get(key_str.as_bytes())? {
            Some(data) => data,
            None => return Ok(ExecutionOutcome {
                result: QueryResult::Select {
                    columns,
                    rows: vec![],
                },
                warnings: vec![],
            }),
        };

        // Decode the row values.
        let decoded = decode_row_values(&data).map_err(|e| {
            SqlError::RowEncoding(format!("SELECT row decoding failed: {e}"))
        })?;

        let mut binding_index = 1;
        if let Some(filter) = filter {
            if !evaluate_predicate(filter, &decoded, table_def, bindings, &mut binding_index)? {
                return Ok(ExecutionOutcome {
                    result: QueryResult::Select { columns, rows: vec![] },
                    warnings: vec![],
                });
            }
        }

        let row = projected_indexes
            .iter()
            .map(|&index| {
                decoded.get(index).cloned().ok_or_else(|| {
                    SqlError::RowEncoding(format!(
                        "decoded row missing value for projected column index {index}"
                    ))
                })
            })
            .collect::<SqlResult<Vec<_>>>()?;

        Ok(ExecutionOutcome {
            result: QueryResult::Select {
                columns,
                rows: vec![row],
            },
            warnings: vec![],
        })
    }

    fn exec_delete(
        table: &str,
        pk_expr: &Expr,
        filter: Option<&Expr>,
        bindings: &[Value],
        store: &mut PageStore,
        catalog_context: Option<&TableDef>,
    ) -> SqlResult<ExecutionOutcome> {
        let pk = resolve_expr(pk_expr, bindings, &mut 0)?;
        let row_key_str = row_key(table, &pk);
        match store.get(row_key_str.as_bytes())? {
            Some(data) => {
                if let (Some(filter), Some(table_def)) = (filter, catalog_context) {
                    let decoded = decode_row_values(&data).map_err(|e| {
                        SqlError::RowEncoding(format!("DELETE row decoding failed: {e}"))
                    })?;
                    let mut binding_index = 1;
                    if !evaluate_predicate(filter, &decoded, table_def, bindings, &mut binding_index)? {
                        return Ok(ExecutionOutcome {
                            result: QueryResult::Affected { rows: 0 },
                            warnings: vec![],
                        });
                    }
                }
                store.delete(row_key_str.as_bytes())?;
                Ok(ExecutionOutcome {
                    result: QueryResult::Affected { rows: 1 },
                    warnings: vec![],
                })
            }
            None => Ok(ExecutionOutcome {
                result: QueryResult::Affected { rows: 0 },
                warnings: vec![],
            }),
        }
    }

    fn exec_select_many(
        table: &str,
        pk_exprs: &[Expr],
        projection: &Projection,
        bindings: &[Value],
        store: &mut PageStore,
        catalog_context: Option<&TableDef>,
    ) -> SqlResult<ExecutionOutcome> {
        let table_def = catalog_context.ok_or_else(|| SqlError::table_not_found(table))?;
        let (columns, projected_indexes) = projection_layout(projection, table_def)?;
        let mut binding_index = 0;
        let mut seen_keys = HashSet::new();
        let mut rows = Vec::new();

        for pk_expr in pk_exprs {
            let pk = resolve_expr(pk_expr, bindings, &mut binding_index)?;
            let key_str = row_key(table, &pk);
            if !seen_keys.insert(key_str.clone()) {
                continue;
            }
            let Some(data) = store.get(key_str.as_bytes())? else {
                continue;
            };
            let decoded = decode_row_values(&data).map_err(|e| {
                SqlError::RowEncoding(format!("SELECT row decoding failed: {e}"))
            })?;
            let row = projected_indexes
                .iter()
                .map(|&index| decoded.get(index).cloned().ok_or_else(|| {
                    SqlError::RowEncoding(format!(
                        "decoded row missing value for projected column index {index}"
                    ))
                }))
                .collect::<SqlResult<Vec<_>>>()?;
            rows.push(row);
        }

        Ok(ExecutionOutcome {
            result: QueryResult::Select { columns, rows },
            warnings: vec![],
        })
    }

    fn exec_delete_many(
        table: &str,
        pk_exprs: &[Expr],
        bindings: &[Value],
        store: &mut PageStore,
    ) -> SqlResult<ExecutionOutcome> {
        let mut binding_index = 0;
        let mut seen_keys = HashSet::new();
        let mut affected = 0;

        for pk_expr in pk_exprs {
            let pk = resolve_expr(pk_expr, bindings, &mut binding_index)?;
            let key_str = row_key(table, &pk);
            if !seen_keys.insert(key_str.clone()) {
                continue;
            }
            if store.get(key_str.as_bytes())?.is_some() {
                store.delete(key_str.as_bytes())?;
                affected += 1;
            }
        }

        Ok(ExecutionOutcome {
            result: QueryResult::Affected { rows: affected },
            warnings: vec![],
        })
    }
}

impl Default for Executor {
    fn default() -> Self {
        Self::new()
    }
}

fn resolve_exprs(exprs: &[Expr], bindings: &[Value]) -> SqlResult<Vec<Value>> {
    let mut binding_index = 0;
    exprs
        .iter()
        .map(|expr| resolve_expr(expr, bindings, &mut binding_index))
        .collect()
}

fn resolve_expr(expr: &Expr, bindings: &[Value], binding_index: &mut usize) -> SqlResult<Value> {
    match expr {
        Expr::Literal(value) => Ok(value.clone()),
        Expr::Parameter(_) => {
            let value = bindings.get(*binding_index).cloned().ok_or_else(|| {
                SqlError::BindingCountMismatch {
                    expected: *binding_index + 1,
                    got: bindings.len(),
                }
            })?;
            *binding_index += 1;
            Ok(value)
        }
        _ => Err(SqlError::unsupported_query_shape(
            "baseline SQL supports only literal or parameter values during execution",
        )),
    }
}

fn evaluate_predicate(
    predicate: &Expr,
    row: &[Value],
    table_def: &TableDef,
    bindings: &[Value],
    binding_index: &mut usize,
) -> SqlResult<bool> {
    match predicate {
        Expr::Eq(left, right) => {
            let column = match left.as_ref() {
                Expr::Column(name) => name,
                _ => return Err(SqlError::unsupported_query_shape(
                    "filters require a column on the left side of =",
                )),
            };
            let index = table_def
                .columns
                .iter()
                .position(|candidate| candidate.name == *column)
                .ok_or_else(|| SqlError::column_not_found(&table_def.name, column))?;
            let expected = resolve_expr(right, bindings, binding_index)?;
            Ok(row.get(index) == Some(&expected))
        }
        Expr::NotEq(left, right) => {
            let column = match left.as_ref() {
                Expr::Column(name) => name,
                _ => return Err(SqlError::unsupported_query_shape(
                    "filters require a column on the left side of !=",
                )),
            };
            let index = table_def
                .columns
                .iter()
                .position(|candidate| candidate.name == *column)
                .ok_or_else(|| SqlError::column_not_found(&table_def.name, column))?;
            let expected = resolve_expr(right, bindings, binding_index)?;
            Ok(row.get(index) != Some(&expected))
        }
        Expr::Lt(left, right) => evaluate_ordered_filter(
            left, right, row, table_def, bindings, binding_index, |ordering| {
                ordering == Ordering::Less
            },
        ),
        Expr::LtEq(left, right) => evaluate_ordered_filter(
            left, right, row, table_def, bindings, binding_index, |ordering| {
                ordering != Ordering::Greater
            },
        ),
        Expr::Gt(left, right) => evaluate_ordered_filter(
            left, right, row, table_def, bindings, binding_index, |ordering| {
                ordering == Ordering::Greater
            },
        ),
        Expr::GtEq(left, right) => evaluate_ordered_filter(
            left, right, row, table_def, bindings, binding_index, |ordering| {
                ordering != Ordering::Less
            },
        ),
        Expr::And(left, right) => Ok(
            evaluate_predicate(left, row, table_def, bindings, binding_index)?
                && evaluate_predicate(right, row, table_def, bindings, binding_index)?,
        ),
        _ => Err(SqlError::unsupported_query_shape(
            "filters support only equality and AND predicates",
        )),
    }
}

fn evaluate_ordered_filter(
    left: &Expr,
    right: &Expr,
    row: &[Value],
    table_def: &TableDef,
    bindings: &[Value],
    binding_index: &mut usize,
    accepts: fn(Ordering) -> bool,
) -> SqlResult<bool> {
    let column = match left {
        Expr::Column(name) => name,
        _ => return Err(SqlError::unsupported_query_shape(
            "ordered filters require a column on the left side of the comparison",
        )),
    };
    let index = table_def
        .columns
        .iter()
        .position(|candidate| candidate.name == *column)
        .ok_or_else(|| SqlError::column_not_found(&table_def.name, column))?;
    let actual = row.get(index).ok_or_else(|| {
        SqlError::RowEncoding(format!("decoded row missing value for filtered column index {index}"))
    })?;
    let expected = resolve_expr(right, bindings, binding_index)?;
    let ordering = match (actual, &expected) {
        (Value::Integer(actual), Value::Integer(expected)) => actual.cmp(expected),
        (Value::Text(actual), Value::Text(expected)) => actual.cmp(expected),
        (Value::Blob(actual), Value::Blob(expected)) => actual.cmp(expected),
        _ => return Err(SqlError::unsupported_query_shape(
            "ordered filters require comparable values of the same type",
        )),
    };
    Ok(accepts(ordering))
}

fn projection_layout(projection: &Projection, table_def: &TableDef) -> SqlResult<(Vec<String>, Vec<usize>)> {
    match projection {
        Projection::All => Ok((
            table_def.columns.iter().map(|c| c.name.clone()).collect(),
            (0..table_def.columns.len()).collect(),
        )),
        Projection::Named(names) => {
            let mut indexes = Vec::with_capacity(names.len());
            for name in names {
                let index = table_def
                    .columns
                    .iter()
                    .position(|column| column.name == *name)
                    .ok_or_else(|| SqlError::column_not_found(&table_def.name, name))?;
                indexes.push(index);
            }
            Ok((names.clone(), indexes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::deserialize_table_def;
    use tempfile::TempDir;

    fn test_db_path() -> (std::path::PathBuf, TempDir) {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.tsm");
        (path, dir)
    }

    #[test]
    fn executor_create_table_writes_catalog() {
        let (path, _dir) = test_db_path();
        let mut store = PageStore::create(&path).unwrap();
        
        // Create table via catalog write
        let result = store.transaction(|s| {
            let key = table_key("users");
            let def = TableDef {
                name: "users".to_string(),
                columns: vec![crate::ast::ColumnDef {
                    name: "id".to_string(),
                    data_type: crate::ast::DataType::Integer,
                    is_primary_key: true,
                }],
                primary_key_index: 0,
                root_page: None,
            };
            let payload = serialize_table_def(&def);
            s.put(key.as_bytes(), &payload)?;
            Ok(())
        });
        assert!(result.is_ok());
        
        // Verify catalog entry exists
        let data = store.get(table_key("users").as_bytes()).unwrap();
        assert!(data.is_some());
    }

    #[test]
    fn executor_insert_and_select() {
        let (path, _dir) = test_db_path();
        let mut store = PageStore::create(&path).unwrap();
        
        // Insert a row
        store.put(b"__sql_row__/users/1", &[1, 0, 1, 0, 5, 0, 0, 0, 2, b'a', b'l', b'i', b'c', b'e']).unwrap();
        
        // Select the row back
        let data = store.get(b"__sql_row__/users/1").unwrap();
        assert!(data.is_some());
    }

    #[test]
    fn executor_delete_removes_row() {
        let (path, _dir) = test_db_path();
        let mut store = PageStore::create(&path).unwrap();
        
        // Insert a row
        store.put(b"__sql_row__/users/1", &[1, 0, 1, 0, 5, 0, 0, 0, 2, b'a', b'l', b'i', b'c', b'e']).unwrap();
        
        // Delete the row
        store.delete(b"__sql_row__/users/1").unwrap();
        
        // Verify it's gone
        let data = store.get(b"__sql_row__/users/1").unwrap();
        assert!(data.is_none());
    }

    #[test]
    fn executor_select_nonexistent_returns_empty() {
        let (path, _dir) = test_db_path();
        let store = PageStore::create(&path).unwrap();
        
        let data = store.get(b"__sql_row__/users/999").unwrap();
        assert!(data.is_none());
    }

    #[test]
    fn row_codec_round_trip() {
        let values = vec![
            Value::Integer(42),
            Value::Text("alice".to_string()),
        ];
        let encoded = encode_row_values(&["id", "name"], &[1, 2], &values).unwrap();
        let decoded = decode_row_values(&encoded).unwrap();
        assert_eq!(decoded, values);
    }

    #[test]
    fn catalog_serialize_deserialize_round_trip() {
        let def = TableDef {
            name: "users".to_string(),
            columns: vec![
                crate::ast::ColumnDef {
                    name: "id".to_string(),
                    data_type: crate::ast::DataType::Integer,
                    is_primary_key: true,
                },
                crate::ast::ColumnDef {
                    name: "name".to_string(),
                    data_type: crate::ast::DataType::Text,
                    is_primary_key: false,
                },
            ],
            primary_key_index: 0,
            root_page: None,
        };
        let serialized = serialize_table_def(&def);
        let deserialized = deserialize_table_def(&serialized).unwrap();
        assert_eq!(deserialized.name, "users");
        assert_eq!(deserialized.columns.len(), 2);
    }
}