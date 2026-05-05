use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn stats_fit_helpers_have_direct_types() {
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
    let zero = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.0)),
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
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let biny = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![zero, one, zero, one],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let intercept = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, one, one, one],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xmat = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::cbind".to_string(),
            args: vec![intercept, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let weights = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, one, one, one],
            names: vec![None, None, None, None],
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
    let glm_fit_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::glm.fit".to_string(),
            args: vec![xmat, biny, family],
            names: vec![
                Some("x".to_string()),
                Some("y".to_string()),
                Some("family".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lm_fit_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm.fit".to_string(),
            args: vec![xmat, ys],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lmw_fit_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm.wfit".to_string(),
            args: vec![xmat, ys, weights],
            names: vec![
                Some("x".to_string()),
                Some("y".to_string()),
                Some("w".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lsfit_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lsfit".to_string(),
            args: vec![xmat, ys],
            names: vec![Some("x".to_string()), Some("y".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ls_diag_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ls.diag".to_string(),
            args: vec![lsfit_v],
            names: vec![None],
        },
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
    let pc_mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![mat_data, two_i],
            names: vec![None, Some("ncol".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let princomp_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::princomp".to_string(),
            args: vec![pc_mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loadings_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::loadings".to_string(),
            args: vec![princomp_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lm_formula_src = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "mpg ~ wt + hp".to_string(),
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
    let fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![lm_formula, mtcars],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let call_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::getCall".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coeffs_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::coefficients".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let makepredictcall_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::makepredictcall".to_string(),
            args: vec![coeffs_v, call_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let na_val = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Na),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let na_vec = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, na_val, three, four],
            names: vec![None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let na_contig_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::na.contiguous".to_string(),
            args: vec![na_vec],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(glm_fit_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [glm_fit_v, lm_fit_v, lmw_fit_v, lsfit_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(out.values[ls_diag_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[ls_diag_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[glm_fit_v].value_term,
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
                "fitted.values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "effects".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "R".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("rank".to_string(), TypeTerm::Int),
            ("qr".to_string(), TypeTerm::Any),
            ("family".to_string(), TypeTerm::Any),
            (
                "linear.predictors".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("deviance".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            ("null.deviance".to_string(), TypeTerm::Double),
            ("iter".to_string(), TypeTerm::Int),
            (
                "weights".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "prior.weights".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("df.residual".to_string(), TypeTerm::Int),
            ("df.null".to_string(), TypeTerm::Int),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("converged".to_string(), TypeTerm::Logical),
            ("boundary".to_string(), TypeTerm::Logical),
        ])
    );
    assert_eq!(out.values[loadings_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[loadings_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[loadings_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[makepredictcall_v].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[makepredictcall_v].value_ty.shape,
        ShapeTy::Scalar
    );
    assert_eq!(out.values[makepredictcall_v].value_term, TypeTerm::Any);
    assert_eq!(
        out.values[ls_diag_v].value_term,
        TypeTerm::NamedList(vec![
            ("std.dev".to_string(), TypeTerm::Double),
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "std.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "stud.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "cooks".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "dfits".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "correlation".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "std.err".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "cov.scaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "cov.unscaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(out.values[na_contig_v].value_ty.shape, ShapeTy::Vector);
}
