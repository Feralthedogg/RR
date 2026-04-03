use super::analysis::{
    affine_iv_offset, array3_access_stride, as_safe_loop_index, axis3_operand_source,
    axis3_vector_operand_source, canonical_value, classify_3d_general_vector_access,
    classify_3d_map_axis, classify_3d_vector_access_axis, classify_store_1d_in_block,
    classify_store_3d_in_block, collect_loop_shadow_vars_for_dest, expr_has_iv_dependency,
    expr_has_non_vector_safe_call_in_vector_context, expr_reads_base, expr_reads_base_non_iv,
    induction_origin_var, is_condition_vectorizable, is_floor_like_iv_expr, is_iv_equivalent,
    is_loop_compatible_base, is_loop_invariant_axis, is_loop_invariant_scalar_expr,
    is_origin_var_iv_alias_in_loop, is_prev_element, is_prev_element_3d, is_vector_safe_call,
    is_vector_safe_call_chain_expr, is_vectorizable_expr, loop_covers_whole_destination,
    loop_has_store_effect, loop_matches_vec, matrix_access_stride, resolve_base_var,
    resolve_call_info, resolve_load_alias_value, resolve_match_alias_value, same_base_value,
    same_loop_invariant_value, structured_reduction_stride_allowed, unique_assign_source,
};
use super::debug::vectorize_trace_enabled;
use super::reconstruct::{
    expr_has_ambiguous_loop_local_load, expr_has_unstable_loop_local_load, expr_reads_var,
    merged_assign_source_in_loop, phi_state_var, unique_assign_source_in_loop,
    unique_origin_phi_value_in_loop,
};
pub use super::types::{Axis3D, CallMapArg, ExprMapEntry, ExprMapEntry3D, ReduceKind, VectorPlan};
use super::types::{
    BlockStore1DMatch, BlockStore3DMatch, CallMap3DMatchCandidate, VectorAccessOperand3D,
    VectorAccessPattern3D,
};
use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::FxHashSet;

#[path = "planning_expr_map.rs"]
pub(super) mod planning_expr_map;
pub(super) use self::planning_expr_map::*;

pub(super) fn expr_has_non_iv_loop_state_load(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    root: ValueId,
    iv_phi: ValueId,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        lp: &LoopInfo,
        root: ValueId,
        iv_phi: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if is_iv_equivalent(fn_ir, root, iv_phi) || !seen_vals.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Load { var } => {
                if !seen_vars.insert(var.clone()) {
                    return true;
                }
                let out = if let Some(phi) = unique_origin_phi_value_in_loop(fn_ir, lp, var) {
                    !is_iv_equivalent(fn_ir, phi, iv_phi)
                } else if let Some(src) = unique_assign_source_in_loop(fn_ir, lp, var) {
                    rec(fn_ir, lp, src, iv_phi, seen_vals, seen_vars)
                } else if let Some(src) = merged_assign_source_in_loop(fn_ir, lp, var) {
                    rec(fn_ir, lp, src, iv_phi, seen_vals, seen_vars)
                } else {
                    false
                };
                seen_vars.remove(var);
                out
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, lp, *lhs, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *rhs, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, lp, *rhs, iv_phi, seen_vals, seen_vars),
            ValueKind::RecordLit { fields } => fields
                .iter()
                .any(|(_, value)| rec(fn_ir, lp, *value, iv_phi, seen_vals, seen_vars)),
            ValueKind::FieldGet { base, .. } => rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars),
            ValueKind::FieldSet { base, value, .. } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *value, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
                .iter()
                .any(|arg| rec(fn_ir, lp, *arg, iv_phi, seen_vals, seen_vars)),
            ValueKind::Phi { args } => args
                .iter()
                .any(|(arg, _)| rec(fn_ir, lp, *arg, iv_phi, seen_vals, seen_vars)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, lp, *start, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *end, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index1D { base, idx, .. } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *idx, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index2D { base, r, c } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *r, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *c, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index3D { base, i, j, k } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *i, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *j, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *k, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
        }
    }

    rec(
        fn_ir,
        lp,
        root,
        iv_phi,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

pub(super) fn reduction_has_non_acc_loop_state_assignments(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    iv_phi: ValueId,
) -> bool {
    let acc_var =
        phi_state_var(fn_ir, acc_phi).or_else(|| fn_ir.values[acc_phi].origin_var.clone());
    let iv_var = induction_origin_var(fn_ir, iv_phi);
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if acc_var.as_deref() == Some(dst.as_str()) || iv_var.as_deref() == Some(dst.as_str()) {
                continue;
            }
            if dst.starts_with(".arg_") {
                continue;
            }
            if expr_has_non_iv_loop_state_load(fn_ir, lp, *src, iv_phi) {
                return true;
            }
        }
    }
    false
}

pub(super) fn reduction_has_extra_state_phi(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    iv_phi: ValueId,
) -> bool {
    let acc_phi = canonical_value(fn_ir, acc_phi);
    let mut seen = FxHashSet::default();
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() {
            continue;
        }
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        if vid == acc_phi || is_iv_equivalent(fn_ir, vid, iv_phi) || !seen.insert(vid) {
            continue;
        }
        return true;
    }
    false
}

pub(super) fn match_reduction(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let reduction_rhs_vectorizable =
        |root: ValueId| is_vectorizable_expr(fn_ir, root, iv_phi, lp, true, false);
    if loop_has_store_effect(fn_ir, lp) {
        // Conservative: do not fold reductions if loop writes memory.
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        if is_iv_equivalent(fn_ir, id, iv_phi)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, id, iv_phi)
        {
            continue;
        }
        if let ValueKind::Phi { args } = &val.kind
            && args.len() == 2
            && args.iter().any(|(_, b)| *b == lp.latch)
        {
            let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
                continue;
            };
            let next_v = &fn_ir.values[*next_val];

            match &next_v.kind {
                ValueKind::Binary {
                    op: crate::syntax::ast::BinOp::Add,
                    lhs,
                    rhs,
                } => {
                    if *lhs == id || *rhs == id {
                        let other = if *lhs == id { *rhs } else { *lhs };
                        if reduction_has_non_acc_loop_state_assignments(fn_ir, lp, id, iv_phi)
                            || reduction_has_extra_state_phi(fn_ir, lp, id, iv_phi)
                        {
                            continue;
                        }
                        let acc_reads_self = phi_state_var(fn_ir, id)
                            .or_else(|| fn_ir.values[id].origin_var.clone())
                            .is_some_and(|acc_var| {
                                expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default())
                            });
                        if expr_has_iv_dependency(fn_ir, other, iv_phi)
                            && !acc_reads_self
                            && !expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi)
                            && !expr_has_unstable_loop_local_load(fn_ir, lp, other)
                            && !expr_has_ambiguous_loop_local_load(fn_ir, lp, other)
                            && !expr_has_non_vector_safe_call_in_vector_context(
                                fn_ir,
                                other,
                                iv_phi,
                                user_call_whitelist,
                                &mut FxHashSet::default(),
                            )
                            && reduction_rhs_vectorizable(other)
                        {
                            if vectorize_trace_enabled() {
                                let lhs_detail = match &fn_ir.values[other].kind {
                                    ValueKind::Binary { lhs, .. } => {
                                        format!("{:?}", fn_ir.values[*lhs].kind)
                                    }
                                    _ => "-".to_string(),
                                };
                                let rhs_detail = match &fn_ir.values[other].kind {
                                    ValueKind::Binary { rhs, .. } => {
                                        format!("{:?}", fn_ir.values[*rhs].kind)
                                    }
                                    _ => "-".to_string(),
                                };
                                eprintln!(
                                    "   [vec-reduce] {} sum acc={:?} other={:?} lhs={} rhs={}",
                                    fn_ir.name,
                                    fn_ir.values[id].origin_var,
                                    fn_ir.values[other].kind,
                                    lhs_detail,
                                    rhs_detail
                                );
                            }
                            return Some(VectorPlan::Reduce {
                                kind: ReduceKind::Sum,
                                acc_phi: id,
                                vec_expr: other,
                                iv_phi,
                            });
                        }
                    }
                }
                ValueKind::Binary {
                    op: crate::syntax::ast::BinOp::Mul,
                    lhs,
                    rhs,
                } => {
                    if *lhs == id || *rhs == id {
                        let other = if *lhs == id { *rhs } else { *lhs };
                        if reduction_has_non_acc_loop_state_assignments(fn_ir, lp, id, iv_phi)
                            || reduction_has_extra_state_phi(fn_ir, lp, id, iv_phi)
                        {
                            continue;
                        }
                        let acc_reads_self = phi_state_var(fn_ir, id)
                            .or_else(|| fn_ir.values[id].origin_var.clone())
                            .is_some_and(|acc_var| {
                                expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default())
                            });
                        if expr_has_iv_dependency(fn_ir, other, iv_phi)
                            && !acc_reads_self
                            && !expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi)
                            && !expr_has_unstable_loop_local_load(fn_ir, lp, other)
                            && !expr_has_ambiguous_loop_local_load(fn_ir, lp, other)
                            && !expr_has_non_vector_safe_call_in_vector_context(
                                fn_ir,
                                other,
                                iv_phi,
                                user_call_whitelist,
                                &mut FxHashSet::default(),
                            )
                            && reduction_rhs_vectorizable(other)
                        {
                            if vectorize_trace_enabled() {
                                eprintln!(
                                    "   [vec-reduce] {} prod acc={:?} other={:?}",
                                    fn_ir.name,
                                    fn_ir.values[id].origin_var,
                                    fn_ir.values[other].kind
                                );
                            }
                            return Some(VectorPlan::Reduce {
                                kind: ReduceKind::Prod,
                                acc_phi: id,
                                vec_expr: other,
                                iv_phi,
                            });
                        }
                    }
                }
                ValueKind::Call { .. } => {
                    let Some(call) = resolve_call_info(fn_ir, *next_val) else {
                        continue;
                    };
                    if !call.args.len().eq(&2)
                        || !call.builtin_kind.is_some_and(BuiltinKind::is_minmax)
                            && !matches!(call.callee.as_str(), "min" | "max")
                    {
                        continue;
                    }
                    let (a, b) = (call.args[0], call.args[1]);
                    let acc_side = if a == id {
                        Some(b)
                    } else if b == id {
                        Some(a)
                    } else {
                        None
                    };
                    if let Some(other) = acc_side
                        && !reduction_has_non_acc_loop_state_assignments(fn_ir, lp, id, iv_phi)
                        && !reduction_has_extra_state_phi(fn_ir, lp, id, iv_phi)
                        && !phi_state_var(fn_ir, id)
                            .or_else(|| fn_ir.values[id].origin_var.clone())
                            .is_some_and(|acc_var| {
                                expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default())
                            })
                        && expr_has_iv_dependency(fn_ir, other, iv_phi)
                        && !expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi)
                        && !expr_has_unstable_loop_local_load(fn_ir, lp, other)
                        && !expr_has_ambiguous_loop_local_load(fn_ir, lp, other)
                        && !expr_has_non_vector_safe_call_in_vector_context(
                            fn_ir,
                            other,
                            iv_phi,
                            user_call_whitelist,
                            &mut FxHashSet::default(),
                        )
                        && reduction_rhs_vectorizable(other)
                    {
                        if vectorize_trace_enabled() {
                            eprintln!(
                                "   [vec-reduce] {} minmax acc={:?} other={:?} callee={}",
                                fn_ir.name,
                                fn_ir.values[id].origin_var,
                                fn_ir.values[other].kind,
                                call.callee
                            );
                        }
                        let kind = if call.builtin_kind == Some(BuiltinKind::Min)
                            || call.callee == "min"
                        {
                            ReduceKind::Min
                        } else {
                            ReduceKind::Max
                        };
                        return Some(VectorPlan::Reduce {
                            kind,
                            acc_phi: id,
                            vec_expr: other,
                            iv_phi,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    None
}

pub(super) fn match_2d_row_reduction_sum(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    if loop_has_store_effect(fn_ir, lp) {
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        if args.len() != 2 || !args.iter().any(|(_, b)| *b == lp.latch) {
            continue;
        }
        let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
            continue;
        };
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = fn_ir.values[*next_val].kind
        else {
            continue;
        };
        let other = if lhs == id {
            rhs
        } else if rhs == id {
            lhs
        } else {
            continue;
        };
        let ValueKind::Index2D { base, r, c } = fn_ir.values[other].kind else {
            continue;
        };
        if !is_iv_equivalent(fn_ir, c, iv_phi) || expr_has_iv_dependency(fn_ir, r, iv_phi) {
            continue;
        }
        if !is_loop_invariant_axis(fn_ir, r, iv_phi, base) {
            continue;
        }
        if !structured_reduction_stride_allowed(fn_ir, lp, matrix_access_stride(true)) {
            continue;
        }
        return Some(VectorPlan::Reduce2DRowSum {
            acc_phi: id,
            base: canonical_value(fn_ir, base),
            row: r,
            start,
            end,
        });
    }
    None
}

pub(super) fn match_2d_col_reduction_sum(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    if loop_has_store_effect(fn_ir, lp) {
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        if args.len() != 2 || !args.iter().any(|(_, b)| *b == lp.latch) {
            continue;
        }
        let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
            continue;
        };
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = fn_ir.values[*next_val].kind
        else {
            continue;
        };
        let other = if lhs == id {
            rhs
        } else if rhs == id {
            lhs
        } else {
            continue;
        };
        let ValueKind::Index2D { base, r, c } = fn_ir.values[other].kind else {
            continue;
        };
        if !is_iv_equivalent(fn_ir, r, iv_phi) || expr_has_iv_dependency(fn_ir, c, iv_phi) {
            continue;
        }
        if !is_loop_invariant_axis(fn_ir, c, iv_phi, base) {
            continue;
        }
        if !structured_reduction_stride_allowed(fn_ir, lp, matrix_access_stride(false)) {
            continue;
        }
        return Some(VectorPlan::Reduce2DColSum {
            acc_phi: id,
            base: canonical_value(fn_ir, base),
            col: c,
            start,
            end,
        });
    }
    None
}

pub(super) fn match_3d_axis_reduction(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    if loop_has_store_effect(fn_ir, lp) {
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        if args.len() != 2 || !args.iter().any(|(_, b)| *b == lp.latch) {
            continue;
        }
        let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
            continue;
        };
        match &fn_ir.values[*next_val].kind {
            ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            }
            | ValueKind::Binary {
                op: BinOp::Mul,
                lhs,
                rhs,
            } => {
                let kind = match &fn_ir.values[*next_val].kind {
                    ValueKind::Binary { op: BinOp::Add, .. } => ReduceKind::Sum,
                    ValueKind::Binary { op: BinOp::Mul, .. } => ReduceKind::Prod,
                    _ => unreachable!(),
                };
                let other = if *lhs == id {
                    *rhs
                } else if *rhs == id {
                    *lhs
                } else {
                    continue;
                };
                let ValueKind::Index3D { base, i, j, k } = fn_ir.values[other].kind else {
                    continue;
                };
                let Some((axis, fixed_a, fixed_b)) =
                    classify_3d_map_axis(fn_ir, base, i, j, k, iv_phi)
                else {
                    continue;
                };
                if !structured_reduction_stride_allowed(fn_ir, lp, array3_access_stride(axis)) {
                    continue;
                }
                return Some(VectorPlan::Reduce3D {
                    kind,
                    acc_phi: id,
                    base: canonical_value(fn_ir, base),
                    axis,
                    fixed_a,
                    fixed_b,
                    start,
                    end,
                });
            }
            ValueKind::Call { .. } => {
                let Some(call) = resolve_call_info(fn_ir, *next_val) else {
                    continue;
                };
                if !call.args.len().eq(&2)
                    || !call.builtin_kind.is_some_and(BuiltinKind::is_minmax)
                        && !matches!(call.callee.as_str(), "min" | "max")
                {
                    continue;
                }
                let (a, b) = (call.args[0], call.args[1]);
                let other = if a == id {
                    b
                } else if b == id {
                    a
                } else {
                    continue;
                };
                let ValueKind::Index3D { base, i, j, k } = fn_ir.values[other].kind else {
                    continue;
                };
                let Some((axis, fixed_a, fixed_b)) =
                    classify_3d_map_axis(fn_ir, base, i, j, k, iv_phi)
                else {
                    continue;
                };
                if !structured_reduction_stride_allowed(fn_ir, lp, array3_access_stride(axis)) {
                    continue;
                }
                let kind = if call.builtin_kind == Some(BuiltinKind::Min) || call.callee == "min" {
                    ReduceKind::Min
                } else {
                    ReduceKind::Max
                };
                return Some(VectorPlan::Reduce3D {
                    kind,
                    acc_phi: id,
                    base: canonical_value(fn_ir, base),
                    axis,
                    fixed_a,
                    fixed_b,
                    start,
                    end,
                });
            }
            _ => {}
        }
    }
    None
}

pub(super) fn match_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let whole_dest = |dest: ValueId| loop_covers_whole_destination(lp, fn_ir, dest, iv.init_val);

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            if let Instr::StoreIndex1D {
                base,
                idx,
                val,
                is_vector: false,
                is_safe,
                is_na_safe,
                ..
            } = instr
                && is_iv_equivalent(fn_ir, *idx, iv_phi)
                && *is_safe
                && *is_na_safe
            {
                let rhs_val = &fn_ir.values[*val];
                if let ValueKind::Binary { op, lhs, rhs } = &rhs_val.kind {
                    let lhs_idx = as_safe_loop_index(fn_ir, *lhs, iv_phi);
                    let rhs_idx = as_safe_loop_index(fn_ir, *rhs, iv_phi);

                    // x[i] OP x[i]  ->  x OP x
                    if let (Some(lbase), Some(rbase)) = (lhs_idx, rhs_idx)
                        && lbase == rbase
                        && loop_matches_vec(lp, fn_ir, lbase)
                        && whole_dest(*base)
                        && whole_dest(lbase)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                            fn_ir,
                            lp,
                            &allowed_dests,
                            *base,
                            iv_phi,
                        ) else {
                            continue;
                        };
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: lbase,
                            op: *op,
                            other: rbase,
                            shadow_vars,
                        });
                    }

                    // x[i] OP k  ->  x OP k
                    if let Some(x_base) = lhs_idx
                        && loop_matches_vec(lp, fn_ir, x_base)
                        && whole_dest(*base)
                        && whole_dest(x_base)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                            fn_ir,
                            lp,
                            &allowed_dests,
                            *base,
                            iv_phi,
                        ) else {
                            continue;
                        };
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: x_base,
                            op: *op,
                            other: *rhs,
                            shadow_vars,
                        });
                    }

                    // k OP x[i]  ->  k OP x
                    if let Some(x_base) = rhs_idx
                        && loop_matches_vec(lp, fn_ir, x_base)
                        && whole_dest(*base)
                        && whole_dest(x_base)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                            fn_ir,
                            lp,
                            &allowed_dests,
                            *base,
                            iv_phi,
                        ) else {
                            continue;
                        };
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: *lhs,
                            op: *op,
                            other: x_base,
                            shadow_vars,
                        });
                    }
                }
            }
        }
    }
    None
}

pub(super) fn match_conditional_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let trace_enabled = vectorize_trace_enabled();

    for &bid in &lp.body {
        if bid == lp.header {
            continue;
        }
        if let Terminator::If {
            cond,
            then_bb,
            else_bb,
        } = fn_ir.blocks[bid].term
        {
            if !lp.body.contains(&then_bb) || !lp.body.contains(&else_bb) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: branch blocks leave loop body (then={}, else={})",
                        fn_ir.name, then_bb, else_bb
                    );
                }
                continue;
            }
            let cond_vec = is_condition_vectorizable(fn_ir, cond, iv_phi, lp, user_call_whitelist);
            let cond_dep = expr_has_iv_dependency(fn_ir, cond, iv_phi);
            if !cond_vec || !cond_dep {
                if trace_enabled {
                    let cond_detail = match &fn_ir.values[cond].kind {
                        ValueKind::Binary { lhs, rhs, .. } => format!(
                            "lhs={:?} rhs={:?}",
                            fn_ir.values[*lhs].kind, fn_ir.values[*rhs].kind
                        ),
                        other => format!("{:?}", other),
                    };
                    eprintln!(
                        "   [vec-cond-map] {} skip: condition rejected (vec={}, dep={}, cond={:?}, detail={})",
                        fn_ir.name, cond_vec, cond_dep, fn_ir.values[cond].kind, cond_detail
                    );
                }
                continue;
            }
            let then_store = classify_store_1d_in_block(fn_ir, then_bb);
            let else_store = classify_store_1d_in_block(fn_ir, else_bb);
            let (then_base, then_val, else_base, else_val) = match (then_store, else_store) {
                (BlockStore1DMatch::One(then_store), BlockStore1DMatch::One(else_store))
                    if !then_store.is_vector
                        && !else_store.is_vector
                        && is_iv_equivalent(fn_ir, then_store.idx, iv_phi)
                        && is_iv_equivalent(fn_ir, else_store.idx, iv_phi) =>
                {
                    (
                        then_store.base,
                        then_store.val,
                        else_store.base,
                        else_store.val,
                    )
                }
                _ => {
                    if trace_enabled {
                        match then_store {
                            BlockStore1DMatch::One(_) => {}
                            _ => eprintln!(
                                "   [vec-cond-map] {} skip: then branch has no canonical single StoreIndex1D",
                                fn_ir.name
                            ),
                        }
                        match else_store {
                            BlockStore1DMatch::One(_) => {}
                            _ => eprintln!(
                                "   [vec-cond-map] {} skip: else branch has no canonical single StoreIndex1D",
                                fn_ir.name
                            ),
                        }
                    }
                    continue;
                }
            };
            let then_base = canonical_value(fn_ir, then_base);
            let else_base = canonical_value(fn_ir, else_base);
            let dest_base = if then_base == else_base {
                then_base
            } else {
                match (
                    resolve_base_var(fn_ir, then_base),
                    resolve_base_var(fn_ir, else_base),
                ) {
                    (Some(a), Some(b)) if a == b => then_base,
                    _ => {
                        if trace_enabled {
                            eprintln!(
                                "   [vec-cond-map] {} skip: then/else destination bases differ",
                                fn_ir.name
                            );
                        }
                        continue;
                    }
                }
            };
            let allowed_dests: Vec<VarId> =
                resolve_base_var(fn_ir, dest_base).into_iter().collect();
            let Some(shadow_vars) =
                collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest_base, iv_phi)
            else {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: loop carries non-destination state",
                        fn_ir.name
                    );
                }
                continue;
            };
            if !is_vectorizable_expr(fn_ir, then_val, iv_phi, lp, true, false) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: then expression is not vectorizable",
                        fn_ir.name
                    );
                }
                continue;
            }
            if !is_vectorizable_expr(fn_ir, else_val, iv_phi, lp, true, false) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: else expression is not vectorizable",
                        fn_ir.name
                    );
                }
                continue;
            }
            return Some(VectorPlan::CondMap {
                dest: dest_base,
                cond,
                then_val,
                else_val,
                iv_phi,
                start,
                end,
                whole_dest: loop_covers_whole_destination(lp, fn_ir, dest_base, start),
                shadow_vars,
            });
        }
    }
    None
}

pub(super) fn match_conditional_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    _user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let trace_enabled = vectorize_trace_enabled();

    for &bid in &lp.body {
        if bid == lp.header {
            continue;
        }
        let Terminator::If {
            cond,
            then_bb,
            else_bb,
        } = fn_ir.blocks[bid].term
        else {
            continue;
        };
        if !lp.body.contains(&then_bb) || !lp.body.contains(&else_bb) {
            continue;
        }
        if !expr_has_iv_dependency(fn_ir, cond, iv_phi) {
            continue;
        }
        let ValueKind::Binary {
            op: cmp_op,
            lhs,
            rhs,
        } = fn_ir.values[cond].kind
        else {
            continue;
        };
        if !matches!(
            cmp_op,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne
        ) {
            continue;
        }

        let then_store = classify_store_3d_in_block(fn_ir, then_bb);
        let else_store = classify_store_3d_in_block(fn_ir, else_bb);
        let (then_store, else_store) = match (then_store, else_store) {
            (BlockStore3DMatch::One(then_store), BlockStore3DMatch::One(else_store)) => {
                (then_store, else_store)
            }
            _ => {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond3d] {} skip: non-canonical StoreIndex3D branches",
                        fn_ir.name
                    );
                }
                continue;
            }
        };

        let then_base = canonical_value(fn_ir, then_store.base);
        let else_base = canonical_value(fn_ir, else_store.base);
        let dest_base = if then_base == else_base {
            then_base
        } else {
            match (
                resolve_base_var(fn_ir, then_base),
                resolve_base_var(fn_ir, else_base),
            ) {
                (Some(a), Some(b)) if a == b => then_base,
                _ => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-cond3d] {} skip: then/else destination bases differ",
                            fn_ir.name
                        );
                    }
                    continue;
                }
            }
        };

        let Some((then_axis, then_fixed_a, then_fixed_b)) = classify_3d_map_axis(
            fn_ir,
            then_store.base,
            then_store.i,
            then_store.j,
            then_store.k,
            iv_phi,
        ) else {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: then branch axis classify failed",
                    fn_ir.name
                );
            }
            continue;
        };
        let Some((else_axis, else_fixed_a, else_fixed_b)) = classify_3d_map_axis(
            fn_ir,
            else_store.base,
            else_store.i,
            else_store.j,
            else_store.k,
            iv_phi,
        ) else {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: else branch axis classify failed",
                    fn_ir.name
                );
            }
            continue;
        };
        if then_axis != else_axis
            || !same_loop_invariant_value(fn_ir, then_fixed_a, else_fixed_a, iv_phi)
            || !same_loop_invariant_value(fn_ir, then_fixed_b, else_fixed_b, iv_phi)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: axis/fixed dims differ across branches",
                    fn_ir.name
                );
            }
            continue;
        }

        let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest_base).into_iter().collect();
        let Some(shadow_vars) =
            collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest_base, iv_phi)
        else {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: unsupported loop-carried state",
                    fn_ir.name
                );
            }
            continue;
        };
        if !shadow_vars.is_empty() {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: loop carries non-destination state",
                    fn_ir.name
                );
            }
            continue;
        }

        let direct_cond_lhs =
            axis3_operand_source(fn_ir, lhs, then_axis, then_fixed_a, then_fixed_b, iv_phi);
        let direct_cond_rhs =
            axis3_operand_source(fn_ir, rhs, then_axis, then_fixed_a, then_fixed_b, iv_phi);
        let direct_then = axis3_operand_source(
            fn_ir,
            then_store.val,
            then_axis,
            then_fixed_a,
            then_fixed_b,
            iv_phi,
        );
        let direct_else = axis3_operand_source(
            fn_ir,
            else_store.val,
            then_axis,
            then_fixed_a,
            then_fixed_b,
            iv_phi,
        );

        if let (Some(cond_lhs), Some(cond_rhs), Some(then_src), Some(else_src)) =
            (direct_cond_lhs, direct_cond_rhs, direct_then, direct_else)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} matched axis={:?} fixed_a={:?} fixed_b={:?}",
                    fn_ir.name, then_axis, then_fixed_a, then_fixed_b
                );
            }
            return Some(VectorPlan::CondMap3D {
                dest: dest_base,
                axis: then_axis,
                fixed_a: then_fixed_a,
                fixed_b: then_fixed_b,
                cond_lhs,
                cond_rhs,
                cmp_op,
                then_src,
                else_src,
                start,
                end,
            });
        }

        let cond_operands_ok = |root: ValueId| {
            is_vectorizable_expr(fn_ir, root, iv_phi, lp, true, false)
                || is_loop_invariant_scalar_expr(fn_ir, root, iv_phi, &FxHashSet::default())
        };
        if !cond_operands_ok(lhs) || !cond_operands_ok(rhs) {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: condition operands are not vectorizable",
                    fn_ir.name
                );
            }
            continue;
        }
        if !is_vectorizable_expr(fn_ir, then_store.val, iv_phi, lp, true, false)
            || !is_vectorizable_expr(fn_ir, else_store.val, iv_phi, lp, true, false)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: branch value is not vectorizable",
                    fn_ir.name
                );
            }
            continue;
        }
        if expr_has_unstable_loop_local_load(fn_ir, lp, lhs)
            || expr_has_unstable_loop_local_load(fn_ir, lp, rhs)
            || expr_has_unstable_loop_local_load(fn_ir, lp, then_store.val)
            || expr_has_unstable_loop_local_load(fn_ir, lp, else_store.val)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: unstable loop-local load in generalized conditional map",
                    fn_ir.name
                );
            }
            continue;
        }

        if trace_enabled {
            eprintln!(
                "   [vec-cond3d] {} matched generalized axis={:?} fixed_a={:?} fixed_b={:?}",
                fn_ir.name, then_axis, then_fixed_a, then_fixed_b
            );
        }
        return Some(VectorPlan::CondMap3DGeneral {
            dest: dest_base,
            axis: then_axis,
            fixed_a: then_fixed_a,
            fixed_b: then_fixed_b,
            cond_lhs: lhs,
            cond_rhs: rhs,
            cmp_op,
            then_val: then_store.val,
            else_val: else_store.val,
            iv_phi,
            start,
            end,
        });
    }
    None
}

pub(super) fn match_recurrence_add_const(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let start = iv.init_val;
    let end = lp.limit?;
    let iv_phi = iv.phi_val;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (base, idx, val, is_vector) = match instr {
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => (*base, *idx, *val, *is_vector),
                _ => continue,
            };
            if is_vector || !is_iv_equivalent(fn_ir, idx, iv_phi) {
                continue;
            }
            let base = canonical_value(fn_ir, base);
            if !is_loop_compatible_base(lp, fn_ir, base) {
                continue;
            }

            let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[val].kind else {
                continue;
            };
            if !matches!(*op, BinOp::Add | BinOp::Sub) {
                continue;
            }

            let (prev_side, delta_side, negate_delta) =
                if is_prev_element(fn_ir, *lhs, base, iv_phi) {
                    // a[i] = a[i-1] + delta  or  a[i] = a[i-1] - delta
                    (*lhs, *rhs, *op == BinOp::Sub)
                } else if *op == BinOp::Add && is_prev_element(fn_ir, *rhs, base, iv_phi) {
                    // a[i] = delta + a[i-1]
                    (*rhs, *lhs, false)
                } else {
                    continue;
                };

            if !is_prev_element(fn_ir, prev_side, base, iv_phi) {
                continue;
            }
            if expr_has_iv_dependency(fn_ir, delta_side, iv_phi) {
                continue;
            }
            if expr_reads_base(fn_ir, delta_side, base) {
                continue;
            }

            return Some(VectorPlan::RecurrenceAddConst {
                base,
                start,
                end,
                delta: delta_side,
                negate_delta,
            });
        }
    }
    None
}

pub(super) fn match_recurrence_add_const_3d(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let start = iv.init_val;
    let end = lp.limit?;
    let iv_phi = iv.phi_val;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (base, i, j, k, val) = match instr {
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => (*base, *i, *j, *k, *val),
                _ => continue,
            };
            let Some((axis, fixed_a, fixed_b)) = classify_3d_map_axis(fn_ir, base, i, j, k, iv_phi)
            else {
                continue;
            };
            let base = canonical_value(fn_ir, base);
            let val = resolve_load_alias_value(fn_ir, val);

            let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[val].kind else {
                continue;
            };
            if !matches!(op, BinOp::Add | BinOp::Sub) {
                continue;
            }
            let lhs = resolve_load_alias_value(fn_ir, *lhs);
            let rhs = resolve_load_alias_value(fn_ir, *rhs);

            let (prev_side, delta_side, negate_delta) =
                if is_prev_element_3d(fn_ir, lhs, base, axis, fixed_a, fixed_b, iv_phi) {
                    (lhs, rhs, *op == BinOp::Sub)
                } else if *op == BinOp::Add
                    && is_prev_element_3d(fn_ir, rhs, base, axis, fixed_a, fixed_b, iv_phi)
                {
                    (rhs, lhs, false)
                } else {
                    continue;
                };

            if !is_prev_element_3d(fn_ir, prev_side, base, axis, fixed_a, fixed_b, iv_phi) {
                continue;
            }
            if expr_has_iv_dependency(fn_ir, delta_side, iv_phi) {
                continue;
            }
            if expr_reads_base(fn_ir, delta_side, base) {
                continue;
            }

            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-recur3d] {} matched axis={:?} fixed_a={:?} fixed_b={:?} negate_delta={}",
                    fn_ir.name, axis, fixed_a, fixed_b, negate_delta
                );
            }
            return Some(VectorPlan::RecurrenceAddConst3D {
                base,
                axis,
                fixed_a,
                fixed_b,
                start,
                end,
                delta: delta_side,
                negate_delta,
            });
        }
    }
    None
}

pub(super) fn match_shifted_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest_base, dest_idx, rhs, is_vector) = match instr {
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => (*base, *idx, *val, *is_vector),
                _ => continue,
            };
            if is_vector || !is_iv_equivalent(fn_ir, dest_idx, iv_phi) {
                continue;
            }

            let ValueKind::Index1D {
                base: src_base,
                idx: src_idx,
                ..
            } = fn_ir.values[rhs].kind.clone()
            else {
                continue;
            };

            let Some(offset) = affine_iv_offset(fn_ir, src_idx, iv_phi) else {
                continue;
            };
            if offset == 0 {
                continue;
            }

            let d = canonical_value(fn_ir, dest_base);
            let s = canonical_value(fn_ir, src_base);
            if d == s && offset < 0 {
                // x[i+1] = x[i] is loop-carried: slice assignment would read the original RHS
                // instead of the progressively updated scalar state.
                continue;
            }
            return Some(VectorPlan::ShiftedMap {
                dest: d,
                src: s,
                start,
                end,
                offset,
            });
        }
    }
    None
}

pub(super) fn match_shifted_map_3d(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest_base, i, j, k, rhs) = match instr {
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => (*base, *i, *j, *k, *val),
                _ => continue,
            };
            let Some((axis, fixed_a, fixed_b)) =
                classify_3d_map_axis(fn_ir, dest_base, i, j, k, iv_phi)
            else {
                continue;
            };

            let rhs = resolve_load_alias_value(fn_ir, rhs);
            let ValueKind::Index3D {
                base: src_base,
                i: src_i,
                j: src_j,
                k: src_k,
            } = fn_ir.values[rhs].kind.clone()
            else {
                continue;
            };

            let (src_axis_idx, src_fixed_a, src_fixed_b) = match axis {
                Axis3D::Dim1 => (src_i, src_j, src_k),
                Axis3D::Dim2 => (src_j, src_i, src_k),
                Axis3D::Dim3 => (src_k, src_i, src_j),
            };
            let src_axis_idx = resolve_load_alias_value(fn_ir, src_axis_idx);
            let src_fixed_a = resolve_load_alias_value(fn_ir, src_fixed_a);
            let src_fixed_b = resolve_load_alias_value(fn_ir, src_fixed_b);
            let fixed_a = resolve_load_alias_value(fn_ir, fixed_a);
            let fixed_b = resolve_load_alias_value(fn_ir, fixed_b);
            if !same_loop_invariant_value(fn_ir, src_fixed_a, fixed_a, iv_phi)
                || !same_loop_invariant_value(fn_ir, src_fixed_b, fixed_b, iv_phi)
            {
                continue;
            }

            let Some(offset) = affine_iv_offset(fn_ir, src_axis_idx, iv_phi) else {
                continue;
            };
            if offset == 0 {
                continue;
            }

            let d = canonical_value(fn_ir, dest_base);
            let s = canonical_value(fn_ir, src_base);
            if d == s && offset < 0 {
                continue;
            }
            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-shift3d] {} matched axis={:?} offset={} fixed_a={:?} fixed_b={:?}",
                    fn_ir.name, axis, offset, fixed_a, fixed_b
                );
            }
            return Some(VectorPlan::ShiftedMap3D {
                dest: d,
                src: s,
                axis,
                fixed_a,
                fixed_b,
                start,
                end,
                offset,
            });
        }
    }
    None
}

pub(super) fn match_call_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest_base, dest_idx, rhs, is_vector, is_safe, is_na_safe) = match instr {
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    is_safe,
                    is_na_safe,
                    ..
                } => (*base, *idx, *val, *is_vector, *is_safe, *is_na_safe),
                _ => continue,
            };
            if is_vector || !is_safe || !is_na_safe || !is_iv_equivalent(fn_ir, dest_idx, iv_phi) {
                continue;
            }
            let rhs = resolve_load_alias_value(fn_ir, rhs);
            let ValueKind::Call { .. } = &fn_ir.values[rhs].kind else {
                continue;
            };
            let Some(call) = resolve_call_info(fn_ir, rhs) else {
                continue;
            };
            if !is_vector_safe_call(&call.callee, call.args.len(), user_call_whitelist) {
                continue;
            }

            let mut mapped_args = Vec::with_capacity(call.args.len());
            let mut has_vector_arg = false;
            for arg in &call.args {
                let arg = resolve_load_alias_value(fn_ir, *arg);
                if expr_has_iv_dependency(fn_ir, arg, iv_phi) {
                    if !is_vector_safe_call_chain_expr(fn_ir, arg, iv_phi, lp, user_call_whitelist)
                    {
                        mapped_args.clear();
                        break;
                    }
                    mapped_args.push(CallMapArg {
                        value: arg,
                        vectorized: true,
                    });
                    has_vector_arg = true;
                } else {
                    if !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, user_call_whitelist) {
                        mapped_args.clear();
                        break;
                    }
                    mapped_args.push(CallMapArg {
                        value: arg,
                        vectorized: false,
                    });
                }
            }
            if mapped_args.is_empty() || !has_vector_arg {
                continue;
            }
            let allowed_dests: Vec<VarId> =
                resolve_base_var(fn_ir, canonical_value(fn_ir, dest_base))
                    .into_iter()
                    .collect();
            let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                fn_ir,
                lp,
                &allowed_dests,
                canonical_value(fn_ir, dest_base),
                iv_phi,
            ) else {
                continue;
            };
            return Some(VectorPlan::CallMap {
                dest: canonical_value(fn_ir, dest_base),
                callee: call.callee,
                args: mapped_args,
                iv_phi,
                start,
                end,
                whole_dest: loop_covers_whole_destination(
                    lp,
                    fn_ir,
                    canonical_value(fn_ir, dest_base),
                    start,
                ),
                shadow_vars,
            });
        }
    }
    None
}

pub(super) fn match_call_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut found: Option<CallMap3DMatchCandidate> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } | Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => {
                    return None;
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    if found.is_some() {
                        return None;
                    }
                    let rhs = resolve_load_alias_value(fn_ir, *val);
                    let ValueKind::Call { .. } = &fn_ir.values[rhs].kind else {
                        continue;
                    };
                    let Some(call) = resolve_call_info(fn_ir, rhs) else {
                        continue;
                    };
                    if !is_vector_safe_call(&call.callee, call.args.len(), user_call_whitelist) {
                        continue;
                    }
                    let Some((axis, fixed_a, fixed_b)) =
                        classify_3d_map_axis(fn_ir, *base, *i, *j, *k, iv_phi)
                    else {
                        continue;
                    };
                    let mut mapped_args = Vec::with_capacity(call.args.len());
                    let mut has_vector_arg = false;
                    let mut generalized = false;
                    for arg in &call.args {
                        let arg = resolve_load_alias_value(fn_ir, *arg);
                        if expr_has_iv_dependency(fn_ir, arg, iv_phi) {
                            if let Some(src_base) = axis3_vector_operand_source(
                                fn_ir, arg, axis, fixed_a, fixed_b, iv_phi,
                            ) {
                                mapped_args.push(CallMapArg {
                                    value: src_base,
                                    vectorized: true,
                                });
                            } else if is_vectorizable_expr(fn_ir, arg, iv_phi, lp, true, false)
                                && !expr_has_unstable_loop_local_load(fn_ir, lp, arg)
                            {
                                mapped_args.push(CallMapArg {
                                    value: arg,
                                    vectorized: true,
                                });
                                generalized = true;
                            } else {
                                mapped_args.clear();
                                break;
                            }
                            has_vector_arg = true;
                        } else if is_loop_invariant_scalar_expr(
                            fn_ir,
                            arg,
                            iv_phi,
                            user_call_whitelist,
                        ) {
                            mapped_args.push(CallMapArg {
                                value: arg,
                                vectorized: false,
                            });
                        } else {
                            mapped_args.clear();
                            break;
                        }
                    }
                    if mapped_args.is_empty() || !has_vector_arg {
                        continue;
                    }
                    let allowed_dests: Vec<VarId> =
                        resolve_base_var(fn_ir, canonical_value(fn_ir, *base))
                            .into_iter()
                            .collect();
                    let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                        fn_ir,
                        lp,
                        &allowed_dests,
                        canonical_value(fn_ir, *base),
                        iv_phi,
                    ) else {
                        continue;
                    };
                    if !shadow_vars.is_empty() {
                        continue;
                    }
                    found = Some(CallMap3DMatchCandidate {
                        dest: canonical_value(fn_ir, *base),
                        axis,
                        fixed_a,
                        fixed_b,
                        callee: call.callee,
                        args: mapped_args,
                        generalized,
                    });
                }
            }
        }
    }

    let CallMap3DMatchCandidate {
        dest,
        axis,
        fixed_a,
        fixed_b,
        callee,
        args,
        generalized,
    } = found?;
    if generalized {
        Some(VectorPlan::CallMap3DGeneral {
            dest,
            callee,
            args,
            axis,
            fixed_a,
            fixed_b,
            iv_phi,
            start,
            end,
        })
    } else {
        Some(VectorPlan::CallMap3D {
            dest,
            callee,
            args,
            axis,
            fixed_a,
            fixed_b,
            start,
            end,
        })
    }
}

pub(super) fn match_multi_expr_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut pending_entries: Vec<(ValueId, ValueId, Axis3D, ValueId, ValueId)> = Vec::new();

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } | Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => {
                    return None;
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    let (axis, fixed_a, fixed_b) =
                        classify_3d_map_axis(fn_ir, *base, *i, *j, *k, iv_phi)?;
                    let expr = resolve_load_alias_value(fn_ir, *val);
                    if !is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false) {
                        return None;
                    }
                    if expr_has_non_vector_safe_call_in_vector_context(
                        fn_ir,
                        expr,
                        iv_phi,
                        user_call_whitelist,
                        &mut FxHashSet::default(),
                    ) {
                        return None;
                    }
                    if expr_has_unstable_loop_local_load(fn_ir, lp, expr) {
                        return None;
                    }
                    let dest = canonical_value(fn_ir, *base);
                    if expr_reads_base(fn_ir, expr, dest) {
                        return None;
                    }
                    if pending_entries
                        .iter()
                        .any(|(prev_dest, _, prev_axis, prev_a, prev_b)| {
                            *prev_dest == dest
                                && *prev_axis == axis
                                && same_loop_invariant_value(fn_ir, *prev_a, fixed_a, iv_phi)
                                && same_loop_invariant_value(fn_ir, *prev_b, fixed_b, iv_phi)
                        })
                    {
                        return None;
                    }
                    pending_entries.push((dest, expr, axis, fixed_a, fixed_b));
                }
            }
        }
    }

    if pending_entries.len() <= 1 {
        return None;
    }

    let allowed_dests: Vec<VarId> = pending_entries
        .iter()
        .filter_map(|(dest, ..)| resolve_base_var(fn_ir, *dest))
        .collect();
    let mut entries = Vec::with_capacity(pending_entries.len());
    for (dest, expr, axis, fixed_a, fixed_b) in pending_entries {
        let shadow_vars =
            collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;
        entries.push(ExprMapEntry3D {
            dest,
            expr,
            axis,
            fixed_a,
            fixed_b,
            shadow_vars,
        });
    }

    Some(VectorPlan::MultiExprMap3D {
        entries,
        iv_phi,
        start,
        end,
    })
}

pub(super) fn match_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut found: Vec<(ValueId, ValueId)> = Vec::new();
    let mut cube_candidate: Option<(ValueId, ValueId, CubeSliceIndexInfo)> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } | Instr::Eval { .. } => {}
                Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => return None,
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => {
                    let candidate = classify_expr_map_store_candidate(
                        fn_ir,
                        lp,
                        user_call_whitelist,
                        ExprMapStoreSpec {
                            base: *base,
                            idx: *idx,
                            expr: *val,
                            is_vector: *is_vector,
                        },
                        iv_phi,
                    )?;

                    match candidate {
                        ExprMapStoreCandidate::Standard { dest, expr } => {
                            if cube_candidate.is_some()
                                || found.iter().any(|(existing_dest, _)| {
                                    match (
                                        resolve_base_var(fn_ir, *existing_dest),
                                        resolve_base_var(fn_ir, dest),
                                    ) {
                                        (Some(a), Some(b)) => a == b,
                                        _ => same_base_value(fn_ir, *existing_dest, dest),
                                    }
                                })
                            {
                                trace_expr_map_duplicate_store_reject();
                                return None;
                            }
                            found.push((dest, expr));
                        }
                        ExprMapStoreCandidate::Cube { dest, expr, cube } => {
                            if !found.is_empty() || cube_candidate.is_some() {
                                trace_expr_map_duplicate_store_reject();
                                return None;
                            }
                            cube_candidate = Some((dest, expr, cube));
                        }
                    }
                }
            }
        }
    }

    finalize_expr_map_plan(fn_ir, lp, iv_phi, start, end, found, cube_candidate)
}

pub(crate) fn is_builtin_vector_safe_call(callee: &str, arity: usize) -> bool {
    let callee = callee.strip_prefix("base::").unwrap_or(callee);
    match callee {
        "abs" | "sqrt" | "exp" | "log" | "log10" | "log2" | "sin" | "cos" | "tan" | "asin"
        | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "sign" | "floor" | "ceiling" | "trunc"
        | "gamma" | "lgamma" | "is.na" | "is.finite" | "seq_len" => arity == 1,
        "rr_wrap_index_vec" | "rr_wrap_index_vec_i" => arity == 4 || arity == 5,
        "rr_idx_cube_vec_i" => arity == 4 || arity == 5,
        "rr_index_vec_floor" => arity == 1 || arity == 2,
        "rr_index1_read_vec" | "rr_index1_read_vec_floor" | "rr_gather" => arity == 2 || arity == 3,
        "atan2" => arity == 2,
        "round" => arity == 1 || arity == 2,
        "pmax" | "pmin" => arity >= 2,
        _ => false,
    }
}
