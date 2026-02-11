use crate::utils::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Fn,
    Let,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Break,
    Next,
    True,
    False,
    Null,
    Na,
    Match,
    Import,
    Export,

    // Identifiers & Literals
    Ident(String),
    Int(i64),
    Float(f64),
    String(String),

    // Operators
    Assign, // = or <-
    PlusAssign,
    MinusAssign,
    StarAssign,
    SlashAssign,
    PercentAssign, // += -= *= /= %=
    Plus,
    Minus,
    Star,
    Slash,
    Percent, // + - * / %
    MatMul,  // %*%
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge, // == != < <= > >=
    And,
    Or,
    Bang,     // && || !
    DotDot,   // ..
    Dot,      // .
    Pipe,     // |>
    Question, // ?
    At,       // @
    Caret,    // ^
    Arrow,    // =>

    // Delimiters
    LParen,
    RParen, // ( )
    LBrace,
    RBrace, // { }
    LBracket,
    RBracket, // [ ]
    Comma,
    Colon,
    Semicolon, // , : ;

    Invalid(String),
    EOF,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
