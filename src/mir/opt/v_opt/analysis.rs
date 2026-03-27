use super::debug::{VectorizeSkipReason, vectorize_trace_enabled};
use super::planning::{Axis3D, CallMapArg, VectorPlan, is_builtin_vector_safe_call};
use super::reconstruct::{
    has_non_passthrough_assignment_in_loop, is_scalar_broadcast_value,
    unique_assign_source_in_loop, unique_assign_source_reaching_block_in_loop,
    value_use_block_in_loop,
};
use super::types::{
    BlockStore1D, BlockStore1DMatch, BlockStore3D, BlockStore3DMatch, CallMapLoweringMode,
    VectorAccessOperand3D, VectorAccessPattern3D,
};
use crate::mir::analyze::effects;
use crate::mir::opt::loop_analysis::{LoopInfo, build_pred_map};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) const CALL_MAP_AUTO_HELPER_COST_THRESHOLD: u32 = 6;

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
    match (callee, arity) {
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
    match callee {
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
        ValueKind::Call { callee, args, .. } => {
            call_runtime_helper_cost(callee)
                + args
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
        && is_builtin_vector_safe_call(callee, args.len())
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
    is_builtin_vector_safe_call(callee, arity) || user_call_whitelist.contains(callee)
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
            ValueKind::Call { callee, args, .. } => {
                is_vector_safe_call(callee, args.len(), user_call_whitelist)
                    && args
                        .iter()
                        .all(|a| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|a| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen)),
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
            ValueKind::Call { callee, args, .. } => {
                is_vector_safe_call(callee, args.len(), user_call_whitelist)
                    && args
                        .iter()
                        .all(|a| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|a| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen)),
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

pub(super) fn loop_vectorize_skip_reason(fn_ir: &FnIR, lp: &LoopInfo) -> VectorizeSkipReason {
    if lp.iv.is_none() {
        return VectorizeSkipReason::NoIv;
    }
    if lp.is_seq_len.is_none() {
        return VectorizeSkipReason::NonCanonicalBound;
    }
    if loop_has_unsupported_cfg_shape(fn_ir, lp) {
        return VectorizeSkipReason::UnsupportedCfgShape;
    }
    if loop_has_indirect_index_access(fn_ir, lp) {
        return VectorizeSkipReason::IndirectIndexAccess;
    }
    if loop_has_store_effect(fn_ir, lp) {
        return VectorizeSkipReason::StoreEffects;
    }
    VectorizeSkipReason::NoSupportedPattern
}

pub(super) fn loop_has_unsupported_cfg_shape(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let preds = build_pred_map(fn_ir);
    let outer_preds = preds
        .get(&lp.header)
        .map(|ps| ps.iter().filter(|b| !lp.body.contains(b)).count())
        .unwrap_or(0);
    outer_preds != 1 || lp.exits.len() != 1
}

pub(super) fn loop_has_indirect_index_access(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let Some(iv) = lp.iv.as_ref() else {
        return false;
    };
    let iv_phi = iv.phi_val;
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                    if expr_has_non_iv_index(fn_ir, *src, iv_phi, &mut FxHashSet::default()) {
                        return true;
                    }
                }
                Instr::StoreIndex1D { idx, val, .. } => {
                    if !is_iv_equivalent(fn_ir, *idx, iv_phi) {
                        return true;
                    }
                    if expr_has_non_iv_index(fn_ir, *val, iv_phi, &mut FxHashSet::default()) {
                        return true;
                    }
                }
                Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => return true,
            }
        }
    }
    false
}

pub(super) fn expr_has_non_iv_index(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Index1D { idx, base, .. } => {
            if !is_iv_equivalent(fn_ir, *idx, iv_phi) {
                return true;
            }
            expr_has_non_iv_index(fn_ir, *base, iv_phi, seen)
                || expr_has_non_iv_index(fn_ir, *idx, iv_phi, seen)
        }
        ValueKind::Index2D { .. } | ValueKind::Index3D { .. } => true,
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_has_non_iv_index(fn_ir, *lhs, iv_phi, seen)
                || expr_has_non_iv_index(fn_ir, *rhs, iv_phi, seen)
        }
        ValueKind::Unary { rhs, .. } => expr_has_non_iv_index(fn_ir, *rhs, iv_phi, seen),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|a| expr_has_non_iv_index(fn_ir, *a, iv_phi, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(a, _)| expr_has_non_iv_index(fn_ir, *a, iv_phi, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_has_non_iv_index(fn_ir, *base, iv_phi, seen)
        }
        ValueKind::Range { start, end } => {
            expr_has_non_iv_index(fn_ir, *start, iv_phi, seen)
                || expr_has_non_iv_index(fn_ir, *end, iv_phi, seen)
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
    }
}

pub(super) fn resolve_base_var(fn_ir: &FnIR, base: ValueId) -> Option<VarId> {
    if let ValueKind::Load { var } = &fn_ir.values[base].kind {
        return Some(var.clone());
    }
    fn_ir.values[base].origin_var.clone()
}

pub(super) fn rewrite_returns_for_var(fn_ir: &mut FnIR, var: &str, new_val: ValueId) {
    for bid in 0..fn_ir.blocks.len() {
        if let Terminator::Return(Some(ret_vid)) = fn_ir.blocks[bid].term
            && fn_ir.values[ret_vid].origin_var.as_deref() == Some(var)
        {
            fn_ir.blocks[bid].term = Terminator::Return(Some(new_val));
        }
    }
}

pub(super) fn extract_store_1d_in_block(
    fn_ir: &FnIR,
    bid: BlockId,
    iv_phi: ValueId,
) -> Option<(ValueId, ValueId)> {
    match classify_store_1d_in_block(fn_ir, bid) {
        BlockStore1DMatch::One(store)
            if !store.is_vector && is_iv_equivalent(fn_ir, store.idx, iv_phi) =>
        {
            Some((store.base, store.val))
        }
        _ => None,
    }
}

pub(super) fn classify_store_1d_in_block(fn_ir: &FnIR, bid: BlockId) -> BlockStore1DMatch {
    let mut found: Option<BlockStore1D> = None;
    for instr in &fn_ir.blocks[bid].instrs {
        match instr {
            Instr::Assign { .. } | Instr::Eval { .. } => {}
            Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => {
                return BlockStore1DMatch::Invalid;
            }
            Instr::StoreIndex1D {
                base,
                idx,
                val,
                is_vector,
                ..
            } => {
                if found.is_some() {
                    return BlockStore1DMatch::Invalid;
                }
                found = Some(BlockStore1D {
                    base: *base,
                    idx: *idx,
                    val: *val,
                    is_vector: *is_vector,
                });
            }
        }
    }
    match found {
        Some(store) => BlockStore1DMatch::One(store),
        None => BlockStore1DMatch::None,
    }
}

pub(super) fn classify_store_3d_in_block(fn_ir: &FnIR, bid: BlockId) -> BlockStore3DMatch {
    let mut found: Option<BlockStore3D> = None;
    for instr in &fn_ir.blocks[bid].instrs {
        match instr {
            Instr::Assign { .. } | Instr::Eval { .. } => {}
            Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => {
                return BlockStore3DMatch::Invalid;
            }
            Instr::StoreIndex3D {
                base, i, j, k, val, ..
            } => {
                if found.is_some() {
                    return BlockStore3DMatch::Invalid;
                }
                found = Some(BlockStore3D {
                    base: *base,
                    i: *i,
                    j: *j,
                    k: *k,
                    val: *val,
                });
            }
        }
    }
    match found {
        Some(store) => BlockStore3DMatch::One(store),
        None => BlockStore3DMatch::None,
    }
}

pub(super) fn is_prev_element(fn_ir: &FnIR, vid: ValueId, base: ValueId, iv_phi: ValueId) -> bool {
    match &fn_ir.values[vid].kind {
        ValueKind::Index1D { base: b, idx, .. } => {
            if canonical_value(fn_ir, *b) != canonical_value(fn_ir, base) {
                return false;
            }
            is_iv_minus_one(fn_ir, *idx, iv_phi)
        }
        _ => false,
    }
}

pub(super) fn is_prev_element_3d(
    fn_ir: &FnIR,
    vid: ValueId,
    base: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    iv_phi: ValueId,
) -> bool {
    match &fn_ir.values[vid].kind {
        ValueKind::Index3D { base: b, i, j, k } => {
            if canonical_value(fn_ir, *b) != canonical_value(fn_ir, base) {
                return false;
            }
            let i = resolve_load_alias_value(fn_ir, *i);
            let j = resolve_load_alias_value(fn_ir, *j);
            let k = resolve_load_alias_value(fn_ir, *k);
            let fixed_a = resolve_load_alias_value(fn_ir, fixed_a);
            let fixed_b = resolve_load_alias_value(fn_ir, fixed_b);
            match axis {
                Axis3D::Dim1 => {
                    is_iv_minus_one(fn_ir, i, iv_phi)
                        && same_loop_invariant_value(fn_ir, j, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, k, fixed_b, iv_phi)
                }
                Axis3D::Dim2 => {
                    is_iv_minus_one(fn_ir, j, iv_phi)
                        && same_loop_invariant_value(fn_ir, i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, k, fixed_b, iv_phi)
                }
                Axis3D::Dim3 => {
                    is_iv_minus_one(fn_ir, k, iv_phi)
                        && same_loop_invariant_value(fn_ir, i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, j, fixed_b, iv_phi)
                }
            }
        }
        _ => false,
    }
}

pub(super) fn is_iv_minus_one(fn_ir: &FnIR, idx: ValueId, iv_phi: ValueId) -> bool {
    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        return false;
    }
    match &fn_ir.values[idx].kind {
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } if is_iv_equivalent(fn_ir, *lhs, iv_phi) => {
            matches!(fn_ir.values[*rhs].kind, ValueKind::Const(Lit::Int(1)))
        }
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } if is_iv_equivalent(fn_ir, *lhs, iv_phi) => {
            matches!(fn_ir.values[*rhs].kind, ValueKind::Const(Lit::Int(-1)))
        }
        _ => false,
    }
}

pub(super) fn expr_reads_base(fn_ir: &FnIR, root: ValueId, base: ValueId) -> bool {
    fn rec(fn_ir: &FnIR, root: ValueId, base: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Index1D { base: b, .. }
            | ValueKind::Index2D { base: b, .. }
            | ValueKind::Index3D { base: b, .. } => {
                if canonical_value(fn_ir, *b) == canonical_value(fn_ir, base) {
                    return true;
                }
            }
            _ => {}
        }
        match &fn_ir.values[root].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, base, seen) || rec(fn_ir, *rhs, base, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, base, seen),
            ValueKind::Call { args, .. } => args.iter().any(|a| rec(fn_ir, *a, base, seen)),
            ValueKind::Phi { args } => args.iter().any(|(a, _)| rec(fn_ir, *a, base, seen)),
            ValueKind::Len { base: b } | ValueKind::Indices { base: b } => {
                rec(fn_ir, *b, base, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, base, seen) || rec(fn_ir, *end, base, seen)
            }
            ValueKind::Index1D { base: b, idx, .. } => {
                rec(fn_ir, *b, base, seen) || rec(fn_ir, *idx, base, seen)
            }
            ValueKind::Index2D { base: b, r, c } => {
                rec(fn_ir, *b, base, seen)
                    || rec(fn_ir, *r, base, seen)
                    || rec(fn_ir, *c, base, seen)
            }
            ValueKind::Index3D { base: b, i, j, k } => {
                rec(fn_ir, *b, base, seen)
                    || rec(fn_ir, *i, base, seen)
                    || rec(fn_ir, *j, base, seen)
                    || rec(fn_ir, *k, base, seen)
            }
            _ => false,
        }
    }
    rec(fn_ir, root, base, &mut FxHashSet::default())
}

pub(super) fn expr_has_non_vector_safe_call(
    fn_ir: &FnIR,
    root: ValueId,
    user_call_whitelist: &FxHashSet<String>,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Call { callee, args, .. } => {
            if !is_vector_safe_call(callee, args.len(), user_call_whitelist)
                && !is_runtime_vector_read_call(callee, args.len())
            {
                if vectorize_trace_enabled() {
                    eprintln!(
                        "   [vec-expr-map] non-vector-safe call: {} / arity {}",
                        callee,
                        args.len()
                    );
                }
                return true;
            }
            args.iter()
                .any(|a| expr_has_non_vector_safe_call(fn_ir, *a, user_call_whitelist, seen))
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *lhs, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *rhs, user_call_whitelist, seen)
        }
        ValueKind::Unary { rhs, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *rhs, user_call_whitelist, seen)
        }
        ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|a| expr_has_non_vector_safe_call(fn_ir, *a, user_call_whitelist, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(a, _)| expr_has_non_vector_safe_call(fn_ir, *a, user_call_whitelist, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
        }
        ValueKind::Range { start, end } => {
            expr_has_non_vector_safe_call(fn_ir, *start, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *end, user_call_whitelist, seen)
        }
        ValueKind::Index1D { base, idx, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *idx, user_call_whitelist, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *r, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *c, user_call_whitelist, seen)
        }
        ValueKind::Index3D { base, i, j, k } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *i, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *j, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *k, user_call_whitelist, seen)
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
    }
}

pub(super) fn expr_has_non_vector_safe_call_in_vector_context(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    user_call_whitelist: &FxHashSet<String>,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    if is_loop_invariant_pure_scalar_call_expr(fn_ir, root, iv_phi) {
        return false;
    }
    if !seen.insert(root) {
        return false;
    }
    if is_scalar_broadcast_value(fn_ir, root) && !expr_has_iv_dependency(fn_ir, root, iv_phi) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Call { callee, args, .. } => {
            if !is_vector_safe_call(callee, args.len(), user_call_whitelist)
                && !is_runtime_vector_read_call(callee, args.len())
            {
                if vectorize_trace_enabled() {
                    eprintln!(
                        "   [vec-expr-map] non-vector-safe call: {} / arity {}",
                        callee,
                        args.len()
                    );
                }
                return true;
            }
            args.iter().any(|a| {
                expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *a,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
            })
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *lhs,
                iv_phi,
                user_call_whitelist,
                seen,
            ) || expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *rhs,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }
        ValueKind::Unary { rhs, .. } => expr_has_non_vector_safe_call_in_vector_context(
            fn_ir,
            *rhs,
            iv_phi,
            user_call_whitelist,
            seen,
        ),
        ValueKind::Intrinsic { args, .. } => args.iter().any(|a| {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *a,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }),
        ValueKind::Phi { args } => args.iter().any(|(a, _)| {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *a,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Range { start, end } => {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *start,
                iv_phi,
                user_call_whitelist,
                seen,
            ) || expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *end,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }
        ValueKind::Index1D { base, idx, .. } => {
            let base_blocking = expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                );
            base_blocking
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *idx,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Index2D { base, r, c } => {
            let base_blocking = expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                );
            base_blocking
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *r,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *c,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Index3D { base, i, j, k } => {
            let base_blocking = expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                );
            base_blocking
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *i,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *j,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *k,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
    }
}

fn is_loop_invariant_pure_scalar_call_expr(fn_ir: &FnIR, root: ValueId, iv_phi: ValueId) -> bool {
    fn scalar_reducer_call(callee: &str, arity: usize) -> bool {
        matches!(
            (callee, arity),
            ("sum", 1)
                | ("mean", 1)
                | ("var", 1)
                | ("sd", 1)
                | ("min", 1)
                | ("max", 1)
                | ("prod", 1)
                | ("length", 1)
                | ("nrow", 1)
                | ("ncol", 1)
        )
    }

    fn invariant_pure_arg_expr(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return true;
        }
        if expr_has_iv_dependency(fn_ir, root, iv_phi) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_)
            | ValueKind::Load { .. }
            | ValueKind::Param { .. }
            | ValueKind::RSymbol { .. } => true,
            ValueKind::Unary { rhs, .. } => invariant_pure_arg_expr(fn_ir, *rhs, iv_phi, seen),
            ValueKind::Binary { lhs, rhs, .. } => {
                invariant_pure_arg_expr(fn_ir, *lhs, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *rhs, iv_phi, seen)
            }
            ValueKind::Call { callee, args, .. } => {
                effects::call_is_pure(callee)
                    && args
                        .iter()
                        .all(|arg| invariant_pure_arg_expr(fn_ir, *arg, iv_phi, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|arg| invariant_pure_arg_expr(fn_ir, *arg, iv_phi, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                invariant_pure_arg_expr(fn_ir, *base, iv_phi, seen)
            }
            ValueKind::Range { start, end } => {
                invariant_pure_arg_expr(fn_ir, *start, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *end, iv_phi, seen)
            }
            ValueKind::Index1D { base, idx, .. } => {
                invariant_pure_arg_expr(fn_ir, *base, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *idx, iv_phi, seen)
            }
            ValueKind::Index2D { base, r, c } => {
                invariant_pure_arg_expr(fn_ir, *base, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *r, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *c, iv_phi, seen)
            }
            ValueKind::Index3D { base, i, j, k } => {
                invariant_pure_arg_expr(fn_ir, *base, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *i, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *j, iv_phi, seen)
                    && invariant_pure_arg_expr(fn_ir, *k, iv_phi, seen)
            }
            ValueKind::Phi { .. } => false,
        }
    }

    if expr_has_iv_dependency(fn_ir, root, iv_phi) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Call { callee, args, .. } => {
            scalar_reducer_call(callee, args.len())
                && args.iter().all(|arg| {
                    invariant_pure_arg_expr(fn_ir, *arg, iv_phi, &mut FxHashSet::default())
                })
        }
        ValueKind::Intrinsic { op, args } => {
            matches!(op, IntrinsicOp::VecSumF64 | IntrinsicOp::VecMeanF64)
                && args.iter().all(|arg| {
                    invariant_pure_arg_expr(fn_ir, *arg, iv_phi, &mut FxHashSet::default())
                })
        }
        _ => false,
    }
}

pub(super) fn is_runtime_vector_read_call(callee: &str, arity: usize) -> bool {
    (matches!(callee, "rr_index1_read" | "rr_index1_read_strict") && (arity == 2 || arity == 3))
        || (callee == "rr_array3_gather_values" && arity == 4)
}

pub(super) fn floor_like_index_source(fn_ir: &FnIR, idx: ValueId) -> Option<ValueId> {
    let idx = canonical_value(fn_ir, idx);
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &fn_ir.values[idx].kind
    else {
        return None;
    };
    if !matches!(callee.as_str(), "floor" | "ceiling" | "trunc") {
        return None;
    }
    if args.len() != 1 {
        return None;
    }
    if !names.is_empty() && names.first().and_then(|n| n.as_ref()).is_some() {
        return None;
    }
    Some(args[0])
}

pub(super) fn same_base_value(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    let a = canonical_value(fn_ir, a);
    let b = canonical_value(fn_ir, b);
    if a == b {
        return true;
    }
    match (resolve_base_var(fn_ir, a), resolve_base_var(fn_ir, b)) {
        (Some(va), Some(vb)) => va == vb,
        _ => false,
    }
}

pub(super) fn expr_reads_base_non_iv(
    fn_ir: &FnIR,
    root: ValueId,
    base: ValueId,
    iv_phi: ValueId,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        base: ValueId,
        iv_phi: ValueId,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Index1D {
                base: read_base,
                idx,
                ..
            } => {
                if same_base_value(fn_ir, *read_base, base)
                    && !is_iv_equivalent(fn_ir, *idx, iv_phi)
                {
                    return true;
                }
                rec(fn_ir, *read_base, base, iv_phi, seen) || rec(fn_ir, *idx, base, iv_phi, seen)
            }
            ValueKind::Index2D {
                base: read_base,
                r,
                c,
            } => {
                if same_base_value(fn_ir, *read_base, base) {
                    return true;
                }
                rec(fn_ir, *read_base, base, iv_phi, seen)
                    || rec(fn_ir, *r, base, iv_phi, seen)
                    || rec(fn_ir, *c, base, iv_phi, seen)
            }
            ValueKind::Index3D {
                base: read_base,
                i,
                j,
                k,
            } => {
                if same_base_value(fn_ir, *read_base, base) {
                    return true;
                }
                rec(fn_ir, *read_base, base, iv_phi, seen)
                    || rec(fn_ir, *i, base, iv_phi, seen)
                    || rec(fn_ir, *j, base, iv_phi, seen)
                    || rec(fn_ir, *k, base, iv_phi, seen)
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, base, iv_phi, seen) || rec(fn_ir, *rhs, base, iv_phi, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, base, iv_phi, seen),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                args.iter().any(|a| rec(fn_ir, *a, base, iv_phi, seen))
            }
            ValueKind::Phi { args } => args.iter().any(|(a, _)| rec(fn_ir, *a, base, iv_phi, seen)),
            ValueKind::Len { base: b } | ValueKind::Indices { base: b } => {
                rec(fn_ir, *b, base, iv_phi, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, base, iv_phi, seen) || rec(fn_ir, *end, base, iv_phi, seen)
            }
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => false,
        }
    }
    rec(fn_ir, root, base, iv_phi, &mut FxHashSet::default())
}

pub(super) fn expr_has_iv_dependency(fn_ir: &FnIR, root: ValueId, iv_phi: ValueId) -> bool {
    fn load_var_depends_on_iv(
        fn_ir: &FnIR,
        var: &str,
        iv_phi: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        if !seen_vars.insert(var.to_string()) {
            return false;
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                if rec(fn_ir, *src, iv_phi, seen_vals, seen_vars) {
                    seen_vars.remove(var);
                    return true;
                }
            }
        }
        seen_vars.remove(var);
        false
    }

    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            return true;
        }
        if !seen_vals.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *rhs, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, iv_phi, seen_vals, seen_vars),
            ValueKind::Call { args, .. } => args
                .iter()
                .any(|a| rec(fn_ir, *a, iv_phi, seen_vals, seen_vars)),
            ValueKind::Phi { args } => args
                .iter()
                .any(|(a, _)| rec(fn_ir, *a, iv_phi, seen_vals, seen_vars)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, *base, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *end, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index1D { idx, .. } => rec(fn_ir, *idx, iv_phi, seen_vals, seen_vars),
            ValueKind::Index2D { r, c, .. } => {
                rec(fn_ir, *r, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *c, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index3D { i, j, k, .. } => {
                rec(fn_ir, *i, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *j, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *k, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Load { var } => {
                load_var_depends_on_iv(fn_ir, var, iv_phi, seen_vals, seen_vars)
            }
            _ => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

pub(super) fn is_vectorizable_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    allow_any_base: bool,
    require_safe_index: bool,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        allow_any_base: bool,
        require_safe_index: bool,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        if root == iv_phi {
            return true;
        }
        if !seen.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. } => true,
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(
                    fn_ir,
                    *lhs,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                ) && rec(
                    fn_ir,
                    *rhs,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                )
            }
            ValueKind::Unary { rhs, .. } => rec(
                fn_ir,
                *rhs,
                iv_phi,
                lp,
                allow_any_base,
                require_safe_index,
                seen,
            ),
            ValueKind::Call { args, .. } => args.iter().all(|a| {
                rec(
                    fn_ir,
                    *a,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                )
            }),
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                if require_safe_index && !(*is_safe && *is_na_safe) {
                    return false;
                }
                if !allow_any_base && !is_loop_compatible_base(lp, fn_ir, *base) {
                    return false;
                }
                if is_iv_equivalent(fn_ir, *idx, iv_phi) {
                    return true;
                }
                !require_safe_index
                    && expr_has_iv_dependency(fn_ir, *idx, iv_phi)
                    && rec(
                        fn_ir,
                        *idx,
                        iv_phi,
                        lp,
                        allow_any_base,
                        require_safe_index,
                        seen,
                    )
            }
            ValueKind::Index3D { base, i, j, k } => {
                if !allow_any_base && !is_loop_compatible_base(lp, fn_ir, *base) {
                    return false;
                }
                let Some(pattern) =
                    classify_3d_general_vector_access(fn_ir, *base, *i, *j, *k, iv_phi)
                else {
                    return false;
                };
                [pattern.i, pattern.j, pattern.k]
                    .into_iter()
                    .all(|operand| match operand {
                        VectorAccessOperand3D::Scalar(_) => true,
                        VectorAccessOperand3D::Vector(dep_idx) => {
                            is_iv_equivalent(fn_ir, dep_idx, iv_phi)
                                || rec(
                                    fn_ir,
                                    dep_idx,
                                    iv_phi,
                                    lp,
                                    allow_any_base,
                                    require_safe_index,
                                    seen,
                                )
                        }
                    })
            }
            ValueKind::Phi { args } => {
                if fn_ir.values[root]
                    .phi_block
                    .is_some_and(|bb| !lp.body.contains(&bb))
                    && !expr_has_iv_dependency(fn_ir, root, iv_phi)
                {
                    true
                } else {
                    args.iter().all(|(a, _)| {
                        rec(
                            fn_ir,
                            *a,
                            iv_phi,
                            lp,
                            allow_any_base,
                            require_safe_index,
                            seen,
                        )
                    })
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => rec(
                fn_ir,
                *base,
                iv_phi,
                lp,
                allow_any_base,
                require_safe_index,
                seen,
            ),
            ValueKind::Range { start, end } => {
                rec(
                    fn_ir,
                    *start,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                ) && rec(
                    fn_ir,
                    *end,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                )
            }
            _ => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        allow_any_base,
        require_safe_index,
        &mut FxHashSet::default(),
    )
}

pub(super) fn is_condition_vectorizable(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    fn load_var_condition_vectorizable(
        fn_ir: &FnIR,
        var: &str,
        iv_phi: ValueId,
        lp: &LoopInfo,
        user_call_whitelist: &FxHashSet<String>,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        if !seen_vars.insert(var.to_string()) {
            return false;
        }
        let mut found = false;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                found = true;
                if !rec(
                    fn_ir,
                    *src,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                ) {
                    seen_vars.remove(var);
                    return false;
                }
            }
        }
        seen_vars.remove(var);
        // Params and immutable captures can appear as bare loads with no local assignment.
        // Treat them as loop-invariant condition inputs.
        if !found {
            return true;
        }
        true
    }

    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        user_call_whitelist: &FxHashSet<String>,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if root == iv_phi || is_iv_equivalent(fn_ir, root, iv_phi) {
            return true;
        }
        if !seen_vals.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Param { .. } => true,
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(
                    fn_ir,
                    *lhs,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                ) && rec(
                    fn_ir,
                    *rhs,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                )
            }
            ValueKind::Unary { rhs, .. } => rec(
                fn_ir,
                *rhs,
                iv_phi,
                lp,
                user_call_whitelist,
                seen_vals,
                seen_vars,
            ),
            // Data-dependent conditions are now allowed if the access is proven safe.
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                let iv_idx = is_iv_equivalent(fn_ir, *idx, iv_phi);
                let iv_dependent_idx = iv_idx || expr_has_iv_dependency(fn_ir, *idx, iv_phi);
                if !is_loop_compatible_base(lp, fn_ir, *base) && !iv_dependent_idx {
                    return false;
                }
                // Fast-path: proven-safe scalar read on IV-aligned index.
                if *is_safe && *is_na_safe && iv_idx {
                    return true;
                }
                // Pre-BCE loops often carry unspecialized index safety flags.
                // Allow IV-dependent index expressions and rely on runtime vector
                // read guards during materialization (`rr_index1_read_vec` path).
                iv_dependent_idx
            }
            ValueKind::Index2D { .. } => false,
            ValueKind::Index3D { base, i, j, k } => {
                is_loop_compatible_base(lp, fn_ir, *base)
                    && classify_3d_general_vector_access(fn_ir, *base, *i, *j, *k, iv_phi)
                        .is_some_and(|pattern| {
                            [pattern.i, pattern.j, pattern.k]
                                .into_iter()
                                .all(|operand| match operand {
                                    VectorAccessOperand3D::Scalar(_) => true,
                                    VectorAccessOperand3D::Vector(dep_idx) => rec(
                                        fn_ir,
                                        dep_idx,
                                        iv_phi,
                                        lp,
                                        user_call_whitelist,
                                        seen_vals,
                                        seen_vars,
                                    ),
                                })
                        })
            }
            ValueKind::Call { callee, args, .. } => {
                let runtime_read = is_runtime_vector_read_call(callee, args.len());
                if runtime_read
                    && let Some(base) = args.first().copied()
                    && !is_loop_compatible_base(lp, fn_ir, base)
                {
                    return false;
                }
                (is_vector_safe_call(callee, args.len(), user_call_whitelist) || runtime_read)
                    && args.iter().all(|a| {
                        rec(
                            fn_ir,
                            *a,
                            iv_phi,
                            lp,
                            user_call_whitelist,
                            seen_vals,
                            seen_vars,
                        )
                    })
            }
            ValueKind::Phi { args } => args.iter().all(|(a, _)| {
                rec(
                    fn_ir,
                    *a,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                )
            }),
            ValueKind::Load { var } => load_var_condition_vectorizable(
                fn_ir,
                var,
                iv_phi,
                lp,
                user_call_whitelist,
                seen_vals,
                seen_vars,
            ),
            _ => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        user_call_whitelist,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

pub(super) fn is_loop_compatible_base(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    if loop_matches_vec(lp, fn_ir, base) {
        return true;
    }
    let Some(loop_key) = loop_length_key(lp, fn_ir) else {
        return false;
    };
    match vector_length_key(fn_ir, base) {
        Some(k) => canonical_value(fn_ir, k) == canonical_value(fn_ir, loop_key),
        None => false,
    }
}

pub(super) fn loop_covers_whole_destination(
    lp: &LoopInfo,
    fn_ir: &FnIR,
    base: ValueId,
    start: ValueId,
) -> bool {
    is_const_one(fn_ir, start) && loop_matches_full_base(lp, fn_ir, base)
}

pub(super) fn loop_length_key(lp: &LoopInfo, fn_ir: &FnIR) -> Option<ValueId> {
    if lp.limit_adjust != 0 {
        return None;
    }
    let limit = lp.limit?;
    match &fn_ir.values[limit].kind {
        ValueKind::Len { base } => vector_length_key(fn_ir, *base),
        _ => Some(resolve_load_alias_value(fn_ir, limit)),
    }
}

pub(super) fn affine_iv_offset(fn_ir: &FnIR, idx: ValueId, iv_phi: ValueId) -> Option<i64> {
    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        return Some(0);
    }
    match &fn_ir.values[idx].kind {
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind
            {
                return Some(k);
            }
            if is_iv_equivalent(fn_ir, *rhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*lhs].kind
            {
                return Some(k);
            }
            None
        }
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind
            {
                return Some(-k);
            }
            None
        }
        _ => None,
    }
}

pub(super) fn loop_matches_full_base(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    if lp.limit_adjust != 0 {
        return false;
    }

    let base = canonical_value(fn_ir, base);
    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) == Some(base) {
        return true;
    }
    if let Some(loop_base) = lp.is_seq_along
        && let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, loop_base),
        )
        && a == b
    {
        return true;
    }

    if let Some(limit) = lp
        .is_seq_len
        .map(|limit| resolve_load_alias_value(fn_ir, limit))
    {
        if let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind {
            if canonical_value(fn_ir, len_base) == base {
                return true;
            }
            if let (Some(a), Some(b)) = (
                resolve_base_var(fn_ir, base),
                resolve_base_var(fn_ir, len_base),
            ) && a == b
            {
                return true;
            }
        }

        if base_length_key(fn_ir, base).is_some_and(|base_key| {
            canonical_value(fn_ir, base_key) == canonical_value(fn_ir, limit)
        }) {
            return true;
        }
    }

    if let (Some(base_key), Some(loop_key)) =
        (base_length_key(fn_ir, base), loop_length_key(lp, fn_ir))
    {
        return canonical_value(fn_ir, base_key) == canonical_value(fn_ir, loop_key);
    }

    false
}

pub(super) fn loop_matches_vec(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    let base = canonical_value(fn_ir, base);
    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) == Some(base) {
        return true;
    }
    if let Some(loop_base) = lp.is_seq_along
        && let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, loop_base),
        )
        && a == b
    {
        return true;
    }
    if let Some(limit) = lp.is_seq_len
        && let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind
    {
        if canonical_value(fn_ir, len_base) == base {
            return true;
        }
        if let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, len_base),
        ) && a == b
        {
            return true;
        }
    }
    false
}

pub(super) fn vector_length_key(fn_ir: &FnIR, root: ValueId) -> Option<ValueId> {
    fn unify_length_keys(
        fn_ir: &FnIR,
        keys: impl IntoIterator<Item = Option<ValueId>>,
    ) -> Option<ValueId> {
        let mut out: Option<ValueId> = None;
        for key in keys {
            let key = key.map(|k| resolve_load_alias_value(fn_ir, k))?;
            match out {
                None => out = Some(key),
                Some(prev) if resolve_load_alias_value(fn_ir, prev) == key => {}
                Some(_) => return None,
            }
        }
        out
    }

    fn rec_var(
        fn_ir: &FnIR,
        var: &str,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<ValueId> {
        if !seen_vars.insert(var.to_string()) {
            return None;
        }
        if let Some(param_index) = fn_ir.params.iter().position(|p| p == var) {
            let param_vid = fn_ir
                .values
                .iter()
                .position(|value| matches!(value.kind, ValueKind::Param { index } if index == param_index))
                .map(|vid| vid as ValueId);
            if let Some(param_vid) = param_vid {
                return Some(resolve_load_alias_value(fn_ir, param_vid));
            }
        }

        let mut saw_assign = false;
        let mut keys = Vec::new();
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                saw_assign = true;
                keys.push(rec(fn_ir, *src, seen_vals, seen_vars));
            }
        }
        if !saw_assign {
            return None;
        }
        unify_length_keys(fn_ir, keys)
    }

    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        if !seen_vals.insert(root) {
            return None;
        }
        if matches!(fn_ir.values[root].kind, ValueKind::Phi { .. })
            && let Some(var) = fn_ir.values[root].origin_var.as_deref()
            && let Some(key) = rec_var(fn_ir, var, seen_vals, seen_vars)
        {
            seen_vals.remove(&root);
            return Some(key);
        }
        let out = match &fn_ir.values[root].kind {
            ValueKind::Load { var } => rec_var(fn_ir, var, seen_vals, seen_vars),
            ValueKind::Phi { args } => {
                let keys = args
                    .iter()
                    .filter_map(|(arg, _)| {
                        let arg = canonical_value(fn_ir, *arg);
                        if arg == root {
                            None
                        } else {
                            rec(fn_ir, arg, seen_vals, seen_vars)
                        }
                    })
                    .collect::<Vec<_>>();
                if keys.is_empty() {
                    None
                } else {
                    unify_length_keys(fn_ir, keys.into_iter().map(Some))
                }
            }
            ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
                Some(resolve_load_alias_value(fn_ir, args[0]))
            }
            ValueKind::Call { callee, args, .. }
                if matches!(callee.as_str(), "rep.int" | "numeric") && !args.is_empty() =>
            {
                let len_arg = if callee == "rep.int" {
                    args.get(1).copied().or_else(|| args.first().copied())
                } else {
                    args.first().copied()
                }?;
                Some(resolve_load_alias_value(fn_ir, len_arg))
            }
            ValueKind::Call { callee, args, .. }
                if is_builtin_vector_safe_call(callee, args.len())
                    || matches!(callee.as_str(), "ifelse" | "rr_ifelse_strict") =>
            {
                let mut vec_keys = Vec::new();
                let mut saw_vectorish = false;
                for arg in args {
                    let key = rec(fn_ir, *arg, seen_vals, seen_vars);
                    if key.is_some() {
                        saw_vectorish = true;
                        vec_keys.push(key);
                    } else if !is_scalar_value(fn_ir, *arg) {
                        return None;
                    }
                }
                if !saw_vectorish {
                    None
                } else {
                    unify_length_keys(fn_ir, vec_keys)
                }
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                let lk = rec(fn_ir, *lhs, seen_vals, seen_vars);
                let rk = rec(fn_ir, *rhs, seen_vals, seen_vars);
                match (lk, rk) {
                    (Some(a), Some(b))
                        if canonical_value(fn_ir, a) == canonical_value(fn_ir, b) =>
                    {
                        Some(canonical_value(fn_ir, a))
                    }
                    (Some(k), None) if is_scalar_value(fn_ir, *rhs) => Some(k),
                    (None, Some(k)) if is_scalar_value(fn_ir, *lhs) => Some(k),
                    _ => None,
                }
            }
            _ => None,
        };
        seen_vals.remove(&root);
        out
    }
    rec(
        fn_ir,
        root,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

pub(super) fn variable_length_key(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
    let mut keys = Vec::new();
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            keys.push(vector_length_key(fn_ir, *src));
        }
    }
    if keys.is_empty() {
        return None;
    }
    let mut out: Option<ValueId> = None;
    for key in keys {
        let key = key.map(|k| resolve_load_alias_value(fn_ir, k))?;
        match out {
            None => out = Some(key),
            Some(prev) if resolve_load_alias_value(fn_ir, prev) == key => {}
            Some(_) => return None,
        }
    }
    out
}

pub(super) fn base_length_key(fn_ir: &FnIR, base: ValueId) -> Option<ValueId> {
    if let Some(var) = resolve_base_var(fn_ir, base)
        && let Some(key) = variable_length_key(fn_ir, &var)
    {
        return Some(key);
    }
    vector_length_key(fn_ir, base)
}

pub(super) fn same_length_proven(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    let a = canonical_value(fn_ir, a);
    let b = canonical_value(fn_ir, b);
    if a == b {
        return true;
    }
    if fn_ir.values[a].value_ty.len_sym.is_some()
        && fn_ir.values[a].value_ty.len_sym == fn_ir.values[b].value_ty.len_sym
    {
        return true;
    }
    match (vector_length_key(fn_ir, a), vector_length_key(fn_ir, b)) {
        (Some(ka), Some(kb)) => canonical_value(fn_ir, ka) == canonical_value(fn_ir, kb),
        _ => false,
    }
}

pub(super) fn is_scalar_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    matches!(
        fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. }
    )
}

pub(super) fn is_iv_equivalent(fn_ir: &FnIR, candidate: ValueId, iv_phi: ValueId) -> bool {
    let mut seen_vals = FxHashSet::default();
    let mut seen_vars = FxHashSet::default();
    is_iv_equivalent_rec(fn_ir, candidate, iv_phi, &mut seen_vals, &mut seen_vars)
}

pub(super) fn is_iv_equivalent_rec(
    fn_ir: &FnIR,
    candidate: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> bool {
    let candidate = canonical_value(fn_ir, candidate);
    if candidate == iv_phi {
        return true;
    }
    if !seen_vals.insert(candidate) {
        return false;
    }
    match &fn_ir.values[candidate].kind {
        ValueKind::Load { var } => {
            if induction_origin_var(fn_ir, iv_phi).as_deref() == Some(var.as_str()) {
                return true;
            }
            if let Some(src) = unique_assign_source(fn_ir, var)
                && src != candidate
            {
                return is_iv_equivalent_rec(fn_ir, src, iv_phi, seen_vals, seen_vars);
            }
            load_var_is_floor_like_iv(fn_ir, var, iv_phi, seen_vals, seen_vars)
        }
        ValueKind::Phi { args } if args.is_empty() => {
            match (
                fn_ir.values[candidate].origin_var.as_deref(),
                fn_ir.values[iv_phi].origin_var.as_deref(),
            ) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == candidate {
                    return false;
                }
                return is_iv_equivalent_rec(fn_ir, first, iv_phi, seen_vals, seen_vars);
            }
            let mut saw_iv_progress = false;
            for (v, _) in args {
                if is_iv_equivalent_rec(fn_ir, *v, iv_phi, seen_vals, seen_vars) {
                    saw_iv_progress = true;
                    continue;
                }
                if is_iv_seed_expr(
                    fn_ir,
                    *v,
                    iv_phi,
                    &mut FxHashSet::default(),
                    &mut FxHashSet::default(),
                ) {
                    continue;
                }
                return false;
            }
            saw_iv_progress
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let floor_like = matches!(callee.as_str(), "floor" | "ceiling" | "trunc");
            let single_positional = args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_equivalent_rec(fn_ir, args[0], iv_phi, seen_vals, seen_vars)
        }
        _ => false,
    }
}

pub(super) fn induction_origin_var(fn_ir: &FnIR, iv_phi: ValueId) -> Option<String> {
    fn rec(fn_ir: &FnIR, vid: ValueId, seen: &mut FxHashSet<ValueId>) -> Option<String> {
        let vid = canonical_value(fn_ir, vid);
        if !seen.insert(vid) {
            return None;
        }
        if let Some(origin) = fn_ir.values[vid].origin_var.clone() {
            return Some(origin);
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Load { var } => Some(var.clone()),
            ValueKind::Phi { args } if !args.is_empty() => {
                let mut name: Option<String> = None;
                for (arg, _) in args {
                    let arg_name = rec(fn_ir, *arg, seen)?;
                    match &name {
                        None => name = Some(arg_name),
                        Some(prev) if prev == &arg_name => {}
                        Some(_) => return None,
                    }
                }
                name
            }
            _ => None,
        }
    }

    rec(fn_ir, iv_phi, &mut FxHashSet::default())
}

pub(super) fn is_iv_seed_expr(
    fn_ir: &FnIR,
    vid: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> bool {
    let vid = canonical_value(fn_ir, vid);
    if vid == iv_phi {
        return false;
    }
    if !seen_vals.insert(vid) {
        return false;
    }
    match &fn_ir.values[vid].kind {
        ValueKind::Const(_) | ValueKind::Param { .. } => true,
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let floor_like = matches!(callee.as_str(), "floor" | "ceiling" | "trunc");
            let single_positional = args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_seed_expr(fn_ir, args[0], iv_phi, seen_vals, seen_vars)
        }
        ValueKind::Load { var } => {
            if !seen_vars.insert(var.to_string()) {
                return false;
            }
            let mut found = false;
            for bb in &fn_ir.blocks {
                for ins in &bb.instrs {
                    let Instr::Assign { dst, src, .. } = ins else {
                        continue;
                    };
                    if dst != var {
                        continue;
                    }
                    found = true;
                    if !is_iv_seed_expr(fn_ir, *src, iv_phi, seen_vals, seen_vars) {
                        seen_vars.remove(var);
                        return false;
                    }
                }
            }
            seen_vars.remove(var);
            found
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == vid {
                    return false;
                }
                return is_iv_seed_expr(fn_ir, first, iv_phi, seen_vals, seen_vars);
            }
            args.iter()
                .all(|(v, _)| is_iv_seed_expr(fn_ir, *v, iv_phi, seen_vals, seen_vars))
        }
        _ => false,
    }
}

pub(super) fn load_var_is_floor_like_iv(
    fn_ir: &FnIR,
    var: &str,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> bool {
    fn is_seed_expr(fn_ir: &FnIR, src: ValueId) -> bool {
        matches!(
            fn_ir.values[canonical_value(fn_ir, src)].kind,
            ValueKind::Const(_) | ValueKind::Param { .. }
        )
    }

    if !seen_vars.insert(var.to_string()) {
        return false;
    }
    let mut found = false;
    let mut all_match = true;
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            // Ignore non-recursive seeds/reinitializations like `ii <- 0`.
            if is_seed_expr(fn_ir, *src) {
                continue;
            }
            found = true;
            if !is_floor_like_iv_expr(fn_ir, *src, iv_phi, seen_vals, seen_vars) {
                all_match = false;
                break;
            }
        }
        if !all_match {
            break;
        }
    }
    seen_vars.remove(var);
    found && all_match
}

pub(super) fn is_floor_like_iv_expr(
    fn_ir: &FnIR,
    src: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> bool {
    let src = canonical_value(fn_ir, src);
    match &fn_ir.values[src].kind {
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let floor_like = matches!(callee.as_str(), "floor" | "ceiling" | "trunc");
            let single_positional = args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_equivalent_rec(fn_ir, args[0], iv_phi, seen_vals, seen_vars)
        }
        ValueKind::Load { var } => {
            load_var_is_floor_like_iv(fn_ir, var, iv_phi, seen_vals, seen_vars)
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == src {
                    return false;
                }
                return is_floor_like_iv_expr(fn_ir, first, iv_phi, seen_vals, seen_vars);
            }
            let mut saw_progress = false;
            for (v, _) in args {
                if is_iv_seed_expr(
                    fn_ir,
                    *v,
                    iv_phi,
                    &mut FxHashSet::default(),
                    &mut FxHashSet::default(),
                ) {
                    continue;
                }
                saw_progress = true;
                if !is_floor_like_iv_expr(fn_ir, *v, iv_phi, seen_vals, seen_vars) {
                    return false;
                }
            }
            saw_progress
        }
        _ => false,
    }
}

pub(super) fn is_value_equivalent(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    if a == b {
        return true;
    }
    if canonical_value(fn_ir, a) == canonical_value(fn_ir, b) {
        return true;
    }
    match (&fn_ir.values[a].kind, &fn_ir.values[b].kind) {
        (ValueKind::Load { var: va }, ValueKind::Load { var: vb }) => va == vb,
        (ValueKind::Load { var }, ValueKind::Phi { args }) if args.is_empty() => {
            fn_ir.values[b].origin_var.as_deref() == Some(var.as_str())
        }
        (ValueKind::Phi { args }, ValueKind::Load { var }) if args.is_empty() => {
            fn_ir.values[a].origin_var.as_deref() == Some(var.as_str())
        }
        _ => false,
    }
}

pub(super) fn canonical_value(fn_ir: &FnIR, mut vid: ValueId) -> ValueId {
    let mut seen = FxHashSet::default();
    loop {
        if !seen.insert(vid) {
            return vid;
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Phi { args } if !args.is_empty() => {
                let first = args[0].0;
                if args.iter().all(|(v, _)| *v == first) {
                    vid = first;
                    continue;
                }
                let mut unique_non_self = FxHashSet::default();
                for (v, _) in args {
                    if *v != vid {
                        unique_non_self.insert(*v);
                    }
                }
                if let Some(unique_non_self_vid) = (unique_non_self.len() == 1)
                    .then(|| unique_non_self.iter().next().copied())
                    .flatten()
                {
                    // loop-invariant self-phi: v = phi(seed, v) -> seed
                    vid = unique_non_self_vid;
                    continue;
                }
            }
            _ => {}
        }
        return vid;
    }
}

pub(super) fn should_hoist_callmap_arg_expr(fn_ir: &FnIR, vid: ValueId) -> bool {
    !matches!(
        &fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. }
    )
}

pub(super) fn next_callmap_tmp_var(fn_ir: &FnIR, prefix: &str) -> VarId {
    let mut idx = 0usize;
    loop {
        let candidate = format!(".tachyon_{}_{}", prefix, idx);
        if fn_ir.params.iter().all(|p| p != &candidate) && !has_any_var_binding(fn_ir, &candidate) {
            return candidate;
        }
        idx += 1;
    }
}

pub(super) fn maybe_hoist_callmap_arg_expr(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    val: ValueId,
    arg_index: usize,
) -> ValueId {
    let val = resolve_materialized_value(fn_ir, val);
    if !should_hoist_callmap_arg_expr(fn_ir, val) {
        return val;
    }
    let var = next_callmap_tmp_var(fn_ir, &format!("callmap_arg{}", arg_index));
    let span = fn_ir.values[val].span;
    let facts = fn_ir.values[val].facts;
    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: var.clone(),
        src: val,
        span: crate::utils::Span::dummy(),
    });
    fn_ir.add_value(ValueKind::Load { var: var.clone() }, span, facts, Some(var))
}

pub(super) fn hoist_vector_expr_temp(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    val: ValueId,
    prefix: &str,
) -> ValueId {
    let val = resolve_materialized_value(fn_ir, val);
    if !should_hoist_callmap_arg_expr(fn_ir, val) {
        return val;
    }
    let var = next_callmap_tmp_var(fn_ir, prefix);
    let span = fn_ir.values[val].span;
    let facts = fn_ir.values[val].facts;
    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: var.clone(),
        src: val,
        span: crate::utils::Span::dummy(),
    });
    fn_ir.add_value(ValueKind::Load { var: var.clone() }, span, facts, Some(var))
}

pub(super) fn resolve_materialized_value(fn_ir: &mut FnIR, vid: ValueId) -> ValueId {
    let c = canonical_value(fn_ir, vid);
    if let ValueKind::Phi { args } = &fn_ir.values[c].kind
        && args.is_empty()
        && let Some(var) = fn_ir.values[c].origin_var.clone()
    {
        return fn_ir.add_value(
            ValueKind::Load { var: var.clone() },
            fn_ir.values[c].span,
            fn_ir.values[c].facts,
            Some(var),
        );
    }
    c
}

pub(super) fn resolve_load_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
    let mut cur = canonical_value(fn_ir, vid);
    let mut seen = FxHashSet::default();
    while seen.insert(cur) {
        if let ValueKind::Load { var } = &fn_ir.values[cur].kind
            && let Some(src) = unique_assign_source(fn_ir, var)
        {
            cur = canonical_value(fn_ir, src);
            continue;
        }
        break;
    }
    cur
}

pub(super) fn resolve_match_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
    let mut cur = canonical_value(fn_ir, vid);
    let mut seen = FxHashSet::default();
    while seen.insert(cur) {
        match &fn_ir.values[cur].kind {
            ValueKind::Load { var } => {
                let Some(src) = unique_assign_source(fn_ir, var) else {
                    break;
                };
                let next = canonical_value(fn_ir, src);
                if next == cur {
                    break;
                }
                cur = next;
            }
            ValueKind::Phi { args } => {
                let folded_non_self_args: Vec<ValueId> = args
                    .iter()
                    .map(|(arg, _)| canonical_value(fn_ir, *arg))
                    .filter(|arg| *arg != cur)
                    .collect();
                if let Some(first) = folded_non_self_args.first().copied()
                    && folded_non_self_args.iter().all(|arg| *arg == first)
                {
                    cur = first;
                    continue;
                }
                if let Some(first) = folded_non_self_args.first().copied()
                    && folded_non_self_args
                        .iter()
                        .all(|arg| fn_ir.values[*arg].kind == fn_ir.values[first].kind)
                {
                    cur = first;
                    continue;
                }
                let Some(var) = fn_ir.values[cur].origin_var.as_deref() else {
                    break;
                };
                let Some(src) = unique_assign_source(fn_ir, var) else {
                    break;
                };
                let next = canonical_value(fn_ir, src);
                if next == cur {
                    break;
                }
                cur = next;
            }
            _ => break,
        }
    }
    cur
}

pub(super) fn unique_assign_source(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
    let mut src: Option<ValueId> = None;
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src: s, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            let s = canonical_value(fn_ir, *s);
            match src {
                None => src = Some(s),
                Some(prev) if canonical_value(fn_ir, prev) == s => {}
                Some(_) => return None,
            }
        }
    }
    src
}

#[cfg(test)]
mod tests {
    use super::super::api::{rank_vector_plans, vector_plan_label};
    use super::super::planning::ExprMapEntry;
    use super::VectorPlan;

    #[test]
    fn rank_prefers_specific_vector_plan_over_generic_map() {
        let mut plans = vec![
            VectorPlan::Map {
                dest: 1,
                src: 2,
                op: crate::syntax::ast::BinOp::Add,
                other: 3,
                shadow_vars: Vec::new(),
            },
            VectorPlan::CondMap {
                dest: 1,
                cond: 4,
                then_val: 5,
                else_val: 6,
                iv_phi: 7,
                start: 8,
                end: 9,
                whole_dest: true,
                shadow_vars: Vec::new(),
            },
        ];
        rank_vector_plans(&mut plans);
        assert_eq!(vector_plan_label(&plans[0]), "cond_map");
    }

    #[test]
    fn rank_prefers_multi_output_expr_map_over_single_expr_map() {
        let mut plans = vec![
            VectorPlan::ExprMap {
                dest: 1,
                expr: 2,
                iv_phi: 3,
                start: 4,
                end: 5,
                whole_dest: true,
                shadow_vars: Vec::new(),
            },
            VectorPlan::MultiExprMap {
                entries: vec![
                    ExprMapEntry {
                        dest: 1,
                        expr: 2,
                        whole_dest: true,
                        shadow_vars: Vec::new(),
                    },
                    ExprMapEntry {
                        dest: 6,
                        expr: 7,
                        whole_dest: true,
                        shadow_vars: Vec::new(),
                    },
                ],
                iv_phi: 3,
                start: 4,
                end: 5,
            },
        ];
        rank_vector_plans(&mut plans);
        assert_eq!(vector_plan_label(&plans[0]), "multi_expr_map");
    }
}
