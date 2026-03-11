use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum VectorizeSkipReason {
    NoIv,
    NonCanonicalBound,
    UnsupportedCfgShape,
    IndirectIndexAccess,
    StoreEffects,
    NoSupportedPattern,
}

impl VectorizeSkipReason {
    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::NoIv => "no induction variable",
            Self::NonCanonicalBound => "non-canonical loop bound (not seq_len/len-based)",
            Self::UnsupportedCfgShape => "unsupported CFG shape (preheader/exit)",
            Self::IndirectIndexAccess => "indirect index gather/scatter pattern",
            Self::StoreEffects => "store effects block reduction patterns",
            Self::NoSupportedPattern => "no supported map/reduction/call-map pattern matched",
        }
    }
}

pub(super) fn vectorize_trace_enabled() -> bool {
    match env::var("RR_VECTORIZE_TRACE") {
        Ok(v) => matches!(v.trim(), "1" | "true" | "TRUE" | "yes" | "on"),
        Err(_) => false,
    }
}

pub(super) fn trace_no_iv_context(fn_ir: &FnIR, lp: &LoopInfo) {
    let mut body_ids: Vec<BlockId> = lp.body.iter().copied().collect();
    body_ids.sort_unstable();
    eprintln!(
        "   [vec-no-iv] {} body={:?} exits={:?}",
        fn_ir.name, body_ids, lp.exits
    );
    match fn_ir.blocks[lp.header].term {
        Terminator::If { cond, .. } => {
            eprintln!(
                "   [vec-no-iv] {} header={} latch={} cond={:?}",
                fn_ir.name, lp.header, lp.latch, fn_ir.values[cond].kind
            );
            if let ValueKind::Binary { lhs, rhs, .. } = &fn_ir.values[cond].kind {
                eprintln!(
                    "   [vec-no-iv] {} cond-lhs={:?} cond-rhs={:?}",
                    fn_ir.name, fn_ir.values[*lhs].kind, fn_ir.values[*rhs].kind
                );
                trace_no_iv_phi_args(fn_ir, *lhs, "lhs");
                trace_no_iv_phi_args(fn_ir, *rhs, "rhs");
                trace_no_iv_load_assignments(fn_ir, lp, *lhs, "lhs");
                trace_no_iv_load_assignments(fn_ir, lp, *rhs, "rhs");
            }
        }
        ref term => {
            eprintln!(
                "   [vec-no-iv] {} header={} latch={} term={:?}",
                fn_ir.name, lp.header, lp.latch, term
            );
        }
    }
}

pub(super) fn trace_no_iv_phi_args(fn_ir: &FnIR, vid: ValueId, side: &str) {
    let ValueKind::Phi { args } = &fn_ir.values[vid].kind else {
        return;
    };
    let detail: Vec<String> = args
        .iter()
        .map(|(v, b)| format!("b{}:{:?}", b, fn_ir.values[*v].kind))
        .collect();
    eprintln!("   [vec-no-iv] {}-phi-args {}", side, detail.join(" | "));
}

pub(super) fn trace_no_iv_load_assignments(fn_ir: &FnIR, lp: &LoopInfo, vid: ValueId, side: &str) {
    let ValueKind::Load { var } = &fn_ir.values[vid].kind else {
        return;
    };
    let mut in_loop = 0usize;
    let mut out_loop = 0usize;
    let mut sample: Vec<String> = Vec::new();
    for (bid, bb) in fn_ir.blocks.iter().enumerate() {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            if lp.body.contains(&bid) {
                in_loop += 1;
            } else {
                out_loop += 1;
            }
            if sample.len() < 4 {
                sample.push(format!("b{}:{:?}", bid, fn_ir.values[*src].kind));
            }
        }
    }
    eprintln!(
        "   [vec-no-iv] {} {}-load var='{}' assigns(out={}, in={}) sample={}",
        fn_ir.name,
        side,
        var,
        out_loop,
        in_loop,
        sample.join(" | ")
    );
}

pub(super) fn trace_materialize_reject(fn_ir: &FnIR, root: ValueId, reason: &str) {
    if !vectorize_trace_enabled() {
        return;
    }
    eprintln!(
        "   [vec-materialize] {} reject {} => {:?}",
        fn_ir.name, reason, fn_ir.values[root].kind
    );
}

pub(super) fn trace_value_tree(
    fn_ir: &FnIR,
    root: ValueId,
    indent: usize,
    seen: &mut FxHashSet<ValueId>,
) {
    if !vectorize_trace_enabled() {
        return;
    }
    let root = canonical_value(fn_ir, root);
    let pad = " ".repeat(indent);
    if !seen.insert(root) {
        eprintln!("{}- v{} {:?} (seen)", pad, root, fn_ir.values[root].kind);
        return;
    }
    eprintln!("{}- v{} {:?}", pad, root, fn_ir.values[root].kind);
    match &fn_ir.values[root].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            trace_value_tree(fn_ir, *lhs, indent + 2, seen);
            trace_value_tree(fn_ir, *rhs, indent + 2, seen);
        }
        ValueKind::Unary { rhs, .. } => {
            trace_value_tree(fn_ir, *rhs, indent + 2, seen);
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
            for arg in args {
                trace_value_tree(fn_ir, *arg, indent + 2, seen);
            }
        }
        ValueKind::Phi { args } => {
            for (arg, bb) in args {
                eprintln!("{}  from bb{}", pad, bb);
                trace_value_tree(fn_ir, *arg, indent + 4, seen);
            }
        }
        ValueKind::Index1D { base, idx, .. } => {
            trace_value_tree(fn_ir, *base, indent + 2, seen);
            trace_value_tree(fn_ir, *idx, indent + 2, seen);
        }
        ValueKind::Index2D { base, r, c } => {
            trace_value_tree(fn_ir, *base, indent + 2, seen);
            trace_value_tree(fn_ir, *r, indent + 2, seen);
            trace_value_tree(fn_ir, *c, indent + 2, seen);
        }
        ValueKind::Index3D { base, i, j, k } => {
            trace_value_tree(fn_ir, *base, indent + 2, seen);
            trace_value_tree(fn_ir, *i, indent + 2, seen);
            trace_value_tree(fn_ir, *j, indent + 2, seen);
            trace_value_tree(fn_ir, *k, indent + 2, seen);
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            trace_value_tree(fn_ir, *base, indent + 2, seen);
        }
        ValueKind::Range { start, end } => {
            trace_value_tree(fn_ir, *start, indent + 2, seen);
            trace_value_tree(fn_ir, *end, indent + 2, seen);
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => {}
    }
}

pub(super) fn trace_block_instrs(fn_ir: &FnIR, bid: BlockId, indent: usize) {
    if !vectorize_trace_enabled() {
        return;
    }
    let Some(block) = fn_ir.blocks.get(bid) else {
        return;
    };
    let pad = " ".repeat(indent);
    eprintln!("{}block {}:", pad, bid);
    for ins in &block.instrs {
        eprintln!("{}  {:?}", pad, ins);
    }
    eprintln!("{}  term {:?}", pad, block.term);
}
