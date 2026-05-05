use super::common::*;

#[test]
pub(crate) fn mutable_base_prefers_origin_over_read_alias_binding() {
    let mut backend = backend_with_sym17_fresh();
    let values = vec![Value {
        id: 0,
        kind: ValueKind::Load {
            var: "out".to_string(),
        },
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: Some("out".to_string()),
        phi_block: None,
        value_ty: TypeState::unknown(),
        value_term: TypeTerm::Any,
        escape: EscapeStatus::Unknown,
    }];

    backend.bind_value_to_var(0, "a");

    assert_eq!(
        backend.resolve_mutable_base(0, &values, &[]),
        "out",
        "write bases must not be redirected through read aliases"
    );
}

#[test]
pub(crate) fn whole_dest_end_known_var_matches_param_alias_expr() {
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
            origin_var: Some("x".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
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
            id: 5,
            kind: ValueKind::Load {
                var: ".arg_size".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some(".arg_size".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: ".arg_size".to_string(),
                src: 4,
                span: Span::dummy(),
            },
            &values,
            &["size".to_string()],
        )
        .expect("param alias assignment should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "x".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &["size".to_string()],
        )
        .expect("fresh allocator assignment should emit");

    assert!(backend.whole_dest_end_matches_known_var("x", 5, &values, &["size".to_string()]));
}

#[test]
pub(crate) fn whole_range_copy_wrapper_finds_mutated_descendant_alias() {
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
            origin_var: Some("temp".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
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
            origin_var: Some("inlined_9_n".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Call {
                callee: "rep.int".to_string(),
                args: vec![1, 2],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("inlined_9_out".to_string()),
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
            origin_var: Some("inlined_9_i".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 5,
            kind: ValueKind::Load {
                var: "temp".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("temp".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 6,
            kind: ValueKind::Call {
                callee: "rr_assign_slice".to_string(),
                args: vec![3, 4, 2, 5],
                names: vec![None; 4],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("temp".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Double)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "temp".to_string(),
                src: 0,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("temp init should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "inlined_9_out".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("copy wrapper base init should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "inlined_9_out".to_string(),
                src: 6,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("whole-range copy replay should lower to direct alias");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "next_temp".to_string(),
                src: 6,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("next_temp copy alias should emit");

    backend.note_var_write("next_temp");

    let alias = backend
        .try_resolve_mutated_whole_range_copy_alias(6, &values, &[])
        .expect("mutated descendant alias should be recoverable");
    assert_eq!(alias, "next_temp");
}

#[test]
pub(crate) fn stale_fresh_aggregate_call_arg_is_not_hoisted_to_cse_temp() {
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

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "r".to_string(),
                src: 0,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("seed assign should emit");
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
        .expect("slice update should emit");
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
        .expect("dot assign should emit");

    assert!(backend.output.contains("rs_old <- Sym_117(r, r, 8"));
    assert!(!backend.output.contains(".__rr_cse_"));
}

#[test]
pub(crate) fn scalar_stage_assignment_reuses_live_origin_var() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "sun".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sun".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 0,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("next_cloud".to_string()),
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
                rhs: 0,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("next_cloud".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "next_cloud".to_string(),
                src: 1,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("stage assign should emit");
    assert_eq!(
        backend.resolve_bound_value(1).as_deref(),
        Some("next_cloud")
    );
    assert_eq!(
        backend.resolve_stale_origin_var(2, &values[2], &values),
        Some("next_cloud".to_string())
    );
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "cloud".to_string(),
                src: 2,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("copy from staged scalar should emit");

    assert!(backend.output.contains("next_cloud <- (sun + sun)"));
    assert!(
        backend
            .output
            .lines()
            .any(|line| line.trim() == "cloud <- next_cloud")
    );
    assert!(
        !backend
            .output
            .lines()
            .any(|line| line.trim() == "cloud <- (sun + sun)")
    );
}

#[test]
pub(crate) fn scalar_stage_assignment_reuses_same_value_id_bound_var() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "sun".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("sun".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 0,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("next_sun".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "next_sun".to_string(),
                src: 1,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("stage assign should emit");
    assert_eq!(backend.resolve_bound_value(1).as_deref(), Some("next_sun"));
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "sun".to_string(),
                src: 1,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("copy from same staged scalar should emit");

    assert!(backend.output.contains("next_sun <- (sun + sun)"));
    assert!(
        backend
            .output
            .lines()
            .any(|line| line.trim() == "sun <- next_sun")
    );
    assert!(
        !backend
            .output
            .lines()
            .any(|line| line.trim() == "sun <- (sun + sun)")
    );
}

#[test]
pub(crate) fn same_origin_assignment_uses_expr_instead_of_stale_bound_var() {
    let mut backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Const(Lit::Int(0)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("s".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Call {
                callee: "sum".to_string(),
                args: vec![2],
                names: vec![None],
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
            id: 2,
            kind: ValueKind::Load {
                var: "xs".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("xs".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "s".to_string(),
                src: 0,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("seed assign should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "s".to_string(),
                src: 1,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("updated assign should emit");

    assert!(backend.output.lines().any(|line| line.trim() == "s <- 0L"));
    assert!(
        backend
            .output
            .lines()
            .any(|line| line.trim() == "s <- sum(xs)")
    );
    assert!(!backend.output.lines().any(|line| line.trim() == "s <- s"));
}

#[test]
pub(crate) fn binary_expr_prefers_shared_origin_var_over_literal_clone() {
    let backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Const(Lit::Float(40.0)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("N".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "N".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("N".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Mul,
                lhs: 0,
                rhs: 1,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("grid_sq".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    let rendered = backend.resolve_val(2, &values, &[], false);
    assert_eq!(rendered, "(N * N)");
}

#[test]
pub(crate) fn binary_expr_prefers_live_scalar_origin_var_over_literal_clone() {
    let mut backend = RBackend::new();
    backend
        .value_tracker
        .var_versions
        .insert("N".to_string(), 1);
    backend
        .value_tracker
        .var_value_bindings
        .insert("N".to_string(), (0, 1));

    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Const(Lit::Float(40.0)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("N".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "rem".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("rem".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Div,
                lhs: 1,
                rhs: 0,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("tmp_div".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Mod,
                lhs: 1,
                rhs: 0,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("tmp_mod".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    let rendered_div = backend.resolve_val(2, &values, &[], false);
    let rendered_mod = backend.resolve_val(3, &values, &[], false);
    assert_eq!(rendered_div, "(rem / N)");
    assert_eq!(rendered_mod, "(rem %% N)");
}

#[test]
pub(crate) fn binary_expr_does_not_replace_literal_with_nonlive_origin_var_name() {
    let backend = RBackend::new();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "ff".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("ff".to_string()),
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
            origin_var: Some("ff".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Binary {
                op: BinOp::Lt,
                lhs: 0,
                rhs: 1,
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

    let rendered = backend.resolve_val(2, &values, &[], false);
    assert_eq!(rendered, "(ff < 1L)");
}

#[test]
pub(crate) fn const_seed_assignment_does_not_alias_to_mutable_bound_var() {
    let mut backend = RBackend::new();
    let values = vec![Value {
        id: 0,
        kind: ValueKind::Const(Lit::Int(1)),
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: None,
        phi_block: None,
        value_ty: TypeState::unknown(),
        value_term: TypeTerm::Any,
        escape: EscapeStatus::Unknown,
    }];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "acc".to_string(),
                src: 0,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("seed assign should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "i".to_string(),
                src: 0,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("second seed assign should emit");

    assert!(backend.output.contains("acc <- 1L"));
    assert!(backend.output.contains("i <- 1L"));
    assert!(!backend.output.contains("i <- acc"));
}

#[test]
pub(crate) fn indexed_store_invalidates_call_aliases_that_read_base() {
    let mut backend = RBackend::with_analysis_options(
        FxHashSet::default(),
        FxHashSet::from_iter([String::from("Sym_pure")]),
        FxHashMap::default(),
        true,
    );
    let values = vec![
        binding_test_value(
            0,
            ValueKind::Load {
                var: "u_stage".to_string(),
            },
            Some("u_stage"),
        ),
        binding_test_value(
            1,
            ValueKind::Load {
                var: "v".to_string(),
            },
            Some("v"),
        ),
        binding_test_value(2, ValueKind::Const(Lit::Int(1)), None),
        binding_test_value(3, ValueKind::Const(Lit::Int(7)), None),
        binding_test_value(
            4,
            ValueKind::Call {
                callee: "Sym_pure".to_string(),
                args: vec![0, 1],
                names: vec![],
            },
            Some("visc2"),
        ),
        binding_test_value(
            5,
            ValueKind::Call {
                callee: "Sym_pure".to_string(),
                args: vec![0, 1],
                names: vec![],
            },
            Some("visc3"),
        ),
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "visc2".to_string(),
                src: 4,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("first pure call should emit");
    backend
        .emit_instr(
            &Instr::StoreIndex1D {
                base: 0,
                idx: 2,
                val: 3,
                is_vector: false,
                is_safe: true,
                is_na_safe: true,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("indexed store should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "visc3".to_string(),
                src: 5,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("second pure call should emit");

    assert!(backend.output.contains("visc2 <- Sym_pure(u_stage, v)"));
    assert!(backend.output.contains("u_stage[1L] <- 7L"));
    assert!(backend.output.contains("visc3 <- Sym_pure(u_stage, v)"));
    assert!(!backend.output.contains("visc3 <- visc2"));
}

pub(crate) fn binding_test_value(id: usize, kind: ValueKind, origin_var: Option<&str>) -> Value {
    Value {
        id,
        kind,
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: origin_var.map(str::to_string),
        phi_block: None,
        value_ty: TypeState::unknown(),
        value_term: TypeTerm::Any,
        escape: EscapeStatus::Unknown,
    }
}
