use super::type_precision_regression_common::*;

#[test]
fn stats_model_plumbing_helpers_have_direct_types() {
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
    let five = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(5.0)),
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
    let y_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two, four, six],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let z_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![three, four, five],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![y_vals, x_vals, z_vals],
            names: vec![
                Some("y".to_string()),
                Some("x".to_string()),
                Some("z".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("y ~ x + z".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let update_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str(". ~ .".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let update_formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str(". ~ . + x".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let keep_true = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(true)),
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
    let update_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![update_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let update_formula2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![update_formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![formula, df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_frame_default_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.frame.default".to_string(),
            args: vec![formula, df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_matrix_default_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix.default".to_string(),
            args: vec![formula, df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_matrix_lm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.matrix.lm".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let update_default_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::update.default".to_string(),
            args: vec![fit, update_formula],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let update_formula_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::update.formula".to_string(),
            args: vec![formula, update_formula2],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glm_control_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::glm.control".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let terms_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::terms".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let drop_terms_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::drop.terms".to_string(),
            args: vec![terms_v, one_i, keep_true],
            names: vec![None, None, Some("keep.response".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let empty_formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("y ~ 1".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let empty_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![empty_formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let empty_fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![empty_formula, df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let empty_terms = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::terms".to_string(),
            args: vec![empty_fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_empty_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::is.empty.model".to_string(),
            args: vec![empty_terms],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(model_matrix_default_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[model_frame_default_v].value_ty.shape,
        ShapeTy::Matrix
    );
    assert_eq!(
        out.values[model_frame_default_v].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "z".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[model_matrix_default_v].value_ty.prim,
        PrimTy::Double
    );
    assert_eq!(
        out.values[model_matrix_default_v].value_ty.shape,
        ShapeTy::Matrix
    );
    assert_eq!(
        out.values[model_matrix_default_v].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), None, Some(3))
    );
    assert_eq!(out.values[model_matrix_lm_v].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[model_matrix_lm_v].value_ty.shape,
        ShapeTy::Matrix
    );
    assert_eq!(
        out.values[model_matrix_lm_v].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), None, Some(3))
    );
    assert_eq!(out.values[update_default_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[update_default_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[update_default_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );
    assert_eq!(out.values[update_formula_v].value_term, TypeTerm::Any);
    assert_eq!(
        out.values[glm_control_v].value_term,
        TypeTerm::NamedList(vec![
            ("epsilon".to_string(), TypeTerm::Double),
            ("maxit".to_string(), TypeTerm::Int),
            ("trace".to_string(), TypeTerm::Logical),
        ])
    );
    assert_eq!(
        out.values[drop_terms_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "variables".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any))
            ),
            (
                "factors".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Int))
            ),
            (
                "term.labels".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "order".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("intercept".to_string(), TypeTerm::Int),
            ("response".to_string(), TypeTerm::Int),
            (
                "predvars".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any))
            ),
            (
                "dataClasses".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "class".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (".Environment".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(out.values[is_empty_v].value_ty.prim, PrimTy::Logical);
    assert_eq!(out.values[is_empty_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[is_empty_v].value_term, TypeTerm::Logical);
}

#[test]
fn stats_signal_helpers_have_direct_types() {
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
    let five = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(5.0)),
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
    let seven = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(7.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let eight = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(8.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four, five, six, seven, eight],
            names: vec![None, None, None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let weights = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, one, two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let small = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let small2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, three, two, four, five],
            names: vec![None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let k02a = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let k06 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.6)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let k02b = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let kernel = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![k02a, k06, k02b],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ts".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let weighted = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::weighted.mean".to_string(),
            args: vec![small, weights],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let runmed_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::runmed".to_string(),
            args: vec![small2, three_i],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let filter_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::filter".to_string(),
            args: vec![ts_v, kernel],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dec_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::decompose".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let false_v = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(false)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let spec_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::spectrum".to_string(),
            args: vec![ts_v, false_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let spec_pgram_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::spec.pgram".to_string(),
            args: vec![ts_v, false_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let spec_taper_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::spec.taper".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plot_spec_coh_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::plot.spec.coherency".to_string(),
            args: vec![spec_pgram_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plot_spec_phase_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::plot.spec.phase".to_string(),
            args: vec![spec_pgram_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let long_ts_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![
                one, two, three, four, five, six, seven, eight, one, two, three, four,
            ],
            names: vec![
                None, None, None, None, None, None, None, None, None, None, None, None,
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stl_x = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ts".to_string(),
            args: vec![long_ts_vec],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let periodic = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("periodic".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stl_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::stl".to_string(),
            args: vec![stl_x, periodic],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(weighted));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[weighted].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[weighted].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[weighted].value_term, TypeTerm::Double);
    assert_eq!(out.values[runmed_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[runmed_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[runmed_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    for vid in [ts_v, filter_v, spec_taper_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(
        out.values[dec_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "seasonal".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "trend".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "random".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "figure".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("type".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[spec_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "freq".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "spec".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("df".to_string(), TypeTerm::Double),
            ("bandwidth".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("series".to_string(), TypeTerm::Char),
            ("snames".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[spec_pgram_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "freq".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "spec".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("coh".to_string(), TypeTerm::Any),
            ("phase".to_string(), TypeTerm::Any),
            ("kernel".to_string(), TypeTerm::Any),
            ("df".to_string(), TypeTerm::Double),
            ("bandwidth".to_string(), TypeTerm::Double),
            ("n.used".to_string(), TypeTerm::Int),
            ("orig.n".to_string(), TypeTerm::Int),
            ("series".to_string(), TypeTerm::Char),
            ("snames".to_string(), TypeTerm::Any),
            ("method".to_string(), TypeTerm::Char),
            ("taper".to_string(), TypeTerm::Double),
            ("pad".to_string(), TypeTerm::Int),
            ("detrend".to_string(), TypeTerm::Logical),
            ("demean".to_string(), TypeTerm::Logical),
        ])
    );
    for vid in [plot_spec_coh_v, plot_spec_phase_v] {
        assert_eq!(out.values[vid].value_ty, RR::typeck::TypeState::null());
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }
    assert_eq!(
        out.values[stl_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "time.series".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "weights".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("call".to_string(), TypeTerm::Any),
            (
                "win".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("deg".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            (
                "jump".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("inner".to_string(), TypeTerm::Int),
            ("outer".to_string(), TypeTerm::Int),
        ])
    );
}

#[test]
fn stats_grouping_helpers_have_direct_types() {
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
    let a = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("a".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let b = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("b".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nums = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let groups = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![a, a, b, b],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let factor_src = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![a, b, a],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::factor".to_string(),
            args: vec![factor_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let agg = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::aggregate".to_string(),
            args: vec![nums, groups],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ave_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ave".to_string(),
            args: vec![nums, groups],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let reorder_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::reorder".to_string(),
            args: vec![groups, nums],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let relevel_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::relevel".to_string(),
            args: vec![factor_v, b],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(agg));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[agg].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[agg].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[agg].value_term, TypeTerm::DataFrame(Vec::new()));
    assert_eq!(out.values[ave_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[ave_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[ave_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    for vid in [reorder_v, relevel_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }
}
