fn build_map_plan(fn_ir: &FnIR, lp: &LoopInfo, scop: &ScopRegion) -> Option<VectorPlan> {
    if scop.dimensions.len() != 1 {
        if super::poly_trace_enabled() {
            eprintln!("   [poly-codegen] map reject: expected one dimension");
        }
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) => {
                Some((stmt, *base, subscripts.clone(), expr_root))
            }
            _ => None,
        });
    let (stmt, dest, subscripts, expr_root) = stores.next()?;
    if stores.next().is_some() {
        if super::poly_trace_enabled() {
            eprintln!("   [poly-codegen] map reject: multiple stores in SCoP");
        }
        return None;
    }
    let rank = subscripts.len();
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == rank
            && is_loop_iv_subscript(scop, &access.subscripts[0])
    })?;
    let _ = write;
    if rank == 1 && !loop_covers_whole_vector(fn_ir, lp, scop, dest) {
        if super::poly_trace_enabled() {
            eprintln!("   [poly-codegen] map reject: loop does not cover whole destination");
        }
        return None;
    }

    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] map reject: rhs is not binary: {:?}",
                fn_ir.values[expr_root].kind
            );
        }
        return None;
    };
    if !matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
    ) {
        if super::poly_trace_enabled() {
            eprintln!("   [poly-codegen] map reject: unsupported op {:?}", op);
        }
        return None;
    }

    if rank == 1 {
        let lhs_vec = index_reads_loop_vector(fn_ir, scop, lhs);
        let rhs_vec = index_reads_loop_vector(fn_ir, scop, rhs);

        if let (Some(lbase), Some(rbase)) = (lhs_vec, rhs_vec) {
            return Some(VectorPlan::Map {
                dest,
                src: lbase,
                op,
                other: rbase,
                shadow_vars: Vec::new(),
            });
        }
        if let Some(lbase) = lhs_vec
            && is_scalarish_value(fn_ir, rhs)
        {
            return Some(VectorPlan::Map {
                dest,
                src: lbase,
                op,
                other: rhs,
                shadow_vars: Vec::new(),
            });
        }
        if let Some(rbase) = rhs_vec
            && is_scalarish_value(fn_ir, lhs)
        {
            return Some(VectorPlan::Map {
                dest,
                src: lhs,
                op,
                other: rbase,
                shadow_vars: Vec::new(),
            });
        }
    } else if rank == 2 && subscripts.len() == 2 && is_loop_iv_subscript(scop, &write.subscripts[0])
    {
        let fixed_col = subscripts[1];
        let lhs_vec = index_reads_2d_col_vector(fn_ir, scop, lhs, fixed_col);
        let rhs_vec = index_reads_2d_col_vector(fn_ir, scop, rhs, fixed_col);

        if let (Some(lbase), Some(rbase)) = (lhs_vec, rhs_vec) {
            return Some(VectorPlan::Map2DCol {
                dest,
                col: fixed_col,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lbase,
                rhs_src: rbase,
                op,
            });
        }
        if let Some(lbase) = lhs_vec
            && is_scalarish_value(fn_ir, rhs)
        {
            return Some(VectorPlan::Map2DCol {
                dest,
                col: fixed_col,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lbase,
                rhs_src: rhs,
                op,
            });
        }
        if let Some(rbase) = rhs_vec
            && is_scalarish_value(fn_ir, lhs)
        {
            return Some(VectorPlan::Map2DCol {
                dest,
                col: fixed_col,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lhs,
                rhs_src: rbase,
                op,
            });
        }
    } else if rank == 3 && subscripts.len() == 3 && is_loop_iv_subscript(scop, &write.subscripts[0])
    {
        let fixed_a = subscripts[1];
        let fixed_b = subscripts[2];
        let lhs_vec = index_reads_3d_dim1_vector(fn_ir, scop, lhs, fixed_a, fixed_b);
        let rhs_vec = index_reads_3d_dim1_vector(fn_ir, scop, rhs, fixed_a, fixed_b);

        if let (Some(lbase), Some(rbase)) = (lhs_vec, rhs_vec) {
            return Some(VectorPlan::Map3D {
                dest,
                axis: Axis3D::Dim1,
                fixed_a,
                fixed_b,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lbase,
                rhs_src: rbase,
                op,
            });
        }
        if let Some(lbase) = lhs_vec
            && is_scalarish_value(fn_ir, rhs)
        {
            return Some(VectorPlan::Map3D {
                dest,
                axis: Axis3D::Dim1,
                fixed_a,
                fixed_b,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lbase,
                rhs_src: rhs,
                op,
            });
        }
        if let Some(rbase) = rhs_vec
            && is_scalarish_value(fn_ir, lhs)
        {
            return Some(VectorPlan::Map3D {
                dest,
                axis: Axis3D::Dim1,
                fixed_a,
                fixed_b,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lhs,
                rhs_src: rbase,
                op,
            });
        }
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-codegen] map reject: rank={} lhs={:?} rhs={:?}",
            rank, fn_ir.values[lhs].kind, fn_ir.values[rhs].kind,
        );
    }
    None
}

fn build_whole_vector_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    dest: ValueId,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, VectorMapOperands)> {
    let dest_var = base_symbol_name(fn_ir, dest);
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
    let lhs_vec = index_reads_loop_vector(fn_ir, scop, lhs);
    let rhs_vec = index_reads_loop_vector(fn_ir, scop, rhs);
    let (out_val, lhs_src, rhs_src) = if let (Some(lbase), Some(rbase)) = (lhs_vec, rhs_vec) {
        (
            fn_ir.add_value(
                ValueKind::Binary {
                    op,
                    lhs: lbase,
                    rhs: rbase,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            lbase,
            rbase,
        )
    } else if let Some(lbase) = lhs_vec {
        if !is_scalarish_value(fn_ir, rhs) {
            return None;
        }
        (
            fn_ir.add_value(
                ValueKind::Binary {
                    op,
                    lhs: lbase,
                    rhs,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            lbase,
            rhs,
        )
    } else if let Some(rbase) = rhs_vec {
        if !is_scalarish_value(fn_ir, lhs) {
            return None;
        }
        (
            fn_ir.add_value(
                ValueKind::Binary {
                    op,
                    lhs,
                    rhs: rbase,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            lhs,
            rbase,
        )
    } else {
        return None;
    };
    Some((
        PreparedVectorAssignment {
            dest_var,
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        VectorMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

fn build_multi_whole_vector_map_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<VectorMapOperands>)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    let mut reference_dest: Option<ValueId> = None;
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 1 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 1
                && is_loop_iv_subscript(scop, &access.subscripts[0])
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var.clone()) {
            return None;
        }
        let whole_dest = loop_covers_whole_vector(fn_ir, lp, scop, *base)
            || reference_dest.is_some_and(|reference| same_length_proven(fn_ir, *base, reference));
        if !whole_dest {
            return None;
        }
        let (assignment, operands) =
            build_whole_vector_map_assignment(fn_ir, scop, *base, expr_root)?;
        if reference_dest.is_none() {
            reference_dest = Some(*base);
        }
        assignments.push(assignment);
        guards.push(operands);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

fn build_single_whole_vector_map_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, VectorMapOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root))
                if subscripts.len() == 1 =>
            {
                Some((stmt, *base, subscripts.clone(), expr_root))
            }
            _ => None,
        });
    let (stmt, dest, _subscripts, expr_root) = stores.next()?;
    if stores.next().is_some() {
        return None;
    }
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == 1
            && is_loop_iv_subscript(scop, &access.subscripts[0])
    })?;
    let _ = write;
    if !loop_covers_whole_vector(fn_ir, lp, scop, dest) {
        return None;
    }
    build_whole_vector_map_assignment(fn_ir, scop, dest, expr_root)
}

fn build_single_range_vector_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, VectorMapOperands)> {
    if scop.dimensions.len() != 1 || scop.dimensions[0].step != 1 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root))
                if subscripts.len() == 1 =>
            {
                Some((stmt, *base, expr_root))
            }
            _ => None,
        });
    let (stmt, dest, expr_root) = stores.next()?;
    if stores.next().is_some() {
        return None;
    }
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == 1
            && is_loop_iv_subscript(scop, &access.subscripts[0])
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

    let lhs_src = if let Some(base) = index_reads_loop_vector(fn_ir, scop, lhs) {
        base
    } else if is_scalarish_value(fn_ir, lhs) {
        lhs
    } else {
        return None;
    };
    let rhs_src = if let Some(base) = index_reads_loop_vector(fn_ir, scop, rhs) {
        base
    } else if is_scalarish_value(fn_ir, rhs) {
        rhs
    } else {
        return None;
    };

    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let lhs_slice = prepare_partial_slice_value(fn_ir, dest, lhs_src, start, end);
    let rhs_slice = prepare_partial_slice_value(fn_ir, dest, rhs_src, start, end);
    let expr_vec = fn_ir.add_value(
        ValueKind::Binary {
            op,
            lhs: lhs_slice,
            rhs: rhs_slice,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = build_slice_assignment_value(fn_ir, dest, start, end, expr_vec);

    Some((
        PreparedVectorAssignment {
            dest_var: base_symbol_name(fn_ir, dest),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        VectorMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

fn build_multi_range_vector_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<VectorMapOperands>)> {
    if scop.dimensions.len() != 1 || scop.dimensions[0].step != 1 {
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
        if subscripts.len() != 1 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 1
                && is_loop_iv_subscript(scop, &access.subscripts[0])
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var.clone()) {
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

        let lhs_src = if let Some(base) = index_reads_loop_vector(fn_ir, scop, lhs) {
            base
        } else if is_scalarish_value(fn_ir, lhs) {
            lhs
        } else {
            return None;
        };
        let rhs_src = if let Some(base) = index_reads_loop_vector(fn_ir, scop, rhs) {
            base
        } else if is_scalarish_value(fn_ir, rhs) {
            rhs
        } else {
            return None;
        };

        let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
        let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
        let lhs_slice = prepare_partial_slice_value(fn_ir, *base, lhs_src, start, end);
        let rhs_slice = prepare_partial_slice_value(fn_ir, *base, rhs_src, start, end);
        let expr_vec = fn_ir.add_value(
            ValueKind::Binary {
                op,
                lhs: lhs_slice,
                rhs: rhs_slice,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let out_val = build_slice_assignment_value(fn_ir, *base, start, end, expr_vec);
        assignments.push(PreparedVectorAssignment {
            dest_var,
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        });
        guards.push(VectorMapOperands {
            dest: *base,
            lhs_src,
            rhs_src,
        });
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

fn build_whole_vector_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, VectorReduceOperands)> {
    if scop.dimensions.len() != 1 {
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
                index_reads_loop_vector(fn_ir, scop, *rhs)
            } else if rhs_self {
                index_reads_loop_vector(fn_ir, scop, *lhs)
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
                index_reads_loop_vector(fn_ir, scop, args[1])
            } else if rhs_self {
                index_reads_loop_vector(fn_ir, scop, args[0])
            } else {
                None
            }?;
            (kind, read_base)
        }
        _ => return None,
    };

    if scop.dimensions[0].step != 1 {
        return None;
    }

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
            callee: "rr_reduce_range".to_string(),
            args: vec![base, start, end, op_lit],
            names: vec![None, None, None, None],
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
        VectorReduceOperands { base, start, end },
    ))
}

fn build_single_whole_vector_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, VectorReduceOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut assignments =
        scop.statements
            .iter()
            .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
                (super::PolyStmtKind::Assign { dst }, Some(expr_root)) => {
                    build_whole_vector_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
                }
                _ => None,
            });
    let assignment = assignments.next()?;
    if assignments.next().is_some() {
        return None;
    }
    Some(assignment)
}

fn build_multi_whole_vector_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<VectorReduceOperands>)> {
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
            build_whole_vector_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
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
