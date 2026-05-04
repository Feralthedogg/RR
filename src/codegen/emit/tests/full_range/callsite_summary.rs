use super::*;
pub(crate) fn callsite_seq_len_summary_allows_replace_at_size_boundary() {
    let mut summaries = FxHashMap::default();
    summaries.insert(
        "Sym_72".to_string(),
        FxHashMap::from_iter([(2usize, 3usize)]),
    );
    let mut backend = RBackend::with_analysis_options(
        FxHashSet::default(),
        FxHashSet::default(),
        summaries,
        false,
    );
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
pub(crate) fn remember_known_full_end_expr_handles_self_referential_assign_slice_cycle() {
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
pub(crate) fn scalar_adjusted_end_expr_is_not_treated_as_full_end() {
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
