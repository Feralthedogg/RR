use super::type_precision_regression_common::*;

#[test]
fn stats_ts_model_helpers_have_direct_types() {
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
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(4.0)),
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
    let sixteen = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(16.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let thirty_two = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(32.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sixty_four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(64.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_twenty_eight = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(128.0)),
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
    let four_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(4)),
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
    let zero_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let level = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("level".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let series = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![
                one,
                two,
                four,
                eight,
                sixteen,
                thirty_two,
                sixty_four,
                one_twenty_eight,
            ],
            names: vec![None, None, None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let order = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one_i, zero_i, zero_i],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ts".to_string(),
            args: vec![series, four_i],
            names: vec![None, Some("frequency".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hw_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::HoltWinters".to_string(),
            args: vec![ts_v, false_v, false_v],
            names: vec![None, Some("beta".to_string()), Some("gamma".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let st_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::StructTS".to_string(),
            args: vec![ts_v, level],
            names: vec![None, Some("type".to_string())],
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
    let true_v = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(true)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let kalman_forecast_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::KalmanForecast".to_string(),
            args: vec![three_i, st_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let kalman_run_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::KalmanRun".to_string(),
            args: vec![ts_v, st_v, true_v],
            names: vec![None, None, Some("update".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let kalman_smooth_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::KalmanSmooth".to_string(),
            args: vec![ts_v, st_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let arima_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::arima".to_string(),
            args: vec![ts_v, order],
            names: vec![None, Some("order".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(hw_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [
        hw_v,
        st_v,
        kalman_forecast_v,
        kalman_run_v,
        kalman_smooth_v,
        arima_v,
    ] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(
        out.values[hw_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "fitted".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("alpha".to_string(), TypeTerm::Double),
            ("beta".to_string(), TypeTerm::Any),
            ("gamma".to_string(), TypeTerm::Any),
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("seasonal".to_string(), TypeTerm::Char),
            ("SSE".to_string(), TypeTerm::Double),
            ("call".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[st_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("loglik".to_string(), TypeTerm::Double),
            ("loglik0".to_string(), TypeTerm::Double),
            (
                "data".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "fitted".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("call".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("code".to_string(), TypeTerm::Int),
            ("model".to_string(), TypeTerm::Any),
            (
                "model0".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any))
            ),
            (
                "xtsp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[kalman_forecast_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "pred".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "var".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[kalman_run_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "resid".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "states".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[kalman_smooth_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "smooth".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "var".to_string(),
                TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, None, None])
            ),
        ])
    );
    assert_eq!(
        out.values[arima_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("sigma2".to_string(), TypeTerm::Double),
            (
                "var.coef".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("mask".to_string(), TypeTerm::Any),
            ("loglik".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            (
                "arma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("call".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("code".to_string(), TypeTerm::Int),
            ("n.cond".to_string(), TypeTerm::Int),
            ("nobs".to_string(), TypeTerm::Int),
            ("model".to_string(), TypeTerm::Any),
        ])
    );
}

#[test]
fn stats_ts_diag_helpers_have_direct_types() {
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
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(4.0)),
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
    let sixteen = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(16.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let thirty_two = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(32.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sixty_four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(64.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_twenty_eight = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(128.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(4)),
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
    let zero_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let five_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let series = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![
                one,
                two,
                four,
                eight,
                sixteen,
                thirty_two,
                sixty_four,
                one_twenty_eight,
            ],
            names: vec![None, None, None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let order = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one_i, zero_i, zero_i],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ts".to_string(),
            args: vec![series, four_i],
            names: vec![None, Some("frequency".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let arima0_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::arima0".to_string(),
            args: vec![ts_v, order],
            names: vec![None, Some("order".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tsdiag_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::tsdiag".to_string(),
            args: vec![arima0_v, five_i],
            names: vec![None, Some("gof.lag".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(arima0_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[arima0_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[arima0_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[tsdiag_v].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[tsdiag_v].value_ty.shape, ShapeTy::Scalar);

    assert_eq!(
        out.values[arima0_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("sigma2".to_string(), TypeTerm::Double),
            (
                "var.coef".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("mask".to_string(), TypeTerm::Any),
            ("loglik".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            (
                "arma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("call".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("code".to_string(), TypeTerm::Int),
            ("n.cond".to_string(), TypeTerm::Int),
        ])
    );
    assert_eq!(out.values[tsdiag_v].value_term, TypeTerm::Null);
}

#[test]
fn stats_nls_helpers_have_direct_types() {
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
    let y1 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(2.4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y2 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(3.8)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y3 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(5.1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y4 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(7.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y5 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(9.1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y6 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(11.2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y7 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(13.1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y8 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(15.4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nls_formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("y ~ a + b * x".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let init_formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str(
            "rate ~ stats::SSasymp(conc, Asym, R0, lrc)".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four, five, six, seven, eight],
            names: vec![None, None, None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![y1, y2, y3, y4, y5, y6, y7, y8],
            names: vec![None, None, None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![x_vals, y_vals],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let start_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::list".to_string(),
            args: vec![one, two],
            names: vec![Some("a".to_string()), Some("b".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nls_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![nls_formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let init_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![init_formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let puromycin = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Puromycin".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nls_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::nls".to_string(),
            args: vec![nls_formula, df, start_vals],
            names: vec![None, Some("data".to_string()), Some("start".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nls_control_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::nls.control".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let get_initial_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::getInitial".to_string(),
            args: vec![init_formula, puromycin],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(nls_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [nls_v, nls_control_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(out.values[get_initial_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[get_initial_v].value_ty.shape, ShapeTy::Vector);

    assert_eq!(
        out.values[nls_v].value_term,
        TypeTerm::NamedList(vec![
            ("m".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
            (
                "convInfo".to_string(),
                TypeTerm::NamedList(vec![
                    ("isConv".to_string(), TypeTerm::Logical),
                    ("finIter".to_string(), TypeTerm::Int),
                    ("finTol".to_string(), TypeTerm::Double),
                    ("stopCode".to_string(), TypeTerm::Int),
                    ("stopMessage".to_string(), TypeTerm::Char),
                ])
            ),
            ("data".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
            (
                "dataClasses".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "control".to_string(),
                TypeTerm::NamedList(vec![
                    ("maxiter".to_string(), TypeTerm::Double),
                    ("tol".to_string(), TypeTerm::Double),
                    ("minFactor".to_string(), TypeTerm::Double),
                    ("printEval".to_string(), TypeTerm::Logical),
                    ("warnOnly".to_string(), TypeTerm::Logical),
                    ("scaleOffset".to_string(), TypeTerm::Double),
                    ("nDcentral".to_string(), TypeTerm::Logical),
                ])
            ),
        ])
    );
    assert_eq!(
        out.values[nls_control_v].value_term,
        TypeTerm::NamedList(vec![
            ("maxiter".to_string(), TypeTerm::Double),
            ("tol".to_string(), TypeTerm::Double),
            ("minFactor".to_string(), TypeTerm::Double),
            ("printEval".to_string(), TypeTerm::Logical),
            ("warnOnly".to_string(), TypeTerm::Logical),
            ("scaleOffset".to_string(), TypeTerm::Double),
            ("nDcentral".to_string(), TypeTerm::Logical),
        ])
    );
    assert_eq!(
        out.values[get_initial_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
}
