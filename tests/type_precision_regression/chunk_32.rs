use super::type_precision_regression_common::*;

#[test]
fn stats_dimred_helpers_have_direct_types() {
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
    let eight = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(8.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ten = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(10.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let twelve = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(12.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dist_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dist".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cmd = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cmdscale".to_string(),
            args: vec![dist_v, two_i],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let left_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four, five, six],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let left_mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![left_vals, two_i],
            names: vec![None, Some("ncol".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let right_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two, four, six, eight, ten, twelve],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let right_mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![right_vals, two_i],
            names: vec![None, Some("ncol".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prin = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::princomp".to_string(),
            args: vec![left_mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let can = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cancor".to_string(),
            args: vec![left_mat, right_mat],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(cmd));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[cmd].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[cmd].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[cmd].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    for vid in [prin, can] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(
        out.values[prin].value_term,
        TypeTerm::NamedList(vec![
            (
                "sdev".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "loadings".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "center".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "scale".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("n.obs".to_string(), TypeTerm::Int),
            (
                "scores".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("call".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[can].value_term,
        TypeTerm::NamedList(vec![
            (
                "cor".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "xcoef".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "ycoef".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "xcenter".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "ycenter".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
}

#[test]
fn stats_smoothing_helpers_have_direct_types() {
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
    let normal_kernel = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("normal".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loess_formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("y ~ x".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ys = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two, three, four, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let approx_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::approx".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ksmooth_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ksmooth".to_string(),
            args: vec![xs, ys, normal_kernel, one],
            names: vec![
                None,
                None,
                Some("kernel".to_string()),
                Some("bandwidth".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lowess_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lowess".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loess_smooth_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::loess.smooth".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let spline_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::spline".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let smooth_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::smooth.spline".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let supsmu_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::supsmu".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loess_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![loess_formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loess_df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![xs, ys],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loess_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::loess".to_string(),
            args: vec![loess_formula, loess_df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loess_control_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::loess.control".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let approxfun_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::approxfun".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let splinefun_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::splinefun".to_string(),
            args: vec![xs, ys],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(approxfun_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [
        approx_v,
        ksmooth_v,
        lowess_v,
        loess_smooth_v,
        spline_v,
        smooth_v,
        supsmu_v,
        loess_v,
        loess_control_v,
    ] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    for vid in [approxfun_v, splinefun_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
    for vid in [
        approx_v,
        ksmooth_v,
        lowess_v,
        loess_smooth_v,
        spline_v,
        supsmu_v,
    ] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::NamedList(vec![
                (
                    "x".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double))
                ),
                (
                    "y".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double))
                ),
            ])
        );
    }
    assert_eq!(
        out.values[smooth_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "w".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "yin".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("df".to_string(), TypeTerm::Double),
            ("lambda".to_string(), TypeTerm::Double),
            ("call".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[loess_v].value_term,
        TypeTerm::NamedList(vec![
            ("n".to_string(), TypeTerm::Int),
            (
                "fitted".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("enp".to_string(), TypeTerm::Double),
            ("s".to_string(), TypeTerm::Double),
            ("one.delta".to_string(), TypeTerm::Double),
            ("two.delta".to_string(), TypeTerm::Double),
            ("trace.hat".to_string(), TypeTerm::Double),
            ("divisor".to_string(), TypeTerm::Double),
            (
                "xnames".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "weights".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[loess_control_v].value_term,
        TypeTerm::NamedList(vec![
            ("surface".to_string(), TypeTerm::Char),
            ("statistics".to_string(), TypeTerm::Char),
            ("trace.hat".to_string(), TypeTerm::Char),
            ("cell".to_string(), TypeTerm::Double),
            ("iterations".to_string(), TypeTerm::Int),
            ("iterTrace".to_string(), TypeTerm::Logical),
        ])
    );
}

#[test]
fn stats_aov_helpers_have_direct_types() {
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
    let c_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("c".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("y ~ grp".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let means = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("means".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four, five, six],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grp_labels = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![a, a, b, b, c_name, c_name],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grp_factor = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::factor".to_string(),
            args: vec![grp_labels],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![y_vals, grp_factor],
            names: vec![Some("y".to_string()), Some("grp".to_string())],
        },
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
    let fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::aov".to_string(),
            args: vec![formula, df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tukey = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::TukeyHSD".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let alias_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::alias".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_tables = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.tables".to_string(),
            args: vec![fit, means],
            names: vec![None, Some("type".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(fit));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [fit, tukey, alias_v, model_tables] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(
        out.values[fit].value_term,
        TypeTerm::NamedList(vec![
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "effects".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("rank".to_string(), TypeTerm::Int),
            (
                "fitted.values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "assign".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("df.residual".to_string(), TypeTerm::Int),
            ("contrasts".to_string(), TypeTerm::Any),
            ("xlevels".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
            ("model".to_string(), TypeTerm::Any),
            ("qr".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[tukey].value_term,
        TypeTerm::List(Box::new(TypeTerm::Matrix(Box::new(TypeTerm::Double))))
    );
    assert_eq!(
        out.values[alias_v].value_term,
        TypeTerm::NamedList(vec![("Model".to_string(), TypeTerm::Any)])
    );
    assert_eq!(
        out.values[model_tables].value_term,
        TypeTerm::NamedList(vec![
            (
                "tables".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any))
            ),
            ("n".to_string(), TypeTerm::Int),
        ])
    );
}
