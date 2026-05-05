use super::super::*;
use super::policy::OutlinePolicy;

#[derive(Debug, Clone)]
pub(crate) struct OutlineCandidate {
    pub(crate) block: BlockId,
    pub(crate) start: usize,
    pub(crate) end: usize,
    pub(crate) live_ins: Vec<VarId>,
    pub(crate) live_outs: Vec<VarId>,
    kind: OutlineRegionKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutlineRegionKind {
    Linear,
    BranchArm,
    LoopBody,
}

impl OutlineCandidate {
    pub(crate) fn find(fn_ir: &FnIR, policy: &OutlinePolicy) -> Option<Self> {
        if fn_ir.name.starts_with("__rr_outline_") || fn_ir.requires_conservative_optimization() {
            return None;
        }
        if fn_ir_size(fn_ir) < policy.min_parent_ir {
            return None;
        }

        let loop_body_blocks = loop_body_blocks(fn_ir);
        let mut best = None;
        for block in &fn_ir.blocks {
            let kind = classify_block_region(fn_ir, block.id, &loop_body_blocks);
            for candidate in Self::for_block(fn_ir, block, policy, kind) {
                if !candidate.is_profitable(fn_ir, policy) {
                    continue;
                }
                if best
                    .as_ref()
                    .is_none_or(|prev: &Self| candidate.score() > prev.score())
                {
                    best = Some(candidate);
                }
            }
        }
        best
    }

    pub(crate) fn is_profitable(&self, fn_ir: &FnIR, policy: &OutlinePolicy) -> bool {
        self.region_len() >= self.min_region_ir(policy)
            && self.live_ins.len() <= policy.max_live_in
            && (!self.live_outs.is_empty() || self.kind == OutlineRegionKind::BranchArm)
            && self.live_outs.len() <= policy.max_live_out
            && region_is_supported(fn_ir, self)
    }

    pub(crate) const fn region_len(&self) -> usize {
        self.end - self.start
    }

    fn score(&self) -> usize {
        let kind_bonus = match self.kind {
            OutlineRegionKind::Linear => 0,
            OutlineRegionKind::BranchArm => 8,
            OutlineRegionKind::LoopBody => 4,
        };
        self.region_len()
            .saturating_add(kind_bonus)
            .saturating_add(self.live_outs.len().saturating_mul(4))
    }

    fn min_region_ir(&self, policy: &OutlinePolicy) -> usize {
        match self.kind {
            OutlineRegionKind::Linear => policy.min_region_ir,
            OutlineRegionKind::BranchArm => policy.branch_min_region_ir,
            OutlineRegionKind::LoopBody => policy.loop_min_region_ir,
        }
    }

    fn for_block(
        fn_ir: &FnIR,
        block: &Block,
        policy: &OutlinePolicy,
        kind: OutlineRegionKind,
    ) -> Vec<Self> {
        let min_region_ir = match kind {
            OutlineRegionKind::Linear => policy.min_region_ir,
            OutlineRegionKind::BranchArm => policy.branch_min_region_ir,
            OutlineRegionKind::LoopBody => policy.loop_min_region_ir,
        };
        if block.instrs.len() < min_region_ir {
            return Vec::new();
        }

        let mut candidates = Vec::new();
        for (run_start, run_end) in supported_runs(fn_ir, block) {
            for (start, end) in candidate_windows(run_start, run_end, min_region_ir) {
                let live_ins = live_ins_for_region(fn_ir, block, start, end);
                let live_outs = live_outs_for_region(fn_ir, block, start, end);
                candidates.push(Self {
                    block: block.id,
                    start,
                    end,
                    live_ins,
                    live_outs,
                    kind,
                });
            }
        }
        candidates
    }
}

fn fn_ir_size(fn_ir: &FnIR) -> usize {
    fn_ir.values.len()
        + fn_ir
            .blocks
            .iter()
            .map(|block| block.instrs.len())
            .sum::<usize>()
}

fn region_is_supported(fn_ir: &FnIR, candidate: &OutlineCandidate) -> bool {
    let Some(block) = fn_ir.blocks.get(candidate.block) else {
        return false;
    };
    if candidate.start >= candidate.end || candidate.end > block.instrs.len() {
        return false;
    }
    block.instrs[candidate.start..candidate.end]
        .iter()
        .all(|instr| instr_is_supported(fn_ir, instr))
}

fn instr_is_supported(fn_ir: &FnIR, instr: &Instr) -> bool {
    let Instr::Assign { src, .. } = instr else {
        return false;
    };
    value_tree_is_supported(fn_ir, *src)
}

fn supported_runs(fn_ir: &FnIR, block: &Block) -> Vec<(usize, usize)> {
    let mut runs = Vec::new();
    let mut start = None;
    for (index, instr) in block.instrs.iter().enumerate() {
        if instr_is_supported(fn_ir, instr) {
            start.get_or_insert(index);
        } else if let Some(run_start) = start.take() {
            runs.push((run_start, index));
        }
    }
    if let Some(run_start) = start {
        runs.push((run_start, block.instrs.len()));
    }
    runs
}

fn candidate_windows(run_start: usize, run_end: usize, min_len: usize) -> Vec<(usize, usize)> {
    let run_len = run_end.saturating_sub(run_start);
    if run_len < min_len {
        return Vec::new();
    }

    let mut windows = Vec::new();
    push_window(&mut windows, run_start, run_end);
    if run_len > min_len {
        push_window(&mut windows, run_start, run_end - 1);
    }
    push_window(&mut windows, run_start, run_start + min_len);
    push_window(&mut windows, run_end - min_len, run_end);

    if run_len >= min_len.saturating_mul(2) {
        let mid_start = run_start + (run_len - min_len) / 2;
        push_window(&mut windows, mid_start, mid_start + min_len);
    }

    let chunk_len = min_len.saturating_mul(2).min(run_len);
    let mut chunk_start = run_start;
    while chunk_start + chunk_len <= run_end && windows.len() < 16 {
        push_window(&mut windows, chunk_start, chunk_start + chunk_len);
        chunk_start = chunk_start.saturating_add(min_len);
    }

    windows
}

fn push_window(windows: &mut Vec<(usize, usize)>, start: usize, end: usize) {
    if start < end && !windows.contains(&(start, end)) {
        windows.push((start, end));
    }
}

fn classify_block_region(
    fn_ir: &FnIR,
    block: BlockId,
    loop_body_blocks: &FxHashSet<BlockId>,
) -> OutlineRegionKind {
    if loop_body_blocks.contains(&block) {
        OutlineRegionKind::LoopBody
    } else if is_branch_arm_block(fn_ir, block) {
        OutlineRegionKind::BranchArm
    } else {
        OutlineRegionKind::Linear
    }
}

fn loop_body_blocks(fn_ir: &FnIR) -> FxHashSet<BlockId> {
    let mut blocks = FxHashSet::default();
    for lp in loop_analysis::LoopAnalyzer::new(fn_ir).find_loops() {
        for block in lp.body {
            if block != lp.header {
                blocks.insert(block);
            }
        }
    }
    blocks
}

fn is_branch_arm_block(fn_ir: &FnIR, block: BlockId) -> bool {
    let preds: Vec<_> = fn_ir
        .blocks
        .iter()
        .filter(|pred| term_successors(&pred.term).contains(&block))
        .collect();
    if preds.len() != 1 {
        return false;
    }
    matches!(
        preds[0].term,
        Terminator::If {
            then_bb,
            else_bb,
            ..
        } if then_bb == block || else_bb == block
    )
}

fn term_successors(term: &Terminator) -> Vec<BlockId> {
    match term {
        Terminator::Goto(target) => vec![*target],
        Terminator::If {
            then_bb, else_bb, ..
        } => vec![*then_bb, *else_bb],
        Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
    }
}

fn value_tree_is_supported(fn_ir: &FnIR, root: ValueId) -> bool {
    let deps = worklist::collect_value_dependencies_iterative(fn_ir, root);
    deps.into_iter().all(|value| {
        fn_ir.values.get(value).is_some_and(|v| {
            !matches!(
                v.kind,
                ValueKind::Call { .. }
                    | ValueKind::Intrinsic { .. }
                    | ValueKind::Phi { .. }
                    | ValueKind::RSymbol { .. }
            )
        })
    })
}

fn live_ins_for_region(fn_ir: &FnIR, block: &Block, start: usize, end: usize) -> Vec<VarId> {
    let mut assigned = FxHashSet::default();
    let mut live_ins = Vec::new();
    let mut seen = FxHashSet::default();
    for instr in &block.instrs[start..end] {
        for var in instr_read_vars(fn_ir, instr) {
            if !assigned.contains(&var) && seen.insert(var.clone()) {
                live_ins.push(var);
            }
        }
        if let Instr::Assign { dst, .. } = instr {
            assigned.insert(dst.clone());
        }
    }
    live_ins
}

fn live_outs_for_region(fn_ir: &FnIR, block: &Block, start: usize, end: usize) -> Vec<VarId> {
    let assigned: FxHashSet<_> = block.instrs[start..end]
        .iter()
        .filter_map(|instr| match instr {
            Instr::Assign { dst, .. } => Some(dst.clone()),
            _ => None,
        })
        .collect();
    let mut live_outs = Vec::new();
    let mut seen = FxHashSet::default();

    collect_live_out_uses(
        fn_ir,
        &assigned,
        &mut seen,
        &mut live_outs,
        &block.instrs[end..],
    );
    collect_term_live_out_uses(fn_ir, &assigned, &mut seen, &mut live_outs, &block.term);
    for other in &fn_ir.blocks {
        if other.id == block.id {
            continue;
        }
        collect_live_out_uses(fn_ir, &assigned, &mut seen, &mut live_outs, &other.instrs);
        collect_term_live_out_uses(fn_ir, &assigned, &mut seen, &mut live_outs, &other.term);
    }
    live_outs
}

fn collect_live_out_uses(
    fn_ir: &FnIR,
    assigned: &FxHashSet<VarId>,
    seen: &mut FxHashSet<VarId>,
    live_outs: &mut Vec<VarId>,
    instrs: &[Instr],
) {
    for instr in instrs {
        for var in instr_read_vars(fn_ir, instr) {
            push_live_out_if_needed(assigned, seen, live_outs, var);
        }
    }
}

fn collect_term_live_out_uses(
    fn_ir: &FnIR,
    assigned: &FxHashSet<VarId>,
    seen: &mut FxHashSet<VarId>,
    live_outs: &mut Vec<VarId>,
    term: &Terminator,
) {
    for var in term_read_vars(fn_ir, term) {
        push_live_out_if_needed(assigned, seen, live_outs, var);
    }
}

fn push_live_out_if_needed(
    assigned: &FxHashSet<VarId>,
    seen: &mut FxHashSet<VarId>,
    live_outs: &mut Vec<VarId>,
    var: VarId,
) {
    if assigned.contains(&var) && seen.insert(var.clone()) {
        live_outs.push(var);
    }
}

fn instr_read_vars(fn_ir: &FnIR, instr: &Instr) -> Vec<VarId> {
    match instr {
        Instr::Assign { src, .. } => value_read_vars(fn_ir, *src),
        Instr::Eval { val, .. } => value_read_vars(fn_ir, *val),
        Instr::StoreIndex1D { base, idx, val, .. } => {
            let mut vars = value_read_vars(fn_ir, *base);
            vars.extend(value_read_vars(fn_ir, *idx));
            vars.extend(value_read_vars(fn_ir, *val));
            vars
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            let mut vars = value_read_vars(fn_ir, *base);
            vars.extend(value_read_vars(fn_ir, *r));
            vars.extend(value_read_vars(fn_ir, *c));
            vars.extend(value_read_vars(fn_ir, *val));
            vars
        }
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => {
            let mut vars = value_read_vars(fn_ir, *base);
            vars.extend(value_read_vars(fn_ir, *i));
            vars.extend(value_read_vars(fn_ir, *j));
            vars.extend(value_read_vars(fn_ir, *k));
            vars.extend(value_read_vars(fn_ir, *val));
            vars
        }
        Instr::UnsafeRBlock { .. } => Vec::new(),
    }
}

fn term_read_vars(fn_ir: &FnIR, term: &Terminator) -> Vec<VarId> {
    match term {
        Terminator::If { cond, .. } => value_read_vars(fn_ir, *cond),
        Terminator::Return(Some(value)) => value_read_vars(fn_ir, *value),
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => Vec::new(),
    }
}

fn value_read_vars(fn_ir: &FnIR, root: ValueId) -> Vec<VarId> {
    let mut vars = Vec::new();
    let mut seen = FxHashSet::default();
    for value in worklist::collect_value_dependencies_iterative(fn_ir, root) {
        if let Some(Value {
            kind: ValueKind::Load { var },
            ..
        }) = fn_ir.values.get(value)
            && seen.insert(var.clone())
        {
            vars.push(var.clone());
        }
    }
    vars
}
