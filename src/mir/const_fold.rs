use crate::syntax::ast::{BinOp, Lit};

pub(crate) fn eval_binary_const(op: BinOp, lhs: &Lit, rhs: &Lit) -> Option<Lit> {
    match op {
        BinOp::Add | BinOp::Sub | BinOp::Mul => eval_numeric_arith(op, lhs, rhs),
        BinOp::Div => eval_div(lhs, rhs),
        BinOp::Mod => eval_mod(lhs, rhs),
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => eval_ordering(op, lhs, rhs),
        BinOp::Eq | BinOp::Ne => eval_equality(op, lhs, rhs),
        BinOp::And | BinOp::Or => eval_logical(op, lhs, rhs),
        BinOp::MatMul => None,
    }
}

fn eval_numeric_arith(op: BinOp, lhs: &Lit, rhs: &Lit) -> Option<Lit> {
    match (lhs, rhs) {
        (Lit::Int(a), Lit::Int(b)) => {
            let folded = match op {
                BinOp::Add => a.checked_add(*b)?,
                BinOp::Sub => a.checked_sub(*b)?,
                BinOp::Mul => a.checked_mul(*b)?,
                _ => return None,
            };
            Some(Lit::Int(folded))
        }
        _ => {
            let (a, b) = numeric_pair(lhs, rhs)?;
            let folded = match op {
                BinOp::Add => a + b,
                BinOp::Sub => a - b,
                BinOp::Mul => a * b,
                _ => return None,
            };
            float_lit(folded)
        }
    }
}

fn eval_div(lhs: &Lit, rhs: &Lit) -> Option<Lit> {
    if matches!((lhs, rhs), (Lit::Int(v), Lit::Int(-1)) if *v == i64::MIN) {
        return None;
    }
    let (a, b) = numeric_pair(lhs, rhs)?;
    if b == 0.0 {
        return None;
    }
    float_lit(a / b)
}

fn eval_mod(lhs: &Lit, rhs: &Lit) -> Option<Lit> {
    match (lhs, rhs) {
        (Lit::Int(a), Lit::Int(b)) if *b != 0 => Some(Lit::Int(r_integer_mod(*a, *b)?)),
        _ => None,
    }
}

fn eval_ordering(op: BinOp, lhs: &Lit, rhs: &Lit) -> Option<Lit> {
    match (lhs, rhs) {
        (Lit::Int(a), Lit::Int(b)) => {
            let result = match op {
                BinOp::Lt => a < b,
                BinOp::Le => a <= b,
                BinOp::Gt => a > b,
                BinOp::Ge => a >= b,
                _ => return None,
            };
            Some(Lit::Bool(result))
        }
        _ => {
            let (a, b) = numeric_pair(lhs, rhs)?;
            let result = match op {
                BinOp::Lt => a < b,
                BinOp::Le => a <= b,
                BinOp::Gt => a > b,
                BinOp::Ge => a >= b,
                _ => return None,
            };
            Some(Lit::Bool(result))
        }
    }
}

fn eval_equality(op: BinOp, lhs: &Lit, rhs: &Lit) -> Option<Lit> {
    let equal = match (lhs, rhs) {
        (Lit::Int(a), Lit::Int(b)) => a == b,
        (Lit::Float(a), Lit::Float(b)) if a.is_finite() && b.is_finite() => a == b,
        (Lit::Int(_), Lit::Float(_)) | (Lit::Float(_), Lit::Int(_)) => {
            let (a, b) = numeric_pair(lhs, rhs)?;
            a == b
        }
        (Lit::Bool(a), Lit::Bool(b)) => a == b,
        (Lit::Str(a), Lit::Str(b)) => a == b,
        _ => return None,
    };
    Some(Lit::Bool(if op == BinOp::Eq { equal } else { !equal }))
}

fn eval_logical(op: BinOp, lhs: &Lit, rhs: &Lit) -> Option<Lit> {
    match (lhs, rhs) {
        (Lit::Bool(a), Lit::Bool(b)) => Some(Lit::Bool(if op == BinOp::And {
            *a && *b
        } else {
            *a || *b
        })),
        _ => None,
    }
}

fn numeric_pair(lhs: &Lit, rhs: &Lit) -> Option<(f64, f64)> {
    Some((numeric_lit(lhs)?, numeric_lit(rhs)?))
}

fn numeric_lit(lit: &Lit) -> Option<f64> {
    match lit {
        Lit::Int(v) => Some(*v as f64),
        Lit::Float(v) if v.is_finite() => Some(*v),
        _ => None,
    }
}

fn float_lit(value: f64) -> Option<Lit> {
    value.is_finite().then_some(Lit::Float(value))
}

fn r_integer_mod(lhs: i64, rhs: i64) -> Option<i64> {
    let a = i128::from(lhs);
    let b = i128::from(rhs);
    let mut q = a / b;
    let rem = a % b;
    if rem != 0 && ((rem > 0) != (b > 0)) {
        q -= 1;
    }
    i64::try_from(a - (q * b)).ok()
}
