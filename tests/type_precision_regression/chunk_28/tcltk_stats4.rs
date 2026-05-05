use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn tcltk_package_calls_have_direct_placeholder_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let alpha = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "alpha".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let obj = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::tclObj".to_string(),
            args: vec![alpha],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_obj = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::as.tclObj".to_string(),
            args: vec![alpha],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let var = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::tclVar".to_string(),
            args: vec![alpha],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let value = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::tclvalue".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_obj = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::is.tclObj".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_win = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::is.tkwin".to_string(),
            args: vec![obj],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dir = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::tclfile.dir".to_string(),
            args: vec![alpha],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tail = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::tclfile.tail".to_string(),
            args: vec![alpha],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let add_path = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::addTclPath".to_string(),
            args: vec![alpha],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let require = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::tclRequire".to_string(),
            args: vec![alpha],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let version = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::tclVersion".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_num = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(1.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let progress = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::tkProgressBar".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let progress_prev = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::getTkProgressBar".to_string(),
            args: vec![progress],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let progress_set = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcltk::setTkProgressBar".to_string(),
            args: vec![progress, one_num],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(tail));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [obj, as_obj, var, progress] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
    for vid in [value, add_path, require] {
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
    for vid in [progress_prev, progress_set] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_term, TypeTerm::Double);
    }
    for vid in [is_obj, is_win] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }
    for vid in [dir, tail] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(out.values[vid].value_term, TypeTerm::Char);
    }
    assert_eq!(out.values[version].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[version].value_ty.prim, PrimTy::Char);
    assert_eq!(out.values[version].value_term, TypeTerm::Char);
}

#[test]
pub(crate) fn stats4_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["fit".to_string(), "fit_summary".to_string()],
    );
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_ty_hints[1] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::List(Box::new(TypeTerm::Any));
    fn_ir.param_term_hints[1] = TypeTerm::List(Box::new(TypeTerm::Any));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let fit = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fit_summary = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let mle = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::mle".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coef = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::coef".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vcov = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::vcov".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let confint = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::confint".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loglik = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::logLik".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let aic = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::AIC".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bic = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::BIC".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let nobs = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::nobs".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let update = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::update".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let summary = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::summary".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let profile = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::profile".to_string(),
            args: vec![fit_summary],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plot = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::plot".to_string(),
            args: vec![fit_summary],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let show = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats4::show".to_string(),
            args: vec![fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(aic));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in [mle, update, summary, profile, plot] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    for vid in [coef, confint] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    assert_eq!(out.values[vcov].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[vcov].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[vcov].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );

    for vid in [loglik, aic, bic] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_term, TypeTerm::Double);
    }

    assert_eq!(out.values[nobs].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[nobs].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[nobs].value_term, TypeTerm::Int);

    assert_eq!(out.values[show].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[show].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[show].value_term, TypeTerm::Double);
}
