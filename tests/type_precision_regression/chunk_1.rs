use super::type_precision_regression_common::*;

#[test]
pub(crate) fn arithmetic_keeps_int_double_boundary_precise() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);
    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let i5 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let i2 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let div = fn_ir.add_value(
        ValueKind::Binary {
            op: rr::compiler::internal::syntax::ast::BinOp::Div,
            lhs: i5,
            rhs: i2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let modu = fn_ir.add_value(
        ValueKind::Binary {
            op: rr::compiler::internal::syntax::ast::BinOp::Mod,
            lhs: i5,
            rhs: i2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(div));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[div].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[modu].value_ty.prim, PrimTy::Int);
}

#[test]
pub(crate) fn hir_type_lowering_preserves_option_union_and_result_shape() {
    let option_int = hir_ty_to_type_state(&Ty::Option(Box::new(Ty::Int)));
    assert_eq!(option_int.prim, PrimTy::Int);
    assert_eq!(option_int.shape, ShapeTy::Scalar);
    assert_eq!(option_int.na, NaTy::Maybe);

    let union_num = hir_ty_to_type_state(&Ty::Union(vec![Ty::Int, Ty::Double]));
    assert_eq!(union_num.prim, PrimTy::Double);
    assert_eq!(union_num.shape, ShapeTy::Scalar);

    let result_num = hir_ty_to_type_state(&Ty::Result(Box::new(Ty::Int), Box::new(Ty::Double)));
    assert_eq!(result_num.prim, PrimTy::Double);
    assert_eq!(result_num.shape, ShapeTy::Scalar);

    let matrix_float = hir_ty_to_type_state(&Ty::Matrix(Box::new(Ty::Double)));
    assert_eq!(matrix_float.prim, PrimTy::Double);
    assert_eq!(matrix_float.shape, ShapeTy::Matrix);

    let df = hir_ty_to_type_state(&Ty::DataFrame(vec![]));
    assert_eq!(df.shape, ShapeTy::Matrix);

    let matrix_term = hir_ty_to_type_term(&Ty::Matrix(Box::new(Ty::Double)));
    assert_eq!(matrix_term, TypeTerm::Matrix(Box::new(TypeTerm::Double)));

    let df_term = hir_ty_to_type_term(&Ty::DataFrame(vec![
        (rr::compiler::internal::hir::def::SymbolId(1), Ty::Int),
        (rr::compiler::internal::hir::def::SymbolId(2), Ty::Double),
    ]));
    assert_eq!(
        df_term,
        TypeTerm::DataFrame(vec![TypeTerm::Int, TypeTerm::Double])
    );
}

#[test]
pub(crate) fn index_refines_scalar_type_from_structural_term() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["xs".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Int));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let p = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let idx = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let elem = fn_ir.add_value(
        ValueKind::Index1D {
            base: p,
            idx,
            is_safe: false,
            is_na_safe: false,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(elem));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[p].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[p].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[elem].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[elem].value_ty.shape, ShapeTy::Scalar);
}
