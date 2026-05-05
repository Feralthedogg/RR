use super::*;

pub(in crate::mir::opt::v_opt) fn intrinsic_for_call(
    callee: &str,
    arity: usize,
) -> Option<IntrinsicOp> {
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

pub(in crate::mir::opt::v_opt) fn call_map_profit_guard_supported(
    callee: &str,
    arity: usize,
) -> bool {
    let callee = normalize_callee_name(callee);
    is_builtin_vector_safe_call(callee, arity) && !matches!(callee, "seq_len" | "sum" | "mean")
}

pub(in crate::mir::opt::v_opt) fn intrinsic_runtime_helper_cost(op: IntrinsicOp) -> u32 {
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

pub(in crate::mir::opt::v_opt) fn call_runtime_helper_cost(callee: &str) -> u32 {
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

pub(in crate::mir::opt::v_opt) fn expr_runtime_helper_cost(
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

pub(in crate::mir::opt::v_opt) fn estimate_call_map_helper_cost(
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

pub(in crate::mir::opt::v_opt) fn choose_call_map_lowering(
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

pub(in crate::mir::opt::v_opt) fn const_index_like_value(
    fn_ir: &FnIR,
    vid: ValueId,
) -> Option<i64> {
    match &fn_ir.values[canonical_value(fn_ir, vid)].kind {
        ValueKind::Const(Lit::Int(n)) => Some(*n),
        ValueKind::Const(Lit::Float(f)) if f.is_finite() && f.fract() == 0.0 => Some(*f as i64),
        _ => None,
    }
}

pub(in crate::mir::opt::v_opt) fn estimate_loop_trip_count_hint(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<u64> {
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

pub(in crate::mir::opt::v_opt) fn estimate_vector_plan_helper_cost(
    fn_ir: &FnIR,
    plan: &VectorPlan,
) -> u32 {
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
