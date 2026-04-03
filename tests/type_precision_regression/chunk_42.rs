use super::type_precision_regression_common::*;

#[test]
fn stats_utility_helpers_have_direct_types() {
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
    let x = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![three, one, two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![four, one, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sxy = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::sortedXyData".to_string(),
            args: vec![x, y],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coef_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coef_nrow = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coef_table = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![coef_vec, coef_nrow],
            names: vec![None, Some("nrow".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pcm = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::printCoefmat".to_string(),
            args: vec![coef_table],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fit_x1 = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fit_x2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, three, two, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fit_x = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cbind".to_string(),
            args: vec![fit_x1, fit_x2],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fit_y = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lsfit_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lsfit".to_string(),
            args: vec![fit_x, fit_y],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ls_print_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ls.print".to_string(),
            args: vec![lsfit_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(ls_print_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[sxy].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[sxy].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[sxy].value_term,
        TypeTerm::DataFrameNamed(vec![
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
    assert_eq!(out.values[pcm].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[pcm].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[pcm].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[ls_print_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[ls_print_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[ls_print_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "summary".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Char))
            ),
            (
                "coef.table".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Matrix(Box::new(TypeTerm::Double))))
            ),
        ])
    );
}

#[test]
fn stats_model_misc_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str(
            "base::cbind(mpg, disp) ~ factor(cyl)".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mtcars = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::mtcars".to_string(),
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
    let manova_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::manova".to_string(),
            args: vec![formula, mtcars],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_manova_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::summary.manova".to_string(),
            args: vec![manova_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let aov_formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("mpg ~ factor(cyl)".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let aov_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![aov_formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let aov_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::aov".to_string(),
            args: vec![aov_formula, mtcars],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let proj_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::proj".to_string(),
            args: vec![aov_v],
            names: vec![None],
        },
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
    let nine = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(9.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let counts = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![ten, five, six, nine],
            names: vec![None, None, None, None],
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
    let tab = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![counts, two_i],
            names: vec![None, Some("nrow".to_string())],
        },
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
    let margins = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::list".to_string(),
            args: vec![one_i, two_i],
            names: vec![None, None],
        },
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
    let false_v = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(false)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loglin_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::loglin".to_string(),
            args: vec![tab, margins, true_v, true_v, false_v],
            names: vec![
                None,
                None,
                Some("fit".to_string()),
                Some("param".to_string()),
                Some("print".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(manova_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[manova_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[manova_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[manova_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "residuals".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "effects".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("rank".to_string(), TypeTerm::Int),
            (
                "fitted.values".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "assign".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("qr".to_string(), TypeTerm::Any),
            ("df.residual".to_string(), TypeTerm::Int),
            ("contrasts".to_string(), TypeTerm::Any),
            ("xlevels".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
            ("model".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(out.values[summary_manova_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[summary_manova_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[summary_manova_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "row.names".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "SS".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Matrix(Box::new(TypeTerm::Double))))
            ),
            (
                "Eigenvalues".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "stats".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(out.values[proj_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[proj_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[proj_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[loglin_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[loglin_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[loglin_v].value_term,
        TypeTerm::NamedList(vec![
            ("lrt".to_string(), TypeTerm::Double),
            ("pearson".to_string(), TypeTerm::Double),
            ("df".to_string(), TypeTerm::Double),
            (
                "margin".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Int))
            ),
            (
                "fit".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("param".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
        ])
    );
}

#[test]
fn stats_ts_helpers_have_direct_types() {
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
    let two_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
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
    let freq_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::frequency".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let time_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::time".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cycle_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cycle".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let window_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::window".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_ts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.ts".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ts_intersect_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ts.intersect".to_string(),
            args: vec![ts_v, as_ts_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ts_union_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ts.union".to_string(),
            args: vec![ts_v, as_ts_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let has_tsp_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::hasTsp".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_ts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::is.ts".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_mts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::is.mts".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tsp_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::tsp".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let start_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::start".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let end_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::end".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let deltat_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::deltat".to_string(),
            args: vec![ts_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lag_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lag".to_string(),
            args: vec![ts_v, one],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let embed_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::embed".to_string(),
            args: vec![xs, two_i],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(freq_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [ts_v, as_ts_v, has_tsp_v, window_v, lag_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    for vid in [is_ts_v, is_mts_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }
    assert_eq!(out.values[freq_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[freq_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[freq_v].value_term, TypeTerm::Double);
    assert_eq!(out.values[deltat_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[deltat_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[deltat_v].value_term, TypeTerm::Double);
    for vid in [time_v, cycle_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(out.values[tsp_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[tsp_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[tsp_v].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(3))
    );
    for vid in [start_v, end_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(out.values[embed_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[embed_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[embed_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    for vid in [ts_intersect_v, ts_union_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }
}
