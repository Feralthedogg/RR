use super::*;
use crate::mir::value_dependencies;
use rustc_hash::FxHashSet;
pub(crate) fn reduction_op_symbol(kind: ReduceKind) -> &'static str {
    match kind {
        ReduceKind::Sum => "sum",
        ReduceKind::Prod => "prod",
        ReduceKind::Min => "min",
        ReduceKind::Max => "max",
    }
}

pub(crate) fn is_same_named_value(fn_ir: &FnIR, value: ValueId, name: &str) -> bool {
    let mut stack = vec![value];
    let mut seen = FxHashSet::default();
    while let Some(value) = stack.pop() {
        if !seen.insert(value) {
            continue;
        }
        let Some(row) = fn_ir.values.get(value) else {
            continue;
        };
        if row.origin_var.as_deref() == Some(name) {
            return true;
        }
        match &row.kind {
            ValueKind::Load { var } if var == name => return true,
            ValueKind::Phi { args } => stack.extend(args.iter().map(|(arg, _)| *arg)),
            _ => {}
        }
    }
    false
}

pub(crate) fn index_reads_nested_2d_rect(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    value: ValueId,
) -> Option<ValueId> {
    if scop.dimensions.len() != 2 {
        return None;
    }
    let value = resolve_scop_local_source(fn_ir, scop, value);
    match &fn_ir.values[value].kind {
        ValueKind::Index2D { base, .. } => {
            let read = scop
                .statements
                .iter()
                .flat_map(|stmt| stmt.accesses.iter())
                .find(|access| {
                    matches!(access.kind, super::access::AccessKind::Read)
                        && access.memref.base == *base
                        && access.subscripts.len() == 2
                })?;
            Some(read.memref.base)
        }
        _ => None,
    }
}

pub(crate) fn seed_assignment_outside_loop(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    var: &str,
) -> Option<ValueId> {
    let mut seed = None;
    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        if lp.body.contains(&bid) {
            continue;
        }
        for instr in &block.instrs {
            let crate::mir::Instr::Assign { dst, src, .. } = instr else {
                continue;
            };
            if dst == var {
                seed = Some(*src);
            }
        }
    }
    seed
}

pub(crate) fn build_nested_2d_full_matrix_reduce_value(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, MatrixRectReduceOperands)> {
    let (mut assignments, mut guards) =
        build_multi_nested_2d_full_matrix_reduce_assignments(fn_ir, lp, scop)?;
    if assignments.len() != 1 {
        return None;
    }
    let assignment = assignments.pop()?;
    let guard = guards.pop()?;
    Some((assignment, guard))
}

pub(crate) fn build_nested_2d_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, MatrixRectReduceOperands)> {
    if scop.dimensions.len() != 2 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let (kind, base) = match &fn_ir.values[expr_root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let kind = if *op == BinOp::Add {
                ReduceKind::Sum
            } else {
                ReduceKind::Prod
            };
            let lhs_self = is_same_named_value(fn_ir, *lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, *rhs, dst);
            let read_base = if lhs_self {
                index_reads_nested_2d_rect(fn_ir, scop, *rhs)
            } else if rhs_self {
                index_reads_nested_2d_rect(fn_ir, scop, *lhs)
            } else {
                None
            }?;
            (kind, read_base)
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(expr_root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let kind = if matches!(
                fn_ir.call_semantics(expr_root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                ReduceKind::Min
            } else {
                ReduceKind::Max
            };
            let lhs_self = is_same_named_value(fn_ir, args[0], dst);
            let rhs_self = is_same_named_value(fn_ir, args[1], dst);
            let read_base = if lhs_self {
                index_reads_nested_2d_rect(fn_ir, scop, args[1])
            } else if rhs_self {
                index_reads_nested_2d_rect(fn_ir, scop, args[0])
            } else {
                None
            }?;
            (kind, read_base)
        }
        _ => return None,
    };
    let r_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let r_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let c_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound)?;
    let c_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let rect_reduce = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_matrix_reduce_rect".to_string(),
            args: vec![base, r_start, r_end, c_start, c_end, op_lit],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );

    let out_val = if let Some(seed) = seed_assignment_outside_loop(fn_ir, lp, dst) {
        match kind {
            ReduceKind::Sum | ReduceKind::Prod => fn_ir.add_value(
                ValueKind::Binary {
                    op: if kind == ReduceKind::Sum {
                        BinOp::Add
                    } else {
                        BinOp::Mul
                    },
                    lhs: seed,
                    rhs: rect_reduce,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            ReduceKind::Min | ReduceKind::Max => fn_ir.add_value(
                ValueKind::Call {
                    callee: reduction_op_symbol(kind).to_string(),
                    args: vec![seed, rect_reduce],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
        }
    } else {
        rect_reduce
    };

    Some((
        PreparedVectorAssignment {
            dest_var: dst.to_string(),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        MatrixRectReduceOperands {
            base,
            r_start,
            r_end,
            c_start,
            c_end,
        },
    ))
}

pub(crate) fn build_multi_nested_2d_full_matrix_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<MatrixRectReduceOperands>)> {
    if scop.dimensions.len() != 2 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        let Some((assignment, guard)) =
            build_nested_2d_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
        else {
            continue;
        };
        if !seen_dests.insert(assignment.dest_var.clone()) {
            return None;
        }
        assignments.push(assignment);
        guards.push(guard);
    }
    (!assignments.is_empty()).then_some((assignments, guards))
}

pub(crate) fn index_reads_nested_3d_cube(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    value: ValueId,
) -> Option<ValueId> {
    if scop.dimensions.len() != 3 {
        return None;
    }
    let value = resolve_scop_local_source(fn_ir, scop, value);
    match &fn_ir.values[value].kind {
        ValueKind::Index3D { base, .. } => {
            let read = scop
                .statements
                .iter()
                .flat_map(|stmt| stmt.accesses.iter())
                .find(|access| {
                    matches!(access.kind, super::access::AccessKind::Read)
                        && access.memref.base == *base
                        && access.subscripts.len() == 3
                })?;
            Some(read.memref.base)
        }
        _ => None,
    }
}

pub(crate) fn build_nested_3d_full_cube_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    dest: ValueId,
    subscripts: &[ValueId],
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, Array3MapOperands)> {
    if scop.dimensions.len() != 3 || subscripts.len() != 3 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
        return None;
    };
    if !matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
    ) {
        return None;
    }
    let lhs_root = resolve_scop_local_source(fn_ir, scop, lhs);
    let rhs_root = resolve_scop_local_source(fn_ir, scop, rhs);
    let lhs_src = match &fn_ir.values[lhs_root].kind {
        ValueKind::Index3D { base, i, j, k }
            if is_same_scalar_value(fn_ir, *i, subscripts[0])
                && is_same_scalar_value(fn_ir, *j, subscripts[1])
                && is_same_scalar_value(fn_ir, *k, subscripts[2]) =>
        {
            *base
        }
        _ if is_scalarish_value(fn_ir, lhs_root) => lhs_root,
        _ => return None,
    };
    let rhs_src = match &fn_ir.values[rhs_root].kind {
        ValueKind::Index3D { base, i, j, k }
            if is_same_scalar_value(fn_ir, *i, subscripts[0])
                && is_same_scalar_value(fn_ir, *j, subscripts[1])
                && is_same_scalar_value(fn_ir, *k, subscripts[2]) =>
        {
            *base
        }
        _ if is_scalarish_value(fn_ir, rhs_root) => rhs_root,
        _ => return None,
    };
    let op_sym = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%%",
        _ => return None,
    };
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(op_sym.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let i_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let i_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let j_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound)?;
    let j_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound)?;
    let k_start = encode_bound(fn_ir, &scop.dimensions[2].lower_bound)?;
    let k_end = encode_bound(fn_ir, &scop.dimensions[2].upper_bound)?;
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_array3_binop_cube_assign".to_string(),
            args: vec![
                dest, lhs_src, rhs_src, i_start, i_end, j_start, j_end, k_start, k_end, op_lit,
            ],
            names: vec![None, None, None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some((
        PreparedVectorAssignment {
            dest_var: base_symbol_name(fn_ir, dest),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        Array3MapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

pub(crate) fn build_multi_nested_3d_full_cube_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<Array3MapOperands>)> {
    if scop.dimensions.len() != 3 {
        return None;
    }
    let i_name = &scop.dimensions[0].iv_name;
    let j_name = &scop.dimensions[1].iv_name;
    let k_name = &scop.dimensions[2].iv_name;
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 3 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 3
                && is_named_loop_iv_subscript(i_name, &access.subscripts[0])
                && is_named_loop_iv_subscript(j_name, &access.subscripts[1])
                && is_named_loop_iv_subscript(k_name, &access.subscripts[2])
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var) {
            return None;
        }
        let (assignment, operands) =
            build_nested_3d_full_cube_map_assignment(fn_ir, scop, *base, subscripts, expr_root)?;
        assignments.push(assignment);
        guards.push(operands);
    }
    (!assignments.is_empty()).then_some((assignments, guards))
}

pub(crate) fn build_single_nested_3d_full_cube_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, Array3MapOperands)> {
    let (mut assignments, mut guards) =
        build_multi_nested_3d_full_cube_map_assignments(fn_ir, scop)?;
    if assignments.len() != 1 {
        return None;
    }
    Some((assignments.pop()?, guards.pop()?))
}

pub(crate) fn build_nested_3d_full_cube_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, Array3CubeReduceOperands)> {
    if scop.dimensions.len() != 3 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let (kind, base) = match &fn_ir.values[expr_root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let kind = if *op == BinOp::Add {
                ReduceKind::Sum
            } else {
                ReduceKind::Prod
            };
            let lhs_self = is_same_named_value(fn_ir, *lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, *rhs, dst);
            let read_base = if lhs_self {
                index_reads_nested_3d_cube(fn_ir, scop, *rhs)
            } else if rhs_self {
                index_reads_nested_3d_cube(fn_ir, scop, *lhs)
            } else {
                None
            }?;
            (kind, read_base)
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(expr_root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let kind = if matches!(
                fn_ir.call_semantics(expr_root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                ReduceKind::Min
            } else {
                ReduceKind::Max
            };
            let lhs_self = is_same_named_value(fn_ir, args[0], dst);
            let rhs_self = is_same_named_value(fn_ir, args[1], dst);
            let read_base = if lhs_self {
                index_reads_nested_3d_cube(fn_ir, scop, args[1])
            } else if rhs_self {
                index_reads_nested_3d_cube(fn_ir, scop, args[0])
            } else {
                None
            }?;
            (kind, read_base)
        }
        _ => return None,
    };
    let i_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let i_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let j_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound)?;
    let j_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound)?;
    let k_start = encode_bound(fn_ir, &scop.dimensions[2].lower_bound)?;
    let k_end = encode_bound(fn_ir, &scop.dimensions[2].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let cube_reduce = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_array3_reduce_cube".to_string(),
            args: vec![base, i_start, i_end, j_start, j_end, k_start, k_end, op_lit],
            names: vec![None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );

    let out_val = if let Some(seed) = seed_assignment_outside_loop(fn_ir, lp, dst) {
        match kind {
            ReduceKind::Sum | ReduceKind::Prod => fn_ir.add_value(
                ValueKind::Binary {
                    op: if kind == ReduceKind::Sum {
                        BinOp::Add
                    } else {
                        BinOp::Mul
                    },
                    lhs: seed,
                    rhs: cube_reduce,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            ReduceKind::Min | ReduceKind::Max => fn_ir.add_value(
                ValueKind::Call {
                    callee: reduction_op_symbol(kind).to_string(),
                    args: vec![seed, cube_reduce],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
        }
    } else {
        cube_reduce
    };

    Some((
        PreparedVectorAssignment {
            dest_var: dst.to_string(),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        Array3CubeReduceOperands {
            base,
            i_start,
            i_end,
            j_start,
            j_end,
            k_start,
            k_end,
        },
    ))
}

pub(crate) fn build_multi_nested_3d_full_cube_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<Array3CubeReduceOperands>)> {
    if scop.dimensions.len() != 3 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        let Some((assignment, guard)) =
            build_nested_3d_full_cube_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
        else {
            continue;
        };
        if !seen_dests.insert(assignment.dest_var.clone()) {
            return None;
        }
        assignments.push(assignment);
        guards.push(guard);
    }
    (!assignments.is_empty()).then_some((assignments, guards))
}

pub(crate) fn build_single_nested_3d_full_cube_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, Array3CubeReduceOperands)> {
    let (mut assignments, mut guards) =
        build_multi_nested_3d_full_cube_reduce_assignments(fn_ir, lp, scop)?;
    if assignments.len() != 1 {
        return None;
    }
    Some((assignments.pop()?, guards.pop()?))
}

pub(crate) fn expr_contains_loop_read(fn_ir: &FnIR, scop: &ScopRegion, root: ValueId) -> bool {
    let mut stack = vec![root];
    let mut seen = FxHashSet::default();
    while let Some(value) = stack.pop() {
        let value = resolve_scop_local_source(fn_ir, scop, value);
        if !seen.insert(value) {
            continue;
        }
        let Some(row) = fn_ir.values.get(value) else {
            continue;
        };
        match &row.kind {
            ValueKind::Index1D { .. } if index_reads_loop_vector(fn_ir, scop, value).is_some() => {
                return true;
            }
            ValueKind::Index2D { c, .. }
                if index_reads_2d_col_vector(fn_ir, scop, value, *c).is_some() =>
            {
                return true;
            }
            ValueKind::Index3D { j, k, .. }
                if index_reads_3d_dim1_vector(fn_ir, scop, value, *j, *k).is_some() =>
            {
                return true;
            }
            _ => {
                stack.extend(value_dependencies(&row.kind));
            }
        }
    }
    false
}

pub(crate) fn build_reduce_plan(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<VectorPlan> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let iv_phi = lp.iv.as_ref()?.phi_val;

    for (id, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if value.phi_block != Some(lp.header) || args.len() != 2 {
            continue;
        }
        let Some((next_val, _)) = args.iter().find(|(_, pred)| *pred == lp.latch) else {
            continue;
        };
        let next = resolve_scop_local_source(fn_ir, scop, *next_val);

        match &fn_ir.values[next].kind {
            ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
                let other = if *lhs == id {
                    *rhs
                } else if *rhs == id {
                    *lhs
                } else {
                    continue;
                };
                let kind = if *op == BinOp::Add {
                    ReduceKind::Sum
                } else {
                    ReduceKind::Prod
                };
                let other_root = resolve_scop_local_source(fn_ir, scop, other);
                if let ValueKind::Index2D { base, c, .. } = &fn_ir.values[other_root].kind
                    && kind == ReduceKind::Sum
                    && index_reads_2d_col_vector(fn_ir, scop, other_root, *c).is_some()
                {
                    return Some(VectorPlan::Reduce2DColSum {
                        acc_phi: id,
                        base: *base,
                        col: *c,
                        start: lp.iv.as_ref()?.init_val,
                        end: lp.limit?,
                    });
                }
                if let ValueKind::Index3D { base, j, k, .. } = &fn_ir.values[other_root].kind
                    && index_reads_3d_dim1_vector(fn_ir, scop, other_root, *j, *k).is_some()
                {
                    return Some(VectorPlan::Reduce3D {
                        kind,
                        acc_phi: id,
                        base: *base,
                        axis: Axis3D::Dim1,
                        fixed_a: *j,
                        fixed_b: *k,
                        start: lp.iv.as_ref()?.init_val,
                        end: lp.limit?,
                    });
                }
                if !expr_contains_loop_read(fn_ir, scop, other) {
                    continue;
                }
                return Some(VectorPlan::Reduce {
                    kind,
                    acc_phi: id,
                    vec_expr: other,
                    iv_phi,
                });
            }
            ValueKind::Call { callee, args, .. }
                if args.len() == 2
                    && matches!(
                        fn_ir.call_semantics(next),
                        Some(crate::mir::CallSemantics::Builtin(
                            crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                        ))
                    )
                    || matches!(
                        callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                        "min" | "max"
                    ) && args.len() == 2 =>
            {
                let other = if args[0] == id {
                    args[1]
                } else if args[1] == id {
                    args[0]
                } else {
                    continue;
                };
                let kind = if matches!(
                    fn_ir.call_semantics(next),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min
                    ))
                ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
                {
                    ReduceKind::Min
                } else {
                    ReduceKind::Max
                };
                let other_root = resolve_scop_local_source(fn_ir, scop, other);
                if let ValueKind::Index3D { base, j, k, .. } = &fn_ir.values[other_root].kind
                    && index_reads_3d_dim1_vector(fn_ir, scop, other_root, *j, *k).is_some()
                {
                    return Some(VectorPlan::Reduce3D {
                        kind,
                        acc_phi: id,
                        base: *base,
                        axis: Axis3D::Dim1,
                        fixed_a: *j,
                        fixed_b: *k,
                        start: lp.iv.as_ref()?.init_val,
                        end: lp.limit?,
                    });
                }
                if !expr_contains_loop_read(fn_ir, scop, other) {
                    continue;
                }
                return Some(VectorPlan::Reduce {
                    kind,
                    acc_phi: id,
                    vec_expr: other,
                    iv_phi,
                });
            }
            _ => {}
        }
    }
    if super::poly_trace_enabled() {
        eprintln!("   [poly-codegen] reduce reject: no matching reduction pattern");
    }
    None
}

pub(crate) fn build_identity_plan(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<VectorPlan> {
    build_map_plan(fn_ir, lp, scop).or_else(|| build_reduce_plan(fn_ir, lp, scop))
}

pub(crate) fn replace_reduction_result_in_assignment(
    fn_ir: &mut FnIR,
    out_val: ValueId,
    new_reduce_val: ValueId,
) -> ValueId {
    match fn_ir.values[out_val].kind.clone() {
        ValueKind::Call { callee, .. } if callee == "rr_matrix_reduce_rect" => new_reduce_val,
        ValueKind::Call { callee, .. } if callee == "rr_reduce_range" => new_reduce_val,
        ValueKind::Call {
            callee,
            args,
            names,
        } if args.len() == 2 => fn_ir.add_value(
            ValueKind::Call {
                callee,
                args: vec![args[0], new_reduce_val],
                names,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ),
        ValueKind::Binary { op, lhs, .. } => fn_ir.add_value(
            ValueKind::Binary {
                op,
                lhs,
                rhs: new_reduce_val,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ),
        _ => new_reduce_val,
    }
}

pub(crate) fn combine_optional_same(lhs: Option<ValueId>, rhs: Option<ValueId>) -> Option<ValueId> {
    match (lhs, rhs) {
        (Some(a), Some(b)) if a == b => Some(a),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        _ => None,
    }
}

pub(crate) fn extract_reduction_op_literal(fn_ir: &FnIR, value: ValueId) -> Option<ValueId> {
    match &fn_ir.values[value].kind {
        ValueKind::Call { callee, args, .. }
            if matches!(
                callee.as_str(),
                "rr_reduce_range"
                    | "rr_col_reduce_range"
                    | "rr_matrix_reduce_rect"
                    | "rr_dim1_reduce_range"
                    | "rr_array3_reduce_cube"
                    | "rr_tile_reduce_range"
                    | "rr_tile_col_reduce_range"
                    | "rr_tile_matrix_reduce_rect"
                    | "rr_tile_dim1_reduce_range"
                    | "rr_tile_array3_reduce_cube"
            ) =>
        {
            args.last().copied()
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
            args.iter().fold(None, |acc, arg| {
                combine_optional_same(acc, extract_reduction_op_literal(fn_ir, *arg))
            })
        }
        ValueKind::Binary { lhs, rhs, .. } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *lhs),
            extract_reduction_op_literal(fn_ir, *rhs),
        ),
        ValueKind::Unary { rhs, .. } => extract_reduction_op_literal(fn_ir, *rhs),
        ValueKind::RecordLit { fields } => fields.iter().fold(None, |acc, (_, value)| {
            combine_optional_same(acc, extract_reduction_op_literal(fn_ir, *value))
        }),
        ValueKind::FieldGet { base, .. } => extract_reduction_op_literal(fn_ir, *base),
        ValueKind::FieldSet { base, value, .. } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *base),
            extract_reduction_op_literal(fn_ir, *value),
        ),
        ValueKind::Phi { args } => args.iter().fold(None, |acc, (arg, _)| {
            combine_optional_same(acc, extract_reduction_op_literal(fn_ir, *arg))
        }),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            extract_reduction_op_literal(fn_ir, *base)
        }
        ValueKind::Range { start, end } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *start),
            extract_reduction_op_literal(fn_ir, *end),
        ),
        ValueKind::Index1D { base, idx, .. } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *base),
            extract_reduction_op_literal(fn_ir, *idx),
        ),
        ValueKind::Index2D { base, r, c } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *base),
            combine_optional_same(
                extract_reduction_op_literal(fn_ir, *r),
                extract_reduction_op_literal(fn_ir, *c),
            ),
        ),
        ValueKind::Index3D { base, i, j, k } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *base),
            combine_optional_same(
                extract_reduction_op_literal(fn_ir, *i),
                combine_optional_same(
                    extract_reduction_op_literal(fn_ir, *j),
                    extract_reduction_op_literal(fn_ir, *k),
                ),
            ),
        ),
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => None,
    }
}
