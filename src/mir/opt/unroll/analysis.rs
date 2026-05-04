use super::*;

#[derive(Debug, Clone)]
pub(crate) struct UnrollCandidate {
    pub(crate) header: BlockId,
    pub(crate) body: BlockId,
    pub(crate) exit: BlockId,
    pub(crate) outside_preds: Vec<BlockId>,
    pub(crate) trip_count: usize,
    pub(crate) mode: UnrollMode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum UnrollMode {
    Full,
    Partial { factor: usize },
}

pub(crate) fn analyze(
    fn_ir: &FnIR,
    lp: &loop_analysis::LoopInfo,
    policy: UnrollPolicy,
) -> Option<UnrollCandidate> {
    if lp.exits.len() != 1 || lp.body.len() != 2 || lp.limit_adjust != 0 {
        return None;
    }
    let iv = lp.iv.as_ref()?;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let trip_count = constant_trip_count(fn_ir, lp, iv)?;
    if trip_count < 2 || trip_count > policy.max_trip.max(policy.max_partial_trip) {
        return None;
    }
    let (body, exit) = loop_body_and_exit(fn_ir, lp.header, &lp.body)?;
    if body != lp.latch || lp.exits[0] != exit {
        return None;
    }
    if !matches!(fn_ir.blocks.get(body)?.term, Terminator::Goto(target) if target == lp.header) {
        return None;
    }
    if fn_ir.blocks[body]
        .instrs
        .iter()
        .any(|instr| matches!(instr, Instr::UnsafeRBlock { .. }))
    {
        return None;
    }
    if !body_values_are_unrollable(fn_ir, body) {
        return None;
    }
    let mode = choose_unroll_mode(fn_ir, body, trip_count, policy)?;
    let outside_preds = outside_predecessors(fn_ir, lp.header, &lp.body);
    if outside_preds.is_empty() {
        return None;
    }
    Some(UnrollCandidate {
        header: lp.header,
        body,
        exit,
        outside_preds,
        trip_count,
        mode,
    })
}

fn choose_unroll_mode(
    fn_ir: &FnIR,
    body: BlockId,
    trip_count: usize,
    policy: UnrollPolicy,
) -> Option<UnrollMode> {
    if trip_count <= policy.max_trip {
        return Some(UnrollMode::Full);
    }
    if !policy.partial_enabled {
        return None;
    }

    let body_cost = block_ir_cost(fn_ir, body);
    let max_factor = policy.max_partial_factor.min(trip_count / 2);
    for factor in (2..=max_factor).rev() {
        let growth = body_cost.saturating_mul(factor.saturating_sub(1));
        if trip_count.is_multiple_of(factor) && growth <= policy.max_growth_ir {
            return Some(UnrollMode::Partial { factor });
        }
    }
    None
}

fn block_ir_cost(fn_ir: &FnIR, body: BlockId) -> usize {
    let Some(block) = fn_ir.blocks.get(body) else {
        return usize::MAX;
    };
    let mut values = FxHashSet::default();
    for root in block.instrs.iter().flat_map(instr_roots) {
        values.extend(worklist::collect_value_dependencies_iterative(fn_ir, root));
    }
    block.instrs.len().saturating_add(values.len())
}

fn loop_body_and_exit(
    fn_ir: &FnIR,
    header: BlockId,
    loop_body: &FxHashSet<BlockId>,
) -> Option<(BlockId, BlockId)> {
    let Terminator::If {
        then_bb, else_bb, ..
    } = fn_ir.blocks.get(header)?.term
    else {
        return None;
    };
    match (loop_body.contains(&then_bb), loop_body.contains(&else_bb)) {
        (true, false) => Some((then_bb, else_bb)),
        (false, true) => Some((else_bb, then_bb)),
        _ => None,
    }
}

fn outside_predecessors(
    fn_ir: &FnIR,
    header: BlockId,
    loop_body: &FxHashSet<BlockId>,
) -> Vec<BlockId> {
    let mut preds = Vec::new();
    for block in &fn_ir.blocks {
        if loop_body.contains(&block.id) {
            continue;
        }
        if term_successors(&block.term).contains(&header) {
            preds.push(block.id);
        }
    }
    preds
}

fn body_values_are_unrollable(fn_ir: &FnIR, body: BlockId) -> bool {
    fn_ir.blocks[body]
        .instrs
        .iter()
        .flat_map(instr_roots)
        .all(|root| value_tree_is_unrollable(fn_ir, root))
}

fn instr_roots(instr: &Instr) -> Vec<ValueId> {
    match instr {
        Instr::Assign { src, .. } => vec![*src],
        Instr::Eval { val, .. } => vec![*val],
        Instr::StoreIndex1D { base, idx, val, .. } => vec![*base, *idx, *val],
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => vec![*base, *r, *c, *val],
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => vec![*base, *i, *j, *k, *val],
        Instr::UnsafeRBlock { .. } => Vec::new(),
    }
}

fn value_tree_is_unrollable(fn_ir: &FnIR, root: ValueId) -> bool {
    for value in worklist::collect_value_dependencies_iterative(fn_ir, root) {
        let Some(kind) = fn_ir.values.get(value).map(|value| &value.kind) else {
            return false;
        };
        if matches!(kind, ValueKind::Phi { .. }) {
            return false;
        }
    }
    true
}

fn constant_trip_count(
    fn_ir: &FnIR,
    lp: &loop_analysis::LoopInfo,
    iv: &loop_analysis::InductionVar,
) -> Option<usize> {
    let init = const_number(fn_ir, iv.init_val)?;
    let limit = const_number(fn_ir, lp.limit.or(lp.is_seq_len)?)?;
    let Terminator::If { cond, .. } = fn_ir.blocks.get(lp.header)?.term else {
        return None;
    };
    let op = compare_op(fn_ir, cond)?;
    match op {
        BinOp::Le if limit >= init => Some((limit - init + 1) as usize),
        BinOp::Lt if limit > init => Some((limit - init) as usize),
        _ => None,
    }
}

fn compare_op(fn_ir: &FnIR, cond: ValueId) -> Option<BinOp> {
    match fn_ir.values.get(cond)?.kind {
        ValueKind::Binary { op, .. } => Some(op),
        _ => None,
    }
}

fn const_number(fn_ir: &FnIR, value: ValueId) -> Option<i64> {
    match fn_ir.values.get(value)?.kind {
        ValueKind::Const(Lit::Int(v)) => Some(v),
        ValueKind::Const(Lit::Float(v)) if v.fract() == 0.0 => Some(v as i64),
        _ => None,
    }
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
