use crate::mir::opt::loop_analysis::{LoopAnalyzer, LoopInfo, build_pred_map};
use crate::mir::*;
use crate::syntax::ast::BinOp;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Debug, Default, Clone, Copy)]
pub struct VOptStats {
    pub vectorized: usize,
    pub reduced: usize,
}

impl VOptStats {
    pub fn changed(self) -> bool {
        self.vectorized > 0 || self.reduced > 0
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
    let analyzer = LoopAnalyzer::new(fn_ir);
    let loops = analyzer.find_loops();

    for lp in loops {
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
        } else if let Some(plan) = match_map(fn_ir, &lp) {
            if apply_vectorization(fn_ir, &lp, plan) {
                stats.vectorized += 1;
            }
        }
    }

    stats
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
        if let ValueKind::Phi { args } = &val.kind {
            if args.len() == 2 && args.iter().any(|(_, b)| *b == lp.latch) {
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
                        if let Some(other) = acc_side {
                            if expr_has_iv_dependency(fn_ir, other, iv_phi)
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
                    }
                    _ => {}
                }
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
            {
                if is_iv_equivalent(fn_ir, *idx, iv_phi) && *is_safe && *is_na_safe {
                    let rhs_val = &fn_ir.values[*val];
                    if let ValueKind::Binary { op, lhs, rhs } = &rhs_val.kind {
                        let lhs_idx = as_safe_loop_index(fn_ir, *lhs, iv_phi);
                        let rhs_idx = as_safe_loop_index(fn_ir, *rhs, iv_phi);

                        // x[i] OP x[i]  ->  x OP x
                        if let (Some(lbase), Some(rbase)) = (lhs_idx, rhs_idx) {
                            if lbase == rbase && loop_matches_vec(lp, fn_ir, lbase) {
                                return Some(VectorPlan::Map {
                                    dest: *base,
                                    src: lbase,
                                    op: *op,
                                    other: rbase,
                                });
                            }
                        }

                        // x[i] OP k  ->  x OP k
                        if let Some(x_base) = lhs_idx {
                            if loop_matches_vec(lp, fn_ir, x_base) {
                                return Some(VectorPlan::Map {
                                    dest: *base,
                                    src: x_base,
                                    op: *op,
                                    other: *rhs,
                                });
                            }
                        }

                        // k OP x[i]  ->  k OP x
                        if let Some(x_base) = rhs_idx {
                            if loop_matches_vec(lp, fn_ir, x_base) {
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
                continue;
            }
            if !is_condition_vectorizable(fn_ir, cond, iv_phi, lp, user_call_whitelist)
                || !expr_has_iv_dependency(fn_ir, cond, iv_phi)
            {
                continue;
            }
            let (then_base, then_val) = match extract_store_1d_in_block(fn_ir, then_bb, iv_phi) {
                Some(v) => v,
                None => continue,
            };
            let (else_base, else_val) = match extract_store_1d_in_block(fn_ir, else_bb, iv_phi) {
                Some(v) => v,
                None => continue,
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
                    _ => continue,
                }
            };
            if !is_loop_compatible_base(lp, fn_ir, dest_base) {
                continue;
            }
            if !is_vectorizable_expr(fn_ir, then_val, iv_phi, lp, true, false) {
                continue;
            }
            if !is_vectorizable_expr(fn_ir, else_val, iv_phi, lp, true, false) {
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

pub(crate) fn is_builtin_vector_safe_call(callee: &str, arity: usize) -> bool {
    match callee {
        "abs" | "sqrt" | "exp" | "log" | "log10" | "log2" | "sin" | "cos" | "tan" | "asin"
        | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "sign" | "floor" | "ceiling" | "trunc"
        | "gamma" | "lgamma" | "is.na" | "is.finite" => arity == 1,
        "atan2" => arity == 2,
        "round" => arity == 1 || arity == 2,
        "pmax" | "pmin" => arity >= 2,
        _ => false,
    }
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
    {
        if is_iv_equivalent(fn_ir, *idx, iv_phi) && *is_safe && *is_na_safe {
            return Some(canonical_value(fn_ir, *base));
        }
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
                    let input_vec = match materialize_vector_expr(
                        fn_ir, vec_expr, iv_phi, idx_vec, lp, &mut memo, false, true,
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
                let input_vec = match materialize_vector_expr(
                    fn_ir, vec_expr, iv_phi, idx_vec, lp, &mut memo, false, true,
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
            let cond_vec = match materialize_vector_expr(
                fn_ir, cond, iv_phi, idx_vec, lp, &mut memo, true, true,
            ) {
                Some(v) => v,
                None => return false,
            };
            let then_vec = match materialize_vector_expr(
                fn_ir, then_val, iv_phi, idx_vec, lp, &mut memo, true, false,
            ) {
                Some(v) => v,
                None => return false,
            };
            let else_vec = match materialize_vector_expr(
                fn_ir, else_val, iv_phi, idx_vec, lp, &mut memo, true, false,
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
            let mut mapped_args = Vec::with_capacity(args.len());
            let mut vector_args = Vec::new();
            for (arg_i, arg) in args.into_iter().enumerate() {
                let out = if arg.vectorized {
                    match materialize_vector_expr(
                        fn_ir, arg.value, iv_phi, idx_vec, lp, &mut memo, true, false,
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
            if is_iv_equivalent(fn_ir, *lhs, iv_phi) {
                if let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind {
                    return Some(k);
                }
            }
            if is_iv_equivalent(fn_ir, *rhs, iv_phi) {
                if let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*lhs].kind {
                    return Some(k);
                }
            }
            None
        }
        ValueKind::Binary {
            op: BinOp::Sub,
            lhs,
            rhs,
        } => {
            if is_iv_equivalent(fn_ir, *lhs, iv_phi) {
                if let ValueKind::Const(Lit::Int(k)) = fn_ir.values[*rhs].kind {
                    return Some(-k);
                }
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
    if let Some(loop_base) = lp.is_seq_along {
        if let (Some(a), Some(b)) = (
            resolve_base_var(fn_ir, base),
            resolve_base_var(fn_ir, loop_base),
        ) {
            if a == b {
                return true;
            }
        }
    }
    if let Some(limit) = lp.is_seq_len {
        if let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind {
            if canonical_value(fn_ir, len_base) == base {
                return true;
            }
            if let (Some(a), Some(b)) = (
                resolve_base_var(fn_ir, base),
                resolve_base_var(fn_ir, len_base),
            ) {
                if a == b {
                    return true;
                }
            }
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

fn resolve_base_var(fn_ir: &FnIR, base: ValueId) -> Option<VarId> {
    if let ValueKind::Load { var } = &fn_ir.values[base].kind {
        return Some(var.clone());
    }
    fn_ir.values[base].origin_var.clone()
}

fn rewrite_returns_for_var(fn_ir: &mut FnIR, var: &str, new_val: ValueId) {
    for bid in 0..fn_ir.blocks.len() {
        if let Terminator::Return(Some(ret_vid)) = fn_ir.blocks[bid].term {
            if fn_ir.values[ret_vid].origin_var.as_deref() == Some(var) {
                fn_ir.blocks[bid].term = Terminator::Return(Some(new_val));
            }
        }
    }
}

fn extract_store_1d_in_block(
    fn_ir: &FnIR,
    bid: BlockId,
    iv_phi: ValueId,
) -> Option<(ValueId, ValueId)> {
    let mut found = None;
    for instr in &fn_ir.blocks[bid].instrs {
        let Instr::StoreIndex1D {
            base,
            idx,
            val,
            is_vector,
            ..
        } = instr
        else {
            continue;
        };
        if *is_vector || !is_iv_equivalent(fn_ir, *idx, iv_phi) {
            continue;
        }
        if found.is_some() {
            return None;
        }
        found = Some((*base, *val));
    }
    found
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

fn expr_has_iv_dependency(fn_ir: &FnIR, root: ValueId, iv_phi: ValueId) -> bool {
    fn rec(fn_ir: &FnIR, root: ValueId, iv_phi: ValueId, seen: &mut FxHashSet<ValueId>) -> bool {
        if is_iv_equivalent(fn_ir, root, iv_phi) {
            return true;
        }
        if !seen.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, *lhs, iv_phi, seen) || rec(fn_ir, *rhs, iv_phi, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, iv_phi, seen),
            ValueKind::Call { args, .. } => args.iter().any(|a| rec(fn_ir, *a, iv_phi, seen)),
            ValueKind::Phi { args } => args.iter().any(|(a, _)| rec(fn_ir, *a, iv_phi, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, *base, iv_phi, seen)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, *start, iv_phi, seen) || rec(fn_ir, *end, iv_phi, seen)
            }
            ValueKind::Index1D { idx, .. } => rec(fn_ir, *idx, iv_phi, seen),
            ValueKind::Index2D { r, c, .. } => {
                rec(fn_ir, *r, iv_phi, seen) || rec(fn_ir, *c, iv_phi, seen)
            }
            _ => false,
        }
    }
    rec(fn_ir, root, iv_phi, &mut FxHashSet::default())
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
                (!require_safe_index || (*is_safe && *is_na_safe))
                    && is_iv_equivalent(fn_ir, *idx, iv_phi)
                    && (allow_any_base || is_loop_compatible_base(lp, fn_ir, *base))
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
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        iv_phi: ValueId,
        lp: &LoopInfo,
        user_call_whitelist: &FxHashSet<String>,
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
                rec(fn_ir, *lhs, iv_phi, lp, user_call_whitelist, seen)
                    && rec(fn_ir, *rhs, iv_phi, lp, user_call_whitelist, seen)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, iv_phi, lp, user_call_whitelist, seen),
            // Data-dependent conditions are now allowed if the access is proven safe.
            ValueKind::Index1D {
                base,
                idx,
                is_safe,
                is_na_safe,
            } => {
                *is_safe
                    && *is_na_safe
                    && is_iv_equivalent(fn_ir, *idx, iv_phi)
                    && is_loop_compatible_base(lp, fn_ir, *base)
            }
            ValueKind::Index2D { .. } => false,
            ValueKind::Call { callee, args, .. } => {
                is_vector_safe_call(callee, args.len(), user_call_whitelist)
                    && args
                        .iter()
                        .all(|a| rec(fn_ir, *a, iv_phi, lp, user_call_whitelist, seen))
            }
            ValueKind::Phi { args } => args
                .iter()
                .all(|(a, _)| rec(fn_ir, *a, iv_phi, lp, user_call_whitelist, seen)),
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

fn materialize_vector_expr(
    fn_ir: &mut FnIR,
    root: ValueId,
    iv_phi: ValueId,
    idx_vec: ValueId,
    lp: &LoopInfo,
    memo: &mut FxHashMap<ValueId, ValueId>,
    allow_any_base: bool,
    require_safe_index: bool,
) -> Option<ValueId> {
    let root = canonical_value(fn_ir, root);
    if let Some(v) = memo.get(&root) {
        return Some(*v);
    }
    if is_iv_equivalent(fn_ir, root, iv_phi) {
        memo.insert(root, idx_vec);
        return Some(idx_vec);
    }

    let span = fn_ir.values[root].span;
    let facts = fn_ir.values[root].facts;
    let out = match fn_ir.values[root].kind.clone() {
        ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. } => root,
        ValueKind::Binary { op, lhs, rhs } => {
            let l = materialize_vector_expr(
                fn_ir,
                lhs,
                iv_phi,
                idx_vec,
                lp,
                memo,
                allow_any_base,
                require_safe_index,
            )?;
            let r = materialize_vector_expr(
                fn_ir,
                rhs,
                iv_phi,
                idx_vec,
                lp,
                memo,
                allow_any_base,
                require_safe_index,
            )?;
            if l == lhs && r == rhs {
                root
            } else {
                fn_ir.add_value(ValueKind::Binary { op, lhs: l, rhs: r }, span, facts, None)
            }
        }
        ValueKind::Unary { op, rhs } => {
            let r = materialize_vector_expr(
                fn_ir,
                rhs,
                iv_phi,
                idx_vec,
                lp,
                memo,
                allow_any_base,
                require_safe_index,
            )?;
            if r == rhs {
                root
            } else {
                fn_ir.add_value(ValueKind::Unary { op, rhs: r }, span, facts, None)
            }
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let mut new_args = Vec::with_capacity(args.len());
            let mut changed = false;
            for a in args {
                let na = materialize_vector_expr(
                    fn_ir,
                    a,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    allow_any_base,
                    require_safe_index,
                )?;
                changed |= na != a;
                new_args.push(na);
            }
            if !changed {
                root
            } else {
                fn_ir.add_value(
                    ValueKind::Call {
                        callee,
                        args: new_args,
                        names,
                    },
                    span,
                    facts,
                    None,
                )
            }
        }
        ValueKind::Intrinsic { op, args } => {
            let mut new_args = Vec::with_capacity(args.len());
            let mut changed = false;
            for a in args {
                let na = materialize_vector_expr(
                    fn_ir,
                    a,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    allow_any_base,
                    require_safe_index,
                )?;
                changed |= na != a;
                new_args.push(na);
            }
            if !changed {
                root
            } else {
                fn_ir.add_value(
                    ValueKind::Intrinsic { op, args: new_args },
                    span,
                    facts,
                    None,
                )
            }
        }
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => {
            if (!require_safe_index || (is_safe && is_na_safe))
                && is_iv_equivalent(fn_ir, idx, iv_phi)
                && (allow_any_base || is_loop_compatible_base(lp, fn_ir, base))
            {
                resolve_materialized_value(fn_ir, base)
            } else {
                return None;
            }
        }
        ValueKind::Len { base } => {
            let b = materialize_vector_expr(
                fn_ir,
                base,
                iv_phi,
                idx_vec,
                lp,
                memo,
                allow_any_base,
                require_safe_index,
            )?;
            if b == base {
                root
            } else {
                fn_ir.add_value(ValueKind::Len { base: b }, span, facts, None)
            }
        }
        ValueKind::Range { start, end } => {
            let s = materialize_vector_expr(
                fn_ir,
                start,
                iv_phi,
                idx_vec,
                lp,
                memo,
                allow_any_base,
                require_safe_index,
            )?;
            let e = materialize_vector_expr(
                fn_ir,
                end,
                iv_phi,
                idx_vec,
                lp,
                memo,
                allow_any_base,
                require_safe_index,
            )?;
            if s == start && e == end {
                root
            } else {
                fn_ir.add_value(ValueKind::Range { start: s, end: e }, span, facts, None)
            }
        }
        ValueKind::Indices { base } => {
            let b = materialize_vector_expr(
                fn_ir,
                base,
                iv_phi,
                idx_vec,
                lp,
                memo,
                allow_any_base,
                require_safe_index,
            )?;
            if b == base {
                root
            } else {
                fn_ir.add_value(ValueKind::Indices { base: b }, span, facts, None)
            }
        }
        ValueKind::Phi { args } => {
            let outside_args: Vec<ValueId> = args
                .iter()
                .filter_map(|(a, b)| if lp.body.contains(b) { None } else { Some(*a) })
                .collect();
            if outside_args.len() == 1 {
                let seed = materialize_vector_expr(
                    fn_ir,
                    outside_args[0],
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    allow_any_base,
                    require_safe_index,
                )?;
                memo.insert(root, seed);
                return Some(seed);
            }

            let mut picked: Option<ValueId> = None;
            for (a, _) in args {
                let na = materialize_vector_expr(
                    fn_ir,
                    a,
                    iv_phi,
                    idx_vec,
                    lp,
                    memo,
                    allow_any_base,
                    require_safe_index,
                )?;
                match picked {
                    None => picked = Some(na),
                    Some(prev) if canonical_value(fn_ir, prev) == canonical_value(fn_ir, na) => {}
                    Some(_) => return None,
                }
            }
            picked?
        }
        ValueKind::Index2D { .. } => return None,
    };

    memo.insert(root, out);
    Some(out)
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
    if candidate == iv_phi {
        return true;
    }
    if canonical_value(fn_ir, candidate) == iv_phi {
        return true;
    }
    match &fn_ir.values[candidate].kind {
        ValueKind::Load { var } => fn_ir.values[iv_phi].origin_var.as_deref() == Some(var.as_str()),
        ValueKind::Phi { args } if args.is_empty() => {
            match (
                fn_ir.values[candidate].origin_var.as_deref(),
                fn_ir.values[iv_phi].origin_var.as_deref(),
            ) {
                (Some(a), Some(b)) => a == b,
                _ => false,
            }
        }
        ValueKind::Phi { args } if !args.is_empty() => args
            .iter()
            .all(|(v, _)| is_iv_equivalent(fn_ir, *v, iv_phi)),
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
    match &fn_ir.values[canonical_value(fn_ir, vid)].kind {
        ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. } => false,
        _ => true,
    }
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

fn resolve_materialized_value(fn_ir: &mut FnIR, vid: ValueId) -> ValueId {
    let c = canonical_value(fn_ir, vid);
    if let ValueKind::Phi { args } = &fn_ir.values[c].kind {
        if args.is_empty() {
            if let Some(var) = fn_ir.values[c].origin_var.clone() {
                return fn_ir.add_value(
                    ValueKind::Load { var: var.clone() },
                    fn_ir.values[c].span,
                    fn_ir.values[c].facts,
                    Some(var),
                );
            }
        }
    }
    c
}

fn resolve_load_alias_value(fn_ir: &FnIR, vid: ValueId) -> ValueId {
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

    let mut cur = canonical_value(fn_ir, vid);
    let mut seen = FxHashSet::default();
    while seen.insert(cur) {
        match &fn_ir.values[cur].kind {
            ValueKind::Load { var } => {
                if let Some(src) = unique_assign_source(fn_ir, var) {
                    cur = canonical_value(fn_ir, src);
                    continue;
                }
            }
            _ => {}
        }
        break;
    }
    cur
}
