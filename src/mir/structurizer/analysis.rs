use crate::mir::opt::loop_analysis::LoopAnalyzer;
use crate::mir::*;
use rustc_hash::{FxHashMap, FxHashSet};

pub(super) fn compute_reachable(fn_ir: &FnIR) -> FxHashSet<BlockId> {
    let mut reachable = FxHashSet::default();
    let mut queue = vec![fn_ir.entry];
    reachable.insert(fn_ir.entry);

    let mut head = 0;
    while head < queue.len() {
        let bid = queue[head];
        head += 1;
        if let Some(blk) = fn_ir.blocks.get(bid) {
            match &blk.term {
                Terminator::Goto(t) => {
                    if reachable.insert(*t) {
                        queue.push(*t);
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

pub(super) fn compute_loop_headers(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
) -> FxHashMap<BlockId, FxHashSet<BlockId>> {
    let analyzer = LoopAnalyzer::new(fn_ir);
    let loops = analyzer.find_loops();

    let mut grouped: FxHashMap<BlockId, FxHashSet<BlockId>> = FxHashMap::default();
    for lp in loops {
        if !reachable.contains(&lp.header) {
            continue;
        }
        let entry = grouped.entry(lp.header).or_default();
        for b in lp.body {
            entry.insert(b);
        }
    }
    grouped
}

pub(super) fn compute_postdoms(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
) -> FxHashMap<BlockId, FxHashSet<BlockId>> {
    let mut postdoms = FxHashMap::default();
    let all: FxHashSet<BlockId> = reachable.iter().cloned().collect();

    let mut exits = FxHashSet::default();
    for &b in reachable {
        if successors(fn_ir, b).is_empty() {
            exits.insert(b);
        }
    }

    for &b in reachable {
        if exits.contains(&b) {
            postdoms.insert(b, std::iter::once(b).collect());
        } else {
            postdoms.insert(b, all.clone());
        }
    }

    let mut changed = true;
    while changed {
        changed = false;
        for &b in reachable {
            if exits.contains(&b) {
                continue;
            }
            let succs = successors(fn_ir, b);
            if succs.is_empty() {
                continue;
            }

            let mut new_set: Option<FxHashSet<BlockId>> = None;
            for s in succs {
                if !reachable.contains(&s) {
                    continue;
                }
                if let Some(s_set) = postdoms.get(&s) {
                    match new_set {
                        None => new_set = Some(s_set.clone()),
                        Some(ref mut set) => set.retain(|x| s_set.contains(x)),
                    }
                }
            }

            if let Some(mut set) = new_set {
                set.insert(b);
                if postdoms.get(&b).is_some_and(|curr| set != *curr) {
                    postdoms.insert(b, set);
                    changed = true;
                }
            }
        }
    }

    postdoms
}

pub(super) fn compute_postdom_depth(
    postdoms: &FxHashMap<BlockId, FxHashSet<BlockId>>,
    reachable: &FxHashSet<BlockId>,
) -> FxHashMap<BlockId, usize> {
    let mut ipdom: FxHashMap<BlockId, BlockId> = FxHashMap::default();

    for &b in reachable {
        let set = match postdoms.get(&b) {
            Some(s) => s,
            None => continue,
        };
        let candidates: Vec<BlockId> = set.iter().cloned().filter(|x| *x != b).collect();
        if candidates.is_empty() {
            continue;
        }

        let mut chosen: Option<BlockId> = None;
        for &c in &candidates {
            let mut dominated_by_other = false;
            for &d in &candidates {
                if d == c {
                    continue;
                }
                if let Some(d_set) = postdoms.get(&d)
                    && d_set.contains(&c)
                {
                    dominated_by_other = true;
                    break;
                }
            }
            if !dominated_by_other {
                chosen = Some(c);
                break;
            }
        }

        if let Some(c) = chosen {
            ipdom.insert(b, c);
        }
    }

    let mut depth = FxHashMap::default();
    for &b in reachable {
        let mut d = 0usize;
        let mut cur = b;
        let mut guard = 0usize;
        while let Some(next) = ipdom.get(&cur) {
            d += 1;
            cur = *next;
            guard += 1;
            if guard > reachable.len() {
                break;
            }
        }
        depth.insert(b, d);
    }

    depth
}

pub(super) fn successors(fn_ir: &FnIR, bid: BlockId) -> Vec<BlockId> {
    match &fn_ir.blocks[bid].term {
        Terminator::Goto(t) => vec![*t],
        Terminator::If {
            then_bb, else_bb, ..
        } => vec![*then_bb, *else_bb],
        _ => vec![],
    }
}
