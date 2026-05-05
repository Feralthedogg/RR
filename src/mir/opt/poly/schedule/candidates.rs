use super::*;
pub(crate) fn candidate_priority(kind: SchedulePlanKind) -> u8 {
    match kind {
        SchedulePlanKind::Tile3D => 0,
        SchedulePlanKind::Tile2D => 1,
        SchedulePlanKind::Tile1D => 2,
        SchedulePlanKind::Skew2D => 3,
        SchedulePlanKind::Interchange => 4,
        SchedulePlanKind::Identity => 5,
        SchedulePlanKind::None => 6,
    }
}

pub(crate) fn make_plan(
    requested: PolyBackendKind,
    kind: SchedulePlanKind,
    relation: ScheduleRelation,
    tile_size: Option<usize>,
    tile_depth: Option<usize>,
    tile_rows: Option<usize>,
    tile_cols: Option<usize>,
) -> SchedulePlan {
    let backend = match requested {
        PolyBackendKind::Heuristic => PolyBackendUsed::Heuristic,
        PolyBackendKind::Isl => PolyBackendUsed::Isl,
    };
    SchedulePlan {
        kind,
        relation,
        backend,
        tile_size,
        tile_depth,
        tile_rows,
        tile_cols,
    }
}

pub(crate) fn candidate_plans_with_policy(
    scop: &ScopRegion,
    dep_state: DependenceState,
    requested: PolyBackendKind,
    tile_policy: TilePolicy,
) -> Vec<SchedulePlan> {
    let tile_policy = solver_tile_policy(requested, tile_policy);
    match dep_state {
        DependenceState::NotNeeded
        | DependenceState::IdentityProven
        | DependenceState::ReductionProven => {
            let identity = identity_relation(scop);
            let mut candidates = vec![make_plan(
                requested,
                SchedulePlanKind::Identity,
                identity.clone(),
                None,
                None,
                None,
                None,
            )];

            let data_stmt_count = scop
                .statements
                .iter()
                .filter(|stmt| !stmt.accesses.is_empty())
                .count();
            let keep_interchange_open = should_interchange(scop)
                || (requested == PolyBackendKind::Isl
                    && matches!(scop.dimensions.len(), 2 | 3)
                    && data_stmt_count >= 1);
            if keep_interchange_open {
                for relation in interchange_relations(scop) {
                    candidates.push(make_plan(
                        requested,
                        SchedulePlanKind::Interchange,
                        relation,
                        None,
                        None,
                        None,
                        None,
                    ));
                }
            }

            if can_skew_2d(scop, dep_state, tile_policy) {
                candidates.push(make_plan(
                    requested,
                    SchedulePlanKind::Skew2D,
                    skew2d_relation(scop),
                    None,
                    None,
                    None,
                    None,
                ));
            }

            if can_tile_1d(scop, tile_policy) {
                for tile_size in solver_tile1d_variants(tile_policy, requested) {
                    candidates.push(make_plan(
                        requested,
                        SchedulePlanKind::Tile1D,
                        identity.clone(),
                        Some(tile_size),
                        None,
                        None,
                        None,
                    ));
                }
            }
            if can_tile_2d(scop, tile_policy) {
                for (tile_rows, tile_cols) in solver_tile2d_variants(tile_policy, requested) {
                    candidates.push(make_plan(
                        requested,
                        SchedulePlanKind::Tile2D,
                        identity.clone(),
                        None,
                        None,
                        Some(tile_rows),
                        Some(tile_cols),
                    ));
                }
            }
            if can_tile_3d(scop, tile_policy) {
                for (tile_depth, tile_rows, tile_cols) in
                    solver_tile3d_variants(tile_policy, requested)
                {
                    candidates.push(make_plan(
                        requested,
                        SchedulePlanKind::Tile3D,
                        identity.clone(),
                        None,
                        Some(tile_depth),
                        Some(tile_rows),
                        Some(tile_cols),
                    ));
                }
            }

            candidates
        }
        DependenceState::Unknown => vec![make_plan(
            requested,
            SchedulePlanKind::None,
            none_relation(),
            None,
            None,
            None,
            None,
        )],
    }
}
