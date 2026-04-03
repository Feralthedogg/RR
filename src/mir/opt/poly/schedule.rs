use super::ScopRegion;
use super::access::MemoryLayout;
use super::affine::{AffineExpr, AffineSymbol};
use super::cost::estimate_schedule_cost;
use super::dependence_backend::{DependenceResult, DependenceState, DependenceSummary};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyBackendKind {
    Heuristic,
    Isl,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolyBackendUsed {
    Heuristic,
    Isl,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleRelation {
    pub input_dimensions: Vec<String>,
    pub output_expressions: Vec<AffineExpr>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SchedulePlanKind {
    None,
    Identity,
    Interchange,
    Skew2D,
    Tile1D,
    Tile2D,
    Tile3D,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SchedulePlan {
    pub kind: SchedulePlanKind,
    pub relation: ScheduleRelation,
    pub backend: PolyBackendUsed,
    pub tile_size: Option<usize>,
    pub tile_depth: Option<usize>,
    pub tile_rows: Option<usize>,
    pub tile_cols: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct TilePolicy {
    enable_1d: bool,
    skew_2d_mode: AutoChoice,
    allow_skew_with_tiles: bool,
    tile_size: usize,
    enable_2d: bool,
    enable_3d: bool,
    tile_depth: usize,
    tile_rows: usize,
    tile_cols: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AutoChoice {
    Auto,
    ForceOn,
    ForceOff,
}

pub fn parse_backend_name(raw: &str) -> PolyBackendKind {
    match raw.trim().to_ascii_lowercase().as_str() {
        "isl" => PolyBackendKind::Isl,
        _ => PolyBackendKind::Heuristic,
    }
}

pub fn backend_from_env() -> PolyBackendKind {
    std::env::var("RR_POLY_BACKEND")
        .map(|raw| parse_backend_name(&raw))
        .unwrap_or(PolyBackendKind::Heuristic)
}

fn identity_relation(scop: &ScopRegion) -> ScheduleRelation {
    ScheduleRelation {
        input_dimensions: scop
            .dimensions
            .iter()
            .map(|dim| dim.iv_name.clone())
            .collect(),
        output_expressions: scop
            .dimensions
            .iter()
            .map(|dim| AffineExpr::symbol(AffineSymbol::LoopIv(dim.iv_name.clone())))
            .collect(),
    }
}

pub(crate) fn interchange_relation(scop: &ScopRegion) -> ScheduleRelation {
    let mut dims = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.clone())
        .collect::<Vec<_>>();
    if dims.len() >= 2 {
        dims.rotate_left(1);
    }
    interchange_relation_from_order(scop, dims)
}

fn interchange_relation_from_order(scop: &ScopRegion, dims: Vec<String>) -> ScheduleRelation {
    ScheduleRelation {
        input_dimensions: scop
            .dimensions
            .iter()
            .map(|dim| dim.iv_name.clone())
            .collect(),
        output_expressions: dims
            .into_iter()
            .map(|name| AffineExpr::symbol(AffineSymbol::LoopIv(name)))
            .collect(),
    }
}

fn interchange_relations(scop: &ScopRegion) -> Vec<ScheduleRelation> {
    let dims = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.clone())
        .collect::<Vec<_>>();
    match dims.as_slice() {
        [a, b] => vec![interchange_relation_from_order(
            scop,
            vec![b.clone(), a.clone()],
        )],
        [a, b, c] => vec![
            interchange_relation_from_order(scop, vec![b.clone(), c.clone(), a.clone()]),
            interchange_relation_from_order(scop, vec![c.clone(), a.clone(), b.clone()]),
            interchange_relation_from_order(scop, vec![a.clone(), c.clone(), b.clone()]),
            interchange_relation_from_order(scop, vec![b.clone(), a.clone(), c.clone()]),
            interchange_relation_from_order(scop, vec![c.clone(), b.clone(), a.clone()]),
        ],
        _ => Vec::new(),
    }
}

fn none_relation() -> ScheduleRelation {
    ScheduleRelation {
        input_dimensions: Vec::new(),
        output_expressions: Vec::new(),
    }
}

fn skew2d_relation(scop: &ScopRegion) -> ScheduleRelation {
    let input_dimensions = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.clone())
        .collect::<Vec<_>>();
    if input_dimensions.len() != 2 {
        return none_relation();
    }
    let outer = AffineExpr::symbol(AffineSymbol::LoopIv(input_dimensions[0].clone()));
    let mut skewed = AffineExpr::symbol(AffineSymbol::LoopIv(input_dimensions[1].clone()));
    skewed.add_assign(
        &AffineExpr::symbol(AffineSymbol::LoopIv(input_dimensions[0].clone())),
        1,
    );
    ScheduleRelation {
        input_dimensions,
        output_expressions: vec![outer, skewed],
    }
}

fn tile_policy_from_env() -> TilePolicy {
    let enable_1d = std::env::var("RR_POLY_TILE_1D").ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    let tile_size = std::env::var("RR_POLY_TILE_SIZE")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(64);
    let skew_2d_mode = match std::env::var("RR_POLY_SKEW_2D")
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        Some("1" | "true" | "yes" | "on") => AutoChoice::ForceOn,
        Some("0" | "false" | "no" | "off") => AutoChoice::ForceOff,
        Some("auto") | None => AutoChoice::Auto,
        Some(_) => AutoChoice::Auto,
    };
    let enable_2d = std::env::var("RR_POLY_TILE_2D").ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    let enable_3d = std::env::var("RR_POLY_TILE_3D").ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    });
    let tile_depth = std::env::var("RR_POLY_TILE_DEPTH")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(4);
    let tile_rows = std::env::var("RR_POLY_TILE_ROWS")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(8);
    let tile_cols = std::env::var("RR_POLY_TILE_COLS")
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(8);
    TilePolicy {
        enable_1d,
        skew_2d_mode,
        allow_skew_with_tiles: false,
        tile_size,
        enable_2d,
        enable_3d,
        tile_depth,
        tile_rows,
        tile_cols,
    }
}

fn solver_tile_policy(requested: PolyBackendKind, base: TilePolicy) -> TilePolicy {
    if requested != PolyBackendKind::Isl {
        return base;
    }
    TilePolicy {
        enable_1d: true,
        enable_2d: true,
        enable_3d: true,
        allow_skew_with_tiles: true,
        skew_2d_mode: match base.skew_2d_mode {
            AutoChoice::ForceOff => AutoChoice::ForceOff,
            AutoChoice::ForceOn | AutoChoice::Auto => AutoChoice::Auto,
        },
        ..base
    }
}

fn dedup_positive_usizes(values: impl IntoIterator<Item = usize>) -> Vec<usize> {
    let mut out = values
        .into_iter()
        .filter(|value| *value > 0)
        .collect::<Vec<_>>();
    out.sort_unstable();
    out.dedup();
    out
}

fn solver_tile1d_variants(policy: TilePolicy, requested: PolyBackendKind) -> Vec<usize> {
    if requested != PolyBackendKind::Isl {
        return vec![policy.tile_size];
    }
    dedup_positive_usizes([policy.tile_size / 2, policy.tile_size, policy.tile_size * 2])
}

fn solver_tile2d_variants(policy: TilePolicy, requested: PolyBackendKind) -> Vec<(usize, usize)> {
    if requested != PolyBackendKind::Isl {
        return vec![(policy.tile_rows, policy.tile_cols)];
    }
    let row_variants =
        dedup_positive_usizes([policy.tile_rows / 2, policy.tile_rows, policy.tile_rows * 2]);
    let col_variants =
        dedup_positive_usizes([policy.tile_cols / 2, policy.tile_cols, policy.tile_cols * 2]);
    let mut out = Vec::new();
    for rows in &row_variants {
        for cols in &col_variants {
            out.push((*rows, *cols));
        }
    }
    out
}

fn solver_tile3d_variants(
    policy: TilePolicy,
    requested: PolyBackendKind,
) -> Vec<(usize, usize, usize)> {
    if requested != PolyBackendKind::Isl {
        return vec![(policy.tile_depth, policy.tile_rows, policy.tile_cols)];
    }
    let depth_variants = dedup_positive_usizes([
        policy.tile_depth / 2,
        policy.tile_depth,
        policy.tile_depth * 2,
    ]);
    let row_variants =
        dedup_positive_usizes([policy.tile_rows / 2, policy.tile_rows, policy.tile_rows * 2]);
    let col_variants =
        dedup_positive_usizes([policy.tile_cols / 2, policy.tile_cols, policy.tile_cols * 2]);
    let mut out = Vec::new();
    for depth in &depth_variants {
        for rows in &row_variants {
            for cols in &col_variants {
                out.push((*depth, *rows, *cols));
            }
        }
    }
    out
}

fn can_auto_skew_2d(scop: &ScopRegion, dep_state: DependenceState, policy: TilePolicy) -> bool {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count();
    if scop.dimensions.len() != 2
        || data_stmt_count <= 1
        || scop.dimensions.iter().any(|dim| dim.step != 1)
        || !matches!(dep_state, DependenceState::IdentityProven)
        || (!policy.allow_skew_with_tiles
            && (policy.enable_1d || policy.enable_2d || policy.enable_3d))
    {
        return false;
    }
    let loop_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<Vec<_>>();
    scop.statements
        .iter()
        .flat_map(|stmt| &stmt.accesses)
        .all(|access| {
            access.memref.layout == MemoryLayout::ColumnMajor2D
                && access_matches_loop_order(access, &loop_names)
        })
}

fn can_skew_2d(scop: &ScopRegion, dep_state: DependenceState, policy: TilePolicy) -> bool {
    match policy.skew_2d_mode {
        AutoChoice::ForceOff => false,
        AutoChoice::ForceOn => {
            scop.dimensions.len() == 2
                && scop.dimensions.iter().all(|dim| dim.step == 1)
                && scop.statements.iter().any(|stmt| !stmt.accesses.is_empty())
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| &stmt.accesses)
                    .all(|access| access.memref.layout == MemoryLayout::ColumnMajor2D)
        }
        AutoChoice::Auto => can_auto_skew_2d(scop, dep_state, policy),
    }
}

fn should_interchange(scop: &ScopRegion) -> bool {
    if !(scop.dimensions.len() == 2 || scop.dimensions.len() == 3) {
        return false;
    }
    let loop_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<Vec<_>>();
    let outer = &scop.dimensions[0].iv_name;
    let inner = &scop.dimensions[1].iv_name;
    if scop.dimensions.len() == 2 {
        return scop
            .statements
            .iter()
            .flat_map(|stmt| &stmt.accesses)
            .any(|access| {
                access.memref.layout == MemoryLayout::ColumnMajor2D
                    && access_matches_loop_order(access, &loop_names)
                    && access.subscripts.len() == 2
                    && matches!(
                        access.subscripts[0].terms.iter().next(),
                        Some((AffineSymbol::LoopIv(name), coeff)) if name == outer && *coeff == 1
                    )
                    && matches!(
                        access.subscripts[1].terms.iter().next(),
                        Some((AffineSymbol::LoopIv(name), coeff)) if name == inner && *coeff == 1
                    )
            });
    }

    let middle = &scop.dimensions[1].iv_name;
    let inner = &scop.dimensions[2].iv_name;
    scop.statements
        .iter()
        .flat_map(|stmt| &stmt.accesses)
        .any(|access| {
            access.memref.layout == MemoryLayout::ColumnMajor3D
                && access_matches_loop_order(access, &loop_names)
                && access.subscripts.len() == 3
                && matches!(
                    access.subscripts[0].terms.iter().next(),
                    Some((AffineSymbol::LoopIv(name), coeff)) if name == outer && *coeff == 1
                )
                && matches!(
                    access.subscripts[1].terms.iter().next(),
                    Some((AffineSymbol::LoopIv(name), coeff)) if name == middle && *coeff == 1
                )
                && matches!(
                    access.subscripts[2].terms.iter().next(),
                    Some((AffineSymbol::LoopIv(name), coeff)) if name == inner && *coeff == 1
                )
        })
}

fn access_matches_loop_order(
    access: &crate::mir::opt::poly::access::AccessRelation,
    loop_names: &[&str],
) -> bool {
    fn expr_is_loop_aligned(expr: &AffineExpr, expected: &str) -> bool {
        let mut expected_coeff = None;
        for (symbol, coeff) in &expr.terms {
            if let AffineSymbol::LoopIv(name) = symbol {
                if name == expected {
                    if expected_coeff.is_some() {
                        return false;
                    }
                    expected_coeff = Some(*coeff);
                } else {
                    return false;
                }
            }
        }
        expected_coeff == Some(1)
    }

    access.subscripts.len() == loop_names.len()
        && access
            .subscripts
            .iter()
            .zip(loop_names.iter())
            .all(|(expr, expected)| expr_is_loop_aligned(expr, expected))
}

fn can_tile_1d(scop: &ScopRegion, policy: TilePolicy) -> bool {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count();
    let iv_name = &scop.dimensions[0].iv_name;
    policy.enable_1d
        && scop.dimensions.len() == 1
        && scop.dimensions[0].step == 1
        && data_stmt_count >= 1
        && scop
            .statements
            .iter()
            .flat_map(|stmt| &stmt.accesses)
            .all(|access| match access.memref.layout {
                MemoryLayout::Dense1D => access.subscripts.len() == 1,
                MemoryLayout::ColumnMajor2D => {
                    access.subscripts.len() == 2
                        && matches!(
                            access.subscripts[0].terms.iter().next(),
                            Some((AffineSymbol::LoopIv(name), coeff)) if name == iv_name && *coeff == 1
                        )
                }
                MemoryLayout::ColumnMajor3D => {
                    access.subscripts.len() == 3
                        && matches!(
                            access.subscripts[0].terms.iter().next(),
                            Some((AffineSymbol::LoopIv(name), coeff)) if name == iv_name && *coeff == 1
                        )
                }
            })
}

fn can_tile_2d(scop: &ScopRegion, policy: TilePolicy) -> bool {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count();
    policy.enable_2d
        && scop.dimensions.len() == 2
        && scop.dimensions[0].step == 1
        && scop.dimensions[1].step == 1
        && data_stmt_count >= 1
        && {
            let loop_names = scop
                .dimensions
                .iter()
                .map(|dim| dim.iv_name.as_str())
                .collect::<Vec<_>>();
            scop.statements
                .iter()
                .flat_map(|stmt| &stmt.accesses)
                .all(|access| {
                    access.memref.layout == MemoryLayout::ColumnMajor2D
                        && access_matches_loop_order(access, &loop_names)
                })
        }
}

fn can_tile_3d(scop: &ScopRegion, policy: TilePolicy) -> bool {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count();
    policy.enable_3d
        && scop.dimensions.len() == 3
        && scop.dimensions.iter().all(|dim| dim.step == 1)
        && data_stmt_count >= 1
        && {
            let loop_names = scop
                .dimensions
                .iter()
                .map(|dim| dim.iv_name.as_str())
                .collect::<Vec<_>>();
            scop.statements
                .iter()
                .flat_map(|stmt| &stmt.accesses)
                .all(|access| {
                    access.memref.layout == MemoryLayout::ColumnMajor3D
                        && access_matches_loop_order(access, &loop_names)
                })
        }
}

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

fn make_plan(
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

fn candidate_plans_with_policy(
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

fn search_schedule_with_policy(
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

fn solver_candidate_state_for_result(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::opt::poly::access::{AccessKind, AccessRelation, MemRef, MemoryLayout};
    use crate::mir::opt::poly::affine::{AffineConstraint, AffineConstraintKind, PresburgerSet};
    use crate::mir::opt::poly::dependence_backend::{
        DependenceRelation, DependenceResult, DependenceState, DependenceSummary,
    };
    use crate::mir::opt::poly::{LoopDimension, PolyStmt, PolyStmtKind};

    fn loop_iv(name: &str) -> AffineExpr {
        AffineExpr::symbol(AffineSymbol::LoopIv(name.to_string()))
    }

    #[test]
    fn search_prefers_interchange_for_row_major_nest_on_column_major_matrix() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
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
                        expr: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
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
                    subscripts: vec![loop_iv("r"), loop_iv("c")],
                }],
            }],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 1,
            access_count: 1,
            reduction_count: 0,
        };
        let plan = search_schedule(&scop, &deps);
        assert_eq!(plan.kind, SchedulePlanKind::Interchange);
    }

    #[test]
    fn search_keeps_identity_when_interchange_does_not_improve_cost() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "i".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "j".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(vec!["i".to_string(), "j".to_string()], vec![]),
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
                        name: "x".to_string(),
                        rank: 1,
                        layout: MemoryLayout::Dense1D,
                    },
                    subscripts: vec![loop_iv("i")],
                }],
            }],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 1,
            access_count: 1,
            reduction_count: 0,
        };
        let plan = search_schedule(&scop, &deps);
        assert_eq!(plan.kind, SchedulePlanKind::Identity);
    }

    #[test]
    fn search_prefers_interchange_for_row_major_3d_nest_on_column_major_array() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "i".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "j".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "k".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("p".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(
                vec!["i".to_string(), "j".to_string(), "k".to_string()],
                vec![],
            ),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0, 1, 2],
                },
                expr_root: None,
                accesses: vec![AccessRelation {
                    statement_id: 0,
                    kind: AccessKind::Write,
                    memref: MemRef {
                        base: 1,
                        name: "A".to_string(),
                        rank: 3,
                        layout: MemoryLayout::ColumnMajor3D,
                    },
                    subscripts: vec![loop_iv("i"), loop_iv("j"), loop_iv("k")],
                }],
            }],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 1,
            access_count: 1,
            reduction_count: 0,
        };
        let plan = search_schedule(&scop, &deps);
        assert_eq!(plan.kind, SchedulePlanKind::Interchange);
        assert_eq!(plan.relation.output_expressions.len(), 3);
    }

    #[test]
    fn parse_backend_name_defaults_to_heuristic() {
        assert_eq!(parse_backend_name(""), PolyBackendKind::Heuristic);
        assert_eq!(parse_backend_name("weird"), PolyBackendKind::Heuristic);
        assert_eq!(parse_backend_name("isl"), PolyBackendKind::Isl);
    }

    #[test]
    fn search_selects_tile1d_when_enabled_for_dense_1d_scop() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![LoopDimension {
                iv_name: "i".to_string(),
                lower_bound: AffineExpr::constant(2),
                upper_bound: AffineExpr::symbol(AffineSymbol::Length("x".to_string())),
                step: 1,
            }],
            iteration_space: PresburgerSet::new(vec!["i".to_string()], vec![]),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0],
                },
                expr_root: None,
                accesses: vec![
                    AccessRelation {
                        statement_id: 0,
                        kind: AccessKind::Write,
                        memref: MemRef {
                            base: 1,
                            name: "y".to_string(),
                            rank: 1,
                            layout: MemoryLayout::Dense1D,
                        },
                        subscripts: vec![loop_iv("i")],
                    },
                    AccessRelation {
                        statement_id: 0,
                        kind: AccessKind::Read,
                        memref: MemRef {
                            base: 2,
                            name: "x".to_string(),
                            rank: 1,
                            layout: MemoryLayout::Dense1D,
                        },
                        subscripts: vec![loop_iv("i")],
                    },
                ],
            }],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 1,
            access_count: 2,
            reduction_count: 0,
        };
        let plan = search_schedule_with_policy(
            &scop,
            deps.state,
            PolyBackendKind::Heuristic,
            TilePolicy {
                enable_1d: true,
                skew_2d_mode: AutoChoice::ForceOff,
                allow_skew_with_tiles: false,
                tile_size: 8,
                enable_2d: false,
                enable_3d: false,
                tile_depth: 0,
                tile_rows: 0,
                tile_cols: 0,
            },
        );
        assert_eq!(plan.kind, SchedulePlanKind::Tile1D);
        assert_eq!(plan.tile_size, Some(8));
    }

    #[test]
    fn search_selects_tile1d_for_dense_fused_1d_scop() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![LoopDimension {
                iv_name: "i".to_string(),
                lower_bound: AffineExpr::constant(1),
                upper_bound: AffineExpr::symbol(AffineSymbol::Length("x".to_string())),
                step: 1,
            }],
            iteration_space: PresburgerSet::new(vec!["i".to_string()], vec![]),
            parameters: Default::default(),
            statements: vec![
                PolyStmt {
                    id: 0,
                    block: 0,
                    kind: PolyStmtKind::Store {
                        base: 1,
                        subscripts: vec![0],
                    },
                    expr_root: None,
                    accesses: vec![
                        AccessRelation {
                            statement_id: 0,
                            kind: AccessKind::Write,
                            memref: MemRef {
                                base: 1,
                                name: "y".to_string(),
                                rank: 1,
                                layout: MemoryLayout::Dense1D,
                            },
                            subscripts: vec![loop_iv("i")],
                        },
                        AccessRelation {
                            statement_id: 0,
                            kind: AccessKind::Read,
                            memref: MemRef {
                                base: 2,
                                name: "x".to_string(),
                                rank: 1,
                                layout: MemoryLayout::Dense1D,
                            },
                            subscripts: vec![loop_iv("i")],
                        },
                    ],
                },
                PolyStmt {
                    id: 1,
                    block: 0,
                    kind: PolyStmtKind::Store {
                        base: 3,
                        subscripts: vec![0],
                    },
                    expr_root: None,
                    accesses: vec![
                        AccessRelation {
                            statement_id: 1,
                            kind: AccessKind::Write,
                            memref: MemRef {
                                base: 3,
                                name: "z".to_string(),
                                rank: 1,
                                layout: MemoryLayout::Dense1D,
                            },
                            subscripts: vec![loop_iv("i")],
                        },
                        AccessRelation {
                            statement_id: 1,
                            kind: AccessKind::Read,
                            memref: MemRef {
                                base: 4,
                                name: "w".to_string(),
                                rank: 1,
                                layout: MemoryLayout::Dense1D,
                            },
                            subscripts: vec![loop_iv("i")],
                        },
                    ],
                },
            ],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 2,
            access_count: 4,
            reduction_count: 0,
        };
        let plan = search_schedule_with_policy(
            &scop,
            deps.state,
            PolyBackendKind::Heuristic,
            TilePolicy {
                enable_1d: true,
                skew_2d_mode: AutoChoice::ForceOff,
                allow_skew_with_tiles: false,
                tile_size: 8,
                enable_2d: false,
                enable_3d: false,
                tile_depth: 0,
                tile_rows: 0,
                tile_cols: 0,
            },
        );
        assert_eq!(plan.kind, SchedulePlanKind::Tile1D);
        assert_eq!(plan.tile_size, Some(8));
    }

    #[test]
    fn search_selects_tile2d_when_enabled_for_dense_2d_scop() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(vec!["r".to_string(), "c".to_string()], vec![]),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0, 1],
                },
                expr_root: None,
                accesses: vec![
                    AccessRelation {
                        statement_id: 0,
                        kind: AccessKind::Write,
                        memref: MemRef {
                            base: 1,
                            name: "A".to_string(),
                            rank: 2,
                            layout: MemoryLayout::ColumnMajor2D,
                        },
                        subscripts: vec![loop_iv("r"), loop_iv("c")],
                    },
                    AccessRelation {
                        statement_id: 0,
                        kind: AccessKind::Read,
                        memref: MemRef {
                            base: 2,
                            name: "B".to_string(),
                            rank: 2,
                            layout: MemoryLayout::ColumnMajor2D,
                        },
                        subscripts: vec![loop_iv("r"), loop_iv("c")],
                    },
                ],
            }],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 1,
            access_count: 2,
            reduction_count: 0,
        };
        let plan = search_schedule_with_policy(
            &scop,
            deps.state,
            PolyBackendKind::Heuristic,
            TilePolicy {
                enable_1d: false,
                skew_2d_mode: AutoChoice::ForceOff,
                allow_skew_with_tiles: false,
                tile_size: 0,
                enable_2d: true,
                enable_3d: false,
                tile_depth: 0,
                tile_rows: 4,
                tile_cols: 5,
            },
        );
        assert_eq!(plan.kind, SchedulePlanKind::Tile2D);
        assert_eq!(plan.tile_rows, Some(4));
        assert_eq!(plan.tile_cols, Some(5));
    }

    #[test]
    fn search_selects_skew2d_when_enabled_for_dense_2d_scop() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(vec!["r".to_string(), "c".to_string()], vec![]),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0, 1],
                },
                expr_root: None,
                accesses: vec![
                    AccessRelation {
                        statement_id: 0,
                        kind: AccessKind::Write,
                        memref: MemRef {
                            base: 1,
                            name: "A".to_string(),
                            rank: 2,
                            layout: MemoryLayout::ColumnMajor2D,
                        },
                        subscripts: vec![loop_iv("r"), loop_iv("c")],
                    },
                    AccessRelation {
                        statement_id: 0,
                        kind: AccessKind::Read,
                        memref: MemRef {
                            base: 2,
                            name: "B".to_string(),
                            rank: 2,
                            layout: MemoryLayout::ColumnMajor2D,
                        },
                        subscripts: vec![loop_iv("r"), loop_iv("c")],
                    },
                ],
            }],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 1,
            access_count: 2,
            reduction_count: 0,
        };
        let plan = search_schedule_with_policy(
            &scop,
            deps.state,
            PolyBackendKind::Heuristic,
            TilePolicy {
                enable_1d: false,
                skew_2d_mode: AutoChoice::ForceOn,
                allow_skew_with_tiles: false,
                tile_size: 0,
                enable_2d: false,
                enable_3d: false,
                tile_depth: 0,
                tile_rows: 0,
                tile_cols: 0,
            },
        );
        assert_eq!(plan.kind, SchedulePlanKind::Skew2D);
    }

    #[test]
    fn search_selects_tile3d_when_enabled_for_dense_3d_scop() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "i".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "j".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "k".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("p".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(
                vec!["i".to_string(), "j".to_string(), "k".to_string()],
                vec![],
            ),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0, 1, 2],
                },
                expr_root: None,
                accesses: vec![
                    AccessRelation {
                        statement_id: 0,
                        kind: AccessKind::Write,
                        memref: MemRef {
                            base: 1,
                            name: "A".to_string(),
                            rank: 3,
                            layout: MemoryLayout::ColumnMajor3D,
                        },
                        subscripts: vec![loop_iv("i"), loop_iv("j"), loop_iv("k")],
                    },
                    AccessRelation {
                        statement_id: 0,
                        kind: AccessKind::Read,
                        memref: MemRef {
                            base: 2,
                            name: "B".to_string(),
                            rank: 3,
                            layout: MemoryLayout::ColumnMajor3D,
                        },
                        subscripts: vec![loop_iv("i"), loop_iv("j"), loop_iv("k")],
                    },
                ],
            }],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 1,
            access_count: 2,
            reduction_count: 0,
        };
        let plan = search_schedule_with_policy(
            &scop,
            deps.state,
            PolyBackendKind::Heuristic,
            TilePolicy {
                enable_1d: false,
                skew_2d_mode: AutoChoice::ForceOff,
                allow_skew_with_tiles: false,
                tile_size: 0,
                enable_2d: false,
                enable_3d: true,
                tile_depth: 3,
                tile_rows: 4,
                tile_cols: 5,
            },
        );
        assert_eq!(plan.kind, SchedulePlanKind::Tile3D);
        assert_eq!(plan.tile_depth, Some(3));
        assert_eq!(plan.tile_rows, Some(4));
        assert_eq!(plan.tile_cols, Some(5));
    }

    #[test]
    fn search_keeps_identity_for_tiny_1d_scop_even_when_tiling_enabled() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![LoopDimension {
                iv_name: "i".to_string(),
                lower_bound: AffineExpr::constant(1),
                upper_bound: AffineExpr::constant(2),
                step: 1,
            }],
            iteration_space: PresburgerSet::new(vec!["i".to_string()], vec![]),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0],
                },
                expr_root: None,
                accesses: vec![AccessRelation {
                    statement_id: 0,
                    kind: AccessKind::Write,
                    memref: MemRef {
                        base: 1,
                        name: "x".to_string(),
                        rank: 1,
                        layout: MemoryLayout::Dense1D,
                    },
                    subscripts: vec![loop_iv("i")],
                }],
            }],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 1,
            access_count: 1,
            reduction_count: 0,
        };
        let plan = search_schedule_with_policy(
            &scop,
            deps.state,
            PolyBackendKind::Heuristic,
            TilePolicy {
                enable_1d: true,
                skew_2d_mode: AutoChoice::ForceOff,
                allow_skew_with_tiles: false,
                tile_size: 8,
                enable_2d: false,
                enable_3d: false,
                tile_depth: 0,
                tile_rows: 0,
                tile_cols: 0,
            },
        );
        assert_eq!(plan.kind, SchedulePlanKind::Identity);
    }

    #[test]
    fn search_avoids_tile3d_for_tiny_3d_scop_even_when_tiling_enabled() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "i".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(1),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "j".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(1),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "k".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(1),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(
                vec!["i".to_string(), "j".to_string(), "k".to_string()],
                vec![],
            ),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0, 1, 2],
                },
                expr_root: None,
                accesses: vec![AccessRelation {
                    statement_id: 0,
                    kind: AccessKind::Write,
                    memref: MemRef {
                        base: 1,
                        name: "A".to_string(),
                        rank: 3,
                        layout: MemoryLayout::ColumnMajor3D,
                    },
                    subscripts: vec![loop_iv("i"), loop_iv("j"), loop_iv("k")],
                }],
            }],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 1,
            access_count: 1,
            reduction_count: 0,
        };
        let plan = search_schedule_with_policy(
            &scop,
            deps.state,
            PolyBackendKind::Heuristic,
            TilePolicy {
                enable_1d: false,
                skew_2d_mode: AutoChoice::ForceOff,
                allow_skew_with_tiles: false,
                tile_size: 0,
                enable_2d: false,
                enable_3d: true,
                tile_depth: 4,
                tile_rows: 4,
                tile_cols: 4,
            },
        );
        assert_ne!(plan.kind, SchedulePlanKind::Tile3D);
    }

    #[test]
    fn search_does_not_select_tile3d_for_non_full_cube_3d_scop() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "i".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "j".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "k".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("p".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(
                vec!["i".to_string(), "j".to_string(), "k".to_string()],
                vec![],
            ),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0, 1, 2],
                },
                expr_root: None,
                accesses: vec![AccessRelation {
                    statement_id: 0,
                    kind: AccessKind::Write,
                    memref: MemRef {
                        base: 1,
                        name: "A".to_string(),
                        rank: 3,
                        layout: MemoryLayout::ColumnMajor3D,
                    },
                    subscripts: vec![
                        loop_iv("i"),
                        AffineExpr::constant(2),
                        AffineExpr::constant(3),
                    ],
                }],
            }],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 1,
            access_count: 1,
            reduction_count: 0,
        };
        let plan = search_schedule_with_policy(
            &scop,
            deps.state,
            PolyBackendKind::Heuristic,
            TilePolicy {
                enable_1d: false,
                skew_2d_mode: AutoChoice::ForceOff,
                allow_skew_with_tiles: false,
                tile_size: 0,
                enable_2d: false,
                enable_3d: true,
                tile_depth: 4,
                tile_rows: 4,
                tile_cols: 4,
            },
        );
        assert_eq!(plan.kind, SchedulePlanKind::Identity);
    }

    #[test]
    fn search_auto_selects_skew2d_for_dense_fused_2d_scop() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(vec!["r".to_string(), "c".to_string()], vec![]),
            parameters: Default::default(),
            statements: vec![
                PolyStmt {
                    id: 0,
                    block: 0,
                    kind: PolyStmtKind::Store {
                        base: 1,
                        subscripts: vec![0, 1],
                    },
                    expr_root: None,
                    accesses: vec![
                        AccessRelation {
                            statement_id: 0,
                            kind: AccessKind::Write,
                            memref: MemRef {
                                base: 1,
                                name: "A".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![loop_iv("r"), loop_iv("c")],
                        },
                        AccessRelation {
                            statement_id: 0,
                            kind: AccessKind::Read,
                            memref: MemRef {
                                base: 2,
                                name: "B".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![loop_iv("r"), loop_iv("c")],
                        },
                    ],
                },
                PolyStmt {
                    id: 1,
                    block: 0,
                    kind: PolyStmtKind::Store {
                        base: 3,
                        subscripts: vec![0, 1],
                    },
                    expr_root: None,
                    accesses: vec![
                        AccessRelation {
                            statement_id: 1,
                            kind: AccessKind::Write,
                            memref: MemRef {
                                base: 3,
                                name: "C".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![loop_iv("r"), loop_iv("c")],
                        },
                        AccessRelation {
                            statement_id: 1,
                            kind: AccessKind::Read,
                            memref: MemRef {
                                base: 4,
                                name: "D".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![loop_iv("r"), loop_iv("c")],
                        },
                    ],
                },
            ],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 2,
            access_count: 4,
            reduction_count: 0,
        };
        let plan = search_schedule_with_policy(
            &scop,
            deps.state,
            PolyBackendKind::Heuristic,
            TilePolicy {
                enable_1d: false,
                skew_2d_mode: AutoChoice::Auto,
                allow_skew_with_tiles: false,
                tile_size: 0,
                enable_2d: false,
                enable_3d: false,
                tile_depth: 0,
                tile_rows: 0,
                tile_cols: 0,
            },
        );
        assert_eq!(plan.kind, SchedulePlanKind::Skew2D);
    }

    #[test]
    fn search_auto_selects_skew2d_for_offset_dense_fused_2d_scop() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(vec!["r".to_string(), "c".to_string()], vec![]),
            parameters: Default::default(),
            statements: vec![
                PolyStmt {
                    id: 0,
                    block: 0,
                    kind: PolyStmtKind::Store {
                        base: 1,
                        subscripts: vec![0, 1],
                    },
                    expr_root: None,
                    accesses: vec![
                        AccessRelation {
                            statement_id: 0,
                            kind: AccessKind::Write,
                            memref: MemRef {
                                base: 1,
                                name: "A".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![
                                AffineExpr {
                                    constant: 1,
                                    terms: [(AffineSymbol::LoopIv("r".to_string()), 1)]
                                        .into_iter()
                                        .collect(),
                                },
                                loop_iv("c"),
                            ],
                        },
                        AccessRelation {
                            statement_id: 0,
                            kind: AccessKind::Read,
                            memref: MemRef {
                                base: 2,
                                name: "B".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![
                                AffineExpr {
                                    constant: 1,
                                    terms: [(AffineSymbol::LoopIv("r".to_string()), 1)]
                                        .into_iter()
                                        .collect(),
                                },
                                loop_iv("c"),
                            ],
                        },
                    ],
                },
                PolyStmt {
                    id: 1,
                    block: 0,
                    kind: PolyStmtKind::Store {
                        base: 3,
                        subscripts: vec![0, 1],
                    },
                    expr_root: None,
                    accesses: vec![
                        AccessRelation {
                            statement_id: 1,
                            kind: AccessKind::Write,
                            memref: MemRef {
                                base: 3,
                                name: "C".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![
                                AffineExpr {
                                    constant: 1,
                                    terms: [(AffineSymbol::LoopIv("r".to_string()), 1)]
                                        .into_iter()
                                        .collect(),
                                },
                                loop_iv("c"),
                            ],
                        },
                        AccessRelation {
                            statement_id: 1,
                            kind: AccessKind::Read,
                            memref: MemRef {
                                base: 4,
                                name: "D".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![
                                AffineExpr {
                                    constant: 1,
                                    terms: [(AffineSymbol::LoopIv("r".to_string()), 1)]
                                        .into_iter()
                                        .collect(),
                                },
                                loop_iv("c"),
                            ],
                        },
                    ],
                },
            ],
        };
        let deps = DependenceSummary {
            state: DependenceState::IdentityProven,
            write_count: 2,
            access_count: 4,
            reduction_count: 0,
        };
        let plan = search_schedule_with_policy(
            &scop,
            deps.state,
            PolyBackendKind::Heuristic,
            TilePolicy {
                enable_1d: false,
                skew_2d_mode: AutoChoice::Auto,
                allow_skew_with_tiles: false,
                tile_size: 0,
                enable_2d: false,
                enable_3d: false,
                tile_depth: 0,
                tile_rows: 0,
                tile_cols: 0,
            },
        );
        assert_eq!(plan.kind, SchedulePlanKind::Skew2D);
    }

    #[test]
    fn isl_candidate_enumeration_keeps_skew2d_open_alongside_solver_tiles() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(vec!["r".to_string(), "c".to_string()], vec![]),
            parameters: Default::default(),
            statements: vec![
                PolyStmt {
                    id: 0,
                    block: 0,
                    kind: PolyStmtKind::Store {
                        base: 1,
                        subscripts: vec![0, 1],
                    },
                    expr_root: None,
                    accesses: vec![
                        AccessRelation {
                            statement_id: 0,
                            kind: AccessKind::Write,
                            memref: MemRef {
                                base: 1,
                                name: "A".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![loop_iv("r"), loop_iv("c")],
                        },
                        AccessRelation {
                            statement_id: 0,
                            kind: AccessKind::Read,
                            memref: MemRef {
                                base: 2,
                                name: "B".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![loop_iv("r"), loop_iv("c")],
                        },
                    ],
                },
                PolyStmt {
                    id: 1,
                    block: 0,
                    kind: PolyStmtKind::Store {
                        base: 3,
                        subscripts: vec![0, 1],
                    },
                    expr_root: None,
                    accesses: vec![
                        AccessRelation {
                            statement_id: 1,
                            kind: AccessKind::Write,
                            memref: MemRef {
                                base: 3,
                                name: "C".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![loop_iv("r"), loop_iv("c")],
                        },
                        AccessRelation {
                            statement_id: 1,
                            kind: AccessKind::Read,
                            memref: MemRef {
                                base: 4,
                                name: "D".to_string(),
                                rank: 2,
                                layout: MemoryLayout::ColumnMajor2D,
                            },
                            subscripts: vec![loop_iv("r"), loop_iv("c")],
                        },
                    ],
                },
            ],
        };
        let deps = DependenceResult {
            summary: DependenceSummary {
                state: DependenceState::IdentityProven,
                write_count: 2,
                access_count: 4,
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

        let plans = candidate_schedules_for_backend_result(&scop, &deps, PolyBackendKind::Isl);
        assert!(
            plans
                .iter()
                .any(|plan| plan.kind == SchedulePlanKind::Skew2D)
        );
        assert!(
            plans
                .iter()
                .any(|plan| plan.kind == SchedulePlanKind::Tile2D)
        );
    }

    #[test]
    fn isl_candidate_enumeration_includes_multiple_tile2d_variants() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(64),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(64),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(vec!["r".to_string(), "c".to_string()], vec![]),
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
                    subscripts: vec![loop_iv("r"), loop_iv("c")],
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
        let plans = candidate_schedules_for_backend_result(&scop, &deps, PolyBackendKind::Isl);
        let tile_variants = plans
            .iter()
            .filter(|plan| plan.kind == SchedulePlanKind::Tile2D)
            .map(|plan| (plan.tile_rows, plan.tile_cols))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(tile_variants.len() >= 2, "got variants: {tile_variants:?}");
    }

    #[test]
    fn isl_candidate_enumeration_includes_multiple_tile3d_variants() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "i".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(32),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "j".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(32),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "k".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::constant(32),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(
                vec!["i".to_string(), "j".to_string(), "k".to_string()],
                vec![],
            ),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0, 1, 2],
                },
                expr_root: None,
                accesses: vec![AccessRelation {
                    statement_id: 0,
                    kind: AccessKind::Write,
                    memref: MemRef {
                        base: 1,
                        name: "A".to_string(),
                        rank: 3,
                        layout: MemoryLayout::ColumnMajor3D,
                    },
                    subscripts: vec![loop_iv("i"), loop_iv("j"), loop_iv("k")],
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
                iteration_dimensions: vec!["i".to_string(), "j".to_string(), "k".to_string()],
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
        let plans = candidate_schedules_for_backend_result(&scop, &deps, PolyBackendKind::Isl);
        let tile_variants = plans
            .iter()
            .filter(|plan| plan.kind == SchedulePlanKind::Tile3D)
            .map(|plan| (plan.tile_depth, plan.tile_rows, plan.tile_cols))
            .collect::<std::collections::BTreeSet<_>>();
        assert!(tile_variants.len() >= 2, "got variants: {tile_variants:?}");
    }

    #[test]
    fn isl_candidate_enumeration_includes_all_3d_interchange_permutations() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "i".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "j".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "k".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("p".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(
                vec!["i".to_string(), "j".to_string(), "k".to_string()],
                vec![],
            ),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base: 1,
                    subscripts: vec![0, 1, 2],
                },
                expr_root: None,
                accesses: vec![AccessRelation {
                    statement_id: 0,
                    kind: AccessKind::Write,
                    memref: MemRef {
                        base: 1,
                        name: "A".to_string(),
                        rank: 3,
                        layout: MemoryLayout::ColumnMajor3D,
                    },
                    subscripts: vec![loop_iv("i"), loop_iv("j"), loop_iv("k")],
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
                iteration_dimensions: vec!["i".to_string(), "j".to_string(), "k".to_string()],
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
        let interchange_count =
            candidate_schedules_for_backend_result(&scop, &deps, PolyBackendKind::Isl)
                .into_iter()
                .filter(|plan| plan.kind == SchedulePlanKind::Interchange)
                .count();
        assert_eq!(interchange_count, 5);
    }

    #[test]
    fn isl_candidate_enumeration_stays_open_when_validity_relation_exists() {
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![
                LoopDimension {
                    iv_name: "r".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                    step: 1,
                },
                LoopDimension {
                    iv_name: "c".to_string(),
                    lower_bound: AffineExpr::constant(1),
                    upper_bound: AffineExpr::symbol(AffineSymbol::Length("m".to_string())),
                    step: 1,
                },
            ],
            iteration_space: PresburgerSet::new(vec!["r".to_string(), "c".to_string()], vec![]),
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
                    subscripts: vec![loop_iv("r"), loop_iv("c")],
                }],
            }],
        };
        let deps = DependenceResult {
            summary: DependenceSummary {
                state: DependenceState::Unknown,
                write_count: 1,
                access_count: 1,
                reduction_count: 0,
            },
            relation: DependenceRelation {
                iteration_dimensions: vec!["r".to_string(), "c".to_string()],
                edge_count: 1,
                raw_relation: Some("{ S0[r, c] -> S0[r + 1, c] }".to_string()),
                war_relation: None,
                waw_relation: None,
                reduction_relation: None,
                validity_relation: Some("{ S0[r, c] -> S0[r + 1, c] }".to_string()),
                proximity_relation: Some("{ S0[r, c] -> S0[r + 1, c] }".to_string()),
                symbolic_guard_candidate: None,
            },
            edges: Vec::new(),
        };
        let plans = candidate_schedules_for_backend_result(&scop, &deps, PolyBackendKind::Isl);
        assert!(
            plans
                .iter()
                .any(|plan| plan.kind == SchedulePlanKind::Identity)
        );
        assert!(
            plans
                .iter()
                .any(|plan| plan.kind == SchedulePlanKind::Tile2D),
            "expected isl candidate enumeration to keep tile2d open for dense 2d SCoP"
        );
        assert!(plans.iter().all(|plan| plan.kind != SchedulePlanKind::None));
    }
}
