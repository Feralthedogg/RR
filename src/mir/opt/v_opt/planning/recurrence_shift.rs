use super::*;

pub(in crate::mir::opt::v_opt) fn match_recurrence_add_const(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
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

pub(in crate::mir::opt::v_opt) fn match_recurrence_add_const_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
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

pub(in crate::mir::opt::v_opt) fn match_shifted_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
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

pub(in crate::mir::opt::v_opt) fn match_shifted_map_3d(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<VectorPlan> {
    if loop_has_inner_branch(fn_ir, lp) {
        return None;
    }
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
