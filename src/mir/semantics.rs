//! Semantic and runtime-safety validation for MIR.
//!
//! The checks here defend the boundary between lowering/optimization and later
//! codegen/runtime execution by rejecting invalid user-visible MIR states.

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

    let mut user_signatures: FxHashMap<String, UserFnSignature> = FxHashMap::default();
    let mut errors = Vec::new();
    for name in &fn_names {
        if let Some(fn_ir) = all_fns.get(name) {
            user_signatures.insert(
                name.clone(),
                UserFnSignature {
                    display_name: fn_ir.user_name.clone().unwrap_or_else(|| name.clone()),
                    param_names: fn_ir.params.clone(),
                    has_default: fn_ir
                        .param_default_r_exprs
                        .iter()
                        .map(Option::is_some)
                        .collect(),
                },
            );
        }
    }

    for name in fn_names {
        if let Some(fn_ir) = all_fns.get(&name) {
            errors.extend(validate_function(fn_ir, &user_signatures));
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

fn validate_function(
    fn_ir: &FnIR,
    user_signatures: &FxHashMap<String, UserFnSignature>,
) -> Vec<RRException> {
    let mut errors = Vec::new();
    let mut assigned_vars: FxHashSet<String> = fn_ir.params.iter().cloned().collect();
    for block in &fn_ir.blocks {
        for ins in &block.instrs {
            if let Instr::Assign { dst, .. } = ins {
                assigned_vars.insert(dst.clone());
            }
        }
    }

    // Dead blocks still originate from user-written statements after terminators
    // such as `return`, so semantic validation must not silently skip them.
    for v in &fn_ir.values {
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
                        format!(
                            "undefined variable '{}' in function '{}'",
                            var, fn_ir.name
                        ),
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
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                if let Err(e) =
                    validate_call_target(callee, args.len(), names, v.span, user_signatures)
                {
                    errors.push(e);
                }
            }
            _ => {}
        }
    }

    errors
}

#[cfg(test)]
mod tests {
    use super::validate_function_runtime;
    use crate::mir::{Facts, FnIR, Lit, Terminator, ValueKind};
    use crate::utils::Span;

    #[test]
    fn runtime_safety_flags_negative_index_through_phi_merged_record_field() {
        let mut f = FnIR::new("runtime_phi_field".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let neg_one = f.add_value(
            ValueKind::Const(Lit::Int(-1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let x1 = f.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let x2 = f.add_value(
            ValueKind::Const(Lit::Int(2)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let x3 = f.add_value(
            ValueKind::Const(Lit::Int(3)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let x = f.add_value(
            ValueKind::Call {
                callee: "c".to_string(),
                args: vec![x1, x2, x3],
                names: vec![None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let rec_a = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("i".to_string(), neg_one)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let rec_b = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("i".to_string(), neg_one)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let rec_phi = f.add_value(
            ValueKind::Phi {
                args: vec![(rec_a, left), (rec_b, right)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        f.values[rec_phi].phi_block = Some(merge);
        let idx = f.add_value(
            ValueKind::FieldGet {
                base: rec_phi,
                field: "i".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let read = f.add_value(
            ValueKind::Index1D {
                base: x,
                idx,
                is_safe: false,
                is_na_safe: false,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(read));

        let errors = validate_function_runtime(&f);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("must be >= 1") || e.message.contains("out of bounds")),
            "expected runtime negative-index diagnostic, got: {errors:#?}"
        );
    }

    #[test]
    fn runtime_safety_does_not_treat_unknown_index_as_proven_below_one() {
        let mut f = FnIR::new("runtime_unknown_index".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let one = f.add_value(
            ValueKind::Const(Lit::Float(1.0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(Lit::Float(2.0)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let base = f.add_value(
            ValueKind::Call {
                callee: "c".to_string(),
                args: vec![one, two],
                names: vec![None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let idx = f.add_value(
            ValueKind::Load {
                var: "ii".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let read = f.add_value(
            ValueKind::Index1D {
                base,
                idx,
                is_safe: false,
                is_na_safe: false,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Return(Some(read));

        let errors = validate_function_runtime(&f);
        assert!(
            !errors
                .iter()
                .any(|e| e.message.contains("must be >= 1") || e.message.contains("out of bounds")),
            "unexpected proven-below-one diagnostic for unknown index: {errors:#?}"
        );
    }

    #[test]
    fn runtime_safety_does_not_treat_unknown_seq_len_arg_as_proven_negative() {
        let mut f = FnIR::new("runtime_unknown_seq_len".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let n = f.add_value(
            ValueKind::Load {
                var: "n".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let seq = f.add_value(
            ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![n],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Return(Some(seq));

        let errors = validate_function_runtime(&f);
        assert!(
            !errors
                .iter()
                .any(|e| e.message.contains("seq_len() with negative length")),
            "unexpected proven-negative seq_len diagnostic for unknown arg: {errors:#?}"
        );
    }

    #[test]
    fn runtime_safety_flags_negative_seq_len_through_nested_record_field() {
        let mut f = FnIR::new("runtime_nested_seq_len".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let neg_one = f.add_value(
            ValueKind::Const(Lit::Int(-1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let inner = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("n".to_string(), neg_one)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let outer = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("inner".to_string(), inner)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let inner_field = f.add_value(
            ValueKind::FieldGet {
                base: outer,
                field: "inner".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let n = f.add_value(
            ValueKind::FieldGet {
                base: inner_field,
                field: "n".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let seq = f.add_value(
            ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![n],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Return(Some(seq));

        let errors = validate_function_runtime(&f);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("seq_len() with negative length")),
            "expected proven-negative seq_len diagnostic, got: {errors:#?}"
        );
    }

    #[test]
    fn runtime_safety_flags_negative_seq_len_through_fieldset_override() {
        let mut f = FnIR::new("runtime_fieldset_seq_len".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let five = f.add_value(
            ValueKind::Const(Lit::Int(5)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let neg_two = f.add_value(
            ValueKind::Const(Lit::Int(-2)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let record = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("n".to_string(), five)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let updated = f.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "n".to_string(),
                value: neg_two,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let n = f.add_value(
            ValueKind::FieldGet {
                base: updated,
                field: "n".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let seq = f.add_value(
            ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![n],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Return(Some(seq));

        let errors = validate_function_runtime(&f);
        assert!(
            errors
                .iter()
                .any(|e| e.message.contains("seq_len() with negative length")),
            "expected proven-negative seq_len diagnostic after fieldset override, got: {errors:#?}"
        );
    }

    #[test]
    fn runtime_safety_does_not_flag_positive_seq_len_after_fieldset_override() {
        let mut f = FnIR::new("runtime_fieldset_seq_len_positive".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let neg_one = f.add_value(
            ValueKind::Const(Lit::Int(-1)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let five = f.add_value(
            ValueKind::Const(Lit::Int(5)),
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let record = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("n".to_string(), neg_one)],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let updated = f.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "n".to_string(),
                value: five,
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let n = f.add_value(
            ValueKind::FieldGet {
                base: updated,
                field: "n".to_string(),
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );
        let seq = f.add_value(
            ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![n],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Return(Some(seq));

        let errors = validate_function_runtime(&f);
        assert!(
            !errors
                .iter()
                .any(|e| e.message.contains("seq_len() with negative length")),
            "unexpected proven-negative seq_len diagnostic after positive override: {errors:#?}"
        );
    }
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
        let mut targets: Vec<ValueId> = fn_ir
            .values
            .iter()
            .flat_map(|value| match value.kind {
                ValueKind::Binary {
                    op: BinOp::Div | BinOp::Mod,
                    rhs,
                    ..
                } => vec![rhs],
                ValueKind::Call {
                    ref callee,
                    ref args,
                    ..
                } if callee == "seq_len" && args.len() == 1 => vec![args[0]],
                ValueKind::Index1D { idx, .. } => vec![idx],
                ValueKind::Index2D { r, c, .. } => vec![r, c],
                ValueKind::Index3D { i, j, k, .. } => vec![i, j, k],
                _ => Vec::new(),
            })
            .collect();
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                match *instr {
                    Instr::StoreIndex1D { idx, .. } => targets.push(idx),
                    Instr::StoreIndex2D { r, c, .. } => {
                        targets.push(r);
                        targets.push(c);
                    }
                    Instr::StoreIndex3D { i, j, k, .. } => {
                        targets.push(i);
                        targets.push(j);
                        targets.push(k);
                    }
                    Instr::Assign { .. } | Instr::Eval { .. } => {}
                }
            }
        }
        targets
    } else {
        Vec::new()
    };
    let dataflow = needs
        .needs_dataflow
        .then(|| crate::mir::flow::DataflowSolver::analyze_values(fn_ir, &dataflow_targets));
    let dataflow_interval =
        |value: ValueId| dataflow.as_ref().and_then(|facts| facts.get(&value).map(|f| f.interval));

    // Proof correspondence:
    // `proof/runtime_safety_correspondence.md` ties this reduced runtime slice
    // to `RuntimeSafetyFieldRangeSubset`: range analysis first preserves exact
    // field-derived intervals, then the hazard helpers in
    // `semantics/runtime_proofs.rs` turn those intervals into the E2007-style
    // `< 1` / `< 0` checks used below for indexing and `seq_len()`.
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
                    } else if flow_interval_guarantees_below_one(dataflow_interval(*idx)) {
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
                                "dataflow proves the index is always < 1",
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
                        } else if flow_interval_guarantees_below_one(dataflow_interval(idx)) {
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
                                    "dataflow proves the matrix index is always < 1",
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
                            } else if flow_interval_guarantees_below_one(dataflow_interval(idx)) {
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
                                        "dataflow proves the 3D index is always < 1",
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
                    } else if flow_interval_guarantees_below_one(dataflow_interval(*idx)) {
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
                                "dataflow proves the index is always < 1",
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
                        } else if flow_interval_guarantees_below_one(dataflow_interval(idx)) {
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
                                    "dataflow proves the matrix index is always < 1",
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
                        } else if flow_interval_guarantees_below_one(dataflow_interval(idx)) {
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
                                    "dataflow proves the 3D index is always < 1",
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
                }) || dataflow.as_ref().is_some_and(|facts| {
                    flow_interval_guarantees_negative(facts.get(&args[0]).map(|f| f.interval))
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
