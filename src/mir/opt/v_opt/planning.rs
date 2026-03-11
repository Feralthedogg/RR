use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis3D {
    Dim1,
    Dim2,
    Dim3,
}

#[derive(Debug, Clone)]
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
    Reduce3D {
        kind: ReduceKind,
        acc_phi: ValueId,
        base: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
    },
    Map {
        dest: ValueId,
        src: ValueId,
        op: crate::syntax::ast::BinOp,
        other: ValueId,
        shadow_vars: Vec<VarId>,
    },
    CondMap {
        dest: ValueId,
        cond: ValueId,
        then_val: ValueId,
        else_val: ValueId,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
        whole_dest: bool,
        shadow_vars: Vec<VarId>,
    },
    CondMap3D {
        dest: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        cond_lhs: ValueId,
        cond_rhs: ValueId,
        cmp_op: crate::syntax::ast::BinOp,
        then_src: ValueId,
        else_src: ValueId,
        start: ValueId,
        end: ValueId,
    },
    CondMap3DGeneral {
        dest: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        cond_lhs: ValueId,
        cond_rhs: ValueId,
        cmp_op: crate::syntax::ast::BinOp,
        then_val: ValueId,
        else_val: ValueId,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
    },
    RecurrenceAddConst {
        base: ValueId,
        start: ValueId,
        end: ValueId,
        delta: ValueId,
        negate_delta: bool,
    },
    RecurrenceAddConst3D {
        base: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
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
    ShiftedMap3D {
        dest: ValueId,
        src: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
        offset: i64,
    },
    CallMap {
        dest: ValueId,
        callee: String,
        args: Vec<CallMapArg>,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
        whole_dest: bool,
        shadow_vars: Vec<VarId>,
    },
    CallMap3D {
        dest: ValueId,
        callee: String,
        args: Vec<CallMapArg>,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
    },
    CallMap3DGeneral {
        dest: ValueId,
        callee: String,
        args: Vec<CallMapArg>,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
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
        shadow_vars: Vec<VarId>,
    },
    ExprMap {
        dest: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
        whole_dest: bool,
        shadow_vars: Vec<VarId>,
    },
    ExprMap3D {
        dest: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
    },
    MultiExprMap3D {
        entries: Vec<ExprMapEntry3D>,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
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
    ScatterExprMap3D {
        dest: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        idx: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
    },
    ScatterExprMap3DGeneral {
        dest: ValueId,
        i: VectorAccessOperand3D,
        j: VectorAccessOperand3D,
        k: VectorAccessOperand3D,
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
    Map3D {
        dest: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
        lhs_src: ValueId,
        rhs_src: ValueId,
        op: crate::syntax::ast::BinOp,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct CallMapArg {
    pub(super) value: ValueId,
    pub(super) vectorized: bool,
}

#[derive(Debug, Clone)]
pub struct ExprMapEntry {
    pub(super) dest: ValueId,
    pub(super) expr: ValueId,
    pub(super) whole_dest: bool,
    pub(super) shadow_vars: Vec<VarId>,
}

#[derive(Debug, Clone)]
pub struct ExprMapEntry3D {
    pub(super) dest: ValueId,
    pub(super) expr: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) shadow_vars: Vec<VarId>,
}

#[derive(Debug, Clone)]
pub(super) struct CallMap3DMatchCandidate {
    pub(super) dest: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) callee: String,
    pub(super) args: Vec<CallMapArg>,
    pub(super) generalized: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CallMapLoweringMode {
    DirectVector,
    RuntimeAuto { helper_cost: u32 },
}

#[derive(Debug, Clone, Copy)]
pub(super) struct BlockStore1D {
    pub(super) base: ValueId,
    pub(super) idx: ValueId,
    pub(super) val: ValueId,
    pub(super) is_vector: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum BlockStore1DMatch {
    None,
    One(BlockStore1D),
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BlockStore3D {
    pub(super) base: ValueId,
    pub(super) i: ValueId,
    pub(super) j: ValueId,
    pub(super) k: ValueId,
    pub(super) val: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockStore3DMatch {
    None,
    One(BlockStore3D),
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReduceKind {
    Sum,
    Prod,
    Min,
    Max,
}

fn expr_has_non_iv_loop_state_load(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    root: ValueId,
    iv_phi: ValueId,
) -> bool {
    fn rec(
        fn_ir: &FnIR,
        lp: &LoopInfo,
        root: ValueId,
        iv_phi: ValueId,
        seen_vals: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> bool {
        let root = canonical_value(fn_ir, root);
        if is_iv_equivalent(fn_ir, root, iv_phi) || !seen_vals.insert(root) {
            return false;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Load { var } => {
                if !seen_vars.insert(var.clone()) {
                    return true;
                }
                let out = if let Some(phi) = unique_origin_phi_value_in_loop(fn_ir, lp, var) {
                    !is_iv_equivalent(fn_ir, phi, iv_phi)
                } else if let Some(src) = unique_assign_source_in_loop(fn_ir, lp, var) {
                    rec(fn_ir, lp, src, iv_phi, seen_vals, seen_vars)
                } else if let Some(src) = merged_assign_source_in_loop(fn_ir, lp, var) {
                    rec(fn_ir, lp, src, iv_phi, seen_vals, seen_vars)
                } else {
                    false
                };
                seen_vars.remove(var);
                out
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, lp, *lhs, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *rhs, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Unary { rhs, .. } => rec(fn_ir, lp, *rhs, iv_phi, seen_vals, seen_vars),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
                .iter()
                .any(|arg| rec(fn_ir, lp, *arg, iv_phi, seen_vals, seen_vars)),
            ValueKind::Phi { args } => args
                .iter()
                .any(|(arg, _)| rec(fn_ir, lp, *arg, iv_phi, seen_vals, seen_vars)),
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, lp, *start, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *end, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index1D { base, idx, .. } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *idx, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index2D { base, r, c } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *r, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *c, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Index3D { base, i, j, k } => {
                rec(fn_ir, lp, *base, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *i, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *j, iv_phi, seen_vals, seen_vars)
                    || rec(fn_ir, lp, *k, iv_phi, seen_vals, seen_vars)
            }
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
        }
    }

    rec(
        fn_ir,
        lp,
        root,
        iv_phi,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

fn reduction_has_non_acc_loop_state_assignments(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    iv_phi: ValueId,
) -> bool {
    let acc_var = phi_state_var(fn_ir, acc_phi).or_else(|| fn_ir.values[acc_phi].origin_var.clone());
    let iv_var = induction_origin_var(fn_ir, iv_phi);
    for bid in &lp.body {
        for ins in &fn_ir.blocks[*bid].instrs {
            let Instr::Assign { dst, src, .. } = ins else {
                continue;
            };
            if acc_var.as_deref() == Some(dst.as_str()) || iv_var.as_deref() == Some(dst.as_str()) {
                continue;
            }
            if dst.starts_with(".arg_") {
                continue;
            }
            if expr_has_non_iv_loop_state_load(fn_ir, lp, *src, iv_phi) {
                return true;
            }
        }
    }
    false
}

fn reduction_has_extra_state_phi(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    acc_phi: ValueId,
    iv_phi: ValueId,
) -> bool {
    let acc_phi = canonical_value(fn_ir, acc_phi);
    let mut seen = FxHashSet::default();
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if args.is_empty() {
            continue;
        }
        let Some(phi_bb) = value.phi_block else {
            continue;
        };
        if !lp.body.contains(&phi_bb) {
            continue;
        }
        let vid = canonical_value(fn_ir, vid);
        if vid == acc_phi || is_iv_equivalent(fn_ir, vid, iv_phi) || !seen.insert(vid) {
            continue;
        }
        return true;
    }
    false
}

pub(super) fn match_reduction(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let reduction_rhs_vectorizable = |root: ValueId| {
        if expr_contains_index3d(fn_ir, root) {
            is_vectorizable_expr(fn_ir, root, iv_phi, lp, true, false)
        } else {
            is_vectorizable_expr(fn_ir, root, iv_phi, lp, false, true)
        }
    };
    if loop_has_store_effect(fn_ir, lp) {
        // Conservative: do not fold reductions if loop writes memory.
        return None;
    }

    for (id, val) in fn_ir.values.iter().enumerate() {
        if let ValueKind::Phi { args } = &val.kind
            && args.len() == 2
            && args.iter().any(|(_, b)| *b == lp.latch)
        {
            let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
                continue;
            };
            let next_v = &fn_ir.values[*next_val];

            match &next_v.kind {
                ValueKind::Binary {
                    op: crate::syntax::ast::BinOp::Add,
                    lhs,
                    rhs,
                } => {
                    if *lhs == id || *rhs == id {
                        let other = if *lhs == id { *rhs } else { *lhs };
                        if reduction_has_non_acc_loop_state_assignments(fn_ir, lp, id, iv_phi)
                            || reduction_has_extra_state_phi(fn_ir, lp, id, iv_phi)
                        {
                            continue;
                        }
                        let acc_reads_self = phi_state_var(fn_ir, id)
                            .or_else(|| fn_ir.values[id].origin_var.clone())
                            .is_some_and(|acc_var| {
                                expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default())
                            });
                        if expr_has_iv_dependency(fn_ir, other, iv_phi)
                            && !acc_reads_self
                            && !expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi)
                            && !expr_has_unstable_loop_local_load(fn_ir, lp, other)
                            && !expr_has_ambiguous_loop_local_load(fn_ir, lp, other)
                            && !expr_has_non_vector_safe_call_in_vector_context(
                                fn_ir,
                                other,
                                iv_phi,
                                user_call_whitelist,
                                &mut FxHashSet::default(),
                            )
                            && reduction_rhs_vectorizable(other)
                        {
                            if vectorize_trace_enabled() {
                                let lhs_detail = match &fn_ir.values[other].kind {
                                    ValueKind::Binary { lhs, .. } => format!("{:?}", fn_ir.values[*lhs].kind),
                                    _ => "-".to_string(),
                                };
                                let rhs_detail = match &fn_ir.values[other].kind {
                                    ValueKind::Binary { rhs, .. } => format!("{:?}", fn_ir.values[*rhs].kind),
                                    _ => "-".to_string(),
                                };
                                eprintln!(
                                    "   [vec-reduce] {} sum acc={:?} other={:?} lhs={} rhs={}",
                                    fn_ir.name,
                                    fn_ir.values[id].origin_var,
                                    fn_ir.values[other].kind,
                                    lhs_detail,
                                    rhs_detail
                                );
                            }
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
                        if reduction_has_non_acc_loop_state_assignments(fn_ir, lp, id, iv_phi)
                            || reduction_has_extra_state_phi(fn_ir, lp, id, iv_phi)
                        {
                            continue;
                        }
                        let acc_reads_self = phi_state_var(fn_ir, id)
                            .or_else(|| fn_ir.values[id].origin_var.clone())
                            .is_some_and(|acc_var| {
                                expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default())
                            });
                        if expr_has_iv_dependency(fn_ir, other, iv_phi)
                            && !acc_reads_self
                            && !expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi)
                            && !expr_has_unstable_loop_local_load(fn_ir, lp, other)
                            && !expr_has_ambiguous_loop_local_load(fn_ir, lp, other)
                            && !expr_has_non_vector_safe_call_in_vector_context(
                                fn_ir,
                                other,
                                iv_phi,
                                user_call_whitelist,
                                &mut FxHashSet::default(),
                            )
                            && reduction_rhs_vectorizable(other)
                        {
                            if vectorize_trace_enabled() {
                                eprintln!(
                                    "   [vec-reduce] {} prod acc={:?} other={:?}",
                                    fn_ir.name,
                                    fn_ir.values[id].origin_var,
                                    fn_ir.values[other].kind
                                );
                            }
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
                        && !reduction_has_non_acc_loop_state_assignments(fn_ir, lp, id, iv_phi)
                        && !reduction_has_extra_state_phi(fn_ir, lp, id, iv_phi)
                        && !phi_state_var(fn_ir, id)
                            .or_else(|| fn_ir.values[id].origin_var.clone())
                            .is_some_and(|acc_var| {
                                expr_reads_var(fn_ir, other, &acc_var, &mut FxHashSet::default())
                            })
                        && expr_has_iv_dependency(fn_ir, other, iv_phi)
                        && !expr_has_non_iv_loop_state_load(fn_ir, lp, other, iv_phi)
                        && !expr_has_unstable_loop_local_load(fn_ir, lp, other)
                        && !expr_has_ambiguous_loop_local_load(fn_ir, lp, other)
                        && !expr_has_non_vector_safe_call_in_vector_context(
                            fn_ir,
                            other,
                            iv_phi,
                            user_call_whitelist,
                            &mut FxHashSet::default(),
                        )
                        && reduction_rhs_vectorizable(other)
                    {
                        if vectorize_trace_enabled() {
                            eprintln!(
                                "   [vec-reduce] {} minmax acc={:?} other={:?} callee={}",
                                fn_ir.name,
                                fn_ir.values[id].origin_var,
                                fn_ir.values[other].kind,
                                callee
                            );
                        }
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

pub(super) fn match_2d_row_reduction_sum(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
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
        let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
            continue;
        };
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

pub(super) fn match_2d_col_reduction_sum(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
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
        let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
            continue;
        };
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

pub(super) fn match_3d_axis_reduction(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
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
        let Some((next_val, _)) = args.iter().find(|(_, b)| *b == lp.latch) else {
            continue;
        };
        match &fn_ir.values[*next_val].kind {
            ValueKind::Binary {
                op: BinOp::Add,
                lhs,
                rhs,
            }
            | ValueKind::Binary {
                op: BinOp::Mul,
                lhs,
                rhs,
            } => {
                let kind = match &fn_ir.values[*next_val].kind {
                    ValueKind::Binary { op: BinOp::Add, .. } => ReduceKind::Sum,
                    ValueKind::Binary { op: BinOp::Mul, .. } => ReduceKind::Prod,
                    _ => unreachable!(),
                };
                let other = if *lhs == id {
                    *rhs
                } else if *rhs == id {
                    *lhs
                } else {
                    continue;
                };
                let ValueKind::Index3D { base, i, j, k } = fn_ir.values[other].kind else {
                    continue;
                };
                let Some((axis, fixed_a, fixed_b)) =
                    classify_3d_map_axis(fn_ir, base, i, j, k, iv_phi)
                else {
                    continue;
                };
                return Some(VectorPlan::Reduce3D {
                    kind,
                    acc_phi: id,
                    base: canonical_value(fn_ir, base),
                    axis,
                    fixed_a,
                    fixed_b,
                    start,
                    end,
                });
            }
            ValueKind::Call { callee, args, .. }
                if (callee == "min" || callee == "max") && args.len() == 2 =>
            {
                let (a, b) = (args[0], args[1]);
                let other = if a == id {
                    b
                } else if b == id {
                    a
                } else {
                    continue;
                };
                let ValueKind::Index3D { base, i, j, k } = fn_ir.values[other].kind else {
                    continue;
                };
                let Some((axis, fixed_a, fixed_b)) =
                    classify_3d_map_axis(fn_ir, base, i, j, k, iv_phi)
                else {
                    continue;
                };
                let kind = if callee == "min" {
                    ReduceKind::Min
                } else {
                    ReduceKind::Max
                };
                return Some(VectorPlan::Reduce3D {
                    kind,
                    acc_phi: id,
                    base: canonical_value(fn_ir, base),
                    axis,
                    fixed_a,
                    fixed_b,
                    start,
                    end,
                });
            }
            _ => {}
        }
    }
    None
}

pub(super) fn match_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let whole_dest = |dest: ValueId| loop_covers_whole_destination(lp, fn_ir, dest, iv.init_val);

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
                        && whole_dest(*base)
                        && whole_dest(lbase)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                            fn_ir,
                            lp,
                            &allowed_dests,
                            *base,
                            iv_phi,
                        ) else {
                            continue;
                        };
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: lbase,
                            op: *op,
                            other: rbase,
                            shadow_vars,
                        });
                    }

                    // x[i] OP k  ->  x OP k
                    if let Some(x_base) = lhs_idx
                        && loop_matches_vec(lp, fn_ir, x_base)
                        && whole_dest(*base)
                        && whole_dest(x_base)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                            fn_ir,
                            lp,
                            &allowed_dests,
                            *base,
                            iv_phi,
                        ) else {
                            continue;
                        };
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: x_base,
                            op: *op,
                            other: *rhs,
                            shadow_vars,
                        });
                    }

                    // k OP x[i]  ->  k OP x
                    if let Some(x_base) = rhs_idx
                        && loop_matches_vec(lp, fn_ir, x_base)
                        && whole_dest(*base)
                        && whole_dest(x_base)
                    {
                        let allowed_dests: Vec<VarId> =
                            resolve_base_var(fn_ir, *base).into_iter().collect();
                        let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                            fn_ir,
                            lp,
                            &allowed_dests,
                            *base,
                            iv_phi,
                        ) else {
                            continue;
                        };
                        return Some(VectorPlan::Map {
                            dest: *base,
                            src: *lhs,
                            op: *op,
                            other: x_base,
                            shadow_vars,
                        });
                    }
                }
            }
        }
    }
    None
}

pub(super) fn match_conditional_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
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
            let Some(shadow_vars) =
                collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest_base, iv_phi)
            else {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond-map] {} skip: loop carries non-destination state",
                        fn_ir.name
                    );
                }
                continue;
            };
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
                start,
                end,
                whole_dest: loop_covers_whole_destination(lp, fn_ir, dest_base, start),
                shadow_vars,
            });
        }
    }
    None
}

pub(super) fn match_conditional_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    _user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let trace_enabled = vectorize_trace_enabled();

    for &bid in &lp.body {
        if bid == lp.header {
            continue;
        }
        let Terminator::If {
            cond,
            then_bb,
            else_bb,
        } = fn_ir.blocks[bid].term
        else {
            continue;
        };
        if !lp.body.contains(&then_bb) || !lp.body.contains(&else_bb) {
            continue;
        }
        if !expr_has_iv_dependency(fn_ir, cond, iv_phi) {
            continue;
        }
        let ValueKind::Binary {
            op: cmp_op,
            lhs,
            rhs,
        } = fn_ir.values[cond].kind
        else {
            continue;
        };
        if !matches!(
            cmp_op,
            BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge | BinOp::Eq | BinOp::Ne
        ) {
            continue;
        }

        let then_store = classify_store_3d_in_block(fn_ir, then_bb);
        let else_store = classify_store_3d_in_block(fn_ir, else_bb);
        let (then_store, else_store) = match (then_store, else_store) {
            (BlockStore3DMatch::One(then_store), BlockStore3DMatch::One(else_store)) => {
                (then_store, else_store)
            }
            _ => {
                if trace_enabled {
                    eprintln!(
                        "   [vec-cond3d] {} skip: non-canonical StoreIndex3D branches",
                        fn_ir.name
                    );
                }
                continue;
            }
        };

        let then_base = canonical_value(fn_ir, then_store.base);
        let else_base = canonical_value(fn_ir, else_store.base);
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
                            "   [vec-cond3d] {} skip: then/else destination bases differ",
                            fn_ir.name
                        );
                    }
                    continue;
                }
            }
        };

        let Some((then_axis, then_fixed_a, then_fixed_b)) = classify_3d_map_axis(
            fn_ir,
            then_store.base,
            then_store.i,
            then_store.j,
            then_store.k,
            iv_phi,
        ) else {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: then branch axis classify failed",
                    fn_ir.name
                );
            }
            continue;
        };
        let Some((else_axis, else_fixed_a, else_fixed_b)) = classify_3d_map_axis(
            fn_ir,
            else_store.base,
            else_store.i,
            else_store.j,
            else_store.k,
            iv_phi,
        ) else {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: else branch axis classify failed",
                    fn_ir.name
                );
            }
            continue;
        };
        if then_axis != else_axis
            || !same_loop_invariant_value(fn_ir, then_fixed_a, else_fixed_a, iv_phi)
            || !same_loop_invariant_value(fn_ir, then_fixed_b, else_fixed_b, iv_phi)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: axis/fixed dims differ across branches",
                    fn_ir.name
                );
            }
            continue;
        }

        let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest_base).into_iter().collect();
        let Some(shadow_vars) =
            collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest_base, iv_phi)
        else {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: unsupported loop-carried state",
                    fn_ir.name
                );
            }
            continue;
        };
        if !shadow_vars.is_empty() {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: loop carries non-destination state",
                    fn_ir.name
                );
            }
            continue;
        }

        let direct_cond_lhs =
            axis3_operand_source(fn_ir, lhs, then_axis, then_fixed_a, then_fixed_b, iv_phi);
        let direct_cond_rhs =
            axis3_operand_source(fn_ir, rhs, then_axis, then_fixed_a, then_fixed_b, iv_phi);
        let direct_then = axis3_operand_source(
            fn_ir,
            then_store.val,
            then_axis,
            then_fixed_a,
            then_fixed_b,
            iv_phi,
        );
        let direct_else = axis3_operand_source(
            fn_ir,
            else_store.val,
            then_axis,
            then_fixed_a,
            then_fixed_b,
            iv_phi,
        );

        if let (Some(cond_lhs), Some(cond_rhs), Some(then_src), Some(else_src)) =
            (direct_cond_lhs, direct_cond_rhs, direct_then, direct_else)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} matched axis={:?} fixed_a={:?} fixed_b={:?}",
                    fn_ir.name, then_axis, then_fixed_a, then_fixed_b
                );
            }
            return Some(VectorPlan::CondMap3D {
                dest: dest_base,
                axis: then_axis,
                fixed_a: then_fixed_a,
                fixed_b: then_fixed_b,
                cond_lhs,
                cond_rhs,
                cmp_op,
                then_src,
                else_src,
                start,
                end,
            });
        }

        let cond_operands_ok = |root: ValueId| {
            is_vectorizable_expr(fn_ir, root, iv_phi, lp, true, false)
                || is_loop_invariant_scalar_expr(fn_ir, root, iv_phi, &FxHashSet::default())
        };
        if !cond_operands_ok(lhs) || !cond_operands_ok(rhs) {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: condition operands are not vectorizable",
                    fn_ir.name
                );
            }
            continue;
        }
        if !is_vectorizable_expr(fn_ir, then_store.val, iv_phi, lp, true, false)
            || !is_vectorizable_expr(fn_ir, else_store.val, iv_phi, lp, true, false)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: branch value is not vectorizable",
                    fn_ir.name
                );
            }
            continue;
        }
        if expr_has_unstable_loop_local_load(fn_ir, lp, lhs)
            || expr_has_unstable_loop_local_load(fn_ir, lp, rhs)
            || expr_has_unstable_loop_local_load(fn_ir, lp, then_store.val)
            || expr_has_unstable_loop_local_load(fn_ir, lp, else_store.val)
        {
            if trace_enabled {
                eprintln!(
                    "   [vec-cond3d] {} skip: unstable loop-local load in generalized conditional map",
                    fn_ir.name
                );
            }
            continue;
        }

        if trace_enabled {
            eprintln!(
                "   [vec-cond3d] {} matched generalized axis={:?} fixed_a={:?} fixed_b={:?}",
                fn_ir.name, then_axis, then_fixed_a, then_fixed_b
            );
        }
        return Some(VectorPlan::CondMap3DGeneral {
            dest: dest_base,
            axis: then_axis,
            fixed_a: then_fixed_a,
            fixed_b: then_fixed_b,
            cond_lhs: lhs,
            cond_rhs: rhs,
            cmp_op,
            then_val: then_store.val,
            else_val: else_store.val,
            iv_phi,
            start,
            end,
        });
    }
    None
}

pub(super) fn match_recurrence_add_const(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
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

pub(super) fn match_recurrence_add_const_3d(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let start = iv.init_val;
    let end = lp.limit?;
    let iv_phi = iv.phi_val;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (base, i, j, k, val) = match instr {
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => (*base, *i, *j, *k, *val),
                _ => continue,
            };
            let Some((axis, fixed_a, fixed_b)) = classify_3d_map_axis(fn_ir, base, i, j, k, iv_phi)
            else {
                continue;
            };
            let base = canonical_value(fn_ir, base);
            let val = resolve_load_alias_value(fn_ir, val);

            let ValueKind::Binary { op, lhs, rhs } = &fn_ir.values[val].kind else {
                continue;
            };
            if !matches!(op, BinOp::Add | BinOp::Sub) {
                continue;
            }
            let lhs = resolve_load_alias_value(fn_ir, *lhs);
            let rhs = resolve_load_alias_value(fn_ir, *rhs);

            let (prev_side, delta_side, negate_delta) =
                if is_prev_element_3d(fn_ir, lhs, base, axis, fixed_a, fixed_b, iv_phi) {
                    (lhs, rhs, *op == BinOp::Sub)
                } else if *op == BinOp::Add
                    && is_prev_element_3d(fn_ir, rhs, base, axis, fixed_a, fixed_b, iv_phi)
                {
                    (rhs, lhs, false)
                } else {
                    continue;
                };

            if !is_prev_element_3d(fn_ir, prev_side, base, axis, fixed_a, fixed_b, iv_phi) {
                continue;
            }
            if expr_has_iv_dependency(fn_ir, delta_side, iv_phi) {
                continue;
            }
            if expr_reads_base(fn_ir, delta_side, base) {
                continue;
            }

            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-recur3d] {} matched axis={:?} fixed_a={:?} fixed_b={:?} negate_delta={}",
                    fn_ir.name, axis, fixed_a, fixed_b, negate_delta
                );
            }
            return Some(VectorPlan::RecurrenceAddConst3D {
                base,
                axis,
                fixed_a,
                fixed_b,
                start,
                end,
                delta: delta_side,
                negate_delta,
            });
        }
    }
    None
}

pub(super) fn match_shifted_map(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
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
            if d == s && offset < 0 {
                // x[i+1] = x[i] is loop-carried: slice assignment would read the original RHS
                // instead of the progressively updated scalar state.
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

pub(super) fn match_shifted_map_3d(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest_base, i, j, k, rhs) = match instr {
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => (*base, *i, *j, *k, *val),
                _ => continue,
            };
            let Some((axis, fixed_a, fixed_b)) =
                classify_3d_map_axis(fn_ir, dest_base, i, j, k, iv_phi)
            else {
                continue;
            };

            let rhs = resolve_load_alias_value(fn_ir, rhs);
            let ValueKind::Index3D {
                base: src_base,
                i: src_i,
                j: src_j,
                k: src_k,
            } = fn_ir.values[rhs].kind.clone()
            else {
                continue;
            };

            let (src_axis_idx, src_fixed_a, src_fixed_b) = match axis {
                Axis3D::Dim1 => (src_i, src_j, src_k),
                Axis3D::Dim2 => (src_j, src_i, src_k),
                Axis3D::Dim3 => (src_k, src_i, src_j),
            };
            let src_axis_idx = resolve_load_alias_value(fn_ir, src_axis_idx);
            let src_fixed_a = resolve_load_alias_value(fn_ir, src_fixed_a);
            let src_fixed_b = resolve_load_alias_value(fn_ir, src_fixed_b);
            let fixed_a = resolve_load_alias_value(fn_ir, fixed_a);
            let fixed_b = resolve_load_alias_value(fn_ir, fixed_b);
            if !same_loop_invariant_value(fn_ir, src_fixed_a, fixed_a, iv_phi)
                || !same_loop_invariant_value(fn_ir, src_fixed_b, fixed_b, iv_phi)
            {
                continue;
            }

            let Some(offset) = affine_iv_offset(fn_ir, src_axis_idx, iv_phi) else {
                continue;
            };
            if offset == 0 {
                continue;
            }

            let d = canonical_value(fn_ir, dest_base);
            let s = canonical_value(fn_ir, src_base);
            if d == s && offset < 0 {
                continue;
            }
            if vectorize_trace_enabled() {
                eprintln!(
                    "   [vec-shift3d] {} matched axis={:?} offset={} fixed_a={:?} fixed_b={:?}",
                    fn_ir.name, axis, offset, fixed_a, fixed_b
                );
            }
            return Some(VectorPlan::ShiftedMap3D {
                dest: d,
                src: s,
                axis,
                fixed_a,
                fixed_b,
                start,
                end,
                offset,
            });
        }
    }
    None
}

pub(super) fn match_call_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;

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
            let allowed_dests: Vec<VarId> =
                resolve_base_var(fn_ir, canonical_value(fn_ir, dest_base))
                    .into_iter()
                    .collect();
            let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                fn_ir,
                lp,
                &allowed_dests,
                canonical_value(fn_ir, dest_base),
                iv_phi,
            ) else {
                continue;
            };
            return Some(VectorPlan::CallMap {
                dest: canonical_value(fn_ir, dest_base),
                callee: callee.clone(),
                args: mapped_args,
                iv_phi,
                start,
                end,
                whole_dest: loop_covers_whole_destination(
                    lp,
                    fn_ir,
                    canonical_value(fn_ir, dest_base),
                    start,
                ),
                shadow_vars,
            });
        }
    }
    None
}

pub(super) fn match_call_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut found: Option<CallMap3DMatchCandidate> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } | Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => {
                    return None;
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    if found.is_some() {
                        return None;
                    }
                    let rhs = resolve_load_alias_value(fn_ir, *val);
                    let ValueKind::Call { callee, args, .. } = &fn_ir.values[rhs].kind else {
                        continue;
                    };
                    if !is_vector_safe_call(callee, args.len(), user_call_whitelist) {
                        continue;
                    }
                    let Some((axis, fixed_a, fixed_b)) =
                        classify_3d_map_axis(fn_ir, *base, *i, *j, *k, iv_phi)
                    else {
                        continue;
                    };
                    let mut mapped_args = Vec::with_capacity(args.len());
                    let mut has_vector_arg = false;
                    let mut generalized = false;
                    for arg in args {
                        let arg = resolve_load_alias_value(fn_ir, *arg);
                        if expr_has_iv_dependency(fn_ir, arg, iv_phi) {
                            if let Some(src_base) = axis3_vector_operand_source(
                                fn_ir, arg, axis, fixed_a, fixed_b, iv_phi,
                            ) {
                                mapped_args.push(CallMapArg {
                                    value: src_base,
                                    vectorized: true,
                                });
                            } else if is_vectorizable_expr(fn_ir, arg, iv_phi, lp, true, false)
                                && !expr_has_unstable_loop_local_load(fn_ir, lp, arg)
                            {
                                mapped_args.push(CallMapArg {
                                    value: arg,
                                    vectorized: true,
                                });
                                generalized = true;
                            } else {
                                mapped_args.clear();
                                break;
                            }
                            has_vector_arg = true;
                        } else if is_loop_invariant_scalar_expr(
                            fn_ir,
                            arg,
                            iv_phi,
                            user_call_whitelist,
                        ) {
                            mapped_args.push(CallMapArg {
                                value: arg,
                                vectorized: false,
                            });
                        } else {
                            mapped_args.clear();
                            break;
                        }
                    }
                    if mapped_args.is_empty() || !has_vector_arg {
                        continue;
                    }
                    let allowed_dests: Vec<VarId> =
                        resolve_base_var(fn_ir, canonical_value(fn_ir, *base))
                            .into_iter()
                            .collect();
                    let Some(shadow_vars) = collect_loop_shadow_vars_for_dest(
                        fn_ir,
                        lp,
                        &allowed_dests,
                        canonical_value(fn_ir, *base),
                        iv_phi,
                    ) else {
                        continue;
                    };
                    if !shadow_vars.is_empty() {
                        continue;
                    }
                    found = Some(CallMap3DMatchCandidate {
                        dest: canonical_value(fn_ir, *base),
                        axis,
                        fixed_a,
                        fixed_b,
                        callee: callee.clone(),
                        args: mapped_args,
                        generalized,
                    });
                }
            }
        }
    }

    let CallMap3DMatchCandidate {
        dest,
        axis,
        fixed_a,
        fixed_b,
        callee,
        args,
        generalized,
    } = found?;
    if generalized {
        Some(VectorPlan::CallMap3DGeneral {
            dest,
            callee,
            args,
            axis,
            fixed_a,
            fixed_b,
            iv_phi,
            start,
            end,
        })
    } else {
        Some(VectorPlan::CallMap3D {
            dest,
            callee,
            args,
            axis,
            fixed_a,
            fixed_b,
            start,
            end,
        })
    }
}

pub(super) fn match_expr_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut found: Option<(ValueId, ValueId, Axis3D, ValueId, ValueId)> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } | Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => {
                    return None;
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    if found.is_some() {
                        return None;
                    }
                    let Some((axis, fixed_a, fixed_b)) =
                        classify_3d_map_axis(fn_ir, *base, *i, *j, *k, iv_phi)
                    else {
                        continue;
                    };
                    let expr = resolve_load_alias_value(fn_ir, *val);
                    if !is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false) {
                        continue;
                    }
                    if expr_has_non_vector_safe_call_in_vector_context(
                        fn_ir,
                        expr,
                        iv_phi,
                        user_call_whitelist,
                        &mut FxHashSet::default(),
                    ) {
                        continue;
                    }
                    if expr_has_unstable_loop_local_load(fn_ir, lp, expr) {
                        continue;
                    }
                    let dest = canonical_value(fn_ir, *base);
                    let allowed_dests: Vec<VarId> =
                        resolve_base_var(fn_ir, dest).into_iter().collect();
                    let Some(shadow_vars) =
                        collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)
                    else {
                        continue;
                    };
                    if !shadow_vars.is_empty() || expr_reads_base(fn_ir, expr, dest) {
                        continue;
                    }
                    found = Some((dest, expr, axis, fixed_a, fixed_b));
                }
            }
        }
    }

    let (dest, expr, axis, fixed_a, fixed_b) = found?;
    Some(VectorPlan::ExprMap3D {
        dest,
        expr,
        iv_phi,
        axis,
        fixed_a,
        fixed_b,
        start,
        end,
    })
}

pub(super) fn match_multi_expr_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut pending_entries: Vec<(ValueId, ValueId, Axis3D, ValueId, ValueId)> = Vec::new();

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } | Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => {
                    return None;
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    let (axis, fixed_a, fixed_b) =
                        classify_3d_map_axis(fn_ir, *base, *i, *j, *k, iv_phi)?;
                    let expr = resolve_load_alias_value(fn_ir, *val);
                    if !is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false) {
                        return None;
                    }
                    if expr_has_non_vector_safe_call_in_vector_context(
                        fn_ir,
                        expr,
                        iv_phi,
                        user_call_whitelist,
                        &mut FxHashSet::default(),
                    ) {
                        return None;
                    }
                    if expr_has_unstable_loop_local_load(fn_ir, lp, expr) {
                        return None;
                    }
                    let dest = canonical_value(fn_ir, *base);
                    if expr_reads_base(fn_ir, expr, dest) {
                        return None;
                    }
                    if pending_entries
                        .iter()
                        .any(|(prev_dest, _, prev_axis, prev_a, prev_b)| {
                            *prev_dest == dest
                                && *prev_axis == axis
                                && same_loop_invariant_value(fn_ir, *prev_a, fixed_a, iv_phi)
                                && same_loop_invariant_value(fn_ir, *prev_b, fixed_b, iv_phi)
                        })
                    {
                        return None;
                    }
                    pending_entries.push((dest, expr, axis, fixed_a, fixed_b));
                }
            }
        }
    }

    if pending_entries.len() <= 1 {
        return None;
    }

    let allowed_dests: Vec<VarId> = pending_entries
        .iter()
        .filter_map(|(dest, ..)| resolve_base_var(fn_ir, *dest))
        .collect();
    let mut entries = Vec::with_capacity(pending_entries.len());
    for (dest, expr, axis, fixed_a, fixed_b) in pending_entries {
        let shadow_vars =
            collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;
        entries.push(ExprMapEntry3D {
            dest,
            expr,
            axis,
            fixed_a,
            fixed_b,
            shadow_vars,
        });
    }

    Some(VectorPlan::MultiExprMap3D {
        entries,
        iv_phi,
        start,
        end,
    })
}

#[derive(Clone, Copy)]
pub(super) enum ExprMapStoreCandidate {
    Standard {
        dest: ValueId,
        expr: ValueId,
    },
    Cube {
        dest: ValueId,
        expr: ValueId,
        cube: CubeSliceIndexInfo,
    },
}

#[derive(Clone, Copy)]
pub(super) struct ExprMapStoreSpec {
    base: ValueId,
    idx: ValueId,
    expr: ValueId,
    is_vector: bool,
}

pub(super) fn is_canonical_expr_map_store_index(
    fn_ir: &FnIR,
    idx: ValueId,
    iv_phi: ValueId,
) -> bool {
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

pub(super) fn trace_expr_map_non_canonical_store_reject(
    fn_ir: &FnIR,
    idx: ValueId,
    idx_ok: bool,
    iv_phi: ValueId,
) {
    if !vectorize_trace_enabled() {
        return;
    }
    let phi_detail = match &fn_ir.values[idx].kind {
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
        fn_ir.name, idx_ok, fn_ir.values[idx].kind, phi_detail
    );
}

pub(super) fn trace_expr_map_duplicate_store_reject() {
    if vectorize_trace_enabled() {
        eprintln!("   [vec-expr-map] reject: duplicate StoreIndex1D destination");
    }
}

pub(super) fn validate_expr_map_rhs(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
    dest: ValueId,
    expr: ValueId,
    iv_phi: ValueId,
) -> bool {
    let expr_iv_dependent = expr_has_iv_dependency(fn_ir, expr, iv_phi);
    let expr_ok = if expr_iv_dependent {
        is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false)
    } else {
        is_loop_invariant_scalar_expr(fn_ir, expr, iv_phi, user_call_whitelist)
    };
    if !expr_ok {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-expr-map] reject: rhs is neither vectorizable nor loop-invariant scalar"
            );
        }
        return false;
    }
    if expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        expr,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    ) {
        if vectorize_trace_enabled() {
            let rhs_detail = match &fn_ir.values[expr].kind {
                ValueKind::Binary { lhs, rhs, .. } => format!(
                    "lhs={:?} rhs={:?}",
                    fn_ir.values[*lhs].kind, fn_ir.values[*rhs].kind
                ),
                other => format!("{:?}", other),
            };
            eprintln!(
                "   [vec-expr-map] reject: rhs contains non-vector-safe call; rhs={:?}; detail={}",
                fn_ir.values[expr].kind, rhs_detail
            );
        }
        return false;
    }
    if expr_has_ambiguous_loop_local_load(fn_ir, lp, expr) {
        if vectorize_trace_enabled() {
            eprintln!("   [vec-expr-map] reject: rhs depends on ambiguous loop-local state");
        }
        return false;
    }
    if expr_reads_base_non_iv(fn_ir, expr, dest, iv_phi) {
        if vectorize_trace_enabled() {
            eprintln!("   [vec-expr-map] reject: loop-carried dependence on destination");
        }
        return false;
    }
    true
}

pub(super) fn classify_expr_map_store_candidate(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
    store: ExprMapStoreSpec,
    iv_phi: ValueId,
) -> Option<ExprMapStoreCandidate> {
    let idx_ok = is_canonical_expr_map_store_index(fn_ir, store.idx, iv_phi);
    if store.is_vector {
        if vectorize_trace_enabled() {
            eprintln!(
                "   [vec-expr-map] reject: non-canonical store index (fn={}, idx_ok={}, idx={:?})",
                fn_ir.name, idx_ok, fn_ir.values[store.idx].kind
            );
        }
        return None;
    }

    let dest = canonical_value(fn_ir, store.base);
    if !idx_ok {
        let Some(cube) =
            match_cube_slice_index_info(fn_ir, lp, user_call_whitelist, store.idx, iv_phi)
        else {
            trace_expr_map_non_canonical_store_reject(fn_ir, store.idx, idx_ok, iv_phi);
            return None;
        };
        if !validate_expr_map_rhs(fn_ir, lp, user_call_whitelist, dest, store.expr, iv_phi) {
            return None;
        }
        return Some(ExprMapStoreCandidate::Cube {
            dest,
            expr: store.expr,
            cube,
        });
    }

    if !validate_expr_map_rhs(fn_ir, lp, user_call_whitelist, dest, store.expr, iv_phi) {
        return None;
    }
    Some(ExprMapStoreCandidate::Standard {
        dest,
        expr: store.expr,
    })
}

pub(super) fn build_expr_map_entries(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    iv_phi: ValueId,
    start: ValueId,
    pending_entries: Vec<(ValueId, ValueId)>,
) -> Option<Vec<ExprMapEntry>> {
    let allowed_dests: Vec<VarId> = pending_entries
        .iter()
        .filter_map(|(dest, _)| resolve_base_var(fn_ir, *dest))
        .collect();
    let mut entries = Vec::with_capacity(pending_entries.len());
    for (dest, expr) in pending_entries {
        let shadow_vars =
            collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;
        entries.push(ExprMapEntry {
            dest,
            expr,
            whole_dest: loop_covers_whole_destination(lp, fn_ir, dest, start),
            shadow_vars,
        });
    }
    Some(entries)
}

pub(super) fn finalize_expr_map_plan(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    iv_phi: ValueId,
    start: ValueId,
    end: ValueId,
    found: Vec<(ValueId, ValueId)>,
    cube_candidate: Option<(ValueId, ValueId, CubeSliceIndexInfo)>,
) -> Option<VectorPlan> {
    if found.is_empty() {
        let (dest, expr, cube) = cube_candidate?;
        let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest).into_iter().collect();
        let shadow_vars =
            collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;
        return Some(VectorPlan::CubeSliceExprMap {
            dest,
            expr,
            iv_phi,
            face: cube.face,
            row: cube.row,
            size: cube.size,
            ctx: cube.ctx,
            start,
            end,
            shadow_vars,
        });
    }
    if cube_candidate.is_some() {
        return None;
    }

    let mut entries = build_expr_map_entries(fn_ir, lp, iv_phi, start, found)?;
    if entries.len() == 1 {
        let entry = entries.remove(0);
        return Some(VectorPlan::ExprMap {
            dest: entry.dest,
            expr: entry.expr,
            iv_phi,
            start,
            end,
            whole_dest: entry.whole_dest,
            shadow_vars: entry.shadow_vars,
        });
    }
    Some(VectorPlan::MultiExprMap {
        entries,
        iv_phi,
        start,
        end,
    })
}

pub(super) fn match_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let start = iv.init_val;
    let end = lp.limit?;
    let mut found: Vec<(ValueId, ValueId)> = Vec::new();
    let mut cube_candidate: Option<(ValueId, ValueId, CubeSliceIndexInfo)> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } | Instr::Eval { .. } => {}
                Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => return None,
                Instr::StoreIndex1D {
                    base,
                    idx,
                    val,
                    is_vector,
                    ..
                } => {
                    let candidate = classify_expr_map_store_candidate(
                        fn_ir,
                        lp,
                        user_call_whitelist,
                        ExprMapStoreSpec {
                            base: *base,
                            idx: *idx,
                            expr: *val,
                            is_vector: *is_vector,
                        },
                        iv_phi,
                    )?;

                    match candidate {
                        ExprMapStoreCandidate::Standard { dest, expr } => {
                            if cube_candidate.is_some()
                                || found.iter().any(|(existing_dest, _)| {
                                    same_base_value(fn_ir, *existing_dest, dest)
                                })
                            {
                                trace_expr_map_duplicate_store_reject();
                                return None;
                            }
                            found.push((dest, expr));
                        }
                        ExprMapStoreCandidate::Cube { dest, expr, cube } => {
                            if !found.is_empty() || cube_candidate.is_some() {
                                trace_expr_map_duplicate_store_reject();
                                return None;
                            }
                            cube_candidate = Some((dest, expr, cube));
                        }
                    }
                }
            }
        }
    }

    finalize_expr_map_plan(fn_ir, lp, iv_phi, start, end, found, cube_candidate)
}

pub(super) fn match_scatter_expr_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let trace_enabled = vectorize_trace_enabled();
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let mut found: Option<(BlockId, ValueId, ValueId, ValueId)> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } | Instr::Eval { .. } => {}
                Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-scatter] {} reject: saw multi-dimensional store in loop body",
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
                                "   [vec-scatter] {} reject: non-canonical store count/vector flag (is_vector={}, found_already={})",
                                fn_ir.name,
                                is_vector,
                                found.is_some()
                            );
                        }
                        return None;
                    }
                    found = Some((bid, canonical_value(fn_ir, *base), *idx, *val));
                }
            }
        }
    }

    let (store_bid, dest, idx, expr) = found?;
    let idx_root = resolve_match_alias_value(fn_ir, idx);
    let is_cube_scatter = matches!(
        &fn_ir.values[idx_root].kind,
        ValueKind::Call { callee, args, names }
            if callee == "rr_idx_cube_vec_i"
                && (args.len() == 4 || args.len() == 5)
                && names.iter().all(|n| n.is_none())
    );
    if store_bid != lp.latch && !is_cube_scatter {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: store not in latch (bb={} latch={})",
                fn_ir.name, store_bid, lp.latch
            );
        }
        return None;
    }
    if lp.body.len() > 2 && !is_cube_scatter {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: loop body too large for non-cube scatter (body_len={})",
                fn_ir.name,
                lp.body.len()
            );
        }
        return None;
    }
    if is_iv_equivalent(fn_ir, idx, iv_phi)
        || is_floor_like_iv_expr(
            fn_ir,
            idx,
            iv_phi,
            &mut FxHashSet::default(),
            &mut FxHashSet::default(),
        )
    {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: index is canonical IV-equivalent",
                fn_ir.name
            );
        }
        return None;
    }
    if !expr_has_iv_dependency(fn_ir, idx, iv_phi) {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: index has no IV dependency ({:?})",
                fn_ir.name, fn_ir.values[idx_root].kind
            );
        }
        return None;
    }
    if !is_vectorizable_expr(fn_ir, idx, iv_phi, lp, true, false)
        || !is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false)
    {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: idx/expr not vectorizable (idx={:?} expr={:?})",
                fn_ir.name,
                fn_ir.values[idx_root].kind,
                fn_ir.values[canonical_value(fn_ir, expr)].kind
            );
        }
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
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: idx/expr contains non-vector-safe call",
                fn_ir.name
            );
        }
        return None;
    }
    if expr_has_unstable_loop_local_load(fn_ir, lp, idx)
        || expr_has_unstable_loop_local_load(fn_ir, lp, expr)
    {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: idx/expr depends on unstable loop-local state",
                fn_ir.name
            );
        }
        return None;
    }
    if expr_reads_base_non_iv(fn_ir, expr, dest, iv_phi) {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: expr reads destination via non-IV access",
                fn_ir.name
            );
        }
        return None;
    }
    if expr_reads_base(fn_ir, expr, dest) {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter] {} reject: expr reads destination base",
                fn_ir.name
            );
        }
        return None;
    }
    Some(VectorPlan::ScatterExprMap {
        dest,
        idx,
        expr,
        iv_phi,
    })
}

pub(super) fn match_scatter_expr_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    let trace_enabled = vectorize_trace_enabled();
    enum Scatter3DMatch {
        SingleAxis {
            store_bid: BlockId,
            dest: ValueId,
            axis: Axis3D,
            fixed_a: ValueId,
            fixed_b: ValueId,
            idx: ValueId,
            expr: ValueId,
        },
        General {
            store_bid: BlockId,
            dest: ValueId,
            pattern: VectorAccessPattern3D,
            expr: ValueId,
        },
    }
    let mut found: Option<Scatter3DMatch> = None;

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { .. } | Instr::StoreIndex1D { .. } | Instr::StoreIndex2D { .. } => {
                    return None;
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    if found.is_some() {
                        return None;
                    }
                    let Some(pattern) =
                        classify_3d_general_vector_access(fn_ir, *base, *i, *j, *k, iv_phi)
                    else {
                        continue;
                    };
                    if let Some((axis, dep_idx, fixed_a, fixed_b)) =
                        classify_3d_vector_access_axis(fn_ir, *base, *i, *j, *k, iv_phi)
                    {
                        if is_iv_equivalent(fn_ir, dep_idx, iv_phi)
                            || is_floor_like_iv_expr(
                                fn_ir,
                                dep_idx,
                                iv_phi,
                                &mut FxHashSet::default(),
                                &mut FxHashSet::default(),
                            )
                        {
                            continue;
                        }
                        found = Some(Scatter3DMatch::SingleAxis {
                            store_bid: bid,
                            dest: canonical_value(fn_ir, *base),
                            axis,
                            fixed_a,
                            fixed_b,
                            idx: dep_idx,
                            expr: *val,
                        });
                    } else {
                        found = Some(Scatter3DMatch::General {
                            store_bid: bid,
                            dest: canonical_value(fn_ir, *base),
                            pattern,
                            expr: *val,
                        });
                    }
                }
            }
        }
    }

    let (store_bid, dest, idx_reads, expr, single_axis, general_pattern) = match found? {
        Scatter3DMatch::SingleAxis {
            store_bid,
            dest,
            axis,
            fixed_a,
            fixed_b,
            idx,
            expr,
        } => (
            store_bid,
            dest,
            vec![idx],
            expr,
            Some((axis, fixed_a, fixed_b, idx)),
            None,
        ),
        Scatter3DMatch::General {
            store_bid,
            dest,
            pattern,
            expr,
        } => {
            let idx_reads = [pattern.i, pattern.j, pattern.k]
                .into_iter()
                .filter_map(|operand| match operand {
                    VectorAccessOperand3D::Scalar(_) => None,
                    VectorAccessOperand3D::Vector(value) => Some(value),
                })
                .collect();
            (store_bid, dest, idx_reads, expr, None, Some(pattern))
        }
    };
    if store_bid != lp.latch && lp.body.len() > 2 {
        return None;
    }
    if idx_reads.is_empty()
        || !idx_reads
            .iter()
            .all(|idx| expr_has_iv_dependency(fn_ir, *idx, iv_phi))
        || !idx_reads
            .iter()
            .all(|idx| is_vectorizable_expr(fn_ir, *idx, iv_phi, lp, true, false))
        || !is_vectorizable_expr(fn_ir, expr, iv_phi, lp, true, false)
    {
        return None;
    }
    if idx_reads.iter().any(|idx| {
        expr_has_non_vector_safe_call_in_vector_context(
            fn_ir,
            *idx,
            iv_phi,
            user_call_whitelist,
            &mut FxHashSet::default(),
        )
    }) || expr_has_non_vector_safe_call_in_vector_context(
        fn_ir,
        expr,
        iv_phi,
        user_call_whitelist,
        &mut FxHashSet::default(),
    ) {
        return None;
    }
    if idx_reads
        .iter()
        .any(|idx| expr_has_unstable_loop_local_load(fn_ir, lp, *idx))
        || expr_has_unstable_loop_local_load(fn_ir, lp, expr)
    {
        return None;
    }
    let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest).into_iter().collect();
    let shadow_vars = collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;
    if !shadow_vars.is_empty() || expr_reads_base(fn_ir, expr, dest) {
        if trace_enabled {
            eprintln!(
                "   [vec-scatter3d] {} reject: expr reads destination or loop carries non-destination state",
                fn_ir.name
            );
        }
        return None;
    }

    if let Some((axis, fixed_a, fixed_b, idx)) = single_axis {
        Some(VectorPlan::ScatterExprMap3D {
            dest,
            axis,
            fixed_a,
            fixed_b,
            idx,
            expr,
            iv_phi,
        })
    } else {
        general_pattern.map(|pattern| VectorPlan::ScatterExprMap3DGeneral {
            dest,
            i: pattern.i,
            j: pattern.j,
            k: pattern.k,
            expr,
            iv_phi,
        })
    }
}

pub(super) fn match_cube_slice_expr_map(
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
                Instr::StoreIndex2D { .. } | Instr::StoreIndex3D { .. } => {
                    if trace_enabled {
                        eprintln!(
                            "   [vec-cube-slice] {} reject: saw multi-dimensional store in loop body",
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
    let cube = match_cube_slice_index_info(fn_ir, lp, user_call_whitelist, idx, iv_phi)?;

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
    let allowed_dests: Vec<VarId> = resolve_base_var(fn_ir, dest).into_iter().collect();
    let shadow_vars = collect_loop_shadow_vars_for_dest(fn_ir, lp, &allowed_dests, dest, iv_phi)?;

    Some(VectorPlan::CubeSliceExprMap {
        dest,
        expr,
        iv_phi,
        face: cube.face,
        row: cube.row,
        size: cube.size,
        ctx: cube.ctx,
        start,
        end,
        shadow_vars,
    })
}

#[derive(Clone, Copy)]
pub(super) struct CubeSliceIndexInfo {
    face: ValueId,
    row: ValueId,
    size: ValueId,
    ctx: Option<ValueId>,
}

pub(super) fn match_cube_slice_index_info(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    user_call_whitelist: &FxHashSet<String>,
    idx: ValueId,
    iv_phi: ValueId,
) -> Option<CubeSliceIndexInfo> {
    let trace_enabled = vectorize_trace_enabled();
    let idx_root = resolve_match_alias_value(fn_ir, idx);
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &fn_ir.values[idx_root].kind
    else {
        if trace_enabled {
            eprintln!(
                "   [vec-cube-slice] {} reject: store index is not call ({:?})",
                fn_ir.name, fn_ir.values[idx_root].kind
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

    Some(CubeSliceIndexInfo {
        face: args[0],
        row: args[1],
        size: args[3],
        ctx,
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
        "rr_index1_read_vec" | "rr_index1_read_vec_floor" | "rr_gather" => arity == 2 || arity == 3,
        "atan2" => arity == 2,
        "round" => arity == 1 || arity == 2,
        "pmax" | "pmin" => arity >= 2,
        _ => false,
    }
}
