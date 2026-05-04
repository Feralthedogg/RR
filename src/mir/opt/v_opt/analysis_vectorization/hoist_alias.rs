use super::*;
pub(crate) fn should_hoist_callmap_arg_expr(fn_ir: &FnIR, vid: ValueId) -> bool {
    !matches!(
        &fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. }
    )
}

pub(crate) fn next_callmap_tmp_var(fn_ir: &FnIR, prefix: &str) -> VarId {
    let mut idx = 0usize;
    loop {
        let candidate = format!(".tachyon_{}_{}", prefix, idx);
        if fn_ir.params.iter().all(|p| p != &candidate) && !has_any_var_binding(fn_ir, &candidate) {
            return candidate;
        }
        idx += 1;
    }
}

pub(crate) fn maybe_hoist_callmap_arg_expr(
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

pub(crate) fn hoist_vector_expr_temp(
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

pub(crate) fn resolve_materialized_value(fn_ir: &mut FnIR, vid: ValueId) -> ValueId {
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

pub(crate) fn resolve_load_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
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

pub(crate) fn resolve_match_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
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

pub(crate) fn unique_assign_source(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
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
