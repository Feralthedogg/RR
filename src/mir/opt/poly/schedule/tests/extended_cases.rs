use super::*;

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
