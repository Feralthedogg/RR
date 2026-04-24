use crate::mir::opt::parallel_copy::{Move, emit_parallel_copy, move_is_noop};
use crate::mir::*;
use crate::utils::Span;
use std::collections::{HashMap, HashSet};

#[derive(Debug)]
struct EdgeMove {
    pred: BlockId,
    succ: BlockId,
    dst: VarId,
    src: ValueId,
}

pub fn run(fn_ir: &mut FnIR) -> bool {
    let mut changed = false;

    let has_phi = fn_ir
        .values
        .iter()
        .any(|v| matches!(v.kind, ValueKind::Phi { .. }));
    if !has_phi {
        return false;
    }

    let mut reachable = compute_reachable(fn_ir);
    changed |= simplify_trivial_phis(fn_ir, &reachable);
    let mut phi_blocks = collect_phi_blocks(fn_ir);
    phi_blocks.retain(|bid, _| reachable.contains(bid));

    let succs = build_succ_map(fn_ir);
    let mut split_map: HashMap<(BlockId, BlockId), BlockId> = HashMap::new();
    let pred_map = build_pred_map_from_succs(&succs);
    let mut moves_by_block: HashMap<BlockId, Vec<Move>> = HashMap::new();
    let mut phi_infos: Vec<(ValueId, VarId)> = Vec::new();
    let mut needed_moves: Vec<EdgeMove> = Vec::new();

    let mut phi_block_ids: Vec<BlockId> = phi_blocks.keys().copied().collect();
    phi_block_ids.sort_unstable();
    for bid in phi_block_ids {
        let Some(phis) = phi_blocks.get(&bid) else {
            continue;
        };
        for &phi in phis {
            let dest = ensure_phi_var(fn_ir, phi);
            phi_infos.push((phi, dest.clone()));
            if let ValueKind::Phi { args } = &fn_ir.values[phi].kind {
                for (src, pred) in args {
                    if dest.starts_with(".arg_") {
                        // Function parameters are lowered to immutable local copies.
                        // Never synthesize phi-edge overwrites into parameter locals.
                        continue;
                    }
                    let resolved_src = canonicalize_move_source_for_pred(
                        fn_ir,
                        *pred,
                        resolve_phi_source_for_pred(fn_ir, *src, bid, *pred, &mut HashSet::new()),
                    );
                    if let Some(existing) =
                        reaching_assign_source_for_pred(fn_ir, &pred_map, *pred, &dest)
                        && same_canonical_value_before_instr(
                            fn_ir,
                            *pred,
                            fn_ir.blocks[*pred].instrs.len(),
                            existing,
                            resolved_src,
                        )
                    {
                        continue;
                    }
                    let move_candidate = Move {
                        dst: dest.clone(),
                        src: resolved_src,
                    };
                    if move_is_noop(fn_ir, &move_candidate) {
                        continue;
                    }
                    needed_moves.push(EdgeMove {
                        pred: *pred,
                        succ: bid,
                        dst: move_candidate.dst,
                        src: move_candidate.src,
                    });
                }
            }
        }
    }

    let mut edges_to_split: Vec<(BlockId, BlockId)> = needed_moves
        .iter()
        .filter(|edge_move| succs.get(&edge_move.pred).map(|s| s.len()).unwrap_or(0) > 1)
        .map(|edge_move| (edge_move.pred, edge_move.succ))
        .collect();
    edges_to_split.sort_unstable();
    edges_to_split.dedup();

    for (pred, bid) in edges_to_split {
        let key = (pred, bid);
        if let std::collections::hash_map::Entry::Vacant(e) = split_map.entry(key) {
            let new_bid = split_edge(fn_ir, pred, bid);
            e.insert(new_bid);
            changed = true;
        }
    }

    // Recompute reachability after splitting (new blocks may have been introduced).
    reachable = compute_reachable(fn_ir);
    changed |= simplify_trivial_phis(fn_ir, &reachable);

    for edge_move in needed_moves {
        let target_block = split_map
            .get(&(edge_move.pred, edge_move.succ))
            .copied()
            .unwrap_or(edge_move.pred);
        moves_by_block.entry(target_block).or_default().push(Move {
            dst: edge_move.dst,
            src: edge_move.src,
        });
    }

    let mut move_block_ids: Vec<BlockId> = moves_by_block.keys().copied().collect();
    move_block_ids.sort_unstable();
    for bid in move_block_ids {
        let Some(mut moves) = moves_by_block.remove(&bid) else {
            continue;
        };
        normalize_block_moves(fn_ir, &mut moves);
        if moves.is_empty() {
            continue;
        }
        if !reachable.contains(&bid) {
            continue;
        }
        if matches!(fn_ir.blocks[bid].term, Terminator::Unreachable) {
            continue;
        }
        let mut out_instrs = Vec::new();
        emit_parallel_copy(fn_ir, &mut out_instrs, moves, Span::default());
        if !out_instrs.is_empty() {
            fn_ir.blocks[bid].instrs.extend(out_instrs);
            changed = true;
        }
    }

    // Replace Phi nodes with explicit Loads of their assigned variable.
    for (phi, dest) in phi_infos {
        if matches!(fn_ir.values[phi].kind, ValueKind::Phi { .. }) {
            fn_ir.values[phi].kind = ValueKind::Load { var: dest };
            fn_ir.values[phi].phi_block = None;
            changed = true;
        }
    }

    // Eliminate any remaining Phi.
    // Prefer preserving variable semantics when the Phi already has a bound var name.
    for phi in 0..fn_ir.values.len() {
        if !matches!(fn_ir.values[phi].kind, ValueKind::Phi { .. }) {
            continue;
        }
        if let Some(var) = fn_ir.values[phi].origin_var.clone() {
            fn_ir.values[phi].kind = ValueKind::Load { var };
            fn_ir.values[phi].phi_block = None;
            changed = true;
            continue;
        }
        if let Some(var) = phi_shared_origin_var(fn_ir, phi) {
            fn_ir.values[phi].kind = ValueKind::Load { var: var.clone() };
            fn_ir.values[phi].origin_var = Some(var);
            fn_ir.values[phi].phi_block = None;
            changed = true;
            continue;
        }

        let unreachable_phi = match fn_ir.values[phi].phi_block {
            Some(bid) => !reachable.contains(&bid),
            None => true,
        };
        if unreachable_phi {
            // Leave dead/unreachable Phi untouched. Emittable IR validation will reject any
            // reachable Phi that survives de-SSA, but dead Phi values should not be rewritten
            // into NULL because that silently changes semantics if they later become observable.
            continue;
        }
    }

    changed
}

fn canonicalize_move_source_for_pred(fn_ir: &FnIR, pred: BlockId, src: ValueId) -> ValueId {
    canonicalize_value_before_instr(fn_ir, pred, fn_ir.blocks[pred].instrs.len(), src)
}

fn is_commutative_binop(op: crate::syntax::ast::BinOp) -> bool {
    matches!(
        op,
        crate::syntax::ast::BinOp::Add
            | crate::syntax::ast::BinOp::Mul
            | crate::syntax::ast::BinOp::Eq
            | crate::syntax::ast::BinOp::Ne
            | crate::syntax::ast::BinOp::And
            | crate::syntax::ast::BinOp::Or
    )
}

fn canonical_value_fingerprint_before_instr(
    fn_ir: &FnIR,
    pred: BlockId,
    upto: usize,
    src: ValueId,
) -> Option<String> {
    fn rec(
        fn_ir: &FnIR,
        pred: BlockId,
        upto: usize,
        src: ValueId,
        seen: &mut HashSet<(ValueId, usize)>,
    ) -> Option<String> {
        if !seen.insert((src, upto)) {
            return None;
        }
        let out = match &fn_ir.values[src].kind {
            ValueKind::Load { var } => {
                if let Some((idx, next)) = last_assign_to_var_before(fn_ir, pred, var, upto) {
                    rec(fn_ir, pred, idx, next, seen)
                } else {
                    Some(format!("load:{var}"))
                }
            }
            ValueKind::Const(lit) => Some(format!("const:{lit:?}")),
            ValueKind::Param { index } => Some(format!("param:{index}")),
            ValueKind::Binary { op, lhs, rhs } => {
                let mut lhs_fp = rec(fn_ir, pred, upto, *lhs, seen)?;
                let mut rhs_fp = rec(fn_ir, pred, upto, *rhs, seen)?;
                if is_commutative_binop(*op) && lhs_fp > rhs_fp {
                    std::mem::swap(&mut lhs_fp, &mut rhs_fp);
                }
                Some(format!("bin:{op:?}({lhs_fp},{rhs_fp})"))
            }
            ValueKind::Unary { op, rhs } => {
                let rhs_fp = rec(fn_ir, pred, upto, *rhs, seen)?;
                Some(format!("un:{op:?}({rhs_fp})"))
            }
            ValueKind::Call { callee, args, names } => {
                let mut fps = Vec::with_capacity(args.len());
                for arg in args {
                    fps.push(rec(fn_ir, pred, upto, *arg, seen)?);
                }
                Some(format!("call:{callee}:{names:?}({})", fps.join(",")))
            }
            ValueKind::Intrinsic { op, args } => {
                let mut fps = Vec::with_capacity(args.len());
                for arg in args {
                    fps.push(rec(fn_ir, pred, upto, *arg, seen)?);
                }
                Some(format!("intr:{op:?}({})", fps.join(",")))
            }
            ValueKind::RecordLit { fields } => {
                let mut fps = Vec::with_capacity(fields.len());
                for (name, value) in fields {
                    fps.push(format!("{name}={}", rec(fn_ir, pred, upto, *value, seen)?));
                }
                Some(format!("record{{{}}}", fps.join(",")))
            }
            ValueKind::FieldGet { base, field } => {
                let base_fp = rec(fn_ir, pred, upto, *base, seen)?;
                Some(format!("get:{field}({base_fp})"))
            }
            ValueKind::FieldSet { base, field, value } => {
                let base_fp = rec(fn_ir, pred, upto, *base, seen)?;
                let value_fp = rec(fn_ir, pred, upto, *value, seen)?;
                Some(format!("set:{field}({base_fp},{value_fp})"))
            }
            ValueKind::Len { base } => {
                let base_fp = rec(fn_ir, pred, upto, *base, seen)?;
                Some(format!("len({base_fp})"))
            }
            ValueKind::Indices { base } => {
                let base_fp = rec(fn_ir, pred, upto, *base, seen)?;
                Some(format!("indices({base_fp})"))
            }
            ValueKind::Range { start, end } => {
                let start_fp = rec(fn_ir, pred, upto, *start, seen)?;
                let end_fp = rec(fn_ir, pred, upto, *end, seen)?;
                Some(format!("range({start_fp},{end_fp})"))
            }
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                let base_fp = rec(fn_ir, pred, upto, *base, seen)?;
                let idx_fp = rec(fn_ir, pred, upto, *idx, seen)?;
                Some(format!("idx1d:{is_safe}:{is_na_safe}({base_fp},{idx_fp})"))
            }
            ValueKind::Index2D { base, r, c } => {
                let base_fp = rec(fn_ir, pred, upto, *base, seen)?;
                let r_fp = rec(fn_ir, pred, upto, *r, seen)?;
                let c_fp = rec(fn_ir, pred, upto, *c, seen)?;
                Some(format!("idx2d({base_fp},{r_fp},{c_fp})"))
            }
            ValueKind::Index3D { base, i, j, k } => {
                let base_fp = rec(fn_ir, pred, upto, *base, seen)?;
                let i_fp = rec(fn_ir, pred, upto, *i, seen)?;
                let j_fp = rec(fn_ir, pred, upto, *j, seen)?;
                let k_fp = rec(fn_ir, pred, upto, *k, seen)?;
                Some(format!("idx3d({base_fp},{i_fp},{j_fp},{k_fp})"))
            }
            ValueKind::RSymbol { name } => Some(format!("rsym:{name}")),
            ValueKind::Phi { .. } => None,
        };
        seen.remove(&(src, upto));
        out
    }

    rec(fn_ir, pred, upto, src, &mut HashSet::new())
}

fn same_canonical_value_before_instr(
    fn_ir: &FnIR,
    pred: BlockId,
    upto: usize,
    lhs: ValueId,
    rhs: ValueId,
) -> bool {
    if lhs == rhs {
        return true;
    }
    match (
        canonical_value_fingerprint_before_instr(fn_ir, pred, upto, lhs),
        canonical_value_fingerprint_before_instr(fn_ir, pred, upto, rhs),
    ) {
        (Some(lhs_fp), Some(rhs_fp)) => lhs_fp == rhs_fp,
        _ => false,
    }
}

fn canonicalize_value_before_instr(
    fn_ir: &FnIR,
    pred: BlockId,
    upto: usize,
    src: ValueId,
) -> ValueId {
    fn rec(
        fn_ir: &FnIR,
        pred: BlockId,
        upto: usize,
        src: ValueId,
        seen: &mut HashSet<(ValueId, usize)>,
    ) -> ValueId {
        if !seen.insert((src, upto)) {
            return src;
        }
        match &fn_ir.values[src].kind {
            ValueKind::Load { var } => {
                if let Some((idx, next)) = last_assign_to_var_before(fn_ir, pred, var, upto) {
                    return rec(fn_ir, pred, idx, next, seen);
                }
                src
            }
            _ => phi_alias_canonical_value(fn_ir, src),
        }
    }

    rec(fn_ir, pred, upto, src, &mut HashSet::new())
}

fn reaching_assign_source_for_pred(
    fn_ir: &FnIR,
    pred_map: &HashMap<BlockId, Vec<BlockId>>,
    start: BlockId,
    dst: &str,
) -> Option<ValueId> {
    fn rec(
        fn_ir: &FnIR,
        pred_map: &HashMap<BlockId, Vec<BlockId>>,
        block: BlockId,
        dst: &str,
        seen: &mut HashSet<BlockId>,
    ) -> Option<ValueId> {
        if !seen.insert(block) {
            return None;
        }
        if let Some((assign_idx, src)) =
            last_assign_to_var_before(fn_ir, block, dst, fn_ir.blocks[block].instrs.len())
        {
            seen.remove(&block);
            return Some(canonicalize_value_before_instr(
                fn_ir, block, assign_idx, src,
            ));
        }
        let preds = pred_map.get(&block)?;
        if preds.is_empty() {
            seen.remove(&block);
            return None;
        }
        let mut shared: Option<ValueId> = None;
        for pred in preds {
            let Some(src) = rec(fn_ir, pred_map, *pred, dst, seen) else {
                seen.remove(&block);
                return None;
            };
            match shared {
                None => shared = Some(src),
                Some(prev) if prev == src => {}
                Some(_) => {
                    seen.remove(&block);
                    return None;
                }
            }
        }
        seen.remove(&block);
        shared
    }

    rec(fn_ir, pred_map, start, dst, &mut HashSet::new())
}

fn last_assign_to_var_before(
    fn_ir: &FnIR,
    block: BlockId,
    var: &str,
    upto: usize,
) -> Option<(usize, ValueId)> {
    fn_ir.blocks[block]
        .instrs
        .iter()
        .take(upto)
        .enumerate()
        .rev()
        .find_map(|(idx, instr)| match instr {
            Instr::Assign { dst, src, .. } if dst == var => Some((idx, *src)),
            _ => None,
        })
}

fn normalize_block_moves(fn_ir: &FnIR, moves: &mut Vec<Move>) {
    let mut seen: HashSet<(String, ValueId)> = HashSet::new();
    moves.retain(|m| {
        let canonical_src = phi_alias_canonical_value(fn_ir, m.src);
        let key = (m.dst.clone(), canonical_src);
        seen.insert(key)
    });
}

fn ensure_phi_var(fn_ir: &mut FnIR, phi: ValueId) -> VarId {
    if let Some(name) = &fn_ir.values[phi].origin_var
        && var_is_defined_in_fn(fn_ir, name)
    {
        return name.clone();
    }
    let name = format!(".phi_{}", phi);
    fn_ir.values[phi].origin_var = Some(name.clone());
    name
}

fn var_is_defined_in_fn(fn_ir: &FnIR, var: &str) -> bool {
    fn_ir.params.iter().any(|param| param == var)
        || fn_ir.blocks.iter().any(|block| {
            block.instrs.iter().any(|instr| match instr {
                Instr::Assign { dst, .. } => dst == var,
                _ => false,
            })
        })
}

fn simplify_trivial_phis(fn_ir: &mut FnIR, reachable: &HashSet<BlockId>) -> bool {
    let mut changed = false;
    let mut progress = true;
    while progress {
        progress = false;
        for phi in 0..fn_ir.values.len() {
            if !matches!(fn_ir.values[phi].kind, ValueKind::Phi { .. }) {
                continue;
            }
            if let Some(bb) = fn_ir.values[phi].phi_block
                && !reachable.contains(&bb)
            {
                continue;
            }
            let Some(src) = trivial_phi_source(fn_ir, phi, &mut HashSet::new()) else {
                continue;
            };
            if src == phi {
                continue;
            }
            let src_val = fn_ir.values[src].clone();
            let dst = &mut fn_ir.values[phi];
            dst.kind = src_val.kind;
            dst.facts = src_val.facts;
            dst.value_ty = src_val.value_ty;
            dst.value_term = src_val.value_term;
            if dst.origin_var.is_none() {
                dst.origin_var = src_val.origin_var;
            }
            dst.phi_block = None;
            dst.escape = src_val.escape;
            progress = true;
            changed = true;
        }
    }
    changed
}

fn trivial_phi_source(fn_ir: &FnIR, phi: ValueId, seen: &mut HashSet<ValueId>) -> Option<ValueId> {
    if !seen.insert(phi) {
        return None;
    }
    let ValueKind::Phi { args } = &fn_ir.values[phi].kind else {
        return None;
    };
    let mut candidate = None;
    for (arg, _) in args {
        if *arg == phi {
            continue;
        }
        let resolved = if matches!(fn_ir.values[*arg].kind, ValueKind::Phi { .. }) {
            trivial_phi_source(fn_ir, *arg, seen).unwrap_or(*arg)
        } else {
            *arg
        };
        let resolved = phi_alias_canonical_value(fn_ir, resolved);
        match candidate {
            None => candidate = Some(resolved),
            Some(prev) if prev == resolved => {}
            Some(_) => return None,
        }
    }
    candidate
}

fn collect_phi_blocks(fn_ir: &FnIR) -> HashMap<BlockId, Vec<ValueId>> {
    let mut map: HashMap<BlockId, Vec<ValueId>> = HashMap::new();
    for (vid, val) in fn_ir.values.iter().enumerate() {
        if let ValueKind::Phi { .. } = val.kind
            && let Some(bid) = val.phi_block
        {
            map.entry(bid).or_default().push(vid);
        }
    }
    map
}

fn phi_shared_origin_var(fn_ir: &FnIR, phi: ValueId) -> Option<String> {
    let ValueKind::Phi { args } = &fn_ir.values[phi].kind else {
        return None;
    };
    let mut shared: Option<String> = None;
    for (arg, _) in args {
        let arg = phi_alias_canonical_value(fn_ir, *arg);
        let current = match &fn_ir.values[arg].kind {
            ValueKind::Load { var } => Some(var.clone()),
            _ => fn_ir.values[arg].origin_var.clone(),
        }?;
        if !var_is_defined_in_fn(fn_ir, &current) {
            return None;
        }
        match &shared {
            None => shared = Some(current),
            Some(prev) if *prev == current => {}
            Some(_) => return None,
        }
    }
    shared
}

fn phi_alias_canonical_value(fn_ir: &FnIR, mut vid: ValueId) -> ValueId {
    let mut seen = HashSet::new();
    while seen.insert(vid) {
        if let ValueKind::Load { var } = &fn_ir.values[vid].kind {
            let mut next = None;
            for block in &fn_ir.blocks {
                for instr in &block.instrs {
                    let Instr::Assign { dst, src, .. } = instr else {
                        continue;
                    };
                    if dst == var {
                        let src = *src;
                        match next {
                            None => next = Some(src),
                            Some(prev) if prev == src => {}
                            Some(_) => return vid,
                        }
                    }
                }
            }
            if let Some(next_vid) = next {
                vid = next_vid;
                continue;
            }
        }
        break;
    }
    vid
}

fn build_succ_map(fn_ir: &FnIR) -> HashMap<BlockId, Vec<BlockId>> {
    let mut succs: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for (bid, blk) in fn_ir.blocks.iter().enumerate() {
        let list = match &blk.term {
            Terminator::Goto(t) => vec![*t],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            _ => Vec::new(),
        };
        succs.insert(bid, list);
    }
    succs
}

fn build_pred_map_from_succs(
    succs: &HashMap<BlockId, Vec<BlockId>>,
) -> HashMap<BlockId, Vec<BlockId>> {
    let mut preds: HashMap<BlockId, Vec<BlockId>> = HashMap::new();
    for (from, tos) in succs {
        for to in tos {
            preds.entry(*to).or_default().push(*from);
        }
    }
    preds
}

fn compute_reachable(fn_ir: &FnIR) -> std::collections::HashSet<BlockId> {
    let mut reachable = std::collections::HashSet::new();
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

fn split_edge(fn_ir: &mut FnIR, from: BlockId, to: BlockId) -> BlockId {
    let new_bid = fn_ir.add_block();
    fn_ir.blocks[new_bid].term = Terminator::Goto(to);

    // Redirect the edge in the predecessor terminator.
    match &mut fn_ir.blocks[from].term {
        Terminator::Goto(t) => {
            if *t == to {
                *t = new_bid;
            }
        }
        Terminator::If {
            then_bb, else_bb, ..
        } => {
            if *then_bb == to {
                *then_bb = new_bid;
            }
            if *else_bb == to {
                *else_bb = new_bid;
            }
        }
        _ => {}
    }

    // Update Phi args in the target block to use the new predecessor.
    for val in &mut fn_ir.values {
        if val.phi_block != Some(to) {
            continue;
        }
        if let ValueKind::Phi { args } = &mut val.kind {
            for (_, pred) in args.iter_mut() {
                if *pred == from {
                    *pred = new_bid;
                }
            }
        }
    }

    new_bid
}

fn resolve_phi_source_for_pred(
    fn_ir: &FnIR,
    src: ValueId,
    target_block: BlockId,
    pred: BlockId,
    visiting: &mut HashSet<ValueId>,
) -> ValueId {
    if !visiting.insert(src) {
        return src;
    }
    let out = match &fn_ir.values[src].kind {
        ValueKind::Phi { args } if fn_ir.values[src].phi_block == Some(target_block) => args
            .iter()
            .find_map(|(arg, arg_pred)| {
                if *arg_pred == pred {
                    Some(resolve_phi_source_for_pred(
                        fn_ir,
                        *arg,
                        target_block,
                        pred,
                        visiting,
                    ))
                } else {
                    None
                }
            })
            .unwrap_or(src),
        _ => src,
    };
    visiting.remove(&src);
    out
}

#[cfg(test)]
mod tests {
    use super::run;
    use crate::mir::{Facts, FnIR, Instr, IntrinsicOp, Terminator, ValueKind};
    use crate::utils::Span;

    #[test]
    fn unreachable_phi_is_not_rewritten_to_null() {
        let mut f = FnIR::new("dead_phi".to_string(), Vec::new());
        let entry = f.add_block();
        let dead = f.add_block();
        f.entry = entry;
        f.body_head = entry;
        f.blocks[entry].term = Terminator::Return(None);
        f.blocks[dead].term = Terminator::Unreachable;

        let phi = f.add_value(
            ValueKind::Phi { args: Vec::new() },
            Span::default(),
            Facts::empty(),
            None,
        );
        f.values[phi].phi_block = Some(dead);

        let _ = run(&mut f);
        assert!(
            matches!(f.values[phi].kind, ValueKind::Phi { .. }),
            "dead phi should stay dead, not become NULL"
        );
    }

    #[test]
    fn trivial_phi_is_eliminated_before_parallel_copy() {
        let mut f = FnIR::new("trivial_phi".to_string(), vec![]);
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left), (one, right)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].term = Terminator::If {
            cond: one,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let changed = run(&mut f);
        assert!(changed);
        assert!(
            !matches!(f.values[phi].kind, ValueKind::Phi { .. }),
            "trivial phi should be eliminated"
        );
    }

    #[test]
    fn critical_edge_is_not_split_when_existing_assign_already_matches_phi_input() {
        let mut f = FnIR::new("critical_edge_no_split".to_string(), vec![]);
        let entry = f.add_block();
        let other = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, entry), (two, other)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: one,
            span: Span::default(),
        });
        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: merge,
            else_bb: other,
        };

        f.blocks[other].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: two,
            span: Span::default(),
        });
        f.blocks[other].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let before_blocks = f.blocks.len();
        let changed = run(&mut f);
        assert!(changed);
        assert_eq!(
            f.blocks.len(),
            before_blocks,
            "critical edge should not be split when no move is required"
        );
    }

    #[test]
    fn trivial_phi_eliminates_load_alias_inputs_with_same_canonical_source() {
        let mut f = FnIR::new("trivial_phi_load_alias".to_string(), vec![]);
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_left = f.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_right = f.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(load_left, left), (load_right, right)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: one,
            span: Span::default(),
        });
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: one,
            span: Span::default(),
        });
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let changed = run(&mut f);
        assert!(changed);
        assert!(
            !matches!(f.values[phi].kind, ValueKind::Phi { .. }),
            "phi with load-alias inputs from same canonical source should be eliminated"
        );
    }

    #[test]
    fn critical_edge_is_not_split_when_phi_input_is_load_of_existing_assignment() {
        let mut f = FnIR::new("critical_edge_load_alias".to_string(), vec![]);
        let entry = f.add_block();
        let other = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_x = f.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(load_x, entry), (two, other)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: one,
            span: Span::default(),
        });
        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: merge,
            else_bb: other,
        };
        f.blocks[other].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: two,
            span: Span::default(),
        });
        f.blocks[other].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let before_blocks = f.blocks.len();
        let changed = run(&mut f);
        assert!(changed);
        assert_eq!(
            f.blocks.len(),
            before_blocks,
            "load-alias phi input should canonicalize to existing assignment without edge split"
        );
    }

    #[test]
    fn critical_edge_is_not_split_for_noop_phi_edge_move() {
        let mut f = FnIR::new("critical_edge_noop_phi_move".to_string(), vec![]);
        let entry = f.add_block();
        let other = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_x = f.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(load_x, entry), (one, other)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: merge,
            else_bb: other,
        };
        f.blocks[other].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: one,
            span: Span::default(),
        });
        f.blocks[other].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let before_blocks = f.blocks.len();
        let changed = run(&mut f);
        assert!(changed);
        assert_eq!(
            f.blocks.len(),
            before_blocks,
            "critical edge should not be split for phi input that lowers to a no-op move"
        );
    }

    #[test]
    fn unique_predecessor_chain_existing_assign_avoids_redundant_phi_move() {
        let mut f = FnIR::new("unique_pred_chain_phi_move".to_string(), vec![]);
        let entry = f.add_block();
        let body = f.add_block();
        let then_bb = f.add_block();
        let else_bb = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let zero = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_acc = f.add_value(
            ValueKind::Load {
                var: "acc".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        let add = f.add_value(
            ValueKind::Binary {
                op: crate::syntax::ast::BinOp::Add,
                lhs: load_acc,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(add, then_bb), (add, else_bb)],
            },
            Span::default(),
            Facts::empty(),
            Some("acc".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "acc".to_string(),
            src: zero,
            span: Span::default(),
        });
        f.blocks[entry].term = Terminator::Goto(body);
        f.blocks[body].instrs.push(Instr::Assign {
            dst: "acc".to_string(),
            src: add,
            span: Span::default(),
        });
        f.blocks[body].term = Terminator::If {
            cond,
            then_bb,
            else_bb,
        };
        f.blocks[then_bb].term = Terminator::Goto(merge);
        f.blocks[else_bb].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let before_blocks = f.blocks.len();
        let changed = run(&mut f);
        assert!(changed);
        assert_eq!(
            f.blocks.len(),
            before_blocks,
            "unique predecessor chain should let de-SSA see the existing carried assignment"
        );
    }

    #[test]
    fn critical_edge_is_split_when_existing_alias_is_stale_after_later_source_write() {
        let mut f = FnIR::new("critical_edge_existing_stale_alias".to_string(), vec![]);
        let entry = f.add_block();
        let other = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let zero = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_y_for_x = f.add_value(
            ValueKind::Load {
                var: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_y_at_end = f.add_value(
            ValueKind::Load {
                var: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(load_y_at_end, entry), (two, other)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "y".to_string(),
            src: one,
            span: Span::default(),
        });
        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: load_y_for_x,
            span: Span::default(),
        });
        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "y".to_string(),
            src: zero,
            span: Span::default(),
        });
        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: merge,
            else_bb: other,
        };
        f.blocks[other].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: two,
            span: Span::default(),
        });
        f.blocks[other].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let before_blocks = f.blocks.len();
        let changed = run(&mut f);
        assert!(changed);
        assert!(
            f.blocks.len() > before_blocks,
            "critical edge must still split when the predecessor's existing alias is stale relative to a later source-variable write"
        );
    }

    #[test]
    fn critical_edge_is_not_split_when_phi_input_matches_existing_field_get_shape() {
        let mut f = FnIR::new("critical_edge_field_get_shape".to_string(), vec![]);
        let entry = f.add_block();
        let other = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let rec1 = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), one)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rec2 = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), one)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_existing = f.add_value(
            ValueKind::FieldGet {
                base: rec1,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_phi = f.add_value(
            ValueKind::FieldGet {
                base: rec2,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(get_phi, entry), (two, other)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: get_existing,
            span: Span::default(),
        });
        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: merge,
            else_bb: other,
        };
        f.blocks[other].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: two,
            span: Span::default(),
        });
        f.blocks[other].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let before_blocks = f.blocks.len();
        let changed = run(&mut f);
        assert!(changed);
        assert_eq!(
            f.blocks.len(),
            before_blocks,
            "critical edge should not split when predecessor already computes an equivalent field-get source"
        );
    }

    #[test]
    fn critical_edge_is_not_split_when_phi_input_matches_existing_intrinsic_shape() {
        let mut f = FnIR::new("critical_edge_intrinsic_shape".to_string(), vec![]);
        let entry = f.add_block();
        let other = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let intr_existing = f.add_value(
            ValueKind::Intrinsic {
                op: IntrinsicOp::VecAbsF64,
                args: vec![one],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let intr_phi = f.add_value(
            ValueKind::Intrinsic {
                op: IntrinsicOp::VecAbsF64,
                args: vec![one],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(intr_phi, entry), (two, other)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: intr_existing,
            span: Span::default(),
        });
        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: merge,
            else_bb: other,
        };
        f.blocks[other].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: two,
            span: Span::default(),
        });
        f.blocks[other].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let before_blocks = f.blocks.len();
        let changed = run(&mut f);
        assert!(changed);
        assert_eq!(
            f.blocks.len(),
            before_blocks,
            "critical edge should not split when predecessor already computes an equivalent intrinsic source"
        );
    }

    #[test]
    fn critical_edge_is_not_split_when_phi_input_matches_existing_fieldset_shape() {
        let mut f = FnIR::new("critical_edge_fieldset_shape".to_string(), vec![]);
        let entry = f.add_block();
        let other = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let rec1 = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), one)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rec2 = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), one)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let set_existing = f.add_value(
            ValueKind::FieldSet {
                base: rec1,
                field: "x".to_string(),
                value: two,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let set_phi = f.add_value(
            ValueKind::FieldSet {
                base: rec2,
                field: "x".to_string(),
                value: two,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(set_phi, entry), (two, other)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: set_existing,
            span: Span::default(),
        });
        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: merge,
            else_bb: other,
        };
        f.blocks[other].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: two,
            span: Span::default(),
        });
        f.blocks[other].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let before_blocks = f.blocks.len();
        let changed = run(&mut f);
        assert!(changed);
        assert_eq!(
            f.blocks.len(),
            before_blocks,
            "critical edge should not split when predecessor already computes an equivalent field-set source"
        );
    }
}
