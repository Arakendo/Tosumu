use std::path::Path;

use crate::error_boundary::CliError;
use tosumu_sql::{QueryResult, SqlDatabase, Value};

pub(crate) fn run_sql(
    path: &Path,
    query: &str,
    explain: bool,
    raw_params: &[String],
) -> Result<(), CliError> {
    let mut database = SqlDatabase::open(path)?;

    if explain {
        let outcome = database.explain(query)?;
        println!("plan: {}", outcome.plan);
        for warning in outcome.warnings {
            eprintln!("warning: {warning:?}");
        }
        return Ok(());
    }

    let params = raw_params
        .iter()
        .map(|value| parse_param(value))
        .collect::<Result<Vec<_>, _>>()?;
    let statement = database.prepare(query)?;
    let outcome = database.execute_prepared(&statement, &params)?;

    for warning in outcome.warnings {
        eprintln!("warning: {warning:?}");
    }

    println!("{}", render_result(&outcome.result));

    Ok(())
}

fn parse_param(value: &str) -> Result<Value, CliError> {
    if let Ok(integer) = value.parse::<i64>() {
        return Ok(Value::Integer(integer));
    }

    Ok(Value::Text(value.to_string()))
}

fn render_result(result: &QueryResult) -> String {
    match result {
        QueryResult::Affected { rows } => format!("{rows} row(s) affected"),
        QueryResult::Select { columns, rows } => {
            let mut output = format!(
                "{}\n{}",
                columns.join(" | "),
                columns
                    .iter()
                    .map(|column| "-".repeat(column.len()))
                    .collect::<Vec<_>>()
                    .join("-+-")
            );

            for row in rows {
                output.push('\n');
                output.push_str(
                    &row.iter()
                        .map(Value::to_sql_literal)
                        .collect::<Vec<_>>()
                        .join(" | "),
                );
            }

            output
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_param, render_result};
    use tosumu_sql::{QueryResult, Value};

    #[test]
    fn renders_select_rows_as_a_table() {
        let result = QueryResult::Select {
            columns: vec!["id".to_string(), "name".to_string()],
            rows: vec![vec![Value::Integer(1), Value::Text("alice".to_string())]],
        };

        assert_eq!(render_result(&result), "id | name\n---+-----\n1 | 'alice'");
    }

    #[test]
    fn renders_affected_row_count() {
        assert_eq!(
            render_result(&QueryResult::Affected { rows: 1 }),
            "1 row(s) affected"
        );
    }

    #[test]
    fn integer_parameters_are_bound_as_integers() {
        assert_eq!(parse_param("42").unwrap(), Value::Integer(42));
    }

    #[test]
    fn non_integer_parameters_are_bound_as_text() {
        assert_eq!(
            parse_param("alice").unwrap(),
            Value::Text("alice".to_string())
        );
    }
}
