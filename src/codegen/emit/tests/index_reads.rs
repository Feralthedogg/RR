use super::common::*;

#[test]
fn whole_range_call_map_slice_is_emitted_as_direct_vector_call() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![2, 1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("score".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(8)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(0)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Const(Lit::Str("pmax".to_string())),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Const(Lit::Int(25)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "c".to_string(),
                args: vec![3],
                names: vec![None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 7,
            kind: ValueKind::Load {
                var: "x".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("x".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 8,
            kind: ValueKind::Const(Lit::Float(0.05)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 9,
            kind: ValueKind::Call {
                callee: "rr_call_map_slice_auto".to_string(),
                args: vec![0, 3, 1, 4, 5, 6, 7, 8],
                names: vec![None; 8],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("score".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(0, "score");
    backend.bind_var_to_value("score", 0);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "score".to_string(),
                src: 9,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("whole-range call-map emission should succeed");

    assert!(backend.output.contains("score <- pmax(x, 0.05)"));
    assert!(!backend.output.contains("rr_call_map_slice_auto("));
}

#[test]
fn loop_stable_known_full_end_allows_whole_range_call_map_fold() {
    let mut backend = RBackend::new();
    backend
        .loop_analysis
        .known_full_end_exprs
        .insert("score".to_string(), "n".to_string());

    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "score".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("score".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Load {
                var: "i".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("i".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Const(Lit::Str("pmax".to_string())),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Const(Lit::Int(25)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "c".to_string(),
                args: vec![1],
                names: vec![None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 7,
            kind: ValueKind::Load {
                var: "x".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("x".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 8,
            kind: ValueKind::Const(Lit::Float(0.05)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 9,
            kind: ValueKind::Call {
                callee: "rr_call_map_slice_auto".to_string(),
                args: vec![0, 2, 3, 4, 5, 6, 7, 8],
                names: vec![None; 8],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("score".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(1, "i");
    backend.bind_var_to_value("i", 1);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "score".to_string(),
                src: 9,
                span: Span::dummy(),
            },
            &values,
            &["n".to_string()],
        )
        .expect("loop-stable whole-range call-map emission should succeed");

    assert!(backend.output.contains("score <- pmax(x, 0.05)"));
    assert!(!backend.output.contains("rr_call_map_slice_auto("));
}

#[test]
fn whole_range_rr_index1_read_vec_call_elides_to_base_expr() {
    let mut backend = RBackend::new();
    backend
        .loop_analysis
        .known_full_end_exprs
        .insert("x".to_string(), "n".to_string());

    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "x".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("x".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Range { start: 2, end: 1 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Int, true),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Call {
                callee: "rr_index_vec_floor".to_string(),
                args: vec![3],
                names: vec![None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Int, true),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
    ];

    let rendered = backend.resolve_call_expr(
        &values[0],
        "rr_index1_read_vec",
        &[0, 4],
        &[None, None],
        &values,
        &["n".to_string()],
    );
    assert_eq!(rendered, "x");
}

#[test]
fn cube_index_scalar_read_elides_index_wrapper() {
    let backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "u".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("u".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(2)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Const(Lit::Int(3)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Call {
                callee: "rr_idx_cube_vec_i".to_string(),
                args: vec![1, 2, 3, 1],
                names: vec![None, None, None, None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Index1D {
                base: 0,
                idx: 4,
                is_safe: false,
                is_na_safe: false,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
    ];

    let rendered = backend.resolve_val(5, &values, &[], false);
    assert_eq!(rendered, "u[rr_idx_cube_vec_i(1L, 2L, 3L, 1L)]");
}

#[test]
fn cube_index_scalar_read_reuses_bound_plain_symbol_for_index_expr() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "u".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("u".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(2)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Load {
                var: "a".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("a".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Load {
                var: "b".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("b".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 3,
                rhs: 4,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "rr_idx_cube_vec_i".to_string(),
                args: vec![1, 2, 1, 5],
                names: vec![None, None, None, None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 7,
            kind: ValueKind::Index1D {
                base: 0,
                idx: 6,
                is_safe: false,
                is_na_safe: false,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(5, "sum_ab");
    backend.bind_var_to_value("sum_ab", 5);

    let rendered = backend.resolve_val(7, &values, &[], false);
    assert_eq!(rendered, "u[rr_idx_cube_vec_i(1L, 2L, 1L, sum_ab)]");
}

fn wrap_index_scalar_read_elides_index_wrapper() {
    let backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "B".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("B".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(32)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Call {
                callee: "rr_wrap_index_vec_i".to_string(),
                args: vec![1, 1, 1, 1],
                names: vec![None, None, None, None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Index1D {
                base: 0,
                idx: 2,
                is_safe: false,
                is_na_safe: false,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
    ];

    let rendered = backend.resolve_val(3, &values, &[], false);
    assert_eq!(rendered, "B[rr_wrap_index_vec_i(32L, 32L, 32L, 32L)]");
}

#[test]
fn direct_rr_index1_read_call_elides_when_index_is_safe() {
    let backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "clean".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("clean".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(250000)),
            span: Span::dummy(),
            facts: Facts::new(
                Facts::INT_SCALAR | Facts::NON_NA | Facts::ONE_BASED,
                crate::mir::flow::Interval::point(250000),
            ),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    let rendered = backend.resolve_call_expr(
        &values[0],
        "rr_index1_read",
        &[0, 1],
        &[None, None],
        &values,
        &[],
    );
    assert_eq!(rendered, "clean[250000L]");
}

#[test]
fn direct_rr_index1_read_call_elides_when_index_var_is_bound_to_safe_value() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "clean".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("clean".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "n".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(250000)),
            span: Span::dummy(),
            facts: Facts::new(
                Facts::INT_SCALAR | Facts::NON_NA | Facts::ONE_BASED,
                crate::mir::flow::Interval::point(250000),
            ),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];
    backend.bind_var_to_value("n", 2);

    let rendered = backend.resolve_call_expr(
        &values[0],
        "rr_index1_read",
        &[0, 1],
        &[None, None],
        &values,
        &[],
    );
    assert_eq!(rendered, "clean[n]");
}

#[test]
fn index1d_expr_elides_when_index_var_is_bound_to_safe_value() {
    let mut backend = RBackend::new();
    backend
        .loop_analysis
        .known_full_end_exprs
        .insert("clean".to_string(), "n".to_string());
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "clean".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("clean".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "n".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(250000)),
            span: Span::dummy(),
            facts: Facts::new(
                Facts::INT_SCALAR | Facts::NON_NA | Facts::ONE_BASED,
                crate::mir::flow::Interval::point(250000),
            ),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Index1D {
                base: 0,
                idx: 1,
                is_safe: false,
                is_na_safe: false,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
    ];
    backend.bind_var_to_value("n", 2);

    let rendered = backend.resolve_val(3, &values, &[], false);
    assert_eq!(rendered, "clean[n]");
}

#[test]
fn index1d_expr_reuses_live_plain_symbol_alias_for_index_expr() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "clean".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("clean".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "n".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::new(
                Facts::INT_SCALAR | Facts::NON_NA,
                crate::mir::flow::Interval::point(1),
            ),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 1,
                rhs: 2,
            },
            span: Span::dummy(),
            facts: Facts::new(
                Facts::INT_SCALAR | Facts::NON_NA,
                crate::mir::flow::Interval::TOP,
            ),
            origin_var: Some("idx_alias".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Index1D {
                base: 0,
                idx: 3,
                is_safe: true,
                is_na_safe: true,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("idx_alias");
    backend.bind_value_to_var(3, "idx_alias");
    backend.bind_var_to_value("idx_alias", 3);

    let rendered = backend.resolve_val(4, &values, &[], false);
    assert_eq!(rendered, "clean[idx_alias]");
}

#[test]
fn active_scalar_loop_index_load_does_not_fold_to_seed_constant() {
    let mut backend = RBackend::new();
    backend
        .loop_analysis
        .active_scalar_loop_indices
        .push(ActiveScalarLoopIndex {
            var: "i".to_string(),
            start_min: 1,
            cmp: ScalarLoopCmp::Le,
        });
    backend.bind_value_to_var(0, "i");
    backend.bind_var_to_value("i", 1);

    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "i".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("i".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, false),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, false),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
    ];

    let rendered = backend.resolve_val(0, &values, &[], false);
    assert_eq!(rendered, "i");
}

#[test]
fn index1d_expr_elides_when_loop_offset_is_proven_safe() {
    let mut backend = RBackend::new();
    backend
        .loop_analysis
        .active_scalar_loop_indices
        .push(ActiveScalarLoopIndex {
            var: "i".to_string(),
            start_min: 2,
            cmp: ScalarLoopCmp::Lt,
        });
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "a".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("a".to_string()),
            phi_block: None,
            value_ty: TypeState::vector(PrimTy::Double, false),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "i".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("i".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Sub,
                lhs: 1,
                rhs: 2,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, true),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Index1D {
                base: 0,
                idx: 3,
                is_safe: false,
                is_na_safe: false,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
    ];

    let rendered = backend.resolve_val(4, &values, &[], false);
    assert_eq!(rendered, "a[(i - 1L)]");
}
