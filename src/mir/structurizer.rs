use crate::mir::opt::loop_analysis::LoopAnalyzer;
use crate::mir::*;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Clone)]
pub enum StructuredBlock {
    Sequence(Vec<StructuredBlock>),
    If {
        cond: ValueId,
        then_body: Box<StructuredBlock>,
        else_body: Option<Box<StructuredBlock>>,
    },
    Loop {
        header: BlockId,
        cond: ValueId,
        continue_on_true: bool,
        body: Box<StructuredBlock>,
    },
    BasicBlock(BlockId),
    Break,
    Next,
    Return(Option<ValueId>),
}

#[derive(Clone)]
struct LoopCtx {
    header: BlockId,
    body: FxHashSet<BlockId>,
}

pub struct Structurizer<'a> {
    fn_ir: &'a FnIR,
    loop_headers: FxHashMap<BlockId, FxHashSet<BlockId>>,
    postdoms: FxHashMap<BlockId, FxHashSet<BlockId>>,
    postdom_depth: FxHashMap<BlockId, usize>,
    reachable: FxHashSet<BlockId>,
}

impl<'a> Structurizer<'a> {
    pub fn new(fn_ir: &'a FnIR) -> Self {
        let reachable = compute_reachable(fn_ir);
        let loop_headers = compute_loop_headers(fn_ir, &reachable);
        let postdoms = compute_postdoms(fn_ir, &reachable);
        let postdom_depth = compute_postdom_depth(&postdoms, &reachable);
        Self {
            fn_ir,
            loop_headers,
            postdoms,
            postdom_depth,
            reachable,
        }
    }

    pub fn build(&self) -> StructuredBlock {
        let mut visited = FxHashSet::default();
        self.build_sequence(self.fn_ir.entry, &mut visited, None, None)
    }

    fn build_sequence(
        &self,
        start: BlockId,
        visited: &mut FxHashSet<BlockId>,
        loop_ctx: Option<LoopCtx>,
        stop: Option<BlockId>,
    ) -> StructuredBlock {
        let mut seq = Vec::new();
        let mut cur = Some(start);

        while let Some(bid) = cur {
            if let Some(stop_bid) = stop
                && bid == stop_bid {
                    break;
                }

            if let Some(ctx) = &loop_ctx {
                if bid == ctx.header {
                    seq.push(StructuredBlock::Next);
                    break;
                }
                if !ctx.body.contains(&bid) {
                    seq.push(StructuredBlock::Break);
                    break;
                }
            }

            if visited.contains(&bid) {
                break;
            }

            if let Some(loop_body) = self.loop_headers.get(&bid) {
                let (loop_block, exit) = self.build_loop(bid, loop_body, visited);
                seq.push(loop_block);
                cur = exit;
                continue;
            }

            visited.insert(bid);

            let blk = &self.fn_ir.blocks[bid];
            if !blk.instrs.is_empty() {
                seq.push(StructuredBlock::BasicBlock(bid));
            }

            match &blk.term {
                Terminator::If {
                    cond,
                    then_bb,
                    else_bb,
                } => {
                    let join = self.find_join(*then_bb, *else_bb);

                    let join_ok = match (&loop_ctx, join) {
                        (Some(ctx), Some(j)) => ctx.body.contains(&j),
                        (Some(_), None) => false,
                        (None, Some(_)) => true,
                        (None, None) => false,
                    };

                    let stop_at = if join_ok { join } else { None };

                    let mut visited_then = visited.clone();
                    let mut visited_else = visited.clone();
                    let then_body =
                        self.build_sequence(*then_bb, &mut visited_then, loop_ctx.clone(), stop_at);
                    let else_body =
                        self.build_sequence(*else_bb, &mut visited_else, loop_ctx.clone(), stop_at);
                    visited.extend(visited_then);
                    visited.extend(visited_else);

                    seq.push(StructuredBlock::If {
                        cond: *cond,
                        then_body: Box::new(then_body),
                        else_body: Some(Box::new(else_body)),
                    });

                    if join_ok {
                        cur = join;
                    } else {
                        break;
                    }
                }
                Terminator::Goto(target) => {
                    cur = Some(*target);
                }
                Terminator::Return(val) => {
                    seq.push(StructuredBlock::Return(*val));
                    break;
                }
                Terminator::Unreachable => {
                    break;
                }
            }
        }

        match seq.len() {
            0 => StructuredBlock::Sequence(vec![]),
            1 => seq.pop().unwrap(),
            _ => StructuredBlock::Sequence(seq),
        }
    }

    fn build_loop(
        &self,
        header: BlockId,
        body_set: &FxHashSet<BlockId>,
        visited: &mut FxHashSet<BlockId>,
    ) -> (StructuredBlock, Option<BlockId>) {
        visited.insert(header);
        let blk = &self.fn_ir.blocks[header];

        let (cond, then_bb, else_bb) = match &blk.term {
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => (*cond, *then_bb, *else_bb),
            _ => {
                return (StructuredBlock::BasicBlock(header), None);
            }
        };

        let body_entry = if body_set.contains(&then_bb) {
            then_bb
        } else {
            else_bb
        };
        let exit = if body_entry == then_bb {
            else_bb
        } else {
            then_bb
        };
        let continue_on_true = body_entry == then_bb;

        let ctx = LoopCtx {
            header,
            body: body_set.clone(),
        };
        let body = self.build_sequence(body_entry, visited, Some(ctx), None);

        let loop_block = StructuredBlock::Loop {
            header,
            cond,
            continue_on_true,
            body: Box::new(body),
        };

        (loop_block, Some(exit))
    }

    fn find_join(&self, then_bb: BlockId, else_bb: BlockId) -> Option<BlockId> {
        let t = self.postdoms.get(&then_bb)?;
        let e = self.postdoms.get(&else_bb)?;
        let mut best: Option<BlockId> = None;
        let mut best_depth = 0usize;

        for &cand in t.intersection(e) {
            let depth = *self.postdom_depth.get(&cand).unwrap_or(&0);
            if best.is_none() || depth > best_depth {
                best = Some(cand);
                best_depth = depth;
            }
        }
        best
    }
}

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

fn compute_loop_headers(
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

fn compute_postdoms(
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
                if set != *postdoms.get(&b).unwrap() {
                    postdoms.insert(b, set);
                    changed = true;
                }
            }
        }
    }

    postdoms
}

fn compute_postdom_depth(
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
                    && d_set.contains(&c) {
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

fn successors(fn_ir: &FnIR, bid: BlockId) -> Vec<BlockId> {
    match &fn_ir.blocks[bid].term {
        Terminator::Goto(t) => vec![*t],
        Terminator::If {
            then_bb, else_bb, ..
        } => vec![*then_bb, *else_bb],
        _ => vec![],
    }
}
