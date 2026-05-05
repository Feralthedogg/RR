use super::*;

#[test]
pub(crate) fn redundant_self_replay_assignment_is_skipped() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "y".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("y".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "dy".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("dy".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
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
            facts: Facts::empty(),
            origin_var: Some("y".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("y".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("y");
    backend.bind_value_to_var(2, "y");
    backend.bind_var_to_value("y", 2);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "y".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("assign emission should succeed");

    assert!(backend.output.trim().is_empty());
}

#[test]
pub(crate) fn identical_live_binary_expr_reuses_plain_symbol_alias() {
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
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("x".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("y".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("x");
    backend.bind_value_to_var(2, "x");
    backend.bind_var_to_value("x", 2);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "y".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("assign emission should succeed");

    assert!(backend.output.contains("y <- x"), "{}", backend.output);
    assert!(
        !backend.output.contains("y <- (a + b)"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn identical_live_pure_builtin_call_reuses_plain_symbol_alias() {
    let mut backend = RBackend::new();
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Call {
                callee: "sqrt".to_string(),
                args: vec![0],
                names: vec![None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sx".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Call {
                callee: "sqrt".to_string(),
                args: vec![0],
                names: vec![None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sy".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sx");
    backend.bind_value_to_var(1, "sx");
    backend.bind_var_to_value("sx", 1);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "sy".to_string(),
                src: 2,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("assign emission should succeed");

    assert!(backend.output.contains("sy <- sx"), "{}", backend.output);
    assert!(
        !backend.output.contains("sy <- sqrt(x)"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn identical_live_pure_user_call_reuses_plain_symbol_alias() {
    let mut backend = RBackend::with_analysis_options(
        FxHashSet::default(),
        FxHashSet::from_iter(["Sym_pure".to_string()]),
        FxHashMap::default(),
        true,
    );
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
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Call {
                callee: "Sym_pure".to_string(),
                args: vec![0],
                names: vec![None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sx".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Call {
                callee: "Sym_pure".to_string(),
                args: vec![0],
                names: vec![None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sy".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sx");
    backend.bind_value_to_var(1, "sx");
    backend.bind_var_to_value("sx", 1);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "sy".to_string(),
                src: 2,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("assign emission should succeed");

    assert!(backend.output.contains("sy <- sx"), "{}", backend.output);
    assert!(
        !backend.output.contains("sy <- Sym_pure(x)"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn divergent_branch_assignments_invalidate_pre_branch_binding() {
    let mut backend = RBackend::new();
    backend
        .value_tracker
        .var_versions
        .insert("s".to_string(), 1);
    backend
        .value_tracker
        .var_value_bindings
        .insert("s".to_string(), (10, 1));

    let mut then_versions = FxHashMap::default();
    then_versions.insert("s".to_string(), 2);
    let mut then_bindings = FxHashMap::default();
    then_bindings.insert("s".to_string(), (11, 2));

    let mut else_versions = FxHashMap::default();
    else_versions.insert("s".to_string(), 2);
    let mut else_bindings = FxHashMap::default();
    else_bindings.insert("s".to_string(), (12, 2));

    backend.join_branch_var_value_bindings(
        &then_versions,
        &then_bindings,
        &else_versions,
        &else_bindings,
    );

    assert_eq!(
        backend.value_tracker.var_versions.get("s").copied(),
        Some(2)
    );
    assert!(!backend.value_tracker.var_value_bindings.contains_key("s"));
}

#[test]
pub(crate) fn loop_merge_copy_of_current_acc_value_is_skipped_after_branch_join() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "j".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("j".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
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
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("j".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Call {
                callee: "Sym_1".to_string(),
                args: vec![],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("s".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Load {
                var: "acc".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("acc".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 4,
                rhs: 3,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("acc".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 4,
                rhs: 3,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("acc".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("acc");
    backend.bind_value_to_var(5, "acc");
    backend.bind_var_to_value("acc", 5);
    backend.note_var_write("s");
    backend.bind_value_to_var(3, "s");
    backend.bind_var_to_value("s", 3);

    let then_versions = backend.value_tracker.var_versions.clone();
    let then_bindings = backend.value_tracker.var_value_bindings.clone();
    let else_versions = backend.value_tracker.var_versions.clone();
    let else_bindings = backend.value_tracker.var_value_bindings.clone();
    backend.join_branch_var_value_bindings(
        &then_versions,
        &then_bindings,
        &else_versions,
        &else_bindings,
    );

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "j".to_string(),
                src: 2,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("j update should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "acc".to_string(),
                src: 6,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("acc merge copy should emit");

    let lines: Vec<_> = backend
        .output
        .lines()
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect();
    assert!(lines.contains(&"j <- (j + 1L)".to_string()));
    assert!(!lines.contains(&"acc <- (acc + Sym_1())".to_string()));
    assert!(!lines.contains(&"acc <- (acc + s)".to_string()));
}

#[test]
pub(crate) fn reassigning_current_bound_value_is_skipped() {
    let mut backend = RBackend::new();
    let values = vec![Value {
        id: 0,
        kind: ValueKind::Load {
            var: "y".to_string(),
        },
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: Some("y".to_string()),
        phi_block: None,
        value_ty: TypeState::unknown(),
        value_term: TypeTerm::Any,
        escape: EscapeStatus::Unknown,
    }];

    backend.note_var_write("y");
    backend.bind_value_to_var(0, "y");
    backend.bind_var_to_value("y", 0);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "y".to_string(),
                src: 0,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("assign emission should succeed");

    assert!(backend.output.trim().is_empty());
}

#[test]
pub(crate) fn reassigning_same_expr_to_current_bound_var_is_skipped_even_without_origin_var() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "Sym_17".to_string(),
                args: vec![1, 2, 3],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("coriolis".to_string()),
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
            kind: ValueKind::Const(Lit::Int(3)),
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
                callee: "Sym_17".to_string(),
                args: vec![1, 2, 3],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("coriolis");
    backend.bind_value_to_var(0, "coriolis");
    backend.bind_var_to_value("coriolis", 0);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "coriolis".to_string(),
                src: 4,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("assign emission should succeed");

    assert!(backend.output.trim().is_empty());
}

#[test]
pub(crate) fn field_get_rebind_to_same_named_var_emits_when_binding_is_stale() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "p_x".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("p_x".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Call {
                callee: "Sym_186".to_string(),
                args: vec![],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("particles".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("px".to_string(), TypeTerm::Any)]),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::FieldGet {
                base: 1,
                field: "px".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("p_x".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("p_x");
    backend.bind_value_to_var(0, "p_x");
    backend.bind_var_to_value("p_x", 0);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "particles".to_string(),
                src: 1,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("particles assign should emit");

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "p_x".to_string(),
                src: 2,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("field get rebind should emit");

    assert!(
        backend
            .output
            .lines()
            .any(|line| line.trim() == "p_x <- particles[[\"px\"]]"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn equivalent_field_get_rebind_is_skipped_when_live_binding_matches_via_alias_expansion()
{
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "tools::Rd2txt_options".to_string(),
                args: vec![],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("rd_opts".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Int)]),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "rd_opts".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("rd_opts".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Int)]),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::FieldGet {
                base: 1,
                field: "width".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("rd_width".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Call {
                callee: "tools::Rd2txt_options".to_string(),
                args: vec![],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Int)]),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::FieldGet {
                base: 3,
                field: "width".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("rd_width".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "rd_opts".to_string(),
                src: 0,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("rd_opts assign should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "rd_width".to_string(),
                src: 2,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("initial rd_width assign should emit");

    let before = backend.output.clone();
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "rd_width".to_string(),
                src: 4,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("equivalent replay should be handled");

    assert_eq!(backend.output, before, "{}", backend.output);
}

#[test]
pub(crate) fn equivalent_field_get_reuses_live_plain_symbol_alias_via_alias_expansion() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "tools::Rd2txt_options".to_string(),
                args: vec![],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("rd_opts".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Int)]),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "rd_opts".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("rd_opts".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Int)]),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::FieldGet {
                base: 1,
                field: "width".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("rd_width".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Call {
                callee: "tools::Rd2txt_options".to_string(),
                args: vec![],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::NamedList(vec![("width".to_string(), TypeTerm::Int)]),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::FieldGet {
                base: 3,
                field: "width".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("width_copy".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "rd_opts".to_string(),
                src: 0,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("rd_opts assign should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "rd_width".to_string(),
                src: 2,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("initial rd_width assign should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "width_copy".to_string(),
                src: 4,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("equivalent alias should emit");

    assert!(
        backend.output.contains("width_copy <- rd_width"),
        "{}",
        backend.output
    );
    assert!(
        !backend
            .output
            .contains("width_copy <- tools:::Rd2txt_options()[[\"width\"]]"),
        "{}",
        backend.output
    );
}
