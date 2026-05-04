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
                            if nc == '*'
                                && let Some('/') = self.peek()
                            {
                                self.advance(); // eat /
                                break;
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
                let Some(next) = self.advance() else {
                    break;
                };
                s.push(next);
            } else {
                break;
            }
        }
        s
    }

    fn is_ident_continue(c: char) -> bool {
        c.is_alphanumeric() || c == '_'
    }

    fn consume_unsafe_r_keyword(rem: &str, idx: &mut usize, keyword: &str) -> Option<()> {
        rem[*idx..].starts_with(keyword).then_some(())?;
        *idx += keyword.len();
        if rem[*idx..]
            .chars()
            .next()
            .is_some_and(Self::is_ident_continue)
        {
            return None;
        }
        Some(())
    }

    fn consume_whitespace(rem: &str, idx: &mut usize) -> bool {
        let mut saw_ws = false;
        while let Some(c) = rem[*idx..].chars().next()
            && c.is_whitespace()
        {
            saw_ws = true;
            *idx += c.len_utf8();
        }
        saw_ws
    }

    fn consume_optional_read_only_marker(rem: &str, idx: &mut usize) -> bool {
        if !rem[*idx..].starts_with("(read)") {
            return false;
        }
        *idx += "(read)".len();
        Self::consume_whitespace(rem, idx);
        true
    }

    fn unsafe_r_prefix(&self) -> Option<(usize, bool)> {
        let rem = &self.input[self.byte_pos..];
        let mut idx = 0usize;
        Self::consume_unsafe_r_keyword(rem, &mut idx, "unsafe")?;
        if !Self::consume_whitespace(rem, &mut idx) {
            return None;
        }
        Self::consume_unsafe_r_keyword(rem, &mut idx, "r")?;
        Self::consume_whitespace(rem, &mut idx);

        let read_only = Self::consume_optional_read_only_marker(rem, &mut idx);
        Self::consume_whitespace(rem, &mut idx);
        if !rem[idx..].starts_with('{') {
            return None;
        }

        Some((idx + '{'.len_utf8(), read_only))
    }

    fn advance_bytes(&mut self, byte_count: usize) {
        let target = self.byte_pos + byte_count;
        while self.byte_pos < target && self.peek().is_some() {
            self.advance();
        }
    }

    fn try_read_unsafe_r_block(&mut self) -> Option<TokenKind> {
        let (prefix_len, read_only) = self.unsafe_r_prefix()?;
        self.advance_bytes(prefix_len);

        let mut code = String::new();
        let mut depth = 1usize;
        let mut quote: Option<char> = None;
        let mut escaped = false;
        let mut in_comment = false;

        while let Some(c) = self.advance() {
            if in_comment {
                code.push(c);
                if c == '\n' {
                    in_comment = false;
                }
                continue;
            }

            if let Some(q) = quote {
                code.push(c);
                if escaped {
                    escaped = false;
                } else if c == '\\' {
                    escaped = true;
                } else if c == q {
                    quote = None;
                }
                continue;
            }

            match c {
                '"' | '\'' | '`' => {
                    quote = Some(c);
                    code.push(c);
                }
                '#' => {
                    in_comment = true;
                    code.push(c);
                }
                '{' => {
                    depth += 1;
                    code.push(c);
                }
                '}' => {
                    depth -= 1;
                    if depth == 0 {
                        return Some(TokenKind::UnsafeRBlock { code, read_only });
                    }
                    code.push(c);
                }
                _ => code.push(c),
            }
        }

        Some(TokenKind::Invalid(
            "unterminated unsafe r block".to_string(),
        ))
    }

    fn read_identifier_or_keyword(&mut self) -> TokenKind {
        let ident = self.read_while(Self::is_ident_continue);
        match ident.as_str() {
            "fn" | "function" => TokenKind::Fn,
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
            "trait" => TokenKind::Trait,
            "impl" => TokenKind::Impl,
            "where" => TokenKind::Where,
            _ => TokenKind::Ident(ident),
        }
    }

    fn current_dot_starts_float(&self) -> bool {
        let mut lookahead = self.chars.clone();
        lookahead.next();
        lookahead.peek().is_some_and(|c| c.is_ascii_digit())
    }

    fn read_number_literal(&mut self) -> TokenKind {
        let num_str = self.read_while(|c| c.is_ascii_digit());
        let is_float = matches!(self.peek(), Some('.')) && self.current_dot_starts_float();
        if is_float {
            self.advance();
            let frac = self.read_while(|c| c.is_ascii_digit());
            let full = format!("{}.{}", num_str, frac);
            TokenKind::Float(full.parse().unwrap_or(0.0))
        } else {
            if matches!(self.peek(), Some('L' | 'l')) {
                self.advance();
            }
            TokenKind::Int(num_str.parse().unwrap_or(0))
        }
    }

    fn read_string_literal(&mut self) -> TokenKind {
        self.advance();
        let mut s = String::new();
        let mut terminated = false;
        while let Some(c) = self.advance() {
            if c == '"' {
                terminated = true;
                break;
            }
            if c == '\\' {
                self.push_escaped_char(&mut s);
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

    fn push_escaped_char(&mut self, s: &mut String) {
        let Some(next_c) = self.advance() else {
            return;
        };
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
    }

    fn read_lifetime_ident(&mut self) -> TokenKind {
        self.advance();
        match self.peek() {
            Some(c) if c.is_alphabetic() || c == '_' => {
                let ident = self.read_while(Self::is_ident_continue);
                TokenKind::Ident(format!("'{ident}"))
            }
            _ => TokenKind::Invalid("expected lifetime name after '\''".to_string()),
        }
    }

    fn read_equals_token(&mut self) -> TokenKind {
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

    fn read_bang_token(&mut self) -> TokenKind {
        self.advance();
        if let Some('=') = self.peek() {
            self.advance();
            TokenKind::Ne
        } else {
            TokenKind::Bang
        }
    }

    fn read_lt_token(&mut self) -> TokenKind {
        self.advance();
        if let Some('=') = self.peek() {
            self.advance();
            TokenKind::Le
        } else if let Some('-') = self.peek() {
            self.advance();
            TokenKind::Assign
        } else {
            TokenKind::Lt
        }
    }

    fn read_gt_token(&mut self) -> TokenKind {
        self.advance();
        if let Some('=') = self.peek() {
            self.advance();
            TokenKind::Ge
        } else {
            TokenKind::Gt
        }
    }

    fn read_and_token(&mut self) -> TokenKind {
        self.advance();
        if let Some('&') = self.peek() {
            self.advance();
        }
        TokenKind::And
    }

    fn read_pipe_token(&mut self) -> TokenKind {
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

    fn read_dot_token(&mut self) -> TokenKind {
        if self.current_dot_starts_float() {
            self.advance();
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

    fn read_plus_token(&mut self) -> TokenKind {
        self.advance();
        if let Some('=') = self.peek() {
            self.advance();
            TokenKind::PlusAssign
        } else {
            TokenKind::Plus
        }
    }

    fn read_minus_token(&mut self) -> TokenKind {
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

    fn read_star_token(&mut self) -> TokenKind {
        self.advance();
        if let Some('=') = self.peek() {
            self.advance();
            TokenKind::StarAssign
        } else {
            TokenKind::Star
        }
    }

    fn read_slash_token(&mut self) -> TokenKind {
        self.advance();
        if let Some('=') = self.peek() {
            self.advance();
            TokenKind::SlashAssign
        } else {
            TokenKind::Slash
        }
    }

    fn read_percent_token(&mut self) -> TokenKind {
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

    fn read_colon_token(&mut self) -> TokenKind {
        self.advance();
        if let Some(':') = self.peek() {
            self.advance();
            TokenKind::DoubleColon
        } else {
            TokenKind::Colon
        }
    }

    fn simple_token(&mut self, kind: TokenKind) -> TokenKind {
        self.advance();
        kind
    }

    fn invalid_char_token(&mut self, c: char) -> TokenKind {
        self.advance();
        TokenKind::Invalid(format!("unexpected character '{}'", c))
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        let start_byte = self.byte_pos;
        let start_line = self.line;
        let start_col = self.col;

        if let Some(kind) = self.try_read_unsafe_r_block() {
            return Token {
                kind,
                span: Span {
                    start_byte,
                    end_byte: self.byte_pos,
                    start_line,
                    start_col,
                    end_line: self.line,
                    end_col: self.col,
                },
            };
        }

        let kind = match self.peek() {
            Some(c) if c.is_alphabetic() || c == '_' => self.read_identifier_or_keyword(),
            Some(c) if c.is_ascii_digit() => self.read_number_literal(),
            Some('"') => self.read_string_literal(),
            Some('\'') => self.read_lifetime_ident(),
            Some('=') => self.read_equals_token(),
            Some('@') => self.simple_token(TokenKind::At),
            Some('~') => self.simple_token(TokenKind::Tilde),
            Some('^') => self.simple_token(TokenKind::Caret),
            Some('?') => self.simple_token(TokenKind::Question),
            Some('!') => self.read_bang_token(),
            Some('<') => self.read_lt_token(),
            Some('>') => self.read_gt_token(),
            Some('&') => self.read_and_token(),
            Some('|') => self.read_pipe_token(),
            Some('.') => self.read_dot_token(),
            Some('+') => self.read_plus_token(),
            Some('-') => self.read_minus_token(),
            Some('*') => self.read_star_token(),
            Some('/') => self.read_slash_token(),
            Some('%') => self.read_percent_token(),
            Some(':') => self.read_colon_token(),
            Some('(') => self.simple_token(TokenKind::LParen),
            Some(')') => self.simple_token(TokenKind::RParen),
            Some('{') => self.simple_token(TokenKind::LBrace),
            Some('}') => self.simple_token(TokenKind::RBrace),
            Some('[') => self.simple_token(TokenKind::LBracket),
            Some(']') => self.simple_token(TokenKind::RBracket),
            Some(',') => self.simple_token(TokenKind::Comma),
            Some(';') => {
                self.advance();
                TokenKind::Invalid(
                    "semicolons are not supported; end the statement with a newline or '}'"
                        .to_string(),
                )
            }
            None => TokenKind::Eof,
            Some(c) => self.invalid_char_token(c),
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
