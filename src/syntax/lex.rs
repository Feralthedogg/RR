use crate::syntax::token::{Token, TokenKind};
use crate::utils::Span;
use std::iter::Peekable;
use std::str::Chars;

pub struct Lexer<'a> {
    input: &'a str,
    chars: Peekable<Chars<'a>>,
    byte_pos: usize,
    line: u32,
    col: u32,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            chars: input.chars().peekable(),
            byte_pos: 0,
            line: 1,
            col: 1,
        }
    }

    fn peek(&mut self) -> Option<char> {
        self.chars.peek().copied()
    }

    fn advance(&mut self) -> Option<char> {
        match self.chars.next() {
            Some(c) => {
                let len = c.len_utf8();
                self.byte_pos += len;
                if c == '\n' {
                    self.line += 1;
                    self.col = 1;
                } else {
                    self.col += 1;
                }
                Some(c)
            }
            None => None,
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if c.is_whitespace() {
                self.advance();
            } else if c == '/' {
                // Check comment
                let mut clone = self.chars.clone();
                clone.next(); // skip /
                match clone.next() {
                    Some('/') => {
                        // Line comment
                        while let Some(nc) = self.peek() {
                            if nc == '\n' {
                                break;
                            }
                            self.advance();
                        }
                    }
                    Some('*') => {
                        // Block comment
                        self.advance();
                        self.advance(); // eat /*
                        while let Some(nc) = self.advance() {
                            if nc == '*' {
                                if let Some('/') = self.peek() {
                                    self.advance(); // eat /
                                    break;
                                }
                            }
                        }
                    }
                    _ => break,
                }
            } else {
                break;
            }
        }
    }

    fn read_while<F>(&mut self, pred: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if pred(c) {
                s.push(self.advance().unwrap());
            } else {
                break;
            }
        }
        s
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let start_byte = self.byte_pos;
        let start_line = self.line;
        let start_col = self.col;

        let kind = match self.peek() {
            Some(c) if c.is_alphabetic() || c == '_' => {
                let ident = self.read_while(|c| c.is_alphanumeric() || c == '_');
                match ident.as_str() {
                    "fn" => TokenKind::Fn,
                    "function" => TokenKind::Fn,
                    "let" => TokenKind::Let,
                    "if" => TokenKind::If,
                    "else" => TokenKind::Else,
                    "while" => TokenKind::While,
                    "for" => TokenKind::For,
                    "in" => TokenKind::In,
                    "return" => TokenKind::Return,
                    "break" => TokenKind::Break,
                    "next" => TokenKind::Next,
                    "true" | "TRUE" => TokenKind::True,
                    "false" | "FALSE" => TokenKind::False,
                    "null" | "NULL" => TokenKind::Null,
                    "na" | "NA" => TokenKind::Na,
                    "match" => TokenKind::Match,
                    "import" => TokenKind::Import,
                    "export" => TokenKind::Export,
                    _ => TokenKind::Ident(ident),
                }
            }
            Some(c) if c.is_ascii_digit() => {
                let num_str = self.read_while(|c| c.is_ascii_digit());
                // Check for float: must be . followed by digit
                let mut is_float = false;
                if let Some('.') = self.peek() {
                    // Need to peek ahead of the dot without consuming
                    // Clone iterator is cheap
                    let mut lookahead = self.chars.clone();
                    lookahead.next(); // skip .
                    if let Some(next_c) = lookahead.peek() {
                        if next_c.is_ascii_digit() {
                            is_float = true;
                        }
                    }
                }

                if is_float {
                    self.advance(); // eat .
                    let frac = self.read_while(|c| c.is_ascii_digit());
                    let full = format!("{}.{}", num_str, frac);
                    TokenKind::Float(full.parse().unwrap_or(0.0))
                } else {
                    // Accept R-style integer suffix: 1L / 1l
                    if matches!(self.peek(), Some('L' | 'l')) {
                        self.advance();
                    }
                    TokenKind::Int(num_str.parse().unwrap_or(0))
                }
            }
            Some('"') => {
                self.advance(); // eat "
                let mut s = String::new();
                let mut terminated = false;
                while let Some(c) = self.advance() {
                    if c == '"' {
                        terminated = true;
                        break;
                    }
                    if c == '\\' {
                        if let Some(next_c) = self.advance() {
                            match next_c {
                                'n' => s.push('\n'),
                                'r' => s.push('\r'),
                                't' => s.push('\t'),
                                '"' => s.push('"'),
                                '\\' => s.push('\\'),
                                _ => {
                                    s.push('\\');
                                    s.push(next_c);
                                }
                            }
                        } else {
                            break;
                        }
                    } else {
                        s.push(c);
                    }
                }
                if terminated {
                    TokenKind::String(s)
                } else {
                    TokenKind::Invalid("unterminated string literal".to_string())
                }
            }
            Some('=') => {
                self.advance();
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::Eq
                } else if let Some('>') = self.peek() {
                    self.advance();
                    TokenKind::Arrow
                } else {
                    TokenKind::Assign
                }
            }
            Some('@') => {
                self.advance();
                TokenKind::At
            }
            Some('^') => {
                self.advance();
                TokenKind::Caret
            }
            Some('?') => {
                self.advance();
                TokenKind::Question
            }
            Some('!') => {
                self.advance();
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::Ne
                } else {
                    TokenKind::Bang
                }
            }
            Some('<') => {
                self.advance();
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::Le
                } else if let Some('-') = self.peek() {
                    // R-style assignment: <-
                    self.advance();
                    TokenKind::Assign
                } else {
                    TokenKind::Lt
                }
            }
            Some('>') => {
                self.advance();
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::Ge
                } else {
                    TokenKind::Gt
                }
            }
            Some('&') => {
                self.advance();
                if let Some('&') = self.peek() {
                    self.advance();
                }
                TokenKind::And
            }
            Some('|') => {
                self.advance();
                if let Some('|') = self.peek() {
                    self.advance();
                    TokenKind::Or
                } else if let Some('>') = self.peek() {
                    self.advance();
                    TokenKind::Pipe
                } else {
                    TokenKind::Or
                }
            }
            Some('.') => {
                // Check if float first? No, floats handled in digit branch if starts with digit.
                // But .123 is valid float?
                // Current lexer handles leading digit floats.
                // If starts with ., check if next is digit?
                let mut is_start_float = false;
                {
                    let mut lookahead = self.chars.clone();
                    lookahead.next(); // skip .
                    if let Some(c) = lookahead.peek() {
                        if c.is_ascii_digit() {
                            is_start_float = true;
                        }
                    }
                }

                if is_start_float {
                    self.advance(); // eat .
                    let frac = self.read_while(|c| c.is_ascii_digit());
                    let full = format!("0.{}", frac);
                    TokenKind::Float(full.parse().unwrap_or(0.0))
                } else {
                    self.advance();
                    if let Some('.') = self.peek() {
                        self.advance();
                        TokenKind::DotDot
                    } else {
                        TokenKind::Dot
                    }
                }
            }
            Some('+') => {
                self.advance();
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::PlusAssign
                } else {
                    TokenKind::Plus
                }
            }
            Some('-') => {
                self.advance();
                if let Some('>') = self.peek() {
                    self.advance();
                    TokenKind::Arrow
                } else if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::MinusAssign
                } else {
                    TokenKind::Minus
                }
            }
            Some('*') => {
                self.advance();
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::StarAssign
                } else {
                    TokenKind::Star
                }
            }
            Some('/') => {
                self.advance();
                if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::SlashAssign
                } else {
                    TokenKind::Slash
                }
            }
            Some('%') => {
                self.advance();
                if let Some('*') = self.peek() {
                    self.advance();
                    if let Some('%') = self.peek() {
                        self.advance();
                        TokenKind::MatMul
                    } else {
                        TokenKind::Percent
                    }
                } else if let Some('=') = self.peek() {
                    self.advance();
                    TokenKind::PercentAssign
                } else {
                    TokenKind::Percent
                }
            }
            Some('(') => {
                self.advance();
                TokenKind::LParen
            }
            Some(')') => {
                self.advance();
                TokenKind::RParen
            }
            Some('{') => {
                self.advance();
                TokenKind::LBrace
            }
            Some('}') => {
                self.advance();
                TokenKind::RBrace
            }
            Some('[') => {
                self.advance();
                TokenKind::LBracket
            }
            Some(']') => {
                self.advance();
                TokenKind::RBracket
            }
            Some(',') => {
                self.advance();
                TokenKind::Comma
            }
            Some(':') => {
                self.advance();
                TokenKind::Colon
            }
            Some(';') => {
                self.advance();
                TokenKind::Semicolon
            }
            None => TokenKind::EOF,
            Some(c) => {
                self.advance();
                TokenKind::Invalid(format!("unexpected character '{}'", c))
            }
        };

        Token {
            kind,
            span: Span {
                start_byte,
                end_byte: self.byte_pos,
                start_line,
                start_col,
                end_line: self.line,
                end_col: self.col,
            },
        }
    }
}
