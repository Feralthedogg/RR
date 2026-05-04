use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn vector_binary_logical_terms_preserve_vector_shape() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["xs".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Int));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gt = fn_ir.add_value(
        ValueKind::Binary {
            op: rr::compiler::internal::syntax::ast::BinOp::Gt,
            lhs: xs,
            rhs: two,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mask = fn_ir.add_value(
        ValueKind::Binary {
            op: rr::compiler::internal::syntax::ast::BinOp::Or,
            lhs: gt,
            rhs: gt,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(mask));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[gt].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );
    assert_eq!(
        out.values[mask].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );
}

#[test]
pub(crate) fn is_na_guard_refines_branch_return_to_non_na() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["x".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::Int;

    let entry = fn_ir.add_block();
    let then_bb = fn_ir.add_block();
    let else_bb = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let x = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_x = fn_ir.add_value(
        ValueKind::Call {
            callee: "is.na".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let not_na_x = fn_ir.add_value(
        ValueKind::Unary {
            op: rr::compiler::internal::syntax::ast::UnaryOp::Not,
            rhs: is_na_x,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plus_one = fn_ir.add_value(
        ValueKind::Binary {
            op: rr::compiler::internal::syntax::ast::BinOp::Add,
            lhs: x,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fallback = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].term = Terminator::If {
        cond: not_na_x,
        then_bb,
        else_bb,
    };
    fn_ir.blocks[then_bb].term = Terminator::Return(Some(plus_one));
    fn_ir.blocks[else_bb].term = Terminator::Return(Some(fallback));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[plus_one].value_ty.na, NaTy::Never);
    assert_eq!(out.inferred_ret_ty.na, NaTy::Never);
}

#[test]
pub(crate) fn is_na_guard_refines_safe_index_read_to_non_na() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "i".to_string()],
    );
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, true);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Int));
    fn_ir.param_term_hints[1] = TypeTerm::Int;

    let entry = fn_ir.add_block();
    let then_bb = fn_ir.add_block();
    let else_bb = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let i = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_i = fn_ir.add_value(
        ValueKind::Call {
            callee: "is.na".to_string(),
            args: vec![i],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let not_na_i = fn_ir.add_value(
        ValueKind::Unary {
            op: rr::compiler::internal::syntax::ast::UnaryOp::Not,
            rhs: is_na_i,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read = fn_ir.add_value(
        ValueKind::Index1D {
            base: xs,
            idx: i,
            is_safe: true,
            is_na_safe: false,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fallback = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].term = Terminator::If {
        cond: not_na_i,
        then_bb,
        else_bb,
    };
    fn_ir.blocks[then_bb].term = Terminator::Return(Some(read));
    fn_ir.blocks[else_bb].term = Terminator::Return(Some(fallback));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[read].value_ty.na, NaTy::Never);
    let ValueKind::Index1D { is_na_safe, .. } = out.values[read].kind else {
        panic!("expected index read");
    };
    assert!(is_na_safe);
    assert_eq!(out.inferred_ret_ty.na, NaTy::Never);
}

#[test]
pub(crate) fn is_na_guard_does_not_refine_unknown_call_return() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["x".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::Int;

    let entry = fn_ir.add_block();
    let then_bb = fn_ir.add_block();
    let else_bb = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let x = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_x = fn_ir.add_value(
        ValueKind::Call {
            callee: "is.na".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let not_na_x = fn_ir.add_value(
        ValueKind::Unary {
            op: rr::compiler::internal::syntax::ast::UnaryOp::Not,
            rhs: is_na_x,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let unknown = fn_ir.add_value(
        ValueKind::Call {
            callee: "mystery".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.values[unknown].value_ty =
        rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Int, false);
    let fallback = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].term = Terminator::If {
        cond: not_na_x,
        then_bb,
        else_bb,
    };
    fn_ir.blocks[then_bb].term = Terminator::Return(Some(unknown));
    fn_ir.blocks[else_bb].term = Terminator::Return(Some(fallback));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[unknown].value_ty.na, NaTy::Maybe);
    assert_eq!(out.inferred_ret_ty.na, NaTy::Maybe);
}

#[test]
pub(crate) fn known_len_const_index_refines_non_na_read() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["xs".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, true);
    fn_ir.param_term_hints[0] = TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(3));

    let entry = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read = fn_ir.add_value(
        ValueKind::Index1D {
            base: xs,
            idx: one,
            is_safe: false,
            is_na_safe: false,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[entry].term = Terminator::Return(Some(read));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[read].value_ty.na, NaTy::Never);
    let ValueKind::Index1D { is_na_safe, .. } = out.values[read].kind else {
        panic!("expected index read");
    };
    assert!(is_na_safe);
    assert_eq!(out.inferred_ret_ty.na, NaTy::Never);
}

#[test]
pub(crate) fn vector_wide_any_is_na_guard_refines_index_read_to_non_na() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["xs".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(3));

    let entry = fn_ir.add_block();
    let then_bb = fn_ir.add_block();
    let else_bb = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "is.na".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let any_na = fn_ir.add_value(
        ValueKind::Call {
            callee: "any".to_string(),
            args: vec![is_na_xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let not_any_na = fn_ir.add_value(
        ValueKind::Unary {
            op: rr::compiler::internal::syntax::ast::UnaryOp::Not,
            rhs: any_na,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read = fn_ir.add_value(
        ValueKind::Index1D {
            base: xs,
            idx: one,
            is_safe: false,
            is_na_safe: false,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fallback = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].term = Terminator::If {
        cond: not_any_na,
        then_bb,
        else_bb,
    };
    fn_ir.blocks[then_bb].term = Terminator::Return(Some(read));
    fn_ir.blocks[else_bb].term = Terminator::Return(Some(fallback));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[read].value_ty.na, NaTy::Never);
    assert_eq!(out.inferred_ret_ty.na, NaTy::Never);
}

#[test]
pub(crate) fn vector_wide_any_is_na_guard_refines_returned_vector_to_non_na() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["xs".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Int));

    let entry = fn_ir.add_block();
    let then_bb = fn_ir.add_block();
    let else_bb = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "is.na".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let any_na = fn_ir.add_value(
        ValueKind::Call {
            callee: "any".to_string(),
            args: vec![is_na_xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let not_any_na = fn_ir.add_value(
        ValueKind::Unary {
            op: rr::compiler::internal::syntax::ast::UnaryOp::Not,
            rhs: any_na,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let zero = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fallback_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![zero],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].term = Terminator::If {
        cond: not_any_na,
        then_bb,
        else_bb,
    };
    fn_ir.blocks[then_bb].term = Terminator::Return(Some(xs));
    fn_ir.blocks[else_bb].term = Terminator::Return(Some(fallback_vec));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[xs].value_ty.na, NaTy::Maybe);
    assert_eq!(out.inferred_ret_ty.na, NaTy::Never);
}

#[test]
pub(crate) fn plain_vector_is_na_guard_does_not_refine_all_elements() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["xs".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(3));

    let entry = fn_ir.add_block();
    let then_bb = fn_ir.add_block();
    let else_bb = fn_ir.add_block();
    fn_ir.entry = entry;
    fn_ir.body_head = entry;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "is.na".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let not_is_na_xs = fn_ir.add_value(
        ValueKind::Unary {
            op: rr::compiler::internal::syntax::ast::UnaryOp::Not,
            rhs: is_na_xs,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let read = fn_ir.add_value(
        ValueKind::Index1D {
            base: xs,
            idx: one,
            is_safe: false,
            is_na_safe: false,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fallback = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[entry].term = Terminator::If {
        cond: not_is_na_xs,
        then_bb,
        else_bb,
    };
    fn_ir.blocks[then_bb].term = Terminator::Return(Some(read));
    fn_ir.blocks[else_bb].term = Terminator::Return(Some(fallback));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[read].value_ty.na, NaTy::Maybe);
    assert_eq!(out.inferred_ret_ty.na, NaTy::Maybe);
}

#[test]
pub(crate) fn builtin_vector_calls_preserve_len_sym_and_numeric_precision() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "ys".to_string()],
    );
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Int));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ys = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let _four = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let abs_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "abs".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pmax_scalar = fn_ir.add_value(
        ValueKind::Call {
            callee: "pmax".to_string(),
            args: vec![xs, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let log_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "log10".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "is.na".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum_xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "sum".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pmax_zip = fn_ir.add_value(
        ValueKind::Call {
            callee: "pmax".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(sum_xs));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    let xs_len = out.param_ty_hints[0].len_sym;
    let ys_len = out.param_ty_hints[1].len_sym;
    assert!(xs_len.is_some());
    assert!(ys_len.is_some());
    assert_ne!(xs_len, ys_len);

    assert_eq!(out.values[abs_xs].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[abs_xs].value_ty.len_sym, xs_len);

    assert_eq!(out.values[pmax_scalar].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[pmax_scalar].value_ty.len_sym, xs_len);

    assert_eq!(out.values[log_xs].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[log_xs].value_ty.len_sym, xs_len);

    assert_eq!(out.values[is_na_xs].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[is_na_xs].value_ty.len_sym, xs_len);

    assert_eq!(out.values[sum_xs].value_ty.prim, PrimTy::Int);

    assert_eq!(out.values[pmax_zip].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[pmax_zip].value_ty.len_sym, None);
}

#[test]
pub(crate) fn builtin_na_precision_keeps_definite_non_na_results() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let na = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Na),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let true_arg = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(true)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let clean_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dirty_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, na],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_dirty = fn_ir.add_value(
        ValueKind::Call {
            callee: "is.na".to_string(),
            args: vec![dirty_vec],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let finite_clean = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.finite".to_string(),
            args: vec![clean_vec],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let which_na = fn_ir.add_value(
        ValueKind::Call {
            callee: "which".to_string(),
            args: vec![is_na_dirty],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rep_clean = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::rep.int".to_string(),
            args: vec![one, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rep_dirty = fn_ir.add_value(
        ValueKind::Call {
            callee: "rep.int".to_string(),
            args: vec![na, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let numeric_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "numeric".to_string(),
            args: vec![three],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum_clean = fn_ir.add_value(
        ValueKind::Call {
            callee: "sum".to_string(),
            args: vec![clean_vec],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum_dirty_na_rm = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sum".to_string(),
            args: vec![dirty_vec, true_arg],
            names: vec![None, Some("na.rm".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mean_dirty_na_rm = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::mean".to_string(),
            args: vec![dirty_vec, true_arg],
            names: vec![None, Some("na.rm".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(sum_dirty_na_rm));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[clean_vec].value_ty.na, NaTy::Never);
    assert_eq!(out.values[dirty_vec].value_ty.na, NaTy::Maybe);
    for vid in [
        is_na_dirty,
        finite_clean,
        which_na,
        rep_clean,
        numeric_vec,
        sum_clean,
    ] {
        assert_eq!(out.values[vid].value_ty.na, NaTy::Never, "{vid:?}");
    }
    assert_eq!(out.values[rep_dirty].value_ty.na, NaTy::Maybe);
    assert_eq!(out.values[sum_dirty_na_rm].value_ty.na, NaTy::Never);
    assert_eq!(out.values[mean_dirty_na_rm].value_ty.na, NaTy::Maybe);
}
