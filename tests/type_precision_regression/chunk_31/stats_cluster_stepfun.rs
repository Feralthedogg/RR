use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn stats_cluster_helpers_have_direct_types() {
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
    let two_i = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
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
    let pts = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![xs, two_i],
            names: vec![None, Some("ncol".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let km = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::kmeans".to_string(),
            args: vec![pts, two_i],
            names: vec![None, None],
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
    let hc = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::hclust".to_string(),
            args: vec![dist_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cut = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cutree".to_string(),
            args: vec![hc, two_i],
            names: vec![None, Some("k".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_dist_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.dist".to_string(),
            args: vec![pts],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_hclust_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.hclust".to_string(),
            args: vec![hc],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_dend_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.dendrogram".to_string(),
            args: vec![hc],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let coph_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cophenetic".to_string(),
            args: vec![hc],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rect_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rect.hclust".to_string(),
            args: vec![hc, two_i],
            names: vec![None, Some("k".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let acf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::acf".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pacf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::pacf".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ccf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ccf".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(cut));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[km].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[km].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[hc].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[hc].value_ty.shape, ShapeTy::Vector);
    for vid in [dist_v, as_dist_v, coph_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(out.values[cut].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[cut].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[as_hclust_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[as_hclust_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[as_hclust_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "merge".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Int))
            ),
            (
                "height".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "order".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("labels".to_string(), TypeTerm::Any),
            ("method".to_string(), TypeTerm::Char),
            ("call".to_string(), TypeTerm::Any),
            ("dist.method".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(out.values[as_dend_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[as_dend_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[as_dend_v].value_term, TypeTerm::Any);
    assert_eq!(out.values[rect_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[rect_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[rect_v].value_term,
        TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(TypeTerm::Int))))
    );
    for vid in [acf_v, pacf_v, ccf_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }

    assert_eq!(
        out.values[km].value_term,
        TypeTerm::NamedList(vec![
            (
                "cluster".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "centers".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("totss".to_string(), TypeTerm::Double),
            (
                "withinss".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("tot.withinss".to_string(), TypeTerm::Double),
            ("betweenss".to_string(), TypeTerm::Double),
            (
                "size".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("iter".to_string(), TypeTerm::Int),
            ("ifault".to_string(), TypeTerm::Int),
        ])
    );
    assert_eq!(
        out.values[hc].value_term,
        TypeTerm::NamedList(vec![
            (
                "merge".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Int))
            ),
            (
                "height".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "order".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("labels".to_string(), TypeTerm::Any),
            ("method".to_string(), TypeTerm::Char),
            ("call".to_string(), TypeTerm::Any),
            ("dist.method".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[cut].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    for vid in [acf_v, pacf_v, ccf_v] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::NamedList(vec![
                (
                    "acf".to_string(),
                    TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, Some(1), Some(1)]),
                ),
                ("type".to_string(), TypeTerm::Char),
                ("n.used".to_string(), TypeTerm::Int),
                (
                    "lag".to_string(),
                    TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, Some(1), Some(1)]),
                ),
                ("series".to_string(), TypeTerm::Char),
                ("snames".to_string(), TypeTerm::Char),
            ])
        );
    }
}

#[test]
pub(crate) fn stats_stepfun_helpers_have_direct_types() {
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
    let _four = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(4.0)),
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
    let two_i = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![ten, twenty, thirty, forty],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pts = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![x, two_i],
            names: vec![None, Some("ncol".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dist_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dist".to_string(),
            args: vec![pts],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hc = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::hclust".to_string(),
            args: vec![dist_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dend = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.dendrogram".to_string(),
            args: vec![hc],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let step_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::stepfun".to_string(),
            args: vec![x, y],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let as_step_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.stepfun".to_string(),
            args: vec![step_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_step_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::is.stepfun".to_string(),
            args: vec![step_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plot_stepfun_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::plot.stepfun".to_string(),
            args: vec![step_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ecdf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ecdf".to_string(),
            args: vec![x],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plot_ecdf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::plot.ecdf".to_string(),
            args: vec![ecdf_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let air_passengers = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::AirPassengers".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plot_ts_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::plot.ts".to_string(),
            args: vec![air_passengers],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prcomp_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::prcomp".to_string(),
            args: vec![pts],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let screeplot_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::screeplot".to_string(),
            args: vec![prcomp_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dendrapply_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dendrapply".to_string(),
            args: vec![dend, step_v],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let is_leaf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::is.leaf".to_string(),
            args: vec![dend],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let order_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::order.dendrogram".to_string(),
            args: vec![dend],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(order_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [step_v, as_step_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_term, TypeTerm::Any);
    }
    assert_eq!(out.values[plot_stepfun_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[plot_stepfun_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[plot_stepfun_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "t".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    for vid in [is_step_v, is_leaf_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Logical);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
        assert_eq!(out.values[vid].value_term, TypeTerm::Logical);
    }
    for vid in [plot_ecdf_v, plot_ts_v, screeplot_v] {
        assert_eq!(
            out.values[vid].value_ty,
            rr::compiler::internal::typeck::TypeState::null()
        );
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }
    assert_eq!(out.values[dendrapply_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[dendrapply_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(out.values[dendrapply_v].value_term, TypeTerm::Any);
    assert_eq!(out.values[order_v].value_ty.prim, PrimTy::Int);
    assert_eq!(out.values[order_v].value_ty.shape, ShapeTy::Vector);
    assert_eq!(
        out.values[order_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
}
