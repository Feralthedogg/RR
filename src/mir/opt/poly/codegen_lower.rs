//! Tiled and transformed poly schedule lowering helpers.

use super::*;
use crate::mir::opt::poly::{PolyStmtKind, access::AccessKind, poly_trace_enabled};

fn build_tiled_vector_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(PreparedVectorAssignment, VectorMapOperands)> {
    if scop.dimensions.len() != 1 || scop.dimensions[0].step != 1 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (PolyStmtKind::Store { base, subscripts }, Some(expr_root))
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
        matches!(access.kind, AccessKind::Write)
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
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(match op {
            BinOp::Add => "+".to_string(),
            BinOp::Sub => "-".to_string(),
            BinOp::Mul => "*".to_string(),
            BinOp::Div => "/".to_string(),
            BinOp::Mod => "%%".to_string(),
            _ => return None,
        })),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let tile_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_tile_map_range".to_string(),
            args: vec![dest, lhs_src, rhs_src, start, end, op_lit, tile_lit],
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
        VectorMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

fn build_multi_tiled_vector_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<VectorMapOperands>)> {
    if scop.dimensions.len() != 1 || scop.dimensions[0].step != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 1 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, AccessKind::Write)
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
        let op_lit = fn_ir.add_value(
            ValueKind::Const(Lit::Str(match op {
                BinOp::Add => "+".to_string(),
                BinOp::Sub => "-".to_string(),
                BinOp::Mul => "*".to_string(),
                BinOp::Div => "/".to_string(),
                BinOp::Mod => "%%".to_string(),
                _ => return None,
            })),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_lit = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_size as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let out_val = fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_tile_map_range".to_string(),
                args: vec![*base, lhs_src, rhs_src, start, end, op_lit, tile_lit],
                names: vec![None, None, None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
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

fn build_tiled_vector_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(PreparedVectorAssignment, VectorReduceOperands)> {
    if scop.dimensions.len() != 1 || scop.dimensions[0].step != 1 {
        return None;
    }
    let mut assigns =
        scop.statements
            .iter()
            .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
                (PolyStmtKind::Assign { dst }, Some(expr_root)) if !stmt.accesses.is_empty() => {
                    Some((dst.as_str(), expr_root))
                }
                _ => None,
            });
    let (dst, expr_root) = assigns.next()?;
    if assigns.next().is_some() {
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

    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let tile_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_tile_reduce_range".to_string(),
            args: vec![base, start, end, op_lit, tile_lit],
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
        VectorReduceOperands { base, start, end },
    ))
}

fn build_tiled_2d_col_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(PreparedVectorAssignment, MatrixMapOperands)> {
    let (assignment, guard) = build_single_2d_col_map_assignment(fn_ir, scop)?;
    let ValueKind::Call { args, .. } = fn_ir.values[assignment.out_val].kind.clone() else {
        return None;
    };
    let fixed_col = args[3];
    let start = args[4];
    let end = args[5];
    let op_lit = args[6];
    let tile_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_tile_col_binop_assign".to_string(),
            args: vec![
                guard.dest,
                guard.lhs_src,
                guard.rhs_src,
                fixed_col,
                start,
                end,
                op_lit,
                tile_lit,
            ],
            names: vec![None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some((
        PreparedVectorAssignment {
            dest_var: assignment.dest_var,
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        guard,
    ))
}

fn build_multi_tiled_2d_col_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<MatrixMapOperands>)> {
    let (assignments, guards) = build_multi_2d_col_map_assignments(fn_ir, scop)?;
    let tile_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let mut tiled = Vec::new();
    for (assignment, guard) in assignments.into_iter().zip(guards.iter()) {
        let ValueKind::Call { args, .. } = fn_ir.values[assignment.out_val].kind.clone() else {
            return None;
        };
        let out_val = fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_tile_col_binop_assign".to_string(),
                args: vec![
                    guard.dest,
                    guard.lhs_src,
                    guard.rhs_src,
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    tile_lit,
                ],
                names: vec![None, None, None, None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        tiled.push(PreparedVectorAssignment {
            dest_var: assignment.dest_var,
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        });
    }
    Some((tiled, guards))
}

fn build_tiled_2d_col_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(PreparedVectorAssignment, MatrixColReduceOperands)> {
    let (assignment, guard) =
        build_2d_col_reduce_assignment_for_stmt(fn_ir, lp, scop, tile_size, None)?;
    Some((assignment, guard))
}

fn build_2d_col_reduce_assignment_for_stmt(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tile_size: usize,
    target: Option<&str>,
) -> Option<(PreparedVectorAssignment, MatrixColReduceOperands)> {
    let mut found = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (PolyStmtKind::Assign { dst }, Some(expr_root)) => {
                if target.is_none() || target == Some(dst.as_str()) {
                    build_2d_col_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
                } else {
                    None
                }
            }
            _ => None,
        });
    let (assignment, guard) = found.next()?;
    if target.is_none() && found.next().is_some() {
        return None;
    }
    let op_lit = match fn_ir.values[assignment.out_val].kind.clone() {
        ValueKind::Call { args, .. } => args.last().copied(),
        ValueKind::Binary { rhs, .. } => match fn_ir.values[rhs].kind.clone() {
            ValueKind::Call { args, .. } => args.last().copied(),
            _ => None,
        },
        _ => None,
    }?;
    let tile_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_tile_col_reduce_range".to_string(),
            args: vec![
                guard.base,
                guard.col,
                guard.start,
                guard.end,
                op_lit,
                tile_lit,
            ],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some((
        PreparedVectorAssignment {
            dest_var: assignment.dest_var,
            out_val: replace_reduction_result_in_assignment(fn_ir, assignment.out_val, reduce_val),
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        guard,
    ))
}

fn build_multi_tiled_2d_col_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<MatrixColReduceOperands>)> {
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let PolyStmtKind::Assign { dst } = &stmt.kind else {
            continue;
        };
        if !seen.insert(dst.clone()) {
            return None;
        }
        let Some((assignment, guard)) =
            build_2d_col_reduce_assignment_for_stmt(fn_ir, lp, scop, tile_size, Some(dst))
        else {
            continue;
        };
        assignments.push(assignment);
        guards.push(guard);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

fn build_tiled_3d_dim1_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(PreparedVectorAssignment, Array3MapOperands)> {
    let (assignment, guard) = build_single_3d_dim1_map_assignment(fn_ir, scop)?;
    let ValueKind::Call { args, .. } = fn_ir.values[assignment.out_val].kind.clone() else {
        return None;
    };
    let tile_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_tile_dim1_binop_assign".to_string(),
            args: vec![
                guard.dest,
                guard.lhs_src,
                guard.rhs_src,
                args[3],
                args[4],
                args[5],
                args[6],
                args[7],
                tile_lit,
            ],
            names: vec![None, None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some((
        PreparedVectorAssignment {
            dest_var: assignment.dest_var,
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        guard,
    ))
}

fn build_multi_tiled_3d_dim1_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<Array3MapOperands>)> {
    let (assignments, guards) = build_multi_3d_dim1_map_assignments(fn_ir, scop)?;
    let tile_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let mut tiled = Vec::new();
    for (assignment, guard) in assignments.into_iter().zip(guards.iter()) {
        let ValueKind::Call { args, .. } = fn_ir.values[assignment.out_val].kind.clone() else {
            return None;
        };
        let out_val = fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_tile_dim1_binop_assign".to_string(),
                args: vec![
                    guard.dest,
                    guard.lhs_src,
                    guard.rhs_src,
                    args[3],
                    args[4],
                    args[5],
                    args[6],
                    args[7],
                    tile_lit,
                ],
                names: vec![None, None, None, None, None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        tiled.push(PreparedVectorAssignment {
            dest_var: assignment.dest_var,
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        });
    }
    Some((tiled, guards))
}

fn build_tiled_3d_dim1_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(PreparedVectorAssignment, Array3Dim1ReduceOperands)> {
    let (assignment, guard) =
        build_3d_dim1_reduce_assignment_for_stmt(fn_ir, lp, scop, tile_size, None)?;
    Some((assignment, guard))
}

fn build_3d_dim1_reduce_assignment_for_stmt(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tile_size: usize,
    target: Option<&str>,
) -> Option<(PreparedVectorAssignment, Array3Dim1ReduceOperands)> {
    let mut found = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (PolyStmtKind::Assign { dst }, Some(expr_root)) => {
                if target.is_none() || target == Some(dst.as_str()) {
                    build_3d_dim1_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
                } else {
                    None
                }
            }
            _ => None,
        });
    let (assignment, guard) = found.next()?;
    if target.is_none() && found.next().is_some() {
        return None;
    }
    let op_lit = match fn_ir.values[assignment.out_val].kind.clone() {
        ValueKind::Call { args, .. } => args.last().copied(),
        ValueKind::Binary { rhs, .. } => match fn_ir.values[rhs].kind.clone() {
            ValueKind::Call { args, .. } => args.last().copied(),
            _ => None,
        },
        _ => None,
    }?;
    let tile_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_tile_dim1_reduce_range".to_string(),
            args: vec![
                guard.base,
                guard.fixed_a,
                guard.fixed_b,
                guard.start,
                guard.end,
                op_lit,
                tile_lit,
            ],
            names: vec![None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some((
        PreparedVectorAssignment {
            dest_var: assignment.dest_var,
            out_val: replace_reduction_result_in_assignment(fn_ir, assignment.out_val, reduce_val),
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        guard,
    ))
}

fn build_multi_tiled_3d_dim1_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<Array3Dim1ReduceOperands>)> {
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let PolyStmtKind::Assign { dst } = &stmt.kind else {
            continue;
        };
        if !seen.insert(dst.clone()) {
            return None;
        }
        let Some((assignment, guard)) =
            build_3d_dim1_reduce_assignment_for_stmt(fn_ir, lp, scop, tile_size, Some(dst))
        else {
            continue;
        };
        assignments.push(assignment);
        guards.push(guard);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

fn build_multi_tiled_vector_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tile_size: usize,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<VectorReduceOperands>)> {
    if scop.dimensions.len() != 1 || scop.dimensions[0].step != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root) else {
            continue;
        };
        if stmt.accesses.is_empty() {
            continue;
        }
        let Some((assignment, guard)) = build_tiled_vector_reduce_assignment_for_stmt(
            fn_ir, lp, scop, tile_size, dst, expr_root,
        ) else {
            continue;
        };
        if !seen_dests.insert(assignment.dest_var.clone()) {
            return None;
        }
        assignments.push(assignment);
        guards.push(guard);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

fn build_tiled_vector_reduce_assignment_for_stmt(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tile_size: usize,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, VectorReduceOperands)> {
    if scop.dimensions.len() != 1 || scop.dimensions[0].step != 1 {
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
    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let tile_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_tile_reduce_range".to_string(),
            args: vec![base, start, end, op_lit, tile_lit],
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
        VectorReduceOperands { base, start, end },
    ))
}

pub(crate) fn lower_interchange_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> PolyCodegenPlan {
    if schedule.kind != SchedulePlanKind::Interchange {
        return PolyCodegenPlan { emitted: false };
    }
    let generic_covers_map = generic_schedule_supports_map(fn_ir, scop, schedule);
    let generic_covers_reduce = generic_schedule_supports_reduce(fn_ir, scop, schedule);
    if lower_interchange_map_generic(fn_ir, lp, scop, schedule) {
        if poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-interchange-map",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if lower_interchange_reduce_generic(fn_ir, lp, scop, schedule) {
        if poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-interchange-reduce",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if generic_mir_effective_for_schedule(scop, schedule)
        && (generic_covers_map || generic_covers_reduce)
    {
        return PolyCodegenPlan { emitted: false };
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        return PolyCodegenPlan { emitted: false };
    };
    if let Some((assignments, guards)) =
        build_multi_nested_2d_full_matrix_map_assignments(fn_ir, scop)
    {
        let Some(cond) = build_matrix_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((out_val, dest_var, operands)) = build_nested_2d_full_matrix_map_value(fn_ir, scop)
    {
        let Some(cond) = build_matrix_map_guard_cond(fn_ir, &[operands]) else {
            return PolyCodegenPlan { emitted: false };
        };
        return PolyCodegenPlan {
            emitted: finish_vector_assignments_versioned(
                fn_ir,
                lp.header,
                site,
                vec![PreparedVectorAssignment {
                    dest_var,
                    out_val,
                    shadow_vars: Vec::new(),
                    shadow_idx: None,
                }],
                cond,
            ),
        };
    }
    if let Some((assignments, guards)) =
        build_multi_nested_2d_full_matrix_reduce_assignments(fn_ir, lp, scop)
        && assignments.len() >= 2
    {
        let Some(cond) = build_matrix_rect_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignment, guard)) = build_nested_2d_full_matrix_reduce_value(fn_ir, lp, scop) {
        let Some(cond) = build_matrix_rect_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignments, guards)) =
        build_multi_nested_3d_full_cube_map_assignments(fn_ir, scop)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignment, guard)) = build_single_nested_3d_full_cube_map_assignment(fn_ir, scop)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        return PolyCodegenPlan {
            emitted: finish_vector_assignments_versioned(
                fn_ir,
                lp.header,
                site,
                vec![assignment],
                cond,
            ),
        };
    }
    if let Some((assignments, guards)) =
        build_multi_nested_3d_full_cube_reduce_assignments(fn_ir, lp, scop)
        && assignments.len() >= 2
    {
        let Some(cond) = build_array3_cube_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignment, guard)) =
        build_single_nested_3d_full_cube_reduce_assignment(fn_ir, lp, scop)
    {
        let Some(cond) = build_array3_cube_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        return PolyCodegenPlan { emitted };
    }
    PolyCodegenPlan { emitted: false }
}

pub(crate) fn lower_skew2d_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> PolyCodegenPlan {
    if schedule.kind != SchedulePlanKind::Skew2D {
        return PolyCodegenPlan { emitted: false };
    }
    let generic_covers_map = generic_schedule_supports_map(fn_ir, scop, schedule);
    let generic_covers_reduce = generic_schedule_supports_reduce(fn_ir, scop, schedule);
    if lower_skew2d_map_generic(fn_ir, lp, scop, schedule) {
        if poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-skew2d-map",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if lower_skew2d_reduce_generic(fn_ir, lp, scop, schedule) {
        if poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-skew2d-reduce",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if generic_mir_effective_for_schedule(scop, schedule)
        && (generic_covers_map || generic_covers_reduce)
    {
        return PolyCodegenPlan { emitted: false };
    }
    PolyCodegenPlan { emitted: false }
}

pub(crate) fn lower_tile1d_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> PolyCodegenPlan {
    if schedule.kind != SchedulePlanKind::Tile1D {
        return PolyCodegenPlan { emitted: false };
    }
    let Some(tile_size) = schedule.tile_size else {
        return PolyCodegenPlan { emitted: false };
    };
    let generic_covers_map = generic_schedule_supports_map(fn_ir, scop, schedule);
    let generic_covers_reduce = generic_schedule_supports_reduce(fn_ir, scop, schedule);
    if lower_tile1d_map_generic(fn_ir, lp, scop, schedule) {
        if poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-tile1d-map",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if lower_tile1d_reduce_generic(fn_ir, lp, scop, schedule) {
        if poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-tile1d-reduce",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if generic_mir_effective_for_schedule(scop, schedule)
        && (generic_covers_map || generic_covers_reduce)
    {
        return PolyCodegenPlan { emitted: false };
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        return PolyCodegenPlan { emitted: false };
    };
    if let Some((assignments, guards)) =
        build_multi_tiled_vector_map_assignments(fn_ir, scop, tile_size)
    {
        let Some(cond) = build_vector_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignment, guard)) = build_tiled_vector_map_assignment(fn_ir, scop, tile_size) {
        let Some(cond) = build_vector_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignments, guards)) =
        build_multi_tiled_2d_col_map_assignments(fn_ir, scop, tile_size)
    {
        let Some(cond) = build_matrix_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignment, guard)) = build_tiled_2d_col_map_assignment(fn_ir, scop, tile_size) {
        let Some(cond) = build_matrix_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignments, guards)) =
        build_multi_tiled_vector_reduce_assignments(fn_ir, lp, scop, tile_size)
    {
        let Some(cond) = build_vector_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignment, guard)) =
        build_tiled_vector_reduce_assignment(fn_ir, lp, scop, tile_size)
    {
        let Some(cond) = build_vector_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignments, guards)) =
        build_multi_tiled_2d_col_reduce_assignments(fn_ir, lp, scop, tile_size)
    {
        let Some(cond) = build_matrix_col_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignment, guard)) =
        build_tiled_2d_col_reduce_assignment(fn_ir, lp, scop, tile_size)
    {
        let Some(cond) = build_matrix_col_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignments, guards)) =
        build_multi_tiled_3d_dim1_map_assignments(fn_ir, scop, tile_size)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignment, guard)) = build_tiled_3d_dim1_map_assignment(fn_ir, scop, tile_size) {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignments, guards)) =
        build_multi_tiled_3d_dim1_reduce_assignments(fn_ir, lp, scop, tile_size)
    {
        let Some(cond) = build_array3_dim1_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignment, guard)) =
        build_tiled_3d_dim1_reduce_assignment(fn_ir, lp, scop, tile_size)
    {
        let Some(cond) = build_array3_dim1_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        return PolyCodegenPlan { emitted };
    }
    PolyCodegenPlan { emitted: false }
}

pub(crate) fn lower_tile2d_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> PolyCodegenPlan {
    if schedule.kind != SchedulePlanKind::Tile2D {
        return PolyCodegenPlan { emitted: false };
    }
    let (Some(tile_rows), Some(tile_cols)) = (schedule.tile_rows, schedule.tile_cols) else {
        return PolyCodegenPlan { emitted: false };
    };
    let generic_covers_map = generic_schedule_supports_map(fn_ir, scop, schedule);
    let generic_covers_reduce = generic_schedule_supports_reduce(fn_ir, scop, schedule);
    if lower_tile2d_map_generic(fn_ir, lp, scop, schedule) {
        if poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-tile2d-map",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if lower_tile2d_reduce_generic(fn_ir, lp, scop, schedule) {
        if poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-tile2d-reduce",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if generic_mir_effective_for_schedule(scop, schedule)
        && (generic_covers_map || generic_covers_reduce)
    {
        return PolyCodegenPlan { emitted: false };
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        return PolyCodegenPlan { emitted: false };
    };
    if let Some((assignments, guards)) =
        build_multi_nested_2d_full_matrix_map_assignments(fn_ir, scop)
    {
        let tile_r = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_rows as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_c = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_cols as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let row_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound);
        let row_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound);
        let col_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound);
        let col_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound);
        let (Some(rs), Some(re), Some(cs), Some(ce)) = (row_start, row_end, col_start, col_end)
        else {
            return PolyCodegenPlan { emitted: false };
        };
        let mut tiled = Vec::new();
        for (assignment, guard) in assignments.into_iter().zip(guards.iter()) {
            let ValueKind::Call { args, .. } = fn_ir.values[assignment.out_val].kind.clone() else {
                return PolyCodegenPlan { emitted: false };
            };
            let op_lit = args.last().copied();
            let Some(op_lit) = op_lit else {
                return PolyCodegenPlan { emitted: false };
            };
            let out_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_tile_matrix_binop_assign".to_string(),
                    args: vec![
                        guard.dest,
                        guard.lhs_src,
                        guard.rhs_src,
                        rs,
                        re,
                        cs,
                        ce,
                        op_lit,
                        tile_r,
                        tile_c,
                    ],
                    names: vec![None, None, None, None, None, None, None, None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            tiled.push(PreparedVectorAssignment {
                dest_var: assignment.dest_var,
                out_val,
                shadow_vars: Vec::new(),
                shadow_idx: None,
            });
        }
        let Some(cond) = build_matrix_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted = finish_vector_assignments_versioned(fn_ir, lp.header, site, tiled, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((out_val, dest_var, guard)) = build_nested_2d_full_matrix_map_value(fn_ir, scop) {
        let ValueKind::Call { args, .. } = fn_ir.values[out_val].kind.clone() else {
            return PolyCodegenPlan { emitted: false };
        };
        let Some(op_lit) = args.last().copied() else {
            return PolyCodegenPlan { emitted: false };
        };
        let tile_r = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_rows as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_c = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_cols as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let row_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound);
        let row_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound);
        let col_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound);
        let col_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound);
        let (Some(rs), Some(re), Some(cs), Some(ce)) = (row_start, row_end, col_start, col_end)
        else {
            return PolyCodegenPlan { emitted: false };
        };
        let tiled_out = fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_tile_matrix_binop_assign".to_string(),
                args: vec![
                    guard.dest,
                    guard.lhs_src,
                    guard.rhs_src,
                    rs,
                    re,
                    cs,
                    ce,
                    op_lit,
                    tile_r,
                    tile_c,
                ],
                names: vec![None, None, None, None, None, None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let Some(cond) = build_matrix_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted = finish_vector_assignments_versioned(
            fn_ir,
            lp.header,
            site,
            vec![PreparedVectorAssignment {
                dest_var,
                out_val: tiled_out,
                shadow_vars: Vec::new(),
                shadow_idx: None,
            }],
            cond,
        );
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignments, guards)) =
        build_multi_nested_2d_full_matrix_reduce_assignments(fn_ir, lp, scop)
    {
        let tile_r = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_rows as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_c = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_cols as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let mut tiled = Vec::new();
        for (assignment, guard) in assignments.into_iter().zip(guards.iter()) {
            let op_lit = extract_reduction_op_literal(fn_ir, assignment.out_val);
            let Some(op_lit) = op_lit else {
                return PolyCodegenPlan { emitted: false };
            };
            let reduce_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_tile_matrix_reduce_rect".to_string(),
                    args: vec![
                        guard.base,
                        guard.r_start,
                        guard.r_end,
                        guard.c_start,
                        guard.c_end,
                        op_lit,
                        tile_r,
                        tile_c,
                    ],
                    names: vec![None, None, None, None, None, None, None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            tiled.push(PreparedVectorAssignment {
                dest_var: assignment.dest_var,
                out_val: replace_reduction_result_in_assignment(
                    fn_ir,
                    assignment.out_val,
                    reduce_val,
                ),
                shadow_vars: Vec::new(),
                shadow_idx: None,
            });
        }
        let Some(cond) = build_matrix_rect_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted = finish_vector_assignments_versioned(fn_ir, lp.header, site, tiled, cond);
        return PolyCodegenPlan { emitted };
    }
    if let Some((assignment, guard)) = build_nested_2d_full_matrix_reduce_value(fn_ir, lp, scop) {
        let tile_r = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_rows as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_c = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_cols as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let op_lit = extract_reduction_op_literal(fn_ir, assignment.out_val);
        let Some(op_lit) = op_lit else {
            return PolyCodegenPlan { emitted: false };
        };
        let reduce_val = fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_tile_matrix_reduce_rect".to_string(),
                args: vec![
                    guard.base,
                    guard.r_start,
                    guard.r_end,
                    guard.c_start,
                    guard.c_end,
                    op_lit,
                    tile_r,
                    tile_c,
                ],
                names: vec![None, None, None, None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let Some(cond) = build_matrix_rect_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let tiled_assignment = PreparedVectorAssignment {
            dest_var: assignment.dest_var,
            out_val: replace_reduction_result_in_assignment(fn_ir, assignment.out_val, reduce_val),
            shadow_vars: Vec::new(),
            shadow_idx: None,
        };
        let emitted = finish_vector_assignments_versioned(
            fn_ir,
            lp.header,
            site,
            vec![tiled_assignment],
            cond,
        );
        return PolyCodegenPlan { emitted };
    }
    PolyCodegenPlan { emitted: false }
}

pub(crate) fn lower_tile3d_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> PolyCodegenPlan {
    if schedule.kind != SchedulePlanKind::Tile3D {
        return PolyCodegenPlan { emitted: false };
    }
    let (Some(tile_depth), Some(tile_rows), Some(tile_cols)) =
        (schedule.tile_depth, schedule.tile_rows, schedule.tile_cols)
    else {
        return PolyCodegenPlan { emitted: false };
    };
    let generic_covers_map = generic_schedule_supports_map(fn_ir, scop, schedule);
    let generic_covers_reduce = generic_schedule_supports_reduce(fn_ir, scop, schedule);
    if lower_tile3d_map_generic(fn_ir, lp, scop, schedule) {
        if poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-tile3d-map",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if lower_tile3d_reduce_generic(fn_ir, lp, scop, schedule) {
        if poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-tile3d-reduce",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if generic_mir_effective_for_schedule(scop, schedule)
        && (generic_covers_map || generic_covers_reduce)
    {
        return PolyCodegenPlan { emitted: false };
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        return PolyCodegenPlan { emitted: false };
    };

    if let Some((assignments, guards)) =
        build_multi_nested_3d_full_cube_map_assignments(fn_ir, scop)
    {
        let tile_i = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_depth as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_j = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_rows as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_k = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_cols as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let i_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound);
        let i_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound);
        let j_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound);
        let j_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound);
        let k_start = encode_bound(fn_ir, &scop.dimensions[2].lower_bound);
        let k_end = encode_bound(fn_ir, &scop.dimensions[2].upper_bound);
        let (Some(is), Some(ie), Some(js), Some(je), Some(ks), Some(ke)) =
            (i_start, i_end, j_start, j_end, k_start, k_end)
        else {
            return PolyCodegenPlan { emitted: false };
        };
        let mut tiled = Vec::new();
        for (assignment, guard) in assignments.into_iter().zip(guards.iter()) {
            let ValueKind::Call { args, .. } = fn_ir.values[assignment.out_val].kind.clone() else {
                return PolyCodegenPlan { emitted: false };
            };
            let Some(op_lit) = args.last().copied() else {
                return PolyCodegenPlan { emitted: false };
            };
            let out_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_tile_array3_binop_cube_assign".to_string(),
                    args: vec![
                        guard.dest,
                        guard.lhs_src,
                        guard.rhs_src,
                        is,
                        ie,
                        js,
                        je,
                        ks,
                        ke,
                        op_lit,
                        tile_i,
                        tile_j,
                        tile_k,
                    ],
                    names: vec![
                        None, None, None, None, None, None, None, None, None, None, None, None,
                        None,
                    ],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            tiled.push(PreparedVectorAssignment {
                dest_var: assignment.dest_var,
                out_val,
                shadow_vars: Vec::new(),
                shadow_idx: None,
            });
        }
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted = finish_vector_assignments_versioned(fn_ir, lp.header, site, tiled, cond);
        return PolyCodegenPlan { emitted };
    }

    if let Some((assignment, guard)) = build_single_nested_3d_full_cube_map_assignment(fn_ir, scop)
    {
        let ValueKind::Call { args, .. } = fn_ir.values[assignment.out_val].kind.clone() else {
            return PolyCodegenPlan { emitted: false };
        };
        let Some(op_lit) = args.last().copied() else {
            return PolyCodegenPlan { emitted: false };
        };
        let tile_i = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_depth as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_j = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_rows as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_k = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_cols as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let i_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound);
        let i_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound);
        let j_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound);
        let j_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound);
        let k_start = encode_bound(fn_ir, &scop.dimensions[2].lower_bound);
        let k_end = encode_bound(fn_ir, &scop.dimensions[2].upper_bound);
        let (Some(is), Some(ie), Some(js), Some(je), Some(ks), Some(ke)) =
            (i_start, i_end, j_start, j_end, k_start, k_end)
        else {
            return PolyCodegenPlan { emitted: false };
        };
        let tiled_out = fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_tile_array3_binop_cube_assign".to_string(),
                args: vec![
                    guard.dest,
                    guard.lhs_src,
                    guard.rhs_src,
                    is,
                    ie,
                    js,
                    je,
                    ks,
                    ke,
                    op_lit,
                    tile_i,
                    tile_j,
                    tile_k,
                ],
                names: vec![
                    None, None, None, None, None, None, None, None, None, None, None, None, None,
                ],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted = finish_vector_assignments_versioned(
            fn_ir,
            lp.header,
            site,
            vec![PreparedVectorAssignment {
                dest_var: assignment.dest_var,
                out_val: tiled_out,
                shadow_vars: Vec::new(),
                shadow_idx: None,
            }],
            cond,
        );
        return PolyCodegenPlan { emitted };
    }

    if let Some((assignments, guards)) =
        build_multi_nested_3d_full_cube_reduce_assignments(fn_ir, lp, scop)
    {
        let tile_i = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_depth as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_j = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_rows as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_k = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_cols as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let mut tiled = Vec::new();
        for (assignment, guard) in assignments.into_iter().zip(guards.iter()) {
            let op_lit = extract_reduction_op_literal(fn_ir, assignment.out_val);
            let Some(op_lit) = op_lit else {
                return PolyCodegenPlan { emitted: false };
            };
            let reduce_val = fn_ir.add_value(
                ValueKind::Call {
                    callee: "rr_tile_array3_reduce_cube".to_string(),
                    args: vec![
                        guard.base,
                        guard.i_start,
                        guard.i_end,
                        guard.j_start,
                        guard.j_end,
                        guard.k_start,
                        guard.k_end,
                        op_lit,
                        tile_i,
                        tile_j,
                        tile_k,
                    ],
                    names: vec![
                        None, None, None, None, None, None, None, None, None, None, None,
                    ],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            );
            tiled.push(PreparedVectorAssignment {
                dest_var: assignment.dest_var,
                out_val: replace_reduction_result_in_assignment(
                    fn_ir,
                    assignment.out_val,
                    reduce_val,
                ),
                shadow_vars: Vec::new(),
                shadow_idx: None,
            });
        }
        let Some(cond) = build_array3_cube_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted = finish_vector_assignments_versioned(fn_ir, lp.header, site, tiled, cond);
        return PolyCodegenPlan { emitted };
    }

    if let Some((assignment, guard)) =
        build_single_nested_3d_full_cube_reduce_assignment(fn_ir, lp, scop)
    {
        let tile_i = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_depth as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_j = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_rows as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let tile_k = fn_ir.add_value(
            ValueKind::Const(Lit::Int(tile_cols as i64)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let op_lit = extract_reduction_op_literal(fn_ir, assignment.out_val);
        let Some(op_lit) = op_lit else {
            return PolyCodegenPlan { emitted: false };
        };
        let reduce_val = fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_tile_array3_reduce_cube".to_string(),
                args: vec![
                    guard.base,
                    guard.i_start,
                    guard.i_end,
                    guard.j_start,
                    guard.j_end,
                    guard.k_start,
                    guard.k_end,
                    op_lit,
                    tile_i,
                    tile_j,
                    tile_k,
                ],
                names: vec![
                    None, None, None, None, None, None, None, None, None, None, None,
                ],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let Some(cond) = build_array3_cube_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let out_val = replace_reduction_result_in_assignment(fn_ir, assignment.out_val, reduce_val);
        let emitted = finish_vector_assignments_versioned(
            fn_ir,
            lp.header,
            site,
            vec![PreparedVectorAssignment {
                dest_var: assignment.dest_var,
                out_val,
                shadow_vars: Vec::new(),
                shadow_idx: None,
            }],
            cond,
        );
        return PolyCodegenPlan { emitted };
    }

    PolyCodegenPlan { emitted: false }
}
