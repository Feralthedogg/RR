//! IR rewriting routines that materialize approved vectorization plans.
//!
//! Analysis and planning choose the vector form; this module owns the actual
//! MIR mutation that installs vector values, repairs shadow state, and rewrites
//! exit-path uses to preserve scalar semantics.

#[path = "transform_linear.rs"]
mod transform_linear;
use self::transform_linear::*;

use super::analysis::{
    as_safe_loop_index, canonical_value, choose_call_map_lowering, expr_has_iv_dependency,
    hoist_vector_expr_temp, induction_origin_var, intrinsic_for_call, is_const_number,
    is_const_one, is_invariant_reduce_scalar, is_iv_equivalent, is_loop_invariant_scalar_expr,
    loop_entry_seed_source_in_loop, maybe_hoist_callmap_arg_expr, resolve_base_var,
    resolve_materialized_value, rewrite_returns_for_var, same_length_proven, value_depends_on,
    vector_length_key,
};
use super::debug::vectorize_trace_enabled;
use super::reconstruct::{
    MaterializedExprKey, add_int_offset, adjusted_loop_limit, build_loop_index_vector,
    has_assignment_in_loop, has_non_passthrough_assignment_in_loop, intern_materialized_value,
    is_int_index_vector_value, is_scalar_broadcast_value, materialize_loop_invariant_scalar_expr,
    materialize_vector_expr, unique_assign_source_in_loop,
    unique_assign_source_reaching_block_in_loop, value_use_block_in_loop,
};
use super::types::{
    Axis3D, CallMap3DApplyPlan, CallMap3DGeneralApplyPlan, CallMapArg, CallMapLoweringMode,
    CondMap3DApplyPlan, CondMap3DGeneralApplyPlan, ExprMap3DApplyPlan, ExprMapEntry,
    ExprMapEntry3D, Map2DApplyPlan, Map3DApplyPlan, PreparedVectorAssignment,
    RecurrenceAddConst3DApplyPlan, RecurrenceAddConstApplyPlan, Reduce2DApplyPlan,
    Reduce3DApplyPlan, ReduceCondEntry, ReduceKind, ScatterExprMap3DApplyPlan,
    ScatterExprMap3DGeneralApplyPlan, ShiftedMap3DApplyPlan, ShiftedMapApplyPlan,
    VectorAccessOperand3D, VectorApplySite, VectorLoopRange, VectorPlan,
};
use crate::mir::opt::loop_analysis::{LoopInfo, build_pred_map};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::{FxHashMap, FxHashSet};

pub(crate) fn vector_apply_site(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorApplySite> {
    let preds = build_pred_map(fn_ir);
    let outer_preds: Vec<BlockId> = preds
        .get(&lp.header)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|b| !lp.body.contains(b))
        .collect();

    if outer_preds.len() != 1 {
        return None;
    }
    if lp.exits.len() != 1 {
        return None;
    }
    if !matches!(fn_ir.blocks[outer_preds[0]].term, Terminator::Goto(next) if next == lp.header) {
        return None;
    }

    Some(VectorApplySite {
        preheader: outer_preds[0],
        exit_bb: lp.exits[0],
    })
}

pub(crate) fn finish_vector_assignment(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    dest_var: VarId,
    out_val: ValueId,
) -> bool {
    finish_vector_assignment_with_shadow_states(fn_ir, site, dest_var, out_val, &[], None)
}

pub(crate) fn finish_vector_assignments(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    assignments: Vec<PreparedVectorAssignment>,
) -> bool {
    emit_prepared_vector_assignments(fn_ir, site, assignments)
}

fn has_reachable_assignment_to_var_after(fn_ir: &FnIR, start: BlockId, var: &str) -> bool {
    let mut seen = FxHashSet::default();
    let mut stack = vec![start];
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        let Some(block) = fn_ir.blocks.get(bid) else {
            continue;
        };
        for instr in &block.instrs {
            match instr {
                Instr::Assign { dst, .. } if dst == var => return true,
                Instr::StoreIndex1D { base, .. }
                | Instr::StoreIndex2D { base, .. }
                | Instr::StoreIndex3D { base, .. } => {
                    if resolve_base_var(fn_ir, *base).as_deref() == Some(var) {
                        return true;
                    }
                }
                Instr::Assign { .. } | Instr::Eval { .. } => {}
            }
        }
        match block.term {
            Terminator::Goto(next) => stack.push(next),
            Terminator::If {
                then_bb, else_bb, ..
            } => {
                stack.push(then_bb);
                stack.push(else_bb);
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }
    false
}

fn rewrite_reachable_value_uses_for_var_after(
    fn_ir: &mut FnIR,
    start: BlockId,
    var: &str,
    replacement: ValueId,
) {
    fn rewrite_value_tree_for_var(
        fn_ir: &mut FnIR,
        root: ValueId,
        var: &str,
        replacement: ValueId,
        memo: &mut FxHashMap<ValueId, ValueId>,
        visiting: &mut FxHashSet<ValueId>,
    ) -> ValueId {
        let root = canonical_value(fn_ir, root);
        if let Some(mapped) = memo.get(&root) {
            return *mapped;
        }
        if !visiting.insert(root) {
            return root;
        }

        let mapped = match fn_ir.values[root].kind.clone() {
            ValueKind::Load { var: load_var } if load_var == var => root,
            kind if fn_ir.values[root].origin_var.as_deref() == Some(var) => match kind {
                ValueKind::Load { var: load_var } if load_var == var => root,
                _ => replacement,
            },
            ValueKind::Load { .. } => root,
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => root,
            ValueKind::Unary { op, rhs } => {
                let rhs_new =
                    rewrite_value_tree_for_var(fn_ir, rhs, var, replacement, memo, visiting);
                if rhs_new == rhs {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::Unary { op, rhs: rhs_new },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::Binary { op, lhs, rhs } => {
                let lhs_new =
                    rewrite_value_tree_for_var(fn_ir, lhs, var, replacement, memo, visiting);
                let rhs_new =
                    rewrite_value_tree_for_var(fn_ir, rhs, var, replacement, memo, visiting);
                if lhs_new == lhs && rhs_new == rhs {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::Binary {
                            op,
                            lhs: lhs_new,
                            rhs: rhs_new,
                        },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::RecordLit { fields } => {
                let new_fields: Vec<(String, ValueId)> = fields
                    .iter()
                    .map(|(name, value)| {
                        (
                            name.clone(),
                            rewrite_value_tree_for_var(
                                fn_ir,
                                *value,
                                var,
                                replacement,
                                memo,
                                visiting,
                            ),
                        )
                    })
                    .collect();
                if new_fields == fields {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::RecordLit { fields: new_fields },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::FieldGet { base, field } => {
                let base_new =
                    rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                if base_new == base {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::FieldGet {
                            base: base_new,
                            field,
                        },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::FieldSet { base, field, value } => {
                let base_new =
                    rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                let value_new =
                    rewrite_value_tree_for_var(fn_ir, value, var, replacement, memo, visiting);
                if base_new == base && value_new == value {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::FieldSet {
                            base: base_new,
                            field,
                            value: value_new,
                        },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                let new_args: Vec<ValueId> = args
                    .iter()
                    .map(|arg| {
                        rewrite_value_tree_for_var(fn_ir, *arg, var, replacement, memo, visiting)
                    })
                    .collect();
                if new_args == args {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::Call {
                            callee,
                            args: new_args,
                            names,
                        },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::Intrinsic { op, args } => {
                let new_args: Vec<ValueId> = args
                    .iter()
                    .map(|arg| {
                        rewrite_value_tree_for_var(fn_ir, *arg, var, replacement, memo, visiting)
                    })
                    .collect();
                if new_args == args {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::Intrinsic { op, args: new_args },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::Phi { args } => {
                let new_args: Vec<(ValueId, BlockId)> = args
                    .iter()
                    .map(|(arg, bb)| {
                        (
                            rewrite_value_tree_for_var(
                                fn_ir,
                                *arg,
                                var,
                                replacement,
                                memo,
                                visiting,
                            ),
                            *bb,
                        )
                    })
                    .collect();
                if new_args == args {
                    root
                } else {
                    let phi = fn_ir.add_value(
                        ValueKind::Phi { args: new_args },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    );
                    fn_ir.values[phi].phi_block = fn_ir.values[root].phi_block;
                    phi
                }
            }
            ValueKind::Len { base } => {
                let base_new =
                    rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                if base_new == base {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::Len { base: base_new },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::Indices { base } => {
                let base_new =
                    rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                if base_new == base {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::Indices { base: base_new },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::Range { start, end } => {
                let start_new =
                    rewrite_value_tree_for_var(fn_ir, start, var, replacement, memo, visiting);
                let end_new =
                    rewrite_value_tree_for_var(fn_ir, end, var, replacement, memo, visiting);
                if start_new == start && end_new == end {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::Range {
                            start: start_new,
                            end: end_new,
                        },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                let base_new =
                    rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                let idx_new =
                    rewrite_value_tree_for_var(fn_ir, idx, var, replacement, memo, visiting);
                if base_new == base && idx_new == idx {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::Index1D {
                            base: base_new,
                            idx: idx_new,
                            is_safe,
                            is_na_safe,
                        },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::Index2D { base, r, c } => {
                let base_new =
                    rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                let r_new = rewrite_value_tree_for_var(fn_ir, r, var, replacement, memo, visiting);
                let c_new = rewrite_value_tree_for_var(fn_ir, c, var, replacement, memo, visiting);
                if base_new == base && r_new == r && c_new == c {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::Index2D {
                            base: base_new,
                            r: r_new,
                            c: c_new,
                        },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
            ValueKind::Index3D { base, i, j, k } => {
                let base_new =
                    rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                let i_new = rewrite_value_tree_for_var(fn_ir, i, var, replacement, memo, visiting);
                let j_new = rewrite_value_tree_for_var(fn_ir, j, var, replacement, memo, visiting);
                let k_new = rewrite_value_tree_for_var(fn_ir, k, var, replacement, memo, visiting);
                if base_new == base && i_new == i && j_new == j && k_new == k {
                    root
                } else {
                    fn_ir.add_value(
                        ValueKind::Index3D {
                            base: base_new,
                            i: i_new,
                            j: j_new,
                            k: k_new,
                        },
                        fn_ir.values[root].span,
                        fn_ir.values[root].facts,
                        fn_ir.values[root].origin_var.clone(),
                    )
                }
            }
        };

        visiting.remove(&root);
        memo.insert(root, mapped);
        mapped
    }

    let mut seen = FxHashSet::default();
    let mut stack = vec![start];
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        let Some(block) = fn_ir.blocks.get(bid) else {
            continue;
        };
        let succs = match block.term {
            Terminator::Goto(next) => vec![next],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![then_bb, else_bb],
            Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
        };
        let mut memo = FxHashMap::default();
        let mut visiting = FxHashSet::default();
        let instrs = fn_ir.blocks[bid].instrs.clone();
        let mut new_instrs = Vec::with_capacity(instrs.len());
        for mut instr in instrs {
            match &mut instr {
                Instr::Assign { src, .. } => {
                    *src = rewrite_value_tree_for_var(
                        fn_ir,
                        *src,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                }
                Instr::Eval { val, .. } => {
                    *val = rewrite_value_tree_for_var(
                        fn_ir,
                        *val,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    *base = rewrite_value_tree_for_var(
                        fn_ir,
                        *base,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                    *idx = rewrite_value_tree_for_var(
                        fn_ir,
                        *idx,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                    *val = rewrite_value_tree_for_var(
                        fn_ir,
                        *val,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    *base = rewrite_value_tree_for_var(
                        fn_ir,
                        *base,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                    *r = rewrite_value_tree_for_var(
                        fn_ir,
                        *r,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                    *c = rewrite_value_tree_for_var(
                        fn_ir,
                        *c,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                    *val = rewrite_value_tree_for_var(
                        fn_ir,
                        *val,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    *base = rewrite_value_tree_for_var(
                        fn_ir,
                        *base,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                    *i = rewrite_value_tree_for_var(
                        fn_ir,
                        *i,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                    *j = rewrite_value_tree_for_var(
                        fn_ir,
                        *j,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                    *k = rewrite_value_tree_for_var(
                        fn_ir,
                        *k,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                    *val = rewrite_value_tree_for_var(
                        fn_ir,
                        *val,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    );
                }
            }
            new_instrs.push(instr);
        }
        fn_ir.blocks[bid].instrs = new_instrs;
        let term = std::mem::replace(&mut fn_ir.blocks[bid].term, Terminator::Unreachable);
        fn_ir.blocks[bid].term = match term {
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => Terminator::If {
                cond: rewrite_value_tree_for_var(
                    fn_ir,
                    cond,
                    var,
                    replacement,
                    &mut memo,
                    &mut visiting,
                ),
                then_bb,
                else_bb,
            },
            Terminator::Return(Some(ret)) => Terminator::Return(Some(rewrite_value_tree_for_var(
                fn_ir,
                ret,
                var,
                replacement,
                &mut memo,
                &mut visiting,
            ))),
            other => other,
        };
        stack.extend(succs);
    }
}

pub(crate) fn finish_vector_assignments_versioned(
    fn_ir: &mut FnIR,
    fallback_bb: BlockId,
    site: VectorApplySite,
    assignments: Vec<PreparedVectorAssignment>,
    guard_cond: ValueId,
) -> bool {
    if assignments.is_empty()
        || assignments
            .iter()
            .any(|assignment| !assignment.shadow_vars.is_empty() || assignment.shadow_idx.is_some())
    {
        return false;
    }

    let preds = build_pred_map(fn_ir);
    let original_exit_preds = preds
        .get(&site.exit_bb)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|pred| *pred != site.preheader)
        .collect::<Vec<_>>();
    if original_exit_preds.is_empty() {
        return false;
    }

    let apply_bb = fn_ir.add_block();
    for assignment in &assignments {
        fn_ir.blocks[apply_bb].instrs.push(Instr::Assign {
            dst: assignment.dest_var.clone(),
            src: assignment.out_val,
            span: crate::utils::Span::dummy(),
        });
    }
    fn_ir.blocks[apply_bb].term = Terminator::Goto(site.exit_bb);
    fn_ir.blocks[site.preheader].term = Terminator::If {
        cond: guard_cond,
        then_bb: apply_bb,
        else_bb: fallback_bb,
    };

    for assignment in assignments {
        let scalar_val = fn_ir.add_value(
            ValueKind::Load {
                var: assignment.dest_var.clone(),
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            Some(assignment.dest_var.clone()),
        );
        let mut phi_args = original_exit_preds
            .iter()
            .map(|pred| (scalar_val, *pred))
            .collect::<Vec<_>>();
        phi_args.push((assignment.out_val, apply_bb));
        let phi = fn_ir.add_value(
            ValueKind::Phi { args: phi_args },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            Some(assignment.dest_var.clone()),
        );
        fn_ir.values[phi].phi_block = Some(site.exit_bb);
        rewrite_reachable_value_uses_for_var_after(fn_ir, site.exit_bb, &assignment.dest_var, phi);
        if !has_reachable_assignment_to_var_after(fn_ir, site.exit_bb, &assignment.dest_var) {
            rewrite_returns_for_var(fn_ir, &assignment.dest_var, phi);
        }
    }
    true
}

pub(super) fn build_shadow_state_read(fn_ir: &mut FnIR, out_val: ValueId, idx: ValueId) -> ValueId {
    let out_val = resolve_materialized_value(fn_ir, out_val);
    let idx = resolve_materialized_value(fn_ir, idx);
    let ctx = fn_ir.add_value(
        ValueKind::Const(Lit::Str("index".to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_index1_read".to_string(),
            args: vec![out_val, idx, ctx],
            names: vec![None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

pub(super) fn emit_vector_preheader_eval(fn_ir: &mut FnIR, preheader: BlockId, val: ValueId) {
    fn_ir.blocks[preheader].instrs.push(Instr::Eval {
        val,
        span: crate::utils::Span::dummy(),
    });
}

pub(super) fn emit_same_len_guard(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    lhs: ValueId,
    rhs: ValueId,
) {
    let check_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_same_len".to_string(),
            args: vec![lhs, rhs],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    emit_vector_preheader_eval(fn_ir, preheader, check_val);
}

pub(super) fn emit_same_or_scalar_guard(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    lhs: ValueId,
    rhs: ValueId,
) {
    let check_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_same_or_scalar".to_string(),
            args: vec![lhs, rhs],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    emit_vector_preheader_eval(fn_ir, preheader, check_val);
}

pub(crate) fn emit_same_matrix_shape_or_scalar_guard(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    lhs: ValueId,
    rhs: ValueId,
) {
    let check_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_same_matrix_shape_or_scalar".to_string(),
            args: vec![lhs, rhs],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    emit_vector_preheader_eval(fn_ir, preheader, check_val);
}

pub(crate) fn emit_same_array3_shape_or_scalar_guard(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    lhs: ValueId,
    rhs: ValueId,
) {
    let check_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_same_array3_shape_or_scalar".to_string(),
            args: vec![lhs, rhs],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    emit_vector_preheader_eval(fn_ir, preheader, check_val);
}

pub(crate) fn build_slice_assignment_value(
    fn_ir: &mut FnIR,
    dest: ValueId,
    start: ValueId,
    end: ValueId,
    value: ValueId,
) -> ValueId {
    fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_assign_slice".to_string(),
            args: vec![dest, start, end, value],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn prepare_cond_map_operands(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    dest_ref: ValueId,
    cond_vec: ValueId,
    then_vec: ValueId,
    else_vec: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
) -> (ValueId, ValueId, ValueId) {
    if whole_dest {
        if !same_length_proven(fn_ir, dest_ref, cond_vec) {
            emit_same_len_guard(fn_ir, preheader, dest_ref, cond_vec);
        }
        for branch_vec in [then_vec, else_vec] {
            if is_const_number(fn_ir, branch_vec) || same_length_proven(fn_ir, dest_ref, branch_vec)
            {
                continue;
            }
            emit_same_or_scalar_guard(fn_ir, preheader, dest_ref, branch_vec);
        }
        return (cond_vec, then_vec, else_vec);
    }

    (
        prepare_partial_slice_value(fn_ir, dest_ref, cond_vec, start, end),
        prepare_partial_slice_value(fn_ir, dest_ref, then_vec, start, end),
        prepare_partial_slice_value(fn_ir, dest_ref, else_vec, start, end),
    )
}

pub(super) fn emit_prepared_vector_assignments(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    assignments: Vec<PreparedVectorAssignment>,
) -> bool {
    fn has_reachable_assignment_to_var(fn_ir: &FnIR, start: BlockId, var: &str) -> bool {
        let mut seen = FxHashSet::default();
        let mut stack = vec![start];
        while let Some(bid) = stack.pop() {
            if !seen.insert(bid) {
                continue;
            }
            let Some(block) = fn_ir.blocks.get(bid) else {
                continue;
            };
            for instr in &block.instrs {
                match instr {
                    Instr::Assign { dst, .. } if dst == var => return true,
                    Instr::StoreIndex1D { base, .. }
                    | Instr::StoreIndex2D { base, .. }
                    | Instr::StoreIndex3D { base, .. } => {
                        if resolve_base_var(fn_ir, *base).as_deref() == Some(var) {
                            return true;
                        }
                    }
                    Instr::Assign { .. } | Instr::Eval { .. } => {}
                }
            }
            match block.term {
                Terminator::Goto(next) => stack.push(next),
                Terminator::If {
                    then_bb, else_bb, ..
                } => {
                    stack.push(then_bb);
                    stack.push(else_bb);
                }
                Terminator::Return(_) | Terminator::Unreachable => {}
            }
        }
        false
    }

    fn rewrite_reachable_value_uses_for_var(
        fn_ir: &mut FnIR,
        start: BlockId,
        var: &str,
        replacement: ValueId,
    ) {
        fn rewrite_value_tree_for_var(
            fn_ir: &mut FnIR,
            root: ValueId,
            var: &str,
            replacement: ValueId,
            memo: &mut FxHashMap<ValueId, ValueId>,
            visiting: &mut FxHashSet<ValueId>,
        ) -> ValueId {
            let root = canonical_value(fn_ir, root);
            if let Some(mapped) = memo.get(&root) {
                return *mapped;
            }
            if !visiting.insert(root) {
                return root;
            }

            let mapped = match fn_ir.values[root].kind.clone() {
                ValueKind::Load { var: load_var } if load_var == var => root,
                kind if fn_ir.values[root].origin_var.as_deref() == Some(var) => match kind {
                    ValueKind::Load { var: load_var } if load_var == var => root,
                    _ => replacement,
                },
                ValueKind::Load { .. } => root,
                ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => root,
                ValueKind::Unary { op, rhs } => {
                    let rhs_new =
                        rewrite_value_tree_for_var(fn_ir, rhs, var, replacement, memo, visiting);
                    if rhs_new == rhs {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Unary { op, rhs: rhs_new },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Binary { op, lhs, rhs } => {
                    let lhs_new =
                        rewrite_value_tree_for_var(fn_ir, lhs, var, replacement, memo, visiting);
                    let rhs_new =
                        rewrite_value_tree_for_var(fn_ir, rhs, var, replacement, memo, visiting);
                    if lhs_new == lhs && rhs_new == rhs {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Binary {
                                op,
                                lhs: lhs_new,
                                rhs: rhs_new,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::RecordLit { fields } => {
                    let new_fields: Vec<(String, ValueId)> = fields
                        .iter()
                        .map(|(name, value)| {
                            (
                                name.clone(),
                                rewrite_value_tree_for_var(
                                    fn_ir,
                                    *value,
                                    var,
                                    replacement,
                                    memo,
                                    visiting,
                                ),
                            )
                        })
                        .collect();
                    if new_fields == fields {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::RecordLit { fields: new_fields },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::FieldGet { base, field } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    if base_new == base {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::FieldGet {
                                base: base_new,
                                field,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::FieldSet { base, field, value } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    let value_new =
                        rewrite_value_tree_for_var(fn_ir, value, var, replacement, memo, visiting);
                    if base_new == base && value_new == value {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::FieldSet {
                                base: base_new,
                                field,
                                value: value_new,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Call {
                    callee,
                    args,
                    names,
                } => {
                    let new_args: Vec<ValueId> = args
                        .iter()
                        .map(|arg| {
                            rewrite_value_tree_for_var(
                                fn_ir,
                                *arg,
                                var,
                                replacement,
                                memo,
                                visiting,
                            )
                        })
                        .collect();
                    if new_args == args {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Call {
                                callee,
                                args: new_args,
                                names,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Intrinsic { op, args } => {
                    let new_args: Vec<ValueId> = args
                        .iter()
                        .map(|arg| {
                            rewrite_value_tree_for_var(
                                fn_ir,
                                *arg,
                                var,
                                replacement,
                                memo,
                                visiting,
                            )
                        })
                        .collect();
                    if new_args == args {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Intrinsic { op, args: new_args },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Phi { args } => {
                    let new_args: Vec<(ValueId, BlockId)> = args
                        .iter()
                        .map(|(arg, bid)| {
                            (
                                rewrite_value_tree_for_var(
                                    fn_ir,
                                    *arg,
                                    var,
                                    replacement,
                                    memo,
                                    visiting,
                                ),
                                *bid,
                            )
                        })
                        .collect();
                    if new_args == args {
                        root
                    } else {
                        let phi = fn_ir.add_value(
                            ValueKind::Phi {
                                args: new_args.clone(),
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        );
                        fn_ir.values[phi].phi_block = fn_ir.values[root].phi_block;
                        phi
                    }
                }
                ValueKind::Len { base } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    if base_new == base {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Len { base: base_new },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Indices { base } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    if base_new == base {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Indices { base: base_new },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Range { start, end } => {
                    let start_new =
                        rewrite_value_tree_for_var(fn_ir, start, var, replacement, memo, visiting);
                    let end_new =
                        rewrite_value_tree_for_var(fn_ir, end, var, replacement, memo, visiting);
                    if start_new == start && end_new == end {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Range {
                                start: start_new,
                                end: end_new,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Index1D {
                    base,
                    idx,
                    is_safe,
                    is_na_safe,
                } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    let idx_new =
                        rewrite_value_tree_for_var(fn_ir, idx, var, replacement, memo, visiting);
                    if base_new == base && idx_new == idx {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Index1D {
                                base: base_new,
                                idx: idx_new,
                                is_safe,
                                is_na_safe,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Index2D { base, r, c } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    let r_new =
                        rewrite_value_tree_for_var(fn_ir, r, var, replacement, memo, visiting);
                    let c_new =
                        rewrite_value_tree_for_var(fn_ir, c, var, replacement, memo, visiting);
                    if base_new == base && r_new == r && c_new == c {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Index2D {
                                base: base_new,
                                r: r_new,
                                c: c_new,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
                ValueKind::Index3D { base, i, j, k } => {
                    let base_new =
                        rewrite_value_tree_for_var(fn_ir, base, var, replacement, memo, visiting);
                    let i_new =
                        rewrite_value_tree_for_var(fn_ir, i, var, replacement, memo, visiting);
                    let j_new =
                        rewrite_value_tree_for_var(fn_ir, j, var, replacement, memo, visiting);
                    let k_new =
                        rewrite_value_tree_for_var(fn_ir, k, var, replacement, memo, visiting);
                    if base_new == base && i_new == i && j_new == j && k_new == k {
                        root
                    } else {
                        fn_ir.add_value(
                            ValueKind::Index3D {
                                base: base_new,
                                i: i_new,
                                j: j_new,
                                k: k_new,
                            },
                            fn_ir.values[root].span,
                            fn_ir.values[root].facts,
                            fn_ir.values[root].origin_var.clone(),
                        )
                    }
                }
            };

            visiting.remove(&root);
            memo.insert(root, mapped);
            mapped
        }

        let mut memo = FxHashMap::default();
        let mut visiting = FxHashSet::default();

        for bid in [start] {
            let old_instrs = std::mem::take(&mut fn_ir.blocks[bid].instrs);
            let mut new_instrs = Vec::with_capacity(old_instrs.len());
            for mut instr in old_instrs {
                match &mut instr {
                    Instr::Assign { src, .. } => {
                        *src = rewrite_value_tree_for_var(
                            fn_ir,
                            *src,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                    }
                    Instr::Eval { val, .. } => {
                        *val = rewrite_value_tree_for_var(
                            fn_ir,
                            *val,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                    }
                    Instr::StoreIndex1D { base, idx, val, .. } => {
                        *base = rewrite_value_tree_for_var(
                            fn_ir,
                            *base,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *idx = rewrite_value_tree_for_var(
                            fn_ir,
                            *idx,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *val = rewrite_value_tree_for_var(
                            fn_ir,
                            *val,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                    }
                    Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        *base = rewrite_value_tree_for_var(
                            fn_ir,
                            *base,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *r = rewrite_value_tree_for_var(
                            fn_ir,
                            *r,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *c = rewrite_value_tree_for_var(
                            fn_ir,
                            *c,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *val = rewrite_value_tree_for_var(
                            fn_ir,
                            *val,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                    }
                    Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        *base = rewrite_value_tree_for_var(
                            fn_ir,
                            *base,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *i = rewrite_value_tree_for_var(
                            fn_ir,
                            *i,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *j = rewrite_value_tree_for_var(
                            fn_ir,
                            *j,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *k = rewrite_value_tree_for_var(
                            fn_ir,
                            *k,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                        *val = rewrite_value_tree_for_var(
                            fn_ir,
                            *val,
                            var,
                            replacement,
                            &mut memo,
                            &mut visiting,
                        );
                    }
                }
                new_instrs.push(instr);
            }
            fn_ir.blocks[bid].instrs = new_instrs;

            let term = std::mem::replace(&mut fn_ir.blocks[bid].term, Terminator::Unreachable);
            fn_ir.blocks[bid].term = match term {
                Terminator::If {
                    cond,
                    then_bb,
                    else_bb,
                } => Terminator::If {
                    cond: rewrite_value_tree_for_var(
                        fn_ir,
                        cond,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    ),
                    then_bb,
                    else_bb,
                },
                Terminator::Return(Some(ret)) => {
                    Terminator::Return(Some(rewrite_value_tree_for_var(
                        fn_ir,
                        ret,
                        var,
                        replacement,
                        &mut memo,
                        &mut visiting,
                    )))
                }
                other => other,
            };
        }
    }

    for assignment in assignments {
        fn_ir.blocks[site.preheader].instrs.push(Instr::Assign {
            dst: assignment.dest_var.clone(),
            src: assignment.out_val,
            span: crate::utils::Span::dummy(),
        });
        let rewrite_dest_returns =
            !has_reachable_assignment_to_var(fn_ir, site.exit_bb, &assignment.dest_var);
        if let Some(shadow_idx) = assignment.shadow_idx
            && !assignment.shadow_vars.is_empty()
        {
            let shadow_val = build_shadow_state_read(fn_ir, assignment.out_val, shadow_idx);
            for shadow_var in &assignment.shadow_vars {
                fn_ir.blocks[site.preheader].instrs.push(Instr::Assign {
                    dst: shadow_var.clone(),
                    src: shadow_val,
                    span: crate::utils::Span::dummy(),
                });
                rewrite_reachable_value_uses_for_var(fn_ir, site.exit_bb, shadow_var, shadow_val);
                if !has_reachable_assignment_to_var(fn_ir, site.exit_bb, shadow_var) {
                    rewrite_returns_for_var(fn_ir, shadow_var, shadow_val);
                }
            }
        }
        rewrite_reachable_value_uses_for_var(
            fn_ir,
            site.exit_bb,
            &assignment.dest_var,
            assignment.out_val,
        );
        if rewrite_dest_returns {
            rewrite_returns_for_var(fn_ir, &assignment.dest_var, assignment.out_val);
        }
    }
    fn_ir.blocks[site.preheader].term = Terminator::Goto(site.exit_bb);
    true
}

/// Commit a vectorized assignment in the loop preheader and rewrite reachable
/// exit-path uses/returns so loop-carried shadow state still observes the
/// vectorized result at the correct index.
pub(super) fn finish_vector_assignment_with_shadow_states(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    dest_var: VarId,
    out_val: ValueId,
    shadow_vars: &[VarId],
    shadow_idx: Option<ValueId>,
) -> bool {
    emit_prepared_vector_assignments(
        fn_ir,
        site,
        vec![PreparedVectorAssignment {
            dest_var,
            out_val,
            shadow_vars: shadow_vars.to_vec(),
            shadow_idx,
        }],
    )
}

pub(super) fn finish_vector_phi_assignment(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    acc_phi: ValueId,
    out_val: ValueId,
) -> bool {
    let Some(acc_var) = fn_ir.values[acc_phi].origin_var.clone() else {
        return false;
    };
    finish_vector_assignment(fn_ir, site, acc_var, out_val)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_recurrence_add_const_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: RecurrenceAddConstApplyPlan,
) -> bool {
    let Some(base_var) = resolve_base_var(fn_ir, plan.base) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let delta_val = if plan.negate_delta {
        fn_ir.add_value(
            ValueKind::Unary {
                op: crate::syntax::ast::UnaryOp::Neg,
                rhs: plan.delta,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    } else {
        plan.delta
    };
    let recur_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_recur_add_const".to_string(),
            args: vec![plan.base, plan.range.start, end, delta_val],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, base_var, recur_val)
}

pub(super) fn apply_recurrence_add_const_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: RecurrenceAddConst3DApplyPlan,
) -> bool {
    let Some(base_var) = resolve_base_var(fn_ir, plan.base) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let delta_val = if plan.negate_delta {
        fn_ir.add_value(
            ValueKind::Unary {
                op: crate::syntax::ast::UnaryOp::Neg,
                rhs: plan.delta,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    } else {
        plan.delta
    };
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_recur_add_const",
        Axis3D::Dim2 => "rr_dim2_recur_add_const",
        Axis3D::Dim3 => "rr_dim3_recur_add_const",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let recur_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![
                plan.base,
                fixed_a,
                fixed_b,
                plan.range.start,
                end,
                delta_val,
            ],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, base_var, recur_val)
}

pub(super) fn apply_shifted_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ShiftedMapApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let src_start = add_int_offset(fn_ir, plan.range.start, plan.offset);
    let src_end = add_int_offset(fn_ir, end, plan.offset);
    let shifted_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_shift_assign".to_string(),
            args: vec![
                plan.dest,
                plan.src,
                plan.range.start,
                end,
                src_start,
                src_end,
            ],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, shifted_val)
}

pub(super) fn apply_shifted_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ShiftedMap3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let src_start = add_int_offset(fn_ir, plan.range.start, plan.offset);
    let src_end = add_int_offset(fn_ir, end, plan.offset);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_shift_assign",
        Axis3D::Dim2 => "rr_dim2_shift_assign",
        Axis3D::Dim3 => "rr_dim3_shift_assign",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let shifted_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![
                plan.dest,
                plan.src,
                fixed_a,
                fixed_b,
                plan.range.start,
                end,
                src_start,
                src_end,
            ],
            names: vec![None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, shifted_val)
}

pub(super) fn emit_call_map_argument_guards(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    dest: ValueId,
    whole_dest: bool,
    mapped_args: &[(ValueId, bool)],
    vector_args: &[ValueId],
) {
    for (arg, is_vec) in mapped_args {
        let check_val = if whole_dest && *is_vec && !same_length_proven(fn_ir, dest, *arg) {
            Some(fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_same_len".to_string(),
                    args: vec![dest, *arg],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ))
        } else if !*is_vec && !is_const_number(fn_ir, *arg) {
            Some(fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_same_or_scalar".to_string(),
                    args: vec![dest, *arg],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ))
        } else {
            None
        };
        if let Some(val) = check_val {
            fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                val,
                span: crate::utils::Span::dummy(),
            });
        }
    }

    for i in 0..vector_args.len() {
        for j in (i + 1)..vector_args.len() {
            let a = vector_args[i];
            let b = vector_args[j];
            if same_length_proven(fn_ir, a, b) {
                continue;
            }
            let check_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_same_len".to_string(),
                    args: vec![a, b],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                val: check_val,
                span: crate::utils::Span::dummy(),
            });
        }
    }
}

pub(super) fn build_int_vector_literal(fn_ir: &mut FnIR, items: &[i64]) -> ValueId {
    let args: Vec<ValueId> = items
        .iter()
        .map(|item| {
            fn_ir.add_value(
                ValueKind::Const(Lit::Int(*item)),
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            )
        })
        .collect();
    fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args,
            names: vec![None; items.len()],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build_call_map_auto_value(
    fn_ir: &mut FnIR,
    dest: ValueId,
    start: ValueId,
    end: ValueId,
    callee: &str,
    helper_cost: u32,
    mapped_args: &[(ValueId, bool)],
    whole_dest: bool,
) -> ValueId {
    let callee_val = fn_ir.add_value(
        ValueKind::Const(Lit::Str(callee.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let helper_cost_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(helper_cost as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let vector_slots: Vec<i64> = mapped_args
        .iter()
        .enumerate()
        .filter_map(|(index, (_, is_vec))| is_vec.then_some((index + 1) as i64))
        .collect();
    let vector_slots_val = build_int_vector_literal(fn_ir, &vector_slots);
    let mut args = if whole_dest {
        vec![dest, callee_val, helper_cost_val, vector_slots_val]
    } else {
        vec![
            dest,
            start,
            end,
            callee_val,
            helper_cost_val,
            vector_slots_val,
        ]
    };
    args.extend(mapped_args.iter().map(|(arg, _)| *arg));
    let callee_name = if whole_dest {
        "rr_call_map_whole_auto"
    } else {
        "rr_call_map_slice_auto"
    };
    fn_ir.add_value(
        ValueKind::Call {
            callee: callee_name.to_string(),
            args,
            names: vec![None; mapped_args.len() + if whole_dest { 4 } else { 6 }],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_call_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    callee: String,
    args: Vec<CallMapArg>,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
    shadow_vars: Vec<VarId>,
) -> bool {
    let whole_dest = whole_dest && lp.limit_adjust == 0;
    let trace_enabled = vectorize_trace_enabled();
    let lowering_mode = choose_call_map_lowering(fn_ir, &callee, &args, whole_dest, &shadow_vars);
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut mapped_args = Vec::with_capacity(args.len());
    let mut vector_args = Vec::new();
    for (arg_i, arg) in args.into_iter().enumerate() {
        let out = if arg.vectorized {
            match materialize_vector_expr(
                fn_ir,
                arg.value,
                iv_phi,
                idx_vec,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            ) {
                Some(v) => v,
                None => return false,
            }
        } else {
            resolve_materialized_value(fn_ir, arg.value)
        };
        let out = if arg.vectorized {
            maybe_hoist_callmap_arg_expr(fn_ir, site.preheader, out, arg_i)
        } else {
            out
        };
        if arg.vectorized {
            vector_args.push(out);
        }
        mapped_args.push((out, arg.vectorized));
    }

    emit_call_map_argument_guards(
        fn_ir,
        site.preheader,
        dest,
        whole_dest,
        &mapped_args,
        &vector_args,
    );

    let out_val = match lowering_mode {
        CallMapLoweringMode::RuntimeAuto { helper_cost } => {
            if trace_enabled {
                eprintln!(
                    "   [vec-profit] {} call_map runtime-auto callee={} helper_cost={} whole_dest={}",
                    fn_ir.name, callee, helper_cost, whole_dest
                );
            }
            build_call_map_auto_value(
                fn_ir,
                dest,
                start,
                end,
                &callee,
                helper_cost,
                &mapped_args,
                whole_dest,
            )
        }
        CallMapLoweringMode::DirectVector => {
            let mapped_args_vals: Vec<ValueId> = mapped_args.iter().map(|(arg, _)| *arg).collect();
            let mapped_val = fn_ir.add_value(
                if let Some(op) = intrinsic_for_call(&callee, mapped_args_vals.len()) {
                    ValueKind::Intrinsic {
                        op,
                        args: mapped_args_vals,
                    }
                } else {
                    ValueKind::Call {
                        callee,
                        args: mapped_args_vals,
                        names: vec![None; mapped_args.len()],
                    }
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            if whole_dest {
                mapped_val
            } else {
                let mapped_val = prepare_partial_slice_value(fn_ir, dest, mapped_val, start, end);
                fn_ir.add_value(
                    ValueKind::Call {
                        callee: "rr_assign_slice".to_string(),
                        args: vec![dest, start, end, mapped_val],
                        names: vec![None, None, None, None],
                    },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                )
            }
        }
    };
    finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        out_val,
        &shadow_vars,
        Some(end),
    )
}

pub(super) fn apply_call_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: CallMap3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let Some(iv_phi) = lp.iv.as_ref().map(|iv| iv.phi_val) else {
        return false;
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_call_assign",
        Axis3D::Dim2 => "rr_dim2_call_assign",
        Axis3D::Dim3 => "rr_dim3_call_assign",
    };
    let callee_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(plan.callee)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let mut mapped_args = vec![
        plan.dest,
        callee_lit,
        fixed_a,
        fixed_b,
        plan.range.start,
        end,
    ];
    for (arg_i, arg) in plan.args.into_iter().enumerate() {
        let out = if arg.vectorized {
            match materialize_vector_expr(
                fn_ir,
                arg.value,
                iv_phi,
                idx_vec,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            ) {
                Some(v) => v,
                None => return false,
            }
        } else {
            resolve_materialized_value(fn_ir, arg.value)
        };
        let out = if arg.vectorized {
            maybe_hoist_callmap_arg_expr(fn_ir, site.preheader, out, arg_i)
        } else {
            out
        };
        mapped_args.push(out);
    }
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: mapped_args.clone(),
            names: vec![None; mapped_args.len()],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn apply_call_map_3d_general_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: CallMap3DGeneralApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let idx_seed = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut mapped_args = Vec::with_capacity(plan.args.len());
    for arg in plan.args {
        let out = match materialize_vector_or_scalar_expr(
            fn_ir,
            lp,
            arg.value,
            plan.iv_phi,
            idx_seed,
            &mut memo,
            &mut interner,
        ) {
            Some(v) => v,
            None => return false,
        };
        mapped_args.push(out);
    }
    let arg_len = mapped_args.len();
    let call_val = fn_ir.add_value(
        if let Some(op) = intrinsic_for_call(&plan.callee, arg_len) {
            ValueKind::Intrinsic {
                op,
                args: mapped_args.clone(),
            }
        } else {
            ValueKind::Call {
                callee: plan.callee,
                args: mapped_args,
                names: vec![None; arg_len],
            }
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_assign_values",
        Axis3D::Dim2 => "rr_dim2_assign_values",
        Axis3D::Dim3 => "rr_dim3_assign_values",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![plan.dest, call_val, fixed_a, fixed_b, plan.range.start, end],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn apply_expr_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ExprMap3DApplyPlan,
) -> bool {
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        plan.expr,
        plan.iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => v,
        None => return false,
    };
    let end = adjusted_loop_limit(fn_ir, plan.range.end, lp.limit_adjust);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_assign_values",
        Axis3D::Dim2 => "rr_dim2_assign_values",
        Axis3D::Dim3 => "rr_dim3_assign_values",
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![plan.dest, expr_vec, fixed_a, fixed_b, plan.range.start, end],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn same_load_leaf_var_in_phi_tree(
    fn_ir: &FnIR,
    root: ValueId,
    seen: &mut FxHashSet<ValueId>,
) -> Option<VarId> {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return None;
    }
    let out = match &fn_ir.values[root].kind {
        ValueKind::Load { var } => Some(var.clone()),
        ValueKind::Phi { args } if !args.is_empty() => {
            let mut found: Option<VarId> = None;
            for (arg, _) in args {
                let leaf_var = same_load_leaf_var_in_phi_tree(fn_ir, *arg, seen)?;
                match &found {
                    None => found = Some(leaf_var),
                    Some(prev) if prev == &leaf_var => {}
                    Some(_) => return None,
                }
            }
            found
        }
        _ => None,
    };
    seen.remove(&root);
    out
}

pub(super) fn recover_cube_slice_snapshot_scalar(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    value: ValueId,
    iv_phi: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    let root = canonical_value(fn_ir, value);
    let target_bb = fn_ir.values[root]
        .phi_block
        .or_else(|| value_use_block_in_loop(fn_ir, lp, root));
    let snapshot_var = fn_ir.values[root]
        .origin_var
        .clone()
        .or_else(|| induction_origin_var(fn_ir, root))?;
    let candidate = target_bb
        .and_then(|bb| unique_assign_source_reaching_block_in_loop(fn_ir, lp, &snapshot_var, bb))
        .or_else(|| loop_entry_seed_source_in_loop(fn_ir, lp, &snapshot_var))
        .or_else(|| unique_assign_source_in_loop(fn_ir, lp, &snapshot_var))
        .map(|src| canonical_value(fn_ir, src))
        .filter(|src| {
            *src != root && !value_depends_on(fn_ir, *src, root, &mut FxHashSet::default())
        })
        .unwrap_or(root);
    if candidate != root
        && let Some(v) =
            materialize_loop_invariant_scalar_expr(fn_ir, candidate, iv_phi, lp, memo, interner)
    {
        return Some(resolve_materialized_value(fn_ir, v));
    }
    if let Some(var) = fn_ir.values[candidate].origin_var.clone()
        && !has_non_passthrough_assignment_in_loop(fn_ir, lp, &var)
    {
        let load = intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Load { var },
            fn_ir.values[candidate].span,
            fn_ir.values[candidate].facts,
        );
        return Some(resolve_materialized_value(fn_ir, load));
    }
    let leaf_var = same_load_leaf_var_in_phi_tree(fn_ir, candidate, &mut FxHashSet::default())?;
    if has_non_passthrough_assignment_in_loop(fn_ir, lp, &leaf_var) {
        return None;
    }
    let load = intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Load { var: leaf_var },
        fn_ir.values[root].span,
        fn_ir.values[root].facts,
    );
    Some(resolve_materialized_value(fn_ir, load))
}

pub(super) fn cube_slice_expr_has_complex_loop_local_axes(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    expr: ValueId,
    iv_phi: ValueId,
) -> bool {
    let expr = canonical_value(fn_ir, expr);
    let ValueKind::Call { callee, args, .. } = &fn_ir.values[expr].kind else {
        return false;
    };
    if callee != "rr_idx_cube_vec_i" || args.len() < 4 {
        return false;
    }
    for arg in [args[1], args[2]] {
        let arg = canonical_value(fn_ir, arg);
        if is_iv_equivalent(fn_ir, arg, iv_phi) {
            continue;
        }
        if let ValueKind::Load { var } = &fn_ir.values[arg].kind
            && has_assignment_in_loop(fn_ir, lp, var)
        {
            return true;
        }
        if let Some(var) = fn_ir.values[arg].origin_var.as_deref()
            && has_assignment_in_loop(fn_ir, lp, var)
            && !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, &FxHashSet::default())
        {
            return true;
        }
    }
    false
}

#[allow(clippy::too_many_arguments)]
pub(super) fn apply_cube_slice_expr_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
    face: ValueId,
    row: ValueId,
    size: ValueId,
    ctx: Option<ValueId>,
    start: ValueId,
    end: ValueId,
    shadow_vars: Vec<VarId>,
) -> bool {
    let trace_enabled = vectorize_trace_enabled();
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr] {} fail: no loop index vector",
                    fn_ir.name
                );
            }
            return false;
        }
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();

    let materialize_scalar =
        |value: ValueId,
         label: &str,
         fn_ir: &mut FnIR,
         memo: &mut FxHashMap<ValueId, ValueId>,
         interner: &mut FxHashMap<MaterializedExprKey, ValueId>| {
            match materialize_loop_invariant_scalar_expr(fn_ir, value, iv_phi, lp, memo, interner) {
                Some(v) => Some(resolve_materialized_value(fn_ir, v)),
                None => {
                    if let Some(v) =
                        recover_cube_slice_snapshot_scalar(fn_ir, lp, value, iv_phi, memo, interner)
                    {
                        return Some(v);
                    }
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: invariant scalar {} materialization ({:?})",
                            fn_ir.name, label, fn_ir.values[value].kind
                        );
                    }
                    None
                }
            }
        };

    let Some(face) = materialize_scalar(face, "face", fn_ir, &mut memo, &mut interner) else {
        return false;
    };
    let Some(row) = materialize_scalar(row, "row", fn_ir, &mut memo, &mut interner) else {
        return false;
    };
    let Some(size) = materialize_scalar(size, "size", fn_ir, &mut memo, &mut interner) else {
        return false;
    };
    let ctx = match ctx {
        Some(ctx_val) => {
            match materialize_scalar(ctx_val, "ctx", fn_ir, &mut memo, &mut interner) {
                Some(v) => Some(v),
                None => return false,
            }
        }
        None => None,
    };

    if cube_slice_expr_has_complex_loop_local_axes(fn_ir, lp, expr, iv_phi) {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: cube-slice rhs has complex loop-local axes ({:?})",
                fn_ir.name,
                fn_ir.values[canonical_value(fn_ir, expr)].kind
            );
        }
        return false;
    }

    let expr_vec = if expr_has_iv_dependency(fn_ir, expr, iv_phi) {
        match materialize_vector_expr(
            fn_ir,
            expr,
            iv_phi,
            idx_vec,
            lp,
            &mut memo,
            &mut interner,
            true,
            false,
        ) {
            Some(v) => v,
            None => {
                if trace_enabled {
                    eprintln!(
                        "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                        fn_ir.name, fn_ir.values[expr].kind
                    );
                }
                return false;
            }
        }
    } else {
        match materialize_loop_invariant_scalar_expr(
            fn_ir,
            expr,
            iv_phi,
            lp,
            &mut memo,
            &mut interner,
        ) {
            Some(v) => v,
            None => {
                if let Some(v) = materialize_vector_expr(
                    fn_ir,
                    expr,
                    iv_phi,
                    idx_vec,
                    lp,
                    &mut memo,
                    &mut interner,
                    true,
                    false,
                ) {
                    v
                } else {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: invariant scalar expr materialization ({:?})",
                            fn_ir.name, fn_ir.values[expr].kind
                        );
                    }
                    return false;
                }
            }
        }
    };

    let expr_vec = broadcast_scalar_expr_to_slice_len(fn_ir, expr_vec, start, end);

    let has_ctx = ctx.is_some();
    let mut start_args = vec![face, row, start, size];
    let mut end_args = vec![face, row, end, size];
    if let Some(ctx) = ctx {
        start_args.push(ctx);
        end_args.push(ctx);
    }
    let names = vec![None; if has_ctx { 5 } else { 4 }];
    let slice_start = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_idx_cube_vec_i".to_string(),
            args: start_args,
            names: names.clone(),
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let slice_end = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_idx_cube_vec_i".to_string(),
            args: end_args,
            names,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_assign_slice".to_string(),
            args: vec![dest, slice_start, slice_end, expr_vec],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        out_val,
        &shadow_vars,
        Some(slice_end),
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn try_apply_expr_call_map_auto(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
    shadow_vars: &[VarId],
) -> Option<bool> {
    let whole_dest = whole_dest && lp.limit_adjust == 0;
    let root = canonical_value(fn_ir, expr);
    let (callee, args) = match &fn_ir.values[root].kind {
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            if names.iter().any(Option::is_some) {
                return None;
            }
            (callee.clone(), args.clone())
        }
        _ => return None,
    };

    let call_args: Vec<CallMapArg> = args
        .iter()
        .map(|arg| CallMapArg {
            value: *arg,
            vectorized: expr_has_iv_dependency(fn_ir, *arg, iv_phi),
        })
        .collect();
    let CallMapLoweringMode::RuntimeAuto { helper_cost } =
        choose_call_map_lowering(fn_ir, &callee, &call_args, whole_dest, shadow_vars)
    else {
        return None;
    };

    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        return Some(false);
    };
    let idx_vec = build_loop_index_vector(fn_ir, lp)?;
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut mapped_args = Vec::with_capacity(call_args.len());
    let mut vector_args = Vec::new();

    for (arg_i, arg) in call_args.iter().enumerate() {
        let out = if arg.vectorized {
            materialize_vector_expr(
                fn_ir,
                arg.value,
                iv_phi,
                idx_vec,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            )?
        } else {
            materialize_loop_invariant_scalar_expr(
                fn_ir,
                arg.value,
                iv_phi,
                lp,
                &mut memo,
                &mut interner,
            )
            .unwrap_or_else(|| resolve_materialized_value(fn_ir, arg.value))
        };
        let out = if arg.vectorized {
            maybe_hoist_callmap_arg_expr(fn_ir, site.preheader, out, arg_i)
        } else {
            out
        };
        if arg.vectorized {
            vector_args.push(out);
        }
        mapped_args.push((out, arg.vectorized));
    }

    emit_call_map_argument_guards(
        fn_ir,
        site.preheader,
        dest,
        whole_dest,
        &mapped_args,
        &vector_args,
    );
    if vectorize_trace_enabled() {
        eprintln!(
            "   [vec-profit] {} expr_map runtime-auto callee={} helper_cost={} whole_dest={}",
            fn_ir.name, callee, helper_cost, whole_dest
        );
    }
    let out_val = build_call_map_auto_value(
        fn_ir,
        dest,
        start,
        end,
        &callee,
        helper_cost,
        &mapped_args,
        whole_dest,
    );
    Some(finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        out_val,
        shadow_vars,
        Some(end),
    ))
}

#[allow(clippy::too_many_arguments)]
/// Lower a canonical expr-map vectorization plan into preheader materialization
/// plus a single assignment that preserves partial-slice semantics when the
/// original scalar loop did not cover the full destination.
pub(super) fn apply_expr_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
    shadow_vars: Vec<VarId>,
) -> bool {
    let whole_dest = whole_dest && lp.limit_adjust == 0;
    if let Some(applied) = try_apply_expr_call_map_auto(
        fn_ir,
        lp,
        site,
        dest,
        expr,
        iv_phi,
        start,
        end,
        whole_dest,
        &shadow_vars,
    ) {
        return applied;
    }

    let trace_enabled = vectorize_trace_enabled();
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        return false;
    };
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr] {} fail: no loop index vector",
                    fn_ir.name
                );
            }
            return false;
        }
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        expr,
        iv_phi,
        idx_vec,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                    fn_ir.name, fn_ir.values[expr].kind
                );
            }
            return false;
        }
    };
    let out_val = build_expr_map_output_value(
        fn_ir,
        site.preheader,
        dest,
        expr_vec,
        start,
        end,
        whole_dest,
    );
    finish_vector_assignment_with_shadow_states(
        fn_ir,
        site,
        dest_var,
        out_val,
        &shadow_vars,
        Some(end),
    )
}

pub(super) fn build_expr_map_output_value(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    dest: ValueId,
    expr_vec: ValueId,
    start: ValueId,
    end: ValueId,
    whole_dest: bool,
) -> ValueId {
    let expr_is_scalar = is_scalar_broadcast_value(fn_ir, expr_vec);
    let expr_vec = if expr_is_scalar {
        let broadcast_end = if whole_dest && is_const_one(fn_ir, start) {
            vector_length_key(fn_ir, dest).unwrap_or(end)
        } else {
            end
        };
        broadcast_scalar_expr_to_slice_len(fn_ir, expr_vec, start, broadcast_end)
    } else if whole_dest {
        expr_vec
    } else {
        prepare_partial_slice_value(fn_ir, dest, expr_vec, start, end)
    };

    if whole_dest {
        if !expr_is_scalar && !same_length_proven(fn_ir, dest, expr_vec) {
            emit_same_len_guard(fn_ir, preheader, dest, expr_vec);
        }
        expr_vec
    } else {
        build_slice_assignment_value(fn_ir, dest, start, end, expr_vec)
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn stage_multi_expr_map_entry(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    entry_index: usize,
    entry: &ExprMapEntry,
    iv_phi: ValueId,
    idx_vec: ValueId,
    start: ValueId,
    end: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    trace_enabled: bool,
) -> Option<PreparedVectorAssignment> {
    let dest_var = resolve_base_var(fn_ir, entry.dest).or_else(|| {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        None
    })?;
    let expr_vec = materialize_vector_expr(
        fn_ir, entry.expr, iv_phi, idx_vec, lp, memo, interner, true, false,
    )
    .or_else(|| {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                fn_ir.name, fn_ir.values[entry.expr].kind
            );
        }
        None
    })?;
    let expr_vec = hoist_vector_expr_temp(
        fn_ir,
        site.preheader,
        expr_vec,
        &format!("exprmap{}", entry_index),
    );
    let out_val = build_expr_map_output_value(
        fn_ir,
        site.preheader,
        entry.dest,
        expr_vec,
        start,
        end,
        entry.whole_dest && lp.limit_adjust == 0,
    );
    let shadow_idx = if entry.whole_dest && lp.limit_adjust == 0 {
        Some(fn_ir.add_value(
            ValueKind::Len { base: out_val },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ))
    } else {
        Some(end)
    };

    Some(PreparedVectorAssignment {
        dest_var,
        out_val,
        shadow_vars: entry.shadow_vars.clone(),
        shadow_idx,
    })
}

pub(super) fn apply_multi_expr_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    entries: Vec<ExprMapEntry>,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
) -> bool {
    let trace_enabled = vectorize_trace_enabled();
    if trace_enabled {
        let dests: Vec<String> = entries
            .iter()
            .map(|entry| {
                resolve_base_var(fn_ir, entry.dest).unwrap_or_else(|| format!("<v{}>", entry.dest))
            })
            .collect();
        eprintln!(
            "   [vec-apply-expr] {} multi_expr_map entries={:?}",
            fn_ir.name, dests
        );
    }
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr] {} fail: no loop index vector",
                    fn_ir.name
                );
            }
            return false;
        }
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut staged = Vec::with_capacity(entries.len());

    for (entry_index, entry) in entries.iter().enumerate() {
        let Some(assignment) = stage_multi_expr_map_entry(
            fn_ir,
            lp,
            site,
            entry_index,
            entry,
            iv_phi,
            idx_vec,
            start,
            end,
            &mut memo,
            &mut interner,
            trace_enabled,
        ) else {
            return false;
        };
        staged.push(assignment);
    }
    emit_prepared_vector_assignments(fn_ir, site, staged)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn stage_multi_expr_map_3d_entry(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    entry_index: usize,
    entry: &ExprMapEntry3D,
    iv_phi: ValueId,
    idx_vec: ValueId,
    start: ValueId,
    end: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    trace_enabled: bool,
) -> Option<PreparedVectorAssignment> {
    let dest_var = resolve_base_var(fn_ir, entry.dest).or_else(|| {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr3d] {} fail: destination has no base var",
                fn_ir.name
            );
        }
        None
    })?;
    let expr_vec = materialize_vector_expr(
        fn_ir, entry.expr, iv_phi, idx_vec, lp, memo, interner, true, false,
    )
    .or_else(|| {
        if trace_enabled {
            eprintln!(
                "   [vec-apply-expr3d] {} fail: materialize_vector_expr({:?})",
                fn_ir.name, fn_ir.values[entry.expr].kind
            );
        }
        None
    })?;
    let expr_vec = hoist_vector_expr_temp(
        fn_ir,
        site.preheader,
        expr_vec,
        &format!("exprmap3d{}", entry_index),
    );
    let helper = match entry.axis {
        Axis3D::Dim1 => "rr_dim1_assign_values",
        Axis3D::Dim2 => "rr_dim2_assign_values",
        Axis3D::Dim3 => "rr_dim3_assign_values",
    };
    let fixed_a = resolve_materialized_value(fn_ir, entry.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, entry.fixed_b);
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![entry.dest, expr_vec, fixed_a, fixed_b, start, end],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );

    Some(PreparedVectorAssignment {
        dest_var,
        out_val,
        shadow_vars: entry.shadow_vars.clone(),
        shadow_idx: Some(end),
    })
}

pub(super) fn apply_multi_expr_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    entries: Vec<ExprMapEntry3D>,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
) -> bool {
    let trace_enabled = vectorize_trace_enabled();
    let end = adjusted_loop_limit(fn_ir, end, lp.limit_adjust);
    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => {
            if trace_enabled {
                eprintln!(
                    "   [vec-apply-expr3d] {} fail: no loop index vector",
                    fn_ir.name
                );
            }
            return false;
        }
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let mut staged = Vec::with_capacity(entries.len());

    for (entry_index, entry) in entries.iter().enumerate() {
        let Some(assignment) = stage_multi_expr_map_3d_entry(
            fn_ir,
            lp,
            site,
            entry_index,
            entry,
            iv_phi,
            idx_vec,
            start,
            end,
            &mut memo,
            &mut interner,
            trace_enabled,
        ) else {
            return false;
        };
        staged.push(assignment);
    }
    emit_prepared_vector_assignments(fn_ir, site, staged)
}

pub(super) fn apply_scatter_expr_map_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    dest: ValueId,
    idx: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
) -> bool {
    let idx_seed = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
        return false;
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let idx_vec = match materialize_vector_expr(
        fn_ir,
        idx,
        iv_phi,
        idx_seed,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter_idx"),
        None => return false,
    };
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        expr,
        iv_phi,
        idx_seed,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter_val"),
        None => return false,
    };
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_assign_index_vec".to_string(),
            args: vec![dest, idx_vec, expr_vec],
            names: vec![None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn apply_scatter_expr_map_3d_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ScatterExprMap3DApplyPlan,
) -> bool {
    let idx_seed = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let idx_vec = match materialize_vector_expr(
        fn_ir,
        plan.idx,
        plan.iv_phi,
        idx_seed,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter3d_idx"),
        None => return false,
    };
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        plan.expr,
        plan.iv_phi,
        idx_seed,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter3d_val"),
        None => return false,
    };
    let fixed_a = resolve_materialized_value(fn_ir, plan.fixed_a);
    let fixed_b = resolve_materialized_value(fn_ir, plan.fixed_b);
    let helper = match plan.axis {
        Axis3D::Dim1 => "rr_dim1_assign_index_values",
        Axis3D::Dim2 => "rr_dim2_assign_index_values",
        Axis3D::Dim3 => "rr_dim3_assign_index_values",
    };
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: helper.to_string(),
            args: vec![plan.dest, expr_vec, fixed_a, fixed_b, idx_vec],
            names: vec![None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn materialize_3d_index_operand_for_scatter(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    operand: VectorAccessOperand3D,
    iv_phi: ValueId,
    idx_seed: ValueId,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    match operand {
        VectorAccessOperand3D::Scalar(value) => Some(
            materialize_loop_invariant_scalar_expr(fn_ir, value, iv_phi, lp, memo, interner)
                .unwrap_or_else(|| resolve_materialized_value(fn_ir, value)),
        ),
        VectorAccessOperand3D::Vector(value) => {
            let mut materialized = if is_iv_equivalent(fn_ir, value, iv_phi) {
                idx_seed
            } else {
                materialize_vector_expr(
                    fn_ir, value, iv_phi, idx_seed, lp, memo, interner, true, false,
                )?
            };
            if !is_int_index_vector_value(fn_ir, materialized) {
                materialized = fn_ir.add_value(
                    ValueKind::Call {
                        callee: "rr_index_vec_floor".to_string(),
                        args: vec![materialized],
                        names: vec![None],
                    },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                );
            }
            Some(hoist_vector_expr_temp(
                fn_ir,
                site.preheader,
                materialized,
                "scatter3d_axis",
            ))
        }
    }
}

pub(super) fn apply_scatter_expr_map_3d_general_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: ScatterExprMap3DGeneralApplyPlan,
) -> bool {
    let idx_seed = match build_loop_index_vector(fn_ir, lp) {
        Some(v) => v,
        None => return false,
    };
    let Some(dest_var) = resolve_base_var(fn_ir, plan.dest) else {
        return false;
    };
    let mut memo = FxHashMap::default();
    let mut interner = FxHashMap::default();
    let i_vec = match materialize_3d_index_operand_for_scatter(
        fn_ir,
        lp,
        site,
        plan.i,
        plan.iv_phi,
        idx_seed,
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let j_vec = match materialize_3d_index_operand_for_scatter(
        fn_ir,
        lp,
        site,
        plan.j,
        plan.iv_phi,
        idx_seed,
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let k_vec = match materialize_3d_index_operand_for_scatter(
        fn_ir,
        lp,
        site,
        plan.k,
        plan.iv_phi,
        idx_seed,
        &mut memo,
        &mut interner,
    ) {
        Some(v) => v,
        None => return false,
    };
    let expr_vec = match materialize_vector_expr(
        fn_ir,
        plan.expr,
        plan.iv_phi,
        idx_seed,
        lp,
        &mut memo,
        &mut interner,
        true,
        false,
    ) {
        Some(v) => hoist_vector_expr_temp(fn_ir, site.preheader, v, "scatter3d_val"),
        None => return false,
    };
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_array3_assign_gather_values".to_string(),
            args: vec![plan.dest, expr_vec, i_vec, j_vec, k_vec],
            names: vec![None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    finish_vector_assignment(fn_ir, site, dest_var, out_val)
}

pub(super) fn broadcast_scalar_expr_to_slice_len(
    fn_ir: &mut FnIR,
    expr_vec: ValueId,
    start: ValueId,
    end: ValueId,
) -> ValueId {
    if !is_scalar_broadcast_value(fn_ir, expr_vec) {
        return expr_vec;
    }

    let slice_len = if is_const_one(fn_ir, start) {
        end
    } else {
        let span = crate::utils::Span::dummy();
        let facts = crate::mir::def::Facts::empty();
        let span_delta = fn_ir.add_value(
            ValueKind::Binary {
                op: crate::syntax::ast::BinOp::Sub,
                lhs: end,
                rhs: start,
            },
            span,
            facts,
            None,
        );
        let one_val = fn_ir.add_value(ValueKind::Const(Lit::Float(1.0)), span, facts, None);
        fn_ir.add_value(
            ValueKind::Binary {
                op: crate::syntax::ast::BinOp::Add,
                lhs: span_delta,
                rhs: one_val,
            },
            span,
            facts,
            None,
        )
    };

    fn_ir.add_value(
        ValueKind::Call {
            callee: "rep.int".to_string(),
            args: vec![expr_vec, slice_len],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

pub(super) fn narrow_vector_expr_to_slice_range(
    fn_ir: &mut FnIR,
    expr_vec: ValueId,
    start: ValueId,
    end: ValueId,
) -> ValueId {
    let idx_range = fn_ir.add_value(
        ValueKind::Range { start, end },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_index1_read_vec".to_string(),
            args: vec![expr_vec, idx_range],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

pub(crate) fn prepare_partial_slice_value(
    fn_ir: &mut FnIR,
    dest: ValueId,
    expr_vec: ValueId,
    start: ValueId,
    end: ValueId,
) -> ValueId {
    if is_scalar_broadcast_value(fn_ir, expr_vec) {
        return broadcast_scalar_expr_to_slice_len(fn_ir, expr_vec, start, end);
    }
    if same_length_proven(fn_ir, dest, expr_vec) {
        return narrow_vector_expr_to_slice_range(fn_ir, expr_vec, start, end);
    }
    expr_vec
}

#[allow(clippy::too_many_arguments)]
#[allow(clippy::too_many_arguments)]
pub(super) fn apply_expr_vector_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: VectorPlan,
) -> bool {
    match plan {
        VectorPlan::CubeSliceExprMap {
            dest,
            expr,
            iv_phi,
            face,
            row,
            size,
            ctx,
            start,
            end,
            shadow_vars,
        } => apply_cube_slice_expr_map_plan(
            fn_ir,
            lp,
            site,
            dest,
            expr,
            iv_phi,
            face,
            row,
            size,
            ctx,
            start,
            end,
            shadow_vars,
        ),
        VectorPlan::ExprMap {
            dest,
            expr,
            iv_phi,
            start,
            end,
            whole_dest,
            shadow_vars,
        } => apply_expr_map_plan(
            fn_ir,
            lp,
            site,
            dest,
            expr,
            iv_phi,
            start,
            end,
            whole_dest,
            shadow_vars,
        ),
        VectorPlan::MultiExprMap {
            entries,
            iv_phi,
            start,
            end,
        } => apply_multi_expr_map_plan(fn_ir, lp, site, entries, iv_phi, start, end),
        VectorPlan::MultiExprMap3D {
            entries,
            iv_phi,
            start,
            end,
        } => apply_multi_expr_map_3d_plan(fn_ir, lp, site, entries, iv_phi, start, end),
        VectorPlan::ScatterExprMap {
            dest,
            idx,
            expr,
            iv_phi,
        } => apply_scatter_expr_map_plan(fn_ir, lp, site, dest, idx, expr, iv_phi),
        VectorPlan::ScatterExprMap3D {
            dest,
            axis,
            fixed_a,
            fixed_b,
            idx,
            expr,
            iv_phi,
        } => apply_scatter_expr_map_3d_plan(
            fn_ir,
            lp,
            site,
            ScatterExprMap3DApplyPlan {
                dest,
                axis,
                fixed_a,
                fixed_b,
                idx,
                expr,
                iv_phi,
            },
        ),
        VectorPlan::ScatterExprMap3DGeneral {
            dest,
            i,
            j,
            k,
            expr,
            iv_phi,
        } => apply_scatter_expr_map_3d_general_plan(
            fn_ir,
            lp,
            site,
            ScatterExprMap3DGeneralApplyPlan {
                dest,
                i,
                j,
                k,
                expr,
                iv_phi,
            },
        ),
        _ => false,
    }
}

pub(super) fn apply_structured_vector_plan(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    site: VectorApplySite,
    plan: VectorPlan,
) -> bool {
    match plan {
        VectorPlan::Map2DRow {
            dest,
            row,
            start,
            end,
            lhs_src,
            rhs_src,
            op,
        } => apply_map_2d_row_plan(
            fn_ir,
            lp,
            site,
            Map2DApplyPlan {
                dest,
                axis: row,
                range: VectorLoopRange { start, end },
                lhs_src,
                rhs_src,
                op,
            },
        ),
        VectorPlan::Map2DCol {
            dest,
            col,
            start,
            end,
            lhs_src,
            rhs_src,
            op,
        } => apply_map_2d_col_plan(
            fn_ir,
            lp,
            site,
            Map2DApplyPlan {
                dest,
                axis: col,
                range: VectorLoopRange { start, end },
                lhs_src,
                rhs_src,
                op,
            },
        ),
        VectorPlan::Map3D {
            dest,
            axis,
            fixed_a,
            fixed_b,
            start,
            end,
            lhs_src,
            rhs_src,
            op,
        } => apply_map_3d_plan(
            fn_ir,
            lp,
            site,
            Map3DApplyPlan {
                dest,
                axis,
                fixed_a,
                fixed_b,
                range: VectorLoopRange { start, end },
                lhs_src,
                rhs_src,
                op,
            },
        ),
        _ => false,
    }
}

pub(super) fn apply_vectorization(fn_ir: &mut FnIR, lp: &LoopInfo, plan: VectorPlan) -> bool {
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        return false;
    };

    match plan {
        plan @ VectorPlan::Reduce { .. }
        | plan @ VectorPlan::ReduceCond { .. }
        | plan @ VectorPlan::MultiReduceCond { .. }
        | plan @ VectorPlan::Reduce2DRowSum { .. }
        | plan @ VectorPlan::Reduce2DColSum { .. }
        | plan @ VectorPlan::Reduce3D { .. } => apply_reduce_vector_plan(fn_ir, lp, site, plan),
        plan @ VectorPlan::Map { .. }
        | plan @ VectorPlan::CondMap { .. }
        | plan @ VectorPlan::CondMap3D { .. }
        | plan @ VectorPlan::CondMap3DGeneral { .. }
        | plan @ VectorPlan::RecurrenceAddConst { .. }
        | plan @ VectorPlan::RecurrenceAddConst3D { .. }
        | plan @ VectorPlan::ShiftedMap { .. }
        | plan @ VectorPlan::ShiftedMap3D { .. }
        | plan @ VectorPlan::CallMap { .. }
        | plan @ VectorPlan::CallMap3D { .. }
        | plan @ VectorPlan::CallMap3DGeneral { .. }
        | plan @ VectorPlan::ExprMap3D { .. } => apply_linear_vector_plan(fn_ir, lp, site, plan),
        plan @ VectorPlan::CubeSliceExprMap { .. }
        | plan @ VectorPlan::ExprMap { .. }
        | plan @ VectorPlan::MultiExprMap3D { .. }
        | plan @ VectorPlan::MultiExprMap { .. }
        | plan @ VectorPlan::ScatterExprMap { .. }
        | plan @ VectorPlan::ScatterExprMap3D { .. }
        | plan @ VectorPlan::ScatterExprMap3DGeneral { .. } => {
            apply_expr_vector_plan(fn_ir, lp, site, plan)
        }
        plan @ VectorPlan::Map2DRow { .. }
        | plan @ VectorPlan::Map2DCol { .. }
        | plan @ VectorPlan::Map3D { .. } => apply_structured_vector_plan(fn_ir, lp, site, plan),
    }
}

pub(crate) fn try_apply_vectorization_transactionally(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    plan: VectorPlan,
) -> bool {
    let mut cloned = fn_ir.clone();
    if !apply_vectorization(&mut cloned, lp, plan) {
        return false;
    }
    if crate::mir::verify::verify_ir(&cloned).is_err() {
        return false;
    }
    *fn_ir = cloned;
    true
}

pub(super) fn rewrite_sum_add_const(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    vec_expr: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    let root = canonical_value(fn_ir, vec_expr);
    let ValueKind::Binary {
        op: BinOp::Add,
        lhs,
        rhs,
    } = fn_ir.values[root].kind
    else {
        return None;
    };

    let (base, cst) = if let Some(base) = as_safe_loop_index(fn_ir, lhs, iv_phi) {
        if is_invariant_reduce_scalar(fn_ir, rhs, iv_phi, base) {
            (base, rhs)
        } else {
            return None;
        }
    } else if let Some(base) = as_safe_loop_index(fn_ir, rhs, iv_phi) {
        if is_invariant_reduce_scalar(fn_ir, lhs, iv_phi, base) {
            (base, lhs)
        } else {
            return None;
        }
    } else {
        return None;
    };

    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) != Some(canonical_value(fn_ir, base)) {
        return None;
    }

    let sum_base = fn_ir.add_value(
        ValueKind::Call {
            callee: "sum".to_string(),
            args: vec![base],
            names: vec![None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let len_base = fn_ir.add_value(
        ValueKind::Len { base },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let cst = resolve_materialized_value(fn_ir, cst);
    let c_times_n = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Mul,
            lhs: cst,
            rhs: len_base,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some(fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: sum_base,
            rhs: c_times_n,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::opt::loop_analysis::LoopInfo;
    use crate::utils::Span;

    fn simple_non_vectorizable_fn() -> FnIR {
        let mut fn_ir = FnIR::new("tx_fail".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
        let ret = fn_ir.add_value(
            ValueKind::Const(Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(ret));
        fn_ir
    }

    fn loop_info_without_apply_site() -> LoopInfo {
        LoopInfo {
            header: 0,
            latch: 0,
            exits: Vec::new(),
            body: FxHashSet::default(),
            is_seq_len: None,
            is_seq_along: None,
            iv: None,
            limit: None,
            limit_adjust: 0,
        }
    }

    #[test]
    fn transactional_apply_preserves_original_ir_on_failure() {
        let mut fn_ir = simple_non_vectorizable_fn();
        let original = format!("{:?}", fn_ir);
        let lp = loop_info_without_apply_site();
        let plan = VectorPlan::Map {
            dest: 0,
            src: 0,
            op: BinOp::Add,
            other: 0,
            shadow_vars: Vec::new(),
        };

        let applied = try_apply_vectorization_transactionally(&mut fn_ir, &lp, plan);
        assert!(!applied);
        assert_eq!(original, format!("{:?}", fn_ir));
    }
}
