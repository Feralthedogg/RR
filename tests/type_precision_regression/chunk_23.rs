use super::type_precision_regression_common::*;

#[test]
pub(crate) fn base_direct_condition_misc_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["chars".to_string(), "ints".to_string(), "obj".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, false);
    fn_ir.param_ty_hints[2] = rr::compiler::internal::typeck::TypeState::unknown();
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Int));
    fn_ir.param_term_hints[2] = TypeTerm::Any;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let chars = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ints = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let obj = fn_ir.add_value(
        ValueKind::Param { index: 2 },
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
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![ints, chars],
            names: vec![Some("x".to_string()), Some("g".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let arg_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Arg".to_string(),
            args: vec![ints],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let by_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::by.default".to_string(),
            args: vec![ints, factor_v, obj],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let c_date_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c.Date".to_string(),
            args: vec![ints, ints],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let c_factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c.factor".to_string(),
            args: vec![factor_v, factor_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let c_noquote_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c.noquote".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let c_numver_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c.numeric_version".to_string(),
            args: vec![obj, obj],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let callcc_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::callCC".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cbind_df_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cbind.data.frame".to_string(),
            args: vec![df, df],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let close_conn_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::close.connection".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let comment_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::comment".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let restarts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::computeRestarts".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cond_msg_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::conditionMessage".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let conflicts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::conflicts".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let contributors_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::contributors".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cstack_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::Cstack_info".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let curl_headers_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::curlGetHeaders".to_string(),
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

    assert_eq!(out.values[arg_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[arg_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[arg_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    for vid in [by_v, c_numver_v, restarts_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    assert_eq!(out.values[c_date_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[c_date_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[c_date_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[c_factor_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[c_factor_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[c_factor_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    assert_eq!(out.values[c_noquote_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[c_noquote_v].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[c_noquote_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    for vid in [callcc_v, comment_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Unknown);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    assert_eq!(out.values[cbind_df_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[cbind_df_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[cbind_df_v].value_term,
        TypeTerm::DataFrame(Vec::new())
    );

    assert_eq!(out.values[close_conn_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[close_conn_v].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[close_conn_v].value_term, TypeTerm::Null);

    for vid in [cond_msg_v, conflicts_v, curl_headers_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[contributors_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[contributors_v].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[contributors_v].value_term, TypeTerm::Null);

    assert_eq!(out.values[cstack_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[cstack_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[cstack_v].value_term,
        TypeTerm::NamedList(vec![
            ("size".to_string(), TypeTerm::Int),
            ("current".to_string(), TypeTerm::Int),
            ("direction".to_string(), TypeTerm::Int),
            ("eval_depth".to_string(), TypeTerm::Int),
        ])
    );
}

#[test]
pub(crate) fn base_direct_method_family_shape_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec![
            "ints".to_string(),
            "chars".to_string(),
            "mat".to_string(),
            "df".to_string(),
        ],
    );
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Int, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Char, false);
    fn_ir.param_ty_hints[2] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Int, false);
    fn_ir.param_ty_hints[3] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Int));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Char));
    fn_ir.param_term_hints[2] = TypeTerm::Matrix(Box::new(TypeTerm::Int));
    fn_ir.param_term_hints[3] = TypeTerm::DataFrame(Vec::new());

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let ints = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chars = fn_ir.add_value(
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
    let df = fn_ir.add_value(
        ValueKind::Param { index: 3 },
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

    let aperm_default_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::aperm.default".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cut_date_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cut.Date".to_string(),
            args: vec![ints],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let diff_date_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::diff.Date".to_string(),
            args: vec![ints],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let droplevels_df_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::droplevels.data.frame".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let droplevels_factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::droplevels.factor".to_string(),
            args: vec![factor_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let duplicated_posixlt_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::duplicated.POSIXlt".to_string(),
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

    assert_eq!(out.values[aperm_default_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[aperm_default_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[aperm_default_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Int))
    );

    assert_eq!(out.values[cut_date_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[cut_date_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[cut_date_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    assert_eq!(out.values[diff_date_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[diff_date_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[diff_date_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[droplevels_df_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[droplevels_df_v].value_ty.prim, PrimTy::Any);
    assert!(matches!(
        out.values[droplevels_df_v].value_term,
        TypeTerm::DataFrame(_) | TypeTerm::DataFrameNamed(_)
    ));

    assert_eq!(
        out.values[droplevels_factor_v].value_ty.shape,
        ShapeTy::Vector
    );
    assert_eq!(out.values[droplevels_factor_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[droplevels_factor_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    assert_eq!(
        out.values[duplicated_posixlt_v].value_ty.shape,
        ShapeTy::Vector
    );
    assert_eq!(
        out.values[duplicated_posixlt_v].value_ty.prim,
        PrimTy::Logical
    );
    assert_eq!(
        out.values[duplicated_posixlt_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );
}

#[test]
pub(crate) fn base_direct_predicate_helpers_have_builtin_types() {
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

    let identical_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::identical".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let inherits_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::inherits".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let interactive_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::interactive".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_array_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.array".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_element_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.element".to_string(),
            args: vec![chars, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_finite_posixlt_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.finite.POSIXlt".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_na_df_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.na.data.frame".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_numeric_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.numeric".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_unsorted_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::is.unsorted".to_string(),
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

    for vid in [
        identical_v,
        inherits_v,
        interactive_v,
        is_array_v,
        is_numeric_v,
        is_unsorted_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }

    for vid in [is_element_v, is_finite_posixlt_v, is_na_df_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Logical))
        );
    }
}
