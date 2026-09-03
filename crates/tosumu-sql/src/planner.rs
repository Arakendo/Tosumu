//! Planner for the tosumu initial SQL layer (MVP+9).
//!
//! Classifies statements into supported vs unsupported query shapes.
//! Uses catalog-aware PK resolution instead of string heuristics.

use crate::ast::{Expr, Projection, Stmt};
use crate::catalog::TableDef;
use crate::error::{SqlError, SqlResult};

/// Plan nodes that the baseline executor can execute.
pub enum PlanNode {
    /// CREATE TABLE — catalog write only
    CreateTable { table: String },
    /// CREATE INDEX with atomic backfill.
    CreateIndex {
        index: String,
        table: String,
        column: String,
    },
    /// INSERT row values in declaration order.
    InsertRow { table: String, values: Vec<Expr> },
    /// Point lookup by primary key expression.
    PkLookup {
        table: String,
        pk_expr: Expr,
        filter: Option<Expr>,
        projection: Projection,
    },
    /// Multiple point lookups joined by OR on the primary key.
    PkLookupMany {
        table: String,
        pk_exprs: Vec<Expr>,
        projection: Projection,
    },
    /// DELETE by primary key expression.
    DeleteByPk {
        table: String,
        pk_expr: Expr,
        filter: Option<Expr>,
    },
    /// DELETE multiple primary-key rows joined by OR.
    DeleteByPkMany { table: String, pk_exprs: Vec<Expr> },
}

impl PlanNode {
    /// Render the baseline plan shape for diagnostics and explain output.
    pub fn describe(&self) -> String {
        match self {
            Self::CreateTable { table } => format!("CREATE_TABLE table={table}"),
            Self::CreateIndex {
                index,
                table,
                column,
            } => format!("CREATE_INDEX index={index} table={table} column={column}"),
            Self::InsertRow { table, .. } => format!("INSERT_ROW table={table}"),
            Self::PkLookup { table, .. } => format!("PK_LOOKUP table={table}"),
            Self::PkLookupMany { table, .. } => format!("PK_LOOKUP_OR table={table}"),
            Self::DeleteByPk { table, .. } => format!("DELETE_BY_PK table={table}"),
            Self::DeleteByPkMany { table, .. } => format!("DELETE_BY_PK_OR table={table}"),
        }
    }
}

/// Planner warnings.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlanWarning {
    /// SELECT * was used (returns all columns).
    SelectStar { table: String },
}

/// Planner output with plan and any warnings.
pub struct PlanOutput {
    pub plan: PlanNode,
    pub warnings: Vec<PlanWarning>,
}

/// Query planner for the baseline SQL layer.
pub struct Planner;

impl Default for Planner {
    fn default() -> Self {
        Self::new()
    }
}

impl Planner {
    /// Create a new planner.
    pub fn new() -> Self {
        Planner
    }

    /// Plan a statement, returning the plan node and any warnings.
    pub fn plan(&self, stmt: &Stmt) -> SqlResult<PlanOutput> {
        self.plan_with_catalog(stmt, None)
    }

    /// Plan a statement with catalog context for PK-aware predicate validation.
    pub fn plan_with_catalog(
        &self,
        stmt: &Stmt,
        catalog: Option<&TableDef>,
    ) -> SqlResult<PlanOutput> {
        match stmt {
            Stmt::CreateTable { name, .. } => Ok(PlanOutput {
                plan: PlanNode::CreateTable {
                    table: name.clone(),
                },
                warnings: vec![],
            }),
            Stmt::CreateIndex {
                name,
                table,
                column,
            } => Ok(PlanOutput {
                plan: PlanNode::CreateIndex {
                    index: name.clone(),
                    table: table.clone(),
                    column: column.clone(),
                },
                warnings: vec![],
            }),
            Stmt::Insert { table, values } => {
                let pk_expr = match catalog {
                    Some(table_def) => values.get(table_def.primary_key_index),
                    None => values.first(),
                };

                match pk_expr {
                    Some(Expr::Literal(_)) | Some(Expr::Parameter(_)) => {}
                    _ => {
                        return Err(SqlError::unsupported_query_shape(
                            "INSERT primary key value must be a literal or parameter".to_string(),
                        ));
                    }
                }

                Ok(PlanOutput {
                    plan: PlanNode::InsertRow {
                        table: table.clone(),
                        values: values.clone(),
                    },
                    warnings: vec![],
                })
            }
            Stmt::Select {
                table,
                columns,
                predicate,
            } => {
                let mut warnings = vec![];

                if matches!(columns, Projection::All) {
                    warnings.push(PlanWarning::SelectStar {
                        table: table.clone(),
                    });
                }

                // Validate that WHERE clause exists
                if predicate.is_none() {
                    return Err(SqlError::unsupported_query_shape(
                        "baseline SQL requires WHERE clause with primary-key equality for SELECT"
                            .to_string(),
                    ));
                }

                if matches!(predicate, Some(Expr::Or(_, _))) {
                    let pk_exprs = Self::resolve_or_pk_predicate(predicate, catalog, "SELECT")?;
                    return Ok(PlanOutput {
                        plan: PlanNode::PkLookupMany {
                            table: table.clone(),
                            pk_exprs,
                            projection: columns.clone(),
                        },
                        warnings,
                    });
                }

                let (pk, filter) = Self::resolve_pk_from_predicate(predicate, catalog, "SELECT")?;

                Ok(PlanOutput {
                    plan: PlanNode::PkLookup {
                        table: table.clone(),
                        pk_expr: pk,
                        filter,
                        projection: columns.clone(),
                    },
                    warnings,
                })
            }
            Stmt::Delete { table, predicate } => {
                // Validate that WHERE clause exists
                if predicate.is_none() {
                    return Err(SqlError::unsupported_query_shape(
                        "baseline SQL requires WHERE clause with primary-key equality for DELETE"
                            .to_string(),
                    ));
                }

                if matches!(predicate, Some(Expr::Or(_, _))) {
                    let pk_exprs = Self::resolve_or_pk_predicate(predicate, catalog, "DELETE")?;
                    return Ok(PlanOutput {
                        plan: PlanNode::DeleteByPkMany {
                            table: table.clone(),
                            pk_exprs,
                        },
                        warnings: vec![],
                    });
                }

                let (pk, filter) = Self::resolve_pk_from_predicate(predicate, catalog, "DELETE")?;

                Ok(PlanOutput {
                    plan: PlanNode::DeleteByPk {
                        table: table.clone(),
                        pk_expr: pk,
                        filter,
                    },
                    warnings: vec![],
                })
            }
        }
    }

    /// Resolve the primary key value from a predicate, using catalog if available.
    fn resolve_pk_from_predicate(
        predicate: &Option<Expr>,
        catalog: Option<&TableDef>,
        operation: &str,
    ) -> SqlResult<(Expr, Option<Expr>)> {
        let pk_expr = match predicate {
            Some(Expr::Eq(left, right)) => (
                Self::validate_pk_equality(left, right, catalog, operation)?,
                None,
            ),
            Some(Expr::And(left, right)) => {
                let pk_expr = match left.as_ref() {
                    Expr::Eq(pk_left, pk_right) => {
                        Self::validate_pk_equality(pk_left, pk_right, catalog, operation)?
                    }
                    _ => return Err(Self::unsupported_predicate(operation)),
                };
                (pk_expr, Some(right.as_ref().clone()))
            }
            _ => return Err(Self::unsupported_predicate(operation)),
        };

        Ok(pk_expr)
    }

    fn resolve_or_pk_predicate(
        predicate: &Option<Expr>,
        catalog: Option<&TableDef>,
        operation: &str,
    ) -> SqlResult<Vec<Expr>> {
        let mut terms = Vec::new();
        let expression = predicate
            .as_ref()
            .ok_or_else(|| Self::unsupported_predicate(operation))?;
        Self::collect_or_terms(expression, &mut terms);
        if terms.len() < 2 {
            return Err(Self::unsupported_predicate(operation));
        }

        terms
            .into_iter()
            .map(|term| match term {
                Expr::Eq(left, right) => {
                    Self::validate_pk_equality(&left, &right, catalog, operation)
                }
                _ => Err(Self::unsupported_predicate(operation)),
            })
            .collect()
    }

    fn collect_or_terms(expression: &Expr, terms: &mut Vec<Expr>) {
        match expression {
            Expr::Or(left, right) => {
                Self::collect_or_terms(left, terms);
                Self::collect_or_terms(right, terms);
            }
            _ => terms.push(expression.clone()),
        }
    }

    fn validate_pk_equality(
        left: &Expr,
        right: &Expr,
        catalog: Option<&TableDef>,
        operation: &str,
    ) -> SqlResult<Expr> {
        let col_name = match left {
            Expr::Column(name) => name.clone(),
            _ => {
                return Err(SqlError::unsupported_query_shape(format!(
                    "expected column on left side of = for {operation}"
                )))
            }
        };

        // Use catalog to determine if this column is the PK, or fall back to heuristics
        let is_pk_column = match catalog {
            Some(table_def) => {
                // Check against actual schema
                table_def
                    .columns
                    .get(table_def.primary_key_index)
                    .map(|c| c.name == col_name)
                    .unwrap_or(false)
            }
            None => {
                // Fallback: use string heuristics only when no catalog available
                col_name == "id" || col_name.contains("pk") || col_name.contains("key")
            }
        };

        if !is_pk_column {
            return Err(SqlError::unsupported_query_shape(
                        format!("baseline SQL supports only primary-key equality lookups (column '{col_name}' is not the primary key)"),
                    ));
        }

        match right {
            Expr::Literal(_) | Expr::Parameter(_) => Ok(right.clone()),
            _ => Err(SqlError::unsupported_query_shape(format!(
                "baseline SQL supports only pk = ? or pk = <literal> predicates for {operation}"
            ))),
        }
    }

    fn unsupported_predicate(operation: &str) -> SqlError {
        SqlError::unsupported_query_shape(format!(
            "baseline SQL supports only pk = ? or pk = <literal> predicates, optionally followed by AND equality filters, for {operation}"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::Value;

    #[test]
    fn plan_create_table() {
        let planner = Planner::new();
        let stmt = Stmt::CreateTable {
            name: "users".to_string(),
            columns: vec![],
        };
        let output = planner.plan(&stmt).unwrap();
        assert!(matches!(output.plan, PlanNode::CreateTable { ref table } if table == "users"));
        assert!(output.warnings.is_empty());
    }

    #[test]
    fn plan_insert_with_literal_pk() {
        let planner = Planner::new();
        let stmt = Stmt::Insert {
            table: "users".to_string(),
            values: vec![Expr::Literal(Value::Integer(1))],
        };
        let output = planner.plan(&stmt).unwrap();
        assert!(
            matches!(output.plan, PlanNode::InsertRow { ref values, .. } if matches!(values.first(), Some(Expr::Literal(Value::Integer(1)))))
        );
    }

    #[test]
    fn plan_select_pk_lookup() {
        let planner = Planner::new();
        let stmt = Stmt::Select {
            table: "users".to_string(),
            columns: Projection::All,
            predicate: Some(Expr::Eq(
                Box::new(Expr::Column("id".to_string())),
                Box::new(Expr::Literal(Value::Integer(42))),
            )),
        };
        let output = planner.plan(&stmt).unwrap();
        assert!(matches!(
            output.plan,
            PlanNode::PkLookup {
                pk_expr: Expr::Literal(Value::Integer(42)),
                ..
            }
        ));
        assert_eq!(output.warnings.len(), 1); // SELECT * warning
    }

    #[test]
    fn plan_select_without_where_rejected() {
        let planner = Planner::new();
        let stmt = Stmt::Select {
            table: "users".to_string(),
            columns: Projection::All,
            predicate: None,
        };
        assert!(planner.plan(&stmt).is_err());
    }

    #[test]
    fn plan_select_non_pk_column_rejected() {
        let planner = Planner::new();
        let stmt = Stmt::Select {
            table: "users".to_string(),
            columns: Projection::All,
            predicate: Some(Expr::Eq(
                Box::new(Expr::Column("email".to_string())),
                Box::new(Expr::Literal(Value::Text("a@b.com".to_string()))),
            )),
        };
        assert!(planner.plan(&stmt).is_err());
    }

    #[test]
    fn plan_delete_by_pk() {
        let planner = Planner::new();
        let stmt = Stmt::Delete {
            table: "users".to_string(),
            predicate: Some(Expr::Eq(
                Box::new(Expr::Column("id".to_string())),
                Box::new(Expr::Literal(Value::Integer(1))),
            )),
        };
        let output = planner.plan(&stmt).unwrap();
        assert!(matches!(
            output.plan,
            PlanNode::DeleteByPk {
                pk_expr: Expr::Literal(Value::Integer(1)),
                ..
            }
        ));
    }

    #[test]
    fn plan_delete_without_where_rejected() {
        let planner = Planner::new();
        let stmt = Stmt::Delete {
            table: "users".to_string(),
            predicate: None,
        };
        assert!(planner.plan(&stmt).is_err());
    }

    #[test]
    fn plan_insert_empty_values_rejected() {
        let planner = Planner::new();
        let stmt = Stmt::Insert {
            table: "users".to_string(),
            values: vec![],
        };
        assert!(planner.plan(&stmt).is_err());
    }
}
