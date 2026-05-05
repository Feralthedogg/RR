use super::type_precision_regression_common::*;

#[test]
pub(crate) fn base_direct_datetime_helpers_have_builtin_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["chars".to_string(), "dates".to_string()],
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
    let dates = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let as_date_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.Date".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_posixct_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.POSIXct".to_string(),
            args: vec![dates],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_posixlt_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.POSIXlt".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_difftime_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.difftime".to_string(),
            args: vec![dates],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_double_posixlt_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.double.POSIXlt".to_string(),
            args: vec![dates],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_char_date_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.character.Date".to_string(),
            args: vec![dates],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let format_date_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::format.Date".to_string(),
            args: vec![dates],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let months_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::months".to_string(),
            args: vec![dates],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let julian_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::julian".to_string(),
            args: vec![dates],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let olson_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::OlsonNames".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let isodate_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::ISOdate".to_string(),
            args: vec![dates, dates, dates],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seq_date_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::seq.Date".to_string(),
            args: vec![dates],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let strptime_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::strptime".to_string(),
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

    for vid in [
        as_date_v,
        as_posixct_v,
        as_posixlt_v,
        as_difftime_v,
        as_double_posixlt_v,
        julian_v,
        isodate_v,
        seq_date_v,
        strptime_v,
    ] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    for vid in [as_char_date_v, format_date_v, months_v, olson_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }
}

#[test]
pub(crate) fn base_direct_coercion_helpers_have_builtin_types() {
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

    let abbrev_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::abbreviate".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let append_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::append".to_string(),
            args: vec![nums, nums],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_char_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.character".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_double_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.double".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.factor".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let add_na_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::addNA".to_string(),
            args: vec![as_factor_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_logical_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.logical".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_vector_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.vector".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::class".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let attr_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::attr".to_string(),
            args: vec![nums, chars],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let attributes_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::attributes".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let levels_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::levels".to_string(),
            args: vec![as_factor_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ordered_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::ordered".to_string(),
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

    for vid in [abbrev_v, as_char_v, class_v, levels_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    {
        let vid = as_double_v;
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    for vid in [as_factor_v, add_na_v, ordered_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }

    assert_eq!(out.values[as_logical_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[as_logical_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(
        out.values[as_logical_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );

    assert_eq!(out.values[append_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[append_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[append_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[as_vector_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[as_vector_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[as_vector_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[attr_v].value_ty.shape, ShapeTy::Unknown);
    assert_eq!(out.values[attr_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[attr_v].value_term, TypeTerm::Any);

    assert_eq!(out.values[attributes_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[attributes_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[attributes_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );
}

#[test]
pub(crate) fn base_direct_coercion_method_helpers_have_builtin_types() {
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
            args: vec![nums, chars],
            names: vec![Some("x".to_string()), Some("g".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lst = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::list".to_string(),
            args: vec![chars, nums],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let as_array_default_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.array.default".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_char_factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.character.factor".to_string(),
            args: vec![factor_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_char_default_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.character.default".to_string(),
            args: vec![nums],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_df_char_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.data.frame.character".to_string(),
            args: vec![chars],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_df_list_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.data.frame.list".to_string(),
            args: vec![lst],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_df_matrix_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.data.frame.matrix".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_list_df_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.list.data.frame".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_list_factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.list.factor".to_string(),
            args: vec![factor_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_matrix_df_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.matrix.data.frame".to_string(),
            args: vec![df],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_vector_factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.vector.factor".to_string(),
            args: vec![factor_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_logical_factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::as.logical.factor".to_string(),
            args: vec![factor_v],
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

    assert_eq!(
        out.values[as_array_default_v].value_ty.shape,
        ShapeTy::Matrix
    );
    assert_eq!(out.values[as_array_default_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[as_array_default_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );

    for vid in [as_char_factor_v, as_char_default_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    for vid in [as_df_char_v, as_df_list_v, as_df_matrix_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::DataFrame(Vec::new()));
    }

    for vid in [as_list_df_v, as_list_factor_v] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    assert_eq!(out.values[as_matrix_df_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[as_matrix_df_v].value_ty.prim, PrimTy::Any);
    assert!(matches!(
        out.values[as_matrix_df_v].value_term,
        TypeTerm::Matrix(_)
    ));

    assert_eq!(
        out.values[as_vector_factor_v].value_ty.shape,
        ShapeTy::Vector
    );
    assert_eq!(out.values[as_vector_factor_v].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[as_vector_factor_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    assert_eq!(
        out.values[as_logical_factor_v].value_ty.shape,
        ShapeTy::Vector
    );
    assert_eq!(
        out.values[as_logical_factor_v].value_ty.prim,
        PrimTy::Logical
    );
    assert_eq!(
        out.values[as_logical_factor_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    );
}
