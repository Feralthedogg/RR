use super::type_precision_regression_common::*;

#[test]
pub(crate) fn parallel_two_digit_tail_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["cluster".to_string(), "tasks".to_string()],
    );
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::List(Box::new(TypeTerm::Any));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let cluster = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tasks = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let mut add_call = |callee: &str, args: Vec<rr::compiler::internal::mir::ValueId>| {
        let arg_len = args.len();
        fn_ir.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args,
                names: vec![None; arg_len],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        )
    };

    let mut list_any_calls = Vec::new();
    for callee in [
        "parallel::makeForkCluster",
        "parallel::makePSOCKcluster",
        "parallel::parLapplyLB",
        "parallel::mcMap",
        "parallel::getDefaultCluster",
        "parallel::recvData",
        "parallel::recvOneData",
    ] {
        list_any_calls.push(add_call(callee, vec![cluster, tasks]));
    }

    let mut vec_any_calls = Vec::new();
    for callee in [
        "parallel::parCapply",
        "parallel::parRapply",
        "parallel::pvec",
        "parallel::mcmapply",
    ] {
        vec_any_calls.push(add_call(callee, vec![cluster, tasks]));
    }

    let mut vec_int_calls = Vec::new();
    for callee in [
        "parallel::nextRNGStream",
        "parallel::nextRNGSubStream",
        "parallel::mcaffinity",
    ] {
        vec_int_calls.push(add_call(callee, vec![tasks]));
    }

    let mut null_calls = Vec::new();
    for callee in [
        "parallel::closeNode",
        "parallel::clusterSetRNGStream",
        "parallel::mc.reset.stream",
        "parallel::registerClusterType",
        "parallel::sendData",
        "parallel::setDefaultCluster",
    ] {
        null_calls.push(add_call(callee, vec![cluster]));
    }

    fn_ir.blocks[b0].term = Terminator::Return(Some(cluster));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in list_any_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    for vid in vec_any_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Any))
        );
    }

    for vid in vec_int_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }

    for vid in null_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }
}

#[test]
pub(crate) fn stats4_meta_exports_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["fit".to_string()]);
    fn_ir.param_ty_hints[0] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::List(Box::new(TypeTerm::Any));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let fit = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let mut add_call = |callee: &str| {
        fn_ir.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args: vec![fit],
                names: vec![None],
            },
            Span::dummy(),
            Facts::empty(),
            None,
        )
    };

    let mut vids = Vec::new();
    for callee in [
        "stats4::.__C__mle",
        "stats4::.__C__profile.mle",
        "stats4::.__C__summary.mle",
        "stats4::.__T__AIC:stats",
        "stats4::.__T__BIC:stats",
        "stats4::.__T__coef:stats",
        "stats4::.__T__confint:stats",
        "stats4::.__T__logLik:stats",
        "stats4::.__T__nobs:stats",
        "stats4::.__T__plot:base",
        "stats4::.__T__profile:stats",
        "stats4::.__T__show:methods",
        "stats4::.__T__summary:base",
        "stats4::.__T__update:stats",
        "stats4::.__T__vcov:stats",
    ] {
        vids.push(add_call(callee));
    }

    fn_ir.blocks[b0].term = Terminator::Return(Some(fit));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in vids {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }
}

#[test]
pub(crate) fn tools_tail_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec!["path".to_string()]);
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::scalar(PrimTy::Char, false);
    fn_ir.param_term_hints[0] = TypeTerm::Char;

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let path = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let title = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "title".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sweave_path = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "sample.tex".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let engine_name = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            "utils::Sweave".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let adobe = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::Adobe_glyphs".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let gs = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::find_gs_cmd".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mv_site = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::makevars_site".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let header = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::HTMLheader".to_string(),
            args: vec![title],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sweave = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::SweaveTeXFilter".to_string(),
            args: vec![sweave_path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let html_links = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::findHTMLlinks".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sigint = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::SIGINT".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let assert_error = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::assertError".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let check_s3 = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::checkS3methods".to_string(),
            args: vec![],
            names: vec![],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pkg_dep = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::package.dependencies".to_string(),
            args: vec![path],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vignette_engine = fn_ir.add_value(
        ValueKind::Call {
            callee: "tools::vignetteEngine".to_string(),
            args: vec![engine_name],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    fn_ir.blocks[b0].term = Terminator::Return(Some(vignette_engine));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    assert_eq!(out.values[adobe].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[adobe].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[adobe].value_term,
        TypeTerm::DataFrameNamed(vec![
            (
                "adobe".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "unicode".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])
    );

    for vid in [gs, mv_site, header, sweave, html_links] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Char);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Char))
        );
    }

    assert_eq!(out.values[sigint].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[sigint].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[sigint].value_term, TypeTerm::Int);

    assert_eq!(out.values[assert_error].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[assert_error].value_ty.prim, PrimTy::Null);
    assert_eq!(out.values[assert_error].value_term, TypeTerm::Null);

    for vid in [check_s3, pkg_dep, vignette_engine] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }
}
