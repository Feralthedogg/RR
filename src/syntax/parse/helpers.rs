use super::Precedence;
use crate::syntax::ast::{BinOp, TypeExpr};
use crate::syntax::token::{TokenKind, TokenKind::*};
use std::string::String;

pub(crate) fn dotted_segment_name(kind: &TokenKind) -> Option<String> {
    match kind {
        Ident(n) => Some(n.clone()),
        Match => Some("match".to_string()),
        // Allow common R-style dotted names like `is.na` / `is.null`.
        Na => Some("na".to_string()),
        Null => Some("null".to_string()),
        True => Some("true".to_string()),
        False => Some("false".to_string()),
        // Keep R-style selectors such as `utils.getAnywhere("x").where`
        // usable after reserving `where` for trait bounds.
        Where => Some("where".to_string()),
        _ => None,
    }
}

pub(crate) fn call_arg_name(kind: &TokenKind) -> Option<String> {
    match kind {
        Ident(name) => Some(name.clone()),
        // `where` is a trait-bound keyword in declarations, but R APIs also
        // use it as a named argument, e.g. `methods.findUnique(where = env)`.
        Where => Some("where".to_string()),
        _ => None,
    }
}

pub(crate) fn type_expr_key(expr: &TypeExpr) -> String {
    match expr {
        TypeExpr::Named(name) => name.clone(),
        TypeExpr::Generic { base, args } => {
            let args = args.iter().map(type_expr_key).collect::<Vec<_>>().join(",");
            format!("{base}<{args}>")
        }
    }
}

pub(crate) fn token_precedence(kind: &TokenKind) -> Precedence {
    match kind {
        Tilde => Precedence::Formula,
        Pipe => Precedence::Pipe,
        Or => Precedence::LogicOr,
        And => Precedence::LogicAnd,
        Eq | Ne => Precedence::Equality,
        Lt | Le | Gt | Ge => Precedence::Comparison,
        DotDot => Precedence::Range,
        Plus | Minus => Precedence::Sum,
        Star | Slash | Percent | MatMul => Precedence::Product,
        LParen | LBracket | Dot | DoubleColon => Precedence::Call,
        Question => Precedence::Try,
        _ => Precedence::Lowest,
    }
}

pub(crate) fn compound_assign_binop(kind: &TokenKind) -> Option<BinOp> {
    match kind {
        PlusAssign => Some(BinOp::Add),
        MinusAssign => Some(BinOp::Sub),
        StarAssign => Some(BinOp::Mul),
        SlashAssign => Some(BinOp::Div),
        PercentAssign => Some(BinOp::Mod),
        _ => None,
    }
}
