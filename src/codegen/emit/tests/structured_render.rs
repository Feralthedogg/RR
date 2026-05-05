use super::common::*;
use super::*;

#[test]
pub(crate) fn raw_literal_rewrites_skip_dynamic_candidates_and_continue() {
    let mut output = "\
Sym_top_0 <- function() \n\
{\n\
  a <- rr_field_get(particles, dynamic_name) + rr_field_get(particles, \"px\")\n\
  return(c(rr_named_list(dynamic_name, px), rr_named_list(\"py\", py)))\n\
}\n"
    .to_string();

    RBackend::rewrite_literal_field_get_calls(&mut output);
    RBackend::rewrite_literal_named_list_calls(&mut output);

    assert!(
        output.contains("a <- rr_field_get(particles, dynamic_name) + particles[[\"px\"]]"),
        "{output}"
    );
    assert!(
        output.contains("return(c(rr_named_list(dynamic_name, px), list(py = py)))"),
        "{output}"
    );
}

#[test]
pub(crate) fn init_plus_scalar_conditional_loop_is_emitted_as_vector_ifelse() {
    let mut fn_ir = FnIR::new("loop_ifelse".to_string(), vec![]);
    let entry = fn_ir.add_block();
    let header = fn_ir.add_block();
    let then_bb = fn_ir.add_block();
    let else_bb = fn_ir.add_block();
    let incr_bb = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let n = fn_ir.add_value(
        ValueKind::Const(Lit::Int(8)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let zero = fn_ir.add_value(
        ValueKind::Const(Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let i_load = fn_ir.add_value(
        ValueKind::Load {
            var: "i_9".to_string(),
        },
        Span::dummy(),
        Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM),
        Some("i_9".to_string()),
    );
    let loop_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: i_load,
            rhs: n,
        },
        Span::dummy(),
        Facts::new(Facts::BOOL_SCALAR, crate::mir::flow::Interval::BOTTOM),
        None,
    );
    let clean_seed = fn_ir.add_value(
        ValueKind::Call {
            callee: "rep.int".to_string(),
            args: vec![zero, n],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        Some("clean".to_string()),
    );
    let score_load = fn_ir.add_value(
        ValueKind::Load {
            var: "score".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        Some("score".to_string()),
    );
    let clean_load = fn_ir.add_value(
        ValueKind::Load {
            var: "clean".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        Some("clean".to_string()),
    );
    let score_at_i = fn_ir.add_value(
        ValueKind::Index1D {
            base: score_load,
            idx: i_load,
            is_safe: true,
            is_na_safe: true,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let threshold = fn_ir.add_value(
        ValueKind::Const(Lit::Float(0.4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let branch_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Gt,
            lhs: score_at_i,
            rhs: threshold,
        },
        Span::dummy(),
        Facts::new(Facts::BOOL_SCALAR, crate::mir::flow::Interval::BOTTOM),
        None,
    );
    let plus_const = fn_ir.add_value(
        ValueKind::Const(Lit::Float(0.1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let then_add = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: score_at_i,
            rhs: plus_const,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let then_sqrt = fn_ir.add_value(
        ValueKind::Call {
            callee: "sqrt".to_string(),
            args: vec![then_add],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mul_const = fn_ir.add_value(
        ValueKind::Const(Lit::Float(0.55)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let else_mul = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Mul,
            lhs: score_at_i,
            rhs: mul_const,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let add_const = fn_ir.add_value(
        ValueKind::Const(Lit::Float(0.03)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let else_add = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: else_mul,
            rhs: add_const,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let inc = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: i_load,
            rhs: one,
        },
        Span::dummy(),
        Facts::new(Facts::INT_SCALAR, crate::mir::flow::Interval::BOTTOM),
        None,
    );

    fn_ir.blocks[entry].instrs.push(Instr::Assign {
        dst: "i_9".to_string(),
        src: one,
        span: Span::dummy(),
    });
    fn_ir.blocks[entry].term = Terminator::Goto(header);
    fn_ir.blocks[header].term = Terminator::If {
        cond: loop_cond,
        then_bb,
        else_bb: entry,
    };
    fn_ir.blocks[then_bb].instrs.push(Instr::StoreIndex1D {
        base: clean_load,
        idx: i_load,
        val: then_sqrt,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::dummy(),
    });
    fn_ir.blocks[then_bb].term = Terminator::Goto(incr_bb);
    fn_ir.blocks[else_bb].instrs.push(Instr::StoreIndex1D {
        base: clean_load,
        idx: i_load,
        val: else_add,
        is_safe: true,
        is_na_safe: true,
        is_vector: false,
        span: Span::dummy(),
    });
    fn_ir.blocks[else_bb].term = Terminator::Goto(incr_bb);
    fn_ir.blocks[incr_bb].instrs.push(Instr::Assign {
        dst: "i_9".to_string(),
        src: inc,
        span: Span::dummy(),
    });
    fn_ir.blocks[incr_bb].term = Terminator::Goto(header);

    let structured = StructuredBlock::Sequence(vec![
        StructuredBlock::BasicBlock(entry),
        StructuredBlock::Loop {
            header,
            cond: loop_cond,
            continue_on_true: true,
            body: Box::new(StructuredBlock::Sequence(vec![
                StructuredBlock::If {
                    cond: branch_cond,
                    then_body: Box::new(StructuredBlock::BasicBlock(then_bb)),
                    else_body: Some(Box::new(StructuredBlock::BasicBlock(else_bb))),
                },
                StructuredBlock::BasicBlock(incr_bb),
                StructuredBlock::Next,
            ])),
        },
    ]);

    let mut backend = RBackend::new();
    backend.current_fn_name = "loop_ifelse".to_string();
    backend.bind_value_to_var(clean_seed, "clean");
    backend.bind_var_to_value("clean", clean_seed);
    let StructuredBlock::Sequence(items) = &structured else {
        panic!("expected sequence");
    };
    assert_eq!(
        backend.extract_full_range_loop_guard(loop_cond, "i_9", &fn_ir),
        Some(("i_9".to_string(), n))
    );
    assert_eq!(
        backend.extract_conditional_loop_shape(match &items[1] {
            StructuredBlock::Loop { body, .. } => body.as_ref(),
            _ => panic!("expected loop"),
        }),
        Some((branch_cond, then_bb, else_bb, incr_bb))
    );
    assert_eq!(
        backend.extract_conditional_loop_store(then_bb, "i_9", n, &fn_ir),
        Some(("clean".to_string(), then_sqrt))
    );
    assert_eq!(
        backend.extract_conditional_loop_store(else_bb, "i_9", n, &fn_ir),
        Some(("clean".to_string(), else_add))
    );
    assert!(backend.loop_increment_matches(incr_bb, "i_9", &fn_ir));
    assert_eq!(
        backend.try_emit_full_range_conditional_loop_sequence(items, &fn_ir),
        Some(2)
    );

    let mut backend = RBackend::new();
    backend.current_fn_name = "loop_ifelse".to_string();
    backend.bind_value_to_var(clean_seed, "clean");
    backend.bind_var_to_value("clean", clean_seed);
    backend
        .emit_structured(&structured, &fn_ir)
        .expect("structured scalar conditional loop emission should succeed");

    assert!(!backend.output.contains("repeat {"));
    assert!(!backend.output.contains("i_9 <- 1"));
    assert!(backend.output.contains(
        "clean <- ifelse(((score > 0.4)), sqrt((score + 0.1)), ((score * 0.55) + 0.03))"
    ));
}

#[test]
pub(crate) fn stale_fresh_self_replay_after_full_update_is_skipped() {
    let mut backend = backend_with_sym17_fresh();
    let values = vec![
        Value {
            id: 0,
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
            origin_var: Some("adj_ll".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
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
                callee: "rr_assign_slice".to_string(),
                args: vec![3, 4, 0, 3],
                names: vec![],
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("adj_ll".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "adj_ll".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("initial alloc should emit");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "adj_ll".to_string(),
                src: 5,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("full update should fold to an identity");
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "adj_ll".to_string(),
                src: 3,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("stale replay should be skipped");

    let out = backend.output;
    assert_eq!(out.matches("adj_ll <- Sym_17(10L, 0L, 2L)").count(), 1);
    assert_eq!(
        out.matches("adj_ll <-").count(),
        1,
        "whole-range self update plus stale replay should both be skipped as identities: {out}"
    );
}

#[test]
pub(crate) fn earlier_same_origin_fresh_value_is_skipped_after_newer_binding() {
    let mut backend = backend_with_sym17_fresh();
    let values = vec![
        Value {
            id: 0,
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
            origin_var: Some("r".to_string()),
            phi_block: None,
            value_ty: TypeState::unknown(),
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
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
            value_term: TypeTerm::Vector(Box::new(TypeTerm::Int)),
            escape: EscapeStatus::Unknown,
        },
    ];

    backend
        .emit_instr(
            &Instr::Assign {
                dst: "r".to_string(),
                src: 4,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("newer binding should emit");
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
        .expect("stale earlier fresh value should be skipped");

    let out = backend.output;
    assert!(out.contains("r <- b"));
    assert!(!out.contains("r <- Sym_17(10L, 0L, 2L)"));
}

#[test]
pub(crate) fn loop_carried_scalar_self_update_is_emitted_as_assignment() {
    let mut backend = RBackend::new();
    backend
        .loop_analysis
        .active_loop_mutated_vars
        .push(FxHashSet::from_iter(["vy".to_string(), "y".to_string()]));

    let values = vec![
        Value {
            id: 0,
            kind: ValueKind::Load {
                var: "vy".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("vy".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 1,
            kind: ValueKind::Load {
                var: "g".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("g".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 2,
            kind: ValueKind::Load {
                var: "dt".to_string(),
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("dt".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 3,
            kind: ValueKind::Binary {
                op: BinOp::Mul,
                lhs: 1,
                rhs: 2,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: None,
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
        Value {
            id: 4,
            kind: ValueKind::Binary {
                op: BinOp::Add,
                lhs: 0,
                rhs: 3,
            },
            span: Span::dummy(),
            facts: Facts::empty(),
            origin_var: Some("vy".to_string()),
            phi_block: None,
            value_ty: TypeState::scalar(PrimTy::Double, false),
            value_term: TypeTerm::Double,
            escape: EscapeStatus::Unknown,
        },
    ];

    backend.bind_value_to_var(0, "vy");
    backend.bind_var_to_value("vy", 0);
    backend
        .emit_instr(
            &Instr::Assign {
                dst: "vy".to_string(),
                src: 4,
                span: Span::dummy(),
            },
            &values,
            &[],
        )
        .expect("loop-carried scalar self update should emit");

    assert!(
        backend.output.contains("vy <- (vy + (g * dt))"),
        "{}",
        backend.output
    );
}

#[test]
pub(crate) fn float_literals_keep_trailing_decimal_when_integral() {
    let backend = RBackend::new();
    let value = Value {
        id: 0,
        kind: ValueKind::Const(Lit::Float(5.0)),
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: None,
        phi_block: None,
        value_ty: TypeState::scalar(PrimTy::Double, false),
        value_term: TypeTerm::Double,
        escape: EscapeStatus::Unknown,
    };

    assert_eq!(backend.emit_lit_with_value(&Lit::Float(5.0), &value), "5.0");
}

#[test]
pub(crate) fn unary_neg_constant_float_is_folded_in_emission() {
    let backend = RBackend::new();
    let values = vec![Value {
        id: 0,
        kind: ValueKind::Const(Lit::Float(9.81)),
        span: Span::dummy(),
        facts: Facts::empty(),
        origin_var: None,
        phi_block: None,
        value_ty: TypeState::scalar(PrimTy::Double, false),
        value_term: TypeTerm::Double,
        escape: EscapeStatus::Unknown,
    }];

    assert_eq!(
        backend.resolve_unary_expr(UnaryOp::Neg, 0, &values, &[]),
        "-9.81"
    );
}

#[test]
pub(crate) fn marks_emit_integer_suffixes() {
    let mut backend = RBackend::new();
    backend.emit_mark(
        Span {
            start_line: 9,
            start_col: 5,
            end_line: 9,
            end_col: 5,
            ..Span::default()
        },
        None,
    );

    assert!(
        backend.output.contains("rr_mark(9L, 5L);"),
        "{}",
        backend.output
    );
}
