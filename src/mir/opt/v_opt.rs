use crate::mir::opt::loop_analysis::{LoopAnalyzer, LoopInfo, build_pred_map};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use crate::typeck::{PrimTy, ShapeTy};
use rustc_hash::{FxHashMap, FxHashSet};
use std::env;

#[derive(Debug, Default, Clone, Copy)]
pub struct VOptStats {
    pub vectorized: usize,
    pub reduced: usize,
    pub loops_seen: usize,
    pub skipped: usize,
    pub skip_no_iv: usize,
    pub skip_non_canonical_bound: usize,
    pub skip_unsupported_cfg_shape: usize,
    pub skip_indirect_index_access: usize,
    pub skip_store_effects: usize,
    pub skip_no_supported_pattern: usize,
}

impl VOptStats {
    pub fn changed(self) -> bool {
        self.vectorized > 0 || self.reduced > 0
    }

    fn record_skip(&mut self, reason: VectorizeSkipReason) {
        self.skipped += 1;
        match reason {
            VectorizeSkipReason::NoIv => self.skip_no_iv += 1,
            VectorizeSkipReason::NonCanonicalBound => self.skip_non_canonical_bound += 1,
            VectorizeSkipReason::UnsupportedCfgShape => self.skip_unsupported_cfg_shape += 1,
            VectorizeSkipReason::IndirectIndexAccess => self.skip_indirect_index_access += 1,
            VectorizeSkipReason::StoreEffects => self.skip_store_effects += 1,
            VectorizeSkipReason::NoSupportedPattern => self.skip_no_supported_pattern += 1,
        }
    }
}

pub fn optimize(fn_ir: &mut FnIR) -> bool {
    optimize_with_stats(fn_ir).changed()
}

pub fn optimize_with_stats(fn_ir: &mut FnIR) -> VOptStats {
    optimize_with_stats_with_whitelist(fn_ir, &FxHashSet::default())
}

pub fn optimize_with_stats_with_whitelist(
    fn_ir: &mut FnIR,
    user_call_whitelist: &FxHashSet<String>,
) -> VOptStats {
    let mut stats = VOptStats::default();
    let trace_enabled = vectorize_trace_enabled();
    let analyzer = LoopAnalyzer::new(fn_ir);
    let loops = analyzer.find_loops();

    for lp in loops {
        stats.loops_seen += 1;
        let before = stats;
        if trace_enabled {
            let mut body_ids: Vec<BlockId> = lp.body.iter().copied().collect();
            body_ids.sort_unstable();
            let iv_origin = lp
                .iv
                .as_ref()
                .and_then(|iv| induction_origin_var(fn_ir, iv.phi_val));
            eprintln!(
                "   [vec-loop] {} header={} latch={} exits={:?} iv_origin={:?} body={:?}",
                fn_ir.name, lp.header, lp.latch, lp.exits, iv_origin, body_ids
            );
        }
        if let Some(plan) = match_reduction(fn_ir, &lp) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.reduced += 1;
            }
        } else if let Some(plan) = match_2d_row_reduction_sum(fn_ir, &lp) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.reduced += 1;
            }
        } else if let Some(plan) = match_2d_col_reduction_sum(fn_ir, &lp) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.reduced += 1;
            }
        } else if let Some(plan) = match_conditional_map(fn_ir, &lp, user_call_whitelist) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.vectorized += 1;
            }
        } else if let Some(plan) = match_recurrence_add_const(fn_ir, &lp) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.vectorized += 1;
            }
        } else if let Some(plan) = match_shifted_map(fn_ir, &lp) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.vectorized += 1;
            }
        } else if let Some(plan) = match_2d_row_map(fn_ir, &lp) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.vectorized += 1;
            }
        } else if let Some(plan) = match_2d_col_map(fn_ir, &lp) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.vectorized += 1;
            }
        } else if let Some(plan) = match_call_map(fn_ir, &lp, user_call_whitelist) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.vectorized += 1;
            }
        } else if let Some(plan) = match_cube_slice_expr_map(fn_ir, &lp, user_call_whitelist) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.vectorized += 1;
            }
        } else if let Some(plan) = match_expr_map(fn_ir, &lp, user_call_whitelist) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.vectorized += 1;
            }
        } else if let Some(plan) = match_scatter_expr_map(fn_ir, &lp, user_call_whitelist) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.vectorized += 1;
            }
        } else if let Some(plan) = match_map(fn_ir, &lp)
            && apply_vectorization(fn_ir, &lp, plan)
        {
            stats.vectorized += 1;
        }

        let applied = stats.vectorized != before.vectorized || stats.reduced != before.reduced;
        if trace_enabled && !applied {
            let reason = loop_vectorize_skip_reason(fn_ir, &lp);
            eprintln!("   [vec-skip] {}: {}", fn_ir.name, reason.label());
            if reason == VectorizeSkipReason::NoIv {
                trace_no_iv_context(fn_ir, &lp);
            }
        }
        if !applied {
            stats.record_skip(loop_vectorize_skip_reason(fn_ir, &lp));
        }
    }

    stats
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum VectorizeSkipReason {
    NoIv,
    NonCanonicalBound,
    UnsupportedCfgShape,
    IndirectIndexAccess,
    StoreEffects,
    NoSupportedPattern,
}

impl VectorizeSkipReason {
    const fn label(self) -> &'static str {
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

fn vectorize_trace_enabled() -> bool {
    match env::var("RR_VECTORIZE_TRACE") {
        Ok(v) => matches!(v.trim(), "1" | "true" | "TRUE" | "yes" | "on"),
        Err(_) => false,
    }
}

fn trace_no_iv_context(fn_ir: &FnIR, lp: &LoopInfo) {
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

fn trace_no_iv_phi_args(fn_ir: &FnIR, vid: ValueId, side: &str) {
    let ValueKind::Phi { args } = &fn_ir.values[vid].kind else {
        return;
    };
    let detail: Vec<String> = args
        .iter()
        .map(|(v, b)| format!("b{}:{:?}", b, fn_ir.values[*v].kind))
        .collect();
    eprintln!("   [vec-no-iv] {}-phi-args {}", side, detail.join(" | "));
}

fn trace_no_iv_load_assignments(fn_ir: &FnIR, lp: &LoopInfo, vid: ValueId, side: &str) {
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

#[derive(Debug)]
pub enum VectorPlan {
    Reduce {
        kind: ReduceKind,
        acc_phi: ValueId,
        vec_expr: ValueId,
        iv_phi: ValueId,
    },
    Reduce2DRowSum {
        acc_phi: ValueId,
        base: ValueId,
        row: ValueId,
        start: ValueId,
        end: ValueId,
    },
    Reduce2DColSum {
        acc_phi: ValueId,
        base: ValueId,
        col: ValueId,
        start: ValueId,
        end: ValueId,
    },
    Map {
        dest: ValueId,
        src: ValueId,
        op: crate::syntax::ast::BinOp,
        other: ValueId,
    },
    CondMap {
        dest: ValueId,
        cond: ValueId,
        then_val: ValueId,
        else_val: ValueId,
        iv_phi: ValueId,
    },
    RecurrenceAddConst {
        base: ValueId,
        start: ValueId,
        end: ValueId,
        delta: ValueId,
        negate_delta: bool,
    },
    ShiftedMap {
        dest: ValueId,
        src: ValueId,
        start: ValueId,
        end: ValueId,
        offset: i64,
    },
    CallMap {
        dest: ValueId,
        callee: String,
        args: Vec<CallMapArg>,
        iv_phi: ValueId,
    },
    CubeSliceExprMap {
        dest: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
        face: ValueId,
        row: ValueId,
        size: ValueId,
        ctx: Option<ValueId>,
        start: ValueId,
        end: ValueId,
    },
    ExprMap {
        dest: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
        whole_dest: bool,
    },
    MultiExprMap {
        entries: Vec<ExprMapEntry>,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
    },
    ScatterExprMap {
        dest: ValueId,
        idx: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
    },
    Map2DRow {
        dest: ValueId,
        row: ValueId,
        start: ValueId,
        end: ValueId,
        lhs_src: ValueId,
        rhs_src: ValueId,
        op: crate::syntax::ast::BinOp,
    },
    Map2DCol {
        dest: ValueId,
        col: ValueId,
        start: ValueId,
        end: ValueId,
        lhs_src: ValueId,
        rhs_src: ValueId,
        op: crate::syntax::ast::BinOp,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct CallMapArg {
    value: ValueId,
    vectorized: bool,
}

#[derive(Debug, Clone, Copy)]
pub struct ExprMapEntry {
    dest: ValueId,
    expr: ValueId,
    whole_dest: bool,
}

#[derive(Debug, Clone, Copy)]
struct BlockStore1D {
    base: ValueId,
    idx: ValueId,
    val: ValueId,
    is_vector: bool,
}

#[derive(Debug, Clone, Copy)]
enum BlockStore1DMatch {
    None,
    One(BlockStore1D),
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReduceKind {
    Sum,
    Prod,
    Min,
    Max,
}

fn match_reduction(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    if loop_has_store_effect(fn_ir, lp) {
        // Conservative: do not fold reductions if loop writes memory.
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        if let ValueKind::Phi { args } = &val.kind
            && args.len() == 2
            && args.iter().any(|(_, b)| *b == lp.latch)
        {
            let (next_val, _) = args.iter().find(|(_, b)| *b == lp.latch).unwrap();
            let next_v = &fn_ir.values[*next_val];

            match &next_v.kind {
                ValueKind::Binary {
                    op: crate::syntax::ast::BinOp::Add,
                    lhs,
                    rhs,
                } => {
                    if *lhs == id || *rhs == id {
                        let other = if *lhs == id { *rhs } else { *lhs };
                        if expr_has_iv_dependency(fn_ir, other, iv_phi)
                            && is_vectorizable_expr(fn_ir, other, iv_phi, lp, false, true)
                        {
                            return Some(VectorPlan::Reduce {
                                kind: ReduceKind::Sum,
                                acc_phi: id,
                                vec_expr: other,
                                iv_phi,
                            });
                        }
                    }
                }
                ValueKind::Binary {
                    op: crate::syntax::ast::BinOp::Mul,
                    lhs,
                    rhs,
                } => {
                    if *lhs == id || *rhs == id {
                        let other = if *lhs == id { *rhs } else { *lhs };
                        if expr_has_iv_dependency(fn_ir, other, iv_phi)
                            && is_vectorizable_expr(fn_ir, other, iv_phi, lp, false, true)
                        {
                            return Some(VectorPlan::Reduce {
                                kind: ReduceKind::Prod,
                                acc_phi: id,
                                vec_expr: other,
                                iv_phi,
                            });
                        }
                    }
                }
                ValueKind::Call { callee, args, .. }
                    if (callee == "min" || callee == "max") && args.len() == 2 =>
                {
                    let (a, b) = (args[0], args[1]);
                    let acc_side = if a == id {
                        Some(b)
                    } else if b == id {
                        Some(a)
                    } else {
                        None
                    };
                    if let Some(other) = acc_side
                        && expr_has_iv_dependency(fn_ir, other, iv_phi)
                        && is_vectorizable_expr(fn_ir, other, iv_phi, lp, false, true)
                    {
                        let kind = if callee == "min" {
                            ReduceKind::Min
                        } else {
                            ReduceKind::Max
                        };
                        return Some(VectorPlan::Reduce {
                            kind,
                            acc_phi: id,
                            vec_expr: other,
                            iv_phi,
                        });
                    }
                }
                _ => {}
            }
        }
    }
    None
}

fn match_2d_row_reduction_sum(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    if loop_has_store_effect(fn_ir, lp) {
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        if args.len() != 2 || !args.iter().any(|(_, b)| *b == lp.latch) {
            continue;
        }
        let (next_val, _) = args.iter().find(|(_, b)| *b == lp.latch).unwrap();
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = fn_ir.values[*next_val].kind
        else {
            continue;
        };
        let other = if lhs == id {
            rhs
        } else if rhs == id {
            lhs
        } else {
            continue;
        };
        let ValueKind::Index2D { base, r, c } = fn_ir.values[other].kind else {
            continue;
        };
        if !is_iv_equivalent(fn_ir, c, iv_phi) || expr_has_iv_dependency(fn_ir, r, iv_phi) {
            continue;
        }
        if !is_loop_invariant_axis(fn_ir, r, iv_phi, base) {
            continue;
        }
        return Some(VectorPlan::Reduce2DRowSum {
            acc_phi: id,
            base: canonical_value(fn_ir, base),
            row: r,
            start,
            end,
        });
    }
    None
}

fn match_2d_col_reduction_sum(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    if loop_has_store_effect(fn_ir, lp) {
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        if args.len() != 2 || !args.iter().any(|(_, b)| *b == lp.latch) {
            continue;
        }
        let (next_val, _) = args.iter().find(|(_, b)| *b == lp.latch).unwrap();
        let ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } = fn_ir.values[*next_val].kind
        else {
            continue;
        };
        let other = if lhs == id {
            rhs
        } else if rhs == id {
            lhs
        } else {
            continue;
        };
        let ValueKind::Index2D { base, r, c } = fn_ir.values[other].kind else {
            continue;
        };
        if !is_iv_equivalent(fn_ir, r, iv_phi) || expr_has_iv_dependency(fn_ir, c, iv_phi) {
            continue;
        }
        if !is_loop_invariant_axis(fn_ir, c, iv_phi, base) {
            continue;
        }
        return Some(VectorPlan::Reduce2DColSum {
            acc_phi: id,
            base: canonical_value(fn_ir, base),
            col: c,
            start,
            end,
        });
    }
    None
}

fn match_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            if let Instr::StoreIndex1D {
                base,
                idx,
                val,
                is_vector: false,
                is_safe,
                is_na_safe,
                ..
            } = instr
                && is_iv_equivalent(fn_ir, *idx, iv_phi)
                && *is_safe
                && *is_na_safe
            {
                let rhs_val = &fn_ir.values[*val];
                if let ValueKind::Binary { op, lhs, rhs } = &rhs_val.kind {
                    let lhs_idx = as_safe_loop_index(fn_ir, *lhs, iv_phi);
                    let rhs_idx = as_safe_loop_index(fn_ir, *rhs, iv_phi);

                    // x[i] OP x[i]  ->  x OP x
                    if let (Some(lbase), Some(rbase)) = (lhs_idx, rhs_idx)
                        && lbase == rbase
                        && loop_matches_vec(lp, fn_ir, lbase)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        if loop_has_non_iv_loop_carried_state_except(fn_ir, lp, &allowed_dests) {
                            continue;
                        }
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: lbase,
                            op: *op,
                            other: rbase,
                        });
                    }

                    // x[i] OP k  ->  x OP k
                    if let Some(x_base) = lhs_idx
                        && loop_matches_vec(lp, fn_ir, x_base)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        if loop_has_non_iv_loop_carried_state_except(fn_ir, lp, &allowed_dests) {
                            continue;
                        }
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: x_base,
                            op: *op,
                            other: *rhs,
                        });
                    }

                    // k OP x[i]  ->  k OP x
                    if let Some(x_base) = rhs_idx
                        && loop_matches_vec(lp, fn_ir, x_base)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        if loop_has_non_iv_loop_carried_state_except(fn_ir, lp, &allowed_dests) {
                            continue;
                        }
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: *lhs,
                            op: *op,
                            other: x_base,
                        });
                    }
                }
            }
        }
    }
    None
}

fn match_conditional_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let trace_enabled = vectorize_trace_enabled();

    for &bid in &lp.body {
        if bid == lp.header {
            continue;
        }
        if let Terminator::If {
            cond,
            then_bb,
            else_bb,
        } = fn_ir.blocks[bid].term
        {
            if !lp.body.contains(&then_bb) || !lp.body.contains(&else_bb) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: branch blocks leave loop body (then={}, else={})",
                        fn_ir.name, then_bb, else_bb
                    );
                }
                continue;
            }
            let cond_vec = is_condition_vectorizable(fn_ir, cond, iv_phi, lp, user_call_whitelist);
            let cond_dep = expr_has_iv_dependency(fn_ir, cond, iv_phi);
            if !cond_vec || !cond_dep {
                if trace_enabled {
                    let cond_detail = match &fn_ir.values[cond].kind {
                        ValueKind::Binary { lhs, rhs, .. } => format!(
                            "lhs={:?} rhs={:?}",
                            fn_ir.values[*lhs].kind, fn_ir.values[*rhs].kind
                        ),
                        other => format!("{:?}", other),
                    };
                    eprintln!(
                        "   [vec-cond-map] {} skip: condition rejected (vec={}, dep={}, cond={:?}, detail={})",
                        fn_ir.name, cond_vec, cond_dep, fn_ir.values[cond].kind, cond_detail
                    );
                }
                continue;
            }
            let then_store = classify_store_1d_in_block(fn_ir, then_bb);
            let else_store = classify_store_1d_in_block(fn_ir, else_bb);
            let (then_base, then_val, else_base, else_val) = match (then_store, else_store) {
                (BlockStore1DMatch::One(then_store), BlockStore1DMatch::One(else_store))
                    if !then_store.is_vector
                        && !else_store.is_vector
                        && is_iv_equivalent(fn_ir, then_store.idx, iv_phi)
                        && is_iv_equivalent(fn_ir, else_store.idx, iv_phi) =>
                {
                    (
                        then_store.base,
                        then_store.val,
                        else_store.base,
                        else_store.val,
                    )
                }
                _ => {
                    if trace_enabled {
                        match then_store {
                            BlockStore1DMatch::One(_) => {}
                            _ => eprintln!(
                                "   [vec-cond-map] {} skip: then branch has no canonical single StoreIndex1D",
                                fn_ir.name
                            ),
                        }
                        match else_store {
                            BlockStore1DMatch::One(_) => {}
                            _ => eprintln!(
                                "   [vec-cond-map] {} skip: else branch has no canonical single StoreIndex1D",
                                fn_ir.name
                            ),
                        }
                    }
                    continue;
                }
            };
            let then_base = canonical_value(fn_ir, then_base);
            let else_base = canonical_value(fn_ir, else_base);
            let dest_base = if then_base == else_base {
                then_base
            } else {
                match (
                    resolve_base_var(fn_ir, then_base),
                    resolve_base_var(fn_ir, else_base),
                ) {
                    (Some(a), Some(b)) if a == b => then_base,
                    _ => {
                        if trace_enabled {
                            eprintln!(
                                "   [vec-cond-map] {} skip: then/else destination bases differ",
                                fn_ir.name
                            );
                        }
                        continue;
                    }
                }
            };
            let allowed_dests: Vec<VarId> =
                resolve_base_var(fn_ir, dest_base).into_iter().collect();
            if loop_has_non_iv_loop_carried_state_except(fn_ir, lp, &allowed_dests) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: loop carries non-destination state",
                        fn_ir.name
                    );
                }
                continue;
            }
            if !is_const_one(fn_ir, iv.init_val) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: non-unit start index for conditional map",
                        fn_ir.name
                    );
                }
                continue;
            }
            if !is_vectorizable_expr(fn_ir, then_val, iv_phi, lp, true, false) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: then expression is not vectorizable",
                        fn_ir.name
                    );
                }
                continue;
            }
            if !is_vectorizable_expr(fn_ir, else_val, iv_phi, lp, true, false) {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: else expression is not vectorizable",
                        fn_ir.name
                    );
                }
                continue;
            }
            return Some(VectorPlan::CondMap {
                dest: dest_base,
                cond,
                then_val,
                else_val,
                iv_phi,
            });
        }
    }
    None
}

fn match_recurrence_add_const(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let start = iv.init_val;
    let end = lp.limit?;
    let iv_phi = iv.phi_val;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (base, idx, val, is_vector) = match instr {
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => (*base, *idx, *val, *is_vector),
                _ => continue,
            };
            if is_vector || !is_iv_equivalent(fn_ir, idx, iv_phi) {
                continue;
            }
            let base = canonical_value(fn_ir, base);
            if !is_loop_compatible_base(lp, fn_ir, base) {
                continue;
            }

            let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[val].kind else {
                continue;
            };
            if !matches!(*op, BinOp::Add | BinOp::Sub) {
                continue;
            }

            let (prev_side, delta_side, negate_delta) =
                if is_prev_element(fn_ir, *lhs, base, iv_phi) {
                    // a[i] = a[i-1] + delta  or  a[i] = a[i-1] - delta
                    (*lhs, *rhs, *op == BinOp::Sub)
                } else if *op == BinOp::Add && is_prev_element(fn_ir, *rhs, base, iv_phi) {
                    // a[i] = delta + a[i-1]
                    (*rhs, *lhs, false)
                } else {
                    continue;
                };

            if !is_prev_element(fn_ir, prev_side, base, iv_phi) {
                continue;
            }
            if expr_has_iv_dependency(fn_ir, delta_side, iv_phi) {
                continue;
            }
            if expr_reads_base(fn_ir, delta_side, base) {
                continue;
            }

            return Some(VectorPlan::RecurrenceAddConst {
                base,
                start,
                end,
                delta: delta_side,
                negate_delta,
            });
        }
    }
    None
}

fn match_shifted_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest_base, dest_idx, rhs, is_vector) = match instr {
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => (*base, *idx, *val, *is_vector),
                _ => continue,
            };
            if is_vector || !is_iv_equivalent(fn_ir, dest_idx, iv_phi) {
                continue;
            }

            let ValueKind::Index1D {
                base: src_base,
                idx: src_idx,
                ..
            } = fn_ir.values[rhs].kind.clone()
            else {
                continue;
            };

            let Some(offset) = affine_iv_offset(fn_ir, src_idx, iv_phi) else {
                continue;
            };
            if offset == 0 {
                continue;
            }

            let d = canonical_value(fn_ir, dest_base);
            let s = canonical_value(fn_ir, src_base);
            if d == s {
                // Potential loop-carried dependence (recurrence-like); keep scalar semantics.
                continue;
            }
            return Some(VectorPlan::ShiftedMap {
                dest: d,
                src: s,
                start,
                end,
                offset,
            });
        }
    }
    None
}

fn match_call_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest_base, dest_idx, rhs, is_vector, is_safe, is_na_safe) = match instr {
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    is_safe,
                    is_na_safe,
                    ..
                } => (*base, *idx, *val, *is_vector, *is_safe, *is_na_safe),
                _ => continue,
            };
            if is_vector || !is_safe || !is_na_safe || !is_iv_equivalent(fn_ir, dest_idx, iv_phi) {
                continue;
            }
            let rhs = resolve_load_alias_value(fn_ir, rhs);
            let ValueKind::Call { callee, args, .. } = &fn_ir.values[rhs].kind else {
                continue;
            };
            if !is_vector_safe_call(callee, args.len(), user_call_whitelist) {
                continue;
            }

            let mut mapped_args = Vec::with_capacity(args.len());
            let mut has_vector_arg = false;
            for arg in args {
                let arg = resolve_load_alias_value(fn_ir, *arg);
                if expr_has_iv_dependency(fn_ir, arg, iv_phi) {
                    if !is_vector_safe_call_chain_expr(fn_ir, arg, iv_phi, lp, user_call_whitelist)
                    {
                        mapped_args.clear();
                        break;
                    }
                    mapped_args.push(CallMapArg {
                        value: arg,
                        vectorized: true,
                    });
                    has_vector_arg = true;
                } else {
                    if !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, user_call_whitelist) {
                        mapped_args.clear();
                        break;
                    }
                    mapped_args.push(CallMapArg {
                        value: arg,
                        vectorized: false,
                    });
                }
            }
            if mapped_args.is_empty() || !has_vector_arg {
                continue;
            }
            return Some(VectorPlan::CallMap {
                dest: canonical_value(fn_ir, dest_base),
                callee: callee.clone(),
                args: mapped_args,
                iv_phi,
            });
        }
    }
    None
}

fn match_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    fn is_canonical_store_index(fn_ir: &FnIR, idx: ValueId, iv_phi: ValueId) -> bool {
        if is_iv_equivalent(fn_ir, idx, iv_phi) {
            return true;
        }
        is_floor_like_iv_expr(
            fn_ir,
            idx,
            iv_phi,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
        )
    }

    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut found: Vec<(ValueId, ValueId)> = Vec::new();

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } | Instr::Eval { .. } => {}
                Instr::StoreIndex2D { .. } => return None,
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => {
                    let idx_ok = is_canonical_store_index(fn_ir, *idx, iv_phi);
                    if *is_vector || !idx_ok {
                        if vectorize_trace_enabled() {
                            let phi_detail = match &fn_ir.values[*idx].kind {
                                ValueKind::Phi { args } => {
                                    let parts: Vec<String> = args
                                        .iter()
                                        .map(|(v, b)| format!("b{}:{:?}", b, fn_ir.values[*v].kind))
                                        .collect();
                                    format!(" phi_args=[{}]", parts.join(" | "))
                                }
                                ValueKind::Load { var } => {
                                    let unique_src = unique_assign_source(fn_ir, var)
                                        .map(|src| format!("{:?}", fn_ir.values[src].kind))
                                        .unwrap_or_else(|| "none".to_string());
                                    format!(
                                        " load_var='{}' iv_origin={:?} unique_src={}",
                                        var,
                                        induction_origin_var(fn_ir, iv_phi),
                                        unique_src
                                    )
                                }
                                _ => String::new(),
                            };
                            eprintln!(
                                "   [vec-expr-map] reject: non-canonical store index (fn={}, idx_ok={}, idx={:?}{})",
                                fn_ir.name, idx_ok, fn_ir.values[*idx].kind, phi_detail
                            );
                        }
                        return None;
                    }
                    let dest = canonical_value(fn_ir, *base);
                    if found
                        .iter()
                        .any(|(existing_dest, _)| same_base_value(fn_ir, *existing_dest, dest))
                    {
                        if vectorize_trace_enabled() {
                            eprintln!(
                                "   [vec-expr-map] reject: duplicate StoreIndex1D destination"
                            );
                        }
                        return None;
                    }
                    if !expr_has_iv_dependency(fn_ir, *val, iv_phi) {
                        if vectorize_trace_enabled() {
                            eprintln!("   [vec-expr-map] reject: rhs has no IV dependency");
                        }
                        return None;
                    }
                    if !is_vectorizable_expr(fn_ir, *val, iv_phi, lp, true, false) {
                        if vectorize_trace_enabled() {
                            eprintln!("   [vec-expr-map] reject: rhs not vectorizable");
                        }
                        return None;
                    }
                    if expr_has_non_vector_safe_call_in_vector_context(
                        fn_ir,
                        *val,
                        iv_phi,
                        user_call_whitelist,
                        &mut FxHashSet::default(),
                    ) {
                        if vectorize_trace_enabled() {
                            let rhs_detail = match &fn_ir.values[*val].kind {
                                ValueKind::Binary { lhs, rhs, .. } => format!(
                                    "lhs={:?} rhs={:?}",
                                    fn_ir.values[*lhs].kind, fn_ir.values[*rhs].kind
                                ),
                                other => format!("{:?}", other),
                            };
                            eprintln!(
                                "   [vec-expr-map] reject: rhs contains non-vector-safe call; rhs={:?}; detail={}",
                                fn_ir.values[*val].kind, rhs_detail
                            );
                        }
                        return None;
                    }
                    if expr_reads_base_non_iv(fn_ir, *val, dest, iv_phi) {
                        if vectorize_trace_enabled() {
                            eprintln!(
                                "   [vec-expr-map] reject: loop-carried dependence on destination"
                            );
                        }
                        return None;
                    }
                    found.push((dest, *val));
                }
            }
        }
    }

    if found.is_empty() {
        return None;
    }
    let entries: Vec<ExprMapEntry> = found
        .into_iter()
        .map(|(dest, expr)| ExprMapEntry {
            dest,
            expr,
            whole_dest: is_loop_compatible_base(lp, fn_ir, dest) && is_const_one(fn_ir, start),
        })
        .collect();
    if entries.len() == 1 {
        let entry = entries[0];
        return Some(VectorPlan::ExprMap {
            dest: entry.dest,
            expr: entry.expr,
            iv_phi,
            start,
            end,
            whole_dest: entry.whole_dest,
        });
    }
    Some(VectorPlan::MultiExprMap {
        entries,
        iv_phi,
        start,
        end,
    })
}

fn match_scatter_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    if lp.body.len() > 2 {
        return None;
    }
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let mut found: Option<(ValueId, ValueId, ValueId)> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } | Instr::Eval { .. } => {}
                Instr::StoreIndex2D { .. } => return None,
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => {
                    if bid != lp.latch {
                        return None;
                    }
                    if *is_vector || found.is_some() {
                        return None;
                    }
                    found = Some((canonical_value(fn_ir, *base), *idx, *val));
                }
            }
        }
    }

    let (dest, idx, expr) = found?;
    if is_iv_equivalent(fn_ir, idx, iv_phi)
        || is_floor_like_iv_expr(
            fn_ir,
            idx,
            iv_phi,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
        )
    {
        return None;
    }
    if !expr_has_iv_dependency(fn_ir, idx, iv_phi) {
        return None;
    }
    if !is_vectorizable_expr(fn_ir, idx, iv_phi, lp, true, false)
        || !is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false)
    {
        return None;
    }
    if expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        idx,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    ) || expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        expr,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    ) {
        return None;
    }
    if expr_reads_base_non_iv(fn_ir, expr, dest, iv_phi) {
        return None;
    }
    Some(VectorPlan::ScatterExprMap {
        dest,
        idx,
        expr,
        iv_phi,
    })
}

fn match_cube_slice_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let trace_enabled = vectorize_trace_enabled();
    let mut found: Option<(ValueId, ValueId, ValueId)> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } | Instr::Eval { .. } => {}
                Instr::StoreIndex2D { .. } => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-cube-slice] {} reject: saw StoreIndex2D in loop body",
                            fn_ir.name
                        );
                    }
                    return None;
                }
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => {
                    if *is_vector || found.is_some() {
                        if trace_enabled {
                            eprintln!(
                                "   [vec-cube-slice] {} reject: non-canonical store count/vector flag (is_vector={}, found_already={})",
                                fn_ir.name,
                                is_vector,
                                found.is_some()
                            );
                        }
                        return None;
                    }
                    found = Some((canonical_value(fn_ir, *base), *idx, *val));
                }
            }
        }
    }

    let (dest, idx, expr) = found?;
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &fn_ir.values[canonical_value(fn_ir, idx)].kind
    else {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: store index is not call ({:?})",
                fn_ir.name,
                fn_ir.values[canonical_value(fn_ir, idx)].kind
            );
        }
        return None;
    };
    if callee != "rr_idx_cube_vec_i" || (args.len() != 4 && args.len() != 5) {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: index call shape mismatch (callee={}, arity={})",
                fn_ir.name,
                callee,
                args.len()
            );
        }
        return None;
    }
    if names.iter().any(|n| n.is_some()) {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: named args in rr_idx_cube_vec_i",
                fn_ir.name
            );
        }
        return None;
    }

    let y_arg = args[2];
    let y_ok = is_iv_equivalent(fn_ir, y_arg, iv_phi)
        || is_origin_var_iv_alias_in_loop(fn_ir, lp, y_arg, iv_phi)
        || is_floor_like_iv_expr(
            fn_ir,
            y_arg,
            iv_phi,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
        );
    if !y_ok {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: y arg is not IV-equivalent (iv={:?}, iv_origin={:?}, y={:?}, y_origin={:?})",
                fn_ir.name,
                fn_ir.values[iv_phi].kind,
                induction_origin_var(fn_ir, iv_phi),
                fn_ir.values[y_arg].kind,
                fn_ir.values[canonical_value(fn_ir, y_arg)].origin_var
            );
        }
        return None;
    }

    for arg in [args[0], args[1], args[3]] {
        if expr_has_iv_dependency(fn_ir, arg, iv_phi)
            || !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, user_call_whitelist)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cube-slice] {} reject: non-invariant cube arg ({:?}) dep={}",
                    fn_ir.name,
                    fn_ir.values[arg].kind,
                    expr_has_iv_dependency(fn_ir, arg, iv_phi)
                );
            }
            return None;
        }
    }
    let ctx = if args.len() == 5 {
        let arg = args[4];
        if expr_has_iv_dependency(fn_ir, arg, iv_phi)
            || !is_loop_invariant_scalar_expr(fn_ir, arg, iv_phi, user_call_whitelist)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cube-slice] {} reject: non-invariant cube ctx arg ({:?}) dep={}",
                    fn_ir.name,
                    fn_ir.values[arg].kind,
                    expr_has_iv_dependency(fn_ir, arg, iv_phi)
                );
            }
            return None;
        }
        Some(arg)
    } else {
        None
    };

    let expr_iv_dependent = expr_has_iv_dependency(fn_ir, expr, iv_phi);
    let expr_ok = if expr_iv_dependent {
        is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false)
    } else {
        is_loop_invariant_scalar_expr(fn_ir, expr, iv_phi, user_call_whitelist)
    };
    if !expr_ok {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: expr is neither vectorizable nor loop-invariant scalar ({:?})",
                fn_ir.name, fn_ir.values[expr].kind
            );
        }
        return None;
    }
    if expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        expr,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    ) {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: expr contains non-vector-safe call ({:?})",
                fn_ir.name, fn_ir.values[expr].kind
            );
        }
        return None;
    }
    if expr_reads_base_non_iv(fn_ir, expr, dest, iv_phi) {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: expr reads destination via non-IV access",
                fn_ir.name
            );
        }
        return None;
    }

    Some(VectorPlan::CubeSliceExprMap {
        dest,
        expr,
        iv_phi,
        face: args[0],
        row: args[1],
        size: args[3],
        ctx,
        start,
        end,
    })
}

pub(crate) fn is_builtin_vector_safe_call(callee: &str, arity: usize) -> bool {
    match callee {
        "abs" | "sqrt" | "exp" | "log" | "log10" | "log2" | "sin" | "cos" | "tan" | "asin"
        | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "sign" | "floor" | "ceiling" | "trunc"
        | "gamma" | "lgamma" | "is.na" | "is.finite" | "seq_len" => arity == 1,
        "rr_wrap_index_vec" | "rr_wrap_index_vec_i" => arity == 4 || arity == 5,
        "rr_idx_cube_vec_i" => arity == 4 || arity == 5,
        "rr_index_vec_floor" => arity == 1 || arity == 2,
        "rr_index1_read_vec" | "rr_index1_read_vec_floor" => arity == 2 || arity == 3,
        "atan2" => arity == 2,
        "round" => arity == 1 || arity == 2,
        "pmax" | "pmin" => arity >= 2,
        _ => false,
    }
}

fn trace_materialize_reject(fn_ir: &FnIR, root: ValueId, reason: &str) {
    if !vectorize_trace_enabled() {
        return;
    }
    eprintln!(
        "   [vec-materialize] {} reject {} => {:?}",
        fn_ir.name, reason, fn_ir.values[root].kind
    );
}

fn trace_value_tree(fn_ir: &FnIR, root: ValueId, indent: usize, seen: &mut FxHashSet<ValueId>) {
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
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            trace_value_tree(fn_ir, *base, indent + 2, seen);
        }
        ValueKind::Range { start, end } => {
            trace_value_tree(fn_ir, *start, indent + 2, seen);
            trace_value_tree(fn_ir, *end, indent + 2, seen);
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => {}
    }
}

fn trace_block_instrs(fn_ir: &FnIR, bid: BlockId, indent: usize) {
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

fn last_assign_to_var_in_block(fn_ir: &FnIR, bid: BlockId, var: &str) -> Option<ValueId> {
    let block = fn_ir.blocks.get(bid)?;
    for ins in block.instrs.iter().rev() {
        let Instr::Assign { dst, src, .. } = ins else {
            continue;
        };
        if dst == var {
            return Some(canonical_value(fn_ir, *src));
        }
    }
    None
}

fn block_successors(fn_ir: &FnIR, bid: BlockId) -> [Option<BlockId>; 2] {
    match fn_ir.blocks[bid].term {
        Terminator::Goto(target) => [Some(target), None],
        Terminator::If {
            then_bb, else_bb, ..
        } => [Some(then_bb), Some(else_bb)],
        Terminator::Return(_) | Terminator::Unreachable => [None, None],
    }
}

fn block_reaches_before_merge(
    fn_ir: &FnIR,
    start: BlockId,
    target: BlockId,
    merge_bb: BlockId,
) -> bool {
    let mut seen = FxHashSet::default();
    let mut stack = vec![start];
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        if bid == target {
            return true;
        }
        if bid == merge_bb {
            continue;
        }
        for succ in block_successors(fn_ir, bid).into_iter().flatten() {
            stack.push(succ);
        }
    }
    false
}

fn collect_if_ancestors_with_distance(
    fn_ir: &FnIR,
    preds: &FxHashMap<BlockId, Vec<BlockId>>,
    start: BlockId,
) -> FxHashMap<BlockId, usize> {
    let mut out = FxHashMap::default();
    let mut seen = FxHashSet::default();
    let mut queue = std::collections::VecDeque::from([(start, 0usize)]);
    while let Some((bid, dist)) = queue.pop_front() {
        if !seen.insert(bid) {
            continue;
        }
        if matches!(fn_ir.blocks[bid].term, Terminator::If { .. }) {
            out.entry(bid).or_insert(dist);
        }
        if let Some(ps) = preds.get(&bid) {
            for pred in ps {
                queue.push_back((*pred, dist + 1));
            }
        }
    }
    out
}

#[allow(dead_code)]
fn find_conditional_phi_shape_with_blocks(
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

fn find_conditional_phi_shape(
    fn_ir: &FnIR,
    root: ValueId,
    args: &[(ValueId, BlockId)],
) -> Option<(ValueId, ValueId, ValueId)> {
    find_conditional_phi_shape_with_blocks(fn_ir, root, args)
        .map(|(_, cond, then_val, _, else_val, _)| (cond, then_val, else_val))
}

fn branch_origin_for_merge(
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

fn is_passthrough_load_of_var(fn_ir: &FnIR, src: ValueId, var: &str) -> bool {
    matches!(
        &fn_ir.values[canonical_value(fn_ir, src)].kind,
        ValueKind::Load { var: load_var } if load_var == var
    )
}

fn is_prior_origin_phi_state(fn_ir: &FnIR, src: ValueId, var: &str, before_bb: BlockId) -> bool {
    let src = canonical_value(fn_ir, src);
    matches!(&fn_ir.values[src].kind, ValueKind::Phi { args } if !args.is_empty())
        && fn_ir.values[src].origin_var.as_deref() == Some(var)
        && fn_ir.values[src]
            .phi_block
            .is_some_and(|phi_bb| phi_bb < before_bb)
}

fn collapse_prior_origin_phi_state(
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

fn value_depends_on(
    fn_ir: &FnIR,
    root: ValueId,
    target: ValueId,
    visiting: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    let target = canonical_value(fn_ir, target);
    if root == target {
        return true;
    }
    if !visiting.insert(root) {
        return false;
    }
    let out = match &fn_ir.values[root].kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            value_depends_on(fn_ir, *lhs, target, visiting)
                || value_depends_on(fn_ir, *rhs, target, visiting)
        }
        ValueKind::Unary { rhs, .. } => value_depends_on(fn_ir, *rhs, target, visiting),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| value_depends_on(fn_ir, *arg, target, visiting)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(arg, _)| value_depends_on(fn_ir, *arg, target, visiting)),
        ValueKind::Index1D { base, idx, .. } => {
            value_depends_on(fn_ir, *base, target, visiting)
                || value_depends_on(fn_ir, *idx, target, visiting)
        }
        ValueKind::Index2D { base, r, c } => {
            value_depends_on(fn_ir, *base, target, visiting)
                || value_depends_on(fn_ir, *r, target, visiting)
                || value_depends_on(fn_ir, *c, target, visiting)
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            value_depends_on(fn_ir, *base, target, visiting)
        }
        ValueKind::Range { start, end } => {
            value_depends_on(fn_ir, *start, target, visiting)
                || value_depends_on(fn_ir, *end, target, visiting)
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => false,
    };
    visiting.remove(&root);
    out
}

fn block_instr_uses_value(fn_ir: &FnIR, ins: &Instr, vid: ValueId) -> bool {
    let uses = |value: ValueId| value_depends_on(fn_ir, value, vid, &mut FxHashSet::default());
    match ins {
        Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => uses(*src),
        Instr::StoreIndex1D { base, idx, val, .. } => uses(*base) || uses(*idx) || uses(*val),
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => uses(*base) || uses(*r) || uses(*c) || uses(*val),
    }
}

fn block_term_uses_value(fn_ir: &FnIR, term: &Terminator, vid: ValueId) -> bool {
    let uses = |value: ValueId| value_depends_on(fn_ir, value, vid, &mut FxHashSet::default());
    match term {
        Terminator::If { cond, .. } => uses(*cond),
        Terminator::Return(Some(ret)) => uses(*ret),
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => false,
    }
}

fn last_effective_assign_before_value_use_in_block(
    fn_ir: &FnIR,
    bid: BlockId,
    var: &str,
    vid: ValueId,
) -> Option<ValueId> {
    let vid = canonical_value(fn_ir, vid);
    let block = fn_ir.blocks.get(bid)?;
    for (idx, ins) in block.instrs.iter().enumerate() {
        if !block_instr_uses_value(fn_ir, ins, vid) {
            continue;
        }
        for prev in block.instrs[..idx].iter().rev() {
            let Instr::Assign { dst, src, .. } = prev else {
                continue;
            };
            if dst != var {
                continue;
            }
            if is_passthrough_load_of_var(fn_ir, *src, var) {
                continue;
            }
            return Some(canonical_value(fn_ir, *src));
        }
        return None;
    }
    if !block_term_uses_value(fn_ir, &block.term, vid) {
        return None;
    }
    for prev in block.instrs.iter().rev() {
        let Instr::Assign { dst, src, .. } = prev else {
            continue;
        };
        if dst != var {
            continue;
        }
        if is_passthrough_load_of_var(fn_ir, *src, var) {
            continue;
        }
        return Some(canonical_value(fn_ir, *src));
    }
    None
}

fn intrinsic_for_call(callee: &str, arity: usize) -> Option<IntrinsicOp> {
    match (callee, arity) {
        ("abs", 1) => Some(IntrinsicOp::VecAbsF64),
        ("log", 1) => Some(IntrinsicOp::VecLogF64),
        ("sqrt", 1) => Some(IntrinsicOp::VecSqrtF64),
        ("pmax", 2) => Some(IntrinsicOp::VecPmaxF64),
        ("pmin", 2) => Some(IntrinsicOp::VecPminF64),
        ("sum", 1) => Some(IntrinsicOp::VecSumF64),
        ("mean", 1) => Some(IntrinsicOp::VecMeanF64),
        _ => None,
    }
}

fn is_vector_safe_call(
    callee: &str,
    arity: usize,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    is_builtin_vector_safe_call(callee, arity) || user_call_whitelist.contains(callee)
}

fn is_const_number(fn_ir: &FnIR, vid: ValueId) -> bool {
    matches!(
        fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(Lit::Int(_)) | ValueKind::Const(Lit::Float(_))
    )
}

fn is_const_one(fn_ir: &FnIR, vid: ValueId) -> bool {
    match fn_ir.values[canonical_value(fn_ir, vid)].kind {
        ValueKind::Const(Lit::Int(n)) => n == 1,
        ValueKind::Const(Lit::Float(f)) => (f - 1.0).abs() < f64::EPSILON,
        _ => false,
    }
}

fn is_invariant_reduce_scalar(
    fn_ir: &FnIR,
    scalar: ValueId,
    iv_phi: ValueId,
    base: ValueId,
) -> bool {
    if expr_has_iv_dependency(fn_ir, scalar, iv_phi) || expr_reads_base(fn_ir, scalar, base) {
        return false;
    }
    match &fn_ir.values[canonical_value(fn_ir, scalar)].kind {
        ValueKind::Const(Lit::Int(_))
        | ValueKind::Const(Lit::Float(_))
        | ValueKind::Param { .. } => true,
        ValueKind::Load { var } => {
            if let Some(base_var) = resolve_base_var(fn_ir, base) {
                var != &base_var
            } else {
                true
            }
        }
        _ => false,
    }
}

fn is_loop_invariant_scalar_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        user_call_whitelist: &FxHashSet<String>,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return true;
        }
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) => true,
            ValueKind::Load { .. } | ValueKind::Param { .. } => {
                vector_length_key(fn_ir, root).is_none()
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, iv_phi, user_call_whitelist, seen),
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, user_call_whitelist, seen)
                    && rec(fn_ir, *rhs, iv_phi, user_call_whitelist, seen)
            }
            ValueKind::Len { base } => rec(fn_ir, *base, iv_phi, user_call_whitelist, seen),
            ValueKind::Call { callee, args, .. } => {
                is_vector_safe_call(callee, args.len(), user_call_whitelist)
                    && args
                        .iter()
                        .all(|a| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|a| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen)),
            ValueKind::Phi { args } => args
                .iter()
                .all(|(a, _)| rec(fn_ir, *a, iv_phi, user_call_whitelist, seen)),
            ValueKind::Index1D { .. }
            | ValueKind::Index2D { .. }
            | ValueKind::Range { .. }
            | ValueKind::Indices { .. } => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    )
}

fn is_vector_safe_call_chain_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        _lp: &LoopInfo,
        user_call_whitelist: &FxHashSet<String>,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            return true;
        }
        if !seen.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. } => true,
            ValueKind::Unary { rhs, .. } => {
                rec(fn_ir, *rhs, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, _lp, user_call_whitelist, seen)
                    && rec(fn_ir, *rhs, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Call { callee, args, .. } => {
                is_vector_safe_call(callee, args.len(), user_call_whitelist)
                    && args
                        .iter()
                        .all(|a| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen))
            }
            ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|a| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen)),
            ValueKind::Index1D {
                base: _base,
                idx,
                is_safe: _is_safe,
                is_na_safe: _is_na_safe,
            } => is_iv_equivalent(fn_ir, *idx, iv_phi),
            ValueKind::Phi { args } => args
                .iter()
                .all(|(a, _)| rec(fn_ir, *a, iv_phi, _lp, user_call_whitelist, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, *base, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, iv_phi, _lp, user_call_whitelist, seen)
                    && rec(fn_ir, *end, iv_phi, _lp, user_call_whitelist, seen)
            }
            ValueKind::Index2D { .. } => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        user_call_whitelist,
        &mut FxHashSet::default(),
    )
}

fn match_2d_row_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let start = iv.init_val;
    let end = lp.limit?;

    if !loop_is_simple_2d_map(fn_ir, lp) {
        return None;
    }

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest, row, col, rhs) = match instr {
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => (*base, *r, *c, *val),
                _ => continue,
            };
            if !is_loop_invariant_axis(fn_ir, row, iv_phi, dest) {
                continue;
            }
            if !is_iv_equivalent(fn_ir, col, iv_phi) || expr_has_iv_dependency(fn_ir, row, iv_phi) {
                continue;
            }

            let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[rhs].kind.clone() else {
                continue;
            };
            if !matches!(
                op,
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
            ) {
                continue;
            }
            let lhs_src = match row_operand_source(fn_ir, lhs, row, iv_phi) {
                Some(v) => v,
                None => continue,
            };
            let rhs_src = match row_operand_source(fn_ir, rhs, row, iv_phi) {
                Some(v) => v,
                None => continue,
            };

            return Some(VectorPlan::Map2DRow {
                dest: canonical_value(fn_ir, dest),
                row,
                start,
                end,
                lhs_src,
                rhs_src,
                op,
            });
        }
    }
    None
}

fn match_2d_col_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let start = iv.init_val;
    let end = lp.limit?;

    if !loop_is_simple_2d_map(fn_ir, lp) {
        return None;
    }

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest, row, col, rhs) = match instr {
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => (*base, *r, *c, *val),
                _ => continue,
            };
            if !is_loop_invariant_axis(fn_ir, col, iv_phi, dest) {
                continue;
            }
            if !is_iv_equivalent(fn_ir, row, iv_phi) || expr_has_iv_dependency(fn_ir, col, iv_phi) {
                continue;
            }

            let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[rhs].kind.clone() else {
                continue;
            };
            if !matches!(
                op,
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
            ) {
                continue;
            }
            let lhs_src = match col_operand_source(fn_ir, lhs, col, iv_phi) {
                Some(v) => v,
                None => continue,
            };
            let rhs_src = match col_operand_source(fn_ir, rhs, col, iv_phi) {
                Some(v) => v,
                None => continue,
            };

            return Some(VectorPlan::Map2DCol {
                dest: canonical_value(fn_ir, dest),
                col,
                start,
                end,
                lhs_src,
                rhs_src,
                op,
            });
        }
    }
    None
}

fn loop_is_simple_2d_map(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let mut store2d_count = 0usize;
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::StoreIndex2D { .. } => store2d_count += 1,
                Instr::StoreIndex1D { .. } | Instr::Eval { .. } => return false,
                Instr::Assign { .. } => {}
            }
        }
    }
    if store2d_count != 1 {
        return false;
    }
    true
}

fn row_operand_source(
    fn_ir: &FnIR,
    operand: ValueId,
    row: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    match &fn_ir.values[operand].kind {
        ValueKind::Index2D { base, r, c } => {
            if is_iv_equivalent(fn_ir, *c, iv_phi)
                && same_loop_invariant_value(fn_ir, *r, row, iv_phi)
            {
                Some(canonical_value(fn_ir, *base))
            } else {
                None
            }
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => Some(operand),
        _ => None,
    }
}

fn col_operand_source(
    fn_ir: &FnIR,
    operand: ValueId,
    col: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    match &fn_ir.values[operand].kind {
        ValueKind::Index2D { base, r, c } => {
            if is_iv_equivalent(fn_ir, *r, iv_phi)
                && same_loop_invariant_value(fn_ir, *c, col, iv_phi)
            {
                Some(canonical_value(fn_ir, *base))
            } else {
                None
            }
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => Some(operand),
        _ => None,
    }
}

fn same_loop_invariant_value(fn_ir: &FnIR, a: ValueId, b: ValueId, iv_phi: ValueId) -> bool {
    if is_value_equivalent(fn_ir, a, b) {
        return true;
    }
    if expr_has_iv_dependency(fn_ir, a, iv_phi) || expr_has_iv_dependency(fn_ir, b, iv_phi) {
        return false;
    }
    if let (ValueKind::Const(ca), ValueKind::Const(cb)) =
        (&fn_ir.values[a].kind, &fn_ir.values[b].kind)
    {
        return ca == cb;
    }
    match (
        fn_ir.values[a].origin_var.as_deref(),
        fn_ir.values[b].origin_var.as_deref(),
    ) {
        (Some(va), Some(vb)) => va == vb,
        _ => false,
    }
}

fn is_origin_var_iv_alias_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    candidate: ValueId,
    iv_phi: ValueId,
) -> bool {
    let candidate = canonical_value(fn_ir, candidate);
    let Some(origin_var) = fn_ir.values[candidate].origin_var.as_deref() else {
        return false;
    };
    let Some(src) = unique_assign_source_in_loop(fn_ir, lp, origin_var) else {
        return false;
    };
    is_iv_equivalent(fn_ir, src, iv_phi)
        || is_floor_like_iv_expr(
            fn_ir,
            src,
            iv_phi,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
        )
}

fn has_any_var_binding(fn_ir: &FnIR, var: &str) -> bool {
    if fn_ir.params.iter().any(|p| p == var) {
        return true;
    }
    fn_ir.blocks.iter().any(|bb| {
        bb.instrs.iter().any(|ins| match ins {
            Instr::Assign { dst, .. } => dst == var,
            _ => false,
        })
    })
}

fn is_loop_invariant_axis(fn_ir: &FnIR, axis: ValueId, iv_phi: ValueId, dest: ValueId) -> bool {
    if expr_has_iv_dependency(fn_ir, axis, iv_phi) {
        return false;
    }
    match &fn_ir.values[canonical_value(fn_ir, axis)].kind {
        ValueKind::Const(Lit::Int(_)) => true,
        ValueKind::Param { .. } => true,
        ValueKind::Load { var } => {
            if let Some(dest_var) = resolve_base_var(fn_ir, dest) {
                var != &dest_var && has_any_var_binding(fn_ir, var)
            } else {
                has_any_var_binding(fn_ir, var)
            }
        }
        _ => false,
    }
}

fn as_safe_loop_index(fn_ir: &FnIR, vid: ValueId, iv_phi: ValueId) -> Option<ValueId> {
    if let ValueKind::Index1D {
        base,
        idx,
        is_safe,
        is_na_safe,
    } = &fn_ir.values[vid].kind
        && is_iv_equivalent(fn_ir, *idx, iv_phi)
        && *is_safe
        && *is_na_safe
    {
        return Some(canonical_value(fn_ir, *base));
    }
    None
}

fn apply_vectorization(fn_ir: &mut FnIR, lp: &LoopInfo, plan: VectorPlan) -> bool {
    let preds = build_pred_map(fn_ir);
    let outer_preds: Vec<BlockId> = preds
        .get(&lp.header)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|b| !lp.body.contains(b))
        .collect();

    if outer_preds.len() != 1 {
        return false;
    }
    let preheader = outer_preds[0];
    let exit_bb = if lp.exits.len() == 1 {
        lp.exits[0]
    } else {
        return false;
    };

    match plan {
        VectorPlan::Reduce {
            kind,
            acc_phi,
            vec_expr,
            iv_phi,
        } => {
            let func = match kind {
                ReduceKind::Sum => "sum",
                ReduceKind::Prod => "prod",
                ReduceKind::Min => "min",
                ReduceKind::Max => "max",
            };

            let reduce_val = if kind == ReduceKind::Sum {
                if let Some(v) = rewrite_sum_add_const(fn_ir, lp, vec_expr, iv_phi) {
                    v
                } else {
                    let idx_vec = match build_loop_index_vector(fn_ir, lp) {
                        Some(v) => v,
                        None => return false,
                    };
                    let mut memo = FxHashMap::default();
                    let mut interner = FxHashMap::default();
                    let input_vec = match materialize_vector_expr(
                        fn_ir,
                        vec_expr,
                        iv_phi,
                        idx_vec,
                        lp,
                        &mut memo,
                        &mut interner,
                        false,
                        true,
                    ) {
                        Some(v) => v,
                        None => return false,
                    };
                    let reduce_kind = ValueKind::Call {
                        callee: func.to_string(),
                        args: vec![input_vec],
                        names: vec![None],
                    };
                    fn_ir.add_value(
                        reduce_kind,
                        crate::utils::Span::dummy(),
                        crate::mir::def::Facts::empty(),
                        None,
                    )
                }
            } else {
                let idx_vec = match build_loop_index_vector(fn_ir, lp) {
                    Some(v) => v,
                    None => return false,
                };
                let mut memo = FxHashMap::default();
                let mut interner = FxHashMap::default();
                let input_vec = match materialize_vector_expr(
                    fn_ir,
                    vec_expr,
                    iv_phi,
                    idx_vec,
                    lp,
                    &mut memo,
                    &mut interner,
                    false,
                    true,
                ) {
                    Some(v) => v,
                    None => return false,
                };
                let reduce_kind = ValueKind::Call {
                    callee: func.to_string(),
                    args: vec![input_vec],
                    names: vec![None],
                };
                fn_ir.add_value(
                    reduce_kind,
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                )
            };

            if let Some(acc_var) = fn_ir.values[acc_phi].origin_var.clone() {
                fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                    dst: acc_var.clone(),
                    src: reduce_val,
                    span: crate::utils::Span::dummy(),
                });
                rewrite_returns_for_var(fn_ir, &acc_var, reduce_val);
            } else {
                return false;
            }

            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            true
        }
        VectorPlan::Reduce2DRowSum {
            acc_phi,
            base,
            row,
            start,
            end,
        } => {
            let Some(acc_var) = fn_ir.values[acc_phi].origin_var.clone() else {
                return false;
            };
            let row_val = resolve_materialized_value(fn_ir, row);
            let reduce_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_row_sum_range".to_string(),
                    args: vec![base, row_val, start, end],
                    names: vec![None, None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: acc_var.clone(),
                src: reduce_val,
                span: crate::utils::Span::dummy(),
            });
            rewrite_returns_for_var(fn_ir, &acc_var, reduce_val);
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            true
        }
        VectorPlan::Reduce2DColSum {
            acc_phi,
            base,
            col,
            start,
            end,
        } => {
            let Some(acc_var) = fn_ir.values[acc_phi].origin_var.clone() else {
                return false;
            };
            let col_val = resolve_materialized_value(fn_ir, col);
            let reduce_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_col_sum_range".to_string(),
                    args: vec![base, col_val, start, end],
                    names: vec![None, None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: acc_var.clone(),
                src: reduce_val,
                span: crate::utils::Span::dummy(),
            });
            rewrite_returns_for_var(fn_ir, &acc_var, reduce_val);
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            true
        }
        VectorPlan::Map {
            dest,
            src,
            op,
            other,
        } => {
            // y = x * 2 is a binary operation on vectors in R
            let map_kind = ValueKind::Binary {
                op,
                lhs: src,
                rhs: other,
            };
            let map_val = fn_ir.add_value(
                map_kind,
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );

            if let Some(dest_var) = resolve_base_var(fn_ir, dest) {
                let ret_name = dest_var.clone();
                fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                    dst: dest_var,
                    src: map_val,
                    span: crate::utils::Span::dummy(),
                });

                fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);

                // If function exits return the destination variable directly,
                // keep semantics by returning the vectorized value.
                rewrite_returns_for_var(fn_ir, &ret_name, map_val);

                return true;
            }
            false
        }
        VectorPlan::CondMap {
            dest,
            cond,
            then_val,
            else_val,
            iv_phi,
        } => {
            let idx_vec = match build_loop_index_vector(fn_ir, lp) {
                Some(v) => v,
                None => return false,
            };
            let dest_ref = resolve_materialized_value(fn_ir, dest);
            let mut memo = FxHashMap::default();
            let mut interner = FxHashMap::default();
            let cond_vec = match materialize_vector_expr(
                fn_ir,
                cond,
                iv_phi,
                idx_vec,
                lp,
                &mut memo,
                &mut interner,
                true,
                true,
            ) {
                Some(v) => v,
                None => return false,
            };
            let then_vec = match materialize_vector_expr(
                fn_ir,
                then_val,
                iv_phi,
                idx_vec,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            ) {
                Some(v) => v,
                None => return false,
            };
            let else_vec = match materialize_vector_expr(
                fn_ir,
                else_val,
                iv_phi,
                idx_vec,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            ) {
                Some(v) => v,
                None => return false,
            };
            if !same_length_proven(fn_ir, dest_ref, cond_vec) {
                let check_val = fn_ir.add_value(
                    ValueKind::Call {
                        callee: "rr_same_len".to_string(),
                        args: vec![dest_ref, cond_vec],
                        names: vec![None, None],
                    },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                );
                fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                    val: check_val,
                    span: crate::utils::Span::dummy(),
                });
            }
            for branch_vec in [then_vec, else_vec] {
                if is_const_number(fn_ir, branch_vec) {
                    continue;
                }
                if same_length_proven(fn_ir, dest_ref, branch_vec) {
                    continue;
                }
                let check_val = fn_ir.add_value(
                    ValueKind::Call {
                        callee: "rr_same_or_scalar".to_string(),
                        args: vec![dest_ref, branch_vec],
                        names: vec![None, None],
                    },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                );
                fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                    val: check_val,
                    span: crate::utils::Span::dummy(),
                });
            }
            let ifelse_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_ifelse_strict".to_string(),
                    args: vec![cond_vec, then_vec, else_vec],
                    names: vec![None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
                return false;
            };
            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: dest_var.clone(),
                src: ifelse_val,
                span: crate::utils::Span::dummy(),
            });
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            rewrite_returns_for_var(fn_ir, &dest_var, ifelse_val);
            true
        }
        VectorPlan::RecurrenceAddConst {
            base,
            start,
            end,
            delta,
            negate_delta,
        } => {
            let Some(base_var) = resolve_base_var(fn_ir, base) else {
                return false;
            };
            let delta_val = if negate_delta {
                fn_ir.add_value(
                    ValueKind::Unary {
                        op: crate::syntax::ast::UnaryOp::Neg,
                        rhs: delta,
                    },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                )
            } else {
                delta
            };
            let recur_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_recur_add_const".to_string(),
                    args: vec![base, start, end, delta_val],
                    names: vec![None, None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: base_var.clone(),
                src: recur_val,
                span: crate::utils::Span::dummy(),
            });
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            rewrite_returns_for_var(fn_ir, &base_var, recur_val);
            true
        }
        VectorPlan::ShiftedMap {
            dest,
            src,
            start,
            end,
            offset,
        } => {
            let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
                return false;
            };
            let src_start = add_int_offset(fn_ir, start, offset);
            let src_end = add_int_offset(fn_ir, end, offset);
            let shifted_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_shift_assign".to_string(),
                    args: vec![dest, src, start, end, src_start, src_end],
                    names: vec![None, None, None, None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: dest_var.clone(),
                src: shifted_val,
                span: crate::utils::Span::dummy(),
            });
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            rewrite_returns_for_var(fn_ir, &dest_var, shifted_val);
            true
        }
        VectorPlan::CallMap {
            dest,
            callee,
            args,
            iv_phi,
        } => {
            let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
                return false;
            };
            let idx_vec = match build_loop_index_vector(fn_ir, lp) {
                Some(v) => v,
                None => return false,
            };
            let mut memo = FxHashMap::default();
            let mut interner = FxHashMap::default();
            let mut mapped_args = Vec::with_capacity(args.len());
            let mut vector_args = Vec::new();
            for (arg_i, arg) in args.into_iter().enumerate() {
                let out = if arg.vectorized {
                    match materialize_vector_expr(
                        fn_ir,
                        arg.value,
                        iv_phi,
                        idx_vec,
                        lp,
                        &mut memo,
                        &mut interner,
                        true,
                        false,
                    ) {
                        Some(v) => v,
                        None => return false,
                    }
                } else {
                    resolve_materialized_value(fn_ir, arg.value)
                };
                let out = if arg.vectorized {
                    maybe_hoist_callmap_arg_expr(fn_ir, preheader, out, arg_i)
                } else {
                    out
                };
                if arg.vectorized {
                    vector_args.push(out);
                }
                mapped_args.push((out, arg.vectorized));
            }

            for (a, is_vec) in &mapped_args {
                if *is_vec {
                    if !same_length_proven(fn_ir, dest, *a) {
                        let check_val = fn_ir.add_value(
                            ValueKind::Call {
                                callee: "rr_same_len".to_string(),
                                args: vec![dest, *a],
                                names: vec![None, None],
                            },
                            crate::utils::Span::dummy(),
                            crate::mir::def::Facts::empty(),
                            None,
                        );
                        fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                            val: check_val,
                            span: crate::utils::Span::dummy(),
                        });
                    }
                } else if !is_const_number(fn_ir, *a) {
                    // Invariant args must still be scalar or length-compatible at runtime.
                    let check_val = fn_ir.add_value(
                        ValueKind::Call {
                            callee: "rr_same_or_scalar".to_string(),
                            args: vec![dest, *a],
                            names: vec![None, None],
                        },
                        crate::utils::Span::dummy(),
                        crate::mir::def::Facts::empty(),
                        None,
                    );
                    fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                        val: check_val,
                        span: crate::utils::Span::dummy(),
                    });
                }
            }

            for i in 0..vector_args.len() {
                for j in (i + 1)..vector_args.len() {
                    let a = vector_args[i];
                    let b = vector_args[j];
                    if same_length_proven(fn_ir, a, b) {
                        continue;
                    }
                    let check_val = fn_ir.add_value(
                        ValueKind::Call {
                            callee: "rr_same_len".to_string(),
                            args: vec![a, b],
                            names: vec![None, None],
                        },
                        crate::utils::Span::dummy(),
                        crate::mir::def::Facts::empty(),
                        None,
                    );
                    fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                        val: check_val,
                        span: crate::utils::Span::dummy(),
                    });
                }
            }
            let mapped_args_vals: Vec<ValueId> = mapped_args.iter().map(|(a, _)| *a).collect();
            let mapped_kind = if let Some(op) = intrinsic_for_call(&callee, mapped_args_vals.len())
            {
                ValueKind::Intrinsic {
                    op,
                    args: mapped_args_vals,
                }
            } else {
                ValueKind::Call {
                    callee: callee.clone(),
                    args: mapped_args_vals,
                    names: vec![None; mapped_args.len()],
                }
            };
            let mapped_val = fn_ir.add_value(
                mapped_kind,
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: dest_var.clone(),
                src: mapped_val,
                span: crate::utils::Span::dummy(),
            });
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            rewrite_returns_for_var(fn_ir, &dest_var, mapped_val);
            true
        }
        VectorPlan::CubeSliceExprMap {
            dest,
            expr,
            iv_phi,
            face,
            row,
            size,
            ctx,
            start,
            end,
        } => {
            let trace_enabled = vectorize_trace_enabled();
            let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
                if trace_enabled {
                    eprintln!(
                        "   [vec-apply-expr] {} fail: destination has no base var",
                        fn_ir.name
                    );
                }
                return false;
            };
            let idx_vec = match build_loop_index_vector(fn_ir, lp) {
                Some(v) => v,
                None => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: no loop index vector",
                            fn_ir.name
                        );
                    }
                    return false;
                }
            };
            let mut memo = FxHashMap::default();
            let mut interner = FxHashMap::default();
            let face = match materialize_loop_invariant_scalar_expr(
                fn_ir,
                face,
                iv_phi,
                lp,
                &mut memo,
                &mut interner,
            ) {
                Some(v) => resolve_materialized_value(fn_ir, v),
                None => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: invariant scalar face materialization ({:?})",
                            fn_ir.name, fn_ir.values[face].kind
                        );
                    }
                    return false;
                }
            };
            let row = match materialize_loop_invariant_scalar_expr(
                fn_ir,
                row,
                iv_phi,
                lp,
                &mut memo,
                &mut interner,
            ) {
                Some(v) => resolve_materialized_value(fn_ir, v),
                None => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: invariant scalar row materialization ({:?})",
                            fn_ir.name, fn_ir.values[row].kind
                        );
                    }
                    return false;
                }
            };
            let size = match materialize_loop_invariant_scalar_expr(
                fn_ir,
                size,
                iv_phi,
                lp,
                &mut memo,
                &mut interner,
            ) {
                Some(v) => resolve_materialized_value(fn_ir, v),
                None => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: invariant scalar size materialization ({:?})",
                            fn_ir.name, fn_ir.values[size].kind
                        );
                    }
                    return false;
                }
            };
            let ctx = match ctx {
                Some(ctx_val) => match materialize_loop_invariant_scalar_expr(
                    fn_ir,
                    ctx_val,
                    iv_phi,
                    lp,
                    &mut memo,
                    &mut interner,
                ) {
                    Some(v) => Some(resolve_materialized_value(fn_ir, v)),
                    None => {
                        if trace_enabled {
                            eprintln!(
                                "   [vec-apply-expr] {} fail: invariant scalar ctx materialization ({:?})",
                                fn_ir.name, fn_ir.values[ctx_val].kind
                            );
                        }
                        return false;
                    }
                },
                None => None,
            };
            let expr_vec = if expr_has_iv_dependency(fn_ir, expr, iv_phi) {
                match materialize_vector_expr(
                    fn_ir,
                    expr,
                    iv_phi,
                    idx_vec,
                    lp,
                    &mut memo,
                    &mut interner,
                    true,
                    false,
                ) {
                    Some(v) => v,
                    None => {
                        if trace_enabled {
                            eprintln!(
                                "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                                fn_ir.name, fn_ir.values[expr].kind
                            );
                        }
                        return false;
                    }
                }
            } else {
                match materialize_loop_invariant_scalar_expr(
                    fn_ir,
                    expr,
                    iv_phi,
                    lp,
                    &mut memo,
                    &mut interner,
                ) {
                    Some(v) => v,
                    None => {
                        if trace_enabled {
                            eprintln!(
                                "   [vec-apply-expr] {} fail: invariant scalar expr materialization ({:?})",
                                fn_ir.name, fn_ir.values[expr].kind
                            );
                        }
                        return false;
                    }
                }
            };
            let expr_vec = if is_scalar_broadcast_value(fn_ir, expr_vec) {
                let slice_len = if is_const_one(fn_ir, start) {
                    end
                } else {
                    let span = crate::utils::Span::dummy();
                    let facts = crate::mir::def::Facts::empty();
                    let span_delta = fn_ir.add_value(
                        ValueKind::Binary {
                            op: crate::syntax::ast::BinOp::Sub,
                            lhs: end,
                            rhs: start,
                        },
                        span,
                        facts,
                        None,
                    );
                    let one_val =
                        fn_ir.add_value(ValueKind::Const(Lit::Float(1.0)), span, facts, None);
                    fn_ir.add_value(
                        ValueKind::Binary {
                            op: crate::syntax::ast::BinOp::Add,
                            lhs: span_delta,
                            rhs: one_val,
                        },
                        span,
                        facts,
                        None,
                    )
                };
                fn_ir.add_value(
                    ValueKind::Call {
                        callee: "rep.int".to_string(),
                        args: vec![expr_vec, slice_len],
                        names: vec![None, None],
                    },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                )
            } else {
                expr_vec
            };

            let mut start_args = vec![face, row, start, size];
            let mut end_args = vec![face, row, end, size];
            if let Some(ctx) = ctx {
                start_args.push(ctx);
                end_args.push(ctx);
            }
            let slice_start = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_idx_cube_vec_i".to_string(),
                    args: start_args,
                    names: vec![None; if ctx.is_some() { 5 } else { 4 }],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            let slice_end = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_idx_cube_vec_i".to_string(),
                    args: end_args,
                    names: vec![None; if ctx.is_some() { 5 } else { 4 }],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            let out_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_assign_slice".to_string(),
                    args: vec![dest, slice_start, slice_end, expr_vec],
                    names: vec![None, None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: dest_var.clone(),
                src: out_val,
                span: crate::utils::Span::dummy(),
            });
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            rewrite_returns_for_var(fn_ir, &dest_var, out_val);
            true
        }
        VectorPlan::ExprMap {
            dest,
            expr,
            iv_phi,
            start,
            end,
            whole_dest,
        } => {
            let trace_enabled = vectorize_trace_enabled();
            let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
                if trace_enabled {
                    eprintln!(
                        "   [vec-apply-expr] {} fail: destination has no base var",
                        fn_ir.name
                    );
                }
                return false;
            };
            let idx_vec = match build_loop_index_vector(fn_ir, lp) {
                Some(v) => v,
                None => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: no loop index vector",
                            fn_ir.name
                        );
                    }
                    return false;
                }
            };
            let mut memo = FxHashMap::default();
            let mut interner = FxHashMap::default();
            let expr_vec = match materialize_vector_expr(
                fn_ir,
                expr,
                iv_phi,
                idx_vec,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            ) {
                Some(v) => v,
                None => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                            fn_ir.name, fn_ir.values[expr].kind
                        );
                    }
                    return false;
                }
            };

            let out_val = if whole_dest {
                if !same_length_proven(fn_ir, dest, expr_vec) {
                    let check_val = fn_ir.add_value(
                        ValueKind::Call {
                            callee: "rr_same_len".to_string(),
                            args: vec![dest, expr_vec],
                            names: vec![None, None],
                        },
                        crate::utils::Span::dummy(),
                        crate::mir::def::Facts::empty(),
                        None,
                    );
                    fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                        val: check_val,
                        span: crate::utils::Span::dummy(),
                    });
                }
                expr_vec
            } else {
                fn_ir.add_value(
                    ValueKind::Call {
                        callee: "rr_assign_slice".to_string(),
                        args: vec![dest, start, end, expr_vec],
                        names: vec![None, None, None, None],
                    },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                )
            };

            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: dest_var.clone(),
                src: out_val,
                span: crate::utils::Span::dummy(),
            });
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            rewrite_returns_for_var(fn_ir, &dest_var, out_val);
            true
        }
        VectorPlan::MultiExprMap {
            entries,
            iv_phi,
            start,
            end,
        } => {
            let trace_enabled = vectorize_trace_enabled();
            let idx_vec = match build_loop_index_vector(fn_ir, lp) {
                Some(v) => v,
                None => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: no loop index vector",
                            fn_ir.name
                        );
                    }
                    return false;
                }
            };
            let mut memo = FxHashMap::default();
            let mut interner = FxHashMap::default();
            let mut staged: Vec<(VarId, ValueId)> = Vec::with_capacity(entries.len());
            let mut staged_exprs: Vec<(ExprMapEntry, VarId, ValueId)> =
                Vec::with_capacity(entries.len());
            for (entry_index, entry) in entries.iter().copied().enumerate() {
                let Some(dest_var) = resolve_base_var(fn_ir, entry.dest) else {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-apply-expr] {} fail: destination has no base var",
                            fn_ir.name
                        );
                    }
                    return false;
                };
                let expr_vec = match materialize_vector_expr(
                    fn_ir,
                    entry.expr,
                    iv_phi,
                    idx_vec,
                    lp,
                    &mut memo,
                    &mut interner,
                    true,
                    false,
                ) {
                    Some(v) => v,
                    None => {
                        if trace_enabled {
                            eprintln!(
                                "   [vec-apply-expr] {} fail: materialize_vector_expr({:?})",
                                fn_ir.name, fn_ir.values[entry.expr].kind
                            );
                        }
                        return false;
                    }
                };
                let expr_vec = hoist_vector_expr_temp(
                    fn_ir,
                    preheader,
                    expr_vec,
                    &format!("exprmap{}", entry_index),
                );
                staged_exprs.push((entry, dest_var, expr_vec));
            }

            for (entry, dest_var, expr_vec) in staged_exprs {
                let out_val = if entry.whole_dest {
                    if !same_length_proven(fn_ir, entry.dest, expr_vec) {
                        let check_val = fn_ir.add_value(
                            ValueKind::Call {
                                callee: "rr_same_len".to_string(),
                                args: vec![entry.dest, expr_vec],
                                names: vec![None, None],
                            },
                            crate::utils::Span::dummy(),
                            crate::mir::def::Facts::empty(),
                            None,
                        );
                        fn_ir.blocks[preheader].instrs.push(Instr::Eval {
                            val: check_val,
                            span: crate::utils::Span::dummy(),
                        });
                    }
                    expr_vec
                } else {
                    fn_ir.add_value(
                        ValueKind::Call {
                            callee: "rr_assign_slice".to_string(),
                            args: vec![entry.dest, start, end, expr_vec],
                            names: vec![None, None, None, None],
                        },
                        crate::utils::Span::dummy(),
                        crate::mir::def::Facts::empty(),
                        None,
                    )
                };
                staged.push((dest_var, out_val));
            }

            for (dest_var, out_val) in staged {
                fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                    dst: dest_var.clone(),
                    src: out_val,
                    span: crate::utils::Span::dummy(),
                });
                rewrite_returns_for_var(fn_ir, &dest_var, out_val);
            }
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            true
        }
        VectorPlan::ScatterExprMap {
            dest,
            idx,
            expr,
            iv_phi,
        } => {
            let idx_seed = match build_loop_index_vector(fn_ir, lp) {
                Some(v) => v,
                None => return false,
            };
            let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
                return false;
            };
            let mut memo = FxHashMap::default();
            let mut interner = FxHashMap::default();
            let idx_vec = match materialize_vector_expr(
                fn_ir,
                idx,
                iv_phi,
                idx_seed,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            ) {
                Some(v) => hoist_vector_expr_temp(fn_ir, preheader, v, "scatter_idx"),
                None => return false,
            };
            let expr_vec = match materialize_vector_expr(
                fn_ir,
                expr,
                iv_phi,
                idx_seed,
                lp,
                &mut memo,
                &mut interner,
                true,
                false,
            ) {
                Some(v) => hoist_vector_expr_temp(fn_ir, preheader, v, "scatter_val"),
                None => return false,
            };
            let out_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_assign_index_vec".to_string(),
                    args: vec![dest, idx_vec, expr_vec],
                    names: vec![None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: dest_var.clone(),
                src: out_val,
                span: crate::utils::Span::dummy(),
            });
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            rewrite_returns_for_var(fn_ir, &dest_var, out_val);
            true
        }
        VectorPlan::Map2DRow {
            dest,
            row,
            start,
            end,
            lhs_src,
            rhs_src,
            op,
        } => {
            let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
                return false;
            };
            let op_sym = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%%",
                _ => return false,
            };
            let op_lit = fn_ir.add_value(
                ValueKind::Const(Lit::Str(op_sym.to_string())),
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            let row_val = resolve_materialized_value(fn_ir, row);
            let row_map_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_row_binop_assign".to_string(),
                    args: vec![dest, lhs_src, rhs_src, row_val, start, end, op_lit],
                    names: vec![None, None, None, None, None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: dest_var.clone(),
                src: row_map_val,
                span: crate::utils::Span::dummy(),
            });
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            rewrite_returns_for_var(fn_ir, &dest_var, row_map_val);
            true
        }
        VectorPlan::Map2DCol {
            dest,
            col,
            start,
            end,
            lhs_src,
            rhs_src,
            op,
        } => {
            let Some(dest_var) = resolve_base_var(fn_ir, dest) else {
                return false;
            };
            let op_sym = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%%",
                _ => return false,
            };
            let op_lit = fn_ir.add_value(
                ValueKind::Const(Lit::Str(op_sym.to_string())),
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            let col_val = resolve_materialized_value(fn_ir, col);
            let col_map_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_col_binop_assign".to_string(),
                    args: vec![dest, lhs_src, rhs_src, col_val, start, end, op_lit],
                    names: vec![None, None, None, None, None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            fn_ir.blocks[preheader].instrs.push(Instr::Assign {
                dst: dest_var.clone(),
                src: col_map_val,
                span: crate::utils::Span::dummy(),
            });
            fn_ir.blocks[preheader].term = Terminator::Goto(exit_bb);
            rewrite_returns_for_var(fn_ir, &dest_var, col_map_val);
            true
        }
    }
}

fn rewrite_sum_add_const(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    vec_expr: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    let root = canonical_value(fn_ir, vec_expr);
    let ValueKind::Binary {
        op: BinOp::Add,
        lhs,
        rhs,
    } = fn_ir.values[root].kind
    else {
        return None;
    };

    let (base, cst) = if let Some(base) = as_safe_loop_index(fn_ir, lhs, iv_phi) {
        if is_invariant_reduce_scalar(fn_ir, rhs, iv_phi, base) {
            (base, rhs)
        } else {
            return None;
        }
    } else if let Some(base) = as_safe_loop_index(fn_ir, rhs, iv_phi) {
        if is_invariant_reduce_scalar(fn_ir, lhs, iv_phi, base) {
            (base, lhs)
        } else {
            return None;
        }
    } else {
        return None;
    };

    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) != Some(canonical_value(fn_ir, base)) {
        return None;
    }

    let sum_base = fn_ir.add_value(
        ValueKind::Call {
            callee: "sum".to_string(),
            args: vec![base],
            names: vec![None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let len_base = fn_ir.add_value(
        ValueKind::Len { base },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let cst = resolve_materialized_value(fn_ir, cst);
    let c_times_n = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Mul,
            lhs: cst,
            rhs: len_base,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some(fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: sum_base,
            rhs: c_times_n,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    ))
}

fn affine_iv_offset(fn_ir: &FnIR, idx: ValueId, iv_phi: ValueId) -> Option<i64> {
    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        return Some(0);
    }
    match &fn_ir.values[idx].kind {
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind
            {
                return Some(k);
            }
            if is_iv_equivalent(fn_ir, *rhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*lhs].kind
            {
                return Some(k);
            }
            None
        }
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi)
                && let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind
            {
                return Some(-k);
            }
            None
        }
        _ => None,
    }
}

fn add_int_offset(fn_ir: &mut FnIR, base: ValueId, offset: i64) -> ValueId {
    if offset == 0 {
        return base;
    }
    if let ValueKind::Const(Lit::Int(n)) = fn_ir.values[base].kind {
        return fn_ir.add_value(
            ValueKind::Const(Lit::Int(n + offset)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
    }
    let k = fn_ir.add_value(
        ValueKind::Const(Lit::Int(offset)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: base,
            rhs: k,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

fn loop_matches_vec(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    let base = canonical_value(fn_ir, base);
    if lp.is_seq_along.map(|b| canonical_value(fn_ir, b)) == Some(base) {
        return true;
    }
    if let Some(loop_base) = lp.is_seq_along
        && let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, loop_base),
        )
        && a == b
    {
        return true;
    }
    if let Some(limit) = lp.is_seq_len
        && let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind
    {
        if canonical_value(fn_ir, len_base) == base {
            return true;
        }
        if let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, len_base),
        ) && a == b
        {
            return true;
        }
    }
    false
}

fn loop_has_store_effect(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            if matches!(
                instr,
                Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. }
            ) {
                return true;
            }
        }
    }
    false
}

fn loop_has_non_iv_loop_carried_state_except(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    allowed_vars: &[VarId],
) -> bool {
    let Some(iv) = lp.iv.as_ref() else {
        return false;
    };
    let iv_phi = iv.phi_val;
    for value in &fn_ir.values {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) {
            continue;
        }
        let has_loop_pred = args.iter().any(|(_, pred)| lp.body.contains(pred));
        let has_outer_pred = args.iter().any(|(_, pred)| !lp.body.contains(pred));
        if !has_loop_pred || !has_outer_pred {
            continue;
        }
        if is_iv_equivalent(fn_ir, value.id, iv_phi)
            || is_origin_var_iv_alias_in_loop(fn_ir, lp, value.id, iv_phi)
        {
            continue;
        }
        let Some(origin_var) = value.origin_var.as_deref() else {
            continue;
        };
        if allowed_vars
            .iter()
            .any(|allowed| origin_var == allowed.as_str())
        {
            continue;
        }
        if !var_observed_outside_loop(fn_ir, lp, origin_var) {
            continue;
        }
        return true;
    }
    false
}

fn var_observed_outside_loop(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> bool {
    let mut reachable_post_loop = FxHashSet::default();
    let mut stack: Vec<BlockId> = lp.exits.clone();
    while let Some(bid) = stack.pop() {
        if lp.body.contains(&bid) || !reachable_post_loop.insert(bid) {
            continue;
        }
        for succ in block_successors(fn_ir, bid).into_iter().flatten() {
            if !lp.body.contains(&succ) {
                stack.push(succ);
            }
        }
    }

    for bid in reachable_post_loop {
        let block = &fn_ir.blocks[bid];
        for instr in &block.instrs {
            let observed = match instr {
                Instr::Assign { src, .. } => {
                    value_depends_on_origin_var(fn_ir, *src, var, &mut FxHashSet::default())
                }
                Instr::Eval { val, .. } => {
                    value_depends_on_origin_var(fn_ir, *val, var, &mut FxHashSet::default())
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    value_depends_on_origin_var(fn_ir, *base, var, &mut FxHashSet::default())
                        || value_depends_on_origin_var(fn_ir, *idx, var, &mut FxHashSet::default())
                        || value_depends_on_origin_var(fn_ir, *val, var, &mut FxHashSet::default())
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    value_depends_on_origin_var(fn_ir, *base, var, &mut FxHashSet::default())
                        || value_depends_on_origin_var(fn_ir, *r, var, &mut FxHashSet::default())
                        || value_depends_on_origin_var(fn_ir, *c, var, &mut FxHashSet::default())
                        || value_depends_on_origin_var(fn_ir, *val, var, &mut FxHashSet::default())
                }
            };
            if observed {
                return true;
            }
        }
        let observed = match &block.term {
            Terminator::If { cond, .. } => {
                value_depends_on_origin_var(fn_ir, *cond, var, &mut FxHashSet::default())
            }
            Terminator::Return(Some(val)) => {
                value_depends_on_origin_var(fn_ir, *val, var, &mut FxHashSet::default())
            }
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => false,
        };
        if observed {
            return true;
        }
    }
    false
}

fn value_depends_on_origin_var(
    fn_ir: &FnIR,
    root: ValueId,
    var: &str,
    visited: &mut FxHashSet<ValueId>,
) -> bool {
    if !visited.insert(root) {
        return false;
    }
    let value = &fn_ir.values[root];
    if value.origin_var.as_deref() == Some(var) {
        return true;
    }
    match &value.kind {
        ValueKind::Load { var: load_var } => load_var == var,
        ValueKind::Phi { args } => args
            .iter()
            .any(|(src, _)| value_depends_on_origin_var(fn_ir, *src, var, visited)),
        ValueKind::Len { base }
        | ValueKind::Indices { base }
        | ValueKind::Unary { rhs: base, .. } => {
            value_depends_on_origin_var(fn_ir, *base, var, visited)
        }
        ValueKind::Range { start, end }
        | ValueKind::Binary {
            lhs: start,
            rhs: end,
            ..
        } => {
            value_depends_on_origin_var(fn_ir, *start, var, visited)
                || value_depends_on_origin_var(fn_ir, *end, var, visited)
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| value_depends_on_origin_var(fn_ir, *arg, var, visited)),
        ValueKind::Index1D { base, idx, .. } => {
            value_depends_on_origin_var(fn_ir, *base, var, visited)
                || value_depends_on_origin_var(fn_ir, *idx, var, visited)
        }
        ValueKind::Index2D { base, r, c } => {
            value_depends_on_origin_var(fn_ir, *base, var, visited)
                || value_depends_on_origin_var(fn_ir, *r, var, visited)
                || value_depends_on_origin_var(fn_ir, *c, var, visited)
        }
        ValueKind::Const(_) | ValueKind::Param { .. } => false,
    }
}

fn loop_vectorize_skip_reason(fn_ir: &FnIR, lp: &LoopInfo) -> VectorizeSkipReason {
    if lp.iv.is_none() {
        return VectorizeSkipReason::NoIv;
    }
    if lp.is_seq_len.is_none() {
        return VectorizeSkipReason::NonCanonicalBound;
    }
    if loop_has_unsupported_cfg_shape(fn_ir, lp) {
        return VectorizeSkipReason::UnsupportedCfgShape;
    }
    if loop_has_indirect_index_access(fn_ir, lp) {
        return VectorizeSkipReason::IndirectIndexAccess;
    }
    if loop_has_store_effect(fn_ir, lp) {
        return VectorizeSkipReason::StoreEffects;
    }
    VectorizeSkipReason::NoSupportedPattern
}

fn loop_has_unsupported_cfg_shape(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let preds = build_pred_map(fn_ir);
    let outer_preds = preds
        .get(&lp.header)
        .map(|ps| ps.iter().filter(|b| !lp.body.contains(b)).count())
        .unwrap_or(0);
    outer_preds != 1 || lp.exits.len() != 1
}

fn loop_has_indirect_index_access(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let Some(iv) = lp.iv.as_ref() else {
        return false;
    };
    let iv_phi = iv.phi_val;
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                    if expr_has_non_iv_index(fn_ir, *src, iv_phi, &mut FxHashSet::default()) {
                        return true;
                    }
                }
                Instr::StoreIndex1D { idx, val, .. } => {
                    if !is_iv_equivalent(fn_ir, *idx, iv_phi) {
                        return true;
                    }
                    if expr_has_non_iv_index(fn_ir, *val, iv_phi, &mut FxHashSet::default()) {
                        return true;
                    }
                }
                Instr::StoreIndex2D { .. } => return true,
            }
        }
    }
    false
}

fn expr_has_non_iv_index(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Index1D { idx, base, .. } => {
            if !is_iv_equivalent(fn_ir, *idx, iv_phi) {
                return true;
            }
            expr_has_non_iv_index(fn_ir, *base, iv_phi, seen)
                || expr_has_non_iv_index(fn_ir, *idx, iv_phi, seen)
        }
        ValueKind::Index2D { .. } => true,
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_has_non_iv_index(fn_ir, *lhs, iv_phi, seen)
                || expr_has_non_iv_index(fn_ir, *rhs, iv_phi, seen)
        }
        ValueKind::Unary { rhs, .. } => expr_has_non_iv_index(fn_ir, *rhs, iv_phi, seen),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|a| expr_has_non_iv_index(fn_ir, *a, iv_phi, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(a, _)| expr_has_non_iv_index(fn_ir, *a, iv_phi, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_has_non_iv_index(fn_ir, *base, iv_phi, seen)
        }
        ValueKind::Range { start, end } => {
            expr_has_non_iv_index(fn_ir, *start, iv_phi, seen)
                || expr_has_non_iv_index(fn_ir, *end, iv_phi, seen)
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => false,
    }
}

fn resolve_base_var(fn_ir: &FnIR, base: ValueId) -> Option<VarId> {
    if let ValueKind::Load { var } = &fn_ir.values[base].kind {
        return Some(var.clone());
    }
    fn_ir.values[base].origin_var.clone()
}

fn rewrite_returns_for_var(fn_ir: &mut FnIR, var: &str, new_val: ValueId) {
    for bid in 0..fn_ir.blocks.len() {
        if let Terminator::Return(Some(ret_vid)) = fn_ir.blocks[bid].term
            && fn_ir.values[ret_vid].origin_var.as_deref() == Some(var)
        {
            fn_ir.blocks[bid].term = Terminator::Return(Some(new_val));
        }
    }
}

fn extract_store_1d_in_block(
    fn_ir: &FnIR,
    bid: BlockId,
    iv_phi: ValueId,
) -> Option<(ValueId, ValueId)> {
    match classify_store_1d_in_block(fn_ir, bid) {
        BlockStore1DMatch::One(store)
            if !store.is_vector && is_iv_equivalent(fn_ir, store.idx, iv_phi) =>
        {
            Some((store.base, store.val))
        }
        _ => None,
    }
}

fn classify_store_1d_in_block(fn_ir: &FnIR, bid: BlockId) -> BlockStore1DMatch {
    let mut found: Option<BlockStore1D> = None;
    for instr in &fn_ir.blocks[bid].instrs {
        match instr {
            Instr::Assign { .. } | Instr::Eval { .. } => {}
            Instr::StoreIndex2D { .. } => return BlockStore1DMatch::Invalid,
            Instr::StoreIndex1D {
                base,
                idx,
                val,
                is_vector,
                ..
            } => {
                if found.is_some() {
                    return BlockStore1DMatch::Invalid;
                }
                found = Some(BlockStore1D {
                    base: *base,
                    idx: *idx,
                    val: *val,
                    is_vector: *is_vector,
                });
            }
        }
    }
    match found {
        Some(store) => BlockStore1DMatch::One(store),
        None => BlockStore1DMatch::None,
    }
}

fn is_prev_element(fn_ir: &FnIR, vid: ValueId, base: ValueId, iv_phi: ValueId) -> bool {
    match &fn_ir.values[vid].kind {
        ValueKind::Index1D { base: b, idx, .. } => {
            if canonical_value(fn_ir, *b) != canonical_value(fn_ir, base) {
                return false;
            }
            is_iv_minus_one(fn_ir, *idx, iv_phi)
        }
        _ => false,
    }
}

fn is_iv_minus_one(fn_ir: &FnIR, idx: ValueId, iv_phi: ValueId) -> bool {
    if is_iv_equivalent(fn_ir, idx, iv_phi) {
        return false;
    }
    match &fn_ir.values[idx].kind {
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } if is_iv_equivalent(fn_ir, *lhs, iv_phi) => {
            matches!(fn_ir.values[*rhs].kind, ValueKind::Const(Lit::Int(1)))
        }
        ValueKind::Binary {
            op: BinOp::Add,
            lhs,
            rhs,
        } if is_iv_equivalent(fn_ir, *lhs, iv_phi) => {
            matches!(fn_ir.values[*rhs].kind, ValueKind::Const(Lit::Int(-1)))
        }
        _ => false,
    }
}

fn expr_reads_base(fn_ir: &FnIR, root: ValueId, base: ValueId) -> bool {
    fn rec(fn_ir: &FnIR, root: ValueId, base: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Index1D { base: b, .. } | ValueKind::Index2D { base: b, .. } => {
                if canonical_value(fn_ir, *b) == canonical_value(fn_ir, base) {
                    return true;
                }
            }
            _ => {}
        }
        match &fn_ir.values[root].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, base, seen) || rec(fn_ir, *rhs, base, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, base, seen),
            ValueKind::Call { args, .. } => args.iter().any(|a| rec(fn_ir, *a, base, seen)),
            ValueKind::Phi { args } => args.iter().any(|(a, _)| rec(fn_ir, *a, base, seen)),
            ValueKind::Len { base: b } | ValueKind::Indices { base: b } => {
                rec(fn_ir, *b, base, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, base, seen) || rec(fn_ir, *end, base, seen)
            }
            ValueKind::Index1D { base: b, idx, .. } => {
                rec(fn_ir, *b, base, seen) || rec(fn_ir, *idx, base, seen)
            }
            ValueKind::Index2D { base: b, r, c } => {
                rec(fn_ir, *b, base, seen)
                    || rec(fn_ir, *r, base, seen)
                    || rec(fn_ir, *c, base, seen)
            }
            _ => false,
        }
    }
    rec(fn_ir, root, base, &mut FxHashSet::default())
}

fn expr_has_non_vector_safe_call(
    fn_ir: &FnIR,
    root: ValueId,
    user_call_whitelist: &FxHashSet<String>,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Call { callee, args, .. } => {
            if !is_vector_safe_call(callee, args.len(), user_call_whitelist)
                && !is_runtime_vector_read_call(callee, args.len())
            {
                if vectorize_trace_enabled() {
                    eprintln!(
                        "   [vec-expr-map] non-vector-safe call: {} / arity {}",
                        callee,
                        args.len()
                    );
                }
                return true;
            }
            args.iter()
                .any(|a| expr_has_non_vector_safe_call(fn_ir, *a, user_call_whitelist, seen))
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *lhs, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *rhs, user_call_whitelist, seen)
        }
        ValueKind::Unary { rhs, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *rhs, user_call_whitelist, seen)
        }
        ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|a| expr_has_non_vector_safe_call(fn_ir, *a, user_call_whitelist, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(a, _)| expr_has_non_vector_safe_call(fn_ir, *a, user_call_whitelist, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
        }
        ValueKind::Range { start, end } => {
            expr_has_non_vector_safe_call(fn_ir, *start, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *end, user_call_whitelist, seen)
        }
        ValueKind::Index1D { base, idx, .. } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *idx, user_call_whitelist, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            expr_has_non_vector_safe_call(fn_ir, *base, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *r, user_call_whitelist, seen)
                || expr_has_non_vector_safe_call(fn_ir, *c, user_call_whitelist, seen)
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => false,
    }
}

fn expr_has_non_vector_safe_call_in_vector_context(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    user_call_whitelist: &FxHashSet<String>,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    let root = canonical_value(fn_ir, root);
    if !seen.insert(root) {
        return false;
    }
    if is_scalar_broadcast_value(fn_ir, root) && !expr_has_iv_dependency(fn_ir, root, iv_phi) {
        return false;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Call { callee, args, .. } => {
            if !is_vector_safe_call(callee, args.len(), user_call_whitelist)
                && !is_runtime_vector_read_call(callee, args.len())
            {
                if vectorize_trace_enabled() {
                    eprintln!(
                        "   [vec-expr-map] non-vector-safe call: {} / arity {}",
                        callee,
                        args.len()
                    );
                }
                return true;
            }
            args.iter().any(|a| {
                expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *a,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
            })
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *lhs,
                iv_phi,
                user_call_whitelist,
                seen,
            ) || expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *rhs,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }
        ValueKind::Unary { rhs, .. } => expr_has_non_vector_safe_call_in_vector_context(
            fn_ir,
            *rhs,
            iv_phi,
            user_call_whitelist,
            seen,
        ),
        ValueKind::Intrinsic { args, .. } => args.iter().any(|a| {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *a,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }),
        ValueKind::Phi { args } => args.iter().any(|(a, _)| {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *a,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Range { start, end } => {
            expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *start,
                iv_phi,
                user_call_whitelist,
                seen,
            ) || expr_has_non_vector_safe_call_in_vector_context(
                fn_ir,
                *end,
                iv_phi,
                user_call_whitelist,
                seen,
            )
        }
        ValueKind::Index1D { base, idx, .. } => {
            let base_blocking = expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                );
            base_blocking
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *idx,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Index2D { base, r, c } => {
            let base_blocking = expr_has_iv_dependency(fn_ir, *base, iv_phi)
                && expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *base,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                );
            base_blocking
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *r,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
                || expr_has_non_vector_safe_call_in_vector_context(
                    fn_ir,
                    *c,
                    iv_phi,
                    user_call_whitelist,
                    seen,
                )
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => false,
    }
}

fn is_runtime_vector_read_call(callee: &str, arity: usize) -> bool {
    matches!(callee, "rr_index1_read" | "rr_index1_read_strict") && (arity == 2 || arity == 3)
}

fn floor_like_index_source(fn_ir: &FnIR, idx: ValueId) -> Option<ValueId> {
    let idx = canonical_value(fn_ir, idx);
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &fn_ir.values[idx].kind
    else {
        return None;
    };
    if !matches!(callee.as_str(), "floor" | "ceiling" | "trunc") {
        return None;
    }
    if args.len() != 1 {
        return None;
    }
    if !names.is_empty() && names.first().and_then(|n| n.as_ref()).is_some() {
        return None;
    }
    Some(args[0])
}

fn same_base_value(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    let a = canonical_value(fn_ir, a);
    let b = canonical_value(fn_ir, b);
    if a == b {
        return true;
    }
    match (resolve_base_var(fn_ir, a), resolve_base_var(fn_ir, b)) {
        (Some(va), Some(vb)) => va == vb,
        _ => false,
    }
}

fn expr_reads_base_non_iv(fn_ir: &FnIR, root: ValueId, base: ValueId, iv_phi: ValueId) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        base: ValueId,
        iv_phi: ValueId,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Index1D {
                base: read_base,
                idx,
                ..
            } => {
                if same_base_value(fn_ir, *read_base, base)
                    && !is_iv_equivalent(fn_ir, *idx, iv_phi)
                {
                    return true;
                }
                rec(fn_ir, *read_base, base, iv_phi, seen) || rec(fn_ir, *idx, base, iv_phi, seen)
            }
            ValueKind::Index2D {
                base: read_base,
                r,
                c,
            } => {
                if same_base_value(fn_ir, *read_base, base) {
                    return true;
                }
                rec(fn_ir, *read_base, base, iv_phi, seen)
                    || rec(fn_ir, *r, base, iv_phi, seen)
                    || rec(fn_ir, *c, base, iv_phi, seen)
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, base, iv_phi, seen) || rec(fn_ir, *rhs, base, iv_phi, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, base, iv_phi, seen),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                args.iter().any(|a| rec(fn_ir, *a, base, iv_phi, seen))
            }
            ValueKind::Phi { args } => args.iter().any(|(a, _)| rec(fn_ir, *a, base, iv_phi, seen)),
            ValueKind::Len { base: b } | ValueKind::Indices { base: b } => {
                rec(fn_ir, *b, base, iv_phi, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, base, iv_phi, seen) || rec(fn_ir, *end, base, iv_phi, seen)
            }
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => false,
        }
    }
    rec(fn_ir, root, base, iv_phi, &mut FxHashSet::default())
}

fn expr_has_iv_dependency(fn_ir: &FnIR, root: ValueId, iv_phi: ValueId) -> bool {
    fn load_var_depends_on_iv(
        fn_ir: &FnIR,
        var: &str,
        iv_phi: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        if !seen_vars.insert(var.to_string()) {
            return false;
        }
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                if rec(fn_ir, *src, iv_phi, seen_vals, seen_vars) {
                    seen_vars.remove(var);
                    return true;
                }
            }
        }
        seen_vars.remove(var);
        false
    }

    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            return true;
        }
        if !seen_vals.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *rhs, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, iv_phi, seen_vals, seen_vars),
            ValueKind::Call { args, .. } => args
                .iter()
                .any(|a| rec(fn_ir, *a, iv_phi, seen_vals, seen_vars)),
            ValueKind::Phi { args } => args
                .iter()
                .any(|(a, _)| rec(fn_ir, *a, iv_phi, seen_vals, seen_vars)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, *base, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *end, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index1D { idx, .. } => rec(fn_ir, *idx, iv_phi, seen_vals, seen_vars),
            ValueKind::Index2D { r, c, .. } => {
                rec(fn_ir, *r, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, *c, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Load { var } => {
                load_var_depends_on_iv(fn_ir, var, iv_phi, seen_vals, seen_vars)
            }
            _ => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

fn is_vectorizable_expr(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    allow_any_base: bool,
    require_safe_index: bool,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        allow_any_base: bool,
        require_safe_index: bool,
        seen: &mut FxHashSet<ValueId>,
    ) -> bool {
        if root == iv_phi {
            return true;
        }
        if !seen.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. } => true,
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(
                    fn_ir,
                    *lhs,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                ) && rec(
                    fn_ir,
                    *rhs,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                )
            }
            ValueKind::Unary { rhs, .. } => rec(
                fn_ir,
                *rhs,
                iv_phi,
                lp,
                allow_any_base,
                require_safe_index,
                seen,
            ),
            ValueKind::Call { args, .. } => args.iter().all(|a| {
                rec(
                    fn_ir,
                    *a,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                )
            }),
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                if require_safe_index && !(*is_safe && *is_na_safe) {
                    return false;
                }
                if !allow_any_base && !is_loop_compatible_base(lp, fn_ir, *base) {
                    return false;
                }
                if is_iv_equivalent(fn_ir, *idx, iv_phi) {
                    return true;
                }
                !require_safe_index
                    && expr_has_iv_dependency(fn_ir, *idx, iv_phi)
                    && rec(
                        fn_ir,
                        *idx,
                        iv_phi,
                        lp,
                        allow_any_base,
                        require_safe_index,
                        seen,
                    )
            }
            ValueKind::Phi { args } => args.iter().all(|(a, _)| {
                rec(
                    fn_ir,
                    *a,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                )
            }),
            ValueKind::Len { base } | ValueKind::Indices { base } => rec(
                fn_ir,
                *base,
                iv_phi,
                lp,
                allow_any_base,
                require_safe_index,
                seen,
            ),
            ValueKind::Range { start, end } => {
                rec(
                    fn_ir,
                    *start,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                ) && rec(
                    fn_ir,
                    *end,
                    iv_phi,
                    lp,
                    allow_any_base,
                    require_safe_index,
                    seen,
                )
            }
            _ => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        allow_any_base,
        require_safe_index,
        &mut FxHashSet::default(),
    )
}

fn is_condition_vectorizable(
    fn_ir: &FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> bool {
    fn load_var_condition_vectorizable(
        fn_ir: &FnIR,
        var: &str,
        iv_phi: ValueId,
        lp: &LoopInfo,
        user_call_whitelist: &FxHashSet<String>,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        if !seen_vars.insert(var.to_string()) {
            return false;
        }
        let mut found = false;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                found = true;
                if !rec(
                    fn_ir,
                    *src,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                ) {
                    seen_vars.remove(var);
                    return false;
                }
            }
        }
        seen_vars.remove(var);
        // Params and immutable captures can appear as bare loads with no local assignment.
        // Treat them as loop-invariant condition inputs.
        if !found {
            return true;
        }
        true
    }

    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        user_call_whitelist: &FxHashSet<String>,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if root == iv_phi || is_iv_equivalent(fn_ir, root, iv_phi) {
            return true;
        }
        if !seen_vals.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Param { .. } => true,
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(
                    fn_ir,
                    *lhs,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                ) && rec(
                    fn_ir,
                    *rhs,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                )
            }
            ValueKind::Unary { rhs, .. } => rec(
                fn_ir,
                *rhs,
                iv_phi,
                lp,
                user_call_whitelist,
                seen_vals,
                seen_vars,
            ),
            // Data-dependent conditions are now allowed if the access is proven safe.
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                let iv_idx = is_iv_equivalent(fn_ir, *idx, iv_phi);
                let iv_dependent_idx = iv_idx || expr_has_iv_dependency(fn_ir, *idx, iv_phi);
                if !is_loop_compatible_base(lp, fn_ir, *base) && !iv_dependent_idx {
                    return false;
                }
                // Fast-path: proven-safe scalar read on IV-aligned index.
                if *is_safe && *is_na_safe && iv_idx {
                    return true;
                }
                // Pre-BCE loops often carry unspecialized index safety flags.
                // Allow IV-dependent index expressions and rely on runtime vector
                // read guards during materialization (`rr_index1_read_vec` path).
                iv_dependent_idx
            }
            ValueKind::Index2D { .. } => false,
            ValueKind::Call { callee, args, .. } => {
                let runtime_read = is_runtime_vector_read_call(callee, args.len());
                if runtime_read
                    && let Some(base) = args.first().copied()
                    && !is_loop_compatible_base(lp, fn_ir, base)
                {
                    return false;
                }
                (is_vector_safe_call(callee, args.len(), user_call_whitelist) || runtime_read)
                    && args.iter().all(|a| {
                        rec(
                            fn_ir,
                            *a,
                            iv_phi,
                            lp,
                            user_call_whitelist,
                            seen_vals,
                            seen_vars,
                        )
                    })
            }
            ValueKind::Phi { args } => args.iter().all(|(a, _)| {
                rec(
                    fn_ir,
                    *a,
                    iv_phi,
                    lp,
                    user_call_whitelist,
                    seen_vals,
                    seen_vars,
                )
            }),
            ValueKind::Load { var } => load_var_condition_vectorizable(
                fn_ir,
                var,
                iv_phi,
                lp,
                user_call_whitelist,
                seen_vals,
                seen_vars,
            ),
            _ => false,
        }
    }
    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        user_call_whitelist,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

fn build_loop_index_vector(fn_ir: &mut FnIR, lp: &LoopInfo) -> Option<ValueId> {
    let iv = lp.iv.as_ref()?;
    let end = lp.limit?;
    Some(fn_ir.add_value(
        ValueKind::Range {
            start: iv.init_val,
            end,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    ))
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct MaterializedExprKey {
    kind: ValueKind,
}

type MaterializeRecurseFn = fn(
    &mut FnIR,
    ValueId,
    ValueId,
    ValueId,
    &LoopInfo,
    &mut FxHashMap<ValueId, ValueId>,
    &mut FxHashMap<MaterializedExprKey, ValueId>,
    &mut FxHashSet<ValueId>,
    bool,
    bool,
) -> Option<ValueId>;

fn intern_materialized_value(
    fn_ir: &mut FnIR,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    kind: ValueKind,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
) -> ValueId {
    let key = MaterializedExprKey { kind: kind.clone() };
    if let Some(existing) = interner.get(&key) {
        // Reuse structurally identical expressions, but keep analysis metadata
        // conservative across reuse sites.
        let merged = fn_ir.values[*existing].facts.join(&facts);
        fn_ir.values[*existing].facts = merged;
        return *existing;
    }
    let id = fn_ir.add_value(kind, span, facts, None);
    interner.insert(key, id);
    id
}

#[allow(clippy::too_many_arguments)]
fn materialize_vector_load(
    fn_ir: &mut FnIR,
    root: ValueId,
    var: &str,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    let use_bb = value_use_block_in_loop(fn_ir, lp, root);
    if let Some(src) = unique_assign_source_in_loop(fn_ir, lp, var) {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-materialize] {} load {} via unique-assign {:?}",
                fn_ir.name, var, src
            );
        }
        if canonical_value(fn_ir, src) == root {
            return Some(root);
        }
        return recurse(
            fn_ir,
            src,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        );
    }

    if let Some(src) = merged_assign_source_in_loop(fn_ir, lp, var) {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-materialize] {} load {} via merged-assign {:?}",
                fn_ir.name, var, src
            );
        }
        return recurse(
            fn_ir,
            src,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        );
    }

    if let Some(use_bb) = use_bb
        && let Some(src) = unique_origin_phi_value_in_loop(fn_ir, lp, var)
            .filter(|src| {
                let src = canonical_value(fn_ir, *src);
                !visiting.contains(&src)
                    && fn_ir.values[src]
                        .phi_block
                        .is_some_and(|phi_bb| phi_bb < use_bb)
            })
            .or_else(|| {
                nearest_origin_phi_value_in_loop(fn_ir, lp, var, use_bb)
                    .filter(|src| !visiting.contains(&canonical_value(fn_ir, *src)))
            })
    {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-materialize] {} load {} via prior-origin-phi {:?} before bb {}",
                fn_ir.name, var, src, use_bb
            );
        }
        return recurse(
            fn_ir,
            src,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        );
    }

    if let Some(src) = unique_origin_phi_value_in_loop(fn_ir, lp, var)
        .filter(|src| !visiting.contains(&canonical_value(fn_ir, *src)))
    {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-materialize] {} load {} via fallback-origin-phi {:?}",
                fn_ir.name, var, src
            );
        }
        return recurse(
            fn_ir,
            src,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        );
    }

    if let Some(use_bb) = use_bb {
        let nearest_phi = nearest_origin_phi_value_in_loop(fn_ir, lp, var, use_bb);
        if let Some(phi_src) = nearest_phi
            .filter(|src| visiting.contains(&canonical_value(fn_ir, *src)))
            .and_then(|src| {
                materialize_passthrough_origin_phi_state(
                    fn_ir,
                    src,
                    var,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    interner,
                    allow_any_base,
                    require_safe_index,
                )
            })
        {
            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-materialize] {} load {} via passthrough-origin-phi {:?}",
                    fn_ir.name, var, nearest_phi
                );
            }
            return Some(phi_src);
        }

        let src = last_effective_assign_before_value_use_in_block(fn_ir, use_bb, var, root);
        if let Some(src) = src {
            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-materialize] {} load {} via local-block-assign {:?} in bb {}",
                    fn_ir.name, var, src, use_bb
                );
            }
            return recurse(
                fn_ir,
                src,
                iv_phi,
                idx_vec,
                lp,
                memo,
                interner,
                visiting,
                allow_any_base,
                require_safe_index,
            );
        }

        if has_assignment_in_loop(fn_ir, lp, var) {
            let unique_assign = unique_assign_source_in_loop(fn_ir, lp, var);
            let merged_assign = merged_assign_source_in_loop(fn_ir, lp, var);
            let unique_phi = unique_origin_phi_value_in_loop(fn_ir, lp, var);
            let nearest_phi = nearest_origin_phi_value_in_loop(fn_ir, lp, var, use_bb);
            let nearest_phi_block = nearest_phi.and_then(|src| fn_ir.values[src].phi_block);
            let nearest_phi_visiting =
                nearest_phi.is_some_and(|src| visiting.contains(&canonical_value(fn_ir, src)));
            let nearest_phi_kind = nearest_phi
                .map(|src| format!("{:?}", fn_ir.values[src].kind))
                .unwrap_or_else(|| "None".to_string());
            let detail = format!(
                "loop-local load without unique materializable source (var={}, use_bb={}, unique_assign={:?}, merged_assign={:?}, unique_phi={:?}, nearest_phi={:?}, nearest_phi_block={:?}, nearest_phi_visiting={}, nearest_phi_kind={})",
                var,
                use_bb,
                unique_assign,
                merged_assign,
                unique_phi,
                nearest_phi,
                nearest_phi_block,
                nearest_phi_visiting,
                nearest_phi_kind
            );
            trace_materialize_reject(fn_ir, root, &detail);
            return None;
        }

        return Some(root);
    }

    if has_assignment_in_loop(fn_ir, lp, var) {
        let detail = format!(
            "loop-local load without unique materializable source (var={}, use_bb=none)",
            var
        );
        trace_materialize_reject(fn_ir, root, &detail);
        return None;
    }

    Some(root)
}

#[allow(clippy::too_many_arguments)]
fn materialize_vector_phi(
    fn_ir: &mut FnIR,
    root: ValueId,
    args: Vec<(ValueId, BlockId)>,
    span: crate::utils::Span,
    facts: crate::mir::flow::Facts,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    visiting: &mut FxHashSet<ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
    recurse: MaterializeRecurseFn,
) -> Option<ValueId> {
    if phi_loads_same_var(fn_ir, &args) {
        let folded = recurse(
            fn_ir,
            args[0].0,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        memo.insert(root, folded);
        visiting.remove(&root);
        return Some(folded);
    }

    let folded_non_self_args: Vec<ValueId> = args
        .iter()
        .map(|(a, _)| canonical_value(fn_ir, *a))
        .filter(|a| *a != root)
        .collect();
    if let Some(first) = folded_non_self_args.first().copied()
        && folded_non_self_args.iter().all(|a| *a == first)
    {
        let folded = recurse(
            fn_ir,
            first,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        memo.insert(root, folded);
        visiting.remove(&root);
        return Some(folded);
    }

    let outside_args: Vec<ValueId> = args
        .iter()
        .filter_map(|(a, b)| if lp.body.contains(b) { None } else { Some(*a) })
        .collect();
    if outside_args.len() == 1 {
        let seed = recurse(
            fn_ir,
            outside_args[0],
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        memo.insert(root, seed);
        visiting.remove(&root);
        return Some(seed);
    }

    if let Some(var) = fn_ir.values[root].origin_var.clone()
        && let Some(phi_vec) = materialize_passthrough_origin_phi_state(
            fn_ir,
            root,
            &var,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            allow_any_base,
            require_safe_index,
        )
    {
        memo.insert(root, phi_vec);
        visiting.remove(&root);
        return Some(phi_vec);
    }

    if fn_ir.values[root].phi_block != Some(lp.header)
        && args.iter().all(|(_, b)| lp.body.contains(b))
        && let Some((cond, then_val, else_val)) = find_conditional_phi_shape(fn_ir, root, &args)
    {
        if expr_has_non_vector_safe_call_in_vector_context(
            fn_ir,
            cond,
            iv_phi,
            &FxHashSet::default(),
            &mut FxHashSet::default(),
        ) {
            trace_materialize_reject(fn_ir, root, "conditional phi has non-vector-safe condition");
            return None;
        }
        let cond_vec = recurse(
            fn_ir, cond, iv_phi, idx_vec, lp, memo, interner, visiting, true, true,
        )?;
        let then_vec = recurse(
            fn_ir,
            then_val,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        let else_vec = recurse(
            fn_ir,
            else_val,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        let ifelse_val = intern_materialized_value(
            fn_ir,
            interner,
            ValueKind::Call {
                callee: "rr_ifelse_strict".to_string(),
                args: vec![cond_vec, then_vec, else_vec],
                names: vec![None, None, None],
            },
            span,
            facts,
        );
        memo.insert(root, ifelse_val);
        visiting.remove(&root);
        return Some(ifelse_val);
    }

    let mut picked: Option<ValueId> = None;
    for (arg, _) in args {
        let materialized = recurse(
            fn_ir,
            arg,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            visiting,
            allow_any_base,
            require_safe_index,
        )?;
        match picked {
            None => picked = Some(materialized),
            Some(prev) if canonical_value(fn_ir, prev) == canonical_value(fn_ir, materialized) => {}
            Some(_) => {
                trace_materialize_reject(
                    fn_ir,
                    root,
                    "phi arguments materialize to distinct values",
                );
                return None;
            }
        }
    }
    picked
}

fn is_int_index_vector_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    let v = &fn_ir.values[canonical_value(fn_ir, vid)];
    (v.value_ty.shape == ShapeTy::Vector && v.value_ty.prim == PrimTy::Int)
        || v.facts
            .has(crate::mir::flow::Facts::IS_VECTOR | crate::mir::flow::Facts::INT_SCALAR)
}

fn is_scalar_broadcast_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    let v = &fn_ir.values[canonical_value(fn_ir, vid)];
    v.value_ty.shape == ShapeTy::Scalar
}

fn has_assignment_in_loop(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> bool {
    lp.body.iter().any(|bid| {
        fn_ir.blocks[*bid].instrs.iter().any(|ins| match ins {
            Instr::Assign { dst, .. } => dst == var,
            _ => false,
        })
    })
}

fn unique_assign_source_in_loop(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> Option<ValueId> {
    let mut src: Option<ValueId> = None;
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src: s, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            let s = canonical_value(fn_ir, *s);
            match src {
                None => src = Some(s),
                Some(prev) if canonical_value(fn_ir, prev) == s => {}
                Some(_) => return None,
            }
        }
    }
    src
}

fn merged_assign_source_in_loop(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> Option<ValueId> {
    let mut assigned = Vec::new();
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst == var {
                assigned.push(canonical_value(fn_ir, *src));
            }
        }
    }
    assigned.sort_unstable();
    assigned.dedup();

    let mut phi_srcs = assigned
        .iter()
        .copied()
        .filter(
            |src| matches!(&fn_ir.values[*src].kind, ValueKind::Phi { args } if !args.is_empty()),
        )
        .filter(|src| {
            fn_ir.values[*src]
                .phi_block
                .is_some_and(|bb| lp.body.contains(&bb))
        });
    let phi_src = phi_srcs.next()?;
    if phi_srcs.next().is_some() {
        return None;
    }

    let ValueKind::Phi { args } = &fn_ir.values[phi_src].kind else {
        return None;
    };
    let phi_args: FxHashSet<ValueId> = args
        .iter()
        .map(|(arg, _)| canonical_value(fn_ir, *arg))
        .collect();
    if assigned
        .iter()
        .all(|src| *src == phi_src || phi_args.contains(src))
    {
        Some(phi_src)
    } else {
        None
    }
}

fn unique_origin_phi_value_in_loop(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> Option<ValueId> {
    let mut found: Option<ValueId> = None;
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() || value.origin_var.as_deref() != Some(var) {
            continue;
        }
        if !value.phi_block.is_some_and(|bb| lp.body.contains(&bb)) {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        match found {
            None => found = Some(vid),
            Some(prev) if canonical_value(fn_ir, prev) == vid => {}
            Some(_) => return None,
        }
    }
    found
}

fn phi_loads_same_var(fn_ir: &FnIR, args: &[(ValueId, BlockId)]) -> bool {
    let mut found: Option<&str> = None;
    for (arg, _) in args {
        let ValueKind::Load { var } = &fn_ir.values[canonical_value(fn_ir, *arg)].kind else {
            return false;
        };
        match found {
            None => found = Some(var.as_str()),
            Some(prev) if prev == var => {}
            Some(_) => return false,
        }
    }
    found.is_some()
}

fn value_use_block_in_loop(fn_ir: &FnIR, lp: &LoopInfo, vid: ValueId) -> Option<BlockId> {
    let vid = canonical_value(fn_ir, vid);
    let mut use_blocks: Vec<Option<BlockId>> = vec![None; fn_ir.values.len()];
    let mut worklist: Vec<(ValueId, BlockId)> = Vec::new();
    let mut body: Vec<BlockId> = lp.body.iter().copied().collect();
    body.sort_unstable();
    for bid in body {
        for ins in &fn_ir.blocks[bid].instrs {
            match ins {
                Instr::Assign { src, .. } => worklist.push((canonical_value(fn_ir, *src), bid)),
                Instr::Eval { val, .. } => worklist.push((canonical_value(fn_ir, *val), bid)),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    worklist.push((canonical_value(fn_ir, *base), bid));
                    worklist.push((canonical_value(fn_ir, *idx), bid));
                    worklist.push((canonical_value(fn_ir, *val), bid));
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    worklist.push((canonical_value(fn_ir, *base), bid));
                    worklist.push((canonical_value(fn_ir, *r), bid));
                    worklist.push((canonical_value(fn_ir, *c), bid));
                    worklist.push((canonical_value(fn_ir, *val), bid));
                }
            }
        }
        match &fn_ir.blocks[bid].term {
            Terminator::If { cond, .. } => worklist.push((canonical_value(fn_ir, *cond), bid)),
            Terminator::Return(Some(ret)) => worklist.push((canonical_value(fn_ir, *ret), bid)),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    while let Some((curr, bid)) = worklist.pop() {
        if let Some(prev) = use_blocks[curr]
            && bid >= prev
        {
            continue;
        }
        use_blocks[curr] = Some(bid);
        match &fn_ir.values[curr].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                worklist.push((canonical_value(fn_ir, *lhs), bid));
                worklist.push((canonical_value(fn_ir, *rhs), bid));
            }
            ValueKind::Unary { rhs, .. } => {
                worklist.push((canonical_value(fn_ir, *rhs), bid));
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    worklist.push((canonical_value(fn_ir, *arg), bid));
                }
            }
            ValueKind::Phi { args } => {
                for (arg, _) in args {
                    worklist.push((canonical_value(fn_ir, *arg), bid));
                }
            }
            ValueKind::Index1D { base, idx, .. } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
                worklist.push((canonical_value(fn_ir, *idx), bid));
            }
            ValueKind::Index2D { base, r, c } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
                worklist.push((canonical_value(fn_ir, *r), bid));
                worklist.push((canonical_value(fn_ir, *c), bid));
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                worklist.push((canonical_value(fn_ir, *base), bid));
            }
            ValueKind::Range { start, end } => {
                worklist.push((canonical_value(fn_ir, *start), bid));
                worklist.push((canonical_value(fn_ir, *end), bid));
            }
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } => {}
        }
    }
    use_blocks[vid]
}

fn nearest_origin_phi_value_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    use_bb: BlockId,
) -> Option<ValueId> {
    let mut best: Option<(BlockId, ValueId)> = None;
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() || value.origin_var.as_deref() != Some(var) {
            continue;
        }
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) || phi_bb >= use_bb {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        match best {
            None => best = Some((phi_bb, vid)),
            Some((best_bb, _)) if phi_bb > best_bb => best = Some((phi_bb, vid)),
            Some((best_bb, best_vid))
                if phi_bb == best_bb && canonical_value(fn_ir, best_vid) != vid =>
            {
                return None;
            }
            Some(_) => {}
        }
    }
    best.map(|(_, vid)| vid)
}

fn unique_assign_source_reaching_block_in_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
    target_bb: BlockId,
) -> Option<ValueId> {
    let preds = build_pred_map(fn_ir);
    let mut seen = FxHashSet::default();
    let mut stack: Vec<BlockId> = preds
        .get(&target_bb)
        .into_iter()
        .flat_map(|ps| ps.iter().copied())
        .filter(|bb| lp.body.contains(bb))
        .collect();
    let mut src: Option<ValueId> = None;
    while let Some(bid) = stack.pop() {
        if !seen.insert(bid) {
            continue;
        }
        for ins in &fn_ir.blocks[bid].instrs {
            let Instr::Assign { dst, src: s, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            let s = canonical_value(fn_ir, *s);
            match src {
                None => src = Some(s),
                Some(prev) if canonical_value(fn_ir, prev) == s => {}
                Some(_) => return None,
            }
        }
        if let Some(ps) = preds.get(&bid) {
            for pred in ps {
                if lp.body.contains(pred) {
                    stack.push(*pred);
                }
            }
        }
    }
    src
}

fn unwrap_vector_condition_value(fn_ir: &FnIR, root: ValueId) -> ValueId {
    let root = canonical_value(fn_ir, root);
    match &fn_ir.values[root].kind {
        ValueKind::Call { callee, args, .. }
            if matches!(callee.as_str(), "rr_truthy1" | "rr_bool") && !args.is_empty() =>
        {
            canonical_value(fn_ir, args[0])
        }
        _ => root,
    }
}

fn is_comparison_op(op: BinOp) -> bool {
    matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    )
}

#[allow(clippy::too_many_arguments)]
fn materialize_passthrough_origin_phi_state(
    fn_ir: &mut FnIR,
    phi: ValueId,
    var: &str,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    let phi = canonical_value(fn_ir, phi);
    let ValueKind::Phi { args } = fn_ir.values[phi].kind.clone() else {
        trace_materialize_reject(fn_ir, phi, "passthrough-origin-phi: root is not phi");
        return None;
    };
    let phi_bb = fn_ir.values[phi].phi_block?;
    let Some((_, cond, then_val, then_bb, else_val, else_bb)) =
        find_conditional_phi_shape_with_blocks(fn_ir, phi, &args)
    else {
        trace_materialize_reject(
            fn_ir,
            phi,
            "passthrough-origin-phi: no conditional phi shape",
        );
        return None;
    };
    if vectorize_trace_enabled() {
        eprintln!(
            "   [vec-materialize] {} phi-step phi={} bb={} cond={:?} then={:?}@{} else={:?}@{}",
            fn_ir.name,
            phi,
            phi_bb,
            fn_ir.values[canonical_value(fn_ir, cond)].kind,
            fn_ir.values[canonical_value(fn_ir, then_val)].kind,
            then_bb,
            fn_ir.values[canonical_value(fn_ir, else_val)].kind,
            else_bb
        );
        let mut seen = FxHashSet::default();
        trace_value_tree(fn_ir, cond, 6, &mut seen);
        let mut seen = FxHashSet::default();
        trace_value_tree(fn_ir, then_val, 6, &mut seen);
        let mut seen = FxHashSet::default();
        trace_value_tree(fn_ir, else_val, 6, &mut seen);
        trace_block_instrs(fn_ir, then_bb, 6);
        trace_block_instrs(fn_ir, else_bb, 6);
        eprintln!(
            "      block-last-assign then={:?} else={:?}",
            last_assign_to_var_in_block(fn_ir, then_bb, var),
            last_assign_to_var_in_block(fn_ir, else_bb, var)
        );
    }

    let then_assign = if is_passthrough_load_of_var(fn_ir, then_val, var) {
        last_assign_to_var_in_block(fn_ir, then_bb, var)
    } else {
        None
    };
    let else_assign = if is_passthrough_load_of_var(fn_ir, else_val, var) {
        last_assign_to_var_in_block(fn_ir, else_bb, var)
    } else {
        None
    };
    let then_prior_state = is_prior_origin_phi_state(fn_ir, then_val, var, phi_bb);
    let else_prior_state = is_prior_origin_phi_state(fn_ir, else_val, var, phi_bb);
    let then_passthrough = then_prior_state
        || (is_passthrough_load_of_var(fn_ir, then_val, var) && then_assign.is_none());
    let else_passthrough = else_prior_state
        || (is_passthrough_load_of_var(fn_ir, else_val, var) && else_assign.is_none());
    let (pass_then, prev_state_raw, update_val) = if then_passthrough && !else_passthrough {
        (
            true,
            then_prior_state.then_some(canonical_value(fn_ir, then_val)),
            else_assign.unwrap_or_else(|| canonical_value(fn_ir, else_val)),
        )
    } else if else_passthrough && !then_passthrough {
        (
            false,
            else_prior_state.then_some(canonical_value(fn_ir, else_val)),
            then_assign.unwrap_or_else(|| canonical_value(fn_ir, then_val)),
        )
    } else {
        trace_materialize_reject(
            fn_ir,
            phi,
            "passthrough-origin-phi: could not classify pass/update arms",
        );
        return None;
    };

    let prev_state = if let Some(prev_raw) = prev_state_raw {
        let prev_raw = collapse_prior_origin_phi_state(
            fn_ir,
            prev_raw,
            var,
            phi_bb,
            &mut FxHashSet::default(),
        )
        .unwrap_or(prev_raw);
        materialize_vector_expr(
            fn_ir,
            prev_raw,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            allow_any_base,
            require_safe_index,
        )?
    } else if let Some(prev_phi) = nearest_origin_phi_value_in_loop(fn_ir, lp, var, phi_bb)
        .filter(|src| canonical_value(fn_ir, *src) != phi)
    {
        let prev_phi = collapse_prior_origin_phi_state(
            fn_ir,
            prev_phi,
            var,
            phi_bb,
            &mut FxHashSet::default(),
        )
        .unwrap_or(prev_phi);
        materialize_vector_expr(
            fn_ir,
            prev_phi,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            allow_any_base,
            require_safe_index,
        )?
    } else {
        let Some(seed) = unique_assign_source_reaching_block_in_loop(fn_ir, lp, var, phi_bb) else {
            trace_materialize_reject(
                fn_ir,
                phi,
                "passthrough-origin-phi: no reaching seed assign",
            );
            return None;
        };
        materialize_vector_expr(
            fn_ir,
            seed,
            iv_phi,
            idx_vec,
            lp,
            memo,
            interner,
            allow_any_base,
            require_safe_index,
        )?
    };

    let cond_root = unwrap_vector_condition_value(fn_ir, cond);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[cond_root].kind.clone() else {
        trace_materialize_reject(
            fn_ir,
            phi,
            "passthrough-origin-phi: condition is not a binary compare",
        );
        return None;
    };
    if !is_comparison_op(op) {
        trace_materialize_reject(
            fn_ir,
            phi,
            "passthrough-origin-phi: binary op is not a comparison",
        );
        return None;
    }
    let prev_cmp_raw = prev_state_raw.map(|src| canonical_value(fn_ir, src));
    let materialize_cmp_side =
        |operand: ValueId,
         fn_ir: &mut FnIR,
         memo: &mut FxHashMap<ValueId, ValueId>,
         interner: &mut FxHashMap<MaterializedExprKey, ValueId>| {
            let operand = canonical_value(fn_ir, operand);
            if is_passthrough_load_of_var(fn_ir, operand, var)
                || prev_cmp_raw.is_some_and(|raw| raw == operand)
            {
                Some(prev_state)
            } else {
                materialize_vector_expr(
                    fn_ir,
                    operand,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    interner,
                    allow_any_base,
                    require_safe_index,
                )
            }
        };
    let cmp_lhs = materialize_cmp_side(lhs, fn_ir, memo, interner)?;
    let cmp_rhs = materialize_cmp_side(rhs, fn_ir, memo, interner)?;
    if cmp_lhs == prev_state && cmp_rhs == prev_state {
        trace_materialize_reject(
            fn_ir,
            phi,
            "passthrough-origin-phi: comparison collapsed to same prev state on both sides",
        );
        return None;
    }
    let cond_vec = intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Binary {
            op,
            lhs: cmp_lhs,
            rhs: cmp_rhs,
        },
        fn_ir.values[cond_root].span,
        fn_ir.values[cond_root].facts,
    );
    let update_vec = materialize_vector_expr(
        fn_ir,
        update_val,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        allow_any_base,
        require_safe_index,
    )?;
    let then_vec = if pass_then { prev_state } else { update_vec };
    let else_vec = if pass_then { update_vec } else { prev_state };
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: "rr_ifelse_strict".to_string(),
            args: vec![cond_vec, then_vec, else_vec],
            names: vec![None, None, None],
        },
        fn_ir.values[phi].span,
        fn_ir.values[phi].facts,
    ))
}

#[allow(clippy::too_many_arguments)]
fn materialize_vector_expr(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    #[allow(clippy::only_used_in_recursion)]
    fn rec(
        fn_ir: &mut FnIR,
        root: ValueId,
        iv_phi: ValueId,
        idx_vec: ValueId,
        lp: &LoopInfo,
        memo: &mut FxHashMap<ValueId, ValueId>,
        interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
        visiting: &mut FxHashSet<ValueId>,
        allow_any_base: bool,
        require_safe_index: bool,
    ) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        let _ = require_safe_index;
        if let Some(v) = memo.get(&root) {
            return Some(*v);
        }
        // Guard against pathological phi/load cycles that stay syntactically
        // productive enough to evade the simple `visiting` back-edge check.
        // In those cases we want a clean vectorization reject, not a stack overflow.
        if visiting.len() > 256 {
            trace_materialize_reject(fn_ir, root, "materialize_vector_expr recursion depth limit");
            return None;
        }
        if !visiting.insert(root) {
            trace_materialize_reject(fn_ir, root, "cycle in materialize_vector_expr");
            return None;
        }
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            memo.insert(root, idx_vec);
            visiting.remove(&root);
            return Some(idx_vec);
        }
        if is_scalar_broadcast_value(fn_ir, root) && !expr_has_iv_dependency(fn_ir, root, iv_phi) {
            memo.insert(root, root);
            visiting.remove(&root);
            return Some(root);
        }

        let span = fn_ir.values[root].span;
        let facts = fn_ir.values[root].facts;
        let out = match fn_ir.values[root].kind.clone() {
            ValueKind::Const(_) | ValueKind::Param { .. } => root,
            ValueKind::Load { var } => materialize_vector_load(
                fn_ir,
                root,
                &var,
                iv_phi,
                idx_vec,
                lp,
                memo,
                interner,
                visiting,
                allow_any_base,
                require_safe_index,
                rec,
            )?,
            ValueKind::Binary { op, lhs, rhs } => {
                let l = rec(
                    fn_ir,
                    lhs,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    interner,
                    visiting,
                    allow_any_base,
                    require_safe_index,
                )?;
                let r = rec(
                    fn_ir,
                    rhs,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    interner,
                    visiting,
                    allow_any_base,
                    require_safe_index,
                )?;
                if l == lhs && r == rhs {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Binary { op, lhs: l, rhs: r },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Unary { op, rhs } => {
                let r = rec(
                    fn_ir,
                    rhs,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    interner,
                    visiting,
                    allow_any_base,
                    require_safe_index,
                )?;
                if r == rhs {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Unary { op, rhs: r },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                let mut new_args = Vec::with_capacity(args.len());
                let mut changed = false;
                for a in &args {
                    let na = rec(
                        fn_ir,
                        *a,
                        iv_phi,
                        idx_vec,
                        lp,
                        memo,
                        interner,
                        visiting,
                        allow_any_base,
                        require_safe_index,
                    )?;
                    changed |= na != *a;
                    new_args.push(na);
                }
                let rewrite_runtime_read = is_runtime_vector_read_call(&callee, new_args.len())
                    && args
                        .get(1)
                        .copied()
                        .is_some_and(|idx_arg| expr_has_iv_dependency(fn_ir, idx_arg, iv_phi));
                if rewrite_runtime_read
                    && let Some(raw_idx) = args.get(1).copied().and_then(|idx_arg| {
                        floor_like_index_source(fn_ir, idx_arg)
                            .filter(|inner| expr_has_iv_dependency(fn_ir, *inner, iv_phi))
                    })
                {
                    let raw_idx_vec = rec(
                        fn_ir,
                        raw_idx,
                        iv_phi,
                        idx_vec,
                        lp,
                        memo,
                        interner,
                        visiting,
                        allow_any_base,
                        require_safe_index,
                    )?;
                    if raw_idx_vec != new_args[1] {
                        new_args[1] = raw_idx_vec;
                        changed = true;
                    }
                    if !is_int_index_vector_value(fn_ir, new_args[1]) {
                        let floor_idx_vec = intern_materialized_value(
                            fn_ir,
                            interner,
                            ValueKind::Call {
                                callee: "rr_index_vec_floor".to_string(),
                                args: vec![new_args[1]],
                                names: vec![None],
                            },
                            span,
                            facts,
                        );
                        if floor_idx_vec != new_args[1] {
                            new_args[1] = floor_idx_vec;
                            changed = true;
                        }
                    }
                }
                if !changed && !rewrite_runtime_read {
                    root
                } else {
                    let (out_callee, out_names) = if rewrite_runtime_read {
                        ("rr_index1_read_vec".to_string(), vec![None; new_args.len()])
                    } else {
                        (callee, names)
                    };
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Call {
                            callee: out_callee,
                            args: new_args,
                            names: out_names,
                        },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Intrinsic { op, args } => {
                let mut new_args = Vec::with_capacity(args.len());
                let mut changed = false;
                for a in args {
                    let na = rec(
                        fn_ir,
                        a,
                        iv_phi,
                        idx_vec,
                        lp,
                        memo,
                        interner,
                        visiting,
                        allow_any_base,
                        require_safe_index,
                    )?;
                    changed |= na != a;
                    new_args.push(na);
                }
                if !changed {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Intrinsic { op, args: new_args },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                if !allow_any_base && !is_loop_compatible_base(lp, fn_ir, base) {
                    trace_materialize_reject(fn_ir, root, "Index1D base is not loop-compatible");
                    return None;
                }
                if is_iv_equivalent(fn_ir, idx, iv_phi) {
                    let base_ref = resolve_materialized_value(fn_ir, base);
                    if is_safe && is_na_safe {
                        base_ref
                    } else {
                        let mut direct_idx = idx_vec;
                        if !is_int_index_vector_value(fn_ir, direct_idx) {
                            direct_idx = intern_materialized_value(
                                fn_ir,
                                interner,
                                ValueKind::Call {
                                    callee: "rr_index_vec_floor".to_string(),
                                    args: vec![direct_idx],
                                    names: vec![None],
                                },
                                span,
                                facts,
                            );
                        }
                        intern_materialized_value(
                            fn_ir,
                            interner,
                            ValueKind::Call {
                                callee: "rr_index1_read_vec".to_string(),
                                args: vec![base_ref, direct_idx],
                                names: vec![None, None],
                            },
                            span,
                            facts,
                        )
                    }
                } else if expr_has_iv_dependency(fn_ir, idx, iv_phi) {
                    let floor_src = if is_safe && is_na_safe {
                        None
                    } else {
                        floor_like_index_source(fn_ir, idx)
                            .filter(|inner| expr_has_iv_dependency(fn_ir, *inner, iv_phi))
                    };
                    let idx_src = floor_src.unwrap_or(idx);
                    let mut idx_vec = rec(
                        fn_ir,
                        idx_src,
                        iv_phi,
                        idx_vec,
                        lp,
                        memo,
                        interner,
                        visiting,
                        allow_any_base,
                        require_safe_index,
                    )?;
                    if floor_src.is_some() && !is_int_index_vector_value(fn_ir, idx_vec) {
                        idx_vec = intern_materialized_value(
                            fn_ir,
                            interner,
                            ValueKind::Call {
                                callee: "rr_index_vec_floor".to_string(),
                                args: vec![idx_vec],
                                names: vec![None],
                            },
                            span,
                            facts,
                        );
                    }
                    let base_ref = resolve_materialized_value(fn_ir, base);
                    if is_safe && is_na_safe {
                        intern_materialized_value(
                            fn_ir,
                            interner,
                            ValueKind::Index1D {
                                base: base_ref,
                                idx: idx_vec,
                                is_safe: true,
                                is_na_safe: true,
                            },
                            span,
                            facts,
                        )
                    } else {
                        intern_materialized_value(
                            fn_ir,
                            interner,
                            ValueKind::Call {
                                callee: "rr_index1_read_vec".to_string(),
                                args: vec![base_ref, idx_vec],
                                names: vec![None, None],
                            },
                            span,
                            facts,
                        )
                    }
                } else {
                    trace_materialize_reject(fn_ir, root, "Index1D index is not vectorizable");
                    return None;
                }
            }
            ValueKind::Len { base } => {
                let b = rec(
                    fn_ir,
                    base,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    interner,
                    visiting,
                    allow_any_base,
                    require_safe_index,
                )?;
                if b == base {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Len { base: b },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Range { start, end } => {
                let s = rec(
                    fn_ir,
                    start,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    interner,
                    visiting,
                    allow_any_base,
                    require_safe_index,
                )?;
                let e = rec(
                    fn_ir,
                    end,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    interner,
                    visiting,
                    allow_any_base,
                    require_safe_index,
                )?;
                if s == start && e == end {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Range { start: s, end: e },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Indices { base } => {
                let b = rec(
                    fn_ir,
                    base,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    interner,
                    visiting,
                    allow_any_base,
                    require_safe_index,
                )?;
                if b == base {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Indices { base: b },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Phi { args } => materialize_vector_phi(
                fn_ir,
                root,
                args,
                span,
                facts,
                iv_phi,
                idx_vec,
                lp,
                memo,
                interner,
                visiting,
                allow_any_base,
                require_safe_index,
                rec,
            )?,
            ValueKind::Index2D { .. } => {
                trace_materialize_reject(fn_ir, root, "Index2D is not vector-materializable");
                return None;
            }
        };

        memo.insert(root, out);
        visiting.remove(&root);
        Some(out)
    }

    rec(
        fn_ir,
        root,
        iv_phi,
        idx_vec,
        lp,
        memo,
        interner,
        &mut FxHashSet::default(),
        allow_any_base,
        require_safe_index,
    )
}

fn materialize_passthrough_origin_phi_state_scalar(
    fn_ir: &mut FnIR,
    phi: ValueId,
    var: &str,
    iv_phi: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    let trace_enabled = vectorize_trace_enabled();
    let depends_on_phi =
        |vid: ValueId, fn_ir: &FnIR| value_depends_on(fn_ir, vid, phi, &mut FxHashSet::default());
    let phi = canonical_value(fn_ir, phi);
    let ValueKind::Phi { args } = fn_ir.values[phi].kind.clone() else {
        return None;
    };
    let phi_bb = fn_ir.values[phi].phi_block?;
    let Some((_, cond, then_val, then_bb, else_val, else_bb)) =
        find_conditional_phi_shape_with_blocks(fn_ir, phi, &args)
    else {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: no conditional phi shape ({:?})",
                fn_ir.name, phi, var, fn_ir.values[phi].kind
            );
        }
        return None;
    };
    if trace_enabled {
        eprintln!(
            "   [vec-scalar-phi] {} phi={} var={} bb={} cond={:?} then={:?}@{} else={:?}@{}",
            fn_ir.name,
            phi,
            var,
            phi_bb,
            fn_ir.values[canonical_value(fn_ir, cond)].kind,
            fn_ir.values[canonical_value(fn_ir, then_val)].kind,
            then_bb,
            fn_ir.values[canonical_value(fn_ir, else_val)].kind,
            else_bb
        );
        eprintln!(
            "      last-assign then={:?} else={:?}",
            last_assign_to_var_in_block(fn_ir, then_bb, var),
            last_assign_to_var_in_block(fn_ir, else_bb, var)
        );
    }

    let then_assign = if is_passthrough_load_of_var(fn_ir, then_val, var) {
        last_assign_to_var_in_block(fn_ir, then_bb, var)
    } else {
        None
    };
    let else_assign = if is_passthrough_load_of_var(fn_ir, else_val, var) {
        last_assign_to_var_in_block(fn_ir, else_bb, var)
    } else {
        None
    };
    let then_prior_state = is_prior_origin_phi_state(fn_ir, then_val, var, phi_bb);
    let else_prior_state = is_prior_origin_phi_state(fn_ir, else_val, var, phi_bb);
    let then_passthrough = then_prior_state
        || (is_passthrough_load_of_var(fn_ir, then_val, var) && then_assign.is_none());
    let else_passthrough = else_prior_state
        || (is_passthrough_load_of_var(fn_ir, else_val, var) && else_assign.is_none());
    let (pass_then, prev_state_raw, update_val) = if then_passthrough && !else_passthrough {
        (
            true,
            then_prior_state.then_some(canonical_value(fn_ir, then_val)),
            else_assign.unwrap_or_else(|| canonical_value(fn_ir, else_val)),
        )
    } else if else_passthrough && !then_passthrough {
        (
            false,
            else_prior_state.then_some(canonical_value(fn_ir, else_val)),
            then_assign.unwrap_or_else(|| canonical_value(fn_ir, then_val)),
        )
    } else {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: could not classify pass/update (then_passthrough={} else_passthrough={} then_prior={} else_prior={})",
                fn_ir.name,
                phi,
                var,
                then_passthrough,
                else_passthrough,
                then_prior_state,
                else_prior_state
            );
        }
        return None;
    };
    if trace_enabled {
        eprintln!(
            "      classified pass_then={} prev_raw={:?} update={:?}",
            pass_then,
            prev_state_raw,
            fn_ir.values[canonical_value(fn_ir, update_val)].kind
        );
    }

    let prev_state = if let Some(prev_raw) = prev_state_raw {
        let prev_raw = collapse_prior_origin_phi_state(
            fn_ir,
            prev_raw,
            var,
            phi_bb,
            &mut FxHashSet::default(),
        )
        .unwrap_or(prev_raw);
        if depends_on_phi(prev_raw, fn_ir) {
            if trace_enabled {
                eprintln!(
                    "   [vec-scalar-phi] {} phi={} var={} reject: prev_raw still depends on phi ({:?})",
                    fn_ir.name,
                    phi,
                    var,
                    fn_ir.values[canonical_value(fn_ir, prev_raw)].kind
                );
            }
            return None;
        }
        materialize_loop_invariant_scalar_expr(fn_ir, prev_raw, iv_phi, lp, memo, interner)?
    } else if let Some(prev_phi) = nearest_origin_phi_value_in_loop(fn_ir, lp, var, phi_bb)
        .filter(|src| canonical_value(fn_ir, *src) != phi)
    {
        let prev_phi = collapse_prior_origin_phi_state(
            fn_ir,
            prev_phi,
            var,
            phi_bb,
            &mut FxHashSet::default(),
        )
        .unwrap_or(prev_phi);
        if depends_on_phi(prev_phi, fn_ir) {
            if trace_enabled {
                eprintln!(
                    "   [vec-scalar-phi] {} phi={} var={} reject: prev_phi still depends on phi ({:?})",
                    fn_ir.name,
                    phi,
                    var,
                    fn_ir.values[canonical_value(fn_ir, prev_phi)].kind
                );
            }
            return None;
        }
        materialize_loop_invariant_scalar_expr(fn_ir, prev_phi, iv_phi, lp, memo, interner)?
    } else {
        let seed = unique_assign_source_reaching_block_in_loop(fn_ir, lp, var, phi_bb)?;
        if depends_on_phi(seed, fn_ir) {
            if trace_enabled {
                eprintln!(
                    "   [vec-scalar-phi] {} phi={} var={} reject: seed depends on phi ({:?})",
                    fn_ir.name,
                    phi,
                    var,
                    fn_ir.values[canonical_value(fn_ir, seed)].kind
                );
            }
            return None;
        }
        materialize_loop_invariant_scalar_expr(fn_ir, seed, iv_phi, lp, memo, interner)?
    };

    let cond_root = unwrap_vector_condition_value(fn_ir, cond);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[cond_root].kind.clone() else {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: condition not binary ({:?})",
                fn_ir.name, phi, var, fn_ir.values[cond_root].kind
            );
        }
        return None;
    };
    if !is_comparison_op(op) {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: comparison op not supported ({:?})",
                fn_ir.name, phi, var, op
            );
        }
        return None;
    }
    let prev_cmp_raw = prev_state_raw.map(|src| canonical_value(fn_ir, src));
    let materialize_cmp_side =
        |operand: ValueId,
         fn_ir: &mut FnIR,
         memo: &mut FxHashMap<ValueId, ValueId>,
         interner: &mut FxHashMap<MaterializedExprKey, ValueId>| {
            let operand = canonical_value(fn_ir, operand);
            if is_passthrough_load_of_var(fn_ir, operand, var)
                || prev_cmp_raw.is_some_and(|raw| raw == operand)
            {
                Some(prev_state)
            } else {
                if depends_on_phi(operand, fn_ir) {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-scalar-phi] {} phi={} var={} reject: cmp operand depends on phi ({:?})",
                            fn_ir.name, phi, var, fn_ir.values[operand].kind
                        );
                    }
                    return None;
                }
                materialize_loop_invariant_scalar_expr(fn_ir, operand, iv_phi, lp, memo, interner)
            }
        };
    let cmp_lhs = materialize_cmp_side(lhs, fn_ir, memo, interner)?;
    let cmp_rhs = materialize_cmp_side(rhs, fn_ir, memo, interner)?;
    if cmp_lhs == prev_state && cmp_rhs == prev_state {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: comparison collapsed to prev_state on both sides",
                fn_ir.name, phi, var
            );
        }
        return None;
    }
    let cond_scalar = intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Binary {
            op,
            lhs: cmp_lhs,
            rhs: cmp_rhs,
        },
        fn_ir.values[cond_root].span,
        fn_ir.values[cond_root].facts,
    );
    if depends_on_phi(update_val, fn_ir) {
        if trace_enabled {
            eprintln!(
                "   [vec-scalar-phi] {} phi={} var={} reject: update depends on phi ({:?})",
                fn_ir.name,
                phi,
                var,
                fn_ir.values[canonical_value(fn_ir, update_val)].kind
            );
        }
        return None;
    }
    let update_scalar =
        materialize_loop_invariant_scalar_expr(fn_ir, update_val, iv_phi, lp, memo, interner)?;
    let then_scalar = if pass_then { prev_state } else { update_scalar };
    let else_scalar = if pass_then { update_scalar } else { prev_state };
    Some(intern_materialized_value(
        fn_ir,
        interner,
        ValueKind::Call {
            callee: "rr_ifelse_strict".to_string(),
            args: vec![cond_scalar, then_scalar, else_scalar],
            names: vec![None, None, None],
        },
        fn_ir.values[phi].span,
        fn_ir.values[phi].facts,
    ))
}

fn materialize_loop_invariant_scalar_expr(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
) -> Option<ValueId> {
    fn rec(
        fn_ir: &mut FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        memo: &mut FxHashMap<ValueId, ValueId>,
        interner: &mut FxHashMap<MaterializedExprKey, ValueId>,
        visiting: &mut FxHashSet<ValueId>,
    ) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        if let Some(v) = memo.get(&root) {
            return Some(*v);
        }
        if expr_has_iv_dependency(fn_ir, root, iv_phi) {
            return None;
        }
        if !visiting.insert(root) {
            return None;
        }

        let span = fn_ir.values[root].span;
        let facts = fn_ir.values[root].facts;
        let out = match fn_ir.values[root].kind.clone() {
            ValueKind::Const(_) | ValueKind::Param { .. } => root,
            ValueKind::Load { var } => {
                if let Some(src) = unique_assign_source_in_loop(fn_ir, lp, &var) {
                    rec(fn_ir, src, iv_phi, lp, memo, interner, visiting)?
                } else if let Some(src) = merged_assign_source_in_loop(fn_ir, lp, &var) {
                    rec(fn_ir, src, iv_phi, lp, memo, interner, visiting)?
                } else if let Some(src) =
                    unique_origin_phi_value_in_loop(fn_ir, lp, &var).or_else(|| {
                        nearest_origin_phi_value_in_loop(fn_ir, lp, &var, fn_ir.blocks.len())
                    })
                {
                    rec(fn_ir, src, iv_phi, lp, memo, interner, visiting)?
                } else if has_assignment_in_loop(fn_ir, lp, &var) {
                    return None;
                } else {
                    root
                }
            }
            ValueKind::Unary { op, rhs } => {
                let rhs_v = rec(fn_ir, rhs, iv_phi, lp, memo, interner, visiting)?;
                if rhs_v == rhs {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Unary { op, rhs: rhs_v },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Binary { op, lhs, rhs } => {
                let lhs_v = rec(fn_ir, lhs, iv_phi, lp, memo, interner, visiting)?;
                let rhs_v = rec(fn_ir, rhs, iv_phi, lp, memo, interner, visiting)?;
                if lhs_v == lhs && rhs_v == rhs {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Binary {
                            op,
                            lhs: lhs_v,
                            rhs: rhs_v,
                        },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Call {
                callee,
                args,
                names,
            } => {
                let mut new_args = Vec::with_capacity(args.len());
                let mut changed = false;
                for arg in &args {
                    let mapped = rec(fn_ir, *arg, iv_phi, lp, memo, interner, visiting)?;
                    changed |= mapped != *arg;
                    new_args.push(mapped);
                }
                if !changed {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Call {
                            callee,
                            args: new_args,
                            names,
                        },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Intrinsic { op, args } => {
                let mut new_args = Vec::with_capacity(args.len());
                let mut changed = false;
                for arg in args {
                    let mapped = rec(fn_ir, arg, iv_phi, lp, memo, interner, visiting)?;
                    changed |= mapped != arg;
                    new_args.push(mapped);
                }
                if !changed {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Intrinsic { op, args: new_args },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Phi { args } => {
                if let Some(var) = fn_ir.values[root].origin_var.clone()
                    && let Some(v) = materialize_passthrough_origin_phi_state_scalar(
                        fn_ir, root, &var, iv_phi, lp, memo, interner,
                    )
                {
                    v
                } else if phi_loads_same_var(fn_ir, &args) {
                    rec(fn_ir, args[0].0, iv_phi, lp, memo, interner, visiting)?
                } else if let Some((cond, then_val, else_val)) =
                    find_conditional_phi_shape(fn_ir, root, &args)
                {
                    let cond_v = rec(fn_ir, cond, iv_phi, lp, memo, interner, visiting)?;
                    let then_v = rec(fn_ir, then_val, iv_phi, lp, memo, interner, visiting)?;
                    let else_v = rec(fn_ir, else_val, iv_phi, lp, memo, interner, visiting)?;
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Call {
                            callee: "rr_ifelse_strict".to_string(),
                            args: vec![cond_v, then_v, else_v],
                            names: vec![None, None, None],
                        },
                        span,
                        facts,
                    )
                } else {
                    let mut picked: Option<ValueId> = None;
                    for (arg, _) in args {
                        let mapped = rec(fn_ir, arg, iv_phi, lp, memo, interner, visiting)?;
                        match picked {
                            None => picked = Some(mapped),
                            Some(prev)
                                if canonical_value(fn_ir, prev)
                                    == canonical_value(fn_ir, mapped) => {}
                            Some(_) => return None,
                        }
                    }
                    picked?
                }
            }
            ValueKind::Len { base } => {
                let base_v = rec(fn_ir, base, iv_phi, lp, memo, interner, visiting)?;
                if base_v == base {
                    root
                } else {
                    intern_materialized_value(
                        fn_ir,
                        interner,
                        ValueKind::Len { base: base_v },
                        span,
                        facts,
                    )
                }
            }
            ValueKind::Range { .. }
            | ValueKind::Indices { .. }
            | ValueKind::Index1D { .. }
            | ValueKind::Index2D { .. } => return None,
        };

        memo.insert(root, out);
        visiting.remove(&root);
        Some(out)
    }

    rec(
        fn_ir,
        root,
        iv_phi,
        lp,
        memo,
        interner,
        &mut FxHashSet::default(),
    )
}

fn is_loop_compatible_base(lp: &LoopInfo, fn_ir: &FnIR, base: ValueId) -> bool {
    if loop_matches_vec(lp, fn_ir, base) {
        return true;
    }
    let Some(loop_key) = loop_length_key(lp, fn_ir) else {
        return false;
    };
    match vector_length_key(fn_ir, base) {
        Some(k) => canonical_value(fn_ir, k) == canonical_value(fn_ir, loop_key),
        None => false,
    }
}

fn loop_length_key(lp: &LoopInfo, fn_ir: &FnIR) -> Option<ValueId> {
    let limit = lp.limit?;
    match &fn_ir.values[limit].kind {
        ValueKind::Len { base } => vector_length_key(fn_ir, *base),
        _ => Some(canonical_value(fn_ir, limit)),
    }
}

fn vector_length_key(fn_ir: &FnIR, root: ValueId) -> Option<ValueId> {
    fn single_assign_source(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
        let mut src: Option<ValueId> = None;
        for bb in &fn_ir.blocks {
            for ins in &bb.instrs {
                let Instr::Assign { dst, src: s, .. } = ins else {
                    continue;
                };
                if dst != var {
                    continue;
                }
                let s = canonical_value(fn_ir, *s);
                match src {
                    None => src = Some(s),
                    Some(prev) if canonical_value(fn_ir, prev) == s => {}
                    Some(_) => return None,
                }
            }
        }
        src
    }

    fn rec(fn_ir: &FnIR, root: ValueId, seen: &mut FxHashSet<ValueId>) -> Option<ValueId> {
        let root = canonical_value(fn_ir, root);
        if !seen.insert(root) {
            return None;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Load { var } => {
                let src = single_assign_source(fn_ir, var)?;
                rec(fn_ir, src, seen)
            }
            ValueKind::Call { callee, args, .. } if callee == "seq_len" && args.len() == 1 => {
                Some(canonical_value(fn_ir, args[0]))
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                let lk = rec(fn_ir, *lhs, seen);
                let rk = rec(fn_ir, *rhs, seen);
                match (lk, rk) {
                    (Some(a), Some(b))
                        if canonical_value(fn_ir, a) == canonical_value(fn_ir, b) =>
                    {
                        Some(canonical_value(fn_ir, a))
                    }
                    (Some(k), None) if is_scalar_value(fn_ir, *rhs) => Some(k),
                    (None, Some(k)) if is_scalar_value(fn_ir, *lhs) => Some(k),
                    _ => None,
                }
            }
            _ => None,
        }
    }
    rec(fn_ir, root, &mut FxHashSet::default())
}

fn same_length_proven(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    let a = canonical_value(fn_ir, a);
    let b = canonical_value(fn_ir, b);
    if a == b {
        return true;
    }
    if fn_ir.values[a].value_ty.len_sym.is_some()
        && fn_ir.values[a].value_ty.len_sym == fn_ir.values[b].value_ty.len_sym
    {
        return true;
    }
    match (vector_length_key(fn_ir, a), vector_length_key(fn_ir, b)) {
        (Some(ka), Some(kb)) => canonical_value(fn_ir, ka) == canonical_value(fn_ir, kb),
        _ => false,
    }
}

fn is_scalar_value(fn_ir: &FnIR, vid: ValueId) -> bool {
    matches!(
        fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. }
    )
}

fn is_iv_equivalent(fn_ir: &FnIR, candidate: ValueId, iv_phi: ValueId) -> bool {
    let mut seen_vals = FxHashSet::default();
    let mut seen_vars = FxHashSet::default();
    is_iv_equivalent_rec(fn_ir, candidate, iv_phi, &mut seen_vals, &mut seen_vars)
}

fn is_iv_equivalent_rec(
    fn_ir: &FnIR,
    candidate: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> bool {
    let candidate = canonical_value(fn_ir, candidate);
    if candidate == iv_phi {
        return true;
    }
    if !seen_vals.insert(candidate) {
        return false;
    }
    match &fn_ir.values[candidate].kind {
        ValueKind::Load { var } => {
            if induction_origin_var(fn_ir, iv_phi).as_deref() == Some(var.as_str()) {
                return true;
            }
            if let Some(src) = unique_assign_source(fn_ir, var)
                && src != candidate
            {
                return is_iv_equivalent_rec(fn_ir, src, iv_phi, seen_vals, seen_vars);
            }
            load_var_is_floor_like_iv(fn_ir, var, iv_phi, seen_vals, seen_vars)
        }
        ValueKind::Phi { args } if args.is_empty() => {
            match (
                fn_ir.values[candidate].origin_var.as_deref(),
                fn_ir.values[iv_phi].origin_var.as_deref(),
            ) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == candidate {
                    return false;
                }
                return is_iv_equivalent_rec(fn_ir, first, iv_phi, seen_vals, seen_vars);
            }
            let mut saw_iv_progress = false;
            for (v, _) in args {
                if is_iv_equivalent_rec(fn_ir, *v, iv_phi, seen_vals, seen_vars) {
                    saw_iv_progress = true;
                    continue;
                }
                if is_iv_seed_expr(
                    fn_ir,
                    *v,
                    iv_phi,
                    &mut FxHashSet::default(),
                    &mut FxHashSet::default(),
                ) {
                    continue;
                }
                return false;
            }
            saw_iv_progress
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let floor_like = matches!(callee.as_str(), "floor" | "ceiling" | "trunc");
            let single_positional = args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_equivalent_rec(fn_ir, args[0], iv_phi, seen_vals, seen_vars)
        }
        _ => false,
    }
}

fn induction_origin_var(fn_ir: &FnIR, iv_phi: ValueId) -> Option<String> {
    fn rec(fn_ir: &FnIR, vid: ValueId, seen: &mut FxHashSet<ValueId>) -> Option<String> {
        let vid = canonical_value(fn_ir, vid);
        if !seen.insert(vid) {
            return None;
        }
        if let Some(origin) = fn_ir.values[vid].origin_var.clone() {
            return Some(origin);
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Load { var } => Some(var.clone()),
            ValueKind::Phi { args } if !args.is_empty() => {
                let mut name: Option<String> = None;
                for (arg, _) in args {
                    let arg_name = rec(fn_ir, *arg, seen)?;
                    match &name {
                        None => name = Some(arg_name),
                        Some(prev) if prev == &arg_name => {}
                        Some(_) => return None,
                    }
                }
                name
            }
            _ => None,
        }
    }

    rec(fn_ir, iv_phi, &mut FxHashSet::default())
}

fn is_iv_seed_expr(
    fn_ir: &FnIR,
    vid: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> bool {
    let vid = canonical_value(fn_ir, vid);
    if vid == iv_phi {
        return false;
    }
    if !seen_vals.insert(vid) {
        return false;
    }
    match &fn_ir.values[vid].kind {
        ValueKind::Const(_) | ValueKind::Param { .. } => true,
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let floor_like = matches!(callee.as_str(), "floor" | "ceiling" | "trunc");
            let single_positional = args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_seed_expr(fn_ir, args[0], iv_phi, seen_vals, seen_vars)
        }
        ValueKind::Load { var } => {
            if !seen_vars.insert(var.to_string()) {
                return false;
            }
            let mut found = false;
            for bb in &fn_ir.blocks {
                for ins in &bb.instrs {
                    let Instr::Assign { dst, src, .. } = ins else {
                        continue;
                    };
                    if dst != var {
                        continue;
                    }
                    found = true;
                    if !is_iv_seed_expr(fn_ir, *src, iv_phi, seen_vals, seen_vars) {
                        seen_vars.remove(var);
                        return false;
                    }
                }
            }
            seen_vars.remove(var);
            found
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == vid {
                    return false;
                }
                return is_iv_seed_expr(fn_ir, first, iv_phi, seen_vals, seen_vars);
            }
            args.iter()
                .all(|(v, _)| is_iv_seed_expr(fn_ir, *v, iv_phi, seen_vals, seen_vars))
        }
        _ => false,
    }
}

fn load_var_is_floor_like_iv(
    fn_ir: &FnIR,
    var: &str,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> bool {
    fn is_seed_expr(fn_ir: &FnIR, src: ValueId) -> bool {
        matches!(
            fn_ir.values[canonical_value(fn_ir, src)].kind,
            ValueKind::Const(_) | ValueKind::Param { .. }
        )
    }

    if !seen_vars.insert(var.to_string()) {
        return false;
    }
    let mut found = false;
    let mut all_match = true;
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            // Ignore non-recursive seeds/reinitializations like `ii <- 0`.
            if is_seed_expr(fn_ir, *src) {
                continue;
            }
            found = true;
            if !is_floor_like_iv_expr(fn_ir, *src, iv_phi, seen_vals, seen_vars) {
                all_match = false;
                break;
            }
        }
        if !all_match {
            break;
        }
    }
    seen_vars.remove(var);
    found && all_match
}

fn is_floor_like_iv_expr(
    fn_ir: &FnIR,
    src: ValueId,
    iv_phi: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> bool {
    let src = canonical_value(fn_ir, src);
    match &fn_ir.values[src].kind {
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let floor_like = matches!(callee.as_str(), "floor" | "ceiling" | "trunc");
            let single_positional = args.len() == 1
                && names.len() <= 1
                && names
                    .first()
                    .and_then(std::option::Option::as_ref)
                    .is_none();
            floor_like
                && single_positional
                && is_iv_equivalent_rec(fn_ir, args[0], iv_phi, seen_vals, seen_vars)
        }
        ValueKind::Load { var } => {
            load_var_is_floor_like_iv(fn_ir, var, iv_phi, seen_vals, seen_vars)
        }
        ValueKind::Phi { args } if !args.is_empty() => {
            let first = canonical_value(fn_ir, args[0].0);
            if args
                .iter()
                .all(|(v, _)| canonical_value(fn_ir, *v) == first)
            {
                if first == src {
                    return false;
                }
                return is_floor_like_iv_expr(fn_ir, first, iv_phi, seen_vals, seen_vars);
            }
            let mut saw_progress = false;
            for (v, _) in args {
                if is_iv_seed_expr(
                    fn_ir,
                    *v,
                    iv_phi,
                    &mut FxHashSet::default(),
                    &mut FxHashSet::default(),
                ) {
                    continue;
                }
                saw_progress = true;
                if !is_floor_like_iv_expr(fn_ir, *v, iv_phi, seen_vals, seen_vars) {
                    return false;
                }
            }
            saw_progress
        }
        _ => false,
    }
}

fn is_value_equivalent(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    if a == b {
        return true;
    }
    if canonical_value(fn_ir, a) == canonical_value(fn_ir, b) {
        return true;
    }
    match (&fn_ir.values[a].kind, &fn_ir.values[b].kind) {
        (ValueKind::Load { var: va }, ValueKind::Load { var: vb }) => va == vb,
        (ValueKind::Load { var }, ValueKind::Phi { args }) if args.is_empty() => {
            fn_ir.values[b].origin_var.as_deref() == Some(var.as_str())
        }
        (ValueKind::Phi { args }, ValueKind::Load { var }) if args.is_empty() => {
            fn_ir.values[a].origin_var.as_deref() == Some(var.as_str())
        }
        _ => false,
    }
}

fn canonical_value(fn_ir: &FnIR, mut vid: ValueId) -> ValueId {
    let mut seen = FxHashSet::default();
    loop {
        if !seen.insert(vid) {
            return vid;
        }
        match &fn_ir.values[vid].kind {
            ValueKind::Phi { args } if !args.is_empty() => {
                let first = args[0].0;
                if args.iter().all(|(v, _)| *v == first) {
                    vid = first;
                    continue;
                }
                let mut unique_non_self = FxHashSet::default();
                for (v, _) in args {
                    if *v != vid {
                        unique_non_self.insert(*v);
                    }
                }
                if unique_non_self.len() == 1 {
                    // loop-invariant self-phi: v = phi(seed, v) -> seed
                    vid = *unique_non_self.iter().next().unwrap();
                    continue;
                }
            }
            _ => {}
        }
        return vid;
    }
}

fn should_hoist_callmap_arg_expr(fn_ir: &FnIR, vid: ValueId) -> bool {
    !matches!(
        &fn_ir.values[canonical_value(fn_ir, vid)].kind,
        ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. }
    )
}

fn next_callmap_tmp_var(fn_ir: &FnIR, prefix: &str) -> VarId {
    let mut idx = 0usize;
    loop {
        let candidate = format!(".tachyon_{}_{}", prefix, idx);
        if fn_ir.params.iter().all(|p| p != &candidate) && !has_any_var_binding(fn_ir, &candidate) {
            return candidate;
        }
        idx += 1;
    }
}

fn maybe_hoist_callmap_arg_expr(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    val: ValueId,
    arg_index: usize,
) -> ValueId {
    let val = resolve_materialized_value(fn_ir, val);
    if !should_hoist_callmap_arg_expr(fn_ir, val) {
        return val;
    }
    let var = next_callmap_tmp_var(fn_ir, &format!("callmap_arg{}", arg_index));
    let span = fn_ir.values[val].span;
    let facts = fn_ir.values[val].facts;
    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: var.clone(),
        src: val,
        span: crate::utils::Span::dummy(),
    });
    fn_ir.add_value(ValueKind::Load { var: var.clone() }, span, facts, Some(var))
}

fn hoist_vector_expr_temp(
    fn_ir: &mut FnIR,
    preheader: BlockId,
    val: ValueId,
    prefix: &str,
) -> ValueId {
    let val = resolve_materialized_value(fn_ir, val);
    if !should_hoist_callmap_arg_expr(fn_ir, val) {
        return val;
    }
    let var = next_callmap_tmp_var(fn_ir, prefix);
    let span = fn_ir.values[val].span;
    let facts = fn_ir.values[val].facts;
    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: var.clone(),
        src: val,
        span: crate::utils::Span::dummy(),
    });
    fn_ir.add_value(ValueKind::Load { var: var.clone() }, span, facts, Some(var))
}

fn resolve_materialized_value(fn_ir: &mut FnIR, vid: ValueId) -> ValueId {
    let c = canonical_value(fn_ir, vid);
    if let ValueKind::Phi { args } = &fn_ir.values[c].kind
        && args.is_empty()
        && let Some(var) = fn_ir.values[c].origin_var.clone()
    {
        return fn_ir.add_value(
            ValueKind::Load { var: var.clone() },
            fn_ir.values[c].span,
            fn_ir.values[c].facts,
            Some(var),
        );
    }
    c
}

fn resolve_load_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
    let mut cur = canonical_value(fn_ir, vid);
    let mut seen = FxHashSet::default();
    while seen.insert(cur) {
        if let ValueKind::Load { var } = &fn_ir.values[cur].kind
            && let Some(src) = unique_assign_source(fn_ir, var)
        {
            cur = canonical_value(fn_ir, src);
            continue;
        }
        break;
    }
    cur
}

fn unique_assign_source(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
    let mut src: Option<ValueId> = None;
    for bb in &fn_ir.blocks {
        for ins in &bb.instrs {
            let Instr::Assign { dst, src: s, .. } = ins else {
                continue;
            };
            if dst != var {
                continue;
            }
            let s = canonical_value(fn_ir, *s);
            match src {
                None => src = Some(s),
                Some(prev) if canonical_value(fn_ir, prev) == s => {}
                Some(_) => return None,
            }
        }
    }
    src
}
