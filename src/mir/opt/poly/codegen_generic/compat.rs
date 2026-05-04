use super::*;
pub(crate) fn scop_is_generic_map_compatible(fn_ir: &FnIR, scop: &ScopRegion) -> bool {
    let loop_iv_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<FxHashSet<_>>();
    scop.statements
        .iter()
        .all(|stmt| match (&stmt.kind, stmt.expr_root) {
            (PolyStmtKind::Assign { dst }, _) if loop_iv_names.contains(dst.as_str()) => true,
            (PolyStmtKind::Assign { .. }, _) if stmt_is_progress_assign(fn_ir, scop, stmt) => true,
            (PolyStmtKind::Assign { dst }, Some(expr_root)) => {
                !expr_mentions_var(fn_ir, expr_root, dst, &mut FxHashSet::default())
            }
            (PolyStmtKind::Store { .. }, Some(_)) => true,
            (PolyStmtKind::Eval, Some(_)) => true,
            _ => false,
        })
}

pub(crate) fn scop_is_generic_reduce_compatible(fn_ir: &FnIR, scop: &ScopRegion) -> bool {
    let loop_iv_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<FxHashSet<_>>();
    let mut reduction_stmt_count = 0usize;
    for stmt in &scop.statements {
        if stmt.accesses.is_empty() {
            continue;
        }
        match (&stmt.kind, stmt.expr_root) {
            (PolyStmtKind::Assign { dst }, _) if loop_iv_names.contains(dst.as_str()) => continue,
            (PolyStmtKind::Assign { .. }, _) if stmt_is_progress_assign(fn_ir, scop, stmt) => {
                continue;
            }
            (PolyStmtKind::Assign { dst }, Some(expr_root)) => {
                if classify_generic_reduce_kind(fn_ir, scop, stmt, dst, expr_root).is_none() {
                    return false;
                }
                reduction_stmt_count += 1;
            }
            _ => return false,
        }
    }
    reduction_stmt_count >= 1
}

pub(crate) fn scop_is_generic_nested_reduce_compatible(fn_ir: &FnIR, scop: &ScopRegion) -> bool {
    let loop_iv_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<FxHashSet<_>>();
    let mut reduction_stmt_count = 0usize;
    for stmt in &scop.statements {
        if stmt.accesses.is_empty() {
            continue;
        }
        match (&stmt.kind, stmt.expr_root) {
            (PolyStmtKind::Assign { dst }, _) if loop_iv_names.contains(dst.as_str()) => continue,
            (PolyStmtKind::Assign { .. }, _) if stmt_is_progress_assign(fn_ir, scop, stmt) => {
                continue;
            }
            (PolyStmtKind::Assign { dst }, Some(expr_root)) => {
                if classify_generic_nested_reduce_kind(fn_ir, scop, stmt, dst, expr_root).is_none()
                {
                    return false;
                }
                reduction_stmt_count += 1;
            }
            _ => return false,
        }
    }
    reduction_stmt_count >= 1
}

pub(crate) fn classify_generic_reduce_kind(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    stmt: &PolyStmt,
    dst: &str,
    expr_root: ValueId,
) -> Option<ReduceKind> {
    if !stmt_reads_dense_1d_loop_vector(stmt, scop) {
        return None;
    }
    let root = resolve_scop_local_source(fn_ir, scop, expr_root);
    match &fn_ir.values[root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let lhs = resolve_scop_local_source(fn_ir, scop, *lhs);
            let rhs = resolve_scop_local_source(fn_ir, scop, *rhs);
            let lhs_self = is_same_named_value(fn_ir, lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, rhs, dst);
            if lhs_self ^ rhs_self {
                Some(if *op == BinOp::Add {
                    ReduceKind::Sum
                } else {
                    ReduceKind::Prod
                })
            } else {
                None
            }
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let lhs = resolve_scop_local_source(fn_ir, scop, args[0]);
            let rhs = resolve_scop_local_source(fn_ir, scop, args[1]);
            let lhs_self = is_same_named_value(fn_ir, lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, rhs, dst);
            if !(lhs_self ^ rhs_self) {
                return None;
            }
            if matches!(
                fn_ir.call_semantics(root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                Some(ReduceKind::Min)
            } else {
                Some(ReduceKind::Max)
            }
        }
        _ => None,
    }
}

pub(crate) fn classify_generic_nested_reduce_kind(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    _stmt: &PolyStmt,
    dst: &str,
    expr_root: ValueId,
) -> Option<ReduceKind> {
    let root = resolve_scop_local_source(fn_ir, scop, expr_root);
    match &fn_ir.values[root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let lhs = resolve_scop_local_source(fn_ir, scop, *lhs);
            let rhs = resolve_scop_local_source(fn_ir, scop, *rhs);
            let lhs_self = is_same_named_value(fn_ir, lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, rhs, dst);
            if lhs_self ^ rhs_self {
                Some(if *op == BinOp::Add {
                    ReduceKind::Sum
                } else {
                    ReduceKind::Prod
                })
            } else {
                None
            }
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let lhs = resolve_scop_local_source(fn_ir, scop, args[0]);
            let rhs = resolve_scop_local_source(fn_ir, scop, args[1]);
            let lhs_self = is_same_named_value(fn_ir, lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, rhs, dst);
            if !(lhs_self ^ rhs_self) {
                return None;
            }
            if matches!(
                fn_ir.call_semantics(root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                Some(ReduceKind::Min)
            } else {
                Some(ReduceKind::Max)
            }
        }
        _ => None,
    }
}

pub(crate) fn stmt_reads_dense_1d_loop_vector(stmt: &PolyStmt, scop: &ScopRegion) -> bool {
    stmt.accesses.iter().any(|access| {
        matches!(access.kind, super::access::AccessKind::Read)
            && access_is_generic_single_dim_contiguous(access, scop)
    })
}

pub(crate) fn has_multiple_data_statements(scop: &ScopRegion) -> bool {
    scop.statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count()
        > 1
}

pub(crate) fn expr_mentions_any_loop_iv(scop: &ScopRegion, expr: &AffineExpr) -> bool {
    expr.terms.iter().any(|(symbol, coeff)| {
        *coeff != 0
            && matches!(
                symbol,
                AffineSymbol::LoopIv(name) if scop.dimensions.iter().any(|dim| dim.iv_name == *name)
            )
    })
}

pub(crate) fn access_is_generic_single_dim_contiguous(
    access: &super::access::AccessRelation,
    scop: &ScopRegion,
) -> bool {
    fn expr_is_unit_stride_for_loop(expr: &AffineExpr, scop: &ScopRegion) -> bool {
        let mut found = false;
        for (symbol, coeff) in &expr.terms {
            match symbol {
                AffineSymbol::LoopIv(name)
                    if scop.dimensions.iter().any(|dim| dim.iv_name == *name) =>
                {
                    if found || *coeff != 1 {
                        return false;
                    }
                    found = true;
                }
                AffineSymbol::LoopIv(_) => return false,
                _ => {}
            }
        }
        found
    }

    if scop.dimensions.len() != 1 {
        return false;
    }
    match access.memref.layout {
        MemoryLayout::Dense1D => {
            access.subscripts.len() == 1
                && expr_is_unit_stride_for_loop(&access.subscripts[0], scop)
        }
        MemoryLayout::ColumnMajor2D => {
            access.subscripts.len() == 2
                && expr_is_unit_stride_for_loop(&access.subscripts[0], scop)
                && !expr_mentions_any_loop_iv(scop, &access.subscripts[1])
        }
        MemoryLayout::ColumnMajor3D => {
            access.subscripts.len() == 3
                && expr_is_unit_stride_for_loop(&access.subscripts[0], scop)
                && !expr_mentions_any_loop_iv(scop, &access.subscripts[1])
                && !expr_mentions_any_loop_iv(scop, &access.subscripts[2])
        }
    }
}

pub(crate) fn is_loop_iv_subscript(scop: &ScopRegion, expr: &AffineExpr) -> bool {
    expr.constant == 0
        && matches!(
            expr.terms.iter().next(),
            Some((AffineSymbol::LoopIv(name), coeff))
                if expr.terms.len() == 1
                    && *coeff == 1
                    && scop.dimensions.iter().any(|dim| dim.iv_name == *name)
        )
}

pub(crate) fn resolve_scop_local_source(fn_ir: &FnIR, scop: &ScopRegion, root: ValueId) -> ValueId {
    let mut current = root;
    let mut seen = FxHashSet::default();
    loop {
        if !seen.insert(current) {
            return current;
        }
        let ValueKind::Load { var } = &fn_ir.values[current].kind else {
            return current;
        };
        let mut matches =
            scop.statements
                .iter()
                .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
                    (PolyStmtKind::Assign { dst }, Some(expr_root)) if dst == var => {
                        Some(expr_root)
                    }
                    _ => None,
                });
        let Some(next) = matches.next() else {
            return current;
        };
        if matches.next().is_some() {
            return current;
        }
        current = next;
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

pub(crate) fn expr_mentions_var(
    fn_ir: &FnIR,
    root: ValueId,
    var: &str,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    if !seen.insert(root) {
        return false;
    }
    if fn_ir.values[root].origin_var.as_deref() == Some(var) {
        return true;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Load { var: load_var } => load_var == var,
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_mentions_var(fn_ir, *lhs, var, seen) || expr_mentions_var(fn_ir, *rhs, var, seen)
        }
        ValueKind::Unary { rhs, .. } => expr_mentions_var(fn_ir, *rhs, var, seen),
        ValueKind::RecordLit { fields } => fields
            .iter()
            .any(|(_, value)| expr_mentions_var(fn_ir, *value, var, seen)),
        ValueKind::FieldGet { base, .. } => expr_mentions_var(fn_ir, *base, var, seen),
        ValueKind::FieldSet { base, value, .. } => {
            expr_mentions_var(fn_ir, *base, var, seen)
                || expr_mentions_var(fn_ir, *value, var, seen)
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| expr_mentions_var(fn_ir, *arg, var, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(arg, _)| expr_mentions_var(fn_ir, *arg, var, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_mentions_var(fn_ir, *base, var, seen)
        }
        ValueKind::Range { start, end } => {
            expr_mentions_var(fn_ir, *start, var, seen) || expr_mentions_var(fn_ir, *end, var, seen)
        }
        ValueKind::Index1D { base, idx, .. } => {
            expr_mentions_var(fn_ir, *base, var, seen) || expr_mentions_var(fn_ir, *idx, var, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            expr_mentions_var(fn_ir, *base, var, seen)
                || expr_mentions_var(fn_ir, *r, var, seen)
                || expr_mentions_var(fn_ir, *c, var, seen)
        }
        ValueKind::Index3D { base, i, j, k } => {
            expr_mentions_var(fn_ir, *base, var, seen)
                || expr_mentions_var(fn_ir, *i, var, seen)
                || expr_mentions_var(fn_ir, *j, var, seen)
                || expr_mentions_var(fn_ir, *k, var, seen)
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
    }
}
