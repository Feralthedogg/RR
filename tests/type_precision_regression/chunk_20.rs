use super::type_precision_regression_common::*;

#[test]
pub(crate) fn base_direct_cumulative_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["ints".to_string(), "mat".to_string()],
    );
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, false);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Int, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Int));
    fn_ir.param_term_hints[1] = TypeTerm::Matrix(Box::new(TypeTerm::Int));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let ints = fn_ir.add_value(
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

    let cumsum_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cumsum".to_string(),
            args: vec![ints],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cumprod_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cumprod".to_string(),
            args: vec![ints],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cummax_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cummax".to_string(),
            args: vec![ints],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let diff_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::diff".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let complete_cases_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::complete.cases".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let complex_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::complex".to_string(),
            args: vec![ints],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let drop_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::drop".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let duplicated_default_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::duplicated.default".to_string(),
            args: vec![ints],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(ints));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [cumsum_v, cummax_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }

    assert_eq!(out.values[cumprod_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[cumprod_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[cumprod_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[diff_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[diff_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[diff_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Int))
    );

    assert_eq!(out.values[complete_cases_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[complete_cases_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(
        out.values[complete_cases_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );

    assert_eq!(out.values[complex_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[complex_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[complex_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Any))
    );

    assert_eq!(out.values[drop_v].value_ty.shape, ShapeTy::Unknown);
    assert_eq!(out.values[drop_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[drop_v].value_term, TypeTerm::Any);

    assert_eq!(
        out.values[duplicated_default_v].value_ty.shape,
        ShapeTy::Vector
    );
    assert_eq!(
        out.values[duplicated_default_v].value_ty.prim,
        PrimTy::Logical
    );
    assert_eq!(
        out.values[duplicated_default_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );
}

#[test]
pub(crate) fn base_direct_setter_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec![
            "chars".to_string(),
            "nums".to_string(),
            "mat".to_string(),
            "obj".to_string(),
        ],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[2] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Int, false);
    fn_ir.param_ty_hints[3] = rr::compiler::internal::typeck::TypeState::unknown();
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[2] = TypeTerm::Matrix(Box::new(TypeTerm::Int));
    fn_ir.param_term_hints[3] = TypeTerm::Any;

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
    let obj = fn_ir.add_value(
        ValueKind::Param { index: 3 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![nums, chars],
            names: vec![Some("x".to_string()), Some("g".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::factor".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let attr_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::attr<-".to_string(),
            args: vec![df, chars, nums],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let attrs_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::attributes<-".to_string(),
            args: vec![df, obj],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let body_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::body<-".to_string(),
            args: vec![obj, obj],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::class<-".to_string(),
            args: vec![df, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let colnames_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::colnames<-".to_string(),
            args: vec![mat, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let comment_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::comment<-".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dimnames_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dimnames<-".to_string(),
            args: vec![mat, obj],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let levels_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::levels<-".to_string(),
            args: vec![factor_v, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let names_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::names<-".to_string(),
            args: vec![nums, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_names_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::row.names<-".to_string(),
            args: vec![df, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rownames_set_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::rownames<-".to_string(),
            args: vec![mat, chars],
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

    for vid in [attr_set_v, attrs_set_v, class_set_v, row_names_set_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert!(matches!(
            out.values[vid].value_term,
            TypeTerm::DataFrame(_) | TypeTerm::DataFrameNamed(_)
        ));
    }

    for vid in [colnames_set_v, dimnames_set_v, rownames_set_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Int))
        );
    }

    assert_eq!(out.values[comment_set_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[comment_set_v].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[comment_set_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[levels_set_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[levels_set_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[levels_set_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    assert_eq!(out.values[names_set_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[names_set_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[names_set_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[body_set_v].value_ty.shape, ShapeTy::Unknown);
    assert_eq!(out.values[body_set_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[body_set_v].value_term, TypeTerm::Any);
}

#[test]
pub(crate) fn base_direct_command_encoding_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["chars".to_string(), "nums".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));

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

    let command_args_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::commandArgs".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let data_class_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.class".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let date_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::date".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let deparse_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::deparse".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let deparse1_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::deparse1".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dquote_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::dQuote".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let enc2native_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::enc2native".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let encoding_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Encoding".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ext_soft_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::extSoftVersion".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let find_interval_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::findInterval".to_string(),
            args: vec![nums, nums],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_choose_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.choose".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let file_show_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::file.show".to_string(),
            args: vec![chars],
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

    for vid in [command_args_v, data_class_v, deparse_v, ext_soft_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [date_v, deparse1_v, file_choose_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }

    for vid in [dquote_v, enc2native_v, encoding_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[find_interval_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[find_interval_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[find_interval_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    assert_eq!(out.values[file_show_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[file_show_v].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[file_show_v].value_term, TypeTerm::Null);
}
