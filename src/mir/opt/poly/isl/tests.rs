use super::*;
use crate::mir::opt::poly::affine::{AffineExpr, AffineSymbol, PresburgerSet};
use crate::mir::opt::poly::schedule::SchedulePlanKind;
use crate::mir::opt::poly::{LoopDimension, PolyStmt, PolyStmtKind, ScopRegion};

fn test_scop(stmt_count: usize) -> ScopRegion {
    ScopRegion {
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
        statements: (0..stmt_count)
            .map(|id| PolyStmt {
                id,
                block: 0,
                kind: PolyStmtKind::Assign {
                    dst: format!("v{id}"),
                },
                expr_root: None,
                accesses: vec![crate::mir::opt::poly::access::AccessRelation {
                    statement_id: id,
                    kind: crate::mir::opt::poly::access::AccessKind::Write,
                    memref: crate::mir::opt::poly::access::MemRef {
                        base: id + 1,
                        name: format!("A{id}"),
                        rank: 2,
                        layout: crate::mir::opt::poly::access::MemoryLayout::ColumnMajor2D,
                    },
                    subscripts: vec![
                        AffineExpr::symbol(AffineSymbol::LoopIv("r".to_string())),
                        AffineExpr::symbol(AffineSymbol::LoopIv("c".to_string())),
                    ],
                }],
            })
            .collect(),
    }
}

#[test]
fn infer_transform_hints_detects_skew2d_from_partial_schedule() {
    let scop = test_scop(2);
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
        first_band_partial_schedule: Some("{ S0[r, c] -> [r, c + r] }".to_string()),
    };
    let hints = infer_transform_hints(
        &scop,
        super::super::schedule::PolyBackendUsed::Isl,
        &artifacts,
    );
    assert_eq!(
        hints.inferred_plan.as_ref().map(|plan| plan.kind),
        Some(SchedulePlanKind::Skew2D)
    );
    assert!(hints.reason.contains("hint_plan=Skew2D"));
}

#[test]
fn infer_transform_hints_prefers_fission_for_sequence_artifact() {
    let scop = test_scop(2);
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
        contains_sequence_node: true,
        contains_filter_node: true,
        first_band_members: 2,
        first_band_partial_schedule: Some("{ S0[r, c] -> [r, c] }".to_string()),
    };
    let hints = infer_transform_hints(
        &scop,
        super::super::schedule::PolyBackendUsed::Isl,
        &artifacts,
    );
    assert!(hints.prefer_fission);
    assert!(hints.reason.contains("hint_fission=1"));
}

#[test]
fn infer_transform_hints_accepts_non_rotated_3d_permutation_as_interchange() {
    let scop = ScopRegion {
        header: 0,
        latch: 1,
        exits: vec![2],
        dimensions: vec![
            LoopDimension {
                iv_name: "i".to_string(),
                lower_bound: AffineExpr::constant(1),
                upper_bound: AffineExpr::constant(4),
                step: 1,
            },
            LoopDimension {
                iv_name: "j".to_string(),
                lower_bound: AffineExpr::constant(1),
                upper_bound: AffineExpr::constant(4),
                step: 1,
            },
            LoopDimension {
                iv_name: "k".to_string(),
                lower_bound: AffineExpr::constant(1),
                upper_bound: AffineExpr::constant(4),
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
            kind: PolyStmtKind::Assign {
                dst: "v0".to_string(),
            },
            expr_root: None,
            accesses: vec![],
        }],
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
        first_band_members: 3,
        first_band_partial_schedule: Some("{ S0[i, j, k] -> [k, i, j] }".to_string()),
    };
    let hints = infer_transform_hints(
        &scop,
        super::super::schedule::PolyBackendUsed::Isl,
        &artifacts,
    );
    let inferred = hints.inferred_plan.expect("expected inferred plan");
    assert_eq!(inferred.kind, SchedulePlanKind::Interchange);
    assert_eq!(
        inferred
            .relation
            .output_expressions
            .iter()
            .map(|expr| {
                expr.terms
                    .iter()
                    .next()
                    .and_then(|(symbol, _)| match symbol {
                        AffineSymbol::LoopIv(name) => Some(name.clone()),
                        _ => None,
                    })
            })
            .collect::<Vec<_>>(),
        vec![
            Some("k".to_string()),
            Some("i".to_string()),
            Some("j".to_string()),
        ]
    );
}

#[test]
fn infer_transform_hints_detects_tile2d_from_artifact_choice() {
    let scop = test_scop(2);
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
        computed_schedule: "chosen_kind=Tile2D".to_string(),
        root_type: "band".to_string(),
        contains_sequence_node: false,
        contains_filter_node: false,
        first_band_members: 2,
        first_band_partial_schedule: Some("{ S0[r, c] -> [r, c] }".to_string()),
    };
    let hints = infer_transform_hints(
        &scop,
        super::super::schedule::PolyBackendUsed::Isl,
        &artifacts,
    );
    assert_eq!(
        hints.inferred_plan.as_ref().map(|plan| plan.kind),
        Some(SchedulePlanKind::Tile2D)
    );
}

#[test]
fn infer_transform_hints_detects_tile3d_from_artifact_choice() {
    let scop = ScopRegion {
        header: 0,
        latch: 1,
        exits: vec![2],
        dimensions: vec![
            LoopDimension {
                iv_name: "i".to_string(),
                lower_bound: AffineExpr::constant(1),
                upper_bound: AffineExpr::constant(4),
                step: 1,
            },
            LoopDimension {
                iv_name: "j".to_string(),
                lower_bound: AffineExpr::constant(1),
                upper_bound: AffineExpr::constant(4),
                step: 1,
            },
            LoopDimension {
                iv_name: "k".to_string(),
                lower_bound: AffineExpr::constant(1),
                upper_bound: AffineExpr::constant(4),
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
            kind: PolyStmtKind::Assign {
                dst: "v0".to_string(),
            },
            expr_root: None,
            accesses: vec![],
        }],
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
        computed_schedule: "chosen_kind=Tile3D".to_string(),
        root_type: "band".to_string(),
        contains_sequence_node: false,
        contains_filter_node: false,
        first_band_members: 3,
        first_band_partial_schedule: Some("{ S0[i, j, k] -> [i, j, k] }".to_string()),
    };
    let hints = infer_transform_hints(
        &scop,
        super::super::schedule::PolyBackendUsed::Isl,
        &artifacts,
    );
    assert_eq!(
        hints.inferred_plan.as_ref().map(|plan| plan.kind),
        Some(SchedulePlanKind::Tile3D)
    );
}
