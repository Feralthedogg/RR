use super::*;
#[test]
pub(crate) fn named_call_args_reuse_live_plain_symbol_alias_for_identical_scalar_exprs() {
    let mut backend = RBackend::new();
    let int_scalar_ty = TypeState::scalar(PrimTy::Int, false);
    let int_scalar_facts = Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM);
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
            value_ty: int_scalar_ty,
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
            value_ty: int_scalar_ty,
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
            facts: int_scalar_facts,
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: int_scalar_ty,
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
            facts: int_scalar_facts,
            origin_var: None,
            phi_block: None,
            value_ty: int_scalar_ty,
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Call {
                callee: "round".to_string(),
                args: vec![3],
                names: vec![Some("x".to_string())],
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

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    let rendered = backend.resolve_val(4, &values, &[], false);
    assert_eq!(rendered, "round(x = sum_ab)");
}

#[test]
pub(crate) fn store_index_uses_live_plain_symbol_aliases() {
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
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Len { base: 2 },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("n".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Int,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
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
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);
    backend.note_var_write("n");
    backend.bind_value_to_var(3, "n");
    backend.bind_var_to_value("n", 3);

    backend
        .emit_instr(
            &Instr::StoreIndex1D {
                base: 4,
                idx: 2,
                val: 3,
                is_vector: false,
                is_safe: false,
                is_na_safe: false,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("store should emit");

    assert!(
        backend
            .output
            .contains("x[rr_index1_write(sum_ab, \"index\")] <- n"),
        "{}",
        backend.output
    );
    assert!(
        !backend
            .output
            .contains("x[rr_index1_write((a + b), \"index\")] <- length(sum_ab)"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn generic_call_arg_reuses_live_plain_symbol_alias() {
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
            origin_var: Some("sum_ab".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Call {
                callee: "Sym_use".to_string(),
                args: vec![2],
                names: vec![None],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("out".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.note_var_write("sum_ab");
    backend.bind_value_to_var(2, "sum_ab");
    backend.bind_var_to_value("sum_ab", 2);

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "out".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("call assign should emit");

    assert!(
        backend.output.contains("out <- Sym_use(sum_ab)"),
        "{}",
        backend.output
    );
    assert!(
        !backend.output.contains("out <- Sym_use((a + b))"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn strip_unreachable_sym_helpers_drops_unreferenced_helper_function() {
    let mut output = [
        "main <- Sym_top_0",
        "Sym_top_0 <- function() ",
        "{",
        "  return(Sym_live_1())",
        "}",
        "Sym_live_1 <- function() ",
        "{",
        "  return(1L)",
        "}",
        "Sym_dead_2 <- function() ",
        "{",
        "  return(2L)",
        "}",
        "",
    ]
    .join("\n");

    RBackend::strip_unreachable_sym_helpers(&mut output);

    assert!(output.contains("Sym_top_0 <- function()"), "{}", output);
    assert!(output.contains("Sym_live_1 <- function()"), "{}", output);
    assert!(!output.contains("Sym_dead_2 <- function()"), "{}", output);
}

#[test]
pub(crate) fn strip_redundant_tail_assign_slice_return_drops_tail_assign() {
    let mut output = [
        "Sym_1 <- function() ",
        "{",
        "  i <- 1",
        "  .tachyon_exprmap0_1 <- rr_map_int(x, f)",
        "  repeat {",
        "if (!(i <= n)) break",
        "x <- rr_assign_slice(x, i, n, .tachyon_exprmap0_1)",
        "next",
        "  }",
        "  x <- rr_assign_slice(x, 1, n, .tachyon_exprmap0_1)",
        "  return(x)",
        "}",
        "",
    ]
    .join("\n");

    RBackend::strip_redundant_tail_assign_slice_return(&mut output);

    assert!(
        !output.contains("\n  x <- rr_assign_slice(x, 1, n, .tachyon_exprmap0_1)\n"),
        "{}",
        output
    );
    assert!(output.contains("  return(x)"), "{}", output);
}

#[test]
pub(crate) fn stale_fresh_clone_selection_is_deterministic_across_binding_insertion_order() {
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Call {
                callee: "Sym_17".to_string(),
                args: vec![3, 4, 5],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("beta".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Call {
                callee: "Sym_17".to_string(),
                args: vec![3, 4, 5],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("alpha".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Call {
                callee: "Sym_17".to_string(),
                args: vec![3, 4, 5],
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
        Value {
            id: 3,
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
            id: 4,
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
            id: 5,
            kind: ValueKind::Const(Lit::Int(3)),
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Unknown,
        },
    ];

    let mut backend_a = backend_with_sym17_fresh();
    backend_a
        .value_tracker
        .var_versions
        .insert("beta".to_string(), 1);
    backend_a
        .value_tracker
        .var_versions
        .insert("alpha".to_string(), 1);
    backend_a
        .value_tracker
        .value_bindings
        .insert(0, ("beta".to_string(), 0));
    backend_a
        .value_tracker
        .value_bindings
        .insert(1, ("alpha".to_string(), 0));

    let mut backend_b = backend_with_sym17_fresh();
    backend_b
        .value_tracker
        .var_versions
        .insert("beta".to_string(), 1);
    backend_b
        .value_tracker
        .var_versions
        .insert("alpha".to_string(), 1);
    backend_b
        .value_tracker
        .value_bindings
        .insert(1, ("alpha".to_string(), 0));
    backend_b
        .value_tracker
        .value_bindings
        .insert(0, ("beta".to_string(), 0));

    assert_eq!(
        backend_a.resolve_stale_fresh_clone_var(2, &values[2], &values),
        Some("alpha".to_string())
    );
    assert_eq!(
        backend_b.resolve_stale_fresh_clone_var(2, &values[2], &values),
        Some("alpha".to_string())
    );
}

#[test]
pub(crate) fn loop_local_reseed_is_not_skipped_when_var_is_mutated_in_loop() {
    let mut fn_ir = FnIR::new("loop_reset".to_string(), vec![]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let body = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cond = fn_ir.add_value(
        ValueKind::Const(Lit::Bool(true)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: one,
        span: Span::dummy(),
    });
    fn_ir.blocks[entry].term = Terminator::Unreachable;
    fn_ir.blocks[header].term = Terminator::If {
        cond,
        then_bb: body,
        else_bb: entry,
    };
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: one,
        span: Span::dummy(),
    });
    fn_ir.blocks[body].instrs.push(Instr::Assign {
        dst: "i".to_string(),
        src: two,
        span: Span::dummy(),
    });
    fn_ir.blocks[body].term = Terminator::Unreachable;

    let structured = StructuredBlock::Sequence(vec![
        StructuredBlock::BasicBlock(entry),
        StructuredBlock::Loop {
            header,
            cond,
            continue_on_true: true,
            body: Box::new(StructuredBlock::BasicBlock(body)),
        },
    ]);

    let mut backend = RBackend::new();
    backend.current_fn_name = "loop_reset".to_string();
    backend
        .emit_structured(&structured, &fn_ir)
        .expect("structured loop emission should succeed");

    assert_eq!(backend.output.matches("i <- 1").count(), 2);
    assert!(backend.output.contains("i <- 2"));
}

#[test]
pub(crate) fn generated_loop_seed_assign_emits_raw_const() {
    let mut backend = RBackend::new();
    backend.current_fn_name = "generated_loop_seed".to_string();
    let values = vec![Value {
        id: 0,
        kind: ValueKind::Const(Lit::Int(1)),
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: Some(".__poly_gen_iv_tile_2_c".to_string()),
        phi_block: None,
        value_ty: TypeState::unknown(),
        value_term: TypeTerm::Any,
        escape: EscapeStatus::Local,
    }];
    backend
        .emit_instr(
            &Instr::Assign {
                dst: ".__poly_gen_iv_tile_2_c".to_string(),
                src: 0,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("generated loop seed assign should emit");
    assert!(
        backend.output.contains(".__poly_gen_iv_tile_2_c <- 1L"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn generated_loop_step_assign_emits_raw_self_increment() {
    let mut backend = RBackend::new();
    backend.current_fn_name = "generated_loop_step".to_string();
    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: ".__poly_gen_iv_2_c".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some(".__poly_gen_iv_2_c".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Local,
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
            escape: EscapeStatus::Local,
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
            origin_var: Some(".__poly_gen_iv_2_c".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Any,
            escape: EscapeStatus::Local,
        },
    ];
    backend
        .emit_instr(
            &Instr::Assign {
                dst: ".__poly_gen_iv_2_c".to_string(),
                src: 2,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("generated loop step assign should emit");
    assert!(
        backend
            .output
            .contains(".__poly_gen_iv_2_c <- (.__poly_gen_iv_2_c + 1L)"),
        "{}",
        backend.output
    );
}
