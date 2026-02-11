use crate::error::{RR, RRCode, RRException, Stage};
use crate::mir::*;
use crate::utils::Span;
use std::collections::{HashMap, HashSet};

pub fn validate_program(all_fns: &HashMap<String, FnIR>) -> RR<()> {
    let mut user_arities: HashMap<String, usize> = HashMap::new();
    let mut errors = Vec::new();
    for (name, fn_ir) in all_fns {
        user_arities.insert(name.clone(), fn_ir.params.len());
    }

    for fn_ir in all_fns.values() {
        errors.extend(validate_function(fn_ir, &user_arities));
    }
    finish_validation(
        "RR.SemanticError",
        RRCode::E1002,
        Stage::Mir,
        "semantic validation failed",
        errors,
    )
}

pub fn validate_runtime_safety(all_fns: &HashMap<String, FnIR>) -> RR<()> {
    let mut errors = Vec::new();
    for fn_ir in all_fns.values() {
        errors.extend(validate_function_runtime(fn_ir));
    }
    finish_validation(
        "RR.RuntimeError",
        RRCode::E2001,
        Stage::Mir,
        "runtime safety validation failed",
        errors,
    )
}

fn validate_function(fn_ir: &FnIR, user_arities: &HashMap<String, usize>) -> Vec<RRException> {
    let mut errors = Vec::new();
    let mut assigned_vars: HashSet<String> = fn_ir.params.iter().cloned().collect();
    for block in &fn_ir.blocks {
        for ins in &block.instrs {
            if let Instr::Assign { dst, .. } = ins {
                assigned_vars.insert(dst.clone());
            }
        }
    }

    for v in &fn_ir.values {
        match &v.kind {
            ValueKind::Load { var } => {
                if !assigned_vars.contains(var) && !is_runtime_reserved_symbol(var) {
                    errors.push(
                        RRException::new(
                            "RR.SemanticError",
                            RRCode::E1001,
                            Stage::Mir,
                            format!("undefined variable '{}'", var),
                        )
                        .at(v.span)
                        .push_frame("mir::semantics::validate_function/2", Some(v.span))
                        .note("Declare the variable with let before use."),
                    );
                }
            }
            ValueKind::Call { callee, args, .. } => {
                if let Err(e) = validate_call_target(callee, args.len(), v.span, user_arities) {
                    errors.push(e);
                }
            }
            _ => {}
        }
    }

    errors
}

fn validate_function_runtime(fn_ir: &FnIR) -> Vec<RRException> {
    let mut errors = Vec::new();
    let mut memo: HashMap<ValueId, Option<Lit>> = HashMap::new();
    let reachable_blocks = collect_reachable_blocks(fn_ir);
    let reachable_values = collect_reachable_values(fn_ir, &reachable_blocks);

    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        if !reachable_blocks.contains(&bid) {
            continue;
        }
        if let Terminator::If { cond, .. } = block.term {
            if reachable_values.contains(&cond)
                && let Some(lit) = eval_const(fn_ir, cond, &mut memo, &mut HashSet::new())
            {
                if let Err(e) = validate_const_condition(lit, fn_ir.values[cond].span) {
                    errors.push(e);
                }
            }
        }

        for ins in &block.instrs {
            match ins {
                Instr::StoreIndex1D { idx, span, .. } => {
                    if let Some(lit) = eval_const(fn_ir, *idx, &mut memo, &mut HashSet::new()) {
                        if let Err(e) = validate_index_lit_for_write(lit, *span) {
                            errors.push(e);
                        }
                    }
                }
                Instr::StoreIndex2D { r, c, span, .. } => {
                    if let Some(lit) = eval_const(fn_ir, *r, &mut memo, &mut HashSet::new()) {
                        if let Err(e) = validate_index_lit_for_write(lit, *span) {
                            errors.push(e);
                        }
                    }
                    if let Some(lit) = eval_const(fn_ir, *c, &mut memo, &mut HashSet::new()) {
                        if let Err(e) = validate_index_lit_for_write(lit, *span) {
                            errors.push(e);
                        }
                    }
                }
                _ => {}
            }
        }

        let _ = bid;
    }

    for (vid, v) in fn_ir.values.iter().enumerate() {
        if !reachable_values.contains(&vid) {
            continue;
        }
        match &v.kind {
            ValueKind::Binary { op, rhs, .. } if matches!(op, BinOp::Div | BinOp::Mod) => {
                if let Some(lit) = eval_const(fn_ir, *rhs, &mut memo, &mut HashSet::new()) {
                    if is_zero_number(&lit) {
                        errors.push(
                            RRException::new(
                                "RR.RuntimeError",
                                RRCode::E2001,
                                Stage::Mir,
                                "division by zero is guaranteed at compile-time".to_string(),
                            )
                            .at(v.span)
                            .push_frame("mir::semantics::validate_function_runtime/1", Some(v.span))
                            .note("Adjust divisor so it cannot become zero."),
                        );
                    }
                }
            }
            ValueKind::Index1D { idx, .. } => {
                if let Some(lit) = eval_const(fn_ir, *idx, &mut memo, &mut HashSet::new()) {
                    if let Err(e) = validate_index_lit_for_read(lit, v.span) {
                        errors.push(e);
                    }
                }
            }
            ValueKind::Index2D { r, c, .. } => {
                if let Some(lit) = eval_const(fn_ir, *r, &mut memo, &mut HashSet::new()) {
                    if let Err(e) = validate_index_lit_for_read(lit, v.span) {
                        errors.push(e);
                    }
                }
                if let Some(lit) = eval_const(fn_ir, *c, &mut memo, &mut HashSet::new()) {
                    if let Err(e) = validate_index_lit_for_read(lit, v.span) {
                        errors.push(e);
                    }
                }
            }
            ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
                if let Some(lit) = eval_const(fn_ir, args[0], &mut memo, &mut HashSet::new()) {
                    if let Some(n) = as_integral(&lit) {
                        if n < 0 {
                            errors.push(
                                RRException::new(
                                    "RR.RuntimeError",
                                    RRCode::E2007,
                                    Stage::Mir,
                                    "seq_len() with negative length is guaranteed to fail"
                                        .to_string(),
                                )
                                .at(v.span)
                                .push_frame(
                                    "mir::semantics::validate_function_runtime/1",
                                    Some(v.span),
                                )
                                .note("Ensure seq_len argument is >= 0."),
                            );
                        }
                    }
                }
            }
            _ => {
                let _ = vid;
            }
        }
    }

    errors
}

fn collect_reachable_blocks(fn_ir: &FnIR) -> HashSet<BlockId> {
    let mut seen = HashSet::new();
    let mut stack = vec![fn_ir.entry];
    let mut memo: HashMap<ValueId, Option<Lit>> = HashMap::new();
    while let Some(bb) = stack.pop() {
        if !seen.insert(bb) {
            continue;
        }
        let Some(block) = fn_ir.blocks.get(bb) else {
            continue;
        };
        match block.term {
            Terminator::Goto(next) => stack.push(next),
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => {
                let cond_lit = eval_const(fn_ir, cond, &mut memo, &mut HashSet::new());
                match cond_lit {
                    Some(Lit::Bool(true)) => stack.push(then_bb),
                    Some(Lit::Bool(false)) => stack.push(else_bb),
                    _ => {
                        stack.push(then_bb);
                        stack.push(else_bb);
                    }
                }
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }
    seen
}

fn collect_reachable_values(fn_ir: &FnIR, reachable_blocks: &HashSet<BlockId>) -> HashSet<ValueId> {
    let mut roots = Vec::new();
    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        if !reachable_blocks.contains(&bid) {
            continue;
        }
        match &block.term {
            Terminator::If { cond, .. } => roots.push(*cond),
            Terminator::Return(Some(v)) => roots.push(*v),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
        for ins in &block.instrs {
            match ins {
                Instr::Assign { src, .. } => roots.push(*src),
                Instr::Eval { val, .. } => roots.push(*val),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    roots.push(*base);
                    roots.push(*idx);
                    roots.push(*val);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    roots.push(*base);
                    roots.push(*r);
                    roots.push(*c);
                    roots.push(*val);
                }
            }
        }
    }

    let mut seen = HashSet::new();
    let mut stack = roots;
    while let Some(vid) = stack.pop() {
        if !seen.insert(vid) {
            continue;
        }
        let Some(v) = fn_ir.values.get(vid) else {
            continue;
        };
        match &v.kind {
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => {}
            ValueKind::Phi { args } => {
                for (src, _) in args {
                    stack.push(*src);
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => stack.push(*base),
            ValueKind::Range { start, end } => {
                stack.push(*start);
                stack.push(*end);
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                stack.push(*lhs);
                stack.push(*rhs);
            }
            ValueKind::Unary { rhs, .. } => stack.push(*rhs),
            ValueKind::Call { args, .. } => {
                for arg in args {
                    stack.push(*arg);
                }
            }
            ValueKind::Index1D { base, idx, .. } => {
                stack.push(*base);
                stack.push(*idx);
            }
            ValueKind::Index2D { base, r, c } => {
                stack.push(*base);
                stack.push(*r);
                stack.push(*c);
            }
        }
    }
    seen
}

fn eval_const(
    fn_ir: &FnIR,
    vid: ValueId,
    memo: &mut HashMap<ValueId, Option<Lit>>,
    visiting: &mut HashSet<ValueId>,
) -> Option<Lit> {
    if let Some(v) = memo.get(&vid) {
        return v.clone();
    }
    if !visiting.insert(vid) {
        return None;
    }
    let out = match &fn_ir.values[vid].kind {
        ValueKind::Const(l) => Some(l.clone()),
        ValueKind::Unary { op, rhs } => {
            let r = eval_const(fn_ir, *rhs, memo, visiting)?;
            match (op, r) {
                (crate::syntax::ast::UnaryOp::Neg, Lit::Int(i)) => Some(Lit::Int(-i)),
                (crate::syntax::ast::UnaryOp::Neg, Lit::Float(f)) => Some(Lit::Float(-f)),
                (crate::syntax::ast::UnaryOp::Not, Lit::Bool(b)) => Some(Lit::Bool(!b)),
                _ => None,
            }
        }
        ValueKind::Binary { op, lhs, rhs } => {
            let l = eval_const(fn_ir, *lhs, memo, visiting)?;
            let r = eval_const(fn_ir, *rhs, memo, visiting)?;
            eval_binary_const(*op, l, r)
        }
        ValueKind::Phi { args } => {
            if args.is_empty() {
                None
            } else {
                let first = eval_const(fn_ir, args[0].0, memo, visiting)?;
                for (v, _) in &args[1..] {
                    if eval_const(fn_ir, *v, memo, visiting) != Some(first.clone()) {
                        return None;
                    }
                }
                Some(first)
            }
        }
        _ => None,
    };
    visiting.remove(&vid);
    memo.insert(vid, out.clone());
    out
}

fn eval_binary_const(op: BinOp, lhs: Lit, rhs: Lit) -> Option<Lit> {
    use crate::syntax::ast::BinOp::*;
    match op {
        Add => match (lhs, rhs) {
            (Lit::Int(a), Lit::Int(b)) => Some(Lit::Int(a + b)),
            (Lit::Float(a), Lit::Float(b)) => Some(Lit::Float(a + b)),
            (Lit::Int(a), Lit::Float(b)) => Some(Lit::Float(a as f64 + b)),
            (Lit::Float(a), Lit::Int(b)) => Some(Lit::Float(a + b as f64)),
            _ => None,
        },
        Sub => match (lhs, rhs) {
            (Lit::Int(a), Lit::Int(b)) => Some(Lit::Int(a - b)),
            (Lit::Float(a), Lit::Float(b)) => Some(Lit::Float(a - b)),
            (Lit::Int(a), Lit::Float(b)) => Some(Lit::Float(a as f64 - b)),
            (Lit::Float(a), Lit::Int(b)) => Some(Lit::Float(a - b as f64)),
            _ => None,
        },
        Mul => match (lhs, rhs) {
            (Lit::Int(a), Lit::Int(b)) => Some(Lit::Int(a * b)),
            (Lit::Float(a), Lit::Float(b)) => Some(Lit::Float(a * b)),
            (Lit::Int(a), Lit::Float(b)) => Some(Lit::Float(a as f64 * b)),
            (Lit::Float(a), Lit::Int(b)) => Some(Lit::Float(a * b as f64)),
            _ => None,
        },
        Div => match (lhs, rhs) {
            (Lit::Int(a), Lit::Int(b)) => Some(Lit::Float(a as f64 / b as f64)),
            (Lit::Float(a), Lit::Float(b)) => Some(Lit::Float(a / b)),
            (Lit::Int(a), Lit::Float(b)) => Some(Lit::Float(a as f64 / b)),
            (Lit::Float(a), Lit::Int(b)) => Some(Lit::Float(a / b as f64)),
            _ => None,
        },
        Mod => match (lhs, rhs) {
            (Lit::Int(_), Lit::Int(0)) => None,
            (Lit::Int(a), Lit::Int(b)) => Some(Lit::Int(a % b)),
            _ => None,
        },
        Eq => Some(Lit::Bool(lhs == rhs)),
        Ne => Some(Lit::Bool(lhs != rhs)),
        Lt | Le | Gt | Ge => {
            let (a, b) = match (lhs, rhs) {
                (Lit::Int(a), Lit::Int(b)) => (a as f64, b as f64),
                (Lit::Float(a), Lit::Float(b)) => (a, b),
                (Lit::Int(a), Lit::Float(b)) => (a as f64, b),
                (Lit::Float(a), Lit::Int(b)) => (a, b as f64),
                _ => return None,
            };
            let r = match op {
                Lt => a < b,
                Le => a <= b,
                Gt => a > b,
                Ge => a >= b,
                _ => false,
            };
            Some(Lit::Bool(r))
        }
        And | Or => match (lhs, rhs) {
            (Lit::Bool(a), Lit::Bool(b)) => {
                Some(Lit::Bool(if op == And { a && b } else { a || b }))
            }
            _ => None,
        },
        _ => None,
    }
}

fn validate_const_condition(lit: Lit, span: Span) -> RR<()> {
    match lit {
        Lit::Bool(_) => Ok(()),
        Lit::Na => Err(RRException::new(
            "RR.RuntimeError",
            RRCode::E2001,
            Stage::Mir,
            "condition is statically NA".to_string(),
        )
        .at(span)
        .push_frame("mir::semantics::validate_const_condition/2", Some(span))
        .note("R requires TRUE/FALSE in if/while conditions.")),
        _ => Err(RRException::new(
            "RR.TypeError",
            RRCode::E1002,
            Stage::Mir,
            "condition must be logical scalar".to_string(),
        )
        .at(span)
        .push_frame("mir::semantics::validate_const_condition/2", Some(span))),
    }
}

fn validate_index_lit_for_read(lit: Lit, span: Span) -> RR<()> {
    if matches!(lit, Lit::Na) {
        return Ok(());
    }
    validate_index_integral_positive(lit, span)
}

fn validate_index_lit_for_write(lit: Lit, span: Span) -> RR<()> {
    if matches!(lit, Lit::Na) {
        return Err(RRException::new(
            "RR.RuntimeError",
            RRCode::E2001,
            Stage::Mir,
            "index is statically NA in assignment".to_string(),
        )
        .at(span)
        .push_frame("mir::semantics::validate_index_lit_for_write/2", Some(span)));
    }
    validate_index_integral_positive(lit, span)
}

fn validate_index_integral_positive(lit: Lit, span: Span) -> RR<()> {
    let Some(i) = as_integral(&lit) else {
        return Err(RRException::new(
            "RR.TypeError",
            RRCode::E1002,
            Stage::Mir,
            "index must be an integer scalar".to_string(),
        )
        .at(span)
        .push_frame(
            "mir::semantics::validate_index_integral_positive/2",
            Some(span),
        ));
    };
    if i < 1 {
        return Err(RRException::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            format!("index {} is out of bounds (must be >= 1)", i),
        )
        .at(span)
        .push_frame(
            "mir::semantics::validate_index_integral_positive/2",
            Some(span),
        )
        .note("R indexing is 1-based at runtime."));
    }
    Ok(())
}

fn as_integral(lit: &Lit) -> Option<i64> {
    match lit {
        Lit::Int(i) => Some(*i),
        Lit::Float(f) => {
            if f.is_finite() && (*f - f.trunc()).abs() < f64::EPSILON {
                Some(*f as i64)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn is_zero_number(lit: &Lit) -> bool {
    match lit {
        Lit::Int(i) => *i == 0,
        Lit::Float(f) => *f == 0.0,
        _ => false,
    }
}

fn validate_call_target(
    callee: &str,
    argc: usize,
    span: Span,
    user_arities: &HashMap<String, usize>,
) -> RR<()> {
    if let Some(expected) = user_arities.get(callee) {
        if *expected != argc {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Mir,
                format!(
                    "function '{}' expects {} argument(s), got {}",
                    callee, expected, argc
                ),
            )
            .at(span)
            .push_frame("mir::semantics::validate_call_target/4", Some(span)));
        }
        return Ok(());
    }

    if let Some((min, max)) = builtin_arity(callee) {
        if argc < min || max.is_some_and(|m| argc > m) {
            let upper = max
                .map(|m| m.to_string())
                .unwrap_or_else(|| "inf".to_string());
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Mir,
                format!(
                    "builtin '{}' expects {}..{} argument(s), got {}",
                    callee, min, upper, argc
                ),
            )
            .at(span)
            .push_frame("mir::semantics::validate_call_target/4", Some(span)));
        }
        return Ok(());
    }

    if is_dynamic_fallback_builtin(callee) || is_runtime_helper(callee) {
        return Ok(());
    }

    Err(RRException::new(
        "RR.SemanticError",
        RRCode::E1001,
        Stage::Mir,
        format!("undefined function '{}'", callee),
    )
    .at(span)
    .push_frame("mir::semantics::validate_call_target/4", Some(span))
    .note("Define the function before calling it, or import the module that provides it."))
}

fn builtin_arity(name: &str) -> Option<(usize, Option<usize>)> {
    match name {
        "length" | "seq_len" | "seq_along" | "abs" | "sqrt" | "sin" | "cos" | "tan" | "asin"
        | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "log10" | "log2" | "exp" | "sign"
        | "gamma" | "lgamma" | "floor" | "ceiling" | "trunc" | "colSums" | "rowSums" => {
            Some((1, Some(1)))
        }
        "atan2" => Some((2, Some(2))),
        "round" | "log" => Some((1, Some(2))),
        "pmax" | "pmin" => Some((2, None)),
        "sum" | "mean" | "var" | "sd" | "min" | "max" | "prod" | "print" | "c" | "list" => {
            Some((1, None))
        }
        "numeric" => Some((1, Some(1))),
        "rep.int" => Some((2, Some(2))),
        "vector" => Some((1, Some(2))),
        "matrix" => Some((1, Some(4))),
        "crossprod" | "tcrossprod" => Some((1, Some(2))),
        _ => None,
    }
}

fn is_dynamic_fallback_builtin(name: &str) -> bool {
    matches!(
        name,
        "eval"
            | "parse"
            | "get"
            | "assign"
            | "exists"
            | "mget"
            | "rm"
            | "ls"
            | "parent.frame"
            | "environment"
            | "sys.frame"
            | "sys.call"
            | "do.call"
    )
}

fn is_runtime_helper(name: &str) -> bool {
    name.starts_with("rr_")
}

fn is_runtime_reserved_symbol(name: &str) -> bool {
    name.starts_with(".phi_")
        || name.starts_with(".tachyon_")
        || name.starts_with("Sym_")
        || name.starts_with("__lambda_")
        || name.starts_with("rr_")
}

fn finish_validation(
    module: &'static str,
    code: RRCode,
    stage: Stage,
    summary: &str,
    mut errors: Vec<RRException>,
) -> RR<()> {
    if errors.is_empty() {
        return Ok(());
    }
    if errors.len() == 1 {
        return Err(errors.remove(0));
    }
    Err(RRException::aggregate(
        module,
        code,
        stage,
        format!("{}: {} error(s)", summary, errors.len()),
        errors,
    ))
}
