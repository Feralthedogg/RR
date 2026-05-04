use super::ScopRegion;
use super::affine::{AffineExpr, AffineSymbol};
use super::cost::{estimate_fission_benefit, estimate_schedule_cost};
use super::dependence_backend::{DependenceResult, DependenceState};
use super::schedule::{PolyBackendUsed, SchedulePlan, SchedulePlanKind, ScheduleRelation};
use std::collections::BTreeSet;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleTreeAnnotation {
    pub legal: bool,
    pub estimated_cost: u32,
    pub dependence_state: DependenceState,
    pub note: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleBand {
    pub dimensions: Vec<String>,
    pub relation: ScheduleRelation,
    pub annotation: ScheduleTreeAnnotation,
    pub children: Vec<ScheduleTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleFilter {
    pub statement_ids: Vec<usize>,
    pub annotation: ScheduleTreeAnnotation,
    pub children: Vec<ScheduleTreeNode>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScheduleTransformKind {
    Identity,
    Interchange,
    Skew,
    Tile,
    Fuse,
    Split,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleTransform {
    pub kind: ScheduleTransformKind,
    pub plan_kind: SchedulePlanKind,
    pub relation: ScheduleRelation,
    pub permutation: Vec<usize>,
    pub tile_size: Option<usize>,
    pub tile_depth: Option<usize>,
    pub tile_rows: Option<usize>,
    pub tile_cols: Option<usize>,
    pub split_factors: Vec<usize>,
    pub fused_statement_ids: Vec<usize>,
    pub annotation: ScheduleTreeAnnotation,
    pub children: Vec<ScheduleTreeNode>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleTreeNode {
    Sequence(Vec<ScheduleTreeNode>),
    Filter(ScheduleFilter),
    Band(ScheduleBand),
    Transform(ScheduleTransform),
    Leaf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScheduleTree {
    pub backend: PolyBackendUsed,
    pub backend_artifact: Option<String>,
    pub root: ScheduleTreeNode,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FissionMode {
    Auto,
    ForceOn,
    ForceOff,
}

fn fission_mode_from_env() -> FissionMode {
    let Some(raw) = std::env::var("RR_POLY_FISSION").ok() else {
        return FissionMode::Auto;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => FissionMode::ForceOn,
        "0" | "false" | "no" | "off" => FissionMode::ForceOff,
        _ => FissionMode::Auto,
    }
}

fn statement_base_sets(stmt: &super::PolyStmt) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut reads = BTreeSet::new();
    let mut writes = BTreeSet::new();
    for access in &stmt.accesses {
        let key = if access.memref.name.is_empty() {
            format!("v{}", access.memref.base)
        } else {
            access.memref.name.clone()
        };
        match access.kind {
            super::access::AccessKind::Read => {
                reads.insert(key);
            }
            super::access::AccessKind::Write => {
                writes.insert(key);
            }
        }
    }
    (reads, writes)
}

fn has_cross_stmt_memory_flow(scop: &ScopRegion) -> bool {
    let mut prior_writes = BTreeSet::new();
    for stmt in scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
    {
        let (reads, writes) = statement_base_sets(stmt);
        if !prior_writes.is_disjoint(&reads) || !prior_writes.is_disjoint(&writes) {
            return true;
        }
        prior_writes.extend(writes);
    }
    false
}

fn should_auto_fission(scop: &ScopRegion, deps: &DependenceResult, plan: &SchedulePlan) -> bool {
    let data_stmt_count = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count();
    if data_stmt_count <= 1
        || !deps.legality_is_proven()
        || has_cross_stmt_memory_flow(scop)
        || !matches!(
            plan.kind,
            SchedulePlanKind::Tile1D
                | SchedulePlanKind::Tile2D
                | SchedulePlanKind::Tile3D
                | SchedulePlanKind::Skew2D
        )
    {
        return false;
    }
    estimate_fission_benefit(scop, plan) > 0
}

fn resolve_fission_mode(
    scop: &ScopRegion,
    deps: &DependenceResult,
    plan: &SchedulePlan,
) -> (bool, &'static str) {
    match fission_mode_from_env() {
        FissionMode::ForceOn => (
            scop.statements
                .iter()
                .filter(|stmt| !stmt.accesses.is_empty())
                .count()
                > 1,
            "fission-split",
        ),
        FissionMode::ForceOff => (false, "fused-statements"),
        FissionMode::Auto if should_auto_fission(scop, deps, plan) => (true, "auto-fission-split"),
        FissionMode::Auto => (false, "fused-statements"),
    }
}

fn relation_from_dimensions(dimensions: &[String]) -> ScheduleRelation {
    ScheduleRelation {
        input_dimensions: dimensions.to_vec(),
        output_expressions: dimensions
            .iter()
            .cloned()
            .map(|name| AffineExpr::symbol(AffineSymbol::LoopIv(name)))
            .collect(),
    }
}

fn output_dimensions_from_relation(relation: &ScheduleRelation) -> Vec<String> {
    let mut dims = Vec::with_capacity(relation.output_expressions.len());
    for expr in &relation.output_expressions {
        let Some((AffineSymbol::LoopIv(name), coeff)) = expr.terms.iter().next() else {
            return relation.input_dimensions.clone();
        };
        if *coeff != 1 || expr.terms.len() != 1 || expr.constant != 0 {
            return relation.input_dimensions.clone();
        }
        dims.push(name.clone());
    }
    if dims.is_empty() {
        relation.input_dimensions.clone()
    } else {
        dims
    }
}

fn transform_kind_for_plan(kind: SchedulePlanKind) -> Option<ScheduleTransformKind> {
    match kind {
        SchedulePlanKind::Identity => Some(ScheduleTransformKind::Identity),
        SchedulePlanKind::Interchange => Some(ScheduleTransformKind::Interchange),
        SchedulePlanKind::Skew2D => Some(ScheduleTransformKind::Skew),
        SchedulePlanKind::Tile1D | SchedulePlanKind::Tile2D | SchedulePlanKind::Tile3D => {
            Some(ScheduleTransformKind::Tile)
        }
        SchedulePlanKind::None => None,
    }
}

fn permutation_from_relation(relation: &ScheduleRelation) -> Vec<usize> {
    let outputs = output_dimensions_from_relation(relation);
    outputs
        .iter()
        .filter_map(|out| {
            relation
                .input_dimensions
                .iter()
                .position(|input| input == out)
        })
        .collect()
}

fn tile_split_factors(plan: &SchedulePlan) -> Vec<usize> {
    match plan.kind {
        SchedulePlanKind::Tile1D => plan.tile_size.into_iter().collect(),
        SchedulePlanKind::Tile2D => [plan.tile_rows, plan.tile_cols]
            .into_iter()
            .flatten()
            .collect(),
        SchedulePlanKind::Tile3D => [plan.tile_depth, plan.tile_rows, plan.tile_cols]
            .into_iter()
            .flatten()
            .collect(),
        _ => Vec::new(),
    }
}

fn annotate(
    scop: &ScopRegion,
    deps: &DependenceResult,
    plan: &SchedulePlan,
    note: &str,
) -> ScheduleTreeAnnotation {
    let dependence_state = deps.derived_state();
    ScheduleTreeAnnotation {
        legal: deps.legality_is_proven(),
        estimated_cost: estimate_schedule_cost(scop, plan),
        dependence_state,
        note: note.to_string(),
    }
}

impl ScheduleTree {
    pub fn from_plan(scop: &ScopRegion, deps: &DependenceResult, plan: SchedulePlan) -> Self {
        let (fission, _) = resolve_fission_mode(scop, deps, &plan);
        let auto_fused = !fission
            && matches!(fission_mode_from_env(), FissionMode::Auto)
            && scop
                .statements
                .iter()
                .filter(|stmt| !stmt.accesses.is_empty())
                .count()
                > 1;
        let mut tree = Self::from_plan_with_options(scop, deps, plan, fission);
        if auto_fused {
            tree.rewrite_first_matching_note("fused-statements", "auto-fused-statements");
        }
        tree
    }

    pub fn from_plan_with_hints(
        scop: &ScopRegion,
        deps: &DependenceResult,
        plan: SchedulePlan,
        prefer_fission: Option<bool>,
        fission_note_hint: Option<&str>,
    ) -> Self {
        let (resolved_fission, _) = resolve_fission_mode(scop, deps, &plan);
        let fission = prefer_fission.unwrap_or(resolved_fission);
        let mut tree = Self::from_plan_with_options(scop, deps, plan, fission);
        if let Some(note) = fission_note_hint
            && fission
        {
            tree.rewrite_first_matching_note("fission-split", note);
            tree.rewrite_first_matching_note("auto-fission-split", note);
        }
        tree
    }

    fn from_plan_with_options(
        scop: &ScopRegion,
        deps: &DependenceResult,
        plan: SchedulePlan,
        fission: bool,
    ) -> Self {
        let base_dims = if plan.relation.input_dimensions.is_empty() {
            scop.dimensions
                .iter()
                .map(|dim| dim.iv_name.clone())
                .collect::<Vec<_>>()
        } else {
            plan.relation.input_dimensions.clone()
        };
        let stmt_ids = scop
            .statements
            .iter()
            .map(|stmt| stmt.id)
            .collect::<Vec<_>>();
        let base_annotation = annotate(scop, deps, &plan, "iteration-band");
        let filter_annotation = annotate(scop, deps, &plan, "statement-filter");
        let auto_fission = should_auto_fission(scop, deps, &plan);
        let fission_note = if fission {
            if auto_fission {
                "auto-fission-split"
            } else {
                "fission-split"
            }
        } else {
            "fused-statements"
        };

        let mut current = ScheduleTreeNode::Leaf;

        if let Some(transform_kind) = transform_kind_for_plan(plan.kind) {
            let output_dims = output_dimensions_from_relation(&plan.relation);
            current = ScheduleTreeNode::Band(ScheduleBand {
                dimensions: output_dims,
                relation: plan.relation.clone(),
                annotation: annotate(scop, deps, &plan, "scheduled-band"),
                children: vec![ScheduleTreeNode::Leaf],
            });

            let transform = ScheduleTransform {
                kind: transform_kind,
                plan_kind: plan.kind,
                relation: plan.relation.clone(),
                permutation: permutation_from_relation(&plan.relation),
                tile_size: plan.tile_size,
                tile_depth: plan.tile_depth,
                tile_rows: plan.tile_rows,
                tile_cols: plan.tile_cols,
                split_factors: tile_split_factors(&plan),
                fused_statement_ids: Vec::new(),
                annotation: annotate(scop, deps, &plan, "primary-transform"),
                children: vec![current],
            };
            current = ScheduleTreeNode::Transform(transform);

            if matches!(
                plan.kind,
                SchedulePlanKind::Tile1D | SchedulePlanKind::Tile2D | SchedulePlanKind::Tile3D
            ) {
                current = ScheduleTreeNode::Transform(ScheduleTransform {
                    kind: ScheduleTransformKind::Split,
                    plan_kind: SchedulePlanKind::None,
                    relation: plan.relation.clone(),
                    permutation: Vec::new(),
                    tile_size: plan.tile_size,
                    tile_depth: plan.tile_depth,
                    tile_rows: plan.tile_rows,
                    tile_cols: plan.tile_cols,
                    split_factors: tile_split_factors(&plan),
                    fused_statement_ids: Vec::new(),
                    annotation: annotate(scop, deps, &plan, "strip-mine"),
                    children: vec![current],
                });
            }
        }

        if stmt_ids.len() > 1 && !fission {
            current = ScheduleTreeNode::Transform(ScheduleTransform {
                kind: ScheduleTransformKind::Fuse,
                plan_kind: SchedulePlanKind::None,
                relation: relation_from_dimensions(&base_dims),
                permutation: Vec::new(),
                tile_size: None,
                tile_depth: None,
                tile_rows: None,
                tile_cols: None,
                split_factors: Vec::new(),
                fused_statement_ids: stmt_ids.clone(),
                annotation: annotate(scop, deps, &plan, "fused-statements"),
                children: vec![current],
            });
        }

        let root = if fission && stmt_ids.len() > 1 {
            let split_node = ScheduleTreeNode::Transform(ScheduleTransform {
                kind: ScheduleTransformKind::Split,
                plan_kind: SchedulePlanKind::None,
                relation: relation_from_dimensions(&base_dims),
                permutation: Vec::new(),
                tile_size: None,
                tile_depth: None,
                tile_rows: None,
                tile_cols: None,
                split_factors: Vec::new(),
                fused_statement_ids: stmt_ids.clone(),
                annotation: annotate(scop, deps, &plan, fission_note),
                children: vec![current],
            });
            ScheduleTreeNode::Sequence(
                stmt_ids
                    .iter()
                    .map(|stmt_id| {
                        ScheduleTreeNode::Filter(ScheduleFilter {
                            statement_ids: vec![*stmt_id],
                            annotation: annotate(scop, deps, &plan, "fissioned-statement-filter"),
                            children: vec![ScheduleTreeNode::Band(ScheduleBand {
                                dimensions: base_dims.clone(),
                                relation: relation_from_dimensions(&base_dims),
                                annotation: base_annotation.clone(),
                                children: vec![split_node.clone()],
                            })],
                        })
                    })
                    .collect(),
            )
        } else {
            ScheduleTreeNode::Sequence(vec![ScheduleTreeNode::Filter(ScheduleFilter {
                statement_ids: stmt_ids,
                annotation: filter_annotation,
                children: vec![ScheduleTreeNode::Band(ScheduleBand {
                    dimensions: base_dims.clone(),
                    relation: relation_from_dimensions(&base_dims),
                    annotation: base_annotation,
                    children: vec![current],
                })],
            })])
        };

        Self {
            backend: plan.backend,
            backend_artifact: None,
            root,
        }
    }

    pub fn with_backend_artifact(mut self, artifact: Option<String>) -> Self {
        self.backend_artifact = artifact;
        self
    }

    pub fn primary_kind(&self) -> SchedulePlanKind {
        self.to_primary_plan().kind
    }

    pub fn to_primary_plan(&self) -> SchedulePlan {
        fn rec(node: &ScheduleTreeNode, backend: PolyBackendUsed) -> Option<SchedulePlan> {
            match node {
                ScheduleTreeNode::Sequence(nodes) => {
                    nodes.iter().find_map(|node| rec(node, backend))
                }
                ScheduleTreeNode::Filter(filter) => {
                    filter.children.iter().find_map(|node| rec(node, backend))
                }
                ScheduleTreeNode::Band(band) => {
                    band.children.iter().find_map(|node| rec(node, backend))
                }
                ScheduleTreeNode::Transform(transform) => {
                    if transform.plan_kind != SchedulePlanKind::None {
                        Some(SchedulePlan {
                            kind: transform.plan_kind,
                            relation: transform.relation.clone(),
                            backend,
                            tile_size: transform.tile_size,
                            tile_depth: transform.tile_depth,
                            tile_rows: transform.tile_rows,
                            tile_cols: transform.tile_cols,
                        })
                    } else {
                        transform
                            .children
                            .iter()
                            .find_map(|node| rec(node, backend))
                    }
                }
                ScheduleTreeNode::Leaf => None,
            }
        }

        rec(&self.root, self.backend).unwrap_or(SchedulePlan {
            kind: SchedulePlanKind::None,
            relation: ScheduleRelation {
                input_dimensions: Vec::new(),
                output_expressions: Vec::new(),
            },
            backend: self.backend,
            tile_size: None,
            tile_depth: None,
            tile_rows: None,
            tile_cols: None,
        })
    }

    pub fn node_count(&self) -> usize {
        fn rec(node: &ScheduleTreeNode) -> usize {
            match node {
                ScheduleTreeNode::Leaf => 1,
                ScheduleTreeNode::Sequence(nodes) => 1 + nodes.iter().map(rec).sum::<usize>(),
                ScheduleTreeNode::Filter(filter) => {
                    1 + filter.children.iter().map(rec).sum::<usize>()
                }
                ScheduleTreeNode::Band(band) => 1 + band.children.iter().map(rec).sum::<usize>(),
                ScheduleTreeNode::Transform(transform) => {
                    1 + transform.children.iter().map(rec).sum::<usize>()
                }
            }
        }

        rec(&self.root)
    }

    pub fn band_depth(&self) -> usize {
        fn rec(node: &ScheduleTreeNode, current: usize) -> usize {
            match node {
                ScheduleTreeNode::Leaf => current,
                ScheduleTreeNode::Sequence(nodes) => nodes
                    .iter()
                    .map(|node| rec(node, current))
                    .max()
                    .unwrap_or(current),
                ScheduleTreeNode::Filter(filter) => filter
                    .children
                    .iter()
                    .map(|node| rec(node, current))
                    .max()
                    .unwrap_or(current),
                ScheduleTreeNode::Band(band) => band
                    .children
                    .iter()
                    .map(|node| rec(node, current + 1))
                    .max()
                    .unwrap_or(current + 1),
                ScheduleTreeNode::Transform(transform) => transform
                    .children
                    .iter()
                    .map(|node| rec(node, current))
                    .max()
                    .unwrap_or(current),
            }
        }

        rec(&self.root, 0)
    }

    pub fn transform_count(&self, kind: ScheduleTransformKind) -> usize {
        fn rec(node: &ScheduleTreeNode, kind: ScheduleTransformKind) -> usize {
            match node {
                ScheduleTreeNode::Leaf => 0,
                ScheduleTreeNode::Sequence(nodes) => nodes.iter().map(|node| rec(node, kind)).sum(),
                ScheduleTreeNode::Filter(filter) => {
                    filter.children.iter().map(|node| rec(node, kind)).sum()
                }
                ScheduleTreeNode::Band(band) => {
                    band.children.iter().map(|node| rec(node, kind)).sum()
                }
                ScheduleTreeNode::Transform(transform) => {
                    usize::from(transform.kind == kind)
                        + transform
                            .children
                            .iter()
                            .map(|node| rec(node, kind))
                            .sum::<usize>()
                }
            }
        }

        rec(&self.root, kind)
    }

    pub fn contains_annotation_note(&self, needle: &str) -> bool {
        fn rec(node: &ScheduleTreeNode, needle: &str) -> bool {
            match node {
                ScheduleTreeNode::Leaf => false,
                ScheduleTreeNode::Sequence(nodes) => nodes.iter().any(|node| rec(node, needle)),
                ScheduleTreeNode::Filter(filter) => {
                    filter.annotation.note.contains(needle)
                        || filter.children.iter().any(|node| rec(node, needle))
                }
                ScheduleTreeNode::Band(band) => {
                    band.annotation.note.contains(needle)
                        || band.children.iter().any(|node| rec(node, needle))
                }
                ScheduleTreeNode::Transform(transform) => {
                    transform.annotation.note.contains(needle)
                        || transform.children.iter().any(|node| rec(node, needle))
                }
            }
        }

        rec(&self.root, needle)
    }

    fn rewrite_first_matching_note(&mut self, needle: &str, replacement: &str) -> bool {
        fn rec(node: &mut ScheduleTreeNode, needle: &str, replacement: &str) -> bool {
            match node {
                ScheduleTreeNode::Leaf => false,
                ScheduleTreeNode::Sequence(nodes) => {
                    nodes.iter_mut().any(|node| rec(node, needle, replacement))
                }
                ScheduleTreeNode::Filter(filter) => {
                    if filter.annotation.note == needle {
                        filter.annotation.note = replacement.to_string();
                        true
                    } else {
                        filter
                            .children
                            .iter_mut()
                            .any(|node| rec(node, needle, replacement))
                    }
                }
                ScheduleTreeNode::Band(band) => {
                    if band.annotation.note == needle {
                        band.annotation.note = replacement.to_string();
                        true
                    } else {
                        band.children
                            .iter_mut()
                            .any(|node| rec(node, needle, replacement))
                    }
                }
                ScheduleTreeNode::Transform(transform) => {
                    if transform.annotation.note == needle {
                        transform.annotation.note = replacement.to_string();
                        true
                    } else {
                        transform
                            .children
                            .iter_mut()
                            .any(|node| rec(node, needle, replacement))
                    }
                }
            }
        }

        rec(&mut self.root, needle, replacement)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::opt::poly::affine::PresburgerSet;
    use crate::mir::opt::poly::dependence_backend::{DependenceRelation, DependenceSummary};
    use crate::mir::opt::poly::{LoopDimension, PolyStmt, PolyStmtKind};

    fn test_scop(stmt_count: usize, dims: &[&str]) -> ScopRegion {
        let dimensions = dims
            .iter()
            .map(|name| LoopDimension {
                iv_name: (*name).to_string(),
                lower_bound: AffineExpr::constant(1),
                upper_bound: AffineExpr::constant(32),
                step: 1,
            })
            .collect::<Vec<_>>();
        let statements = (0..stmt_count)
            .map(|id| PolyStmt {
                id,
                block: 0,
                kind: PolyStmtKind::Assign {
                    dst: format!("v{id}"),
                },
                expr_root: None,
                accesses: Vec::new(),
            })
            .collect::<Vec<_>>();
        ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions,
            iteration_space: PresburgerSet::new(
                dims.iter().map(|dim| (*dim).to_string()).collect(),
                Vec::new(),
            ),
            parameters: Default::default(),
            statements,
        }
    }

    fn test_deps(state: DependenceState) -> DependenceResult {
        DependenceResult {
            summary: DependenceSummary {
                state,
                write_count: 1,
                access_count: 1,
                reduction_count: 0,
            },
            relation: DependenceRelation {
                iteration_dimensions: vec!["i".to_string()],
                edge_count: 1,
                raw_relation: None,
                war_relation: None,
                waw_relation: None,
                reduction_relation: None,
                validity_relation: None,
                proximity_relation: None,
                symbolic_guard_candidate: None,
            },
            edges: Vec::new(),
        }
    }

    #[test]
    fn schedule_tree_round_trips_primary_plan() {
        let plan = SchedulePlan {
            kind: SchedulePlanKind::Tile2D,
            relation: ScheduleRelation {
                input_dimensions: vec!["r".to_string(), "c".to_string()],
                output_expressions: vec![
                    AffineExpr::symbol(AffineSymbol::LoopIv("r".to_string())),
                    AffineExpr::symbol(AffineSymbol::LoopIv("c".to_string())),
                ],
            },
            backend: PolyBackendUsed::Heuristic,
            tile_size: None,
            tile_depth: None,
            tile_rows: Some(4),
            tile_cols: Some(8),
        };
        let scop = test_scop(2, &["r", "c"]);
        let deps = test_deps(DependenceState::IdentityProven);
        let tree = ScheduleTree::from_plan(&scop, &deps, plan.clone());
        assert_eq!(tree.primary_kind(), SchedulePlanKind::Tile2D);
        assert_eq!(tree.to_primary_plan(), plan);
        assert!(tree.node_count() >= 6);
        assert_eq!(tree.band_depth(), 2);
    }

    #[test]
    fn schedule_tree_contains_filter_fuse_and_split_nodes() {
        let plan = SchedulePlan {
            kind: SchedulePlanKind::Tile1D,
            relation: ScheduleRelation {
                input_dimensions: vec!["i".to_string()],
                output_expressions: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
            },
            backend: PolyBackendUsed::Heuristic,
            tile_size: Some(8),
            tile_depth: None,
            tile_rows: None,
            tile_cols: None,
        };
        let scop = test_scop(2, &["i"]);
        let deps = test_deps(DependenceState::IdentityProven);
        let tree = ScheduleTree::from_plan_with_options(&scop, &deps, plan, false);
        let debug = format!("{tree:#?}");
        assert!(debug.contains("Filter"));
        assert!(debug.contains("Fuse"));
        assert!(debug.contains("Split"));
        assert!(debug.contains("estimated_cost"));
        assert!(debug.contains("legal: true"));
    }

    #[test]
    fn schedule_tree_sequence_exposes_first_transform_as_primary_plan() {
        let plan = SchedulePlan {
            kind: SchedulePlanKind::Interchange,
            relation: ScheduleRelation {
                input_dimensions: vec!["i".to_string(), "j".to_string()],
                output_expressions: vec![
                    AffineExpr::symbol(AffineSymbol::LoopIv("j".to_string())),
                    AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string())),
                ],
            },
            backend: PolyBackendUsed::Heuristic,
            tile_size: None,
            tile_depth: None,
            tile_rows: None,
            tile_cols: None,
        };
        let scop = test_scop(1, &["i", "j"]);
        let deps = test_deps(DependenceState::IdentityProven);
        let tree = ScheduleTree::from_plan(&scop, &deps, plan);
        assert_eq!(tree.primary_kind(), SchedulePlanKind::Interchange);
        assert_eq!(tree.to_primary_plan().kind, SchedulePlanKind::Interchange);
    }

    #[test]
    fn schedule_tree_can_carry_backend_artifact() {
        let plan = SchedulePlan {
            kind: SchedulePlanKind::Identity,
            relation: ScheduleRelation {
                input_dimensions: vec!["i".to_string()],
                output_expressions: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
            },
            backend: PolyBackendUsed::Isl,
            tile_size: None,
            tile_depth: None,
            tile_rows: None,
            tile_cols: None,
        };
        let scop = test_scop(1, &["i"]);
        let deps = test_deps(DependenceState::IdentityProven);
        let tree = ScheduleTree::from_plan(&scop, &deps, plan)
            .with_backend_artifact(Some("computed_schedule=...".to_string()));
        assert_eq!(tree.backend, PolyBackendUsed::Isl);
        assert_eq!(
            tree.backend_artifact.as_deref(),
            Some("computed_schedule=...")
        );
    }

    #[test]
    fn schedule_tree_can_fission_multi_stmt_scop_into_statement_sequence() {
        let plan = SchedulePlan {
            kind: SchedulePlanKind::Tile1D,
            relation: ScheduleRelation {
                input_dimensions: vec!["i".to_string()],
                output_expressions: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
            },
            backend: PolyBackendUsed::Heuristic,
            tile_size: Some(4),
            tile_depth: None,
            tile_rows: None,
            tile_cols: None,
        };
        let scop = test_scop(2, &["i"]);
        let deps = test_deps(DependenceState::IdentityProven);
        let tree = ScheduleTree::from_plan_with_options(&scop, &deps, plan, true);
        let debug = format!("{tree:#?}");
        assert!(debug.contains("fission-split"));
        assert!(debug.contains("fissioned-statement-filter"));
    }

    #[test]
    fn schedule_tree_auto_fissions_profitable_multi_stmt_tiled_scop() {
        let plan = SchedulePlan {
            kind: SchedulePlanKind::Tile1D,
            relation: ScheduleRelation {
                input_dimensions: vec!["i".to_string()],
                output_expressions: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
            },
            backend: PolyBackendUsed::Heuristic,
            tile_size: Some(4),
            tile_depth: None,
            tile_rows: None,
            tile_cols: None,
        };
        let mut scop = test_scop(2, &["i"]);
        for (stmt, (read_base, write_base)) in scop.statements.iter_mut().zip([(1, 2), (1, 3)]) {
            stmt.accesses = vec![
                crate::mir::opt::poly::access::AccessRelation {
                    statement_id: stmt.id,
                    kind: crate::mir::opt::poly::access::AccessKind::Read,
                    memref: crate::mir::opt::poly::access::MemRef {
                        base: read_base,
                        name: format!("v{read_base}"),
                        rank: 1,
                        layout: crate::mir::opt::poly::access::MemoryLayout::Dense1D,
                    },
                    subscripts: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
                },
                crate::mir::opt::poly::access::AccessRelation {
                    statement_id: stmt.id,
                    kind: crate::mir::opt::poly::access::AccessKind::Write,
                    memref: crate::mir::opt::poly::access::MemRef {
                        base: write_base,
                        name: format!("v{write_base}"),
                        rank: 1,
                        layout: crate::mir::opt::poly::access::MemoryLayout::Dense1D,
                    },
                    subscripts: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
                },
            ];
        }
        let deps = test_deps(DependenceState::IdentityProven);
        let tree = ScheduleTree::from_plan(&scop, &deps, plan);
        assert!(format!("{tree:#?}").contains("auto-fission-split"));
    }

    #[test]
    fn schedule_tree_does_not_auto_fission_cross_stmt_flow() {
        let plan = SchedulePlan {
            kind: SchedulePlanKind::Tile1D,
            relation: ScheduleRelation {
                input_dimensions: vec!["i".to_string()],
                output_expressions: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
            },
            backend: PolyBackendUsed::Heuristic,
            tile_size: Some(4),
            tile_depth: None,
            tile_rows: None,
            tile_cols: None,
        };
        let mut scop = test_scop(2, &["i"]);
        scop.statements[0].accesses = vec![crate::mir::opt::poly::access::AccessRelation {
            statement_id: 0,
            kind: crate::mir::opt::poly::access::AccessKind::Write,
            memref: crate::mir::opt::poly::access::MemRef {
                base: 2,
                name: "v2".to_string(),
                rank: 1,
                layout: crate::mir::opt::poly::access::MemoryLayout::Dense1D,
            },
            subscripts: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
        }];
        scop.statements[1].accesses = vec![
            crate::mir::opt::poly::access::AccessRelation {
                statement_id: 1,
                kind: crate::mir::opt::poly::access::AccessKind::Read,
                memref: crate::mir::opt::poly::access::MemRef {
                    base: 20,
                    name: "v2".to_string(),
                    rank: 1,
                    layout: crate::mir::opt::poly::access::MemoryLayout::Dense1D,
                },
                subscripts: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
            },
            crate::mir::opt::poly::access::AccessRelation {
                statement_id: 1,
                kind: crate::mir::opt::poly::access::AccessKind::Write,
                memref: crate::mir::opt::poly::access::MemRef {
                    base: 3,
                    name: "v3".to_string(),
                    rank: 1,
                    layout: crate::mir::opt::poly::access::MemoryLayout::Dense1D,
                },
                subscripts: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
            },
        ];
        let deps = test_deps(DependenceState::IdentityProven);
        let tree = ScheduleTree::from_plan(&scop, &deps, plan);
        let debug = format!("{tree:#?}");
        assert!(!debug.contains("auto-fission-split"));
        assert!(debug.contains("auto-fused-statements"));
    }
}
