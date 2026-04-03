use super::ScopRegion;
use super::cost::estimate_schedule_cost;
use super::dependence_backend::DependenceResult;
use super::isl::{
    IslArtifacts, IslTransformHints, infer_transform_hints, isl_available,
    materialize_schedule_artifacts, snapshot_schedule_artifacts,
};
use super::schedule::{
    PolyBackendKind, PolyBackendUsed, SchedulePlan, SchedulePlanKind,
    backend_from_env as requested_backend_from_env, candidate_priority,
    candidate_schedules_for_backend_result, search_schedule_for_backend_result,
};
use super::tree::ScheduleTree;

pub trait PolySolverBackend {
    fn used_backend(&self) -> PolyBackendUsed;
    fn build_schedule_tree(&self, scop: &ScopRegion, deps: &DependenceResult) -> ScheduleTree;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HeuristicBackend;

#[derive(Debug, Default, Clone, Copy)]
pub struct IslBackend;

#[derive(Debug, Clone)]
struct IslCandidateEvaluation {
    plan: SchedulePlan,
    artifacts: Option<IslArtifacts>,
    hints: IslTransformHints,
}

fn constant_iteration_volume(scop: &ScopRegion) -> Option<u32> {
    let mut volume = 1u32;
    for dim in &scop.dimensions {
        if !dim.lower_bound.terms.is_empty() || !dim.upper_bound.terms.is_empty() || dim.step == 0 {
            return None;
        }
        let step = dim.step.unsigned_abs();
        if step == 0 {
            return None;
        }
        let lower = dim.lower_bound.constant;
        let upper = dim.upper_bound.constant;
        if upper < lower {
            return Some(0);
        }
        let extent = ((upper - lower) as u64 / step + 1) as u32;
        volume = volume.checked_mul(extent)?;
    }
    Some(volume)
}

fn tile_volume(plan: &SchedulePlan) -> Option<u32> {
    match plan.kind {
        SchedulePlanKind::Tile1D => Some(plan.tile_size? as u32),
        SchedulePlanKind::Tile2D => {
            Some((plan.tile_rows? as u32).saturating_mul(plan.tile_cols? as u32))
        }
        SchedulePlanKind::Tile3D => Some(
            (plan.tile_depth? as u32)
                .saturating_mul(plan.tile_rows? as u32)
                .saturating_mul(plan.tile_cols? as u32),
        ),
        _ => None,
    }
}

fn tiny_tile_candidate_penalty(scop: &ScopRegion, plan: &SchedulePlan) -> u32 {
    let (Some(iter_vol), Some(tile_vol)) = (constant_iteration_volume(scop), tile_volume(plan))
    else {
        return 0;
    };
    if tile_vol == 0 {
        return 0;
    }
    if iter_vol <= tile_vol.saturating_mul(2) {
        128
    } else {
        0
    }
}

fn normalize_hints_for_candidate(
    scop: &ScopRegion,
    plan: &SchedulePlan,
    artifacts: Option<&IslArtifacts>,
    mut hints: IslTransformHints,
) -> IslTransformHints {
    let is_tile = matches!(
        plan.kind,
        SchedulePlanKind::Tile1D | SchedulePlanKind::Tile2D | SchedulePlanKind::Tile3D
    );
    let neutral_identity_hint = matches!(
        hints.inferred_plan.as_ref().map(|plan| plan.kind),
        Some(SchedulePlanKind::Identity)
    );
    let artifact_is_neutral = artifacts.is_some_and(|artifact| {
        artifact.first_band_members == scop.dimensions.len()
            && matches!(artifact.root_type.as_str(), "domain" | "band")
    });
    if is_tile && artifact_is_neutral && neutral_identity_hint {
        hints.inferred_plan = Some(plan.clone());
        let tile_plan_reason = format!("hint_plan={:?}", plan.kind);
        if hints.reason.contains("hint_plan=Identity") {
            hints.reason = hints
                .reason
                .replace("hint_plan=Identity", &tile_plan_reason);
        } else if !hints.reason.contains("hint_plan=") {
            if hints.reason.is_empty() {
                hints.reason = tile_plan_reason;
            } else {
                hints.reason.push(',');
                hints.reason.push_str(&tile_plan_reason);
            }
        }
        if hints.reason.is_empty() {
            hints.reason = "hint_candidate_tile=1".to_string();
        } else if !hints.reason.contains("hint_candidate_tile=1") {
            hints.reason.push_str(",hint_candidate_tile=1");
        }
    }
    hints
}

fn safe_reduction_conditional_validity_candidate<'a>(
    scop: &ScopRegion,
    deps: &'a DependenceResult,
    _plan: &SchedulePlan,
) -> Option<&'a str> {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count();
    if data_stmt_count == 1
        && scop.dimensions.iter().all(|dim| dim.step == 1)
        && deps.relation.validity_relation.is_none()
        && deps.relation.reduction_relation.is_some()
    {
        deps.relation.reduction_relation.as_deref()
    } else {
        None
    }
}

fn safe_reduction_conditional_validity_direct<'a>(
    scop: &ScopRegion,
    deps: &'a DependenceResult,
    plan: &SchedulePlan,
) -> Option<&'a str> {
    safe_reduction_conditional_validity_candidate(scop, deps, plan)
}

fn evaluate_isl_candidate(
    scop: &ScopRegion,
    deps: &DependenceResult,
    plan: &SchedulePlan,
) -> IslCandidateEvaluation {
    let reduction_sensitive = deps.relation.reduction_relation.is_some();
    let direct_conditional_validity = safe_reduction_conditional_validity_direct(scop, deps, plan);
    let conditional_validity_candidate =
        safe_reduction_conditional_validity_candidate(scop, deps, plan);
    let (artifacts, conditional_validity_helper_fallback) = if reduction_sensitive {
        let materialized = materialize_schedule_artifacts(
            scop,
            plan,
            deps.relation.validity_relation.as_deref(),
            deps.relation.proximity_relation.as_deref(),
            None,
            direct_conditional_validity,
            conditional_validity_candidate,
        );
        let helper_fallback = direct_conditional_validity.is_some() && materialized.is_none();
        (
            materialized.or_else(|| {
                Some(snapshot_schedule_artifacts(
                    scop,
                    plan,
                    deps.relation.validity_relation.as_deref(),
                    deps.relation.proximity_relation.as_deref(),
                    None,
                    direct_conditional_validity,
                    conditional_validity_candidate,
                ))
            }),
            helper_fallback,
        )
    } else {
        (
            materialize_schedule_artifacts(
                scop,
                plan,
                deps.relation.validity_relation.as_deref(),
                deps.relation.proximity_relation.as_deref(),
                None,
                direct_conditional_validity,
                conditional_validity_candidate,
            ),
            false,
        )
    };
    let mut hints = artifacts
        .as_ref()
        .map(|artifact| infer_transform_hints(scop, PolyBackendUsed::Isl, artifact))
        .map(|hints| normalize_hints_for_candidate(scop, plan, artifacts.as_ref(), hints))
        .unwrap_or_default();
    if conditional_validity_helper_fallback
        && !hints
            .reason
            .contains("hint_conditional_validity_helper_fallback=1")
    {
        if !hints.reason.is_empty() {
            hints.reason.push(',');
        }
        hints
            .reason
            .push_str("hint_conditional_validity_helper_fallback=1");
    }
    if artifacts
        .as_ref()
        .is_some_and(|artifact| artifact.conditional_validity_applied)
        && !hints.reason.contains("hint_conditional_validity_applied=1")
    {
        if !hints.reason.is_empty() {
            hints.reason.push(',');
        }
        hints.reason.push_str("hint_conditional_validity_applied=1");
    }
    IslCandidateEvaluation {
        plan: plan.clone(),
        artifacts,
        hints,
    }
}

fn evaluate_isl_solver_only(
    scop: &ScopRegion,
    deps: &DependenceResult,
    backend: PolyBackendUsed,
) -> IslCandidateEvaluation {
    let empty_plan = SchedulePlan {
        kind: SchedulePlanKind::None,
        relation: super::schedule::ScheduleRelation {
            input_dimensions: Vec::new(),
            output_expressions: Vec::new(),
        },
        backend,
        tile_size: None,
        tile_depth: None,
        tile_rows: None,
        tile_cols: None,
    };
    evaluate_isl_candidate(scop, deps, &empty_plan)
}

fn isl_candidate_score(
    scop: &ScopRegion,
    heuristic_plan: &SchedulePlan,
    candidate: &IslCandidateEvaluation,
    reduction_sensitive: bool,
) -> (u8, u8, u8, u32, u8, u8) {
    let support_penalty = if candidate.artifacts.is_some() { 0 } else { 4 };
    let root_penalty = match candidate
        .artifacts
        .as_ref()
        .map(|artifact| artifact.root_type.as_str())
    {
        Some("band") | Some("sequence") | Some("set") => 0,
        Some("domain") => 1,
        Some(_) => 2,
        None => 3,
    };
    let first_band_penalty = match candidate.artifacts.as_ref() {
        Some(artifact) if artifact.first_band_partial_schedule.is_some() => 0,
        Some(artifact) if artifact.first_band_members > 0 => 1,
        Some(_) => 2,
        None => 3,
    };
    let hint_penalty = match candidate.hints.inferred_plan.as_ref() {
        Some(plan) if plan.kind == candidate.plan.kind => 0,
        Some(_) => 2,
        None => 1,
    };
    let raw_hint_matches_candidate = candidate
        .hints
        .inferred_plan
        .as_ref()
        .is_some_and(|plan| plan.kind == candidate.plan.kind);
    let hint_matches_candidate = raw_hint_matches_candidate
        && !(candidate.plan.kind == SchedulePlanKind::Identity
            && scop.dimensions.len() > 1
            && heuristic_plan.kind != SchedulePlanKind::Identity);
    let heuristic_divergence_penalty = if candidate.plan.kind != heuristic_plan.kind
        && !reduction_sensitive
        && !hint_matches_candidate
    {
        6
    } else {
        0
    };
    let cost = estimate_schedule_cost(scop, &candidate.plan)
        .saturating_add(tiny_tile_candidate_penalty(scop, &candidate.plan));
    (
        support_penalty,
        root_penalty,
        first_band_penalty,
        cost,
        hint_penalty + heuristic_divergence_penalty,
        candidate_priority(candidate.plan.kind),
    )
}

fn choose_best_isl_candidate(
    scop: &ScopRegion,
    heuristic_plan: &SchedulePlan,
    candidates: Vec<SchedulePlan>,
    deps: &DependenceResult,
) -> IslCandidateEvaluation {
    let reduction_sensitive = deps.relation.reduction_relation.is_some();
    let mut evals = candidates
        .into_iter()
        .map(|plan| evaluate_isl_candidate(scop, deps, &plan))
        .collect::<Vec<_>>();
    evals.push(evaluate_isl_solver_only(scop, deps, heuristic_plan.backend));
    let hinted_plans = evals
        .iter()
        .filter_map(|candidate| candidate.hints.inferred_plan.clone())
        .filter(|plan| {
            !evals.iter().any(|existing| {
                existing.plan.kind == plan.kind && existing.plan.relation == plan.relation
            })
        })
        .collect::<Vec<_>>();
    for plan in hinted_plans {
        evals.push(evaluate_isl_candidate(scop, deps, &plan));
    }
    evals.sort_by_key(|candidate| {
        isl_candidate_score(scop, heuristic_plan, candidate, reduction_sensitive)
    });
    evals
        .into_iter()
        .next()
        .unwrap_or_else(|| evaluate_isl_candidate(scop, deps, heuristic_plan))
}

impl PolySolverBackend for HeuristicBackend {
    fn used_backend(&self) -> PolyBackendUsed {
        PolyBackendUsed::Heuristic
    }

    fn build_schedule_tree(&self, scop: &ScopRegion, deps: &DependenceResult) -> ScheduleTree {
        ScheduleTree::from_plan(
            scop,
            deps,
            search_schedule_for_backend_result(scop, deps, PolyBackendKind::Heuristic),
        )
    }
}

impl PolySolverBackend for IslBackend {
    fn used_backend(&self) -> PolyBackendUsed {
        PolyBackendUsed::Isl
    }

    fn build_schedule_tree(&self, scop: &ScopRegion, deps: &DependenceResult) -> ScheduleTree {
        let heuristic_plan = search_schedule_for_backend_result(scop, deps, PolyBackendKind::Isl);
        let candidates = candidate_schedules_for_backend_result(scop, deps, PolyBackendKind::Isl);
        let selected = choose_best_isl_candidate(scop, &heuristic_plan, candidates, deps);
        let hint_selected =
            usize::from(selected.hints.prefer_fission || selected.hints.inferred_plan.is_some());
        let artifact = selected.artifacts.map(|artifact| {
            format!(
                "{}; hint_selected={}; hint_reason={}; chosen_kind={:?}",
                artifact.render(),
                hint_selected,
                selected.hints.reason,
                selected.plan.kind,
            )
        });
        ScheduleTree::from_plan_with_hints(
            scop,
            deps,
            selected.plan,
            selected.hints.prefer_fission.then_some(true),
            selected
                .hints
                .prefer_fission
                .then_some("backend-hint-fission"),
        )
        .with_backend_artifact(artifact)
    }
}

pub fn make_backend_from_env() -> Box<dyn PolySolverBackend> {
    match requested_backend_from_env() {
        PolyBackendKind::Heuristic => Box::new(HeuristicBackend),
        PolyBackendKind::Isl if isl_available() => Box::new(IslBackend),
        PolyBackendKind::Isl => Box::new(HeuristicBackend),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::opt::poly::access::{AccessKind, AccessRelation, MemRef, MemoryLayout};
    use crate::mir::opt::poly::affine::{
        AffineConstraint, AffineConstraintKind, AffineExpr, AffineSymbol, PresburgerSet,
    };
    use crate::mir::opt::poly::dependence_backend::{
        DependenceEdge, DependenceKind, DependenceRelation, DependenceResult, DependenceState,
        DependenceSummary,
    };
    use crate::mir::opt::poly::schedule::{
        PolyBackendKind, ScheduleRelation, candidate_schedules_for_backend_result,
    };
    use crate::mir::opt::poly::{LoopDimension, PolyStmt, PolyStmtKind, ScopRegion};

    #[test]
    fn env_backend_factory_defaults_to_heuristic() {
        let backend = HeuristicBackend;
        assert_eq!(backend.used_backend(), PolyBackendUsed::Heuristic);
        let backend = IslBackend;
        assert_eq!(backend.used_backend(), PolyBackendUsed::Isl);
    }

    #[test]
    fn isl_reduction_candidate_search_is_not_locked_to_heuristic_identity() {
        let loop_r = AffineExpr::symbol(AffineSymbol::LoopIv("r".to_string()));
        let loop_c = AffineExpr::symbol(AffineSymbol::LoopIv("c".to_string()));
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(8),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(8),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(
                vec!["r".to_string(), "c".to_string()],
                vec![
                    AffineConstraint {
                        expr: AffineExpr::constant(1),
                        kind: AffineConstraintKind::LowerBound,
                    },
                    AffineConstraint {
                        expr: AffineExpr::constant(8),
                        kind: AffineConstraintKind::UpperBound,
                    },
                ],
            ),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Assign {
                    dst: "acc".to_string(),
                },
                expr_root: None,
                accesses: vec![AccessRelation {
                    statement_id: 0,
                    kind: AccessKind::Read,
                    memref: MemRef {
                        base: 0,
                        name: "a".to_string(),
                        rank: 2,
                        layout: MemoryLayout::ColumnMajor2D,
                    },
                    subscripts: vec![loop_r.clone(), loop_c.clone()],
                }],
            }],
        };
        let deps = DependenceResult {
            summary: DependenceSummary {
                state: DependenceState::ReductionProven,
                write_count: 0,
                access_count: 1,
                reduction_count: 1,
            },
            relation: DependenceRelation {
                iteration_dimensions: vec!["r".to_string(), "c".to_string()],
                edge_count: 1,
                raw_relation: None,
                war_relation: None,
                waw_relation: None,
                reduction_relation: Some("{ S0[r, c] -> S0[r, c] }".to_string()),
                validity_relation: None,
                proximity_relation: None,
                symbolic_guard_candidate: None,
            },
            edges: vec![DependenceEdge {
                kind: DependenceKind::Reduction,
                statement_id: 0,
                memref_base: 0,
            }],
        };

        let heuristic_plan = SchedulePlan {
            kind: SchedulePlanKind::Identity,
            relation: ScheduleRelation {
                input_dimensions: vec!["r".to_string(), "c".to_string()],
                output_expressions: vec![loop_r.clone(), loop_c.clone()],
            },
            backend: PolyBackendUsed::Isl,
            tile_size: None,
            tile_depth: None,
            tile_rows: None,
            tile_cols: None,
        };
        let candidates = candidate_schedules_for_backend_result(&scop, &deps, PolyBackendKind::Isl)
            .into_iter()
            .filter(|plan| {
                matches!(
                    plan.kind,
                    SchedulePlanKind::Identity | SchedulePlanKind::Interchange
                )
            })
            .collect::<Vec<_>>();

        let selected = choose_best_isl_candidate(&scop, &heuristic_plan, candidates, &deps);
        assert_eq!(selected.plan.kind, SchedulePlanKind::Interchange);
    }

    #[test]
    fn tile_candidate_identity_hint_is_normalized_back_to_tile() {
        let loop_r = AffineExpr::symbol(AffineSymbol::LoopIv("r".to_string()));
        let loop_c = AffineExpr::symbol(AffineSymbol::LoopIv("c".to_string()));
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(8),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(8),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(vec!["r".to_string(), "c".to_string()], vec![]),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Assign {
                    dst: "v0".to_string(),
                },
                expr_root: None,
                accesses: vec![],
            }],
        };
        let plan = SchedulePlan {
            kind: SchedulePlanKind::Tile2D,
            relation: ScheduleRelation {
                input_dimensions: vec!["r".to_string(), "c".to_string()],
                output_expressions: vec![loop_r.clone(), loop_c.clone()],
            },
            backend: PolyBackendUsed::Isl,
            tile_size: None,
            tile_depth: None,
            tile_rows: Some(8),
            tile_cols: Some(8),
        };
        let artifacts = IslArtifacts {
            domain: String::new(),
            validity: None,
            proximity: None,
            coincidence: None,
            conditional_validity: None,
            conditional_validity_applied: false,
            conditional_validity_candidate: None,
            candidate_schedule_map: None,
            candidate_schedule_roundtrip: None,
            computed_schedule: String::new(),
            root_type: "domain".to_string(),
            contains_sequence_node: false,
            contains_filter_node: false,
            first_band_members: 2,
            first_band_partial_schedule: Some("{ S0[r, c] -> [r, c] }".to_string()),
        };
        let hints = normalize_hints_for_candidate(
            &scop,
            &plan,
            Some(&artifacts),
            IslTransformHints {
                inferred_plan: Some(SchedulePlan {
                    kind: SchedulePlanKind::Identity,
                    relation: ScheduleRelation {
                        input_dimensions: vec!["r".to_string(), "c".to_string()],
                        output_expressions: vec![loop_r.clone(), loop_c.clone()],
                    },
                    backend: PolyBackendUsed::Isl,
                    tile_size: None,
                    tile_depth: None,
                    tile_rows: None,
                    tile_cols: None,
                }),
                prefer_fission: false,
                reason: "hint_plan=Identity".to_string(),
            },
        );
        assert_eq!(
            hints.inferred_plan.as_ref().map(|plan| plan.kind),
            Some(SchedulePlanKind::Tile2D)
        );
        assert!(hints.reason.contains("hint_candidate_tile=1"));
    }

    #[test]
    fn isl_candidate_search_prefers_lower_cost_interchange_over_heuristic_identity() {
        let loop_r = AffineExpr::symbol(AffineSymbol::LoopIv("r".to_string()));
        let loop_c = AffineExpr::symbol(AffineSymbol::LoopIv("c".to_string()));
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(8),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(8),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(
                vec!["r".to_string(), "c".to_string()],
                vec![
                    AffineConstraint {
                        expr: AffineExpr::constant(1),
                        kind: AffineConstraintKind::LowerBound,
                    },
                    AffineConstraint {
                        expr: AffineExpr::constant(8),
                        kind: AffineConstraintKind::UpperBound,
                    },
                ],
            ),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0, 1],
                },
                expr_root: None,
                accesses: vec![AccessRelation {
                    statement_id: 0,
                    kind: AccessKind::Write,
                    memref: MemRef {
                        base: 1,
                        name: "A".to_string(),
                        rank: 2,
                        layout: MemoryLayout::ColumnMajor2D,
                    },
                    subscripts: vec![loop_r.clone(), loop_c.clone()],
                }],
            }],
        };
        let deps = DependenceResult {
            summary: DependenceSummary {
                state: DependenceState::IdentityProven,
                write_count: 1,
                access_count: 1,
                reduction_count: 0,
            },
            relation: DependenceRelation {
                iteration_dimensions: vec!["r".to_string(), "c".to_string()],
                edge_count: 0,
                raw_relation: None,
                war_relation: None,
                waw_relation: None,
                reduction_relation: None,
                validity_relation: None,
                proximity_relation: None,
                symbolic_guard_candidate: None,
            },
            edges: Vec::new(),
        };

        let heuristic_plan = SchedulePlan {
            kind: SchedulePlanKind::Identity,
            relation: ScheduleRelation {
                input_dimensions: vec!["r".to_string(), "c".to_string()],
                output_expressions: vec![loop_r.clone(), loop_c.clone()],
            },
            backend: PolyBackendUsed::Isl,
            tile_size: None,
            tile_depth: None,
            tile_rows: None,
            tile_cols: None,
        };
        let candidates = candidate_schedules_for_backend_result(&scop, &deps, PolyBackendKind::Isl)
            .into_iter()
            .filter(|plan| {
                matches!(
                    plan.kind,
                    SchedulePlanKind::Identity | SchedulePlanKind::Interchange
                )
            })
            .collect::<Vec<_>>();

        let selected = choose_best_isl_candidate(&scop, &heuristic_plan, candidates, &deps);
        assert_eq!(selected.plan.kind, SchedulePlanKind::Interchange);
    }
}
