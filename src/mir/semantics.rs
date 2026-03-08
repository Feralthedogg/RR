use crate::diagnostic::{DiagnosticBuilder, finish_diagnostics};
use crate::error::{RR, RRCode, RRException, Stage};
use crate::mir::*;
use crate::utils::Span;
use rustc_hash::{FxHashMap, FxHashSet};

pub fn validate_program(all_fns: &FxHashMap<String, FnIR>) -> RR<()> {
    let mut fn_names: Vec<String> = all_fns.keys().cloned().collect();
    fn_names.sort();

    let mut user_arities: FxHashMap<String, usize> = FxHashMap::default();
    let mut errors = Vec::new();
    for name in &fn_names {
        if let Some(fn_ir) = all_fns.get(name) {
            user_arities.insert(name.clone(), fn_ir.params.len());
        }
    }

    for name in fn_names {
        if let Some(fn_ir) = all_fns.get(&name) {
            errors.extend(validate_function(fn_ir, &user_arities));
        }
    }
    finish_diagnostics(
        "RR.SemanticError",
        RRCode::E1002,
        Stage::Mir,
        format!("semantic validation failed: {} error(s)", errors.len()),
        errors,
    )
}

pub fn validate_runtime_safety(all_fns: &FxHashMap<String, FnIR>) -> RR<()> {
    let mut fn_names: Vec<String> = all_fns.keys().cloned().collect();
    fn_names.sort();

    let mut errors = Vec::new();
    for name in fn_names {
        if let Some(fn_ir) = all_fns.get(&name) {
            errors.extend(validate_function_runtime(fn_ir));
        }
    }
    finish_diagnostics(
        "RR.RuntimeError",
        RRCode::E2001,
        Stage::Mir,
        format!(
            "runtime safety validation failed: {} error(s)",
            errors.len()
        ),
        errors,
    )
}

fn validate_function(fn_ir: &FnIR, user_arities: &FxHashMap<String, usize>) -> Vec<RRException> {
    let mut errors = Vec::new();
    let reachable_blocks = collect_reachable_blocks(fn_ir);
    let reachable_values = collect_reachable_values(fn_ir, &reachable_blocks);
    let mut assigned_vars: FxHashSet<String> = fn_ir.params.iter().cloned().collect();
    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        if !reachable_blocks.contains(&bid) {
            continue;
        }
        for ins in &block.instrs {
            if let Instr::Assign { dst, .. } = ins {
                assigned_vars.insert(dst.clone());
            }
        }
    }

    for (vid, v) in fn_ir.values.iter().enumerate() {
        if !reachable_values.contains(&vid) {
            continue;
        }
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
    let mut memo: FxHashMap<ValueId, Option<Lit>> = FxHashMap::default();
    let reachable_blocks = collect_reachable_blocks(fn_ir);
    let reachable_values = collect_reachable_values(fn_ir, &reachable_blocks);
    let needs = runtime_safety_needs(fn_ir);
    let na_states = needs
        .needs_na
        .then(|| crate::mir::analyze::na::compute_na_states(fn_ir));
    let range_in = needs
        .needs_range
        .then(|| crate::mir::analyze::range::analyze_ranges(fn_ir));
    let range_out = range_in.as_ref().map(|facts| {
        let mut out = facts.clone();
        for (bid, block_facts) in out.iter_mut().enumerate() {
            crate::mir::analyze::range::transfer_block(bid, fn_ir, block_facts);
        }
        out
    });
    let dataflow_targets: Vec<ValueId> = if needs.needs_dataflow {
        fn_ir
            .values
            .iter()
            .filter_map(|value| match value.kind {
                ValueKind::Binary {
                    op: BinOp::Div | BinOp::Mod,
                    rhs,
                    ..
                } => Some(rhs),
                _ => None,
            })
            .collect()
    } else {
        Vec::new()
    };
    let dataflow = needs
        .needs_dataflow
        .then(|| crate::mir::flow::DataflowSolver::analyze_values(fn_ir, &dataflow_targets));

    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        if !reachable_blocks.contains(&bid) {
            continue;
        }
        let out_ranges = range_out.as_ref().map(|facts| &facts[bid]);
        if let Terminator::If { cond, .. } = block.term
            && reachable_values.contains(&cond)
            && let Some(lit) = eval_const(fn_ir, cond, &mut memo, &mut FxHashSet::default())
            && let Err(e) = validate_const_condition(lit, fn_ir.values[cond].span)
        {
            errors.push(e);
        } else if let Terminator::If { cond, .. } = block.term
            && reachable_values.contains(&cond)
            && na_states.as_ref().is_some_and(|states| {
                matches!(states[cond], crate::mir::analyze::na::NaState::Always)
            })
        {
            errors.push(
                DiagnosticBuilder::new(
                    "RR.RuntimeError",
                    RRCode::E2001,
                    Stage::Mir,
                    "condition is guaranteed to evaluate to NA at runtime".to_string(),
                )
                .at(fn_ir.values[cond].span)
                .origin(
                    fn_ir.values[cond].span,
                    "condition value originates here and propagates NA on all paths",
                )
                .constraint(
                    fn_ir.values[cond].span,
                    "branch conditions must evaluate to TRUE or FALSE",
                )
                .use_site(
                    fn_ir.values[cond].span,
                    "used here as an if/while condition",
                )
                .fix("guard NA before branching, for example with is.na(...) checks")
                .build(),
            );
        }

        for ins in &block.instrs {
            match ins {
                Instr::StoreIndex1D { idx, span, .. } => {
                    if let Some(lit) = eval_const(fn_ir, *idx, &mut memo, &mut FxHashSet::default())
                        && let Err(e) = validate_index_lit_for_write(lit, *span)
                    {
                        errors.push(e);
                    } else if na_states.as_ref().is_some_and(|states| {
                        matches!(states[*idx], crate::mir::analyze::na::NaState::Always)
                    }) {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.RuntimeError",
                                RRCode::E2001,
                                Stage::Mir,
                                "index is guaranteed to evaluate to NA in assignment".to_string(),
                            )
                            .at(*span)
                            .origin(
                                fn_ir.values[*idx].span,
                                "index value originates here and is always NA",
                            )
                            .constraint(*span, "assignment indices must be non-NA integer scalars")
                            .use_site(*span, "used here as an assignment index")
                            .fix("validate or cast the index before assignment")
                            .build(),
                        );
                    } else if let Some(facts) = out_ranges
                        && interval_guarantees_below_one(&facts.get(*idx))
                    {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.RuntimeError",
                                RRCode::E2007,
                                Stage::Mir,
                                "assignment index is guaranteed out of bounds (must be >= 1)"
                                    .to_string(),
                            )
                            .at(*span)
                            .origin(
                                fn_ir.values[*idx].span,
                                format!(
                                    "index range is proven as {}",
                                    format_interval(&facts.get(*idx))
                                ),
                            )
                            .constraint(*span, "R assignment indexing is 1-based")
                            .use_site(*span, "used here as an assignment index")
                            .fix("shift the index into the 1-based domain before writing")
                            .build(),
                        );
                    }
                }
                Instr::StoreIndex2D { r, c, span, .. } => {
                    if let Some(lit) = eval_const(fn_ir, *r, &mut memo, &mut FxHashSet::default())
                        && let Err(e) = validate_index_lit_for_write(lit, *span)
                    {
                        errors.push(e);
                    }
                    if let Some(lit) = eval_const(fn_ir, *c, &mut memo, &mut FxHashSet::default())
                        && let Err(e) = validate_index_lit_for_write(lit, *span)
                    {
                        errors.push(e);
                    }
                    for idx in [*r, *c] {
                        if na_states.as_ref().is_some_and(|states| {
                            matches!(states[idx], crate::mir::analyze::na::NaState::Always)
                        }) {
                            errors.push(
                                DiagnosticBuilder::new(
                                    "RR.RuntimeError",
                                    RRCode::E2001,
                                    Stage::Mir,
                                    "matrix assignment index is guaranteed to evaluate to NA"
                                        .to_string(),
                                )
                                .at(*span)
                                .origin(
                                    fn_ir.values[idx].span,
                                    "matrix index originates here and is always NA",
                                )
                                .constraint(
                                    *span,
                                    "matrix assignment indices must be non-NA integer scalars",
                                )
                                .use_site(*span, "used here as a matrix assignment index")
                                .fix("validate or cast the matrix index before assignment")
                                .build(),
                            );
                        } else if let Some(facts) = out_ranges
                            && interval_guarantees_below_one(&facts.get(idx))
                        {
                            errors.push(
                                DiagnosticBuilder::new(
                                    "RR.RuntimeError",
                                    RRCode::E2007,
                                    Stage::Mir,
                                    "matrix assignment index is guaranteed out of bounds (must be >= 1)"
                                        .to_string(),
                                )
                                .at(*span)
                                .origin(
                                    fn_ir.values[idx].span,
                                    format!(
                                        "index range is proven as {}",
                                        format_interval(&facts.get(idx))
                                    ),
                                )
                                .constraint(*span, "matrix indexing is 1-based")
                                .use_site(*span, "used here as a matrix assignment index")
                                .fix("shift the index into the 1-based domain before writing")
                                .build(),
                            );
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
            ValueKind::Binary {
                op: BinOp::Div | BinOp::Mod,
                rhs,
                ..
            } => {
                if let Some(lit) = eval_const(fn_ir, *rhs, &mut memo, &mut FxHashSet::default())
                    && is_zero_number(&lit)
                {
                    errors.push(division_by_zero_diagnostic(
                        v.span,
                        fn_ir.values[*rhs].span,
                        "division by zero is guaranteed at compile-time",
                    ));
                } else if interval_guarantees_zero(
                    dataflow
                        .as_ref()
                        .and_then(|facts| facts.get(rhs).map(|f| f.interval)),
                ) || range_out.as_ref().is_some_and(|facts| {
                    interval_guarantees_zero(range_interval_to_fact_interval(
                        facts,
                        bid_for_value(fn_ir, vid),
                        *rhs,
                    ))
                }) {
                    errors.push(division_by_zero_diagnostic(
                        v.span,
                        fn_ir.values[*rhs].span,
                        "division by zero is guaranteed by range/dataflow analysis",
                    ));
                }
            }
            ValueKind::Index1D { idx, .. } => {
                if let Some(lit) = eval_const(fn_ir, *idx, &mut memo, &mut FxHashSet::default())
                    && let Err(e) = validate_index_lit_for_read(lit, v.span)
                {
                    errors.push(e);
                } else if let Some(facts) = range_out.as_ref() {
                    let bid = bid_for_value(fn_ir, vid);
                    let idx_range = facts[bid].get(*idx);
                    if interval_guarantees_below_one(&idx_range) {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.RuntimeError",
                                RRCode::E2007,
                                Stage::Mir,
                                "index is guaranteed out of bounds (must be >= 1)".to_string(),
                            )
                            .at(v.span)
                            .origin(
                                fn_ir.values[*idx].span,
                                format!("index range is proven as {}", format_interval(&idx_range)),
                            )
                            .constraint(v.span, "R indexing is 1-based at runtime")
                            .use_site(v.span, "used here in an index read")
                            .fix("shift the index into the 1-based domain before reading")
                            .build(),
                        );
                    }
                }
            }
            ValueKind::Index2D { r, c, .. } => {
                if let Some(lit) = eval_const(fn_ir, *r, &mut memo, &mut FxHashSet::default())
                    && let Err(e) = validate_index_lit_for_read(lit, v.span)
                {
                    errors.push(e);
                }
                if let Some(lit) = eval_const(fn_ir, *c, &mut memo, &mut FxHashSet::default())
                    && let Err(e) = validate_index_lit_for_read(lit, v.span)
                {
                    errors.push(e);
                }
                for idx in [*r, *c] {
                    if let Some(facts) = range_out.as_ref() {
                        let bid = bid_for_value(fn_ir, vid);
                        let idx_range = facts[bid].get(idx);
                        if interval_guarantees_below_one(&idx_range) {
                            errors.push(
                                DiagnosticBuilder::new(
                                    "RR.RuntimeError",
                                    RRCode::E2007,
                                    Stage::Mir,
                                    "matrix index is guaranteed out of bounds (must be >= 1)"
                                        .to_string(),
                                )
                                .at(v.span)
                                .origin(
                                    fn_ir.values[idx].span,
                                    format!(
                                        "index range is proven as {}",
                                        format_interval(&idx_range)
                                    ),
                                )
                                .constraint(v.span, "R matrix indexing is 1-based at runtime")
                                .use_site(v.span, "used here in a matrix index read")
                                .fix(
                                    "shift the row/column index into the 1-based domain before reading",
                                )
                                .build(),
                            );
                        }
                    }
                }
            }
            ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
                if let Some(lit) = eval_const(fn_ir, args[0], &mut memo, &mut FxHashSet::default())
                    && let Some(n) = as_integral(&lit)
                    && n < 0
                {
                    errors.push(seq_len_negative_diagnostic(
                        v.span,
                        fn_ir.values[args[0]].span,
                    ));
                } else if range_out.as_ref().is_some_and(|facts| {
                    interval_guarantees_negative(&facts[bid_for_value(fn_ir, vid)].get(args[0]))
                }) {
                    errors.push(seq_len_negative_diagnostic(
                        v.span,
                        fn_ir.values[args[0]].span,
                    ));
                }
            }
            _ => {
                let _ = vid;
            }
        }
    }

    errors
}

struct RuntimeSafetyNeeds {
    needs_na: bool,
    needs_range: bool,
    needs_dataflow: bool,
}

fn runtime_safety_needs(fn_ir: &FnIR) -> RuntimeSafetyNeeds {
    let mut needs = RuntimeSafetyNeeds {
        needs_na: false,
        needs_range: false,
        needs_dataflow: false,
    };

    for block in &fn_ir.blocks {
        if matches!(block.term, Terminator::If { .. }) {
            needs.needs_na = true;
        }
        for instr in &block.instrs {
            if matches!(
                instr,
                Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. }
            ) {
                needs.needs_na = true;
                needs.needs_range = true;
            }
        }
    }

    for value in &fn_ir.values {
        match &value.kind {
            ValueKind::Binary {
                op: BinOp::Div | BinOp::Mod,
                ..
            } => {
                needs.needs_dataflow = true;
                needs.needs_range = true;
            }
            ValueKind::Index1D { .. } | ValueKind::Index2D { .. } => {
                needs.needs_range = true;
            }
            ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
                needs.needs_range = true;
            }
            _ => {}
        }
    }

    needs
}

fn collect_reachable_blocks(fn_ir: &FnIR) -> FxHashSet<BlockId> {
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

fn collect_reachable_values(
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
            ValueKind::Call { args, .. } => {
                for arg in args {
                    stack.push(*arg);
                }
            }
            ValueKind::Intrinsic { args, .. } => {
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
    memo: &mut FxHashMap<ValueId, Option<Lit>>,
    visiting: &mut FxHashSet<ValueId>,
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
        ValueKind::Intrinsic { .. } => None,
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
    user_arities: &FxHashMap<String, usize>,
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

    if is_dynamic_fallback_builtin(callee)
        || is_namespaced_r_call(callee)
        || is_supported_package_call(callee)
        || is_tidy_helper_call(callee)
        || is_supported_tidy_helper_call(callee)
        || is_runtime_helper(callee)
    {
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
        | "gamma" | "lgamma" | "floor" | "ceiling" | "trunc" | "colSums" | "rowSums" | "is.na"
        | "is.finite" => Some((1, Some(1))),
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
            | "library"
            | "require"
            | "png"
            | "plot"
            | "lines"
            | "legend"
            | "dev.off"
    )
}

fn is_namespaced_r_call(name: &str) -> bool {
    let Some((pkg, sym)) = name.split_once("::") else {
        return false;
    };
    !pkg.is_empty() && !sym.is_empty() && !pkg.contains(':') && !sym.contains(':')
}

fn is_tidy_helper_call(name: &str) -> bool {
    matches!(
        name,
        "starts_with"
            | "ends_with"
            | "contains"
            | "matches"
            | "everything"
            | "all_of"
            | "any_of"
            | "where"
            | "desc"
            | "between"
            | "n"
            | "row_number"
    )
}

fn is_supported_package_call(name: &str) -> bool {
    matches!(
        name,
        "base::data.frame"
            | "stats::median"
            | "stats::sd"
            | "stats::lm"
            | "stats::predict"
            | "stats::quantile"
            | "stats::glm"
            | "stats::as.formula"
            | "readr::read_csv"
            | "readr::write_csv"
            | "tidyr::pivot_longer"
            | "tidyr::pivot_wider"
            | "graphics::plot"
            | "graphics::lines"
            | "graphics::legend"
            | "grDevices::png"
            | "grDevices::dev.off"
            | "ggplot2::aes"
            | "ggplot2::ggplot"
            | "ggplot2::geom_line"
            | "ggplot2::geom_point"
            | "ggplot2::ggtitle"
            | "ggplot2::theme_minimal"
            | "ggplot2::ggsave"
            | "dplyr::mutate"
            | "dplyr::filter"
            | "dplyr::select"
            | "dplyr::summarise"
            | "dplyr::arrange"
            | "dplyr::group_by"
            | "dplyr::rename"
    )
}

fn is_supported_tidy_helper_call(name: &str) -> bool {
    is_tidy_helper_call(name)
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

fn division_by_zero_diagnostic(use_span: Span, origin_span: Span, message: &str) -> RRException {
    DiagnosticBuilder::new(
        "RR.RuntimeError",
        RRCode::E2001,
        Stage::Mir,
        message.to_string(),
    )
    .at(use_span)
    .origin(
        origin_span,
        "divisor originates here and is proven to be zero",
    )
    .constraint(use_span, "division and modulo require a non-zero divisor")
    .use_site(use_span, "used here as a divisor")
    .fix("guard the divisor or clamp it away from zero before division")
    .build()
}

fn seq_len_negative_diagnostic(use_span: Span, origin_span: Span) -> RRException {
    DiagnosticBuilder::new(
        "RR.RuntimeError",
        RRCode::E2007,
        Stage::Mir,
        "seq_len() with negative length is guaranteed to fail".to_string(),
    )
    .at(use_span)
    .origin(
        origin_span,
        "length value originates here and is proven negative",
    )
    .constraint(use_span, "seq_len() requires an argument >= 0")
    .use_site(use_span, "used here as the seq_len() length")
    .fix("clamp the length to 0 or prove it non-negative before calling seq_len()")
    .build()
}

fn interval_guarantees_below_one(intv: &crate::mir::analyze::range::RangeInterval) -> bool {
    upper_const(intv).is_some_and(|hi| hi < 1)
}

fn interval_guarantees_negative(intv: &crate::mir::analyze::range::RangeInterval) -> bool {
    upper_const(intv).is_some_and(|hi| hi < 0)
}

fn upper_const(intv: &crate::mir::analyze::range::RangeInterval) -> Option<i64> {
    match intv.hi {
        crate::mir::analyze::range::SymbolicBound::Const(v) => Some(v),
        _ => None,
    }
}

fn format_interval(intv: &crate::mir::analyze::range::RangeInterval) -> String {
    format!("[{}, {}]", format_bound(&intv.lo), format_bound(&intv.hi))
}

fn format_bound(bound: &crate::mir::analyze::range::SymbolicBound) -> String {
    match bound {
        crate::mir::analyze::range::SymbolicBound::NegInf => "-inf".to_string(),
        crate::mir::analyze::range::SymbolicBound::PosInf => "+inf".to_string(),
        crate::mir::analyze::range::SymbolicBound::Const(v) => v.to_string(),
        crate::mir::analyze::range::SymbolicBound::VarPlus(v, off) => {
            format!("v{}+{}", v, off)
        }
        crate::mir::analyze::range::SymbolicBound::LenOf(v, off) => {
            format!("len(v{})+{}", v, off)
        }
    }
}

fn interval_guarantees_zero(interval: Option<crate::mir::flow::Interval>) -> bool {
    interval.is_some_and(|intv| intv.min == 0 && intv.max == 0)
}

fn range_interval_to_fact_interval(
    range_in: &[crate::mir::analyze::range::RangeFacts],
    bid: BlockId,
    vid: ValueId,
) -> Option<crate::mir::flow::Interval> {
    let intv = range_in.get(bid)?.get(vid);
    let lo = upper_const_from_bound(&intv.lo)?;
    let hi = upper_const_from_bound(&intv.hi)?;
    Some(crate::mir::flow::Interval::new(lo, hi))
}

fn upper_const_from_bound(bound: &crate::mir::analyze::range::SymbolicBound) -> Option<i64> {
    match bound {
        crate::mir::analyze::range::SymbolicBound::Const(v) => Some(*v),
        _ => None,
    }
}

fn bid_for_value(fn_ir: &FnIR, vid: ValueId) -> BlockId {
    let mut seen = FxHashSet::default();
    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        for ins in &block.instrs {
            match ins {
                Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                    if root_depends_on_value(fn_ir, *src, vid, &mut seen) {
                        return bid;
                    }
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    if root_depends_on_value(fn_ir, *base, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *idx, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *val, vid, &mut seen)
                    {
                        return bid;
                    }
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    if root_depends_on_value(fn_ir, *base, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *r, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *c, vid, &mut seen)
                        || root_depends_on_value(fn_ir, *val, vid, &mut seen)
                    {
                        return bid;
                    }
                }
            }
        }
        match block.term {
            Terminator::If { cond, .. } => {
                if root_depends_on_value(fn_ir, cond, vid, &mut seen) {
                    return bid;
                }
            }
            Terminator::Return(Some(ret)) => {
                if root_depends_on_value(fn_ir, ret, vid, &mut seen) {
                    return bid;
                }
            }
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }
    fn_ir.entry
}

fn root_depends_on_value(
    fn_ir: &FnIR,
    root: ValueId,
    target: ValueId,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    if root == target {
        return true;
    }
    if !seen.insert(root) {
        return false;
    }
    let depends = match &fn_ir.values[root].kind {
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
        ValueKind::Phi { args } => args
            .iter()
            .any(|(src, _)| root_depends_on_value(fn_ir, *src, target, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            root_depends_on_value(fn_ir, *base, target, seen)
        }
        ValueKind::Range { start, end } => {
            root_depends_on_value(fn_ir, *start, target, seen)
                || root_depends_on_value(fn_ir, *end, target, seen)
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            root_depends_on_value(fn_ir, *lhs, target, seen)
                || root_depends_on_value(fn_ir, *rhs, target, seen)
        }
        ValueKind::Unary { rhs, .. } => root_depends_on_value(fn_ir, *rhs, target, seen),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| root_depends_on_value(fn_ir, *arg, target, seen)),
        ValueKind::Index1D { base, idx, .. } => {
            root_depends_on_value(fn_ir, *base, target, seen)
                || root_depends_on_value(fn_ir, *idx, target, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            root_depends_on_value(fn_ir, *base, target, seen)
                || root_depends_on_value(fn_ir, *r, target, seen)
                || root_depends_on_value(fn_ir, *c, target, seen)
        }
    };
    seen.remove(&root);
    depends
}
