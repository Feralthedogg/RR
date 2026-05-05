use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn base_direct_core_helpers_have_builtin_types() {
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
    let true_v = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(true)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let false_v = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Bool(false)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let alpha = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "alpha".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let beta = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "beta".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fmt = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "%s-%d".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let chars = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::character".to_string(),
            args: vec![three],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let flags = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::logical".to_string(),
            args: vec![three],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ids = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::integer".to_string(),
            args: vec![two],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::double".to_string(),
            args: vec![two],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let list_mode = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "list".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let char_mode = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "character".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vector_char = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::vector".to_string(),
            args: vec![char_mode, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vector_list = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::vector".to_string(),
            args: vec![list_mode, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rep = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::rep".to_string(),
            args: vec![two, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bools = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![false_v, true_v, false_v],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let any_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::any".to_string(),
            args: vec![bools],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let all_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::all".to_string(),
            args: vec![bools],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let which_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::which".to_string(),
            args: vec![bools],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nums = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![two, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prod_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::prod".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sum_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sum".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mean_nums = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![two, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mean_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::mean".to_string(),
            args: vec![mean_nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let length_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::length".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let numeric_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::numeric".to_string(),
            args: vec![three],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let c_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let r1 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "r1".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let r2 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "r2".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let c1 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "c1".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let c2 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "c2".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_names = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![r1, r2],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_names = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![c1, c2],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dimnames_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::list".to_string(),
            args: vec![row_names, col_names],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matrix_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, two],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matrix_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![matrix_data, two, two, dimnames_v],
            names: vec![
                None,
                Some("nrow".to_string()),
                Some("ncol".to_string()),
                Some("dimnames".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dim".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dimnames_lookup = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dimnames".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nrow_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::nrow".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ncol_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::ncol".to_string(),
            args: vec![matrix_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let paste_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::paste".to_string(),
            args: vec![alpha, beta],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let paste0_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::paste0".to_string(),
            args: vec![alpha, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sprintf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::sprintf".to_string(),
            args: vec![fmt, alpha, two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cat_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cat".to_string(),
            args: vec![alpha, beta],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(cat_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[chars].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[vector_char].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[vector_list].value_ty.shape, ShapeTy::Vector);
    for vid in [paste_v, paste0_v, sprintf_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
    }
    assert_eq!(out.values[chars].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[vector_char].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[vector_list].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[flags].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[ids].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[vals].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[any_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[all_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[which_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[prod_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[sum_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[mean_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[length_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[numeric_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[c_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[paste_v].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[paste0_v].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[sprintf_v].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[matrix_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[dim_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[nrow_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[ncol_v].value_ty.prim, PrimTy::Int);

    assert_eq!(
        out.values[chars].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(
        out.values[flags].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );
    assert_eq!(
        out.values[ids].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[vals].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[vector_char].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(
        out.values[vector_list].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );
    assert_eq!(
        out.values[rep].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(out.values[any_v].value_term, TypeTerm::Logical);
    assert_eq!(out.values[all_v].value_term, TypeTerm::Logical);
    assert_eq!(
        out.values[which_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(out.values[prod_v].value_term, TypeTerm::Double);
    assert_eq!(out.values[sum_v].value_term, TypeTerm::Int);
    assert_eq!(out.values[mean_v].value_term, TypeTerm::Double);
    assert_eq!(out.values[length_v].value_term, TypeTerm::Int);
    assert_eq!(
        out.values[numeric_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[c_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(out.values[paste_v].value_term, TypeTerm::Char);
    assert_eq!(out.values[paste0_v].value_term, TypeTerm::Char);
    assert_eq!(out.values[sprintf_v].value_term, TypeTerm::Char);
    assert_eq!(
        out.values[matrix_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[dim_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[dimnames_lookup].value_term,
        TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(TypeTerm::Char))))
    );
    assert_eq!(out.values[nrow_v].value_term, TypeTerm::Int);
    assert_eq!(out.values[ncol_v].value_term, TypeTerm::Int);
    assert_eq!(out.values[cat_v].value_term, TypeTerm::Null);
}
