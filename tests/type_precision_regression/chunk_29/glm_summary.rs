use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn stats_summary_glm_fields_have_direct_named_types() {
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
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ x".to_string(),
        )),
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
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "dispersion".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coefficients_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "coefficients".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let deviance_resid_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "deviance.resid".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let order_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "order".to_string(),
        )),
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
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "family".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let link_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "link".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let terms_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "terms".to_string(),
        )),
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
