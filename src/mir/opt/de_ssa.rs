use crate::mir::opt::parallel_copy::{Move, emit_parallel_copy};
use crate::mir::*;
use crate::utils::Span;
use std::collections::{HashMap, HashSet};

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
    let mut phi_blocks = collect_phi_blocks(fn_ir);
    phi_blocks.retain(|bid, _| reachable.contains(bid));

    // Split edges from multi-succ blocks so we can insert per-edge copies safely.
    let succs = build_succ_map(fn_ir);
    let mut split_map: HashMap<(BlockId, BlockId), BlockId> = HashMap::new();

    let mut edges_to_split: Vec<(BlockId, BlockId)> = Vec::new();
    for (bid, phis) in phi_blocks.iter() {
        for &phi in phis {
            if let ValueKind::Phi { args } = &fn_ir.values[phi].kind {
                for (_, pred) in args {
                    if !reachable.contains(pred) {
                        continue;
                    }
                    if succs.get(pred).map(|s| s.len()).unwrap_or(0) > 1 {
                        edges_to_split.push((*pred, *bid));
                    }
                }
            }
        }
    }

    for (pred, bid) in edges_to_split {
        let key = (pred, bid);
        if !split_map.contains_key(&key) {
            let new_bid = split_edge(fn_ir, pred, bid);
            split_map.insert(key, new_bid);
            changed = true;
        }
    }

    // Recompute reachability after splitting (new blocks may have been introduced).
    reachable = compute_reachable(fn_ir);

    // Recollect after splitting (phi args updated).
    phi_blocks = collect_phi_blocks(fn_ir);
    phi_blocks.retain(|bid, _| reachable.contains(bid));

    let last_assign_map = build_last_assign_map(fn_ir);
    let mut moves_by_block: HashMap<BlockId, Vec<Move>> = HashMap::new();
    let mut phi_infos: Vec<(ValueId, VarId)> = Vec::new();

    for (bid, phis) in phi_blocks.iter() {
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
                    let resolved_src =
                        resolve_phi_source_for_pred(fn_ir, *src, *bid, *pred, &mut HashSet::new());
                    if let Some(block_map) = last_assign_map.get(pred) {
                        if let Some(existing) = block_map.get(&dest) {
                            if *existing == resolved_src {
                                continue;
                            }
                        }
                    }
                    moves_by_block.entry(*pred).or_default().push(Move {
                        dst: dest.clone(),
                        src: resolved_src,
                    });
                }
            }
        }
    }

    for (bid, moves) in moves_by_block {
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
    for val in &mut fn_ir.values {
        if let ValueKind::Phi { .. } = val.kind {
            if let Some(var) = val.origin_var.clone() {
                val.kind = ValueKind::Load { var };
                val.phi_block = None;
                changed = true;
                continue;
            }

            let unreachable_phi = match val.phi_block {
                Some(bid) => !reachable.contains(&bid),
                None => true,
            };
            if unreachable_phi {
                val.kind = ValueKind::Const(Lit::Null);
                val.phi_block = None;
                changed = true;
            }
        }
    }

    changed
}

fn ensure_phi_var(fn_ir: &mut FnIR, phi: ValueId) -> VarId {
    if let Some(name) = &fn_ir.values[phi].origin_var {
        return name.clone();
    }
    let name = format!(".phi_{}", phi);
    fn_ir.values[phi].origin_var = Some(name.clone());
    name
}

fn collect_phi_blocks(fn_ir: &FnIR) -> HashMap<BlockId, Vec<ValueId>> {
    let mut map: HashMap<BlockId, Vec<ValueId>> = HashMap::new();
    for (vid, val) in fn_ir.values.iter().enumerate() {
        if let ValueKind::Phi { .. } = val.kind {
            if let Some(bid) = val.phi_block {
                map.entry(bid).or_default().push(vid);
            }
        }
    }
    map
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

fn build_last_assign_map(fn_ir: &FnIR) -> HashMap<BlockId, HashMap<VarId, ValueId>> {
    let mut map: HashMap<BlockId, HashMap<VarId, ValueId>> = HashMap::new();
    for (bid, blk) in fn_ir.blocks.iter().enumerate() {
        let mut last: HashMap<VarId, ValueId> = HashMap::new();
        for instr in &blk.instrs {
            if let Instr::Assign { dst, src, .. } = instr {
                last.insert(dst.clone(), *src);
            }
        }
        map.insert(bid, last);
    }
    map
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
