use super::*;

pub(in crate::mir::opt::v_opt) fn infer_passthrough_origin_var_from_phi_arms(
    fn_ir: &FnIR,
    args: &[(ValueId, BlockId)],
) -> Option<String> {
    let mut found: Option<String> = None;
    for (arg, _) in args {
        let ValueKind::Load { var } = &fn_ir.values[canonical_value(fn_ir, *arg)].kind else {
            continue;
        };
        match &found {
            None => found = Some(var.clone()),
            Some(prev) if prev == var => {}
            Some(_) => return None,
        }
    }
    found
}

pub(in crate::mir::opt::v_opt) fn is_int_index_vector_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    fn rec(fn_ir: &FnIR, vid: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        let vid = canonical_value(fn_ir, vid);
        if !seen.insert(vid) {
            return false;
        }
        let v = &fn_ir.values[vid];
        if (v.value_ty.shape == ShapeTy::Vector && v.value_ty.prim == PrimTy::Int)
            || v.facts
                .has(crate::mir::flow::Facts::IS_VECTOR | crate::mir::flow::Facts::INT_SCALAR)
        {
            return true;
        }
        match &v.kind {
            ValueKind::Call { callee, args, .. } => match callee.as_str() {
                "rr_index_vec_floor" => true,
                "rr_index1_read_vec" | "rr_index1_read_vec_floor" | "rr_gather" => args
                    .first()
                    .copied()
                    .is_some_and(|base| rec(fn_ir, base, seen)),
                _ => false,
            },
            ValueKind::Index1D { base, .. } => rec(fn_ir, *base, seen),
            ValueKind::Phi { args } if !args.is_empty() => {
                args.iter().all(|(arg, _)| rec(fn_ir, *arg, seen))
            }
            _ => false,
        }
    }

    rec(fn_ir, vid, &mut FxHashSet::default())
}

pub(in crate::mir::opt::v_opt) fn is_scalar_broadcast_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    let root = canonical_value(fn_ir, vid);
    let v = &fn_ir.values[root];
    matches!(v.kind, ValueKind::Const(_))
        || v.value_ty.shape == ShapeTy::Scalar
        || v.facts.has(Facts::INT_SCALAR)
        || v.facts.has(Facts::BOOL_SCALAR)
}

pub(in crate::mir::opt::v_opt) fn value_is_definitely_scalar_like(
    fn_ir: &FnIR,
    vid: ValueId,
) -> bool {
    let root = canonical_value(fn_ir, vid);
    let value = &fn_ir.values[root];
    value.value_ty.shape == ShapeTy::Scalar
        || value.facts.has(Facts::INT_SCALAR)
        || value.facts.has(Facts::BOOL_SCALAR)
        || matches!(
            value.kind,
            ValueKind::Const(_)
                | ValueKind::Param { .. }
                | ValueKind::Load { .. }
                | ValueKind::Len { .. }
        ) && vector_length_key(fn_ir, root).is_none()
}

pub(in crate::mir::opt::v_opt) fn has_assignment_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> bool {
    lp.body.iter().any(|bid| {
        fn_ir.blocks[*bid].instrs.iter().any(|ins| match ins {
            Instr::Assign { dst, .. } => dst == var,
            _ => false,
        })
    })
}

pub(in crate::mir::opt::v_opt) fn expr_has_unstable_loop_local_load(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    root: ValueId,
) -> bool {
    fn rec(fn_ir: &FnIR, lp: &LoopInfo, root: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Load { var } => has_non_passthrough_assignment_in_loop(fn_ir, lp, var),
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, lp, *lhs, seen) || rec(fn_ir, lp, *rhs, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, lp, *rhs, seen),
            ValueKind::RecordLit { fields } => {
                fields.iter().any(|(_, value)| rec(fn_ir, lp, *value, seen))
            }
            ValueKind::FieldGet { base, .. } => rec(fn_ir, lp, *base, seen),
            ValueKind::FieldSet { base, value, .. } => {
                rec(fn_ir, lp, *base, seen) || rec(fn_ir, lp, *value, seen)
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                args.iter().any(|arg| rec(fn_ir, lp, *arg, seen))
            }
            ValueKind::Phi { args } => args.iter().any(|(arg, _)| rec(fn_ir, lp, *arg, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => rec(fn_ir, lp, *base, seen),
            ValueKind::Range { start, end } => {
                rec(fn_ir, lp, *start, seen) || rec(fn_ir, lp, *end, seen)
            }
            ValueKind::Index1D { base, idx, .. } => {
                rec(fn_ir, lp, *base, seen) || rec(fn_ir, lp, *idx, seen)
            }
            ValueKind::Index2D { base, r, c } => {
                rec(fn_ir, lp, *base, seen) || rec(fn_ir, lp, *r, seen) || rec(fn_ir, lp, *c, seen)
            }
            ValueKind::Index3D { base, i, j, k } => {
                rec(fn_ir, lp, *base, seen)
                    || rec(fn_ir, lp, *i, seen)
                    || rec(fn_ir, lp, *j, seen)
                    || rec(fn_ir, lp, *k, seen)
            }
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
        }
    }

    rec(fn_ir, lp, root, &mut FxHashSet::default())
}

pub(in crate::mir::opt::v_opt) fn expr_has_ambiguous_loop_local_load(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    root: ValueId,
) -> bool {
    fn rec(fn_ir: &FnIR, lp: &LoopInfo, root: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Load { var } => {
                let can_materialize_state_chain = value_use_block_in_loop(fn_ir, lp, root)
                    .and_then(|use_bb| nearest_state_phi_value_in_loop(fn_ir, lp, var, use_bb))
                    .and_then(|phi_root| {
                        collect_independent_if_state_chain(fn_ir, lp, phi_root, var)
                    })
                    .is_some();
                has_non_passthrough_assignment_in_loop(fn_ir, lp, var)
                    && unique_assign_source_in_loop(fn_ir, lp, var).is_none()
                    && merged_assign_source_in_loop(fn_ir, lp, var).is_none()
                    && unique_origin_phi_value_in_loop(fn_ir, lp, var).is_none()
                    && !can_materialize_state_chain
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, lp, *lhs, seen) || rec(fn_ir, lp, *rhs, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, lp, *rhs, seen),
            ValueKind::RecordLit { fields } => {
                fields.iter().any(|(_, value)| rec(fn_ir, lp, *value, seen))
            }
            ValueKind::FieldGet { base, .. } => rec(fn_ir, lp, *base, seen),
            ValueKind::FieldSet { base, value, .. } => {
                rec(fn_ir, lp, *base, seen) || rec(fn_ir, lp, *value, seen)
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                args.iter().any(|arg| rec(fn_ir, lp, *arg, seen))
            }
            ValueKind::Phi { args } => args.iter().any(|(arg, _)| rec(fn_ir, lp, *arg, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => rec(fn_ir, lp, *base, seen),
            ValueKind::Range { start, end } => {
                rec(fn_ir, lp, *start, seen) || rec(fn_ir, lp, *end, seen)
            }
            ValueKind::Index1D { base, idx, .. } => {
                rec(fn_ir, lp, *base, seen) || rec(fn_ir, lp, *idx, seen)
            }
            ValueKind::Index2D { base, r, c } => {
                rec(fn_ir, lp, *base, seen) || rec(fn_ir, lp, *r, seen) || rec(fn_ir, lp, *c, seen)
            }
            ValueKind::Index3D { base, i, j, k } => {
                rec(fn_ir, lp, *base, seen)
                    || rec(fn_ir, lp, *i, seen)
                    || rec(fn_ir, lp, *j, seen)
                    || rec(fn_ir, lp, *k, seen)
            }
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
        }
    }

    rec(fn_ir, lp, root, &mut FxHashSet::default())
}

pub(in crate::mir::opt::v_opt) fn has_non_passthrough_assignment_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> bool {
    lp.body.iter().any(|bid| {
        fn_ir.blocks[*bid].instrs.iter().any(|ins| {
            let Instr::Assign { dst, src, .. } = ins else {
                return false;
            };
            if dst != var {
                return false;
            }
            let src = preserve_phi_value(fn_ir, *src);
            !matches!(
                &fn_ir.values[src].kind,
                ValueKind::Load { var: load_var } if load_var == var
            ) && !matches!(&fn_ir.values[src].kind, ValueKind::Param { .. })
                && !matches!(
                    &fn_ir.values[src].kind,
                    ValueKind::Phi { args }
                        if !args.is_empty()
                            && fn_ir.values[src].origin_var.as_deref() == Some(var)
                )
        })
    })
}

pub(in crate::mir::opt::v_opt) fn unique_assign_source_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> Option<ValueId> {
    let mut src: Option<ValueId> = None;
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
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

pub(in crate::mir::opt::v_opt) fn merged_assign_source_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> Option<ValueId> {
    let mut assigned = Vec::new();
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst == var {
                assigned.push(canonical_value(fn_ir, *src));
            }
        }
    }
    assigned.sort_unstable();
    assigned.dedup();

    let mut phi_srcs = assigned
        .iter()
        .copied()
        .filter(
            |src| matches!(&fn_ir.values[*src].kind, ValueKind::Phi { args } if !args.is_empty()),
        )
        .filter(|src| {
            fn_ir.values[*src]
                .phi_block
                .is_some_and(|bb| lp.body.contains(&bb))
        });
    let phi_src = phi_srcs.next()?;
    if phi_srcs.next().is_some() {
        return None;
    }

    let ValueKind::Phi { args } = &fn_ir.values[phi_src].kind else {
        return None;
    };
    let phi_args: FxHashSet<ValueId> = args
        .iter()
        .map(|(arg, _)| canonical_value(fn_ir, *arg))
        .collect();
    if assigned
        .iter()
        .all(|src| *src == phi_src || phi_args.contains(src))
    {
        Some(phi_src)
    } else {
        None
    }
}

pub(in crate::mir::opt::v_opt) fn unique_origin_phi_value_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> Option<ValueId> {
    let mut found: Option<ValueId> = None;
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() || value.origin_var.as_deref() != Some(var) {
            continue;
        }
        if !value.phi_block.is_some_and(|bb| lp.body.contains(&bb)) {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        match found {
            None => found = Some(vid),
            Some(prev) if canonical_value(fn_ir, prev) == vid => {}
            Some(_) => return None,
        }
    }
    found
}

pub(in crate::mir::opt::v_opt) fn phi_loads_same_var(
    fn_ir: &FnIR,
    args: &[(ValueId, BlockId)],
) -> bool {
    let mut found: Option<&str> = None;
    for (arg, _) in args {
        let ValueKind::Load { var } = &fn_ir.values[canonical_value(fn_ir, *arg)].kind else {
            return false;
        };
        match found {
            None => found = Some(var.as_str()),
            Some(prev) if prev == var => {}
            Some(_) => return false,
        }
    }
    found.is_some()
}

pub(in crate::mir::opt::v_opt) fn value_use_block_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    vid: ValueId,
) -> Option<BlockId> {
    let vid = canonical_value(fn_ir, vid);
    let mut use_blocks: Vec<Option<BlockId>> = vec![None; fn_ir.values.len()];
    let mut worklist: Vec<(ValueId, BlockId)> = Vec::new();
    let mut body: Vec<BlockId> = lp.body.iter().copied().collect();
    body.sort_unstable();
    for bid in body {
        for ins in &fn_ir.blocks[bid].instrs {
            match ins {
                Instr::Assign { src, .. } => worklist.push((canonical_value(fn_ir, *src), bid)),
                Instr::Eval { val, .. } => worklist.push((canonical_value(fn_ir, *val), bid)),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    worklist.push((canonical_value(fn_ir, *base), bid));
                    worklist.push((canonical_value(fn_ir, *idx), bid));
                    worklist.push((canonical_value(fn_ir, *val), bid));
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    worklist.push((canonical_value(fn_ir, *base), bid));
                    worklist.push((canonical_value(fn_ir, *r), bid));
                    worklist.push((canonical_value(fn_ir, *c), bid));
                    worklist.push((canonical_value(fn_ir, *val), bid));
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    worklist.push((canonical_value(fn_ir, *base), bid));
                    worklist.push((canonical_value(fn_ir, *i), bid));
                    worklist.push((canonical_value(fn_ir, *j), bid));
                    worklist.push((canonical_value(fn_ir, *k), bid));
                    worklist.push((canonical_value(fn_ir, *val), bid));
                }
                Instr::UnsafeRBlock { .. } => {}
            }
        }
        match &fn_ir.blocks[bid].term {
            Terminator::If { cond, .. } => worklist.push((canonical_value(fn_ir, *cond), bid)),
            Terminator::Return(Some(ret)) => worklist.push((canonical_value(fn_ir, *ret), bid)),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    while let Some((curr, bid)) = worklist.pop() {
        if let Some(prev) = use_blocks[curr]
            && bid >= prev
        {
            continue;
        }
        use_blocks[curr] = Some(bid);
        match &fn_ir.values[curr].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                worklist.push((canonical_value(fn_ir, *lhs), bid));
                worklist.push((canonical_value(fn_ir, *rhs), bid));
            }
            ValueKind::Unary { rhs, .. } => {
                worklist.push((canonical_value(fn_ir, *rhs), bid));
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    worklist.push((canonical_value(fn_ir, *value), bid));
                }
            }
            ValueKind::FieldGet { base, .. } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
            }
            ValueKind::FieldSet { base, value, .. } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
                worklist.push((canonical_value(fn_ir, *value), bid));
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    worklist.push((canonical_value(fn_ir, *arg), bid));
                }
            }
            ValueKind::Phi { args } => {
                for (arg, _) in args {
                    worklist.push((canonical_value(fn_ir, *arg), bid));
                }
            }
            ValueKind::Index1D { base, idx, .. } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
                worklist.push((canonical_value(fn_ir, *idx), bid));
            }
            ValueKind::Index2D { base, r, c } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
                worklist.push((canonical_value(fn_ir, *r), bid));
                worklist.push((canonical_value(fn_ir, *c), bid));
            }
            ValueKind::Index3D { base, i, j, k } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
                worklist.push((canonical_value(fn_ir, *i), bid));
                worklist.push((canonical_value(fn_ir, *j), bid));
                worklist.push((canonical_value(fn_ir, *k), bid));
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
            }
            ValueKind::Range { start, end } => {
                worklist.push((canonical_value(fn_ir, *start), bid));
                worklist.push((canonical_value(fn_ir, *end), bid));
            }
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => {}
        }
    }
    use_blocks[vid]
}

pub(in crate::mir::opt::v_opt) fn nearest_origin_phi_value_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    use_bb: BlockId,
) -> Option<ValueId> {
    let mut best: Option<(BlockId, ValueId)> = None;
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() || value.origin_var.as_deref() != Some(var) {
            continue;
        }
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) || phi_bb >= use_bb {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        match best {
            None => best = Some((phi_bb, vid)),
            Some((best_bb, _)) if phi_bb > best_bb => best = Some((phi_bb, vid)),
            Some((best_bb, best_vid))
                if phi_bb == best_bb && canonical_value(fn_ir, best_vid) != vid =>
            {
                return None;
            }
            Some(_) => {}
        }
    }
    best.map(|(_, vid)| vid)
}

pub(in crate::mir::opt::v_opt) fn nearest_visiting_origin_phi_value_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    use_bb: BlockId,
    visiting: &FxHashSet<ValueId>,
) -> Option<ValueId> {
    let mut best: Option<(BlockId, ValueId)> = None;
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() || value.origin_var.as_deref() != Some(var) {
            continue;
        }
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) || phi_bb > use_bb {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        if !visiting.contains(&vid) {
            continue;
        }
        match best {
            None => best = Some((phi_bb, vid)),
            Some((best_bb, _)) if phi_bb > best_bb => best = Some((phi_bb, vid)),
            Some((best_bb, best_vid))
                if phi_bb == best_bb && canonical_value(fn_ir, best_vid) != vid =>
            {
                return None;
            }
            Some(_) => {}
        }
    }
    best.map(|(_, vid)| vid)
}

pub(in crate::mir::opt::v_opt) fn unique_assign_source_reaching_block_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    target_bb: BlockId,
) -> Option<ValueId> {
    let preds = build_pred_map(fn_ir);
    let mut seen = FxHashSet::default();
    let mut stack: Vec<BlockId> = preds
        .get(&target_bb)
        .into_iter()
        .flat_map(|ps| ps.iter().copied())
        .filter(|bb| lp.body.contains(bb))
        .collect();
    let mut src: Option<ValueId> = None;
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        for ins in &fn_ir.blocks[bid].instrs {
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
        if let Some(ps) = preds.get(&bid) {
            for pred in ps {
                if lp.body.contains(pred) {
                    stack.push(*pred);
                }
            }
        }
    }
    src
}

pub(in crate::mir::opt::v_opt) fn unwrap_vector_condition_value(
    fn_ir: &FnIR,
    root: ValueId,
) -> ValueId {
    let root = canonical_value(fn_ir, root);
    match &fn_ir.values[root].kind {
        ValueKind::Call { callee, args, .. }
            if matches!(callee.as_str(), "rr_truthy1" | "rr_bool") && !args.is_empty() =>
        {
            canonical_value(fn_ir, args[0])
        }
        _ => root,
    }
}

pub(in crate::mir::opt::v_opt) fn is_comparison_op(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    )
}
