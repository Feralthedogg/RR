use super::type_precision_regression_common::*;

#[test]
fn stats_cluster_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

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
    let three = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(4.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
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
fn stats_stepfun_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

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
    let three = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let _four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(4.0)),
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
    let twenty = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(20.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let thirty = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(30.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let forty = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(40.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
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
        assert_eq!(out.values[vid].value_ty, RR::typeck::TypeState::null());
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

#[test]
fn stats_htest_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

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
    let three = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(3.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let four = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(4.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let five = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(5.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let six = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(6.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seven = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(7.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let eight = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(8.0)),
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
    let twenty = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(20.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let thirty = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(30.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let false_v = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Bool(false)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let one_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let holm = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("holm".to_string())),
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
    let ljung_box = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Ljung-Box".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grp_formula = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("y ~ grp".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let x = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let y = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![two, three, four],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pair_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let groups = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![a, a, b, b],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let success = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![three, four, five],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let totals = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![five, six, seven],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let two_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grouped_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three, four, five, six],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grouped_labels = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![a, a, b, b, c_name, c_name],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grouped_factor = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::factor".to_string(),
            args: vec![grouped_labels],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ts_input = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three, four, five, six, seven, eight],
            names: vec![None, None, None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let box_series = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ts".to_string(),
            args: vec![ts_input],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matrix_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three, two, three, four],
            names: vec![None, None, None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let three_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rank_matrix = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![matrix_data, three_i],
            names: vec![None, Some("nrow".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let array_dims = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![two_i, two_i, three_i],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let array_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![
                one, two, three, four, five, six, one, two, three, four, five, six,
            ],
            names: vec![
                None, None, None, None, None, None, None, None, None, None, None, None,
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let contingency_3d = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::array".to_string(),
            args: vec![array_data, array_dims],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grouped_df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![grouped_vals, grouped_factor],
            names: vec![Some("y".to_string()), Some("grp".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let grouped_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![grp_formula],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let ttest = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::t.test".to_string(),
            args: vec![x, y],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let wtest = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::wilcox.test".to_string(),
            args: vec![x, y, false_v],
            names: vec![None, None, Some("exact".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let btest = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::binom.test".to_string(),
            args: vec![three, five],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ptest = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::prop.test".to_string(),
            args: vec![success, totals],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let potest = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::poisson.test".to_string(),
            args: vec![success, x],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![ten, twenty, twenty, thirty],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let contingency = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![mat_data, two_i],
            names: vec![None, Some("nrow".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let chisq = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::chisq.test".to_string(),
            args: vec![contingency],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fisher = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::fisher.test".to_string(),
            args: vec![contingency],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cor_test = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cor.test".to_string(),
            args: vec![pair_vals, pair_vals],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ks_test = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ks.test".to_string(),
            args: vec![pair_vals, pair_vals],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let shapiro = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::shapiro.test".to_string(),
            args: vec![pair_vals],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ansari = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ansari.test".to_string(),
            args: vec![x, y, false_v],
            names: vec![None, None, Some("exact".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let bartlett = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::bartlett.test".to_string(),
            args: vec![grouped_vals, grouped_factor],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matrix_formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str(
            "base::cbind(mpg, disp) ~ factor(cyl)".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let matrix_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![matrix_formula_src],
            names: vec![None],
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
    let mlm_fit = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lm".to_string(),
            args: vec![matrix_formula, mtcars],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mauchly = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::mauchly.test".to_string(),
            args: vec![mlm_fit],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let box_test = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::Box.test".to_string(),
            args: vec![box_series, one_i, ljung_box],
            names: vec![None, Some("lag".to_string()), Some("type".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let fligner = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::fligner.test".to_string(),
            args: vec![grouped_vals, grouped_factor],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let friedman = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::friedman.test".to_string(),
            args: vec![rank_matrix],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let kruskal = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::kruskal.test".to_string(),
            args: vec![grouped_vals, grouped_factor],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mantelhaen = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::mantelhaen.test".to_string(),
            args: vec![contingency_3d],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mcnemar = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::mcnemar.test".to_string(),
            args: vec![contingency],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mood = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::mood.test".to_string(),
            args: vec![x, y, false_v],
            names: vec![None, None, Some("exact".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let oneway = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::oneway.test".to_string(),
            args: vec![grouped_formula, grouped_df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prop_trend = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::prop.trend.test".to_string(),
            args: vec![success, totals],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let quade = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::quade.test".to_string(),
            args: vec![rank_matrix],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let var_test = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::var.test".to_string(),
            args: vec![x, y],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pair_t = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::pairwise.t.test".to_string(),
            args: vec![pair_vals, groups, holm],
            names: vec![None, None, Some("p.adjust.method".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pair_w = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::pairwise.wilcox.test".to_string(),
            args: vec![pair_vals, groups, holm, false_v],
            names: vec![
                None,
                None,
                Some("p.adjust.method".to_string()),
                Some("exact".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pair_p = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::pairwise.prop.test".to_string(),
            args: vec![success, totals, holm],
            names: vec![None, None, Some("p.adjust.method".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(ttest));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [
        ttest, wtest, btest, ptest, potest, chisq, fisher, cor_test, ks_test, shapiro, ansari,
        bartlett, mauchly, box_test, fligner, friedman, kruskal, mantelhaen, mcnemar, mood, oneway,
        prop_trend, quade, var_test, pair_t, pair_w, pair_p,
    ] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Any);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }

    assert_eq!(
        out.values[ttest].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "estimate".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("null.value".to_string(), TypeTerm::Double),
            ("stderr".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[wtest].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Any),
            ("p.value".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[btest].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("estimate".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[ptest].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "estimate".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("null.value".to_string(), TypeTerm::Any),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[potest].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "estimate".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[chisq].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
            (
                "observed".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "expected".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "residuals".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            (
                "stdres".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[fisher].value_term,
        TypeTerm::NamedList(vec![
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("estimate".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[cor_test].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("estimate".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[ks_test].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
            ("exact".to_string(), TypeTerm::Logical),
        ])
    );
    assert_eq!(
        out.values[shapiro].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[ansari].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[bartlett].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("data.name".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[mauchly].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    for vid in [box_test, fligner, friedman, mcnemar, prop_trend] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::NamedList(vec![
                ("statistic".to_string(), TypeTerm::Double),
                ("parameter".to_string(), TypeTerm::Double),
                ("p.value".to_string(), TypeTerm::Double),
                ("method".to_string(), TypeTerm::Char),
                ("data.name".to_string(), TypeTerm::Char),
            ])
        );
    }
    assert_eq!(
        out.values[kruskal].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Int),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[mantelhaen].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("estimate".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[mood].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[oneway].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            (
                "parameter".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[quade].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            (
                "parameter".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical))
            ),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    assert_eq!(
        out.values[var_test].value_term,
        TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            (
                "parameter".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("estimate".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])
    );
    for vid in [pair_t, pair_w, pair_p] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::NamedList(vec![
                ("method".to_string(), TypeTerm::Char),
                ("data.name".to_string(), TypeTerm::Char),
                (
                    "p.value".to_string(),
                    TypeTerm::Matrix(Box::new(TypeTerm::Double))
                ),
                ("p.adjust.method".to_string(), TypeTerm::Char),
            ])
        );
    }
}
