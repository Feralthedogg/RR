use super::type_precision_regression_common::*;

#[test]
pub(crate) fn base_data_frame_and_stats_predict_preserve_schema_and_len_sym() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["model".to_string(), "xs".to_string()],
    );
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::List(Box::new(TypeTerm::Any));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let model = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let global_env = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::globalenv".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![xs],
            names: vec![Some("x".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let field_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "x".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let field = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![df, field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pred_positional = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::predict".to_string(),
            args: vec![model, df],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pred_named = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::predict".to_string(),
            args: vec![model, df],
            names: vec![None, Some("newdata".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(pred_named));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    let xs_len = out.param_ty_hints[1].len_sym;
    assert!(xs_len.is_some());

    assert_eq!(
        out.values[df].value_term,
        TypeTerm::DataFrameNamed(vec![(
            "x".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Double)),
        )])
    );
    assert_eq!(
        out.values[global_env].value_ty,
        rr::compiler::internal::typeck::TypeState::unknown()
    );
    assert_eq!(out.values[global_env].value_term, TypeTerm::Any);
    assert_eq!(out.values[df].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[df].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[df].value_ty.len_sym, xs_len);

    assert_eq!(
        out.values[field].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[field].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[field].value_ty.prim, PrimTy::Double);

    assert_eq!(out.values[pred_positional].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[pred_positional].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[pred_positional].value_ty.len_sym, xs_len);

    assert_eq!(out.values[pred_named].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[pred_named].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[pred_named].value_ty.len_sym, xs_len);
}

#[test]
pub(crate) fn field_access_refines_from_dataframe_term() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["df".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::DataFrameNamed(vec![
        (
            "left".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Int)),
        ),
        (
            "right".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Double)),
        ),
    ]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let df = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "right".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let field = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![df, name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(field));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[field].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[field].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[field].value_ty.prim, PrimTy::Double);
}

#[test]
pub(crate) fn strict_dataframe_schema_rejects_missing_and_mismatched_fields() {
    let mut callee = FnIR::new(
        "Sym_main".to_string(),
        vec!["df".to_string(), "bad".to_string()],
    );
    callee.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Any, false);
    callee.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Char, false);
    callee.param_term_hints[0] = TypeTerm::DataFrameNamed(vec![
        (
            "left".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Int)),
        ),
        (
            "right".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Double)),
        ),
    ]);
    callee.param_term_hints[1] = TypeTerm::Char;

    let b0 = callee.add_block();
    callee.entry = b0;
    callee.body_head = b0;

    let df = callee.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bad = callee.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let missing_name = callee.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "missing".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let right_name = callee.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "right".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let missing_get = callee.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![df, missing_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let _bad_set = callee.add_value(
        ValueKind::Call {
            callee: "rr_field_set".to_string(),
            args: vec![df, right_name, bad],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    callee.blocks[b0].term = Terminator::Return(Some(missing_get));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), callee);
    let err = analyze_program(
        &mut all,
        TypeConfig {
            mode: rr::compiler::internal::typeck::TypeMode::Strict,
            native_backend: rr::compiler::internal::typeck::NativeBackend::Off,
        },
    )
    .expect_err("strict analysis must fail");
    let text = format!("{err:?}");
    assert!(text.contains("unknown field"));
    assert!(text.contains("expects"));
    assert!(!text.contains("receiver method"));
    assert!(!text.contains("explicit static type hint"));
    assert!(!text.contains("dataframe schema"));
}
