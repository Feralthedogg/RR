pub(super) fn lower_fission_sequence_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if !generic_fission_effective(scop, schedule) || !has_multiple_data_statements(scop) {
        return false;
    }

    match schedule.kind {
        SchedulePlanKind::Identity => {
            if scop.dimensions.len() == 1
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .all(|access| access_is_generic_single_dim_contiguous(access, scop))
                && scop_is_generic_map_compatible(fn_ir, scop)
            {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_loop_nest,
                );
            }
            if scop.dimensions.len() == 1 && scop_is_generic_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::Interchange => {
            if scop_is_generic_map_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_loop_nest,
                );
            }
            if scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::Skew2D => {
            if scop_is_generic_map_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_skewed_2d_loop_nest,
                );
            }
            if scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_skewed_2d_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::Tile1D => {
            if scop.dimensions.len() == 1
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .all(|access| access_is_generic_single_dim_contiguous(access, scop))
                && scop_is_generic_map_compatible(fn_ir, scop)
            {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_tiled_1d_loop_nest,
                );
            }
            if scop.dimensions.len() == 1 && scop_is_generic_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_tiled_1d_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::Tile2D => {
            if scop_is_generic_map_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_tiled_2d_loop_nest,
                );
            }
            if scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_tiled_2d_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::Tile3D => {
            if scop_is_generic_map_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_tiled_3d_loop_nest,
                );
            }
            if scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_tiled_3d_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::None => false,
    }
}
