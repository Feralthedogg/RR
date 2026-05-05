use super::*;

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
    let plan = search_schedule_for_backend(&scop, &deps, PolyBackendKind::Heuristic);
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
    let plan = search_schedule_for_backend(&scop, &deps, PolyBackendKind::Heuristic);
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
    let plan = search_schedule_for_backend(&scop, &deps, PolyBackendKind::Heuristic);
    assert_eq!(plan.kind, SchedulePlanKind::Interchange);
    assert_eq!(plan.relation.output_expressions.len(), 3);
}

#[test]
pub(crate) fn parse_backend_name_defaults_to_heuristic() {
    assert_eq!(parse_backend_name(""), PolyBackendKind::Isl);
    assert_eq!(parse_backend_name("weird"), PolyBackendKind::Heuristic);
    assert_eq!(parse_backend_name("isl"), PolyBackendKind::Isl);
}

#[test]
fn backend_auto_prefers_isl_when_build_has_isl() {
    assert_eq!(backend_from_setting(None, false), PolyBackendKind::Isl);
    assert_eq!(backend_from_setting(None, true), PolyBackendKind::Isl);
    assert_eq!(
        backend_from_setting(Some("auto"), false),
        PolyBackendKind::Isl
    );
    assert_eq!(
        backend_from_setting(Some("auto"), true),
        PolyBackendKind::Isl
    );
}

#[test]
fn backend_explicit_value_overrides_auto_default() {
    assert_eq!(
        backend_from_setting(Some("isl"), false),
        PolyBackendKind::Isl
    );
    assert_eq!(
        backend_from_setting(Some("heuristic"), true),
        PolyBackendKind::Heuristic
    );
    assert_eq!(
        backend_from_setting(Some("weird"), true),
        PolyBackendKind::Heuristic
    );
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
