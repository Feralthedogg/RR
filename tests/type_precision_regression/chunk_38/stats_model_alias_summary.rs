use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn stats_model_alias_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let lm_formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "mpg ~ wt + hp".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glm_formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "am ~ mpg".to_string(),
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
    let lm_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![lm_formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glm_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![glm_formula_src],
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
    let lm_fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![lm_formula, mtcars],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glm_fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::glm".to_string(),
            args: vec![glm_formula, mtcars, family],
            names: vec![None, Some("data".to_string()), Some("family".to_string())],
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
    let simulate_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::simulate".to_string(),
            args: vec![lm_fit, two_i],
            names: vec![None, Some("nsim".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let calls = vec![
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::coefficients".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::fitted.values".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::resid".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::predict.lm".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::predict.glm".to_string(),
                args: vec![glm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::residuals.lm".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::residuals.glm".to_string(),
                args: vec![glm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::confint.lm".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::confint.default".to_string(),
                args: vec![glm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::getCall".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::hatvalues".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::hat".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::cooks.distance".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::covratio".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::dfbeta".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::dfbetas".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::dffits".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::rstandard".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::rstudent".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::weighted.residuals".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::influence".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::influence.measures".to_string(),
                args: vec![lm_fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::qr.influence".to_string(),
                args: vec![lm_fit, lm_fit],
                names: vec![None, None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::lm.influence".to_string(),
                args: vec![lm_fit],
                names: vec![None],
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
    assert_eq!(out.values[simulate_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[simulate_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[simulate_v].value_term,
        TypeTerm::DataFrame(Vec::new())
    );
    for vid in &calls[..7] {
        assert_eq!(out.values[*vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[*vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[*vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    for vid in &calls[7..9] {
        assert_eq!(out.values[*vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[*vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(
            out.values[*vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(out.values[calls[9]].value_term, TypeTerm::Any);
    for vid in [
        calls[10], calls[11], calls[12], calls[13], calls[16], calls[17], calls[18], calls[19],
    ] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    for vid in [calls[14], calls[15]] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }
    for vid in [calls[20], calls[21], calls[22], calls[23]] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(
        out.values[calls[20]].value_term,
        TypeTerm::NamedList(vec![
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "sigma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "wt.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[calls[21]].value_term,
        TypeTerm::NamedList(vec![
            (
                "infmat".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "is.inf".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Logical))
            ),
            ("call".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[calls[22]].value_term,
        TypeTerm::NamedList(vec![
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "sigma".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(out.values[calls[23]].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[calls[23]].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[calls[23]].value_term,
        TypeTerm::NamedList(vec![
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "sigma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "wt.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
}

#[test]
pub(crate) fn stats_summary_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let lm_formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "mpg ~ wt + hp".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glm_formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "am ~ mpg".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let aov_formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "mpg ~ factor(cyl)".to_string(),
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
    let lm_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![lm_formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glm_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![glm_formula_src],
            names: vec![None],
        },
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
    let lm_fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![lm_formula, mtcars],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let glm_fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::glm".to_string(),
            args: vec![glm_formula, mtcars, family],
            names: vec![None, Some("data".to_string()), Some("family".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let aov_fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::aov".to_string(),
            args: vec![aov_formula, mtcars],
            names: vec![None, Some("data".to_string())],
        },
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
    let ten = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(10.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let twenty = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(20.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let thirty = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(30.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let forty = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(40.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let step_x = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let step_y = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![ten, twenty, thirty, forty],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let stepfun_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::stepfun".to_string(),
            args: vec![step_x, step_y],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_lm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::summary.lm".to_string(),
            args: vec![lm_fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_glm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::summary.glm".to_string(),
            args: vec![glm_fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_aov_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::summary.aov".to_string(),
            args: vec![aov_fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary_stepfun_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::summary.stepfun".to_string(),
            args: vec![stepfun_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(summary_lm_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [summary_lm_v, summary_glm_v, summary_aov_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(out.values[summary_stepfun_v].value_ty.prim, PrimTy::Null);
    assert_eq!(
        out.values[summary_stepfun_v].value_ty.shape,
        ShapeTy::Scalar
    );
    assert_eq!(out.values[summary_stepfun_v].value_term, TypeTerm::Null);
    assert_eq!(
        out.values[summary_lm_v].value_term,
        TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "aliased".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical))
            ),
            ("sigma".to_string(), TypeTerm::Double),
            ("df".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            ("r.squared".to_string(), TypeTerm::Double),
            ("adj.r.squared".to_string(), TypeTerm::Double),
            (
                "fstatistic".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "cov.unscaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[summary_glm_v].value_term,
        TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
            (
                "family".to_string(),
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
                    ("dispersion".to_string(), TypeTerm::Double),
                ])
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
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "aliased".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical))
            ),
            ("dispersion".to_string(), TypeTerm::Double),
            ("df".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            (
                "cov.unscaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "cov.scaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[summary_aov_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::DataFrame(Vec::new())))
    );
}
