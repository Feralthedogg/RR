use super::type_precision_regression_common::*;

#[test]
pub(crate) fn matrix_shape_algebra_preserves_dimension_terms() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);
    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let six = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(6)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rows = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cols = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "seq_len".to_string(),
            args: vec![six],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "matrix".to_string(),
            args: vec![vals, rows, cols],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let trans = fn_ir.add_value(
        ValueKind::Call {
            callee: "t".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cross = fn_ir.add_value(
        ValueKind::Call {
            callee: "crossprod".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tcross = fn_ir.add_value(
        ValueKind::Call {
            callee: "tcrossprod".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let diag_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "diag".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rb = fn_ir.add_value(
        ValueKind::Call {
            callee: "rbind".to_string(),
            args: vec![mat, mat],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cb = fn_ir.add_value(
        ValueKind::Call {
            callee: "cbind".to_string(),
            args: vec![mat, mat],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mm = fn_ir.add_value(
        ValueKind::Binary {
            op: rr::compiler::internal::syntax::ast::BinOp::MatMul,
            lhs: mat,
            rhs: trans,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(mm));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(
        out.values[mat].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(2), Some(3))
    );
    assert_eq!(
        out.values[trans].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(3), Some(2))
    );
    assert_eq!(
        out.values[cross].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(3), Some(3))
    );
    assert_eq!(
        out.values[tcross].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(2), Some(2))
    );
    assert_eq!(
        out.values[diag_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[rb].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(4), Some(3))
    );
    assert_eq!(
        out.values[cb].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(2), Some(6))
    );
    assert_eq!(
        out.values[mm].value_term,
        TypeTerm::MatrixDim(Box::new(TypeTerm::Int), Some(2), Some(2))
    );
    assert_eq!(out.values[mm].value_ty.shape, ShapeTy::Matrix);
}

#[test]
pub(crate) fn stats_two_digit_tail_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "mat".to_string(), "obj".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Double, false);
    fn_ir.param_ty_hints[2] = rr::compiler::internal::typeck::TypeState::vector(PrimTy::Any, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[1] = TypeTerm::Matrix(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[2] = TypeTerm::List(Box::new(TypeTerm::Any));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let obj = fn_ir.add_value(
        ValueKind::Param { index: 2 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let _one = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(1.0)),
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

    let mut any_vec_calls = Vec::new();
    for callee in [
        "stats::.checkMFClasses",
        "stats::.getXlevels",
        "stats::.lm.fit",
        "stats::.MFclass",
        "stats::DF2formula",
        "stats::Gamma",
        "stats::D",
        "stats::C",
        "stats::KalmanLike",
        "stats::makeARIMA",
        "stats::Pair",
        "stats::power",
        "stats::ppr",
        "stats::preplot",
        "stats::profile",
        "stats::splinefunH",
        "stats::SSD",
        "stats::stat.anova",
        "stats::eff.aovlist",
        "stats::.preformat.ts",
    ] {
        any_vec_calls.push(add_call(callee, vec![obj]));
    }

    let nknots = add_call("stats::.nknots.smspl", vec![xs]);
    let p_adjust_methods = add_call("stats::p.adjust.methods", vec![]);
    let read_ftable = add_call("stats::read.ftable", vec![obj]);
    let expand_model_frame = add_call("stats::expand.model.frame", vec![obj]);

    let mut double_vec_calls = Vec::new();
    for callee in [
        "stats::knots",
        "stats::NLSstAsymptotic",
        "stats::NLSstClosestX",
        "stats::NLSstLfAsymptote",
        "stats::NLSstRtAsymptote",
        "stats::se.contrast",
    ] {
        double_vec_calls.push(add_call(callee, vec![xs]));
    }

    let mut matrix_double_calls = Vec::new();
    for callee in [
        "stats::.vcov.aliased",
        "stats::estVar",
        "stats::pairwise.table",
    ] {
        matrix_double_calls.push(add_call(callee, vec![mat]));
    }

    let mut null_calls = Vec::new();
    for callee in [
        "stats::arima0.diag",
        "stats::cpgram",
        "stats::plclust",
        "stats::ts.plot",
        "stats::write.ftable",
    ] {
        null_calls.push(add_call(callee, vec![obj]));
    }

    let contrasts_set = add_call("stats::contrasts<-", vec![mat, mat]);
    let tsp_set = add_call("stats::tsp<-", vec![xs, xs]);
    let window_set = add_call("stats::window<-", vec![xs, xs]);
    let ts_smooth = add_call("stats::tsSmooth", vec![xs]);
    let pp_test = add_call("stats::PP.test", vec![xs]);

    fn_ir.blocks[b0].term = Terminator::Return(Some(pp_test));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in any_vec_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }

    assert_eq!(out.values[nknots].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[nknots].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[nknots].value_term, TypeTerm::Int);

    assert_eq!(out.values[p_adjust_methods].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[p_adjust_methods].value_ty.prim, PrimTy::Char);
    assert_eq!(
        out.values[p_adjust_methods].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    );

    assert_eq!(out.values[read_ftable].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[read_ftable].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[read_ftable].value_term,
        TypeTerm::DataFrame(Vec::new())
    );

    assert_eq!(
        out.values[expand_model_frame].value_ty.shape,
        ShapeTy::Matrix
    );
    assert_eq!(out.values[expand_model_frame].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[expand_model_frame].value_term,
        TypeTerm::DataFrame(Vec::new())
    );

    for vid in double_vec_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    for vid in matrix_double_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }

    for vid in null_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }

    assert_eq!(out.values[contrasts_set].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[contrasts_set].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[contrasts_set].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );

    for vid in [tsp_set, window_set, ts_smooth] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    assert_eq!(out.values[pp_test].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[pp_test].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[pp_test].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
}

#[test]
pub(crate) fn graphics_two_digit_tail_package_calls_have_direct_types() {
    let mut fn_ir = FnIR::new(
        "Sym_main".to_string(),
        vec!["xs".to_string(), "ys".to_string(), "mat".to_string()],
    );
    fn_ir.param_ty_hints[0] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[1] =
        rr::compiler::internal::typeck::TypeState::vector(PrimTy::Double, false);
    fn_ir.param_ty_hints[2] =
        rr::compiler::internal::typeck::TypeState::matrix(PrimTy::Double, false);
    fn_ir.param_term_hints[0] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[1] = TypeTerm::Vector(Box::new(TypeTerm::Double));
    fn_ir.param_term_hints[2] = TypeTerm::Matrix(Box::new(TypeTerm::Double));

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let xs = fn_ir.add_value(
        ValueKind::Param { index: 0 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ys = fn_ir.add_value(
        ValueKind::Param { index: 1 },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let _mat = fn_ir.add_value(
        ValueKind::Param { index: 2 },
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

    let mut null_calls = Vec::new();
    for callee in [
        "graphics::.filled.contour",
        "graphics::filled.contour",
        "graphics::cdplot",
        "graphics::close.screen",
        "graphics::co.intervals",
        "graphics::coplot",
        "graphics::curve",
        "graphics::erase.screen",
        "graphics::frame",
        "graphics::grid",
        "graphics::panel.smooth",
        "graphics::plot.default",
        "graphics::plot.design",
        "graphics::plot.function",
        "graphics::plot.new",
        "graphics::plot.window",
        "graphics::plot.xy",
        "graphics::lines.default",
        "graphics::points.default",
        "graphics::text.default",
        "graphics::contour.default",
        "graphics::image.default",
        "graphics::polypath",
        "graphics::rasterImage",
        "graphics::rect",
        "graphics::spineplot",
        "graphics::stars",
        "graphics::sunflowerplot",
    ] {
        null_calls.push(add_call(callee, vec![xs, ys]));
    }

    let mut any_list_calls = Vec::new();
    for callee in [
        "graphics::barplot",
        "graphics::barplot.default",
        "graphics::boxplot.default",
        "graphics::boxplot.matrix",
        "graphics::bxp",
        "graphics::hist.default",
        "graphics::screen",
        "graphics::split.screen",
    ] {
        any_list_calls.push(add_call(callee, vec![xs]));
    }

    let identify = add_call("graphics::identify", vec![xs, ys]);
    let locator = add_call("graphics::locator", vec![]);

    let mut double_vec_calls = Vec::new();
    for callee in [
        "graphics::Axis",
        "graphics::axis.Date",
        "graphics::axis.POSIXct",
    ] {
        double_vec_calls.push(add_call(callee, vec![xs]));
    }

    let lcm = add_call("graphics::lcm", vec![one]);
    let xinch = add_call("graphics::xinch", vec![one]);
    let yinch = add_call("graphics::yinch", vec![one]);
    let xyinch = add_call("graphics::xyinch", vec![xs]);

    fn_ir.blocks[b0].term = Terminator::Return(Some(locator));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");

    for vid in null_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Null);
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }

    for vid in any_list_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::List(Box::new(TypeTerm::Any))
        );
    }

    assert_eq!(out.values[identify].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[identify].value_ty.prim, PrimTy::Int);
    assert_eq!(
        out.values[identify].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );

    for vid in double_vec_calls {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }

    for vid in [lcm, xinch, yinch] {
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_term, TypeTerm::Double);
    }

    assert_eq!(out.values[xyinch].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[xyinch].value_ty.prim, PrimTy::Double);
    assert_eq!(
        out.values[xyinch].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );

    assert_eq!(out.values[locator].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[locator].value_ty.prim, PrimTy::Any);
    assert_eq!(
        out.values[locator].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])
    );
}
