use super::type_precision_regression_common::*;

#[test]
pub(crate) fn stats_multivar_math_helpers_have_direct_types() {
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
    let five = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(5.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let six = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(6.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seven = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(7.0)),
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
    let five_i = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let m_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four, five, six],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let m = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![m_data, three_i],
            names: vec![None, Some("nrow".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cov_mat_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two, one, one, three],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cov_mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![cov_mat_data, two_i],
            names: vec![None, Some("nrow".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sample_mat_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sample_mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![sample_mat_data, two_i],
            names: vec![None, Some("nrow".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let center = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, one],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_totals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![five, seven],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_totals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![six, six],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let p02 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let p03 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let p05 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let probs = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![p02, p03, p05],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let diag2_a = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::diag".to_string(),
            args: vec![two_i],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let diag2_b = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::diag".to_string(),
            args: vec![two_i],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let counts = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two, one, one],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let cov_wt_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cov.wt".to_string(),
            args: vec![m],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cov2cor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cov2cor".to_string(),
            args: vec![cov_mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mahalanobis_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::mahalanobis".to_string(),
            args: vec![sample_mat, center, diag2_a],
            names: vec![None, Some("center".to_string()), Some("cov".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rwishart_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rWishart".to_string(),
            args: vec![two_i, five_i, diag2_b],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let r2dtable_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::r2dtable".to_string(),
            args: vec![two_i, row_totals, col_totals],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dmultinom_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dmultinom".to_string(),
            args: vec![counts, probs],
            names: vec![None, Some("prob".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rmultinom_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rmultinom".to_string(),
            args: vec![three_i, four, probs],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(cov_wt_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[cov_wt_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[cov_wt_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[cov2cor_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[cov2cor_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[mahalanobis_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[mahalanobis_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[rwishart_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[rwishart_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[r2dtable_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[r2dtable_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[dmultinom_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[dmultinom_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[rmultinom_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[rmultinom_v].value_ty.shape, ShapeTy::Matrix);

    assert_eq!(
        out.values[cov_wt_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "center".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("n.obs".to_string(), TypeTerm::Int),
        ])
    );
    assert_eq!(
        out.values[cov2cor_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[mahalanobis_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[rwishart_v].value_term,
        TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, None, None])
    );
    assert_eq!(
        out.values[r2dtable_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Matrix(Box::new(TypeTerm::Int))))
    );
    assert_eq!(out.values[dmultinom_v].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[rmultinom_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Int))
    );
}

#[test]
pub(crate) fn stats_family_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "am ~ mpg".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let logit = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "logit".to_string(),
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
    let binom = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::binomial".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glm = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::glm".to_string(),
            args: vec![formula, mtcars, binom],
            names: vec![None, Some("data".to_string()), Some("family".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let family_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::family".to_string(),
            args: vec![glm],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let make_link_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::make.link".to_string(),
            args: vec![logit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let quasi_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::quasi".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qb_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::quasibinomial".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qp_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::quasipoisson".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ig_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::inverse.gaussian".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(family_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [binom, family_v, make_link_v, quasi_v, qb_v, qp_v, ig_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }

    let base_family_term = TypeTerm::NamedList(vec![
        ("family".to_string(), TypeTerm::Char),
        ("link".to_string(), TypeTerm::Char),
        ("linkfun".to_string(), TypeTerm::Any),
        ("linkinv".to_string(), TypeTerm::Any),
        ("variance".to_string(), TypeTerm::Any),
        ("dev.resids".to_string(), TypeTerm::Any),
        ("aic".to_string(), TypeTerm::Any),
        ("mu.eta".to_string(), TypeTerm::Any),
        ("initialize".to_string(), TypeTerm::Any),
        ("validmu".to_string(), TypeTerm::Any),
        ("valideta".to_string(), TypeTerm::Any),
        ("dispersion".to_string(), TypeTerm::Double),
    ]);
    for vid in [binom, family_v, qb_v, qp_v, ig_v] {
        assert_eq!(out.values[vid].value_term, base_family_term.clone());
    }
    assert_eq!(
        out.values[make_link_v].value_term,
        TypeTerm::NamedList(vec![
            ("linkfun".to_string(), TypeTerm::Any),
            ("linkinv".to_string(), TypeTerm::Any),
            ("mu.eta".to_string(), TypeTerm::Any),
            ("valideta".to_string(), TypeTerm::Any),
            ("name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[quasi_v].value_term,
        TypeTerm::NamedList(vec![
            ("family".to_string(), TypeTerm::Char),
            ("link".to_string(), TypeTerm::Char),
            ("linkfun".to_string(), TypeTerm::Any),
            ("linkinv".to_string(), TypeTerm::Any),
            ("variance".to_string(), TypeTerm::Any),
            ("dev.resids".to_string(), TypeTerm::Any),
            ("aic".to_string(), TypeTerm::Any),
            ("mu.eta".to_string(), TypeTerm::Any),
            ("initialize".to_string(), TypeTerm::Any),
            ("validmu".to_string(), TypeTerm::Any),
            ("valideta".to_string(), TypeTerm::Any),
            ("varfun".to_string(), TypeTerm::Char),
            ("dispersion".to_string(), TypeTerm::Double),
        ])
    );
}

#[test]
pub(crate) fn stats_selfstart_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let x1 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x2 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(2.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x3 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x4 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(4.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![x1, x2, x3, x4],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ten = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(10.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let neg_one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(-1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let neg_half = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(-0.5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let five = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(5.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let neg_tenth = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(-0.1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pt_two = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.2)),
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
    let one_half = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(1.5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let calls = vec![
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::SSasymp".to_string(),
                args: vec![x, ten, one, neg_one],
                names: vec![None, None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::SSasympOff".to_string(),
                args: vec![x, ten, neg_one, one],
                names: vec![None, None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::SSasympOrig".to_string(),
                args: vec![x, ten, neg_one],
                names: vec![None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::SSbiexp".to_string(),
                args: vec![x, ten, neg_half, five, neg_tenth],
                names: vec![None, None, None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::SSfol".to_string(),
                args: vec![x, ten, neg_one, neg_half, pt_two],
                names: vec![None, None, None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::SSfpl".to_string(),
                args: vec![x, ten, one, two, one],
                names: vec![None, None, None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::SSgompertz".to_string(),
                args: vec![x, ten, two, one],
                names: vec![None, None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::SSlogis".to_string(),
                args: vec![x, ten, two, one],
                names: vec![None, None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::SSmicmen".to_string(),
                args: vec![x, ten, two],
                names: vec![None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::SSweibull".to_string(),
                args: vec![x, ten, two, neg_one, one_half],
                names: vec![None, None, None, None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
    ];
    fn_ir.blocks[b0].term = Terminator::Return(Some(calls[0]));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in calls {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
}
