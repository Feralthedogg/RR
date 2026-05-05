use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn stats_misc_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(2.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(4.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let half = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hundredth = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.01)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_i = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three_i = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(3)),
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
    let probs = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![hundredth, half, half],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![mat_data, two_i],
            names: vec![None, Some("ncol".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let padj_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::p.adjust".to_string(),
            args: vec![probs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let padj_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::p.adjust".to_string(),
            args: vec![hundredth],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ppoints_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ppoints".to_string(),
            args: vec![three_i],
            names: vec![None],
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
    let density_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::density".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qqnorm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qqnorm".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qqplot_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qqplot".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qqline_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qqline".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let interaction_plot_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::interaction.plot".to_string(),
            args: vec![xs, xs, xs],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lag_plot_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lag.plot".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let monthplot_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::monthplot".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let scatter_smooth_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::scatter.smooth".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let poly_m = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::poly".to_string(),
            args: vec![xs, two_i],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prcomp_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::prcomp".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ecdf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ecdf".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let biplot_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::biplot".to_string(),
            args: vec![prcomp_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cov_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cov".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cor_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cor".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let var_m = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::var".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let iqr_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::IQR".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mad_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::mad".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(padj_s));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[padj_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[padj_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[ppoints_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[dist_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[cov_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[cor_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[var_m].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[poly_m].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[poly_m].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[qqline_n].value_ty,
        rr::compiler::internal::typeck::TypeState::null()
    );
    assert_eq!(out.values[qqplot_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[qqplot_v].value_ty.shape, ShapeTy::Vector);
    for vid in [
        interaction_plot_n,
        lag_plot_n,
        monthplot_n,
        scatter_smooth_n,
        biplot_n,
    ] {
        assert_eq!(
            out.values[vid].value_ty,
            rr::compiler::internal::typeck::TypeState::null()
        );
    }
    assert_eq!(out.values[ecdf_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[iqr_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[mad_s].value_ty.shape, ShapeTy::Scalar);

    assert_eq!(
        out.values[padj_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[padj_s].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[ppoints_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[dist_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[cov_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[cor_s].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[var_m].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[density_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("bw".to_string(), TypeTerm::Double),
            ("n".to_string(), TypeTerm::Int),
            ("old.coords".to_string(), TypeTerm::Logical),
            ("call".to_string(), TypeTerm::Any),
            ("data.name".to_string(), TypeTerm::Char),
            ("has.na".to_string(), TypeTerm::Logical),
        ])
    );
    assert_eq!(
        out.values[qqnorm_v].value_term,
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
    assert_eq!(
        out.values[qqplot_v].value_term,
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
    assert_eq!(out.values[qqline_n].value_term, TypeTerm::Null);
    for vid in [
        interaction_plot_n,
        lag_plot_n,
        monthplot_n,
        scatter_smooth_n,
        biplot_n,
    ] {
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }
    assert_eq!(out.values[iqr_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[mad_s].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[poly_m].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[prcomp_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "sdev".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "rotation".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("center".to_string(), TypeTerm::Any),
            ("scale".to_string(), TypeTerm::Any),
            (
                "x".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(out.values[ecdf_v].value_term, TypeTerm::Any);
}

#[test]
pub(crate) fn stats_table_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(2.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(4.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat_data = fn_ir.add_value(
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
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![mat_data, two_i],
            names: vec![None, Some("nrow".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let titanic = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Titanic".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let freq = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_a = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "A".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_b = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "B".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let yes = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "Yes".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let no = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "No".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![class_a, class_a, class_b, class_b],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let surv_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![yes, no, yes, no],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tab_df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![freq, class_vals, surv_vals],
            names: vec![
                Some("Freq".to_string()),
                Some("Class".to_string()),
                Some("Survived".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xtabs_formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "Freq ~ Class + Survived".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xtabs_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![xtabs_formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let addmargins_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::addmargins".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ftable_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ftable".to_string(),
            args: vec![titanic],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xtabs_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::xtabs".to_string(),
            args: vec![xtabs_formula, tab_df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let isoreg_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::isoreg".to_string(),
            args: vec![mat_data],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let smooth_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::smooth".to_string(),
            args: vec![mat_data],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let smooth_ends_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::smoothEnds".to_string(),
            args: vec![mat_data],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let line_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::line".to_string(),
            args: vec![mat_data],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let varimax_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::varimax".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let promax_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::promax".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(addmargins_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [addmargins_v, ftable_v, xtabs_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }
    for vid in [smooth_v, smooth_ends_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(
        out.values[isoreg_v].value_term,
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
                "yf".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "yc".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "iKnots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("isOrd".to_string(), TypeTerm::Logical),
            ("ord".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[line_v].value_term,
        TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "fitted.values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    for vid in [varimax_v, promax_v] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::NamedList(vec![
                (
                    "loadings".to_string(),
                    TypeTerm::Matrix(Box::new(TypeTerm::Double))
                ),
                (
                    "rotmat".to_string(),
                    TypeTerm::Matrix(Box::new(TypeTerm::Double))
                ),
            ])
        );
    }
}
