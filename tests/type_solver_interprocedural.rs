use rr::Span;
use rr::compiler::internal::mir::Facts;
use rr::compiler::internal::mir::{FnIR, Terminator, ValueKind};
use rr::compiler::internal::typeck::solver::{TypeConfig, analyze_program};
use rr::compiler::internal::typeck::{PrimTy, TypeState};
use rustc_hash::FxHashMap;

#[test]
fn interprocedural_solver_propagates_user_return_type() {
    let mut callee = FnIR::new("Sym_callee".to_string(), vec!["x".to_string()]);
    callee.param_ty_hints[0] = TypeState::scalar(PrimTy::Double, true);
    let cb = callee.add_block();
    callee.entry = cb;
    callee.body_head = cb;
    let p = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = callee.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ret = callee.add_value(
        ValueKind::Binary {
            op: rr::compiler::internal::syntax::ast::BinOp::Add,
            lhs: p,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    callee.blocks[cb].term = Terminator::Return(Some(ret));

    let mut main = FnIR::new("Sym_main".to_string(), vec![]);
    let mb = main.add_block();
    main.entry = mb;
    main.body_head = mb;
    let arg = main.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(2.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let call = main.add_value(
        ValueKind::Call {
            callee: "Sym_callee".to_string(),
            args: vec![arg],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    main.blocks[mb].term = Terminator::Return(Some(call));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), main);
    all.insert("Sym_callee".to_string(), callee);

    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out_main = all.get("Sym_main").expect("main");
    assert_eq!(out_main.values[call].value_ty.prim, PrimTy::Double);
    assert_eq!(out_main.inferred_ret_ty.prim, PrimTy::Double);
}
