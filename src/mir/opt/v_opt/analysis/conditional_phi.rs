use super::*;

pub(in crate::mir::opt::v_opt) fn find_conditional_phi_shape_with_blocks(
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

pub(in crate::mir::opt::v_opt) fn find_conditional_phi_shape(
    fn_ir: &FnIR,
    root: ValueId,
    args: &[(ValueId, BlockId)],
) -> Option<(ValueId, ValueId, ValueId)> {
    find_conditional_phi_shape_with_blocks(fn_ir, root, args)
        .map(|(_, cond, then_val, _, else_val, _)| (cond, then_val, else_val))
}

pub(in crate::mir::opt::v_opt) fn branch_origin_for_merge(
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

pub(in crate::mir::opt::v_opt) fn is_passthrough_load_of_var(
    fn_ir: &FnIR,
    src: ValueId,
    var: &str,
) -> bool {
    matches!(
        &fn_ir.values[canonical_value(fn_ir, src)].kind,
        ValueKind::Load { var: load_var } if load_var == var
    )
}

pub(in crate::mir::opt::v_opt) fn is_prior_origin_phi_state(
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

pub(in crate::mir::opt::v_opt) fn collapse_prior_origin_phi_state(
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
