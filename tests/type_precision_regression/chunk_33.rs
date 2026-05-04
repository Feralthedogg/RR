use super::type_precision_regression_common::*;

#[test]
pub(crate) fn stats_multivar_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let us_arrests = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::USArrests".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_i = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(1)),
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
    let factanal_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::factanal".to_string(),
            args: vec![us_arrests, one_i],
            names: vec![None, Some("factors".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let heatmap_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::heatmap".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(factanal_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [factanal_v, heatmap_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(
        out.values[factanal_v].value_term,
        TypeTerm::NamedList(vec![
            ("converged".to_string(), TypeTerm::Logical),
            (
                "loadings".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "uniquenesses".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "correlation".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "criteria".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("factors".to_string(), TypeTerm::Double),
            ("dof".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("STATISTIC".to_string(), TypeTerm::Double),
            ("PVAL".to_string(), TypeTerm::Double),
            ("n.obs".to_string(), TypeTerm::Int),
            ("call".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[heatmap_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "rowInd".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            (
                "colInd".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("Rowv".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
            ("Colv".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
        ])
    );
}

#[test]
pub(crate) fn stats_model_selection_helpers_have_direct_types() {
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
    let eight = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(8.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sixteen = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(16.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let thirty_two = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(32.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_formula = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ 1".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let full_formula = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "y ~ x + z".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let scope_formula = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "~ x + z".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let scope_formula2 = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "~ x + z + qsec".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let test_f = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "F".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let trace_zero = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, four, eight, sixteen, thirty_two],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four, five, six],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let z_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two, one, three, two, four, three],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let train = fn_ir.add_value(
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
    let formula0 = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![one_formula],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let formula1 = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![full_formula],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let scope = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![scope_formula],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fit0 = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![formula0, train],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fit1 = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![formula1, train],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let add1_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::add1".to_string(),
            args: vec![fit0, scope],
            names: vec![None, Some("scope".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let drop1_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::drop1".to_string(),
            args: vec![fit1, test_f],
            names: vec![None, Some("test".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let step_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::step".to_string(),
            args: vec![fit1, trace_zero],
            names: vec![None, Some("trace".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let scope2 = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![scope_formula2],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let terms_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::terms".to_string(),
            args: vec![step_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let model_frame_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::model.frame".to_string(),
            args: vec![step_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let terms2_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::terms".to_string(),
            args: vec![scope2],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let extract_aic_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::extractAIC".to_string(),
            args: vec![fit1],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let add_scope_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::add.scope".to_string(),
            args: vec![terms_v, terms2_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let drop_scope_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::drop.scope".to_string(),
            args: vec![terms_v, terms2_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let factor_scope_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::factor.scope".to_string(),
            args: vec![terms_v, terms2_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dummy_coef_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dummy.coef".to_string(),
            args: vec![fit1],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dummy_coef_lm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dummy.coef.lm".to_string(),
            args: vec![fit1],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let effects_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::effects".to_string(),
            args: vec![fit1],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(step_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [add1_v, drop1_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_term, TypeTerm::DataFrame(Vec::new()));
    }

    assert_eq!(out.values[step_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[step_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[step_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Any))
    );
    assert_eq!(
        out.values[terms_v].value_term,
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
    assert_eq!(
        out.values[model_frame_v].value_term,
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
    assert_eq!(out.values[extract_aic_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[extract_aic_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[extract_aic_v].value_term,
        TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(2))
    );
    for vid in [add_scope_v, drop_scope_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }
    assert_eq!(out.values[factor_scope_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[factor_scope_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[factor_scope_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "drop".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
            (
                "add".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char))
            ),
        ])
    );
    for vid in [dummy_coef_v, dummy_coef_lm_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(out.values[effects_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[effects_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[effects_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
}

#[test]
pub(crate) fn stats_optimizer_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let zero = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let neg_ten = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(-10.0)),
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
    let init = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![zero, zero],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let interval = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![neg_ten, ten],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let quad_fn = fn_ir.add_value(
        ValueKind::Load {
            var: "quad".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grad_fn = fn_ir.add_value(
        ValueKind::Load {
            var: "grad".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_fn = fn_ir.add_value(
        ValueKind::Load {
            var: "one".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let quad1_fn = fn_ir.add_value(
        ValueKind::Load {
            var: "quad1".to_string(),
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
    let ui_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, zero, zero, one],
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
    let ui = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![ui_data, two_i],
            names: vec![None, Some("nrow".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ci = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![neg_ten, neg_ten],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let optim_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::optim".to_string(),
            args: vec![init, quad_fn],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let optim_hess_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::optimHess".to_string(),
            args: vec![init, quad_fn, grad_fn],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let optimize_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::optimize".to_string(),
            args: vec![one_fn, interval],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let optimise_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::optimise".to_string(),
            args: vec![one_fn, interval],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nlm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::nlm".to_string(),
            args: vec![one_fn, zero],
            names: vec![None, Some("p".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nlminb_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::nlminb".to_string(),
            args: vec![init, quad_fn, grad_fn],
            names: vec![
                None,
                Some("objective".to_string()),
                Some("gradient".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let constr_optim_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::constrOptim".to_string(),
            args: vec![init, quad_fn, grad_fn, ui, ci],
            names: vec![None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let uniroot_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::uniroot".to_string(),
            args: vec![quad1_fn, interval],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let integrate_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::integrate".to_string(),
            args: vec![quad1_fn, zero, one],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(optim_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [
        optim_v,
        optimize_v,
        optimise_v,
        nlm_v,
        nlminb_v,
        constr_optim_v,
        uniroot_v,
        integrate_v,
    ] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(out.values[optim_hess_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[optim_hess_v].value_ty.shape, ShapeTy::Matrix);

    assert_eq!(
        out.values[optim_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "par".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("value".to_string(), TypeTerm::Double),
            (
                "counts".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("convergence".to_string(), TypeTerm::Int),
            ("message".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[optim_hess_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    for vid in [optimize_v, optimise_v] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::NamedList(vec![
                ("minimum".to_string(), TypeTerm::Double),
                ("objective".to_string(), TypeTerm::Double),
            ])
        );
    }
    assert_eq!(
        out.values[nlm_v].value_term,
        TypeTerm::NamedList(vec![
            ("minimum".to_string(), TypeTerm::Double),
            (
                "estimate".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "gradient".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("code".to_string(), TypeTerm::Int),
            ("iterations".to_string(), TypeTerm::Int),
        ])
    );
    assert_eq!(
        out.values[nlminb_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "par".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("objective".to_string(), TypeTerm::Double),
            ("convergence".to_string(), TypeTerm::Int),
            ("iterations".to_string(), TypeTerm::Int),
            (
                "evaluations".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("message".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[constr_optim_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "par".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("value".to_string(), TypeTerm::Double),
            (
                "counts".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("convergence".to_string(), TypeTerm::Int),
            ("message".to_string(), TypeTerm::Any),
            ("outer.iterations".to_string(), TypeTerm::Int),
            ("barrier.value".to_string(), TypeTerm::Double),
        ])
    );
    assert_eq!(
        out.values[uniroot_v].value_term,
        TypeTerm::NamedList(vec![
            ("root".to_string(), TypeTerm::Double),
            ("f.root".to_string(), TypeTerm::Double),
            ("iter".to_string(), TypeTerm::Int),
            ("init.it".to_string(), TypeTerm::Int),
            ("estim.prec".to_string(), TypeTerm::Double),
        ])
    );
    assert_eq!(
        out.values[integrate_v].value_term,
        TypeTerm::NamedList(vec![
            ("value".to_string(), TypeTerm::Double),
            ("abs.error".to_string(), TypeTerm::Double),
            ("subdivisions".to_string(), TypeTerm::Int),
            ("message".to_string(), TypeTerm::Char),
            ("call".to_string(), TypeTerm::Any),
        ])
    );
}
