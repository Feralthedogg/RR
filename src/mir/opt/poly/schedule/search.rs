use super::*;
pub(crate) fn search_schedule_with_policy(
    scop: &ScopRegion,
    dep_state: DependenceState,
    requested: PolyBackendKind,
    tile_policy: TilePolicy,
) -> SchedulePlan {
    candidate_plans_with_policy(scop, dep_state, requested, tile_policy)
        .into_iter()
        .min_by_key(|plan| {
            (
                estimate_schedule_cost(scop, plan),
                candidate_priority(plan.kind),
            )
        })
        .unwrap_or_else(|| {
            make_plan(
                requested,
                SchedulePlanKind::Identity,
                none_relation(),
                None,
                None,
                None,
                None,
            )
        })
}

pub(crate) fn solver_candidate_state_for_result(
    deps: &DependenceResult,
    requested: PolyBackendKind,
) -> DependenceState {
    if requested == PolyBackendKind::Isl && deps.has_explicit_relations() {
        if deps.relation.reduction_relation.is_some() {
            DependenceState::ReductionProven
        } else if deps.summary.write_count == 0 {
            DependenceState::NotNeeded
        } else {
            DependenceState::IdentityProven
        }
    } else {
        deps.derived_state()
    }
}

pub(crate) fn candidate_schedules_for_backend_result(
    scop: &ScopRegion,
    deps: &DependenceResult,
    requested: PolyBackendKind,
) -> Vec<SchedulePlan> {
    candidate_plans_with_policy(
        scop,
        solver_candidate_state_for_result(deps, requested),
        requested,
        tile_policy_from_env(),
    )
}

pub(crate) fn candidate_schedules_for_backend(
    scop: &ScopRegion,
    deps: &DependenceSummary,
    requested: PolyBackendKind,
) -> Vec<SchedulePlan> {
    candidate_plans_with_policy(scop, deps.state, requested, tile_policy_from_env())
}

pub(crate) fn search_schedule_for_backend(
    scop: &ScopRegion,
    deps: &DependenceSummary,
    requested: PolyBackendKind,
) -> SchedulePlan {
    search_schedule_with_policy(scop, deps.state, requested, tile_policy_from_env())
}

pub(crate) fn search_schedule_for_backend_result(
    scop: &ScopRegion,
    deps: &DependenceResult,
    requested: PolyBackendKind,
) -> SchedulePlan {
    search_schedule_with_policy(
        scop,
        solver_candidate_state_for_result(deps, requested),
        requested,
        tile_policy_from_env(),
    )
}

pub fn search_schedule(scop: &ScopRegion, deps: &DependenceSummary) -> SchedulePlan {
    search_schedule_for_backend(scop, deps, backend_from_env())
}
