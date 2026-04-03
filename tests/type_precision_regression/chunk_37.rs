use super::type_precision_regression_common::*;

#[test]
fn stats_symbolic_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("~ x^2 + y".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("x".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("y".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let theta = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![x_name, y_name],
            names: vec![None, None],
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
    let deriv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::deriv".to_string(),
            args: vec![formula, theta],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let deriv3_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::deriv3".to_string(),
            args: vec![formula, theta],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let numeric_deriv_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::numericDeriv".to_string(),
            args: vec![deriv_v, theta],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let self_start_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::selfStart".to_string(),
            args: vec![formula, deriv_v, theta],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(numeric_deriv_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [deriv_v, deriv3_v, self_start_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
    assert_eq!(out.values[numeric_deriv_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[numeric_deriv_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[numeric_deriv_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
}

#[test]
fn stats_power_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let three_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(3)),
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
    let fifty = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(50.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let p02 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let p04 = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.4)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let twenty = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(20.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let anova_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::power.anova.test".to_string(),
            args: vec![three_i, ten, one, two],
            names: vec![
                Some("groups".to_string()),
                Some("n".to_string()),
                Some("between.var".to_string()),
                Some("within.var".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prop_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::power.prop.test".to_string(),
            args: vec![fifty, p02, p04],
            names: vec![
                Some("n".to_string()),
                Some("p1".to_string()),
                Some("p2".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let t_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::power.t.test".to_string(),
            args: vec![twenty, one, two],
            names: vec![
                Some("n".to_string()),
                Some("delta".to_string()),
                Some("sd".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(anova_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [anova_v, prop_v, t_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(
        out.values[anova_v].value_term,
        TypeTerm::NamedList(vec![
            ("groups".to_string(), TypeTerm::Int),
            ("n".to_string(), TypeTerm::Double),
            ("between.var".to_string(), TypeTerm::Double),
            ("within.var".to_string(), TypeTerm::Double),
            ("sig.level".to_string(), TypeTerm::Double),
            ("power".to_string(), TypeTerm::Double),
            ("note".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[prop_v].value_term,
        TypeTerm::NamedList(vec![
            ("n".to_string(), TypeTerm::Double),
            ("p1".to_string(), TypeTerm::Double),
            ("p2".to_string(), TypeTerm::Double),
            ("sig.level".to_string(), TypeTerm::Double),
            ("power".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("note".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[t_v].value_term,
        TypeTerm::NamedList(vec![
            ("n".to_string(), TypeTerm::Double),
            ("delta".to_string(), TypeTerm::Double),
            ("sd".to_string(), TypeTerm::Double),
            ("sig.level".to_string(), TypeTerm::Double),
            ("power".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("note".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
        ])
    );
}

#[test]
fn stats_contrast_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let three_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(3)),
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
        ValueKind::Const(RR::syntax::ast::Lit::Str("am ~ mpg".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let response_name = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("y".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let var_x = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("x".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let var_z = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("z".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let factor_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![a, b, c_name],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let factor_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::factor".to_string(),
            args: vec![factor_vals],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_formula = fn_ir.add_value(
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
    let mtcars = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::mtcars".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::glm".to_string(),
            args: vec![model_formula, mtcars, family],
            names: vec![None, Some("data".to_string()), Some("family".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let reform_terms = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![var_x, var_z],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let calls = [
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::contr.treatment".to_string(),
                args: vec![three_i],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::contr.sum".to_string(),
                args: vec![three_i],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::contr.helmert".to_string(),
                args: vec![three_i],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::contr.SAS".to_string(),
                args: vec![three_i],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::contr.poly".to_string(),
                args: vec![three_i],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::contrasts".to_string(),
                args: vec![factor_v],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::formula".to_string(),
                args: vec![model],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        ),
        fn_ir.add_value(
            ValueKind::Call {
                callee: "stats::reformulate".to_string(),
                args: vec![reform_terms, response_name],
                names: vec![None, Some("response".to_string())],
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
    for vid in &calls[..6] {
        assert_eq!(out.values[*vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[*vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(
            out.values[*vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }
    for vid in &calls[6..] {
        assert_eq!(out.values[*vid].value_term, TypeTerm::Any);
    }
}
