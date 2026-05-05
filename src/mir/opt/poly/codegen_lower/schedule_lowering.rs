use super::*;
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
