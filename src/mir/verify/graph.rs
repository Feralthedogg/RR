fn is_loop_header_forwarder_pred(
    fn_ir: &FnIR,
    preds: &[Vec<BlockId>],
    reachable: &FxHashSet<BlockId>,
    header: BlockId,
    body: &FxHashSet<BlockId>,
    pred: BlockId,
) -> bool {
    if body.contains(&pred) {
        return false;
    }
    if !matches!(fn_ir.blocks[pred].term, Terminator::Goto(target) if target == header) {
        return false;
    }
    let incoming: Vec<BlockId> = preds
        .get(pred)
        .into_iter()
        .flatten()
        .copied()
        .filter(|pp| reachable.contains(pp))
        .collect();
    !incoming.is_empty() && incoming.iter().all(|pp| body.contains(pp))
}

pub fn verify_emittable_ir(fn_ir: &FnIR) -> Result<(), VerifyError> {
    // Proof correspondence:
    // `VerifyIrExecutableLite` / `VerifyIrRustErrorLite` approximate the final
    // executable check that no reachable `Phi` survives into codegen-ready MIR.
    verify_ir(fn_ir)?;
    let reachable = compute_reachable(fn_ir);
    let used_values = collect_used_values(fn_ir, &reachable);
    for vid in used_values {
        if matches!(fn_ir.values[vid].kind, ValueKind::Phi { .. }) {
            return Err(VerifyError::ReachablePhi { value: vid });
        }
    }
    Ok(())
}

fn check_val(fn_ir: &FnIR, vid: ValueId) -> Result<(), VerifyError> {
    if vid >= fn_ir.values.len() {
        Err(VerifyError::BadValue(vid))
    } else {
        Ok(())
    }
}

fn check_blk(fn_ir: &FnIR, bid: BlockId) -> Result<(), VerifyError> {
    if bid >= fn_ir.blocks.len() {
        Err(VerifyError::BadBlock(bid))
    } else {
        Ok(())
    }
}

fn param_runtime_var_name(fn_ir: &FnIR, index: usize) -> Option<VarId> {
    for v in &fn_ir.values {
        if let ValueKind::Param { index: i } = v.kind
            && i == index
        {
            if let Some(name) = &v.origin_var {
                return Some(name.clone());
            }
            break;
        }
    }
    fn_ir.params.get(index).cloned()
}

fn fn_is_self_recursive(fn_ir: &FnIR) -> bool {
    fn_ir.values.iter().any(|value| {
        matches!(
            &value.kind,
            ValueKind::Call { callee, .. } if callee == &fn_ir.name
        )
    })
}

fn value_has_direct_self_reference(vid: ValueId, kind: &ValueKind) -> bool {
    match kind {
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => false,
        ValueKind::Len { base }
        | ValueKind::Indices { base }
        | ValueKind::Unary { rhs: base, .. }
        | ValueKind::FieldGet { base, .. } => *base == vid,
        ValueKind::Range { start, end } => *start == vid || *end == vid,
        ValueKind::Binary { lhs, rhs, .. } => *lhs == vid || *rhs == vid,
        ValueKind::Phi { args } => args.iter().any(|(arg, _)| *arg == vid),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args.contains(&vid),
        ValueKind::RecordLit { fields } => fields.iter().any(|(_, value)| *value == vid),
        ValueKind::FieldSet { base, value, .. } => *base == vid || *value == vid,
        ValueKind::Index1D { base, idx, .. } => *base == vid || *idx == vid,
        ValueKind::Index2D { base, r, c } => *base == vid || *r == vid || *c == vid,
        ValueKind::Index3D { base, i, j, k } => *base == vid || *i == vid || *j == vid || *k == vid,
    }
}

fn block_reaches(fn_ir: &FnIR, start: BlockId, target: BlockId) -> bool {
    if start == target {
        return true;
    }
    let mut seen = FxHashSet::default();
    let mut stack = vec![start];
    seen.insert(start);
    while let Some(bid) = stack.pop() {
        let succs = match fn_ir.blocks[bid].term {
            Terminator::Goto(next) => [Some(next), None],
            Terminator::If {
                then_bb, else_bb, ..
            } => [Some(then_bb), Some(else_bb)],
            Terminator::Return(_) | Terminator::Unreachable => [None, None],
        };
        for succ in succs.into_iter().flatten() {
            if succ == target {
                return true;
            }
            if seen.insert(succ) {
                stack.push(succ);
            }
        }
    }
    false
}

fn detect_non_phi_value_cycle(fn_ir: &FnIR) -> Option<ValueId> {
    fn visit(fn_ir: &FnIR, vid: ValueId, colors: &mut [u8]) -> Option<ValueId> {
        if matches!(fn_ir.values[vid].kind, ValueKind::Phi { .. }) {
            colors[vid] = 2;
            return None;
        }
        match colors[vid] {
            1 => return Some(vid),
            2 => return None,
            _ => {}
        }
        colors[vid] = 1;
        for dep in non_phi_dependencies(&fn_ir.values[vid].kind) {
            if dep >= fn_ir.values.len() || matches!(fn_ir.values[dep].kind, ValueKind::Phi { .. })
            {
                continue;
            }
            if let Some(cycle) = visit(fn_ir, dep, colors) {
                return Some(cycle);
            }
        }
        colors[vid] = 2;
        None
    }

    let mut colors = vec![0u8; fn_ir.values.len()];
    for vid in 0..fn_ir.values.len() {
        if matches!(fn_ir.values[vid].kind, ValueKind::Phi { .. }) || colors[vid] == 2 {
            continue;
        }
        if let Some(value) = visit(fn_ir, vid, &mut colors) {
            return Some(value);
        }
    }
    None
}

fn non_phi_dependencies(kind: &ValueKind) -> Vec<ValueId> {
    // Proof correspondence:
    // `VerifyIrChildDepsSubset` fixes the reduced non-`Phi` child-edge
    // extraction shape for unary wrappers, binary/range pairs, `Call` /
    // `Intrinsic` arg lists, `RecordLit` field values, and `Index*` nodes;
    // `VerifyIrConsumerGraphSubset` then lifts those extracted child ids into
    // a reduced seen/fuel graph closer to the recursive traversal below.
    match kind {
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => Vec::new(),
        ValueKind::Len { base }
        | ValueKind::Indices { base }
        | ValueKind::Unary { rhs: base, .. }
        | ValueKind::FieldGet { base, .. } => vec![*base],
        ValueKind::Range { start, end } => vec![*start, *end],
        ValueKind::Binary { lhs, rhs, .. } => vec![*lhs, *rhs],
        ValueKind::Phi { .. } => Vec::new(),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args.clone(),
        ValueKind::RecordLit { fields } => fields.iter().map(|(_, value)| *value).collect(),
        ValueKind::FieldSet { base, value, .. } => vec![*base, *value],
        ValueKind::Index1D { base, idx, .. } => vec![*base, *idx],
        ValueKind::Index2D { base, r, c } => vec![*base, *r, *c],
        ValueKind::Index3D { base, i, j, k } => vec![*base, *i, *j, *k],
    }
}

fn depends_on_phi_in_block_except(
    fn_ir: &FnIR,
    root: ValueId,
    phi_block: BlockId,
    exempt_phi: ValueId,
) -> bool {
    // Proof correspondence:
    // `VerifyIrValueDepsWalkSubset` fixes the reduced full `value_dependencies`
    // shape, including `Phi` arg lists, and lifts it into a reduced seen/fuel
    // stack walk approximating this helper's exempt-phi search;
    // `VerifyIrValueTableWalkSubset` then rephrases that walk over an explicit
    // value-table lookup with stored `phi_block` metadata closer to `FnIR.values`,
    // while `VerifyIrValueKindTableSubset` refines those rows to actual
    // `ValueKind`-named payload constructors.
    let mut seen = FxHashSet::default();
    let mut stack = vec![root];

    while let Some(vid) = stack.pop() {
        if vid >= fn_ir.values.len() || !seen.insert(vid) {
            continue;
        }
        let value = &fn_ir.values[vid];
        if matches!(value.kind, ValueKind::Phi { .. }) {
            // Check: is this a different Phi at the same join block?
            if value.phi_block == Some(phi_block) && vid != exempt_phi {
                return true;
            }
            // Stop here: do not follow through Phi args.
            // Phi args come from different control-flow paths, and
            // traversing them (especially across loop back-edges)
            // produces false positives where cross-variable loop
            // dependencies are misidentified as same-block Phi cycles.
            continue;
        }
        for dep in non_phi_dependencies(&value.kind) {
            stack.push(dep);
        }
    }
    false
}

fn infer_phi_owner_block(fn_ir: &FnIR, args: &[(ValueId, BlockId)]) -> Option<BlockId> {
    // Proof correspondence:
    // `VerifyIrStructLite` fixes the reduced owner/join discipline,
    // `VerifyIrValueEnvSubset` models the predecessor-selected value
    // environment that this inferred join block is meant to govern,
    // `VerifyIrArgEnvSubset` extends that reduced env story to arg/field-list
    // consumers under the selected predecessor,
    // `VerifyIrArgEnvTraversalSubset` adds reduced missing-use scans over
    // those selected-edge consumers, and
    // `VerifyIrEnvScanComposeSubset` packages those env-selected scan facts
    // alongside the reduced value-kind scan facts.
    fn successors(fn_ir: &FnIR, bid: BlockId) -> Vec<BlockId> {
        if bid >= fn_ir.blocks.len() {
            return Vec::new();
        }
        match fn_ir.blocks[bid].term {
            Terminator::Goto(target) => vec![target],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![then_bb, else_bb],
            Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
        }
    }

    let (_, first_pred) = args.first().copied()?;
    let mut common: FxHashSet<BlockId> = successors(fn_ir, first_pred).into_iter().collect();
    for (_, pred) in args.iter().skip(1) {
        let succs: FxHashSet<BlockId> = successors(fn_ir, *pred).into_iter().collect();
        common.retain(|bid| succs.contains(bid));
        if common.is_empty() {
            return None;
        }
    }
    if common.len() == 1 {
        common.into_iter().next()
    } else {
        None
    }
}
