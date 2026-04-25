const GENERATED_LOOP_IV_PREFIX: &str = ".__poly_gen_iv_";

pub fn generic_mir_enabled() -> bool {
    std::env::var("RR_POLY_GENERIC_MIR")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

pub fn generic_mir_effective_for_schedule(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    generic_mir_enabled() || default_generic_schedule(scop, schedule)
}

fn generic_mir_effective(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    generic_mir_effective_for_schedule(scop, schedule)
}

fn default_generic_schedule(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    default_generic_single_dim(scop, schedule) || default_generic_full_rank_nested(scop, schedule)
}

fn default_generic_single_dim(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    if scop.dimensions.len() != 1 {
        return false;
    }
    let dim = &scop.dimensions[0];
    if dim.step != 1 {
        return false;
    }
    matches!(
        schedule.kind,
        SchedulePlanKind::Identity | SchedulePlanKind::Tile1D
    )
}

fn default_generic_full_rank_nested(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    if scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    match schedule.kind {
        SchedulePlanKind::Identity => matches!(scop.dimensions.len(), 2 | 3),
        SchedulePlanKind::Interchange => matches!(scop.dimensions.len(), 2 | 3),
        SchedulePlanKind::Skew2D => scop.dimensions.len() == 2,
        SchedulePlanKind::Tile2D => scop.dimensions.len() == 2,
        SchedulePlanKind::Tile3D => scop.dimensions.len() == 3,
        _ => false,
    }
}

pub fn generic_fission_enabled() -> bool {
    let enabled = |key: &str| {
        std::env::var(key).ok().is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    };
    enabled("RR_POLY_GENERIC_FISSION") || enabled("RR_POLY_FISSION")
}

fn generic_fission_effective(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    generic_fission_enabled() || default_generic_schedule(scop, schedule)
}

pub fn is_generated_loop_iv_name(name: &str) -> bool {
    name.starts_with(GENERATED_LOOP_IV_PREFIX)
}
