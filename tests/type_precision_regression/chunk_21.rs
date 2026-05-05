use super::type_precision_regression_common::*;

#[test]
pub(crate) fn base_direct_linear_algebra_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec![
            "chars".to_string(),
            "nums".to_string(),
            "df".to_string(),
            "mat".to_string(),
        ],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[2] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Any, false);
    fn_ir.param_ty_hints[3] =
        rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[2] = TypeTerm::DataFrame(Vec::new());
    fn_ir.param_term_hints[3] = TypeTerm::Matrix(Box::new(TypeTerm::Double));

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
    let df = fn_ir.add_value(
        ValueKind::Param { index: 2 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Param { index: 3 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let data_matrix_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.matrix".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let det_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::det".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let determinant_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::determinant".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dget_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dget".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let digamma_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::digamma".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let eigen_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::eigen".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let expm1_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::expm1".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let factorial_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::factorial".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let acosh_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::acosh".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cospi_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cospi".to_string(),
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

    assert_eq!(out.values[data_matrix_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[data_matrix_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[data_matrix_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[det_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[det_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[det_v].value_term, TypeTerm::Double);

    assert_eq!(out.values[determinant_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[determinant_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[determinant_v].value_term,
        TypeTerm::NamedList(vec![
            ("modulus".to_string(), TypeTerm::Double),
            ("sign".to_string(), TypeTerm::Int),
        ])
    );

    assert_eq!(out.values[dget_v].value_ty.shape, ShapeTy::Unknown);
    assert_eq!(out.values[dget_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[dget_v].value_term, TypeTerm::Any);

    for vid in [digamma_v, expm1_v, factorial_v, acosh_v, cospi_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    assert_eq!(out.values[eigen_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[eigen_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[eigen_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "vectors".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])
    );
}

#[test]
pub(crate) fn base_direct_format_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["chars".to_string(), "df".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[1] = TypeTerm::DataFrame(Vec::new());

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let chars = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let format_default_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::format.default".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let format_df_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::format.data.frame".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let format_info_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::format.info".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let format_pval_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::format.pval".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formatc_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::formatC".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formatdl_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::formatDL".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
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

    for vid in [format_default_v, format_pval_v, formatc_v, formatdl_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [format_df_v, format_info_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::DataFrame(Vec::new()));
    }
}

#[test]
pub(crate) fn base_direct_print_summary_method_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["chars".to_string(), "df".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[1] = TypeTerm::DataFrame(Vec::new());

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let chars = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let print_default_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::print.default".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let print_df_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::print.data.frame".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_default_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::summary.default".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_df_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::summary.data.frame".to_string(),
            args: vec![df],
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

    assert_eq!(out.values[print_default_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[print_default_v].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[print_default_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[print_df_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[print_df_v].value_ty.prim, PrimTy::Any);
    assert!(matches!(
        out.values[print_df_v].value_term,
        TypeTerm::DataFrame(_) | TypeTerm::DataFrameNamed(_)
    ));

    for vid in [summary_default_v, summary_df_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
}
