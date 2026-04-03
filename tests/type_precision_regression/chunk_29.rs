use super::type_precision_regression_common::*;

#[test]
fn compiler_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["expr".to_string()]);
    fn_ir.param_ty_hints[0] = RR::typeck::TypeState::unknown();
    fn_ir.param_term_hints[0] = TypeTerm::Any;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let expr = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let zero = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let option_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("optimize".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let suppress_all_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("suppressAll".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let suppress_undefined_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("suppressUndefined".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let suppress_no_super_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str(
            "suppressNoSuperAssignVar".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let optimize_level = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let enable_jit = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::enableJIT".to_string(),
            args: vec![zero],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_compile_flag = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(false)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let disable_pkg_compile = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::compilePKGS".to_string(),
            args: vec![pkg_compile_flag],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let compiler_opt = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::getCompilerOption".to_string(),
            args: vec![option_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let compiler_suppress_all = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::getCompilerOption".to_string(),
            args: vec![suppress_all_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let compiler_suppress_undefined = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::getCompilerOption".to_string(),
            args: vec![suppress_undefined_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let compiler_suppress_no_super = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::getCompilerOption".to_string(),
            args: vec![suppress_no_super_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set_options = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::setCompilerOptions".to_string(),
            args: vec![optimize_level],
            names: vec![Some("optimize".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set_suppress_all = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::setCompilerOptions".to_string(),
            args: vec![pkg_compile_flag],
            names: vec![Some("suppressAll".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let undefined_list = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![expr, expr],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set_suppress_undefined = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::setCompilerOptions".to_string(),
            args: vec![undefined_list],
            names: vec![Some("suppressUndefined".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set_suppress_no_super = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::setCompilerOptions".to_string(),
            args: vec![pkg_compile_flag],
            names: vec![Some("suppressNoSuperAssignVar".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let optimize_field_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("optimize".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let suppress_all_field_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("suppressAll".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let suppress_undefined_field_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("suppressUndefined".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let suppress_no_super_field_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str(
            "suppressNoSuperAssignVar".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prev_optimize = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![set_options, optimize_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prev_suppress_all = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![set_suppress_all, suppress_all_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prev_suppress_undefined = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![set_suppress_undefined, suppress_undefined_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prev_suppress_no_super = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![set_suppress_no_super, suppress_no_super_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let set_both_options = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::setCompilerOptions".to_string(),
            args: vec![optimize_level, pkg_compile_flag],
            names: vec![
                Some("optimize".to_string()),
                Some("suppressAll".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prev_both_optimize = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![set_both_options, optimize_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prev_both_suppress_all = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![set_both_options, suppress_all_field_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bytecode = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::compile".to_string(),
            args: vec![expr],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let compiled_fn = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::cmpfun".to_string(),
            args: vec![expr],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let disassembled = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::disassemble".to_string(),
            args: vec![bytecode],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let infile = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("sample.R".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let outfile = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("sample.Rc".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cmpfile = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::cmpfile".to_string(),
            args: vec![infile, outfile],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loadcmp = fn_ir.add_value(
        ValueKind::Call {
            callee: "compiler::loadcmp".to_string(),
            args: vec![outfile],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(enable_jit));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[enable_jit].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[enable_jit].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[enable_jit].value_term, TypeTerm::Int);

    assert_eq!(
        out.values[disable_pkg_compile].value_ty.shape,
        ShapeTy::Scalar
    );
    assert_eq!(
        out.values[disable_pkg_compile].value_ty.prim,
        PrimTy::Logical
    );
    assert_eq!(
        out.values[disable_pkg_compile].value_term,
        TypeTerm::Logical
    );

    assert_eq!(out.values[compiler_opt].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[compiler_opt].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[compiler_opt].value_term, TypeTerm::Int);
    assert_eq!(
        out.values[compiler_suppress_all].value_ty.shape,
        ShapeTy::Scalar
    );
    assert_eq!(
        out.values[compiler_suppress_all].value_ty.prim,
        PrimTy::Logical
    );
    assert_eq!(
        out.values[compiler_suppress_all].value_term,
        TypeTerm::Logical
    );
    assert_eq!(
        out.values[compiler_suppress_undefined].value_ty.shape,
        ShapeTy::Vector
    );
    assert_eq!(
        out.values[compiler_suppress_undefined].value_ty.prim,
        PrimTy::Char
    );
    assert_eq!(
        out.values[compiler_suppress_undefined].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(
        out.values[compiler_suppress_no_super].value_ty.shape,
        ShapeTy::Scalar
    );
    assert_eq!(
        out.values[compiler_suppress_no_super].value_ty.prim,
        PrimTy::Logical
    );
    assert_eq!(
        out.values[compiler_suppress_no_super].value_term,
        TypeTerm::Logical
    );

    assert_eq!(
        out.values[set_options].value_term,
        TypeTerm::NamedList(vec![("optimize".to_string(), TypeTerm::Double)])
    );
    assert_eq!(
        out.values[set_suppress_all].value_term,
        TypeTerm::NamedList(vec![("suppressAll".to_string(), TypeTerm::Logical)])
    );
    assert_eq!(
        out.values[set_suppress_undefined].value_term,
        TypeTerm::NamedList(vec![(
            "suppressUndefined".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        )])
    );
    assert_eq!(
        out.values[set_suppress_no_super].value_term,
        TypeTerm::NamedList(vec![(
            "suppressNoSuperAssignVar".to_string(),
            TypeTerm::Logical
        )])
    );
    assert_eq!(
        out.values[set_both_options].value_term,
        TypeTerm::NamedList(vec![
            ("optimize".to_string(), TypeTerm::Double),
            ("suppressAll".to_string(), TypeTerm::Logical),
        ])
    );
    assert_eq!(out.values[prev_optimize].value_term, TypeTerm::Double);
    assert_eq!(out.values[prev_suppress_all].value_term, TypeTerm::Logical);
    assert_eq!(
        out.values[prev_suppress_undefined].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(
        out.values[prev_suppress_no_super].value_term,
        TypeTerm::Logical
    );
    assert_eq!(out.values[prev_both_optimize].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[prev_both_suppress_all].value_term,
        TypeTerm::Logical
    );

    for vid in [bytecode, disassembled] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    for vid in [cmpfile, loadcmp] {
        assert_eq!(out.values[vid].value_ty, RR::typeck::TypeState::null());
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }

    assert_eq!(
        out.values[compiled_fn].value_ty,
        RR::typeck::TypeState::unknown()
    );
    assert_eq!(out.values[compiled_fn].value_term, TypeTerm::Any);
}

#[test]
fn stats_summary_lm_fields_have_direct_named_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let one = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(2.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(4.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let six = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(6.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ys = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![two, four, six],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![xs, ys],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("y ~ x".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![formula, df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::summary".to_string(),
            args: vec![model],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::update".to_string(),
            args: vec![model, formula],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated_summary = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::summary".to_string(),
            args: vec![updated],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sigma_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("sigma".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coefficients_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("coefficients".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let residuals_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("residuals".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let term_labels_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("term.labels".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let order_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("order".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let factors_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("factors".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let data_classes_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("dataClasses".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sigma = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary, sigma_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coefficients = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary, coefficients_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let residuals = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary, residuals_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let terms = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::terms".to_string(),
            args: vec![model],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let term_labels = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![terms, term_labels_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let order = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![terms, order_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let factors = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![terms, factors_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let terms_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("terms".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_terms = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary, terms_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_terms_order = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary_terms, order_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_terms_data_classes = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary_terms, data_classes_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated_terms = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::terms".to_string(),
            args: vec![updated],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated_sigma = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![updated_summary, sigma_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated_order = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![updated_terms, order_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let data_classes = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![terms, data_classes_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated_data_classes = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![updated_terms, data_classes_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(sigma));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[summary].value_term,
        TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            (
                "terms".to_string(),
                TypeTerm::NamedList(vec![
                    (
                        "variables".to_string(),
                        TypeTerm::List(Box::new(TypeTerm::Any)),
                    ),
                    (
                        "factors".to_string(),
                        TypeTerm::Matrix(Box::new(TypeTerm::Int)),
                    ),
                    (
                        "term.labels".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "order".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Int))
                    ),
                    ("intercept".to_string(), TypeTerm::Int),
                    ("response".to_string(), TypeTerm::Int),
                    (
                        "predvars".to_string(),
                        TypeTerm::List(Box::new(TypeTerm::Any)),
                    ),
                    (
                        "dataClasses".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "class".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char))
                    ),
                    (".Environment".to_string(), TypeTerm::Any),
                ]),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "aliased".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
            ("sigma".to_string(), TypeTerm::Double),
            ("df".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            ("r.squared".to_string(), TypeTerm::Double),
            ("adj.r.squared".to_string(), TypeTerm::Double),
            (
                "fstatistic".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "cov.unscaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])
    );
    assert_eq!(out.values[updated].value_term, out.values[model].value_term);
    assert_eq!(
        out.values[updated_summary].value_term,
        out.values[summary].value_term
    );
    assert_eq!(
        out.values[terms].value_term,
        TypeTerm::NamedList(vec![
            (
                "variables".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any)),
            ),
            (
                "factors".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Int))
            ),
            (
                "term.labels".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "order".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("intercept".to_string(), TypeTerm::Int),
            ("response".to_string(), TypeTerm::Int),
            (
                "predvars".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any)),
            ),
            (
                "dataClasses".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "class".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (".Environment".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[updated_terms].value_term,
        out.values[terms].value_term
    );
    assert_eq!(out.values[sigma].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[coefficients].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[residuals].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[term_labels].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(
        out.values[order].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[factors].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[data_classes].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(
        out.values[summary_terms].value_term,
        out.values[terms].value_term
    );
    assert_eq!(
        out.values[summary_terms_order].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[summary_terms_data_classes].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
    assert_eq!(out.values[updated_sigma].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[updated_order].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[updated_data_classes].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );
}

#[test]
fn stats_summary_glm_fields_have_direct_named_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let one = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(2.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(4.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let zero = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ys = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![zero, zero, one, one],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![xs, ys],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("y ~ x".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let family = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::binomial".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::glm".to_string(),
            args: vec![formula, df, family],
            names: vec![None, Some("data".to_string()), Some("family".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::summary".to_string(),
            args: vec![model],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::update".to_string(),
            args: vec![model, formula],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated_summary = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::summary".to_string(),
            args: vec![updated],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dispersion_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("dispersion".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coefficients_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("coefficients".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let deviance_resid_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("deviance.resid".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let order_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("order".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dispersion = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary, dispersion_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coefficients = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary, coefficients_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let deviance_resid = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary, deviance_resid_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let family_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("family".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let link_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("link".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let terms_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("terms".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_terms = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary, terms_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_family = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary, family_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_family_family = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary_family, family_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_family_link = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary_family, link_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_terms_order = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![summary_terms, order_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated_terms = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::terms".to_string(),
            args: vec![updated],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let updated_dispersion = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_field_get".to_string(),
            args: vec![updated_summary, dispersion_name],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(dispersion));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[summary].value_term,
        TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            (
                "terms".to_string(),
                TypeTerm::NamedList(vec![
                    (
                        "variables".to_string(),
                        TypeTerm::List(Box::new(TypeTerm::Any)),
                    ),
                    (
                        "factors".to_string(),
                        TypeTerm::Matrix(Box::new(TypeTerm::Int)),
                    ),
                    (
                        "term.labels".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "order".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Int))
                    ),
                    ("intercept".to_string(), TypeTerm::Int),
                    ("response".to_string(), TypeTerm::Int),
                    (
                        "predvars".to_string(),
                        TypeTerm::List(Box::new(TypeTerm::Any)),
                    ),
                    (
                        "dataClasses".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "class".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char))
                    ),
                    (".Environment".to_string(), TypeTerm::Any),
                ]),
            ),
            (
                "family".to_string(),
                TypeTerm::NamedList(vec![
                    ("family".to_string(), TypeTerm::Char),
                    ("link".to_string(), TypeTerm::Char),
                ]),
            ),
            ("deviance".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            ("contrasts".to_string(), TypeTerm::Any),
            ("df.residual".to_string(), TypeTerm::Int),
            ("null.deviance".to_string(), TypeTerm::Double),
            ("df.null".to_string(), TypeTerm::Int),
            ("iter".to_string(), TypeTerm::Int),
            (
                "deviance.resid".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "aliased".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
            ("dispersion".to_string(), TypeTerm::Double),
            ("df".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            (
                "cov.unscaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "cov.scaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])
    );
    assert_eq!(out.values[updated].value_term, out.values[model].value_term);
    assert_eq!(
        out.values[updated_summary].value_term,
        out.values[summary].value_term
    );
    assert_eq!(out.values[dispersion].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[coefficients].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[deviance_resid].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[summary_family].value_term,
        TypeTerm::NamedList(vec![
            ("family".to_string(), TypeTerm::Char),
            ("link".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(out.values[summary_family_family].value_term, TypeTerm::Char);
    assert_eq!(out.values[summary_family_link].value_term, TypeTerm::Char);
    assert_eq!(
        out.values[summary_terms].value_term,
        out.values[updated_terms].value_term
    );
    assert_eq!(
        out.values[summary_terms_order].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[updated_terms].value_term,
        TypeTerm::NamedList(vec![
            (
                "variables".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any)),
            ),
            (
                "factors".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Int)),
            ),
            (
                "term.labels".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "order".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("intercept".to_string(), TypeTerm::Int),
            ("response".to_string(), TypeTerm::Int),
            (
                "predvars".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any)),
            ),
            (
                "dataClasses".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "class".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (".Environment".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(out.values[updated_dispersion].value_term, TypeTerm::Double);
}
