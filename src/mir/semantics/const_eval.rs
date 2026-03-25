use crate::diagnostic::DiagnosticBuilder;
use crate::error::{RR, RRCode, RRException, Stage};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use crate::utils::Span;
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) fn collect_reachable_blocks(fn_ir: &FnIR) -> FxHashSet<BlockId> {
    let mut seen = FxHashSet::default();
    let mut stack = vec![fn_ir.entry];
    let mut memo: FxHashMap<ValueId, Option<Lit>> = FxHashMap::default();
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
                let cond_lit = eval_const(fn_ir, cond, &mut memo, &mut FxHashSet::default());
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

pub(super) fn collect_reachable_values(
    fn_ir: &FnIR,
    reachable_blocks: &FxHashSet<BlockId>,
) -> FxHashSet<ValueId> {
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
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    roots.push(*base);
                    roots.push(*i);
                    roots.push(*j);
                    roots.push(*k);
                    roots.push(*val);
                }
            }
        }
    }

    let mut seen = FxHashSet::default();
    let mut stack = roots;
    while let Some(vid) = stack.pop() {
        if !seen.insert(vid) {
            continue;
        }
        let Some(v) = fn_ir.values.get(vid) else {
            continue;
        };
        match &v.kind {
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => {}
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
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
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
            ValueKind::Index3D { base, i, j, k } => {
                stack.push(*base);
                stack.push(*i);
                stack.push(*j);
                stack.push(*k);
            }
        }
    }
    seen
}

pub(super) fn eval_const(
    fn_ir: &FnIR,
    vid: ValueId,
    memo: &mut FxHashMap<ValueId, Option<Lit>>,
    visiting: &mut FxHashSet<ValueId>,
) -> Option<Lit> {
    if let Some(v) = memo.get(&vid) {
        return v.clone();
    }
    if !visiting.insert(vid) {
        return None;
    }
    let out =
        match &fn_ir.values[vid].kind {
            ValueKind::Const(l) => Some(l.clone()),
            ValueKind::Unary { op, rhs } => {
                let r = eval_const(fn_ir, *rhs, memo, visiting)?;
                match (op, r) {
                    (crate::syntax::ast::UnaryOp::Neg, Lit::Int(i)) => Some(Lit::Int(-i)),
                    (crate::syntax::ast::UnaryOp::Neg, Lit::Float(f)) => Some(Lit::Float(-f)),
                    (crate::syntax::ast::UnaryOp::Not, Lit::Bool(b)) => Some(Lit::Bool(!b)),
                    (crate::syntax::ast::UnaryOp::Formula, _) => None,
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
            ValueKind::Call { callee, args, .. } => match callee.as_str() {
                "nrow" if args.len() == 1 => matrix_known_dims(fn_ir, args[0], memo, visiting)
                    .map(|(rows, _)| Lit::Int(rows)),
                "ncol" if args.len() == 1 => matrix_known_dims(fn_ir, args[0], memo, visiting)
                    .map(|(_, cols)| Lit::Int(cols)),
                _ => None,
            },
            ValueKind::Intrinsic { .. } => None,
            _ => None,
        };
    visiting.remove(&vid);
    memo.insert(vid, out.clone());
    out
}

pub(super) fn matrix_known_dims(
    fn_ir: &FnIR,
    vid: ValueId,
    memo: &mut FxHashMap<ValueId, Option<Lit>>,
    visiting: &mut FxHashSet<ValueId>,
) -> Option<(i64, i64)> {
    let mut seen = FxHashSet::default();
    matrix_known_dims_inner(fn_ir, vid, memo, visiting, &mut seen)
}

fn matrix_known_dims_inner(
    fn_ir: &FnIR,
    vid: ValueId,
    memo: &mut FxHashMap<ValueId, Option<Lit>>,
    visiting: &mut FxHashSet<ValueId>,
    seen: &mut FxHashSet<ValueId>,
) -> Option<(i64, i64)> {
    if !seen.insert(vid) {
        return None;
    }
    match &fn_ir.values[vid].kind {
        ValueKind::Call { callee, args, .. } if callee == "matrix" && args.len() >= 3 => {
            let rows = as_integral(&eval_const(fn_ir, args[1], memo, visiting)?)?;
            let cols = as_integral(&eval_const(fn_ir, args[2], memo, visiting)?)?;
            Some((rows, cols))
        }
        ValueKind::Load { var } => {
            let src = unique_assign_source_for_var(fn_ir, var)?;
            matrix_known_dims_inner(fn_ir, src, memo, visiting, seen)
        }
        ValueKind::Phi { args } => {
            let first = matrix_known_dims_inner(fn_ir, args.first()?.0, memo, visiting, seen)?;
            for (src, _) in &args[1..] {
                if matrix_known_dims_inner(fn_ir, *src, memo, visiting, seen)? != first {
                    return None;
                }
            }
            Some(first)
        }
        _ => None,
    }
}

fn unique_assign_source_for_var(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
    let mut found = None;
    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            if let Instr::Assign { dst, src, .. } = instr
                && dst == var
            {
                if found.is_some() {
                    return None;
                }
                found = Some(*src);
            }
        }
    }
    found
}

pub(super) fn eval_binary_const(op: BinOp, lhs: Lit, rhs: Lit) -> Option<Lit> {
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

pub(super) fn validate_const_condition(lit: Lit, span: Span) -> RR<()> {
    match lit {
        Lit::Bool(_) => Ok(()),
        Lit::Na => Err(DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2001,
            Stage::Mir,
            "condition is statically NA".to_string(),
        )
        .at(span)
        .note("R requires TRUE/FALSE in if/while conditions.")
        .constraint(span, "branch conditions must evaluate to TRUE or FALSE")
        .use_site(span, "used here as an if/while condition")
        .fix("guard NA before branching, for example with is.na(...) checks")
        .build()
        .push_frame("mir::semantics::validate_const_condition/2", Some(span))),
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

pub(super) fn validate_index_lit_for_read(lit: Lit, span: Span) -> RR<()> {
    if matches!(lit, Lit::Na) {
        return Ok(());
    }
    validate_index_integral_positive(lit, span)
}

pub(super) fn validate_index_lit_for_write(lit: Lit, span: Span) -> RR<()> {
    if matches!(lit, Lit::Na) {
        return Err(DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2001,
            Stage::Mir,
            "index is statically NA in assignment".to_string(),
        )
        .at(span)
        .constraint(span, "assignment indices must be non-NA integer scalars")
        .use_site(span, "used here as an assignment index")
        .fix("validate or cast the index before assignment")
        .build()
        .push_frame("mir::semantics::validate_index_lit_for_write/2", Some(span)));
    }
    validate_index_integral_positive(lit, span)
}

pub(super) fn validate_index_integral_positive(lit: Lit, span: Span) -> RR<()> {
    let Some(i) = as_integral(&lit) else {
        return Err(DiagnosticBuilder::new(
            "RR.TypeError",
            RRCode::E1002,
            Stage::Mir,
            "index must be an integer scalar".to_string(),
        )
        .at(span)
        .constraint(span, "R indexing expects an integer-like scalar")
        .use_site(span, "used here in an index expression")
        .fix("cast or normalize the index to an integer scalar before indexing")
        .build()
        .push_frame(
            "mir::semantics::validate_index_integral_positive/2",
            Some(span),
        ));
    };
    if i < 1 {
        return Err(DiagnosticBuilder::new(
            "RR.RuntimeError",
            RRCode::E2007,
            Stage::Mir,
            format!("index {} is out of bounds (must be >= 1)", i),
        )
        .at(span)
        .note("R indexing is 1-based at runtime.")
        .constraint(span, "index must be >= 1")
        .use_site(span, "used here in an index expression")
        .fix("shift the index into the 1-based domain before indexing")
        .build()
        .push_frame(
            "mir::semantics::validate_index_integral_positive/2",
            Some(span),
        ));
    }
    Ok(())
}

pub(super) fn as_integral(lit: &Lit) -> Option<i64> {
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

pub(super) fn is_zero_number(lit: &Lit) -> bool {
    match lit {
        Lit::Int(i) => *i == 0,
        Lit::Float(f) => *f == 0.0,
        _ => false,
    }
}
