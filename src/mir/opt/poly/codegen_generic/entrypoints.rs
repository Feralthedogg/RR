pub fn lower_identity_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Identity {
        return false;
    }
    if scop.dimensions.len() == 1
        && scop
            .statements
            .iter()
            .flat_map(|stmt| stmt.accesses.iter())
            .any(|access| !access_is_generic_single_dim_contiguous(access, scop))
    {
        return false;
    }
    if !scop_is_generic_map_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_map_schedule(fn_ir, lp, scop, schedule)
}

pub fn generic_schedule_supports_map(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    match schedule.kind {
        SchedulePlanKind::Identity => {
            if scop.dimensions.len() == 1
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .any(|access| !access_is_generic_single_dim_contiguous(access, scop))
            {
                return false;
            }
            scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Interchange => {
            (scop.dimensions.len() == 2 || scop.dimensions.len() == 3)
                && scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Skew2D => {
            scop.dimensions.len() == 2 && scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile1D => {
            scop.dimensions.len() == 1
                && scop.dimensions[0].step == 1
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .all(|access| access_is_generic_single_dim_contiguous(access, scop))
                && scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile2D => {
            scop.dimensions.len() == 2
                && scop.dimensions.iter().all(|dim| dim.step == 1)
                && scop.statements.iter().all(|stmt| {
                    matches!(
                        &stmt.kind,
                        PolyStmtKind::Store { subscripts, .. } if subscripts.len() == 2
                    ) || !matches!(&stmt.kind, PolyStmtKind::Store { .. })
                })
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .all(|access| {
                        access.memref.layout == MemoryLayout::ColumnMajor2D
                            && access.subscripts.len() == 2
                    })
                && scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile3D => {
            scop.dimensions.len() == 3
                && scop.dimensions.iter().all(|dim| dim.step == 1)
                && scop.statements.iter().all(|stmt| {
                    matches!(
                        &stmt.kind,
                        PolyStmtKind::Store { subscripts, .. } if subscripts.len() == 3
                    ) || !matches!(&stmt.kind, PolyStmtKind::Store { .. })
                })
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .all(|access| {
                        access.memref.layout == MemoryLayout::ColumnMajor3D
                            && access.subscripts.len() == 3
                    })
                && scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::None => false,
    }
}

pub fn lower_tile1d_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile1D || scop.dimensions.len() != 1 {
        return false;
    }
    if scop.dimensions[0].step != 1 {
        return false;
    }
    if scop
        .statements
        .iter()
        .flat_map(|stmt| stmt.accesses.iter())
        .any(|access| !access_is_generic_single_dim_contiguous(access, scop))
    {
        return false;
    }
    if !scop_is_generic_map_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_1d(fn_ir, lp, scop, schedule, false)
}

pub fn lower_tile2d_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile2D || scop.dimensions.len() != 2 {
        return false;
    }
    if scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    if scop.statements.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            PolyStmtKind::Store { subscripts, .. } if subscripts.len() != 2
        )
    }) {
        return false;
    }
    if scop
        .statements
        .iter()
        .flat_map(|stmt| stmt.accesses.iter())
        .any(|access| {
            access.memref.layout != MemoryLayout::ColumnMajor2D || access.subscripts.len() != 2
        })
    {
        return false;
    }
    if !scop_is_generic_map_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_2d(fn_ir, lp, scop, schedule, false)
}

pub fn lower_tile3d_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile3D || scop.dimensions.len() != 3 {
        return false;
    }
    if scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    if scop.statements.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            PolyStmtKind::Store { subscripts, .. } if subscripts.len() != 3
        )
    }) {
        return false;
    }
    if scop
        .statements
        .iter()
        .flat_map(|stmt| stmt.accesses.iter())
        .any(|access| {
            access.memref.layout != MemoryLayout::ColumnMajor3D || access.subscripts.len() != 3
        })
    {
        return false;
    }
    if !scop_is_generic_map_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_3d(fn_ir, lp, scop, schedule, false)
}

pub fn lower_identity_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Identity {
        return false;
    }
    if scop.dimensions.len() == 1 {
        if !scop_is_generic_reduce_compatible(fn_ir, scop) {
            return false;
        }
        return lower_generic_reduce_schedule(fn_ir, lp, scop, schedule);
    }
    if !scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_reduce_schedule(fn_ir, lp, scop, schedule)
}

pub fn generic_schedule_supports_reduce(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    match schedule.kind {
        SchedulePlanKind::Identity => {
            if scop.dimensions.len() == 1 {
                scop_is_generic_reduce_compatible(fn_ir, scop)
            } else {
                scop_is_generic_nested_reduce_compatible(fn_ir, scop)
            }
        }
        SchedulePlanKind::Interchange => {
            (scop.dimensions.len() == 2 || scop.dimensions.len() == 3)
                && scop_is_generic_nested_reduce_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Skew2D => {
            scop.dimensions.len() == 2 && scop_is_generic_nested_reduce_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile1D => {
            scop.dimensions.len() == 1
                && scop.dimensions[0].step == 1
                && scop_is_generic_reduce_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile2D => {
            scop.dimensions.len() == 2
                && scop.dimensions.iter().all(|dim| dim.step == 1)
                && scop_is_generic_nested_reduce_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile3D => {
            scop.dimensions.len() == 3
                && scop.dimensions.iter().all(|dim| dim.step == 1)
                && scop_is_generic_nested_reduce_compatible(fn_ir, scop)
        }
        SchedulePlanKind::None => false,
    }
}

pub fn lower_tile1d_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile1D || scop.dimensions.len() != 1 {
        return false;
    }
    if scop.dimensions[0].step != 1 {
        return false;
    }
    if !scop_is_generic_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_1d(fn_ir, lp, scop, schedule, true)
}

pub fn lower_tile2d_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile2D || scop.dimensions.len() != 2 {
        return false;
    }
    if scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    if !scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_2d(fn_ir, lp, scop, schedule, true)
}

pub fn lower_tile3d_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile3D || scop.dimensions.len() != 3 {
        return false;
    }
    if scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    if !scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_3d(fn_ir, lp, scop, schedule, true)
}

pub fn lower_interchange_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Interchange
        || !(scop.dimensions.len() == 2 || scop.dimensions.len() == 3)
    {
        return false;
    }
    lower_generic_map_schedule(fn_ir, lp, scop, schedule)
}

pub fn lower_interchange_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Interchange
        || !(scop.dimensions.len() == 2 || scop.dimensions.len() == 3)
    {
        return false;
    }
    if !scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_reduce_schedule(fn_ir, lp, scop, schedule)
}

pub fn lower_skew2d_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Skew2D || scop.dimensions.len() != 2 {
        return false;
    }
    lower_generic_skew2d(fn_ir, lp, scop, schedule, false)
}

pub fn lower_skew2d_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Skew2D || scop.dimensions.len() != 2 {
        return false;
    }
    if !scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_skew2d(fn_ir, lp, scop, schedule, true)
}
