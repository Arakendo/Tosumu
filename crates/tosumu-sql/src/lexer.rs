//! SQL lexer for the tosumu toy SQL layer (MVP+9).
//!
//! Tokenizes SQL input into a stream of tokens for the recursive descent parser.

use crate::error::{SqlError, SqlResult};

/// SQL token types.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    /// CREATE keyword
    Create,
    /// TABLE keyword
    Table,
    /// INSERT keyword
    Insert,
    /// INTO keyword
    Into,
    /// SELECT keyword
    Select,
    /// FROM keyword
    From,
    /// WHERE keyword
    Where,
    /// PRIMARY KEY keywords
    PrimaryKey,
    /// VALUES keyword
    Values,
    /// DELETE keyword
    Delete,
    /// INTEGER type keyword
    Integer,
    /// TEXT type keyword
    Text,
    /// BLOB type keyword
    Blob,
    /// Identifier (unquoted)
    Ident(String),
    /// String literal
    LiteralString(String),
    /// Integer literal
    LiteralInteger(i64),
    /// Left parenthesis
    LParen,
    /// Right parenthesis
    RParen,
    /// Comma
    Comma,
    /// Equals
    Eq,
    /// Star (multiplication / wildcard)
    Star,
    /// Question mark (parameter placeholder)
    Parameter,
    /// End of input
    Eof,
}

/// A token with its position in the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub col: usize,
}

impl Token {
    fn new(kind: TokenKind, line: usize, col: usize) -> Self {
        Token { kind, line, col }
    }
}

/// Lexer state.
pub struct Lexer {
    source: String,
    pos: usize,       // byte position
    line: usize,      // 1-based
    col: usize,       // 1-based
}

impl Lexer {
    /// Create a new lexer for the given SQL string.
    pub fn new(sql: &str) -> Self {
        Lexer {
            source: sql.to_string(),
            pos: 0,
            line: 1,
            col: 1,
        }
    }

    /// Lex all tokens from the input.
    pub fn tokenize(&mut self) -> SqlResult<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_and_comments();
            if self.at_eof() {
                tokens.push(Token::new(TokenKind::Eof, self.line, self.col));
                break;
            }
            let token = self.next_token()?;
            tokens.push(token);
        }
        Ok(tokens)
    }

    fn at_eof(&self) -> bool {
        self.pos >= self.source.len()
    }

    fn current_char(&self) -> Option<char> {
        self.source[self.pos..].chars().next()
    }

    fn peek_char(&self, offset: usize) -> Option<char> {
        let idx = self.pos + offset;
        if idx < self.source.len() {
            Some(self.source[idx..].chars().next()?)
        } else {
            None
        }
    }

    fn advance(&mut self) -> char {
        let c = self.source[self.pos..].chars().next().unwrap();
        if c == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        self.pos += c.len_utf8();
        c
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.advance();
            } else if c == '-' && self.peek_char(1) == Some('-') {
                // Line comment
                while let Some(c) = self.current_char() {
                    if c == '\n' { break; }
                    self.advance();
                }
            } else if c == ';' {
                // Semicolons are ignored (statement separators)
                self.advance();
            } else {
                break;
            }
        }
    }

    fn next_token(&mut self) -> SqlResult<Token> {
        let line = self.line;
        let col = self.col;

        match self.current_char() {
            Some('(') => { self.advance(); Ok(Token::new(TokenKind::LParen, line, col)) }
            Some(')') => { self.advance(); Ok(Token::new(TokenKind::RParen, line, col)) }
            Some(',') => { self.advance(); Ok(Token::new(TokenKind::Comma, line, col)) }
            Some('*') => { self.advance(); Ok(Token::new(TokenKind::Star, line, col)) }
            Some('?') => { self.advance(); Ok(Token::new(TokenKind::Parameter, line, col)) }
            Some('\'') => self.read_string_literal(line, col),
            Some(c) if c.is_ascii_digit() => self.read_integer(line, col),
            Some('=') => {
                self.advance();
                Ok(Token::new(TokenKind::Eq, line, col))
            }
            Some(c) if c.is_ascii_alphabetic() || c == '_' => self.read_ident_or_keyword(line, col),
            Some(c) => {
                let msg = format!("unexpected character '{c}' at line {line}, column {col}");
                Err(SqlError::parse_error(msg, line, col))
            }
            None => Ok(Token::new(TokenKind::Eof, line, col)),
        }
    }

    fn read_string_literal(&mut self, line: usize, col: usize) -> SqlResult<Token> {
        self.advance(); // skip opening quote
        let mut s = String::new();
        while let Some(c) = self.current_char() {
            if c == '\'' {
                if self.peek_char(1) == Some('\'') {
                    // Escaped quote
                    s.push('\'');
                    self.advance();
                    self.advance();
                } else {
                    // Closing quote
                    self.advance();
                    return Ok(Token::new(TokenKind::LiteralString(s), line, col));
                }
            } else {
                s.push(c);
                self.advance();
            }
        }
        let msg = "unterminated string literal".to_string();
        Err(SqlError::parse_error(msg, line, self.col))
    }

    fn read_integer(&mut self, line: usize, col: usize) -> SqlResult<Token> {
        let mut s = String::new();
        while let Some(c) = self.current_char() {
            if c.is_ascii_digit() {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }
        let val: i64 = s.parse().map_err(|_| {
            SqlError::parse_error(format!("invalid integer literal '{s}'"), line, col)
        })?;
        Ok(Token::new(TokenKind::LiteralInteger(val), line, col))
    }

    fn read_ident_or_keyword(&mut self, line: usize, col: usize) -> SqlResult<Token> {
        let mut s = String::new();
        while let Some(c) = self.current_char() {
            if c.is_ascii_alphanumeric() || c == '_' {
                s.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Special handling for "PRIMARY KEY" as a single token
        if s.to_uppercase() == "PRIMARY" {
            let saved_pos = self.pos;

            // Peek ahead: skip spaces/tabs only (byte-level, since we know space/tab are 1 byte)
            let mut peek_pos = saved_pos;
            while peek_pos < self.source.len() {
                let ch = self.source.as_bytes()[peek_pos];
                if ch == b' ' || ch == b'\t' {
                    peek_pos += 1;
                } else {
                    break;
                }
            }

            // Check if next non-space word is KEY
            if peek_pos < self.source.len() {
                let remaining = &self.source[peek_pos..];
                let mut key_chars: Vec<char> = Vec::new();
                for ch in remaining.chars() {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        key_chars.push(ch);
                    } else {
                        break;
                    }
                }
                let key_word: String = key_chars.iter().collect();
                if key_word.to_uppercase() == "KEY" {
                    // Consume everything from saved_pos through end of KEY.
                    // The number of bytes to advance is the distance from saved_pos
                    // to the end of the KEY word.
                    let end_of_key_byte = peek_pos + key_word.len();
                    let bytes_to_advance = end_of_key_byte - saved_pos;
                    for _ in 0..bytes_to_advance {
                        let c = self.source.as_bytes()[self.pos];
                        self.pos += 1;
                        if c == b'\n' {
                            self.line += 1;
                            self.col = 1;
                        } else {
                            self.col += 1;
                        }
                    }
                    return Ok(Token::new(TokenKind::PrimaryKey, line, col));
                }
            }

            // Not PRIMARY KEY, restore position
            self.pos = saved_pos;
        }

        let kind = match s.to_uppercase().as_str() {
            "CREATE" => TokenKind::Create,
            "TABLE" => TokenKind::Table,
            "INSERT" => TokenKind::Insert,
            "INTO" => TokenKind::Into,
            "SELECT" => TokenKind::Select,
            "FROM" => TokenKind::From,
            "WHERE" => TokenKind::Where,
            "PRIMARY" => TokenKind::PrimaryKey,
            "VALUES" => TokenKind::Values,
            "DELETE" => TokenKind::Delete,
            "INTEGER" => TokenKind::Integer,
            "TEXT" => TokenKind::Text,
            "BLOB" => TokenKind::Blob,
            _ => TokenKind::Ident(s),
        };
        Ok(Token::new(kind, line, col))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_create_table() {
        let mut lexer = Lexer::new("CREATE TABLE users ( id INTEGER PRIMARY KEY, name TEXT )");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Create);
        assert_eq!(tokens[1].kind, TokenKind::Table);
        assert_eq!(tokens[2].kind, TokenKind::Ident("users".to_string()));
        assert_eq!(tokens[3].kind, TokenKind::LParen);
        assert_eq!(tokens[4].kind, TokenKind::Ident("id".to_string()));
        assert_eq!(tokens[5].kind, TokenKind::Integer);
        assert_eq!(tokens[6].kind, TokenKind::PrimaryKey);
        assert_eq!(tokens[7].kind, TokenKind::Comma);
        assert_eq!(tokens[8].kind, TokenKind::Ident("name".to_string()));
        assert_eq!(tokens[9].kind, TokenKind::Text);
        assert_eq!(tokens[10].kind, TokenKind::RParen);
        assert_eq!(tokens[11].kind, TokenKind::Eof);
    }

    #[test]
    fn tokenize_insert_with_values() {
        let mut lexer = Lexer::new("INSERT INTO users VALUES ( 1, 'alice' )");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Insert);
        assert_eq!(tokens[1].kind, TokenKind::Into);
        assert_eq!(tokens[2].kind, TokenKind::Ident("users".to_string()));
        assert_eq!(tokens[3].kind, TokenKind::Values);
        assert_eq!(tokens[4].kind, TokenKind::LParen);
        assert_eq!(tokens[5].kind, TokenKind::LiteralInteger(1));
        assert_eq!(tokens[6].kind, TokenKind::Comma);
        assert_eq!(tokens[7].kind, TokenKind::LiteralString("alice".to_string()));
        assert_eq!(tokens[8].kind, TokenKind::RParen);
    }

    #[test]
    fn tokenize_select_with_parameter() {
        let mut lexer = Lexer::new("SELECT * FROM users WHERE id = ?");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Select);
        assert_eq!(tokens[1].kind, TokenKind::Star);
        assert_eq!(tokens[2].kind, TokenKind::From);
        assert_eq!(tokens[3].kind, TokenKind::Ident("users".to_string()));
        assert_eq!(tokens[4].kind, TokenKind::Where);
        assert_eq!(tokens[5].kind, TokenKind::Ident("id".to_string()));
        assert_eq!(tokens[6].kind, TokenKind::Eq);
        assert_eq!(tokens[7].kind, TokenKind::Parameter);
    }

    #[test]
    fn tokenize_select_with_literal() {
        let mut lexer = Lexer::new("SELECT * FROM users WHERE id = 42");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[6].kind, TokenKind::Eq);
        assert_eq!(tokens[7].kind, TokenKind::LiteralInteger(42));
    }

    #[test]
    fn tokenize_string_with_escaped_quote() {
        let mut lexer = Lexer::new("INSERT INTO t VALUES ( 'o''reilly' )");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[5].kind, TokenKind::LiteralString("o'reilly".to_string()));
    }

    #[test]
    fn tokenize_case_insensitive_keywords() {
        let mut lexer = Lexer::new("create table t ( id integer primary key )");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Create);
        assert_eq!(tokens[1].kind, TokenKind::Table);
        // Position 4 is "id" (Ident), position 5 is "integer" (Integer)
        assert_eq!(tokens[4].kind, TokenKind::Ident("id".to_string()));
        assert_eq!(tokens[5].kind, TokenKind::Integer);
        // Verify PRIMARY KEY is recognized as one token
        assert_eq!(tokens[6].kind, TokenKind::PrimaryKey);
    }

    #[test]
    fn tokenize_star_projection() {
        let mut lexer = Lexer::new("SELECT * FROM t");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[1].kind, TokenKind::Star);
    }

    #[test]
    fn tokenize_column_list_projection() {
        let mut lexer = Lexer::new("SELECT a, b, c FROM t");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::Select);
        assert_eq!(tokens[1].kind, TokenKind::Ident("a".to_string()));
        assert_eq!(tokens[2].kind, TokenKind::Comma);
        assert_eq!(tokens[3].kind, TokenKind::Ident("b".to_string()));
        assert_eq!(tokens[4].kind, TokenKind::Comma);
        assert_eq!(tokens[5].kind, TokenKind::Ident("c".to_string()));
    }

    #[test]
    fn tokenize_multiple_parameters() {
        let mut lexer = Lexer::new("SELECT * FROM t WHERE a = ? AND b = ?");
        let tokens = lexer.tokenize().unwrap();
        let param_count = tokens.iter().filter(|t| t.kind == TokenKind::Parameter).count();
        assert_eq!(param_count, 2);
    }

    #[test]
    fn tokenize_blob_type() {
        let mut lexer = Lexer::new("CREATE TABLE t ( id INTEGER PRIMARY KEY, data BLOB )");
        let tokens = lexer.tokenize().unwrap();
        let blob_idx = tokens.iter().position(|t| t.kind == TokenKind::Blob).unwrap();
        assert_eq!(tokens[blob_idx].kind, TokenKind::Blob);
    }

    #[test]
    fn tokenize_trailing_semicolon_ignored() {
        let mut lexer = Lexer::new("SELECT * FROM t;");
        let tokens = lexer.tokenize().unwrap();
        // Semicolons are skipped as whitespace-like
        assert_eq!(tokens.last().unwrap().kind, TokenKind::Eof);
    }

    #[test]
    fn tokenize_line_comment_skipped() {
        let mut lexer = Lexer::new("SELECT * FROM t -- comment\nWHERE id = 1");
        let tokens = lexer.tokenize().unwrap();
        let where_idx = tokens.iter().position(|t| t.kind == TokenKind::Where).unwrap();
        assert_eq!(tokens[where_idx].kind, TokenKind::Where);
    }

    #[test]
    fn tokenize_unterminated_string_error() {
        let mut lexer = Lexer::new("INSERT INTO t VALUES ( 'unterminated");
        let result = lexer.tokenize();
        assert!(result.is_err());
    }

    #[test]
    fn tokenize_empty_input() {
        let mut lexer = Lexer::new("");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn tokenize_whitespace_only() {
        let mut lexer = Lexer::new("   \n  ");
        let tokens = lexer.tokenize().unwrap();
        assert_eq!(tokens.len(), 1);
        assert_eq!(tokens[0].kind, TokenKind::Eof);
    }

    #[test]
    fn tokenize_identifier_with_underscore() {
        let mut lexer = Lexer::new("CREATE TABLE my_table ( user_name INTEGER PRIMARY KEY )");
        let tokens = lexer.tokenize().unwrap();
        let idents: Vec<&str> = tokens.iter()
            .filter_map(|t| match &t.kind { TokenKind::Ident(s) => Some(s.as_str()), _ => None })
            .collect();
        assert!(idents.contains(&"my_table"));
        assert!(idents.contains(&"user_name"));
    }

    #[test]
    fn tokenize_all_type_keywords() {
        let mut lexer = Lexer::new("CREATE TABLE t ( a INTEGER PRIMARY KEY, b TEXT, c BLOB )");
        let tokens = lexer.tokenize().unwrap();
        let type_count = tokens.iter().filter(|t| matches!(&t.kind, TokenKind::Integer | TokenKind::Text | TokenKind::Blob)).count();
        assert_eq!(type_count, 3);
    }
}