//! Analysis helpers for vectorization planning and proof checks.
//!
//! This module answers "can this loop/value be vectorized safely?" style
//! questions and intentionally stays on the read-only analysis side of the
//! vector optimizer boundary.

use super::debug::{VectorizeSkipReason, vectorize_trace_enabled};
use super::planning::{Axis3D, CallMapArg, VectorPlan, is_builtin_vector_safe_call};
use super::reconstruct::{
    has_non_passthrough_assignment_in_loop, is_scalar_broadcast_value,
    unique_assign_source_in_loop, unique_assign_source_reaching_block_in_loop,
    value_use_block_in_loop,
};
use super::types::{
    BlockStore1D, BlockStore1DMatch, BlockStore3D, BlockStore3DMatch, CallMapLoweringMode,
    MemoryStrideClass, VectorAccessOperand3D, VectorAccessPattern3D,
};
use crate::mir::analyze::effects;
use crate::mir::opt::loop_analysis::{LoopInfo, build_pred_map};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::{FxHashMap, FxHashSet};

#[path = "analysis_vectorization.rs"]
mod analysis_vectorization;

pub(crate) use self::analysis_vectorization::*;

pub(super) const CALL_MAP_AUTO_HELPER_COST_THRESHOLD: u32 = 6;
pub(super) const MAX_STRIDED_REDUCTION_TRIP_HINT: u64 = 16;
const VOPT_PROOF_RECURSION_LIMIT: usize = 256;
const VOPT_PROOF_VISIT_LIMIT: usize = 8_192;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ResolvedCallInfo {
    pub callee: String,
    pub builtin_kind: Option<BuiltinKind>,
    pub args: Vec<ValueId>,
}

pub(super) fn normalize_callee_name(callee: &str) -> &str {
    callee.strip_prefix("base::").unwrap_or(callee)
}

fn proof_budget_exhausted(
    depth: usize,
    seen_vals: &FxHashSet<ValueId>,
    seen_vars: &FxHashSet<String>,
) -> bool {
    depth >= VOPT_PROOF_RECURSION_LIMIT
        || seen_vals.len().saturating_add(seen_vars.len()) >= VOPT_PROOF_VISIT_LIMIT
}

fn resolve_callable_name(
    fn_ir: &FnIR,
    value: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
    depth: usize,
) -> Option<String> {
    if proof_budget_exhausted(depth, seen_vals, seen_vars) {
        return None;
    }
    let value = canonical_value(fn_ir, value);
    if !seen_vals.insert(value) {
        return None;
    }
    let out = match &fn_ir.values[value].kind {
        ValueKind::Load { var } => {
            if !seen_vars.insert(var.clone()) {
                None
            } else {
                let resolved = if let Some(src) = unique_assign_source(fn_ir, var)
                    && src != value
                {
                    resolve_callable_name(fn_ir, src, seen_vals, seen_vars, depth + 1)
                } else {
                    Some(normalize_callee_name(var).to_string())
                };
                seen_vars.remove(var);
                resolved
            }
        }
        ValueKind::RSymbol { name } => Some(normalize_callee_name(name).to_string()),
        ValueKind::Phi { args } if !args.is_empty() => {
            let mut resolved: Option<String> = None;
            for (arg, _) in args {
                let name = resolve_callable_name(fn_ir, *arg, seen_vals, seen_vars, depth + 1)?;
                match &resolved {
                    None => resolved = Some(name),
                    Some(prev) if prev == &name => {}
                    Some(_) => return None,
                }
            }
            resolved
        }
        _ => None,
    };
    seen_vals.remove(&value);
    out
}

pub(super) fn resolve_call_info(fn_ir: &FnIR, value: ValueId) -> Option<ResolvedCallInfo> {
    let value = canonical_value(fn_ir, value);
    let ValueKind::Call { callee, args, .. } = &fn_ir.values[value].kind else {
        return None;
    };
    if callee != "rr_call_closure" {
        let builtin_kind = match fn_ir.call_semantics(value) {
            Some(CallSemantics::Builtin(kind)) => Some(kind),
            _ => builtin_kind_for_name(callee),
        };
        return Some(ResolvedCallInfo {
            callee: builtin_kind
                .map(BuiltinKind::canonical_name)
                .unwrap_or_else(|| normalize_callee_name(callee))
                .to_string(),
            builtin_kind,
            args: args.clone(),
        });
    }
    let (callee_value, call_args) = args.split_first()?;
    let callee = resolve_callable_name(
        fn_ir,
        *callee_value,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
        0,
    )?;
    let builtin_kind = builtin_kind_for_name(&callee);
    Some(ResolvedCallInfo {
        callee: builtin_kind
            .map(BuiltinKind::canonical_name)
            .unwrap_or(callee.as_str())
            .to_string(),
        builtin_kind,
        args: call_args.to_vec(),
    })
}

fn call_is_semantically_pure(callee: &str) -> bool {
    effects::call_is_pure(callee) || effects::call_is_pure(normalize_callee_name(callee))
}

pub(super) fn matrix_access_stride(varying_col: bool) -> MemoryStrideClass {
    if varying_col {
        MemoryStrideClass::Strided
    } else {
        MemoryStrideClass::Contiguous
    }
}

pub(super) fn array3_access_stride(axis: Axis3D) -> MemoryStrideClass {
    match axis {
        Axis3D::Dim1 => MemoryStrideClass::Contiguous,
        Axis3D::Dim2 | Axis3D::Dim3 => MemoryStrideClass::Strided,
    }
}

pub(super) fn structured_reduction_stride_allowed(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    stride: MemoryStrideClass,
) -> bool {
    matches!(stride, MemoryStrideClass::Contiguous)
        || estimate_loop_trip_count_hint(fn_ir, lp)
            .is_some_and(|trip| trip <= MAX_STRIDED_REDUCTION_TRIP_HINT)
}

pub(super) fn last_assign_to_var_in_block(
    fn_ir: &FnIR,
    bid: BlockId,
    var: &str,
) -> Option<ValueId> {
    let block = fn_ir.blocks.get(bid)?;
    for ins in block.instrs.iter().rev() {
        let Instr::Assign { dst, src, .. } = ins else {
            continue;
        };
        if dst == var {
            return Some(canonical_value(fn_ir, *src));
        }
    }
    None
}

pub(super) fn block_successors(fn_ir: &FnIR, bid: BlockId) -> [Option<BlockId>; 2] {
    match fn_ir.blocks[bid].term {
        Terminator::Goto(target) => [Some(target), None],
        Terminator::If {
            then_bb, else_bb, ..
        } => [Some(then_bb), Some(else_bb)],
        Terminator::Return(_) | Terminator::Unreachable => [None, None],
    }
}

pub(super) fn block_reaches_before_merge(
    fn_ir: &FnIR,
    start: BlockId,
    target: BlockId,
    merge_bb: BlockId,
) -> bool {
    let mut seen = FxHashSet::default();
    let mut stack = vec![start];
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        if bid == target {
            return true;
        }
        if bid == merge_bb {
            continue;
        }
        for succ in block_successors(fn_ir, bid).into_iter().flatten() {
            stack.push(succ);
        }
    }
    false
}

pub(super) fn collect_if_ancestors_with_distance(
    fn_ir: &FnIR,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
    start: BlockId,
) -> FxHashMap<BlockId, usize> {
    let mut out = FxHashMap::default();
    let mut seen = FxHashSet::default();
    let mut queue = std::collections::VecDeque::from([(start, 0usize)]);
    while let Some((bid, dist)) = queue.pop_front() {
        if !seen.insert(bid) {
            continue;
        }
        if matches!(fn_ir.blocks[bid].term, Terminator::If { .. }) {
            out.entry(bid).or_insert(dist);
        }
        if let Some(ps) = preds.get(&bid) {
            for pred in ps {
                queue.push_back((*pred, dist + 1));
            }
        }
    }
    out
}

#[allow(dead_code)]
pub(super) fn find_conditional_phi_shape_with_blocks(
    fn_ir: &FnIR,
    root: ValueId,
    args: &[(ValueId, BlockId)],
) -> Option<(BlockId, ValueId, ValueId, BlockId, ValueId, BlockId)> {
    if args.len() != 2 {
        return None;
    }
    let merge_bb = fn_ir.values[root].phi_block?;
    if merge_bb >= fn_ir.blocks.len() {
        return None;
    }
    if args
        .iter()
        .any(|(arg, _)| canonical_value(fn_ir, *arg) == root)
    {
        return None;
    }

    let preds = build_pred_map(fn_ir);

    if let (Some((left_branch, left_arm)), Some((right_branch, right_arm))) = (
        branch_origin_for_merge(fn_ir, &preds, merge_bb, args[0].1),
        branch_origin_for_merge(fn_ir, &preds, merge_bb, args[1].1),
    ) && left_branch == right_branch
    {
        let Terminator::If {
            cond,
            then_bb,
            else_bb,
        } = fn_ir.blocks[left_branch].term
        else {
            return None;
        };

        if left_arm == then_bb && right_arm == else_bb {
            return Some((left_branch, cond, args[0].0, left_arm, args[1].0, right_arm));
        }
        if left_arm == else_bb && right_arm == then_bb {
            return Some((left_branch, cond, args[1].0, right_arm, args[0].0, left_arm));
        }
    }

    let left_ifs = collect_if_ancestors_with_distance(fn_ir, &preds, args[0].1);
    let right_ifs = collect_if_ancestors_with_distance(fn_ir, &preds, args[1].1);
    let mut best: Option<(usize, BlockId)> = None;
    for (cand, left_dist) in &left_ifs {
        let Some(right_dist) = right_ifs.get(cand) else {
            continue;
        };
        let score = (*left_dist).max(*right_dist) * 1024 + (*left_dist).min(*right_dist);
        match best {
            None => best = Some((score, *cand)),
            Some((best_score, best_bid))
                if score < best_score || (score == best_score && *cand > best_bid) =>
            {
                best = Some((score, *cand));
            }
            Some(_) => {}
        }
    }

    let candidate = best.map(|(_, bid)| bid)?;
    let Terminator::If {
        cond,
        then_bb,
        else_bb,
    } = fn_ir.blocks[candidate].term
    else {
        return None;
    };

    if block_reaches_before_merge(fn_ir, then_bb, args[0].1, merge_bb)
        && block_reaches_before_merge(fn_ir, else_bb, args[1].1, merge_bb)
    {
        return Some((candidate, cond, args[0].0, args[0].1, args[1].0, args[1].1));
    }
    if block_reaches_before_merge(fn_ir, then_bb, args[1].1, merge_bb)
        && block_reaches_before_merge(fn_ir, else_bb, args[0].1, merge_bb)
    {
        return Some((candidate, cond, args[1].0, args[1].1, args[0].0, args[0].1));
    }
    None
}

pub(super) fn find_conditional_phi_shape(
    fn_ir: &FnIR,
    root: ValueId,
    args: &[(ValueId, BlockId)],
) -> Option<(ValueId, ValueId, ValueId)> {
    find_conditional_phi_shape_with_blocks(fn_ir, root, args)
        .map(|(_, cond, then_val, _, else_val, _)| (cond, then_val, else_val))
}

pub(super) fn branch_origin_for_merge(
    fn_ir: &FnIR,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
    merge_bb: BlockId,
    mut block: BlockId,
) -> Option<(BlockId, BlockId)> {
    loop {
        if block == merge_bb {
            return None;
        }
        let block_preds = preds.get(&block)?;
        if block_preds.len() != 1 {
            return None;
        }
        let pred = block_preds[0];
        match fn_ir.blocks[pred].term {
            Terminator::Goto(target) if target == block => {
                block = pred;
            }
            Terminator::If {
                then_bb, else_bb, ..
            } => {
                if block == then_bb || block == else_bb {
                    return Some((pred, block));
                }
                return None;
            }
            Terminator::Goto(_) | Terminator::Return(_) | Terminator::Unreachable => {
                return None;
            }
        }
    }
}

pub(super) fn is_passthrough_load_of_var(fn_ir: &FnIR, src: ValueId, var: &str) -> bool {
    matches!(
        &fn_ir.values[canonical_value(fn_ir, src)].kind,
        ValueKind::Load { var: load_var } if load_var == var
    )
}

pub(super) fn is_prior_origin_phi_state(
    fn_ir: &FnIR,
    src: ValueId,
    var: &str,
    before_bb: BlockId,
) -> bool {
    let src = canonical_value(fn_ir, src);
    matches!(&fn_ir.values[src].kind, ValueKind::Phi { args } if !args.is_empty())
        && fn_ir.values[src].origin_var.as_deref() == Some(var)
        && fn_ir.values[src]
            .phi_block
            .is_some_and(|phi_bb| phi_bb < before_bb)
}

pub(super) fn collapse_prior_origin_phi_state(
    fn_ir: &FnIR,
    src: ValueId,
    var: &str,
    before_bb: BlockId,
    seen: &mut FxHashSet<ValueId>,
) -> Option<ValueId> {
    let src = canonical_value(fn_ir, src);
    if !seen.insert(src) {
        return None;
    }
    let out = match &fn_ir.values[src].kind {
        ValueKind::Phi { args }
            if !args.is_empty()
                && fn_ir.values[src].origin_var.as_deref() == Some(var)
                && fn_ir.values[src]
                    .phi_block
                    .is_some_and(|phi_bb| phi_bb < before_bb) =>
        {
            let mut candidates: Vec<ValueId> = Vec::new();
            for (arg, _) in args {
                let arg = canonical_value(fn_ir, *arg);
                if arg == src || is_passthrough_load_of_var(fn_ir, arg, var) {
                    continue;
                }
                if is_prior_origin_phi_state(fn_ir, arg, var, before_bb) {
                    if let Some(collapsed) =
                        collapse_prior_origin_phi_state(fn_ir, arg, var, before_bb, seen)
                    {
                        candidates.push(canonical_value(fn_ir, collapsed));
                    } else {
                        candidates.push(arg);
                    }
                } else {
                    candidates.push(arg);
                }
            }
            candidates.sort_unstable();
            candidates.dedup();
            match candidates.as_slice() {
                [only] => Some(*only),
                _ => None,
            }
        }
        _ => Some(src),
    };
    seen.remove(&src);
    out
}

pub(super) fn value_depends_on(
    fn_ir: &FnIR,
    root: ValueId,
    target: ValueId,
    visiting: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    let target = canonical_value(fn_ir, target);
    if root == target {
        return true;
    }
    if !visiting.insert(root) {
        return false;
    }
    let out = match &fn_ir.values[root].kind {
        ValueKind::RecordLit { fields } => fields
            .iter()
            .any(|(_, value)| value_depends_on(fn_ir, *value, target, visiting)),
        ValueKind::FieldGet { base, .. } => value_depends_on(fn_ir, *base, target, visiting),
        ValueKind::FieldSet { base, value, .. } => {
            value_depends_on(fn_ir, *base, target, visiting)
                || value_depends_on(fn_ir, *value, target, visiting)
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            value_depends_on(fn_ir, *lhs, target, visiting)
                || value_depends_on(fn_ir, *rhs, target, visiting)
        }
        ValueKind::Unary { rhs, .. } => value_depends_on(fn_ir, *rhs, target, visiting),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| value_depends_on(fn_ir, *arg, target, visiting)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(arg, _)| value_depends_on(fn_ir, *arg, target, visiting)),
        ValueKind::Index1D { base, idx, .. } => {
            value_depends_on(fn_ir, *base, target, visiting)
                || value_depends_on(fn_ir, *idx, target, visiting)
        }
        ValueKind::Index2D { base, r, c } => {
            value_depends_on(fn_ir, *base, target, visiting)
                || value_depends_on(fn_ir, *r, target, visiting)
                || value_depends_on(fn_ir, *c, target, visiting)
        }
        ValueKind::Index3D { base, i, j, k } => {
            value_depends_on(fn_ir, *base, target, visiting)
                || value_depends_on(fn_ir, *i, target, visiting)
                || value_depends_on(fn_ir, *j, target, visiting)
                || value_depends_on(fn_ir, *k, target, visiting)
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            value_depends_on(fn_ir, *base, target, visiting)
        }
        ValueKind::Range { start, end } => {
            value_depends_on(fn_ir, *start, target, visiting)
                || value_depends_on(fn_ir, *end, target, visiting)
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
    };
    visiting.remove(&root);
    out
}

pub(super) fn block_instr_uses_value(fn_ir: &FnIR, ins: &Instr, vid: ValueId) -> bool {
    let uses = |value: ValueId| value_depends_on(fn_ir, value, vid, &mut FxHashSet::default());
    match ins {
        Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => uses(*src),
        Instr::StoreIndex1D { base, idx, val, .. } => uses(*base) || uses(*idx) || uses(*val),
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => uses(*base) || uses(*r) || uses(*c) || uses(*val),
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => uses(*base) || uses(*i) || uses(*j) || uses(*k) || uses(*val),
    }
}

pub(super) fn block_term_uses_value(fn_ir: &FnIR, term: &Terminator, vid: ValueId) -> bool {
    let uses = |value: ValueId| value_depends_on(fn_ir, value, vid, &mut FxHashSet::default());
    match term {
        Terminator::If { cond, .. } => uses(*cond),
        Terminator::Return(Some(ret)) => uses(*ret),
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => false,
    }
}

pub(super) fn last_effective_assign_before_value_use_in_block(
    fn_ir: &FnIR,
    bid: BlockId,
    var: &str,
    vid: ValueId,
) -> Option<ValueId> {
    let vid = canonical_value(fn_ir, vid);
    let block = fn_ir.blocks.get(bid)?;
    for (idx, ins) in block.instrs.iter().enumerate() {
        if !block_instr_uses_value(fn_ir, ins, vid) {
            continue;
        }
        for prev in block.instrs[..idx].iter().rev() {
            let Instr::Assign { dst, src, .. } = prev else {
                continue;
            };
            if dst != var {
                continue;
            }
            if is_passthrough_load_of_var(fn_ir, *src, var) {
                continue;
            }
            return Some(canonical_value(fn_ir, *src));
        }
        return None;
    }
    if !block_term_uses_value(fn_ir, &block.term, vid) {
        return None;
    }
    for prev in block.instrs.iter().rev() {
        let Instr::Assign { dst, src, .. } = prev else {
            continue;
        };
        if dst != var {
            continue;
        }
        if is_passthrough_load_of_var(fn_ir, *src, var) {
            continue;
        }
        return Some(canonical_value(fn_ir, *src));
    }
    None
}

pub(super) fn intrinsic_for_call(callee: &str, arity: usize) -> Option<IntrinsicOp> {
    match (normalize_callee_name(callee), arity) {
        ("abs", 1) => Some(IntrinsicOp::VecAbsF64),
        ("log", 1) => Some(IntrinsicOp::VecLogF64),
        ("sqrt", 1) => Some(IntrinsicOp::VecSqrtF64),
        ("pmax", 2) => Some(IntrinsicOp::VecPmaxF64),
        ("pmin", 2) => Some(IntrinsicOp::VecPminF64),
        ("sum", 1) => Some(IntrinsicOp::VecSumF64),
        ("mean", 1) => Some(IntrinsicOp::VecMeanF64),
        _ => None,
    }
}

pub(super) fn call_map_profit_guard_supported(callee: &str, arity: usize) -> bool {
    let callee = normalize_callee_name(callee);
    is_builtin_vector_safe_call(callee, arity) && !matches!(callee, "seq_len" | "sum" | "mean")
}

pub(super) fn intrinsic_runtime_helper_cost(op: IntrinsicOp) -> u32 {
    match op {
        IntrinsicOp::VecAddF64
        | IntrinsicOp::VecSubF64
        | IntrinsicOp::VecMulF64
        | IntrinsicOp::VecDivF64
        | IntrinsicOp::VecAbsF64
        | IntrinsicOp::VecLogF64
        | IntrinsicOp::VecSqrtF64 => 1,
        IntrinsicOp::VecPmaxF64 | IntrinsicOp::VecPminF64 => 2,
        IntrinsicOp::VecSumF64 | IntrinsicOp::VecMeanF64 => 3,
    }
}

pub(super) fn call_runtime_helper_cost(callee: &str) -> u32 {
    match normalize_callee_name(callee) {
        "rr_idx_cube_vec_i" | "rr_wrap_index_vec" | "rr_wrap_index_vec_i" => 8,
        "rr_array3_gather_values" => 9,
        "rr_gather" | "rr_index1_read_vec" | "rr_index1_read_vec_floor" => 7,
        "rr_assign_index_vec" => 7,
        "rr_assign_slice" | "rr_ifelse_strict" => 6,
        "rr_same_len" | "rr_same_or_scalar" | "rr_index_vec_floor" => 4,
        "pmax" | "pmin" | "atan2" | "round" => 3,
        "abs" | "sqrt" | "exp" | "log" | "log10" | "log2" | "sin" | "cos" | "tan" | "asin"
        | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "sign" | "floor" | "ceiling" | "trunc"
        | "gamma" | "lgamma" | "is.na" | "is.finite" => 1,
        _ => 5,
    }
}

pub(super) fn expr_runtime_helper_cost(
    fn_ir: &FnIR,
    value: ValueId,
    seen: &mut FxHashSet<ValueId>,
) -> u32 {
    let root = canonical_value(fn_ir, value);
    if !seen.insert(root) {
        return 0;
    }
    let cost = match &fn_ir.values[root].kind {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => 0,
        ValueKind::RSymbol { .. } => 1,
        ValueKind::RecordLit { fields } => {
            1 + fields
                .iter()
                .map(|(_, value)| expr_runtime_helper_cost(fn_ir, *value, seen))
                .sum::<u32>()
        }
        ValueKind::FieldGet { base, .. } => 2 + expr_runtime_helper_cost(fn_ir, *base, seen),
        ValueKind::FieldSet { base, value, .. } => {
            3 + expr_runtime_helper_cost(fn_ir, *base, seen)
                + expr_runtime_helper_cost(fn_ir, *value, seen)
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            1 + expr_runtime_helper_cost(fn_ir, *base, seen)
        }
        ValueKind::Range { start, end } => {
            1 + expr_runtime_helper_cost(fn_ir, *start, seen)
                + expr_runtime_helper_cost(fn_ir, *end, seen)
        }
        ValueKind::Unary { rhs, .. } => 1 + expr_runtime_helper_cost(fn_ir, *rhs, seen),
        ValueKind::Binary { lhs, rhs, .. } => {
            1 + expr_runtime_helper_cost(fn_ir, *lhs, seen)
                + expr_runtime_helper_cost(fn_ir, *rhs, seen)
        }
        ValueKind::Phi { args } => {
            1 + args
                .iter()
                .map(|(arg, _)| expr_runtime_helper_cost(fn_ir, *arg, seen))
                .max()
                .unwrap_or(0)
        }
        ValueKind::Call { args, .. } => {
            let resolved = resolve_call_info(fn_ir, root);
            let call_args = resolved
                .as_ref()
                .map(|call| call.args.as_slice())
                .unwrap_or(args.as_slice());
            let callee = resolved
                .as_ref()
                .map(|call| call.callee.as_str())
                .unwrap_or("rr_call_closure");
            call_runtime_helper_cost(callee)
                + call_args
                    .iter()
                    .map(|arg| expr_runtime_helper_cost(fn_ir, *arg, seen))
                    .sum::<u32>()
        }
        ValueKind::Intrinsic { op, args } => {
            intrinsic_runtime_helper_cost(*op)
                + args
                    .iter()
                    .map(|arg| expr_runtime_helper_cost(fn_ir, *arg, seen))
                    .sum::<u32>()
        }
        ValueKind::Index1D { base, idx, .. } => {
            4 + expr_runtime_helper_cost(fn_ir, *base, seen)
                + expr_runtime_helper_cost(fn_ir, *idx, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            6 + expr_runtime_helper_cost(fn_ir, *base, seen)
                + expr_runtime_helper_cost(fn_ir, *r, seen)
                + expr_runtime_helper_cost(fn_ir, *c, seen)
        }
        ValueKind::Index3D { base, i, j, k } => {
            8 + expr_runtime_helper_cost(fn_ir, *base, seen)
                + expr_runtime_helper_cost(fn_ir, *i, seen)
                + expr_runtime_helper_cost(fn_ir, *j, seen)
                + expr_runtime_helper_cost(fn_ir, *k, seen)
        }
    };
    seen.remove(&root);
    cost
}

pub(super) fn estimate_call_map_helper_cost(
    fn_ir: &FnIR,
    callee: &str,
    args: &[CallMapArg],
    whole_dest: bool,
    shadow_vars: &[VarId],
) -> u32 {
    let arg_cost = args
        .iter()
        .filter(|arg| arg.vectorized)
        .map(|arg| expr_runtime_helper_cost(fn_ir, arg.value, &mut FxHashSet::default()))
        .sum::<u32>();
    let vector_arg_count = args.iter().filter(|arg| arg.vectorized).count() as u32;
    let partial_penalty = if whole_dest { 0 } else { 2 };
    let shadow_penalty = shadow_vars.len().min(u8::MAX as usize) as u32;
    call_runtime_helper_cost(callee)
        + arg_cost
        + vector_arg_count
        + partial_penalty
        + shadow_penalty
}

pub(super) fn choose_call_map_lowering(
    fn_ir: &FnIR,
    callee: &str,
    args: &[CallMapArg],
    whole_dest: bool,
    shadow_vars: &[VarId],
) -> CallMapLoweringMode {
    if !call_map_profit_guard_supported(callee, args.len()) {
        return CallMapLoweringMode::DirectVector;
    }
    let helper_cost = estimate_call_map_helper_cost(fn_ir, callee, args, whole_dest, shadow_vars);
    if whole_dest
        && shadow_vars.is_empty()
        && is_builtin_vector_safe_call(normalize_callee_name(callee), args.len())
        && helper_cost < CALL_MAP_AUTO_HELPER_COST_THRESHOLD
    {
        return CallMapLoweringMode::DirectVector;
    }
    if helper_cost >= CALL_MAP_AUTO_HELPER_COST_THRESHOLD {
        CallMapLoweringMode::RuntimeAuto { helper_cost }
    } else {
        CallMapLoweringMode::DirectVector
    }
}

pub(super) fn const_index_like_value(fn_ir: &FnIR, vid: ValueId) -> Option<i64> {
    match &fn_ir.values[canonical_value(fn_ir, vid)].kind {
        ValueKind::Const(Lit::Int(n)) => Some(*n),
        ValueKind::Const(Lit::Float(f)) if f.is_finite() && f.fract() == 0.0 => Some(*f as i64),
        _ => None,
    }
}

pub(super) fn estimate_loop_trip_count_hint(fn_ir: &FnIR, lp: &LoopInfo) -> Option<u64> {
    let iv = lp.iv.as_ref()?;
    let start = const_index_like_value(fn_ir, iv.init_val)?;
    let limit = const_index_like_value(fn_ir, lp.limit?)?.checked_add(lp.limit_adjust)?;
    if iv.step == 0 {
        return None;
    }
    if iv.step > 0 {
        if limit < start {
            return Some(0);
        }
        let span = (limit - start).checked_add(1)?;
        u64::try_from(span).ok()
    } else {
        if limit > start {
            return Some(0);
        }
        let span = (start - limit).checked_add(1)?;
        u64::try_from(span).ok()
    }
}

pub(super) fn estimate_vector_plan_helper_cost(fn_ir: &FnIR, plan: &VectorPlan) -> u32 {
    match plan {
        VectorPlan::Reduce { vec_expr, .. } => {
            expr_runtime_helper_cost(fn_ir, *vec_expr, &mut FxHashSet::default())
        }
        VectorPlan::ReduceCond {
            cond,
            then_val,
            else_val,
            ..
        } => {
            6 + expr_runtime_helper_cost(fn_ir, *cond, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *then_val, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *else_val, &mut FxHashSet::default())
        }
        VectorPlan::MultiReduceCond { cond, entries, .. } => {
            6 + expr_runtime_helper_cost(fn_ir, *cond, &mut FxHashSet::default())
                + entries.iter().fold(0, |acc, entry| {
                    acc + expr_runtime_helper_cost(fn_ir, entry.then_val, &mut FxHashSet::default())
                        + expr_runtime_helper_cost(fn_ir, entry.else_val, &mut FxHashSet::default())
                })
        }
        VectorPlan::Reduce2DRowSum { .. } | VectorPlan::Reduce2DColSum { .. } => 2,
        VectorPlan::Reduce3D { .. } => 3,
        VectorPlan::Map {
            src,
            other,
            shadow_vars,
            ..
        } => {
            expr_runtime_helper_cost(fn_ir, *src, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *other, &mut FxHashSet::default())
                + shadow_vars.len() as u32
        }
        VectorPlan::CondMap {
            cond,
            then_val,
            else_val,
            whole_dest,
            shadow_vars,
            ..
        } => {
            6 + expr_runtime_helper_cost(fn_ir, *cond, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *then_val, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *else_val, &mut FxHashSet::default())
                + u32::from(!*whole_dest) * 2
                + shadow_vars.len() as u32
        }
        VectorPlan::CondMap3D {
            cond_lhs,
            cond_rhs,
            then_src,
            else_src,
            ..
        } => {
            8 + expr_runtime_helper_cost(fn_ir, *cond_lhs, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *cond_rhs, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *then_src, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *else_src, &mut FxHashSet::default())
        }
        VectorPlan::CondMap3DGeneral {
            cond_lhs,
            cond_rhs,
            then_val,
            else_val,
            ..
        } => {
            10 + expr_runtime_helper_cost(fn_ir, *cond_lhs, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *cond_rhs, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *then_val, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *else_val, &mut FxHashSet::default())
        }
        VectorPlan::RecurrenceAddConst3D { delta, .. } => {
            6 + expr_runtime_helper_cost(fn_ir, *delta, &mut FxHashSet::default())
        }
        VectorPlan::ShiftedMap3D { .. } => 6,
        VectorPlan::RecurrenceAddConst { .. } => 2,
        VectorPlan::ShiftedMap { src, .. } => {
            4 + expr_runtime_helper_cost(fn_ir, *src, &mut FxHashSet::default())
        }
        VectorPlan::CallMap {
            callee,
            args,
            whole_dest,
            shadow_vars,
            ..
        } => estimate_call_map_helper_cost(fn_ir, callee, args, *whole_dest, shadow_vars),
        VectorPlan::CallMap3D { args, .. } => {
            8 + args.iter().fold(0, |acc, arg| {
                acc + expr_runtime_helper_cost(fn_ir, arg.value, &mut FxHashSet::default())
                    + u32::from(arg.vectorized)
            })
        }
        VectorPlan::CallMap3DGeneral { args, .. } => {
            10 + args.iter().fold(0, |acc, arg| {
                acc + expr_runtime_helper_cost(fn_ir, arg.value, &mut FxHashSet::default())
                    + u32::from(arg.vectorized)
            })
        }
        VectorPlan::CubeSliceExprMap {
            expr, shadow_vars, ..
        } => {
            8 + expr_runtime_helper_cost(fn_ir, *expr, &mut FxHashSet::default())
                + shadow_vars.len() as u32
        }
        VectorPlan::ExprMap {
            expr,
            whole_dest,
            shadow_vars,
            ..
        } => {
            expr_runtime_helper_cost(fn_ir, *expr, &mut FxHashSet::default())
                + u32::from(!*whole_dest) * 2
                + shadow_vars.len() as u32
        }
        VectorPlan::ExprMap3D { expr, .. } => {
            8 + expr_runtime_helper_cost(fn_ir, *expr, &mut FxHashSet::default())
        }
        VectorPlan::MultiExprMap3D { entries, .. } => entries.iter().fold(0, |acc, entry| {
            acc + 8 + expr_runtime_helper_cost(fn_ir, entry.expr, &mut FxHashSet::default())
        }),
        VectorPlan::MultiExprMap { entries, .. } => entries.iter().fold(0, |acc, entry| {
            acc + expr_runtime_helper_cost(fn_ir, entry.expr, &mut FxHashSet::default())
                + u32::from(!entry.whole_dest) * 2
                + entry.shadow_vars.len() as u32
        }),
        VectorPlan::ScatterExprMap { idx, expr, .. } => {
            8 + expr_runtime_helper_cost(fn_ir, *idx, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *expr, &mut FxHashSet::default())
        }
        VectorPlan::ScatterExprMap3D { idx, expr, .. } => {
            10 + expr_runtime_helper_cost(fn_ir, *idx, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *expr, &mut FxHashSet::default())
        }
        VectorPlan::ScatterExprMap3DGeneral { i, j, k, expr, .. } => {
            let idx_cost = [i, j, k].into_iter().fold(0, |acc, operand| {
                acc + match operand {
                    VectorAccessOperand3D::Scalar(_) => 0,
                    VectorAccessOperand3D::Vector(value) => {
                        expr_runtime_helper_cost(fn_ir, *value, &mut FxHashSet::default())
                    }
                }
            });
            12 + idx_cost + expr_runtime_helper_cost(fn_ir, *expr, &mut FxHashSet::default())
        }
        VectorPlan::Map2DRow {
            lhs_src, rhs_src, ..
        }
        | VectorPlan::Map2DCol {
            lhs_src, rhs_src, ..
        }
        | VectorPlan::Map3D {
            lhs_src, rhs_src, ..
        } => {
            6 + expr_runtime_helper_cost(fn_ir, *lhs_src, &mut FxHashSet::default())
                + expr_runtime_helper_cost(fn_ir, *rhs_src, &mut FxHashSet::default())
        }
    }
}

pub(super) fn is_vector_safe_call(
    callee: &str,
    arity: usize,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    let callee = normalize_callee_name(callee);
    is_builtin_vector_safe_call(callee, arity)
        || user_call_whitelist.contains(callee)
        || user_call_whitelist.contains(&format!("base::{callee}"))
}

pub(super) fn is_const_number(fn_ir: &FnIR, vid: ValueId) -> bool {
    matches!(
        fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(Lit::Int(_)) | ValueKind::Const(Lit::Float(_))
    )
}

pub(super) fn is_const_one(fn_ir: &FnIR, vid: ValueId) -> bool {
    match fn_ir.values[canonical_value(fn_ir, vid)].kind {
        ValueKind::Const(Lit::Int(n)) => n == 1,
        ValueKind::Const(Lit::Float(f)) => (f - 1.0).abs() < f64::EPSILON,
        _ => false,
    }
}

pub(super) fn is_invariant_reduce_scalar(
    fn_ir: &FnIR,
    scalar: ValueId,
    iv_phi: ValueId,
    base: ValueId,
) -> bool {
    if expr_has_iv_dependency(fn_ir, scalar, iv_phi) || expr_reads_base(fn_ir, scalar, base) {
        return false;
    }
    match &fn_ir.values[canonical_value(fn_ir, scalar)].kind {
        ValueKind::Const(Lit::Int(_))
        | ValueKind::Const(Lit::Float(_))
        | ValueKind::Param { .. } => true,
        ValueKind::Load { var } => {
            if let Some(base_var) = resolve_base_var(fn_ir, base) {
                var != &base_var
            } else {
                true
            }
        }
        _ => false,
    }
}

pub(super) fn is_loop_invariant_scalar_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        user_call_whitelist: &FxHashSet<String>,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return true;
        }
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) => true,
            ValueKind::Load { .. } | ValueKind::Param { .. } => {
                vector_length_key(fn_ir, root).is_none()
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, iv_phi, user_call_whitelist, seen),
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, user_call_whitelist, seen)
                    && rec(fn_ir, *rhs, iv_phi, user_call_whitelist, seen)
            }
            ValueKind::Len { base } => rec(fn_ir, *base, iv_phi, user_call_whitelist, seen),
            ValueKind::Call { args, .. } => {
                let resolved = resolve_call_info(fn_ir, root);
                let callee = resolved
                    .as_ref()
                    .map(|call| call.callee.as_str())
                    .unwrap_or("rr_call_closure");
                let call_args = resolved
                    .as_ref()
                    .map(|call| call.args.as_slice())
                    .unwrap_or(args.as_slice());
                is_vector_safe_call(callee, call_args.len(), user_call_whitelist)
                    && call_args
                        .iter()
                        .all(|a| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|a| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen)),
            ValueKind::RecordLit { fields } => fields
                .iter()
                .all(|(_, value)| rec(fn_ir, *value, iv_phi, user_call_whitelist, seen)),
            ValueKind::FieldGet { .. } | ValueKind::FieldSet { .. } => false,
            ValueKind::Phi { args } => args
                .iter()
                .all(|(a, _)| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen)),
            ValueKind::Index1D { .. }
            | ValueKind::Index2D { .. }
            | ValueKind::Index3D { .. }
            | ValueKind::Range { .. }
            | ValueKind::Indices { .. }
            | ValueKind::RSymbol { .. } => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    )
}

pub(super) fn is_vector_safe_call_chain_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        _lp: &LoopInfo,
        user_call_whitelist: &FxHashSet<String>,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            return true;
        }
        if !seen.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. } => true,
            ValueKind::Unary { rhs, .. } => {
                rec(fn_ir, *rhs, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, _lp, user_call_whitelist, seen)
                    && rec(fn_ir, *rhs, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Call { args, .. } => {
                let resolved = resolve_call_info(fn_ir, root);
                let callee = resolved
                    .as_ref()
                    .map(|call| call.callee.as_str())
                    .unwrap_or("rr_call_closure");
                let call_args = resolved
                    .as_ref()
                    .map(|call| call.args.as_slice())
                    .unwrap_or(args.as_slice());
                is_vector_safe_call(callee, call_args.len(), user_call_whitelist)
                    && call_args
                        .iter()
                        .all(|a| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|a| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen)),
            ValueKind::RecordLit { fields } => fields
                .iter()
                .all(|(_, value)| rec(fn_ir, *value, iv_phi, _lp, user_call_whitelist, seen)),
            ValueKind::FieldGet { .. } | ValueKind::FieldSet { .. } => false,
            ValueKind::Index1D {
                base: _base,
                idx,
                is_safe: _is_safe,
                is_na_safe: _is_na_safe,
            } => is_iv_equivalent(fn_ir, *idx, iv_phi),
            ValueKind::Phi { args } => args
                .iter()
                .all(|(a, _)| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, *base, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, iv_phi, _lp, user_call_whitelist, seen)
                    && rec(fn_ir, *end, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Index2D { .. } | ValueKind::Index3D { .. } | ValueKind::RSymbol { .. } => {
                false
            }
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        user_call_whitelist,
        &mut FxHashSet::default(),
    )
}

pub(super) fn match_2d_row_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let start = iv.init_val;
    let end = lp.limit?;

    if !loop_is_simple_2d_map(fn_ir, lp) {
        return None;
    }

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest, row, col, rhs) = match instr {
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => (*base, *r, *c, *val),
                _ => continue,
            };
            if !is_loop_invariant_axis(fn_ir, row, iv_phi, dest) {
                continue;
            }
            if !is_iv_equivalent(fn_ir, col, iv_phi) || expr_has_iv_dependency(fn_ir, row, iv_phi) {
                continue;
            }

            let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[rhs].kind.clone() else {
                continue;
            };
            if !matches!(
                op,
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
            ) {
                continue;
            }
            let lhs_src = match row_operand_source(fn_ir, lhs, row, iv_phi) {
                Some(v) => v,
                None => continue,
            };
            let rhs_src = match row_operand_source(fn_ir, rhs, row, iv_phi) {
                Some(v) => v,
                None => continue,
            };

            return Some(VectorPlan::Map2DRow {
                dest: canonical_value(fn_ir, dest),
                row,
                start,
                end,
                lhs_src,
                rhs_src,
                op,
            });
        }
    }
    None
}

pub(super) fn match_2d_col_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let start = iv.init_val;
    let end = lp.limit?;

    if !loop_is_simple_2d_map(fn_ir, lp) {
        return None;
    }

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest, row, col, rhs) = match instr {
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => (*base, *r, *c, *val),
                _ => continue,
            };
            if !is_loop_invariant_axis(fn_ir, col, iv_phi, dest) {
                continue;
            }
            if !is_iv_equivalent(fn_ir, row, iv_phi) || expr_has_iv_dependency(fn_ir, col, iv_phi) {
                continue;
            }

            let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[rhs].kind.clone() else {
                continue;
            };
            if !matches!(
                op,
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
            ) {
                continue;
            }
            let lhs_src = match col_operand_source(fn_ir, lhs, col, iv_phi) {
                Some(v) => v,
                None => continue,
            };
            let rhs_src = match col_operand_source(fn_ir, rhs, col, iv_phi) {
                Some(v) => v,
                None => continue,
            };

            return Some(VectorPlan::Map2DCol {
                dest: canonical_value(fn_ir, dest),
                col,
                start,
                end,
                lhs_src,
                rhs_src,
                op,
            });
        }
    }
    None
}

pub(super) fn match_3d_axis_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let start = iv.init_val;
    let end = lp.limit?;

    if !loop_is_simple_3d_map(fn_ir, lp) {
        return None;
    }

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest, i, j, k, rhs) = match instr {
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => (*base, *i, *j, *k, *val),
                _ => continue,
            };
            let Some((axis, fixed_a, fixed_b)) = classify_3d_map_axis(fn_ir, dest, i, j, k, iv_phi)
            else {
                continue;
            };

            let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[rhs].kind.clone() else {
                continue;
            };
            if !matches!(
                op,
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
            ) {
                continue;
            }
            let lhs_src = match axis3_operand_source(fn_ir, lhs, axis, fixed_a, fixed_b, iv_phi) {
                Some(v) => v,
                None => continue,
            };
            let rhs_src = match axis3_operand_source(fn_ir, rhs, axis, fixed_a, fixed_b, iv_phi) {
                Some(v) => v,
                None => continue,
            };

            return Some(VectorPlan::Map3D {
                dest: canonical_value(fn_ir, dest),
                axis,
                fixed_a,
                fixed_b,
                start,
                end,
                lhs_src,
                rhs_src,
                op,
            });
        }
    }
    None
}

pub(super) fn loop_is_simple_2d_map(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let mut store2d_count = 0usize;
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::StoreIndex2D { .. } => store2d_count += 1,
                Instr::StoreIndex1D { .. } | Instr::StoreIndex3D { .. } | Instr::Eval { .. } => {
                    return false;
                }
                Instr::Assign { .. } => {}
            }
        }
    }
    if store2d_count != 1 {
        return false;
    }
    true
}

pub(super) fn loop_is_simple_3d_map(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let mut store3d_count = 0usize;
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::StoreIndex3D { .. } => store3d_count += 1,
                Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } | Instr::Eval { .. } => {
                    return false;
                }
                Instr::Assign { .. } => {}
            }
        }
    }
    store3d_count == 1
}

pub(super) fn classify_3d_map_axis(
    fn_ir: &FnIR,
    dest: ValueId,
    i: ValueId,
    j: ValueId,
    k: ValueId,
    iv_phi: ValueId,
) -> Option<(Axis3D, ValueId, ValueId)> {
    if is_iv_equivalent(fn_ir, i, iv_phi)
        && is_loop_invariant_axis(fn_ir, j, iv_phi, dest)
        && is_loop_invariant_axis(fn_ir, k, iv_phi, dest)
    {
        return Some((Axis3D::Dim1, j, k));
    }
    if is_iv_equivalent(fn_ir, j, iv_phi)
        && is_loop_invariant_axis(fn_ir, i, iv_phi, dest)
        && is_loop_invariant_axis(fn_ir, k, iv_phi, dest)
    {
        return Some((Axis3D::Dim2, i, k));
    }
    if is_iv_equivalent(fn_ir, k, iv_phi)
        && is_loop_invariant_axis(fn_ir, i, iv_phi, dest)
        && is_loop_invariant_axis(fn_ir, j, iv_phi, dest)
    {
        return Some((Axis3D::Dim3, i, j));
    }
    None
}

pub(super) fn classify_3d_vector_access_axis(
    fn_ir: &FnIR,
    base: ValueId,
    i: ValueId,
    j: ValueId,
    k: ValueId,
    iv_phi: ValueId,
) -> Option<(Axis3D, ValueId, ValueId, ValueId)> {
    let dep_i = is_iv_equivalent(fn_ir, i, iv_phi) || expr_has_iv_dependency(fn_ir, i, iv_phi);
    let dep_j = is_iv_equivalent(fn_ir, j, iv_phi) || expr_has_iv_dependency(fn_ir, j, iv_phi);
    let dep_k = is_iv_equivalent(fn_ir, k, iv_phi) || expr_has_iv_dependency(fn_ir, k, iv_phi);

    if dep_i
        && !dep_j
        && !dep_k
        && is_loop_invariant_axis(fn_ir, j, iv_phi, base)
        && is_loop_invariant_axis(fn_ir, k, iv_phi, base)
    {
        return Some((Axis3D::Dim1, i, j, k));
    }
    if dep_j
        && !dep_i
        && !dep_k
        && is_loop_invariant_axis(fn_ir, i, iv_phi, base)
        && is_loop_invariant_axis(fn_ir, k, iv_phi, base)
    {
        return Some((Axis3D::Dim2, j, i, k));
    }
    if dep_k
        && !dep_i
        && !dep_j
        && is_loop_invariant_axis(fn_ir, i, iv_phi, base)
        && is_loop_invariant_axis(fn_ir, j, iv_phi, base)
    {
        return Some((Axis3D::Dim3, k, i, j));
    }
    None
}

pub(super) fn classify_3d_general_vector_access(
    fn_ir: &FnIR,
    base: ValueId,
    i: ValueId,
    j: ValueId,
    k: ValueId,
    iv_phi: ValueId,
) -> Option<VectorAccessPattern3D> {
    fn classify_operand(
        fn_ir: &FnIR,
        base: ValueId,
        operand: ValueId,
        iv_phi: ValueId,
    ) -> Option<VectorAccessOperand3D> {
        if is_iv_equivalent(fn_ir, operand, iv_phi)
            || expr_has_iv_dependency(fn_ir, operand, iv_phi)
        {
            return Some(VectorAccessOperand3D::Vector(operand));
        }
        if is_loop_invariant_axis(fn_ir, operand, iv_phi, base)
            || is_loop_invariant_scalar_expr(fn_ir, operand, iv_phi, &FxHashSet::default())
        {
            return Some(VectorAccessOperand3D::Scalar(operand));
        }
        None
    }

    let pattern = VectorAccessPattern3D {
        i: classify_operand(fn_ir, base, i, iv_phi)?,
        j: classify_operand(fn_ir, base, j, iv_phi)?,
        k: classify_operand(fn_ir, base, k, iv_phi)?,
    };
    (pattern.vector_count() >= 1).then_some(pattern)
}

pub(super) fn row_operand_source(
    fn_ir: &FnIR,
    operand: ValueId,
    row: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    match &fn_ir.values[operand].kind {
        ValueKind::Index2D { base, r, c } => {
            if is_iv_equivalent(fn_ir, *c, iv_phi)
                && same_loop_invariant_value(fn_ir, *r, row, iv_phi)
            {
                Some(canonical_value(fn_ir, *base))
            } else {
                None
            }
        }
        ValueKind::Index3D { .. } => None,
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => Some(operand),
        _ => None,
    }
}

pub(super) fn axis3_operand_source(
    fn_ir: &FnIR,
    operand: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    match &fn_ir.values[operand].kind {
        ValueKind::Index3D { base, i, j, k } => {
            let matches = match axis {
                Axis3D::Dim1 => {
                    is_iv_equivalent(fn_ir, *i, iv_phi)
                        && same_loop_invariant_value(fn_ir, *j, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *k, fixed_b, iv_phi)
                }
                Axis3D::Dim2 => {
                    is_iv_equivalent(fn_ir, *j, iv_phi)
                        && same_loop_invariant_value(fn_ir, *i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *k, fixed_b, iv_phi)
                }
                Axis3D::Dim3 => {
                    is_iv_equivalent(fn_ir, *k, iv_phi)
                        && same_loop_invariant_value(fn_ir, *i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *j, fixed_b, iv_phi)
                }
            };
            if matches {
                Some(canonical_value(fn_ir, *base))
            } else {
                None
            }
        }
        _ if is_loop_invariant_scalar_expr(fn_ir, operand, iv_phi, &FxHashSet::default()) => {
            Some(operand)
        }
        _ => None,
    }
}

pub(super) fn axis3_vector_operand_source(
    fn_ir: &FnIR,
    operand: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    match &fn_ir.values[operand].kind {
        ValueKind::Index3D { base, i, j, k } => {
            let matches = match axis {
                Axis3D::Dim1 => {
                    is_iv_equivalent(fn_ir, *i, iv_phi)
                        && same_loop_invariant_value(fn_ir, *j, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *k, fixed_b, iv_phi)
                }
                Axis3D::Dim2 => {
                    is_iv_equivalent(fn_ir, *j, iv_phi)
                        && same_loop_invariant_value(fn_ir, *i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *k, fixed_b, iv_phi)
                }
                Axis3D::Dim3 => {
                    is_iv_equivalent(fn_ir, *k, iv_phi)
                        && same_loop_invariant_value(fn_ir, *i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *j, fixed_b, iv_phi)
                }
            };
            matches.then_some(canonical_value(fn_ir, *base))
        }
        _ => None,
    }
}

pub(super) fn expr_contains_index3d(fn_ir: &FnIR, root: ValueId) -> bool {
    fn rec(fn_ir: &FnIR, root: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Index3D { .. } => true,
            ValueKind::Binary { lhs, rhs, .. } => rec(fn_ir, *lhs, seen) || rec(fn_ir, *rhs, seen),
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, seen),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                args.iter().any(|arg| rec(fn_ir, *arg, seen))
            }
            ValueKind::RecordLit { fields } => {
                fields.iter().any(|(_, value)| rec(fn_ir, *value, seen))
            }
            ValueKind::FieldGet { base, .. } => rec(fn_ir, *base, seen),
            ValueKind::FieldSet { base, value, .. } => {
                rec(fn_ir, *base, seen) || rec(fn_ir, *value, seen)
            }
            ValueKind::Phi { args } => args.iter().any(|(arg, _)| rec(fn_ir, *arg, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => rec(fn_ir, *base, seen),
            ValueKind::Range { start, end } => rec(fn_ir, *start, seen) || rec(fn_ir, *end, seen),
            ValueKind::Index1D { base, idx, .. } => {
                rec(fn_ir, *base, seen) || rec(fn_ir, *idx, seen)
            }
            ValueKind::Index2D { base, r, c } => {
                rec(fn_ir, *base, seen) || rec(fn_ir, *r, seen) || rec(fn_ir, *c, seen)
            }
            ValueKind::Const(_)
            | ValueKind::Load { .. }
            | ValueKind::Param { .. }
            | ValueKind::RSymbol { .. } => false,
        }
    }
    rec(fn_ir, root, &mut FxHashSet::default())
}

pub(super) fn col_operand_source(
    fn_ir: &FnIR,
    operand: ValueId,
    col: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    match &fn_ir.values[operand].kind {
        ValueKind::Index2D { base, r, c } => {
            if is_iv_equivalent(fn_ir, *r, iv_phi)
                && same_loop_invariant_value(fn_ir, *c, col, iv_phi)
            {
                Some(canonical_value(fn_ir, *base))
            } else {
                None
            }
        }
        ValueKind::Index3D { .. } => None,
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => Some(operand),
        _ => None,
    }
}

pub(super) fn same_loop_invariant_value(
    fn_ir: &FnIR,
    a: ValueId,
    b: ValueId,
    iv_phi: ValueId,
) -> bool {
    if is_value_equivalent(fn_ir, a, b) {
        return true;
    }
    if expr_has_iv_dependency(fn_ir, a, iv_phi) || expr_has_iv_dependency(fn_ir, b, iv_phi) {
        return false;
    }
    if let (ValueKind::Const(ca), ValueKind::Const(cb)) =
        (&fn_ir.values[a].kind, &fn_ir.values[b].kind)
    {
        return ca == cb;
    }
    match (
        fn_ir.values[a].origin_var.as_deref(),
        fn_ir.values[b].origin_var.as_deref(),
    ) {
        (Some(va), Some(vb)) => va == vb,
        _ => false,
    }
}

pub(super) fn is_origin_var_iv_alias_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    candidate: ValueId,
    iv_phi: ValueId,
) -> bool {
    let candidate = canonical_value(fn_ir, candidate);
    let Some(origin_var) = fn_ir.values[candidate].origin_var.as_deref() else {
        return false;
    };
    let iv_origin = induction_origin_var(fn_ir, iv_phi);
    let matches_iv = |src: ValueId| {
        is_iv_equivalent(fn_ir, src, iv_phi)
            || is_floor_like_iv_expr(
                fn_ir,
                src,
                iv_phi,
                &mut FxHashSet::default(),
                &mut FxHashSet::default(),
                0,
            )
    };
    let matches_seed = |src: ValueId| {
        let src = canonical_value(fn_ir, src);
        matches_iv(src)
            || matches!(
                (&fn_ir.values[src].kind, iv_origin.as_deref()),
                (ValueKind::Load { var }, Some(iv_var)) if var == iv_var
            )
    };
    if let Some(src) = loop_entry_seed_source_in_loop(fn_ir, lp, origin_var)
        && matches_seed(src)
    {
        return true;
    }
    if let Some(src) = unique_assign_source_in_loop(fn_ir, lp, origin_var)
        && matches_seed(src)
    {
        return true;
    }
    let target_bb =
        value_use_block_in_loop(fn_ir, lp, candidate).or(fn_ir.values[candidate].phi_block);
    let Some(target_bb) = target_bb else {
        return false;
    };
    let Some(src) = unique_assign_source_reaching_block_in_loop(fn_ir, lp, origin_var, target_bb)
    else {
        return false;
    };
    !matches!(
        fn_ir.values[canonical_value(fn_ir, src)].kind,
        ValueKind::Phi { .. }
    ) && matches_seed(src)
}

pub(super) fn collapse_same_var_passthrough_phi_to_load(
    fn_ir: &FnIR,
    root: ValueId,
    var: &str,
    seen: &mut FxHashSet<ValueId>,
) -> Option<ValueId> {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return None;
    }
    let out = match &fn_ir.values[root].kind {
        ValueKind::Load { var: load_var } if load_var == var => Some(root),
        ValueKind::Phi { args }
            if !args.is_empty() && fn_ir.values[root].origin_var.as_deref() == Some(var) =>
        {
            let mut found: Option<ValueId> = None;
            for (arg, _) in args {
                let arg = canonical_value(fn_ir, *arg);
                let load = collapse_same_var_passthrough_phi_to_load(fn_ir, arg, var, seen)?;
                match found {
                    None => found = Some(load),
                    Some(prev) if canonical_value(fn_ir, prev) == canonical_value(fn_ir, load) => {}
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

pub(super) fn loop_entry_seed_source_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> Option<ValueId> {
    let mut seed: Option<ValueId> = None;
    let mut passthrough_seen = FxHashSet::default();
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            let src = canonical_value(fn_ir, *src);
            passthrough_seen.clear();
            if is_passthrough_load_of_var(fn_ir, src, var)
                || collapse_same_var_passthrough_phi_to_load(fn_ir, src, var, &mut passthrough_seen)
                    .is_some()
            {
                continue;
            }
            match seed {
                None => seed = Some(src),
                Some(prev) if canonical_value(fn_ir, prev) == src => {}
                Some(_) => return None,
            }
        }
    }
    seed
}

pub(super) fn has_any_var_binding(fn_ir: &FnIR, var: &str) -> bool {
    if fn_ir.params.iter().any(|p| p == var) {
        return true;
    }
    fn_ir.blocks.iter().any(|bb| {
        bb.instrs.iter().any(|ins| match ins {
            Instr::Assign { dst, .. } => dst == var,
            _ => false,
        })
    })
}

pub(super) fn is_loop_invariant_axis(
    fn_ir: &FnIR,
    axis: ValueId,
    iv_phi: ValueId,
    dest: ValueId,
) -> bool {
    if expr_has_iv_dependency(fn_ir, axis, iv_phi) {
        return false;
    }
    match &fn_ir.values[canonical_value(fn_ir, axis)].kind {
        ValueKind::Const(Lit::Int(_)) => true,
        ValueKind::Param { .. } => true,
        ValueKind::Load { var } => {
            if let Some(dest_var) = resolve_base_var(fn_ir, dest) {
                var != &dest_var && has_any_var_binding(fn_ir, var)
            } else {
                has_any_var_binding(fn_ir, var)
            }
        }
        _ => false,
    }
}

pub(super) fn as_safe_loop_index(fn_ir: &FnIR, vid: ValueId, iv_phi: ValueId) -> Option<ValueId> {
    if let ValueKind::Index1D {
        base,
        idx,
        is_safe,
        is_na_safe,
    } = &fn_ir.values[vid].kind
        && is_iv_equivalent(fn_ir, *idx, iv_phi)
        && *is_safe
        && *is_na_safe
    {
        return Some(canonical_value(fn_ir, *base));
    }
    None
}
pub(super) fn loop_has_store_effect(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            if matches!(
                instr,
                Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
            ) {
                return true;
            }
        }
    }
    false
}

pub(super) fn collect_loop_shadow_vars_for_dest(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    allowed_vars: &[VarId],
    dest_base: ValueId,
    iv_phi: ValueId,
) -> Option<Vec<VarId>> {
    let Some(_) = lp.iv.as_ref() else {
        return Some(Vec::new());
    };
    let mut shadow_vars = Vec::new();
    for value in &fn_ir.values {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) {
            continue;
        }
        let has_loop_pred = args.iter().any(|(_, pred)| lp.body.contains(pred));
        let has_outer_pred = args.iter().any(|(_, pred)| !lp.body.contains(pred));
        if !has_loop_pred || !has_outer_pred {
            continue;
        }
        if is_iv_equivalent(fn_ir, value.id, iv_phi)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, value.id, iv_phi)
        {
            continue;
        }
        if loop_carried_phi_is_unmodified(fn_ir, value.id) {
            continue;
        }
        if loop_carried_phi_is_invariant_passthrough(fn_ir, lp, value.id, iv_phi) {
            continue;
        }
        let Some(origin_var) = value.origin_var.as_deref() else {
            continue;
        };
        if !has_non_passthrough_assignment_in_loop(fn_ir, lp, origin_var) {
            continue;
        }
        if allowed_vars
            .iter()
            .any(|allowed| origin_var == allowed.as_str())
        {
            continue;
        }
        if let Some(shadow_base) =
            loop_carried_phi_last_value_shadow_base(fn_ir, lp, value.id, iv_phi)
        {
            if same_base_value(
                fn_ir,
                canonical_value(fn_ir, shadow_base),
                canonical_value(fn_ir, dest_base),
            ) {
                shadow_vars.push(origin_var.to_string());
                continue;
            }
            if resolve_base_var(fn_ir, shadow_base)
                .is_some_and(|var| allowed_vars.iter().any(|allowed| allowed == &var))
            {
                continue;
            }
        }
        if vectorize_trace_enabled() {
            let phi_args = match &value.kind {
                ValueKind::Phi { args } => args
                    .iter()
                    .map(|(arg, pred)| {
                        let arg = canonical_value(fn_ir, *arg);
                        format!(
                            "{}@{} kind={:?} origin={:?}",
                            arg, pred, fn_ir.values[arg].kind, fn_ir.values[arg].origin_var
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | "),
                _ => String::new(),
            };
            eprintln!(
                "   [vec-loop-state] {} reject: phi={} var={} bb={} non-destination-loop-state kind={:?} args=[{}]",
                fn_ir.name, value.id, origin_var, phi_bb, value.kind, phi_args
            );
        }
        return None;
    }
    shadow_vars.sort_unstable();
    shadow_vars.dedup();
    Some(shadow_vars)
}

pub(super) fn loop_carried_phi_is_dest_last_value_shadow(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    phi_vid: ValueId,
    dest_base: ValueId,
    iv_phi: ValueId,
) -> bool {
    loop_carried_phi_last_value_shadow_base(fn_ir, lp, phi_vid, iv_phi).is_some_and(|shadow_base| {
        same_base_value(
            fn_ir,
            canonical_value(fn_ir, shadow_base),
            canonical_value(fn_ir, dest_base),
        )
    })
}

pub(super) fn loop_carried_phi_last_value_shadow_base(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    phi_vid: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    let phi_vid = preserve_phi_value(fn_ir, phi_vid);
    let ValueKind::Phi { args } = &fn_ir.values[phi_vid].kind else {
        return None;
    };
    let mut loop_arg: Option<ValueId> = None;
    for (arg, pred) in args {
        if !lp.body.contains(pred) {
            continue;
        }
        let arg = resolve_load_alias_value(fn_ir, *arg);
        match loop_arg {
            None => loop_arg = Some(arg),
            Some(prev) if canonical_value(fn_ir, prev) == canonical_value(fn_ir, arg) => {}
            Some(_) => return None,
        }
    }
    let loop_arg = loop_arg?;
    match &fn_ir.values[canonical_value(fn_ir, loop_arg)].kind {
        ValueKind::Index1D { base, idx, .. } => {
            if is_iv_equivalent(fn_ir, *idx, iv_phi)
                || is_floor_like_iv_expr(
                    fn_ir,
                    *idx,
                    iv_phi,
                    &mut FxHashSet::default(),
                    &mut FxHashSet::default(),
                    0,
                )
            {
                Some(canonical_value(fn_ir, *base))
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn loop_has_non_iv_loop_carried_state_except(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    allowed_vars: &[VarId],
) -> bool {
    let Some(iv) = lp.iv.as_ref() else {
        return false;
    };
    let iv_phi = iv.phi_val;
    for value in &fn_ir.values {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) {
            continue;
        }
        let has_loop_pred = args.iter().any(|(_, pred)| lp.body.contains(pred));
        let has_outer_pred = args.iter().any(|(_, pred)| !lp.body.contains(pred));
        if !has_loop_pred || !has_outer_pred {
            continue;
        }
        if is_iv_equivalent(fn_ir, value.id, iv_phi)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, value.id, iv_phi)
        {
            continue;
        }
        if loop_carried_phi_is_unmodified(fn_ir, value.id) {
            continue;
        }
        if loop_carried_phi_is_invariant_passthrough(fn_ir, lp, value.id, iv_phi) {
            continue;
        }
        let Some(origin_var) = value.origin_var.as_deref() else {
            continue;
        };
        if !has_non_passthrough_assignment_in_loop(fn_ir, lp, origin_var) {
            continue;
        }
        if allowed_vars
            .iter()
            .any(|allowed| origin_var == allowed.as_str())
        {
            continue;
        }
        if vectorize_trace_enabled() {
            let phi_args = match &value.kind {
                ValueKind::Phi { args } => args
                    .iter()
                    .map(|(arg, pred)| {
                        let arg = canonical_value(fn_ir, *arg);
                        format!(
                            "{}@{} kind={:?} origin={:?}",
                            arg, pred, fn_ir.values[arg].kind, fn_ir.values[arg].origin_var
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(" | "),
                _ => String::new(),
            };
            eprintln!(
                "   [vec-loop-state] {} reject: phi={} var={} bb={} non-destination-loop-state kind={:?} args=[{}]",
                fn_ir.name, value.id, origin_var, phi_bb, value.kind, phi_args
            );
        }
        return true;
    }
    false
}

pub(super) fn preserve_phi_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
    if matches!(&fn_ir.values[vid].kind, ValueKind::Phi { .. }) {
        vid
    } else {
        canonical_value(fn_ir, vid)
    }
}

pub(super) fn loop_carried_phi_is_unmodified(fn_ir: &FnIR, phi_vid: ValueId) -> bool {
    let phi_vid = preserve_phi_value(fn_ir, phi_vid);
    let ValueKind::Phi { args } = &fn_ir.values[phi_vid].kind else {
        return false;
    };
    let mut found: Option<ValueId> = None;
    for (arg, _) in args {
        let arg = canonical_value(fn_ir, *arg);
        if arg == phi_vid {
            continue;
        }
        match found {
            None => found = Some(arg),
            Some(prev) if canonical_value(fn_ir, prev) == arg => {}
            Some(_) => return false,
        }
    }
    found.is_some()
}

pub(super) fn loop_carried_phi_is_invariant_passthrough(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    phi_vid: ValueId,
    iv_phi: ValueId,
) -> bool {
    let phi_vid = preserve_phi_value(fn_ir, phi_vid);
    if let Some(var) = fn_ir.values[phi_vid].origin_var.as_deref()
        && var.starts_with(".arg_")
        && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
    {
        return true;
    }

    fn invariant_passthrough_key(
        fn_ir: &FnIR,
        lp: &LoopInfo,
        root: ValueId,
        iv_phi: ValueId,
        seen: &mut FxHashSet<ValueId>,
    ) -> Option<String> {
        let root = preserve_phi_value(fn_ir, root);
        if let Some(var) = fn_ir.values[root].origin_var.as_deref()
            && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
        {
            return Some(format!("origin:{var}"));
        }
        if !seen.insert(root) {
            if let Some(var) = fn_ir.values[root].origin_var.as_deref()
                && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
            {
                return Some(format!("origin:{var}"));
            }
            return None;
        }
        let out = match &fn_ir.values[root].kind {
            ValueKind::Phi { args }
                if fn_ir.values[root]
                    .phi_block
                    .is_some_and(|bb| lp.body.contains(&bb)) =>
            {
                if let Some(var) = fn_ir.values[root].origin_var.as_deref()
                    && has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
                {
                    return None;
                }
                let mut found: Option<String> = None;
                let mut saw_non_self = false;
                for (arg, _) in args {
                    let arg = preserve_phi_value(fn_ir, *arg);
                    if arg == root {
                        continue;
                    }
                    saw_non_self = true;
                    let key = invariant_passthrough_key(fn_ir, lp, arg, iv_phi, seen)?;
                    match &found {
                        None => found = Some(key),
                        Some(prev) if prev == &key => {}
                        Some(_) => return None,
                    }
                }
                if saw_non_self {
                    found
                } else if let Some(var) = fn_ir.values[root].origin_var.as_deref()
                    && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
                {
                    Some(format!("origin:{var}"))
                } else {
                    None
                }
            }
            ValueKind::Phi { args } if args.is_empty() => {
                if let Some(var) = fn_ir.values[root].origin_var.as_deref()
                    && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
                {
                    Some(format!("origin:{var}"))
                } else {
                    None
                }
            }
            ValueKind::Const(lit) => Some(format!("const:{lit:?}")),
            ValueKind::Param { index } => {
                if let Some(var) = fn_ir.values[root].origin_var.as_deref()
                    && !has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
                {
                    Some(format!("origin:{var}"))
                } else {
                    Some(format!("param:{index}"))
                }
            }
            ValueKind::Load { var } => {
                if !has_non_passthrough_assignment_in_loop(fn_ir, lp, var) {
                    Some(format!("origin:{var}"))
                } else {
                    Some(format!("load:{var}"))
                }
            }
            ValueKind::Call { callee, args, .. }
                if matches!(callee.as_str(), "seq_len" | "rep.int" | "numeric") =>
            {
                let mut arg_keys = Vec::with_capacity(args.len());
                for arg in args {
                    let arg = preserve_phi_value(fn_ir, *arg);
                    let key = invariant_passthrough_key(fn_ir, lp, arg, iv_phi, seen)?;
                    arg_keys.push(key);
                }
                Some(format!("call:{}({})", callee, arg_keys.join(",")))
            }
            _ if is_iv_equivalent(fn_ir, root, iv_phi)
                || expr_has_iv_dependency(fn_ir, root, iv_phi) =>
            {
                None
            }
            _ => None,
        };
        seen.remove(&root);
        out
    }

    let phi_vid = preserve_phi_value(fn_ir, phi_vid);
    let ValueKind::Phi { args } = &fn_ir.values[phi_vid].kind else {
        return false;
    };
    if !fn_ir.values[phi_vid]
        .phi_block
        .is_some_and(|bb| lp.body.contains(&bb))
    {
        return false;
    }

    let mut found: Option<String> = None;
    for (arg, _) in args {
        let key = invariant_passthrough_key(fn_ir, lp, *arg, iv_phi, &mut FxHashSet::default());
        let Some(key) = key else {
            return false;
        };
        match &found {
            None => found = Some(key),
            Some(prev) if prev == &key => {}
            Some(_) => return false,
        }
    }
    found.is_some()
}
