//! SQL parser for the tosumu toy SQL layer (MVP+9).
//!
//! Recursive descent parser for the baseline grammar: CREATE TABLE, INSERT, SELECT, DELETE.

use crate::error::{SqlError, SqlResult};
use crate::lexer::{Lexer, Token, TokenKind};
use crate::ast::{Stmt, Expr, Projection, Value, DataType, ColumnDef};

/// Parse SQL input into a statement AST.
pub fn parse(sql: &str) -> SqlResult<Stmt> {
    let mut lexer = Lexer::new(sql);
    let tokens = lexer.tokenize()?;
    let mut parser = ParserInner { tokens, pos: 0 };
    let stmt = parser.parse_statement()?;
    parser.expect_eof()?;
    Ok(stmt)
}

/// Internal parser state.
struct ParserInner {
    tokens: Vec<Token>,
    pos: usize,
}

impl ParserInner {
    fn current(&self) -> SqlResult<Token> {
        self.tokens.get(self.pos).cloned().ok_or_else(|| {
            SqlError::parse_error("unexpected end of input".to_string(), 0, 0)
        })
    }

    fn peek_kind(&self) -> SqlResult<TokenKind> {
        Ok(self.current()?.kind.clone())
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens.get(self.pos).cloned().unwrap();
        self.pos += 1;
        token
    }

    fn expect_kind(&mut self, kind: TokenKind) -> SqlResult<Token> {
        let current = self.advance();
        if current.kind == kind {
            Ok(current)
        } else {
            let msg = format!("expected {}, found {:?}", 
                format_token_kind(&kind), 
                format_token_kind(&current.kind));
            Err(SqlError::parse_error(msg, 0, 0))
        }
    }

    fn match_kind(&mut self, kind: TokenKind) -> Option<Token> {
        if self.peek_kind().is_ok_and(|k| k == kind) {
            Some(self.advance())
        } else {
            None
        }
    }

    fn parse_statement(&mut self) -> SqlResult<Stmt> {
        match self.peek_kind()? {
            TokenKind::Create => self.parse_create_table(),
            TokenKind::Insert => self.parse_insert(),
            TokenKind::Select => self.parse_select_stmt(),
            TokenKind::Delete => self.parse_delete_stmt(),
            TokenKind::PrimaryKey => {
                self.advance();
                self.parse_statement()
            }
            _ => {
                let msg = format!("unexpected token: {:?}", self.peek_kind()?);
                Err(SqlError::parse_error(msg, 0, 0))
            }
        }
    }

    fn parse_create_table(&mut self) -> SqlResult<Stmt> {
        self.expect_kind(TokenKind::Create)?;
        self.expect_kind(TokenKind::Table)?;
        let name = match self.peek_kind()? {
            TokenKind::Ident(s) => { self.advance(); s }
            _ => return Err(SqlError::parse_error("expected table name".to_string(), 0, 0)),
        };

        self.expect_kind(TokenKind::LParen)?;
        let mut columns = Vec::new();

        let col = self.parse_column_def()?;
        columns.push(col);

        while self.match_kind(TokenKind::Comma).is_some() {
            let col = self.parse_column_def()?;
            columns.push(col);
        }

        self.expect_kind(TokenKind::RParen)?;

        Ok(Stmt::CreateTable { name, columns })
    }

    fn parse_column_def(&mut self) -> SqlResult<ColumnDef> {
        let name = match self.peek_kind()? {
            TokenKind::Ident(s) => { self.advance(); s }
            _ => return Err(SqlError::parse_error("expected column name".to_string(), 0, 0)),
        };

        let data_type = match self.peek_kind()? {
            TokenKind::Integer => { self.advance(); DataType::Integer }
            TokenKind::Text => { self.advance(); DataType::Text }
            TokenKind::Blob => { self.advance(); DataType::Blob }
            _ => return Err(SqlError::parse_error("expected column type".to_string(), 0, 0)),
        };

        let mut is_primary_key = false;
        if self.match_kind(TokenKind::PrimaryKey).is_some() {
            is_primary_key = true;
        }

        Ok(ColumnDef { name, data_type, is_primary_key })
    }

    fn parse_insert(&mut self) -> SqlResult<Stmt> {
        self.expect_kind(TokenKind::Insert)?;
        self.expect_kind(TokenKind::Into)?;
        let table = match self.peek_kind()? {
            TokenKind::Ident(s) => { self.advance(); s }
            _ => return Err(SqlError::parse_error("expected table name".to_string(), 0, 0)),
        };

        self.expect_kind(TokenKind::Values)?;
        self.expect_kind(TokenKind::LParen)?;

        let mut values = Vec::new();
        if self.peek_kind()? != TokenKind::RParen {
            values.push(self.parse_expr()?);
            while self.match_kind(TokenKind::Comma).is_some() {
                values.push(self.parse_expr()?);
            }
        }

        self.expect_kind(TokenKind::RParen)?;

        Ok(Stmt::Insert { table, values })
    }

    fn parse_select_stmt(&mut self) -> SqlResult<Stmt> {
        let _ = self.advance(); // consume SELECT

        let columns = if self.match_kind(TokenKind::Star).is_some() {
            Projection::All
        } else {
            let mut cols = Vec::new();
            match self.peek_kind()? {
                TokenKind::Ident(s) => {
                    let _name = self.advance();
                    cols.push(s);
                    while self.match_kind(TokenKind::Comma).is_some() {
                        match self.peek_kind()? {
                            TokenKind::Ident(s) => {
                                let _name = self.advance();
                                cols.push(s);
                            }
                            _ => return Err(SqlError::parse_error("expected column name".to_string(), 0, 0)),
                        }
                    }
                }
                _ => return Err(SqlError::parse_error("expected projection".to_string(), 0, 0)),
            }
            Projection::Named(cols)
        };

        self.expect_kind(TokenKind::From)?;
        let table = match self.peek_kind()? {
            TokenKind::Ident(s) => { self.advance(); s }
            _ => return Err(SqlError::parse_error("expected table name".to_string(), 0, 0)),
        };

        let predicate = if self.match_kind(TokenKind::Where).is_some() {
            Some(self.parse_predicate()?)
        } else {
            None
        };

        Ok(Stmt::Select { table, columns, predicate })
    }

    fn parse_delete_stmt(&mut self) -> SqlResult<Stmt> {
        let _ = self.advance(); // consume DELETE
        
        self.expect_kind(TokenKind::From)?;
        let table = match self.peek_kind()? {
            TokenKind::Ident(s) => { self.advance(); s }
            _ => return Err(SqlError::parse_error("expected table name".to_string(), 0, 0)),
        };

        self.expect_kind(TokenKind::Where)?;
        let predicate = Some(self.parse_predicate()?);

        Ok(Stmt::Delete { table, predicate })
    }

    fn parse_predicate(&mut self) -> SqlResult<Expr> {
        let left = match self.peek_kind()? {
            TokenKind::Ident(s) => {
                let _name = self.advance();
                Expr::Column(s)
            }
            _ => return Err(SqlError::parse_error("expected column name in predicate".to_string(), 0, 0)),
        };

        self.expect_kind(TokenKind::Eq)?;

        let right = match self.peek_kind()? {
            TokenKind::Parameter => {
                self.advance();
                Expr::Parameter(1)
            }
            TokenKind::LiteralInteger(n) => {
                let _token = self.advance();
                Expr::Literal(Value::Integer(n))
            }
            TokenKind::LiteralString(s) => {
                let _token = self.advance();
                Expr::Literal(Value::Text(s))
            }
            _ => return Err(SqlError::parse_error("expected value in predicate".to_string(), 0, 0)),
        };

        Ok(Expr::Eq(Box::new(left), Box::new(right)))
    }

    fn parse_expr(&mut self) -> SqlResult<Expr> {
        match self.peek_kind()? {
            TokenKind::LiteralInteger(n) => {
                let _token = self.advance();
                Ok(Expr::Literal(Value::Integer(n)))
            }
            TokenKind::LiteralString(s) => {
                let _token = self.advance();
                Ok(Expr::Literal(Value::Text(s)))
            }
            TokenKind::Parameter => {
                self.advance();
                Ok(Expr::Parameter(1))
            }
            _ => Err(SqlError::parse_error("expected expression".to_string(), 0, 0)),
        }
    }

    fn expect_eof(&mut self) -> SqlResult<()> {
        if self.pos < self.tokens.len() - 1 {
            let msg = format!("unexpected tokens remaining: {:?}", self.peek_kind()?);
            Err(SqlError::parse_error(msg, 0, 0))
        } else {
            Ok(())
        }
    }
}

/// Helper to format a TokenKind for error messages.
fn format_token_kind(kind: &TokenKind) -> String {
    match kind {
        TokenKind::Create => "CREATE".to_string(),
        TokenKind::Table => "TABLE".to_string(),
        TokenKind::Insert => "INSERT".to_string(),
        TokenKind::Into => "INTO".to_string(),
        TokenKind::Select => "SELECT".to_string(),
        TokenKind::From => "FROM".to_string(),
        TokenKind::Where => "WHERE".to_string(),
        TokenKind::PrimaryKey => "PRIMARY KEY".to_string(),
        TokenKind::Values => "VALUES".to_string(),
        TokenKind::Delete => "DELETE".to_string(),
        TokenKind::Integer => "INTEGER".to_string(),
        TokenKind::Text => "TEXT".to_string(),
        TokenKind::Blob => "BLOB".to_string(),
        TokenKind::Ident(s) => format!("'{s}'"),
        TokenKind::LiteralString(_) => "string literal".to_string(),
        TokenKind::LiteralInteger(_) => "integer literal".to_string(),
        TokenKind::LParen => "(" .to_string(),
        TokenKind::RParen => ")".to_string(),
        TokenKind::Comma => ",".to_string(),
        TokenKind::Eq => "=".to_string(),
        TokenKind::Star => "*".to_string(),
        TokenKind::Parameter => "?".to_string(),
        TokenKind::Eof => "EOF".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_create_table_basic() {
        let stmt = parse("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )").unwrap();
        match stmt {
            Stmt::CreateTable { name, columns } => {
                assert_eq!(name, "users");
                assert_eq!(columns.len(), 2);
                assert!(columns[0].is_primary_key);
                assert_eq!(columns[0].data_type, DataType::Integer);
                assert_eq!(columns[1].data_type, DataType::Text);
            }
            _ => panic!("expected CreateTable"),
        }
    }

    #[test]
    fn parse_insert_basic() {
        let stmt = parse("INSERT INTO users VALUES ( 1, 'alice' )").unwrap();
        match stmt {
            Stmt::Insert { table, values } => {
                assert_eq!(table, "users");
                assert_eq!(values.len(), 2);
            }
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn parse_select_with_where_parameter() {
        let stmt = parse("SELECT * FROM users WHERE id = ?").unwrap();
        match stmt {
            Stmt::Select { table, columns, predicate } => {
                assert_eq!(table, "users");
                assert!(matches!(columns, Projection::All));
                assert!(predicate.is_some());
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn parse_select_with_where_literal() {
        let stmt = parse("SELECT * FROM users WHERE id = 42").unwrap();
        match stmt {
            Stmt::Select { predicate, .. } => {
                assert!(predicate.is_some());
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn parse_select_without_where() {
        let stmt = parse("SELECT * FROM users").unwrap();
        match stmt {
            Stmt::Select { predicate, .. } => {
                assert!(predicate.is_none());
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn parse_create_table_blob_type() {
        let stmt = parse("CREATE TABLE blobs ( id INTEGER PRIMARY KEY, data BLOB )").unwrap();
        match stmt {
            Stmt::CreateTable { columns, .. } => {
                assert_eq!(columns[1].data_type, DataType::Blob);
            }
            _ => panic!("expected CreateTable"),
        }
    }

    #[test]
    fn parse_insert_with_string_literal() {
        let stmt = parse("INSERT INTO t VALUES ( 'hello world' )").unwrap();
        match stmt {
            Stmt::Insert { values, .. } => {
                assert_eq!(values[0], Expr::Literal(Value::Text("hello world".to_string())));
            }
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn parse_rejects_multi_statement() {
        let result = parse("SELECT * FROM t; SELECT * FROM u");
        assert!(result.is_err());
    }

    #[test]
    fn parse_column_list_projection() {
        let stmt = parse("SELECT a, b FROM t").unwrap();
        match stmt {
            Stmt::Select { columns, .. } => {
                if let Projection::Named(names) = columns {
                    assert_eq!(names, vec!["a".to_string(), "b".to_string()]);
                } else {
                    panic!("expected Named projection");
                }
            }
            _ => panic!("expected Select"),
        }
    }

    #[test]
    fn parse_case_insensitive_keywords() {
        let stmt = parse("create table t ( id integer primary key )").unwrap();
        match stmt {
            Stmt::CreateTable { name, .. } => {
                assert_eq!(name, "t");
            }
            _ => panic!("expected CreateTable"),
        }
    }

    #[test]
    fn parse_rejects_unsupported_syntax() {
        let result = parse("DROP TABLE t");
        assert!(result.is_err());
    }

    #[test]
    fn parse_empty_values_list() {
        let stmt = parse("INSERT INTO t VALUES ( )").unwrap();
        match stmt {
            Stmt::Insert { values, .. } => {
                assert!(values.is_empty());
            }
            _ => panic!("expected Insert"),
        }
    }

    #[test]
    fn parse_delete_basic() {
        let stmt = parse("DELETE FROM users WHERE id = 1").unwrap();
        match stmt {
            Stmt::Delete { table, predicate } => {
                assert_eq!(table, "users");
                assert!(predicate.is_some());
            }
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn parse_delete_with_parameter() {
        let stmt = parse("DELETE FROM users WHERE id = ?").unwrap();
        match stmt {
            Stmt::Delete { table, predicate } => {
                assert_eq!(table, "users");
                assert!(predicate.is_some());
            }
            _ => panic!("expected Delete"),
        }
    }

    #[test]
    fn parse_delete_case_insensitive() {
        let stmt = parse("delete from users where id = 1").unwrap();
        match stmt {
            Stmt::Delete { table, .. } => {
                assert_eq!(table, "users");
            }
            _ => panic!("expected Delete"),
        }
    }
}
