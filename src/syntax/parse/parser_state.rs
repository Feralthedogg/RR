use super::*;
use crate::syntax::lex::Lexer;
use crate::utils::Span;

#[path = "helpers.rs"]
pub(crate) mod helpers;

pub struct Parser<'a> {
    pub(crate) lexer: Lexer<'a>,
    pub(crate) current: Token,
    pub(crate) peek: Token,
    pub(crate) previous_span: Span,
}

#[derive(PartialEq, PartialOrd)]
pub(crate) enum Precedence {
    Lowest,
    Formula,    // ~
    Pipe,       // |>
    LogicOr,    // ||
    LogicAnd,   // &&
    Equality,   // == !=
    Comparison, // < > <= >=
    Range,      // ..
    Sum,        // + -
    Product,    // * / %
    Prefix,     // -X !X
    Call,       // ( [
    Try,        // ? (Postfix)
}
