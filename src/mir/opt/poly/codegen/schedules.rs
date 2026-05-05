use super::*;
pub fn lower_identity_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> PolyCodegenPlan {
    if schedule.kind != SchedulePlanKind::Identity {
        return PolyCodegenPlan { emitted: false };
    }
    let generic_covers_map = generic_schedule_supports_map(fn_ir, scop, schedule);
    let generic_covers_reduce = generic_schedule_supports_reduce(fn_ir, scop, schedule);
    if lower_identity_map_generic(fn_ir, lp, scop, schedule) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-1d-identity-map",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if lower_identity_reduce_generic(fn_ir, lp, scop, schedule) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-1d-identity-reduce",
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
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) =
            build_multi_whole_vector_map_assignments(fn_ir, lp, scop)
    {
        let Some(cond) = build_vector_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-store-map-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) = build_multi_range_vector_map_assignments(fn_ir, scop)
    {
        let Some(cond) = build_vector_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-range-map-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) = build_single_whole_vector_map_assignment(fn_ir, lp, scop)
    {
        let Some(cond) = build_vector_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-1d-map-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) = build_single_range_vector_map_assignment(fn_ir, scop)
    {
        let Some(cond) = build_vector_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-1d-range-map-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) =
            build_multi_whole_vector_reduce_assignments(fn_ir, lp, scop)
    {
        let Some(cond) = build_vector_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-1d-reduce-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) =
            build_single_whole_vector_reduce_assignment(fn_ir, lp, scop)
    {
        let Some(cond) = build_vector_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-1d-reduce-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) = build_multi_2d_col_map_assignments(fn_ir, scop)
    {
        let Some(cond) = build_matrix_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-2d-col-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) = build_single_2d_col_map_assignment(fn_ir, scop)
    {
        let Some(cond) = build_matrix_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-2d-col-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) = build_multi_2d_col_reduce_assignments(fn_ir, lp, scop)
    {
        let Some(cond) = build_matrix_col_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-2d-col-reduce",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) =
            build_multi_nested_3d_full_cube_map_assignments(fn_ir, scop)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-nested-3d-cube-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) =
            build_single_nested_3d_full_cube_map_assignment(fn_ir, scop)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-nested-3d-cube-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) =
            build_multi_nested_3d_full_cube_reduce_assignments(fn_ir, lp, scop)
    {
        let Some(cond) = build_array3_cube_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-nested-3d-cube-reduce",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) =
            build_single_nested_3d_full_cube_reduce_assignment(fn_ir, lp, scop)
    {
        let Some(cond) = build_array3_cube_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-nested-3d-cube-reduce",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) = build_multi_3d_dim1_map_assignments(fn_ir, scop)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-3d-dim1-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) = build_single_3d_dim1_map_assignment(fn_ir, scop)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-3d-dim1-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) = build_multi_3d_dim1_reduce_assignments(fn_ir, lp, scop)
    {
        let Some(cond) = build_array3_dim1_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-3d-dim1-reduce",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    let Some(plan) = build_identity_plan(fn_ir, lp, scop) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} plan build failed",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: false };
    };
    let emitted = try_apply_vectorization_transactionally(fn_ir, lp, plan.clone());
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-codegen] {} loop header={} apply={} plan={:?}",
            fn_ir.name, lp.header, emitted, plan
        );
    }
    PolyCodegenPlan { emitted }
}
