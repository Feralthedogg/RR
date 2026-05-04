use super::super::ScopRegion;
use super::super::affine::{AffineExpr, AffineSymbol};
use super::super::schedule::{SchedulePlan, SchedulePlanKind};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IslArtifacts {
    pub domain: String,
    pub validity: Option<String>,
    pub proximity: Option<String>,
    pub coincidence: Option<String>,
    pub conditional_validity: Option<String>,
    pub conditional_validity_applied: bool,
    pub conditional_validity_candidate: Option<String>,
    pub candidate_schedule_map: Option<String>,
    pub candidate_schedule_roundtrip: Option<String>,
    pub computed_schedule: String,
    pub root_type: String,
    pub contains_sequence_node: bool,
    pub contains_filter_node: bool,
    pub first_band_members: usize,
    pub first_band_partial_schedule: Option<String>,
}

impl IslArtifacts {
    pub fn render(&self) -> String {
        format!(
            "domain={}; validity={}; proximity={}; coincidence={}; conditional_validity={}; conditional_validity_applied={}; conditional_validity_candidate={}; candidate_map={}; candidate_roundtrip={}; computed_schedule={}; root_type={}; contains_sequence_node={}; contains_filter_node={}; first_band_members={}; first_band_partial_schedule={}",
            self.domain,
            self.validity.as_deref().unwrap_or(""),
            self.proximity.as_deref().unwrap_or(""),
            self.coincidence.as_deref().unwrap_or(""),
            self.conditional_validity.as_deref().unwrap_or(""),
            usize::from(self.conditional_validity_applied),
            self.conditional_validity_candidate.as_deref().unwrap_or(""),
            self.candidate_schedule_map.as_deref().unwrap_or(""),
            self.candidate_schedule_roundtrip.as_deref().unwrap_or(""),
            self.computed_schedule,
            self.root_type,
            usize::from(self.contains_sequence_node),
            usize::from(self.contains_filter_node),
            self.first_band_members,
            self.first_band_partial_schedule.as_deref().unwrap_or(""),
        )
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IslTransformHints {
    pub inferred_plan: Option<SchedulePlan>,
    pub prefer_fission: bool,
    pub reason: String,
}

pub fn snapshot_schedule_artifacts(
    scop: &ScopRegion,
    plan: &SchedulePlan,
    validity: Option<&str>,
    proximity: Option<&str>,
    coincidence: Option<&str>,
    conditional_validity: Option<&str>,
    conditional_validity_candidate: Option<&str>,
) -> IslArtifacts {
    let candidate_schedule_map = {
        let inputs = plan.relation.input_dimensions.to_vec();
        let outputs = plan
            .relation
            .output_expressions
            .iter()
            .map(|expr| {
                let mut parts = Vec::new();
                for (symbol, coeff) in &expr.terms {
                    let name = match symbol {
                        AffineSymbol::LoopIv(name)
                        | AffineSymbol::Param(name)
                        | AffineSymbol::Invariant(name)
                        | AffineSymbol::Length(name) => name.clone(),
                    };
                    let term = match *coeff {
                        1 => name,
                        -1 => format!("-{name}"),
                        coeff => format!("{coeff}*{name}"),
                    };
                    parts.push(term);
                }
                if expr.constant != 0 || parts.is_empty() {
                    parts.push(expr.constant.to_string());
                }
                parts.join(" + ").replace("+ -", "- ")
            })
            .collect::<Vec<_>>();
        if inputs.is_empty() || outputs.is_empty() {
            None
        } else {
            Some(format!(
                "{{ {} }}",
                solver_statement_ids(scop)
                    .into_iter()
                    .map(|stmt_id| format!(
                        "S{stmt_id}[{}] -> [{}]",
                        inputs.join(", "),
                        outputs.join(", ")
                    ))
                    .collect::<Vec<_>>()
                    .join("; ")
            ))
        }
    };
    let domain = format!(
        "{{ {} }}",
        solver_statement_ids(scop)
            .into_iter()
            .map(|stmt_id| {
                let dims = scop
                    .dimensions
                    .iter()
                    .map(|dim| dim.iv_name.clone())
                    .collect::<Vec<_>>();
                format!("S{stmt_id}[{}]", dims.join(", "))
            })
            .collect::<Vec<_>>()
            .join("; ")
    );
    IslArtifacts {
        domain: domain.clone(),
        validity: validity.map(ToOwned::to_owned),
        proximity: proximity.map(ToOwned::to_owned),
        coincidence: coincidence.map(ToOwned::to_owned),
        conditional_validity: conditional_validity.map(ToOwned::to_owned),
        conditional_validity_applied: false,
        conditional_validity_candidate: conditional_validity_candidate.map(ToOwned::to_owned),
        candidate_schedule_map: candidate_schedule_map.clone(),
        candidate_schedule_roundtrip: candidate_schedule_map.clone(),
        computed_schedule: candidate_schedule_map
            .clone()
            .unwrap_or_else(|| domain.clone()),
        root_type: "domain".to_string(),
        contains_sequence_node: false,
        contains_filter_node: !solver_statement_ids(scop).is_empty(),
        first_band_members: plan.relation.output_expressions.len(),
        first_band_partial_schedule: candidate_schedule_map,
    }
}

pub(crate) fn solver_statement_ids(scop: &ScopRegion) -> Vec<usize> {
    let ids = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .map(|stmt| stmt.id)
        .collect::<Vec<_>>();
    if ids.is_empty() {
        scop.statements.iter().map(|stmt| stmt.id).collect()
    } else {
        ids
    }
}

pub fn infer_plan_from_first_band(
    scop: &ScopRegion,
    backend: super::schedule::PolyBackendUsed,
    partial: Option<&str>,
) -> Option<SchedulePlan> {
    let partial = partial?;
    let start = partial.rfind('[')?;
    let end = partial[start..].find(']')? + start;
    let inside = partial.get(start + 1..end)?.trim();
    if inside.is_empty() {
        return None;
    }
    let order = inside
        .split(',')
        .map(|part| part.trim().to_string())
        .collect::<Vec<_>>();
    let dims = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.clone())
        .collect::<Vec<_>>();
    if order.len() != dims.len() || order.iter().any(|name| !dims.contains(name)) {
        return None;
    }
    let relation = super::schedule::ScheduleRelation {
        input_dimensions: dims.clone(),
        output_expressions: order
            .iter()
            .cloned()
            .map(|name| AffineExpr::symbol(AffineSymbol::LoopIv(name)))
            .collect(),
    };
    let kind = if order == dims {
        SchedulePlanKind::Identity
    } else {
        SchedulePlanKind::Interchange
    };
    Some(SchedulePlan {
        kind,
        relation,
        backend,
        tile_size: None,
        tile_depth: None,
        tile_rows: None,
        tile_cols: None,
    })
}

pub(crate) fn infer_skew2d_plan_from_partial(
    scop: &ScopRegion,
    backend: super::schedule::PolyBackendUsed,
    partial: Option<&str>,
) -> Option<SchedulePlan> {
    let partial = partial?;
    if scop.dimensions.len() != 2 {
        return None;
    }
    let outer = scop.dimensions[0].iv_name.clone();
    let inner = scop.dimensions[1].iv_name.clone();
    let outer_name = outer.as_str();
    let inner_name = inner.as_str();
    let looks_skewed =
        partial.contains(outer_name) && partial.contains(inner_name) && partial.contains(" + ");
    if !looks_skewed {
        return None;
    }
    let mut skewed = AffineExpr::symbol(AffineSymbol::LoopIv(inner.clone()));
    skewed.add_assign(&AffineExpr::symbol(AffineSymbol::LoopIv(outer.clone())), 1);
    Some(SchedulePlan {
        kind: SchedulePlanKind::Skew2D,
        relation: super::schedule::ScheduleRelation {
            input_dimensions: vec![outer.clone(), inner],
            output_expressions: vec![AffineExpr::symbol(AffineSymbol::LoopIv(outer)), skewed],
        },
        backend,
        tile_size: None,
        tile_depth: None,
        tile_rows: None,
        tile_cols: None,
    })
}

pub(crate) fn infer_tile_plan_from_artifacts(
    scop: &ScopRegion,
    backend: super::schedule::PolyBackendUsed,
    artifacts: &IslArtifacts,
) -> Option<SchedulePlan> {
    let text = artifacts.computed_schedule.as_str();
    let kind = if text.contains("chosen_kind=Tile3D")
        || artifacts.root_type == "band" && scop.dimensions.len() == 3
    {
        SchedulePlanKind::Tile3D
    } else if text.contains("chosen_kind=Tile2D")
        || (artifacts.root_type == "band" && scop.dimensions.len() == 2)
    {
        SchedulePlanKind::Tile2D
    } else if text.contains("chosen_kind=Tile1D")
        || (artifacts.root_type == "band" && scop.dimensions.len() == 1)
    {
        SchedulePlanKind::Tile1D
    } else {
        return None;
    };

    Some(SchedulePlan {
        kind,
        relation: super::schedule::ScheduleRelation {
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
        },
        backend,
        tile_size: (kind == SchedulePlanKind::Tile1D).then_some(64),
        tile_depth: (kind == SchedulePlanKind::Tile3D).then_some(4),
        tile_rows: matches!(kind, SchedulePlanKind::Tile2D | SchedulePlanKind::Tile3D).then_some(8),
        tile_cols: matches!(kind, SchedulePlanKind::Tile2D | SchedulePlanKind::Tile3D).then_some(8),
    })
}

pub fn infer_transform_hints(
    scop: &ScopRegion,
    backend: super::schedule::PolyBackendUsed,
    artifacts: &IslArtifacts,
) -> IslTransformHints {
    let inferred_plan = infer_tile_plan_from_artifacts(scop, backend, artifacts)
        .or_else(|| {
            infer_plan_from_first_band(
                scop,
                backend,
                artifacts.first_band_partial_schedule.as_deref(),
            )
        })
        .or_else(|| {
            infer_skew2d_plan_from_partial(
                scop,
                backend,
                artifacts.first_band_partial_schedule.as_deref(),
            )
        });
    let prefer_fission = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count()
        > 1
        && (artifacts.contains_sequence_node || artifacts.contains_filter_node);
    let mut reasons = Vec::new();
    if let Some(plan) = &inferred_plan {
        reasons.push(format!("hint_plan={:?}", plan.kind));
    }
    if prefer_fission {
        reasons.push("hint_fission=1".to_string());
    }
    if reasons.is_empty() {
        reasons.push("hint_none".to_string());
    }
    IslTransformHints {
        inferred_plan,
        prefer_fission,
        reason: reasons.join(","),
    }
}
