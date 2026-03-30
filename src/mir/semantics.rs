use crate::diagnostic::{DiagnosticBuilder, finish_diagnostics};
use crate::error::{RR, RRCode, RRException, Stage};
use crate::mir::*;
use crate::utils::did_you_mean;
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "semantics/call_model.rs"]
pub(crate) mod call_model;
#[path = "semantics/const_eval.rs"]
mod const_eval;
#[path = "semantics/runtime_proofs.rs"]
mod runtime_proofs;

use self::call_model::*;
use self::const_eval::*;
use self::runtime_proofs::*;

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

fn suggest_name<I>(name: &str, candidates: I) -> Option<String>
where
    I: IntoIterator<Item = String>,
{
    did_you_mean(name, candidates)
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
                if !assigned_vars.contains(var)
                    && !is_runtime_reserved_symbol(var)
                    && !is_namespaced_r_call(var)
                {
                    let mut err = RRException::new(
                        "RR.SemanticError",
                        RRCode::E1001,
                        Stage::Mir,
                        format!("undefined variable '{}'", var),
                    )
                    .at(v.span)
                    .push_frame("mir::semantics::validate_function/2", Some(v.span))
                    .note("Declare the variable with let before use.");
                    if let Some(suggestion) = suggest_name(var, assigned_vars.iter().cloned()) {
                        err = err.help(suggestion);
                    }
                    errors.push(err);
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
                Instr::StoreIndex1D {
                    base, idx, span, ..
                } => {
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
                    } else if let Some(facts) = out_ranges {
                        let idx_range = facts.get(*idx);
                        if interval_guarantees_above_base_len(&idx_range, *base) {
                            errors.push(
                                DiagnosticBuilder::new(
                                    "RR.RuntimeError",
                                    RRCode::E2007,
                                    Stage::Mir,
                                    "assignment index is guaranteed out of bounds (> length(base))"
                                        .to_string(),
                                )
                                .at(*span)
                                .origin(
                                    fn_ir.values[*idx].span,
                                    format!(
                                        "index range is proven as {}",
                                        format_interval(&idx_range)
                                    ),
                                )
                                .constraint(*span, "assignment index must be <= length(base)")
                                .use_site(*span, "used here as an assignment index")
                                .fix("clamp or guard the index against length(base) before writing")
                                .build(),
                            );
                        }
                    }
                }
                Instr::StoreIndex2D {
                    base, r, c, span, ..
                } => {
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
                        if let Some((rows, cols)) =
                            matrix_known_dims(fn_ir, *base, &mut memo, &mut FxHashSet::default())
                        {
                            let limit = if idx == *r { rows } else { cols };
                            let axis = if idx == *r { "row" } else { "column" };
                            if let Some(lit) =
                                eval_const(fn_ir, idx, &mut memo, &mut FxHashSet::default())
                                && let Some(i) = as_integral(&lit)
                                && i > limit
                            {
                                errors.push(
                                    DiagnosticBuilder::new(
                                        "RR.RuntimeError",
                                        RRCode::E2007,
                                        Stage::Mir,
                                        format!(
                                            "matrix {axis} assignment index is guaranteed out of bounds (>{axis}s={limit})"
                                        ),
                                    )
                                    .at(*span)
                                    .origin(
                                        fn_ir.values[idx].span,
                                        format!("{axis} index is proven constant at {i}"),
                                    )
                                    .constraint(
                                        *span,
                                        format!("matrix {axis} index must be <= {limit}"),
                                    )
                                    .use_site(*span, "used here as a matrix assignment index")
                                    .fix(format!(
                                        "clamp or guard the {axis} index against the matrix extent before writing"
                                    ))
                                    .build(),
                                );
                            } else if let Some(facts) = out_ranges {
                                let idx_range = facts.get(idx);
                                if interval_guarantees_above_const(&idx_range, limit) {
                                    errors.push(
                                        DiagnosticBuilder::new(
                                            "RR.RuntimeError",
                                            RRCode::E2007,
                                            Stage::Mir,
                                            format!(
                                                "matrix {axis} assignment index is guaranteed out of bounds (>{axis}s={limit})"
                                            ),
                                        )
                                        .at(*span)
                                        .origin(
                                            fn_ir.values[idx].span,
                                            format!(
                                                "{axis} index range is proven as {}",
                                                format_interval(&idx_range)
                                            ),
                                        )
                                        .constraint(
                                            *span,
                                            format!("matrix {axis} index must be <= {limit}"),
                                        )
                                        .use_site(*span, "used here as a matrix assignment index")
                                        .fix(format!(
                                            "clamp or guard the {axis} index against the matrix extent before writing"
                                        ))
                                        .build(),
                                    );
                                }
                            }
                        }
                    }
                }
                Instr::StoreIndex3D { i, j, k, span, .. } => {
                    for idx in [*i, *j, *k] {
                        if let Some(lit) =
                            eval_const(fn_ir, idx, &mut memo, &mut FxHashSet::default())
                            && let Err(e) = validate_index_lit_for_write(lit, *span)
                        {
                            errors.push(e);
                        }
                        if let Some(facts) = out_ranges {
                            let idx_range = facts.get(idx);
                            if interval_guarantees_below_one(&idx_range) {
                                errors.push(
                                    DiagnosticBuilder::new(
                                        "RR.RuntimeError",
                                        RRCode::E2007,
                                        Stage::Mir,
                                        "3D assignment index is guaranteed out of bounds (must be >= 1)"
                                            .to_string(),
                                    )
                                    .at(*span)
                                    .origin(
                                        fn_ir.values[idx].span,
                                        format!(
                                            "index range is proven as {}",
                                            format_interval(&idx_range)
                                        ),
                                    )
                                    .constraint(*span, "3D array indexing is 1-based")
                                    .use_site(*span, "used here as a 3D assignment index")
                                    .fix("shift the 3D index into the 1-based domain before writing")
                                    .build(),
                                );
                            }
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
            ValueKind::Index1D { base, idx, .. } => {
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
                    } else if interval_guarantees_above_base_len(&idx_range, *base) {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.RuntimeError",
                                RRCode::E2007,
                                Stage::Mir,
                                "index is guaranteed out of bounds (> length(base))".to_string(),
                            )
                            .at(v.span)
                            .origin(
                                fn_ir.values[*idx].span,
                                format!("index range is proven as {}", format_interval(&idx_range)),
                            )
                            .constraint(v.span, "index must be <= length(base)")
                            .use_site(v.span, "used here in an index read")
                            .fix("clamp or guard the index against length(base) before reading")
                            .build(),
                        );
                    }
                }
            }
            ValueKind::Index2D { base, r, c } => {
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
                        if let Some((rows, cols)) =
                            matrix_known_dims(fn_ir, *base, &mut memo, &mut FxHashSet::default())
                        {
                            let limit = if idx == *r { rows } else { cols };
                            let axis = if idx == *r { "row" } else { "column" };
                            if let Some(lit) =
                                eval_const(fn_ir, idx, &mut memo, &mut FxHashSet::default())
                                && let Some(i) = as_integral(&lit)
                                && i > limit
                            {
                                errors.push(
                                    DiagnosticBuilder::new(
                                        "RR.RuntimeError",
                                        RRCode::E2007,
                                        Stage::Mir,
                                        format!(
                                            "matrix {axis} index is guaranteed out of bounds (>{axis}s={limit})"
                                        ),
                                    )
                                    .at(v.span)
                                    .origin(
                                        fn_ir.values[idx].span,
                                        format!("{axis} index is proven constant at {i}"),
                                    )
                                    .constraint(
                                        v.span,
                                        format!("matrix {axis} index must be <= {limit}"),
                                    )
                                    .use_site(v.span, "used here in a matrix index read")
                                    .fix(format!(
                                        "clamp or guard the {axis} index against the matrix extent before reading"
                                    ))
                                    .build(),
                                );
                            } else if interval_guarantees_above_const(&idx_range, limit) {
                                errors.push(
                                    DiagnosticBuilder::new(
                                        "RR.RuntimeError",
                                        RRCode::E2007,
                                        Stage::Mir,
                                        format!(
                                            "matrix {axis} index is guaranteed out of bounds (>{axis}s={limit})"
                                        ),
                                    )
                                    .at(v.span)
                                    .origin(
                                        fn_ir.values[idx].span,
                                        format!(
                                            "{axis} index range is proven as {}",
                                            format_interval(&idx_range)
                                        ),
                                    )
                                    .constraint(
                                        v.span,
                                        format!("matrix {axis} index must be <= {limit}"),
                                    )
                                    .use_site(v.span, "used here in a matrix index read")
                                    .fix(format!(
                                        "clamp or guard the {axis} index against the matrix extent before reading"
                                    ))
                                    .build(),
                                );
                            }
                        }
                    }
                }
            }
            ValueKind::Index3D { i, j, k, .. } => {
                for idx in [*i, *j, *k] {
                    if let Some(lit) = eval_const(fn_ir, idx, &mut memo, &mut FxHashSet::default())
                        && let Err(e) = validate_index_lit_for_read(lit, v.span)
                    {
                        errors.push(e);
                    }
                    if let Some(facts) = range_out.as_ref() {
                        let bid = bid_for_value(fn_ir, vid);
                        let idx_range = facts[bid].get(idx);
                        if interval_guarantees_below_one(&idx_range) {
                            errors.push(
                                DiagnosticBuilder::new(
                                    "RR.RuntimeError",
                                    RRCode::E2007,
                                    Stage::Mir,
                                    "3D index is guaranteed out of bounds (must be >= 1)"
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
                                .constraint(v.span, "3D array indexing is 1-based at runtime")
                                .use_site(v.span, "used here in a 3D index read")
                                .fix("shift the 3D index into the 1-based domain before reading")
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
