use super::*;

pub(in crate::mir::opt::v_opt) fn match_2d_row_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<VectorPlan> {
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

pub(in crate::mir::opt::v_opt) fn match_2d_col_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<VectorPlan> {
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

pub(in crate::mir::opt::v_opt) fn match_3d_axis_map(
    fn_ir: &FnIR,
    lp: &LoopInfo,
) -> Option<VectorPlan> {
    let iv = lp.iv.as_ref()?;
    let iv_phi = iv.phi_val;
    if iv.step != 1 || iv.step_op != BinOp::Add {
        return None;
    }
    let start = iv.init_val;
    let end = lp.limit?;

    if !loop_is_simple_3d_map(fn_ir, lp) {
        return None;
    }

    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            let (dest, i, j, k, rhs) = match instr {
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => (*base, *i, *j, *k, *val),
                _ => continue,
            };
            let Some((axis, fixed_a, fixed_b)) = classify_3d_map_axis(fn_ir, dest, i, j, k, iv_phi)
            else {
                continue;
            };

            let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[rhs].kind.clone() else {
                continue;
            };
            if !matches!(
                op,
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
            ) {
                continue;
            }
            let lhs_src = match axis3_operand_source(fn_ir, lhs, axis, fixed_a, fixed_b, iv_phi) {
                Some(v) => v,
                None => continue,
            };
            let rhs_src = match axis3_operand_source(fn_ir, rhs, axis, fixed_a, fixed_b, iv_phi) {
                Some(v) => v,
                None => continue,
            };

            return Some(VectorPlan::Map3D {
                dest: canonical_value(fn_ir, dest),
                axis,
                fixed_a,
                fixed_b,
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

pub(in crate::mir::opt::v_opt) fn loop_is_simple_2d_map(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let mut store2d_count = 0usize;
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::StoreIndex2D { .. } => store2d_count += 1,
                Instr::StoreIndex1D { .. }
                | Instr::StoreIndex3D { .. }
                | Instr::Eval { .. }
                | Instr::UnsafeRBlock { .. } => {
                    return false;
                }
                Instr::Assign { .. } => {}
            }
        }
    }
    if store2d_count != 1 {
        return false;
    }
    true
}

pub(in crate::mir::opt::v_opt) fn loop_is_simple_3d_map(fn_ir: &FnIR, lp: &LoopInfo) -> bool {
    let mut store3d_count = 0usize;
    for &bid in &lp.body {
        for instr in &fn_ir.blocks[bid].instrs {
            match instr {
                Instr::StoreIndex3D { .. } => store3d_count += 1,
                Instr::StoreIndex1D { .. }
                | Instr::StoreIndex2D { .. }
                | Instr::Eval { .. }
                | Instr::UnsafeRBlock { .. } => {
                    return false;
                }
                Instr::Assign { .. } => {}
            }
        }
    }
    store3d_count == 1
}

pub(in crate::mir::opt::v_opt) fn classify_3d_map_axis(
    fn_ir: &FnIR,
    dest: ValueId,
    i: ValueId,
    j: ValueId,
    k: ValueId,
    iv_phi: ValueId,
) -> Option<(Axis3D, ValueId, ValueId)> {
    if is_iv_equivalent(fn_ir, i, iv_phi)
        && is_loop_invariant_axis(fn_ir, j, iv_phi, dest)
        && is_loop_invariant_axis(fn_ir, k, iv_phi, dest)
    {
        return Some((Axis3D::Dim1, j, k));
    }
    if is_iv_equivalent(fn_ir, j, iv_phi)
        && is_loop_invariant_axis(fn_ir, i, iv_phi, dest)
        && is_loop_invariant_axis(fn_ir, k, iv_phi, dest)
    {
        return Some((Axis3D::Dim2, i, k));
    }
    if is_iv_equivalent(fn_ir, k, iv_phi)
        && is_loop_invariant_axis(fn_ir, i, iv_phi, dest)
        && is_loop_invariant_axis(fn_ir, j, iv_phi, dest)
    {
        return Some((Axis3D::Dim3, i, j));
    }
    None
}

pub(in crate::mir::opt::v_opt) fn classify_3d_vector_access_axis(
    fn_ir: &FnIR,
    base: ValueId,
    i: ValueId,
    j: ValueId,
    k: ValueId,
    iv_phi: ValueId,
) -> Option<(Axis3D, ValueId, ValueId, ValueId)> {
    let dep_i = is_iv_equivalent(fn_ir, i, iv_phi) || expr_has_iv_dependency(fn_ir, i, iv_phi);
    let dep_j = is_iv_equivalent(fn_ir, j, iv_phi) || expr_has_iv_dependency(fn_ir, j, iv_phi);
    let dep_k = is_iv_equivalent(fn_ir, k, iv_phi) || expr_has_iv_dependency(fn_ir, k, iv_phi);

    if dep_i
        && !dep_j
        && !dep_k
        && is_loop_invariant_axis(fn_ir, j, iv_phi, base)
        && is_loop_invariant_axis(fn_ir, k, iv_phi, base)
    {
        return Some((Axis3D::Dim1, i, j, k));
    }
    if dep_j
        && !dep_i
        && !dep_k
        && is_loop_invariant_axis(fn_ir, i, iv_phi, base)
        && is_loop_invariant_axis(fn_ir, k, iv_phi, base)
    {
        return Some((Axis3D::Dim2, j, i, k));
    }
    if dep_k
        && !dep_i
        && !dep_j
        && is_loop_invariant_axis(fn_ir, i, iv_phi, base)
        && is_loop_invariant_axis(fn_ir, j, iv_phi, base)
    {
        return Some((Axis3D::Dim3, k, i, j));
    }
    None
}

pub(in crate::mir::opt::v_opt) fn classify_3d_general_vector_access(
    fn_ir: &FnIR,
    base: ValueId,
    i: ValueId,
    j: ValueId,
    k: ValueId,
    iv_phi: ValueId,
) -> Option<VectorAccessPattern3D> {
    fn classify_operand(
        fn_ir: &FnIR,
        base: ValueId,
        operand: ValueId,
        iv_phi: ValueId,
    ) -> Option<VectorAccessOperand3D> {
        if is_iv_equivalent(fn_ir, operand, iv_phi)
            || expr_has_iv_dependency(fn_ir, operand, iv_phi)
        {
            return Some(VectorAccessOperand3D::Vector(operand));
        }
        if is_loop_invariant_axis(fn_ir, operand, iv_phi, base)
            || is_loop_invariant_scalar_expr(fn_ir, operand, iv_phi, &FxHashSet::default())
        {
            return Some(VectorAccessOperand3D::Scalar(operand));
        }
        None
    }

    let pattern = VectorAccessPattern3D {
        i: classify_operand(fn_ir, base, i, iv_phi)?,
        j: classify_operand(fn_ir, base, j, iv_phi)?,
        k: classify_operand(fn_ir, base, k, iv_phi)?,
    };
    (pattern.vector_count() >= 1).then_some(pattern)
}

pub(in crate::mir::opt::v_opt) fn row_operand_source(
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
        ValueKind::Index3D { .. } => None,
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => Some(operand),
        _ => None,
    }
}

pub(in crate::mir::opt::v_opt) fn axis3_operand_source(
    fn_ir: &FnIR,
    operand: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    match &fn_ir.values[operand].kind {
        ValueKind::Index3D { base, i, j, k } => {
            let matches = match axis {
                Axis3D::Dim1 => {
                    is_iv_equivalent(fn_ir, *i, iv_phi)
                        && same_loop_invariant_value(fn_ir, *j, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *k, fixed_b, iv_phi)
                }
                Axis3D::Dim2 => {
                    is_iv_equivalent(fn_ir, *j, iv_phi)
                        && same_loop_invariant_value(fn_ir, *i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *k, fixed_b, iv_phi)
                }
                Axis3D::Dim3 => {
                    is_iv_equivalent(fn_ir, *k, iv_phi)
                        && same_loop_invariant_value(fn_ir, *i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *j, fixed_b, iv_phi)
                }
            };
            if matches {
                Some(canonical_value(fn_ir, *base))
            } else {
                None
            }
        }
        _ if is_loop_invariant_scalar_expr(fn_ir, operand, iv_phi, &FxHashSet::default()) => {
            Some(operand)
        }
        _ => None,
    }
}

pub(in crate::mir::opt::v_opt) fn axis3_vector_operand_source(
    fn_ir: &FnIR,
    operand: ValueId,
    axis: Axis3D,
    fixed_a: ValueId,
    fixed_b: ValueId,
    iv_phi: ValueId,
) -> Option<ValueId> {
    match &fn_ir.values[operand].kind {
        ValueKind::Index3D { base, i, j, k } => {
            let matches = match axis {
                Axis3D::Dim1 => {
                    is_iv_equivalent(fn_ir, *i, iv_phi)
                        && same_loop_invariant_value(fn_ir, *j, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *k, fixed_b, iv_phi)
                }
                Axis3D::Dim2 => {
                    is_iv_equivalent(fn_ir, *j, iv_phi)
                        && same_loop_invariant_value(fn_ir, *i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *k, fixed_b, iv_phi)
                }
                Axis3D::Dim3 => {
                    is_iv_equivalent(fn_ir, *k, iv_phi)
                        && same_loop_invariant_value(fn_ir, *i, fixed_a, iv_phi)
                        && same_loop_invariant_value(fn_ir, *j, fixed_b, iv_phi)
                }
            };
            matches.then_some(canonical_value(fn_ir, *base))
        }
        _ => None,
    }
}

pub(in crate::mir::opt::v_opt) fn expr_contains_index3d(fn_ir: &FnIR, root: ValueId) -> bool {
    let mut stack = vec![root];
    let mut seen = FxHashSet::default();
    while let Some(value) = stack.pop() {
        let value = canonical_value(fn_ir, value);
        if !seen.insert(value) {
            continue;
        }
        let Some(row) = fn_ir.values.get(value) else {
            continue;
        };
        if matches!(row.kind, ValueKind::Index3D { .. }) {
            return true;
        }
        for dep in value_dependencies(&row.kind) {
            stack.push(dep);
        }
    }
    false
}

pub(in crate::mir::opt::v_opt) fn col_operand_source(
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
        ValueKind::Index3D { .. } => None,
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => Some(operand),
        _ => None,
    }
}
