use super::common::*;

#[test]
fn typed_parallel_wrapper_tracks_vector_local_back_to_param_slot() {
    let mut fn_ir = FnIR::new("scale".to_string(), vec!["a".to_string()]);
    fn_ir.ret_term_hint = Some(TypeTerm::Vector(Box::new(TypeTerm::Double)));
    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let param = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        Some("a".to_string()),
    );
    let load_v = fn_ir.add_value(
        ValueKind::Load {
            var: "v".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        Some("v".to_string()),
    );
    fn_ir.values[load_v].value_ty = TypeState {
        prim: PrimTy::Double,
        shape: ShapeTy::Vector,
        na: NaTy::Maybe,
        len_sym: None,
    };
    fn_ir.values[load_v].value_term = TypeTerm::Vector(Box::new(TypeTerm::Double));

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "v".to_string(),
        src: param,
        span: Span::dummy(),
    });
    fn_ir.blocks[entry].term = Terminator::Return(Some(load_v));

    let plan = RBackend::typed_parallel_wrapper_plan(&fn_ir).expect("wrapper plan should exist");
    assert_eq!(plan.slice_param_slots, vec![0]);
}

#[test]
fn resolve_val_prefers_current_var_after_indexed_store_mutates_origin() {
    let mut backend = RBackend::new();
    let seq = Value {
        id: 0,
        kind: ValueKind::Call {
            callee: "seq_len".to_string(),
            args: vec![1],
            names: vec![],
        },
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: Some("p".to_string()),
        phi_block: None,
        value_ty: TypeState::unknown(),
        value_term: TypeTerm::Any,
        escape: EscapeStatus::Unknown,
    };
    let n = Value {
        id: 1,
        kind: ValueKind::Const(Lit::Int(10)),
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: None,
        phi_block: None,
        value_ty: TypeState::unknown(),
        value_term: TypeTerm::Any,
        escape: EscapeStatus::Unknown,
    };
    let values = vec![seq, n];

    backend.note_var_write("p");
    backend.bind_value_to_var(0, "p");
    backend.bind_var_to_value("p", 0);
    backend.note_var_write("p");

    let rendered = backend.resolve_val(0, &values, &[], false);
    assert_eq!(rendered, "p");
}

#[test]
fn stale_fresh_alloc_is_rendered_as_current_var_in_call_args() {
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
            origin_var: Some("r".to_string()),
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
            id: 3,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![0, 4, 2, 5],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("r".to_string()),
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
            id: 6,
            kind: ValueKind::Call {
                callee: "Sym_117".to_string(),
                args: vec![0, 0, 2],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("rs_old".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("r");
    backend.bind_value_to_var(0, "r");
    backend.bind_var_to_value("r", 0);
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "r".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("self-update assignment should emit");

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "rs_old".to_string(),
                src: 6,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("dot assignment should emit");

    assert!(backend.output.contains("rs_old <- Sym_117(r, r, 8"));
}

#[test]
fn stale_self_copy_assignment_is_skipped() {
    let mut backend = RBackend::new();
    let values = vec![Value {
        id: 0,
        kind: ValueKind::Call {
            callee: "Sym_17".to_string(),
            args: vec![],
            names: vec![],
        },
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: Some("adj_rr".to_string()),
        phi_block: None,
        value_ty: TypeState::unknown(),
        value_term: TypeTerm::Any,
        escape: EscapeStatus::Unknown,
    }];

    backend.note_var_write("adj_rr");
    backend.bind_value_to_var(0, "adj_rr");
    backend.bind_var_to_value("adj_rr", 0);
    backend.note_var_write("adj_rr");

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "adj_rr".to_string(),
                src: 0,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("assign emission should succeed");

    assert!(
        backend.output.trim().is_empty()
            || backend
                .output
                .lines()
                .any(|line| line.trim() == "adj_rr <- Sym_17()")
    );
    assert!(
        !backend
            .output
            .lines()
            .any(|line| line.trim() == "adj_rr <- adj_rr")
    );
}

#[test]
fn stale_fresh_aggregate_without_live_binding_falls_back_to_origin_var() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![1],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("p".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(10)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("p");
    backend.bind_value_to_var(0, "p");
    backend.bind_var_to_value("p", 0);
    backend.note_var_write("p");
    backend.invalidate_var_binding("p");

    let rendered = backend.resolve_val(0, &values, &[], false);
    assert_eq!(rendered, "p");
}

#[test]
fn same_kind_assignment_after_rhs_change_is_not_skipped() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
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
    ];

    backend
        .value_tracker
        .last_assigned_value_ids
        .insert("x".to_string(), 0);
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "x".to_string(),
                src: 1,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("assign emission should succeed");

    assert!(
        backend.output.lines().any(|line| line.trim() == "x <- b"),
        "same-kind loads should still emit when the RHS source changed: {}",
        backend.output
    );
}

#[test]
fn configured_user_fresh_call_is_treated_as_fresh_aggregate() {
    let mut backend =
        RBackend::with_fresh_result_calls(FxHashSet::from_iter([String::from("Sym_custom_alloc")]));
    let values = vec![Value {
        id: 0,
        kind: ValueKind::Call {
            callee: "Sym_custom_alloc".to_string(),
            args: vec![1],
            names: vec![],
        },
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: Some("buf".to_string()),
        phi_block: None,
        value_ty: TypeState::unknown(),
        value_term: TypeTerm::Any,
        escape: EscapeStatus::Unknown,
    }];

    backend.note_var_write("buf");

    assert_eq!(
        backend.resolve_stale_origin_var(0, &values[0], &values),
        Some("buf".to_string())
    );
}

#[test]
fn configured_user_fresh_call_counts_as_full_dest_end() {
    let backend =
        RBackend::with_fresh_result_calls(FxHashSet::from_iter([String::from("Sym_custom_alloc")]));
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "Sym_custom_alloc".to_string(),
                args: vec![1, 2, 3],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("buf".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Const(Lit::Int(10)),
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
            kind: ValueKind::Const(Lit::Int(2)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    assert!(backend.value_is_full_dest_end(0, 1, &values, &[], &mut FxHashSet::default()));
}
