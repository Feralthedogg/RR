use super::common::*;

#[test]
fn helper_heavy_whole_auto_builtin_call_map_stays_runtime_guarded() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![1, 2],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("out".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(6)),
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
            kind: ValueKind::Const(Lit::Str("abs".to_string())),
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
            kind: ValueKind::Const(Lit::Int(44)),
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
            kind: ValueKind::Call {
                callee: "c".to_string(),
                args: vec![6],
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
            id: 6,
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
            id: 7,
            kind: ValueKind::Load {
                var: "src".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("src".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 8,
            kind: ValueKind::Load {
                var: "idx".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("idx".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 9,
            kind: ValueKind::Call {
                callee: "rr_gather".to_string(),
                args: vec![7, 8],
                names: vec![None, None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 10,
            kind: ValueKind::Call {
                callee: "rr_call_map_whole_auto".to_string(),
                args: vec![0, 3, 4, 5, 9],
                names: vec![None; 5],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("out".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(0, "out");
    backend.bind_var_to_value("out", 0);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "out".to_string(),
                src: 10,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("helper-heavy whole-auto builtin call-map emission should succeed");

    assert!(backend.output.contains("out <- rr_call_map_whole_auto("));
    assert!(!backend.output.contains("out <- abs("));
}

#[test]
fn whole_dest_end_known_var_matches_len_alias_expr() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("xs".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Len { base: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
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
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![2, 1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("out".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Len { base: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "n".to_string(),
                src: 1,
                span: Span::dummy(),
            },
            &values,
            &["xs".to_string()],
        )
        .expect("len alias assignment should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "out".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &["xs".to_string()],
        )
        .expect("fresh allocation assignment should emit");

    assert!(backend.whole_dest_end_matches_known_var("out", 4, &values, &["xs".to_string()]));
}

#[test]
fn whole_range_sym17_allocator_like_assign_resolves_direct_rhs() {
    let mut backend = backend_with_sym17_fresh();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("size".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
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
            id: 2,
            kind: ValueKind::Const(Lit::Int(2)),
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
            kind: ValueKind::Call {
                callee: "Sym_17".to_string(),
                args: vec![0, 1, 2],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("y".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
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
            id: 5,
            kind: ValueKind::Param { index: 1 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("src".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![3, 4, 0, 5],
                names: vec![None; 4],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("y".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "y".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &["size".to_string(), "src".to_string()],
        )
        .expect("fresh allocator assignment should emit");

    let rendered = backend
        .try_resolve_whole_range_self_assign_rhs(
            "y",
            6,
            &values,
            &["size".to_string(), "src".to_string()],
        )
        .expect("whole-range Sym_17 replay should resolve directly");
    assert_eq!(rendered, "src");
}

#[test]
fn singleton_size_boundary_assign_collapses_rep_int_wrapper_to_scalar_rhs() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("size".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(6)),
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
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![1, 0],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Const(Lit::Int(5)),
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
            id: 5,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![3, 4],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![2, 0, 0, 5],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "nf".to_string(),
                src: 2,
                span: Span::dummy(),
            },
            &values,
            &["size".to_string()],
        )
        .expect("base alloc should emit");

    let rendered = backend.resolve_rr_idx_cube_vec_arg_expr(6, &values, &["size".to_string()]);
    assert_eq!(rendered, "replace(nf, size, 5L)");
}

#[test]
fn callsite_seq_len_summary_allows_replace_at_size_boundary() {
    let mut summaries = FxHashMap::default();
    summaries.insert(
        "Sym_72".to_string(),
        FxHashMap::from_iter([(2usize, 3usize)]),
    );
    let mut backend = RBackend::with_analysis_options(FxHashSet::default(), summaries, false);
    backend.current_fn_name = "Sym_72".to_string();
    backend.analysis.current_seq_len_param_end_slots = FxHashMap::from_iter([(2usize, 3usize)]);

    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("f".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Param { index: 2 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("ys".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Len { base: 1 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("width".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![0, 2],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Param { index: 3 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("size".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Const(Lit::Int(5)),
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
                callee: "rr_assign_slice".to_string(),
                args: vec![3, 4, 4, 5],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("nf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
    ];

    let params = [
        "f".to_string(),
        "x".to_string(),
        "ys".to_string(),
        "size".to_string(),
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "nf".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &params,
        )
        .expect("base alloc should emit");

    let rendered = backend.resolve_rr_idx_cube_vec_arg_expr(6, &values, &params);
    assert_eq!(rendered, "replace(nf, size, 5L)");
}

#[test]
fn remember_known_full_end_expr_handles_self_referential_assign_slice_cycle() {
    let mut backend = RBackend::new();
    backend.note_var_write("temp");
    backend
        .loop_analysis
        .known_full_end_exprs
        .insert("temp".to_string(), "n".to_string());

    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "temp".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("temp".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Len { base: 1 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("inlined_n".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
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
            id: 4,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![3, 0],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("inlined_out".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Const(Lit::Int(1)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("i".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![4, 5, 2, 1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("inlined_out".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(6, "temp");
    backend.bind_var_to_value("temp", 6);

    backend.remember_known_full_end_expr("temp", 6, &values, &["n".to_string()]);

    assert_eq!(
        backend
            .loop_analysis
            .known_full_end_exprs
            .get("temp")
            .map(String::as_str),
        Some("n")
    );
}

#[test]
fn scalar_adjusted_end_expr_is_not_treated_as_full_end() {
    let backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Param { index: 0 },
            span: Span::dummy(),
            facts: Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, false),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(-1)),
            span: Span::dummy(),
            facts: Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, false),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Int, false),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
    ];

    assert_eq!(
        backend.known_full_end_expr_for_value(2, &values, &["n".to_string()]),
        None
    );
}
