use super::type_precision_regression_common::*;

#[test]
pub(crate) fn base_direct_array_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "mat".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[1] = TypeTerm::Matrix(Box::new(TypeTerm::Int));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_i = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dims = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two_i, two_i],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let array_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::array".to_string(),
            args: vec![xs, dims],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_array_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.array".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_matrix_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.matrix".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let array2df_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::array2DF".to_string(),
            args: vec![array_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let array_ind_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::arrayInd".to_string(),
            args: vec![xs, dims],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let aperm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::aperm".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::col".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::row".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_means_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::colMeans".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_means_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::rowMeans".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(xs));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[array_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[array_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[array_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );

    for vid in [as_array_v, aperm_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Int))
        );
    }

    assert_eq!(out.values[as_matrix_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[as_matrix_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[as_matrix_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[array2df_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[array2df_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[array2df_v].value_term,
        TypeTerm::DataFrame(Vec::new())
    );

    for vid in [array_ind_v, col_v, row_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Int))
        );
    }

    for vid in [col_means_v, row_means_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
}

#[test]
pub(crate) fn base_direct_introspection_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["chars".to_string()]);
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let chars = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let all_equal_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::all.equal".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let all_names_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::all.names".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let all_vars_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::all.vars".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let any_na_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::anyNA".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let args_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::args".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let body_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::body".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let call_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::call".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bquote_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::bquote".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let browser_text_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::browserText".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let browser_condition_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::browserCondition".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let capabilities_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::capabilities".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(chars));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [all_names_v, all_vars_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[any_na_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[any_na_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[any_na_v].value_term, TypeTerm::Logical);

    assert_eq!(out.values[browser_text_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[browser_text_v].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[browser_text_v].value_term, TypeTerm::Char);

    assert_eq!(out.values[capabilities_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[capabilities_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(
        out.values[capabilities_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );

    for vid in [
        all_equal_v,
        args_v,
        body_v,
        call_v,
        bquote_v,
        browser_condition_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}

#[test]
pub(crate) fn base_direct_method_family_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["chars".to_string(), "nums".to_string(), "mat".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[2] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[2] = TypeTerm::Matrix(Box::new(TypeTerm::Int));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let chars = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nums = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Param { index: 2 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let all_equal_char_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::all.equal.character".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let all_equal_num_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::all.equal.numeric".to_string(),
            args: vec![nums, nums],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let any_dup_array_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::anyDuplicated.array".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let any_dup_matrix_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::anyDuplicated.matrix".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let any_na_numver_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::anyNA.numeric_version".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let any_na_posixlt_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::anyNA.POSIXlt".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(chars));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [all_equal_char_v, all_equal_num_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    for vid in [any_dup_array_v, any_dup_matrix_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_term, TypeTerm::Int);
    }

    for vid in [any_na_numver_v, any_na_posixlt_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }
}
