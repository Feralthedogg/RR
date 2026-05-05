use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::mir::opt::v_opt) struct ResolvedCallInfo {
    pub callee: String,
    pub builtin_kind: Option<BuiltinKind>,
    pub args: Vec<ValueId>,
}

pub(in crate::mir::opt::v_opt) fn normalize_callee_name(callee: &str) -> &str {
    callee.strip_prefix("base::").unwrap_or(callee)
}

pub(in crate::mir::opt::v_opt) fn proof_budget_exhausted(
    depth: usize,
    seen_vals: &FxHashSet<ValueId>,
    seen_vars: &FxHashSet<String>,
) -> bool {
    depth >= VOPT_PROOF_RECURSION_LIMIT
        || seen_vals.len().saturating_add(seen_vars.len()) >= VOPT_PROOF_VISIT_LIMIT
}

pub(in crate::mir::opt::v_opt) fn resolve_callable_name(
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

pub(in crate::mir::opt::v_opt) fn resolve_call_info(
    fn_ir: &FnIR,
    value: ValueId,
) -> Option<ResolvedCallInfo> {
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

pub(in crate::mir::opt::v_opt) fn call_is_semantically_pure(callee: &str) -> bool {
    effects::call_is_pure(callee) || effects::call_is_pure(normalize_callee_name(callee))
}

pub(in crate::mir::opt::v_opt) fn matrix_access_stride(varying_col: bool) -> MemoryStrideClass {
    if varying_col {
        MemoryStrideClass::Strided
    } else {
        MemoryStrideClass::Contiguous
    }
}

pub(in crate::mir::opt::v_opt) fn array3_access_stride(axis: Axis3D) -> MemoryStrideClass {
    match axis {
        Axis3D::Dim1 => MemoryStrideClass::Contiguous,
        Axis3D::Dim2 | Axis3D::Dim3 => MemoryStrideClass::Strided,
    }
}

pub(in crate::mir::opt::v_opt) fn structured_reduction_stride_allowed(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    stride: MemoryStrideClass,
) -> bool {
    matches!(stride, MemoryStrideClass::Contiguous)
        || estimate_loop_trip_count_hint(fn_ir, lp)
            .is_some_and(|trip| trip <= MAX_STRIDED_REDUCTION_TRIP_HINT)
}

pub(in crate::mir::opt::v_opt) fn last_assign_to_var_in_block(
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

pub(in crate::mir::opt::v_opt) fn block_successors(
    fn_ir: &FnIR,
    bid: BlockId,
) -> [Option<BlockId>; 2] {
    match fn_ir.blocks[bid].term {
        Terminator::Goto(target) => [Some(target), None],
        Terminator::If {
            then_bb, else_bb, ..
        } => [Some(then_bb), Some(else_bb)],
        Terminator::Return(_) | Terminator::Unreachable => [None, None],
    }
}

pub(in crate::mir::opt::v_opt) fn loop_has_inner_branch(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    for bid in &lp.body {
        if *bid == lp.header {
            continue;
        }
        let Terminator::If {
            then_bb, else_bb, ..
        } = fn_ir.blocks[*bid].term
        else {
            continue;
        };
        if lp.body.contains(&then_bb) && lp.body.contains(&else_bb) {
            return true;
        }
    }
    false
}

pub(in crate::mir::opt::v_opt) fn block_reaches_before_merge(
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

pub(in crate::mir::opt::v_opt) fn collect_if_ancestors_with_distance(
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
