fn compute_reachable(fn_ir: &FnIR) -> FxHashSet<BlockId> {
    let mut reachable = FxHashSet::default();
    let mut queue = vec![fn_ir.entry];
    reachable.insert(fn_ir.entry);

    let mut head = 0;
    while head < queue.len() {
        let bid = queue[head];
        head += 1;

        if let Some(blk) = fn_ir.blocks.get(bid) {
            match &blk.term {
                Terminator::Goto(target) => {
                    if reachable.insert(*target) {
                        queue.push(*target);
                    }
                }
                Terminator::If {
                    then_bb, else_bb, ..
                } => {
                    if reachable.insert(*then_bb) {
                        queue.push(*then_bb);
                    }
                    if reachable.insert(*else_bb) {
                        queue.push(*else_bb);
                    }
                }
                _ => {}
            }
        }
    }

    reachable
}

fn compute_must_defined_vars(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
    preds: &[Vec<BlockId>],
) -> (Vec<FxHashSet<VarId>>, Vec<FxHashSet<VarId>>) {
    // Proof correspondence:
    // `VerifyIrMustDefSubset` isolates the reduced predecessor-intersection /
    // local-assign step, `VerifyIrMustDefFixedPointSubset` adds a reduced
    // reachable-pred / one-step iteration model, `VerifyIrMustDefConvergenceSubset`
    // adds reduced stable-seed preservation under iteration, and
    // `VerifyIrFlowLite` packages the resulting use-before-def obligation;
    // this helper is the concrete fixed-point computation over the real CFG
    // predecessor map.
    let universe: FxHashSet<VarId> = fn_ir
        .params
        .iter()
        .cloned()
        .chain(
            fn_ir
                .blocks
                .iter()
                .enumerate()
                .filter(|(bid, _)| reachable.contains(bid))
                .flat_map(|(_, block)| block.instrs.iter())
                .filter_map(|instr| match instr {
                    Instr::Assign { dst, .. } => Some(dst.clone()),
                    _ => None,
                }),
        )
        .collect();
    let entry_defs: FxHashSet<VarId> = fn_ir.params.iter().cloned().collect();

    let mut in_defs = vec![FxHashSet::default(); fn_ir.blocks.len()];
    let mut out_defs = vec![FxHashSet::default(); fn_ir.blocks.len()];
    for bid in 0..fn_ir.blocks.len() {
        if !reachable.contains(&bid) {
            continue;
        }
        in_defs[bid] = if bid == fn_ir.entry {
            entry_defs.clone()
        } else {
            universe.clone()
        };
        out_defs[bid] = universe.clone();
    }

    loop {
        let mut changed = false;
        for bid in 0..fn_ir.blocks.len() {
            if !reachable.contains(&bid) {
                continue;
            }
            let new_in = if bid == fn_ir.entry {
                entry_defs.clone()
            } else {
                let mut reachable_preds = preds[bid]
                    .iter()
                    .copied()
                    .filter(|pred| reachable.contains(pred));
                match reachable_preds.next() {
                    Some(first_pred) => {
                        let mut acc = out_defs[first_pred].clone();
                        for pred in reachable_preds {
                            acc.retain(|var| out_defs[pred].contains(var));
                        }
                        acc
                    }
                    None => FxHashSet::default(),
                }
            };
            if new_in != in_defs[bid] {
                in_defs[bid] = new_in.clone();
                changed = true;
            }

            let mut new_out = new_in;
            for instr in &fn_ir.blocks[bid].instrs {
                if let Instr::Assign { dst, .. } = instr {
                    new_out.insert(dst.clone());
                }
            }
            if new_out != out_defs[bid] {
                out_defs[bid] = new_out;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    (in_defs, out_defs)
}

fn first_undefined_load_in_value(
    fn_ir: &FnIR,
    root: ValueId,
    defined: &FxHashSet<VarId>,
    follow_phi_args: bool,
) -> Option<ValueId> {
    // Proof correspondence:
    // `VerifyIrFlowLite` captures the coarse use-before-def side,
    // `VerifyIrUseTraversalSubset` isolates the reduced recursive wrapper/load
    // scan, `VerifyIrValueKindTraversalSubset` refines that scan to reduced
    // `ValueKind`-named wrappers, `VerifyIrArgListTraversalSubset` adds
    // reduced `Call`/`Intrinsic`/`RecordLit` list-argument scans, and
    // `VerifyIrValueEnvSubset` isolates the `Phi`-edge rewrite/evaluation
    // step when `follow_phi_args` is enabled for predecessor environments;
    // `VerifyIrEnvScanComposeSubset` then composes those reduced env-selected
    // scans with the reduced value-kind arg/field scans under generic list /
    // field clean theorems, and `VerifyIrConsumerMetaSubset` lifts that
    // composition under explicit `Call` / `Intrinsic` / `RecordLit` consumer
    // metadata closer to these concrete match arms; `VerifyIrConsumerGraphSubset`
    // then lifts those reduced consumers into a node-id / seen / fuel graph
    // closer to the recursive `ValueId` traversal and shared-child discipline.
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        defined: &FxHashSet<VarId>,
        follow_phi_args: bool,
        seen: &mut FxHashSet<ValueId>,
    ) -> Option<ValueId> {
        if !seen.insert(root) {
            return None;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => None,
            ValueKind::Load { var } => {
                if defined.contains(var)
                    || fn_ir.params.contains(var)
                    || is_reserved_binding(var)
                    || is_namespaced_r_call(var)
                {
                    None
                } else {
                    Some(root)
                }
            }
            ValueKind::Phi { args } => {
                if !follow_phi_args {
                    return None;
                }
                for (arg, _) in args {
                    if let Some(value) = rec(fn_ir, *arg, defined, follow_phi_args, seen) {
                        return Some(value);
                    }
                }
                None
            }
            ValueKind::Len { base }
            | ValueKind::Indices { base }
            | ValueKind::FieldGet { base, .. } => rec(fn_ir, *base, defined, follow_phi_args, seen),
            ValueKind::Range { start, end } => rec(fn_ir, *start, defined, follow_phi_args, seen)
                .or_else(|| rec(fn_ir, *end, defined, follow_phi_args, seen)),
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, defined, follow_phi_args, seen),
            ValueKind::Binary { lhs, rhs, .. } => rec(fn_ir, *lhs, defined, follow_phi_args, seen)
                .or_else(|| rec(fn_ir, *rhs, defined, follow_phi_args, seen)),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    if let Some(value) = rec(fn_ir, *arg, defined, follow_phi_args, seen) {
                        return Some(value);
                    }
                }
                None
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    if let Some(offender) = rec(fn_ir, *value, defined, follow_phi_args, seen) {
                        return Some(offender);
                    }
                }
                None
            }
            ValueKind::FieldSet { base, value, .. } => {
                rec(fn_ir, *base, defined, follow_phi_args, seen)
                    .or_else(|| rec(fn_ir, *value, defined, follow_phi_args, seen))
            }
            ValueKind::Index1D { base, idx, .. } => {
                rec(fn_ir, *base, defined, follow_phi_args, seen)
                    .or_else(|| rec(fn_ir, *idx, defined, follow_phi_args, seen))
            }
            ValueKind::Index2D { base, r, c } => rec(fn_ir, *base, defined, follow_phi_args, seen)
                .or_else(|| rec(fn_ir, *r, defined, follow_phi_args, seen))
                .or_else(|| rec(fn_ir, *c, defined, follow_phi_args, seen)),
            ValueKind::Index3D { base, i, j, k } => {
                rec(fn_ir, *base, defined, follow_phi_args, seen)
                    .or_else(|| rec(fn_ir, *i, defined, follow_phi_args, seen))
                    .or_else(|| rec(fn_ir, *j, defined, follow_phi_args, seen))
                    .or_else(|| rec(fn_ir, *k, defined, follow_phi_args, seen))
            }
        }
    }

    rec(
        fn_ir,
        root,
        defined,
        follow_phi_args,
        &mut FxHashSet::default(),
    )
}

fn collect_used_values(fn_ir: &FnIR, reachable: &FxHashSet<BlockId>) -> FxHashSet<ValueId> {
    fn push_if_valid(
        fn_ir: &FnIR,
        used: &mut FxHashSet<ValueId>,
        worklist: &mut Vec<ValueId>,
        v: ValueId,
    ) {
        if v < fn_ir.values.len() && used.insert(v) {
            worklist.push(v);
        }
    }

    let mut used = FxHashSet::default();
    let mut worklist: Vec<ValueId> = Vec::new();

    for bid in 0..fn_ir.blocks.len() {
        if !reachable.contains(&bid) {
            continue;
        }
        let blk = &fn_ir.blocks[bid];
        for instr in &blk.instrs {
            match instr {
                Instr::Assign { src, .. } => {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *src);
                }
                Instr::Eval { val, .. } => {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *val);
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    for v in [*base, *idx, *val] {
                        push_if_valid(fn_ir, &mut used, &mut worklist, v);
                    }
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    for v in [*base, *r, *c, *val] {
                        push_if_valid(fn_ir, &mut used, &mut worklist, v);
                    }
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    for v in [*base, *i, *j, *k, *val] {
                        push_if_valid(fn_ir, &mut used, &mut worklist, v);
                    }
                }
            }
        }

        match &blk.term {
            Terminator::If { cond, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *cond);
            }
            Terminator::Return(Some(v)) => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *v);
            }
            _ => {}
        }
    }

    while let Some(vid) = worklist.pop() {
        if vid >= fn_ir.values.len() {
            continue;
        }
        let val = &fn_ir.values[vid];
        match &val.kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *lhs);
                push_if_valid(fn_ir, &mut used, &mut worklist, *rhs);
            }
            ValueKind::Unary { rhs, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *rhs);
            }
            ValueKind::Call { args, .. } => {
                for a in args {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *a);
                }
            }
            ValueKind::Intrinsic { args, .. } => {
                for a in args {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *a);
                }
            }
            ValueKind::Phi { args } => {
                for (a, _) in args {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *a);
                }
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *value);
                }
            }
            ValueKind::FieldGet { base, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
            }
            ValueKind::FieldSet { base, value, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
                push_if_valid(fn_ir, &mut used, &mut worklist, *value);
            }
            ValueKind::Index1D { base, idx, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
                push_if_valid(fn_ir, &mut used, &mut worklist, *idx);
            }
            ValueKind::Index2D { base, r, c } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
                push_if_valid(fn_ir, &mut used, &mut worklist, *r);
                push_if_valid(fn_ir, &mut used, &mut worklist, *c);
            }
            ValueKind::Index3D { base, i, j, k } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
                push_if_valid(fn_ir, &mut used, &mut worklist, *i);
                push_if_valid(fn_ir, &mut used, &mut worklist, *j);
                push_if_valid(fn_ir, &mut used, &mut worklist, *k);
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
            }
            ValueKind::Range { start, end } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *start);
                push_if_valid(fn_ir, &mut used, &mut worklist, *end);
            }
            _ => {}
        }
    }

    used
}

fn is_reserved_binding(name: &str) -> bool {
    name.starts_with(".phi_")
        || name.starts_with(".tachyon_")
        || name.starts_with("Sym_")
        || name.starts_with("__lambda_")
        || name.starts_with("rr_")
}
