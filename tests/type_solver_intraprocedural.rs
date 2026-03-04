use RR::mir::{FnIR, Terminator, ValueKind};
use RR::typeck::PrimTy;
use RR::typeck::solver::{TypeConfig, analyze_program};
use RR::{mir::Facts, utils::Span};
use rustc_hash::FxHashMap;

#[test]
fn intraprocedural_solver_infers_numeric_return() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);
    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let d = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(2.5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum = fn_ir.add_value(
        ValueKind::Binary {
            op: RR::syntax::ast::BinOp::Add,
            lhs: i,
            rhs: d,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(sum));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[sum].value_ty.prim, PrimTy::Double);
    assert_eq!(out.inferred_ret_ty.prim, PrimTy::Double);
}
