use super::*;
pub(crate) fn build_2d_col_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    dest: ValueId,
    fixed_col: ValueId,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, MatrixMapOperands)> {
    if scop.dimensions.len() != 1 {
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
    let lhs_src = if let Some(base) = index_reads_2d_col_vector(fn_ir, scop, lhs, fixed_col) {
        base
    } else if is_scalarish_value(fn_ir, lhs) {
        lhs
    } else {
        return None;
    };
    let rhs_src = if let Some(base) = index_reads_2d_col_vector(fn_ir, scop, rhs, fixed_col) {
        base
    } else if is_scalarish_value(fn_ir, rhs) {
        rhs
    } else {
        return None;
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
    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_col_binop_assign".to_string(),
            args: vec![dest, lhs_src, rhs_src, fixed_col, start, end, op_lit],
            names: vec![None, None, None, None, None, None, None],
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
        MatrixMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

pub(crate) fn build_multi_2d_col_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<MatrixMapOperands>)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 2 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 2
                && is_loop_iv_subscript(scop, &access.subscripts[0])
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var) {
            return None;
        }
        let (assignment, operands) =
            build_2d_col_map_assignment(fn_ir, scop, *base, subscripts[1], expr_root)?;
        assignments.push(assignment);
        guards.push(operands);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

pub(crate) fn build_single_2d_col_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, MatrixMapOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root))
                if subscripts.len() == 2 =>
            {
                Some((stmt, *base, subscripts.clone(), expr_root))
            }
            _ => None,
        });
    let (stmt, dest, subscripts, expr_root) = stores.next()?;
    if stores.next().is_some() {
        return None;
    }
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == 2
            && is_loop_iv_subscript(scop, &access.subscripts[0])
    })?;
    let _ = write;
    build_2d_col_map_assignment(fn_ir, scop, dest, subscripts[1], expr_root)
}

pub(crate) fn build_3d_dim1_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    dest: ValueId,
    fixed_a: ValueId,
    fixed_b: ValueId,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, Array3MapOperands)> {
    if scop.dimensions.len() != 1 {
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
    let lhs_src = if let Some(base) = index_reads_3d_dim1_vector(fn_ir, scop, lhs, fixed_a, fixed_b)
    {
        base
    } else if is_scalarish_value(fn_ir, lhs) {
        lhs
    } else {
        return None;
    };
    let rhs_src = if let Some(base) = index_reads_3d_dim1_vector(fn_ir, scop, rhs, fixed_a, fixed_b)
    {
        base
    } else if is_scalarish_value(fn_ir, rhs) {
        rhs
    } else {
        return None;
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
    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_dim1_binop_assign".to_string(),
            args: vec![dest, lhs_src, rhs_src, fixed_a, fixed_b, start, end, op_lit],
            names: vec![None, None, None, None, None, None, None, None],
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

pub(crate) fn build_multi_3d_dim1_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<Array3MapOperands>)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
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
                && is_loop_iv_subscript(scop, &access.subscripts[0])
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var) {
            return None;
        }
        let (assignment, operands) = build_3d_dim1_map_assignment(
            fn_ir,
            scop,
            *base,
            subscripts[1],
            subscripts[2],
            expr_root,
        )?;
        assignments.push(assignment);
        guards.push(operands);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

pub(crate) fn build_single_3d_dim1_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, Array3MapOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root))
                if subscripts.len() == 3 =>
            {
                Some((stmt, *base, subscripts.clone(), expr_root))
            }
            _ => None,
        });
    let (stmt, dest, subscripts, expr_root) = stores.next()?;
    if stores.next().is_some() {
        return None;
    }
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == 3
            && is_loop_iv_subscript(scop, &access.subscripts[0])
    })?;
    let _ = write;
    build_3d_dim1_map_assignment(fn_ir, scop, dest, subscripts[1], subscripts[2], expr_root)
}

pub(crate) fn build_2d_col_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, MatrixColReduceOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let (kind, base, col) = match &fn_ir.values[expr_root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let kind = if *op == BinOp::Add {
                ReduceKind::Sum
            } else {
                ReduceKind::Prod
            };
            let lhs_self = is_same_named_value(fn_ir, *lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, *rhs, dst);
            let other = if lhs_self {
                *rhs
            } else if rhs_self {
                *lhs
            } else {
                return None;
            };
            let other_root = resolve_scop_local_source(fn_ir, scop, other);
            let ValueKind::Index2D { base, c, .. } = &fn_ir.values[other_root].kind else {
                return None;
            };
            index_reads_2d_col_vector(fn_ir, scop, other_root, *c)?;
            (kind, *base, *c)
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
            let other = if lhs_self {
                args[1]
            } else if rhs_self {
                args[0]
            } else {
                return None;
            };
            let other_root = resolve_scop_local_source(fn_ir, scop, other);
            let ValueKind::Index2D { base, c, .. } = &fn_ir.values[other_root].kind else {
                return None;
            };
            index_reads_2d_col_vector(fn_ir, scop, other_root, *c)?;
            (kind, *base, *c)
        }
        _ => return None,
    };
    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_col_reduce_range".to_string(),
            args: vec![base, col, start, end, op_lit],
            names: vec![None, None, None, None, None],
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
                    rhs: reduce_val,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            ReduceKind::Min | ReduceKind::Max => fn_ir.add_value(
                ValueKind::Call {
                    callee: reduction_op_symbol(kind).to_string(),
                    args: vec![seed, reduce_val],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
        }
    } else {
        reduce_val
    };
    Some((
        PreparedVectorAssignment {
            dest_var: dst.to_string(),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        MatrixColReduceOperands {
            base,
            col,
            start,
            end,
        },
    ))
}

pub(crate) fn build_multi_2d_col_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<MatrixColReduceOperands>)> {
    if scop.dimensions.len() != 1 {
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
            build_2d_col_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
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

pub(crate) fn build_3d_dim1_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, Array3Dim1ReduceOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let (kind, base, fixed_a, fixed_b) = match &fn_ir.values[expr_root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let kind = if *op == BinOp::Add {
                ReduceKind::Sum
            } else {
                ReduceKind::Prod
            };
            let lhs_self = is_same_named_value(fn_ir, *lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, *rhs, dst);
            let other = if lhs_self {
                *rhs
            } else if rhs_self {
                *lhs
            } else {
                return None;
            };
            let other_root = resolve_scop_local_source(fn_ir, scop, other);
            let ValueKind::Index3D { base, j, k, .. } = &fn_ir.values[other_root].kind else {
                return None;
            };
            index_reads_3d_dim1_vector(fn_ir, scop, other_root, *j, *k)?;
            (kind, *base, *j, *k)
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
            let other = if lhs_self {
                args[1]
            } else if rhs_self {
                args[0]
            } else {
                return None;
            };
            let other_root = resolve_scop_local_source(fn_ir, scop, other);
            let ValueKind::Index3D { base, j, k, .. } = &fn_ir.values[other_root].kind else {
                return None;
            };
            index_reads_3d_dim1_vector(fn_ir, scop, other_root, *j, *k)?;
            (kind, *base, *j, *k)
        }
        _ => return None,
    };
    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_dim1_reduce_range".to_string(),
            args: vec![base, fixed_a, fixed_b, start, end, op_lit],
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
                    rhs: reduce_val,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            ReduceKind::Min | ReduceKind::Max => fn_ir.add_value(
                ValueKind::Call {
                    callee: reduction_op_symbol(kind).to_string(),
                    args: vec![seed, reduce_val],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
        }
    } else {
        reduce_val
    };
    Some((
        PreparedVectorAssignment {
            dest_var: dst.to_string(),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        Array3Dim1ReduceOperands {
            base,
            fixed_a,
            fixed_b,
            start,
            end,
        },
    ))
}

pub(crate) fn build_multi_3d_dim1_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<Array3Dim1ReduceOperands>)> {
    if scop.dimensions.len() != 1 {
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
            build_3d_dim1_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
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

pub(crate) fn build_nested_2d_full_matrix_map_value(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(ValueId, String, MatrixMapOperands)> {
    if scop.dimensions.len() != 2 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root))
                if subscripts.len() == 2 =>
            {
                Some((stmt, *base, subscripts.clone(), expr_root))
            }
            _ => None,
        });
    let (stmt, dest, subscripts, expr_root) = stores.next()?;
    if stores.next().is_some() {
        return None;
    }
    let outer_name = &scop.dimensions[0].iv_name;
    let inner_name = &scop.dimensions[1].iv_name;
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == 2
            && matches!(
                access.subscripts[0].terms.iter().next(),
                Some((super::affine::AffineSymbol::LoopIv(name), coeff)) if name == outer_name && *coeff == 1
            )
            && matches!(
                access.subscripts[1].terms.iter().next(),
                Some((super::affine::AffineSymbol::LoopIv(name), coeff)) if name == inner_name && *coeff == 1
            )
    })?;
    let _ = write;
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
        ValueKind::Index2D { base, r, c }
            if is_same_scalar_value(fn_ir, *r, subscripts[0])
                && is_same_scalar_value(fn_ir, *c, subscripts[1]) =>
        {
            *base
        }
        _ if is_scalarish_value(fn_ir, lhs_root) => lhs_root,
        _ => return None,
    };
    let rhs_src = match &fn_ir.values[rhs_root].kind {
        ValueKind::Index2D { base, r, c }
            if is_same_scalar_value(fn_ir, *r, subscripts[0])
                && is_same_scalar_value(fn_ir, *c, subscripts[1]) =>
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
    let out_val = {
        let r_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
        let r_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
        let c_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound)?;
        let c_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound)?;
        let args = vec![
            dest, lhs_src, rhs_src, r_start, r_end, c_start, c_end, op_lit,
        ];
        fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_matrix_binop_assign".to_string(),
                args,
                names: vec![None, None, None, None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    };
    Some((
        out_val,
        base_symbol_name(fn_ir, dest),
        MatrixMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

pub(crate) fn build_nested_2d_full_matrix_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    dest: ValueId,
    subscripts: &[ValueId],
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, MatrixMapOperands)> {
    if scop.dimensions.len() != 2 || subscripts.len() != 2 {
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
        ValueKind::Index2D { base, r, c }
            if is_same_scalar_value(fn_ir, *r, subscripts[0])
                && is_same_scalar_value(fn_ir, *c, subscripts[1]) =>
        {
            *base
        }
        _ if is_scalarish_value(fn_ir, lhs_root) => lhs_root,
        _ => return None,
    };
    let rhs_src = match &fn_ir.values[rhs_root].kind {
        ValueKind::Index2D { base, r, c }
            if is_same_scalar_value(fn_ir, *r, subscripts[0])
                && is_same_scalar_value(fn_ir, *c, subscripts[1]) =>
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
    let r_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let r_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let c_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound)?;
    let c_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound)?;
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_matrix_binop_assign".to_string(),
            args: vec![
                dest, lhs_src, rhs_src, r_start, r_end, c_start, c_end, op_lit,
            ],
            names: vec![None, None, None, None, None, None, None, None],
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
        MatrixMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

pub(crate) fn build_multi_nested_2d_full_matrix_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<MatrixMapOperands>)> {
    if scop.dimensions.len() != 2 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    let outer_name = &scop.dimensions[0].iv_name;
    let inner_name = &scop.dimensions[1].iv_name;
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 2 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 2
                && matches!(
                    access.subscripts[0].terms.iter().next(),
                    Some((super::affine::AffineSymbol::LoopIv(name), coeff))
                        if name == outer_name && *coeff == 1
                )
                && matches!(
                    access.subscripts[1].terms.iter().next(),
                    Some((super::affine::AffineSymbol::LoopIv(name), coeff))
                        if name == inner_name && *coeff == 1
                )
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var.clone()) {
            return None;
        }
        let (assignment, operands) =
            build_nested_2d_full_matrix_map_assignment(fn_ir, scop, *base, subscripts, expr_root)?;
        assignments.push(assignment);
        guards.push(operands);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}
