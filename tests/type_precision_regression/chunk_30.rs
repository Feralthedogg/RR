use super::type_precision_regression_common::*;

#[test]
fn stats_distribution_helpers_have_direct_types() {
    let mut fn_ir = FnIR::new("Sym_main".to_string(), vec![]);

    let b0 = fn_ir.add_block();
    fn_ir.entry = b0;
    fn_ir.body_head = b0;

    let zero = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.0)),
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
    let ten = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(10.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let half = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let probs = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![half, half],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![zero, one],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let dnorm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dnorm".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pnorm_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::pnorm".to_string(),
            args: vec![zero],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qnorm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qnorm".to_string(),
            args: vec![probs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dpois_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dpois".to_string(),
            args: vec![one, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qbinom_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qbinom".to_string(),
            args: vec![half, three, half],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let runif_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::runif".to_string(),
            args: vec![three],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rnorm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rnorm".to_string(),
            args: vec![three],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rpois_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rpois".to_string(),
            args: vec![three, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rbinom_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rbinom".to_string(),
            args: vec![three, three, half],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dgamma_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dgamma".to_string(),
            args: vec![xs, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qbeta_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qbeta".to_string(),
            args: vec![probs, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pt_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::pt".to_string(),
            args: vec![zero, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qf_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qf".to_string(),
            args: vec![half, three, two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pchisq_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::pchisq".to_string(),
            args: vec![one, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dexp_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dexp".to_string(),
            args: vec![xs, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pcauchy_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::pcauchy".to_string(),
            args: vec![zero, zero, one],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qgeom_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qgeom".to_string(),
            args: vec![half, half],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dhyper_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dhyper".to_string(),
            args: vec![one, three, two, two],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qnbinom_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qnbinom".to_string(),
            args: vec![half, three, half],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let plogis_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::plogis".to_string(),
            args: vec![zero, zero, one],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qsignrank_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qsignrank".to_string(),
            args: vec![half, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dwilcox_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::dwilcox".to_string(),
            args: vec![one, three, two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let pbirthday_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::pbirthday".to_string(),
            args: vec![three],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qbirthday_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qbirthday".to_string(),
            args: vec![half],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ptukey_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ptukey".to_string(),
            args: vec![three, four, ten],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qtukey_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qtukey".to_string(),
            args: vec![half, four, ten],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let five_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seven_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(7)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let sizes = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![five_i, seven_i],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let psmirnov_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::psmirnov".to_string(),
            args: vec![probs, sizes],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qsmirnov_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qsmirnov".to_string(),
            args: vec![probs, sizes],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let acf2ar_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::acf2AR".to_string(),
            args: vec![probs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rgamma_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rgamma".to_string(),
            args: vec![three, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rbeta_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rbeta".to_string(),
            args: vec![three, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rt_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rt".to_string(),
            args: vec![three, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rf".to_string(),
            args: vec![three, three, two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rchisq_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rchisq".to_string(),
            args: vec![three, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rexp_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rexp".to_string(),
            args: vec![three, two],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rlnorm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rlnorm".to_string(),
            args: vec![three, zero, one],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rweibull_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rweibull".to_string(),
            args: vec![three, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rcauchy_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rcauchy".to_string(),
            args: vec![three, zero, one],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rgeom_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rgeom".to_string(),
            args: vec![three, half],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rhyper_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rhyper".to_string(),
            args: vec![three, three, two, two],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rnbinom_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rnbinom".to_string(),
            args: vec![three, three, half],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rlogis_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rlogis".to_string(),
            args: vec![three, zero, one],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rsignrank_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rsignrank".to_string(),
            args: vec![three, three],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rsmirnov_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rsmirnov".to_string(),
            args: vec![three, sizes],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let rwilcox_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::rwilcox".to_string(),
            args: vec![three, three, two],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(dpois_s));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [
        dnorm_v, qnorm_v, dgamma_v, qbeta_v, dexp_v, runif_v, rnorm_v, rgamma_v, rbeta_v, rt_v,
        rf_v, rchisq_v, rexp_v, rlnorm_v, rweibull_v, rcauchy_v, rlogis_v,
    ] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
    }
    for vid in [
        rpois_v,
        rbinom_v,
        rgeom_v,
        rhyper_v,
        rnbinom_v,
        rsignrank_v,
        rwilcox_v,
    ] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Int);
    }
    assert_eq!(out.values[pnorm_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[dpois_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[qbinom_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[pt_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[qf_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[pchisq_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[pcauchy_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[qgeom_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[dhyper_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[qnbinom_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[plogis_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[qsignrank_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[dwilcox_s].value_ty.shape, ShapeTy::Scalar);
    for vid in [pbirthday_s, qbirthday_s, ptukey_s, qtukey_s] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Scalar);
    }
    for vid in [psmirnov_v, qsmirnov_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
    }
    assert_eq!(out.values[acf2ar_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[acf2ar_v].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(
        out.values[dnorm_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[pnorm_s].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[qnorm_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[dpois_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[qbinom_s].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[dgamma_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[dexp_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[qbeta_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[pt_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[qf_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[pchisq_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[pcauchy_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[qgeom_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[dhyper_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[qnbinom_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[plogis_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[qsignrank_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[dwilcox_s].value_term, TypeTerm::Double);
    for vid in [pbirthday_s, qbirthday_s, ptukey_s, qtukey_s] {
        assert_eq!(out.values[vid].value_term, TypeTerm::Double);
    }
    for vid in [psmirnov_v, qsmirnov_v] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(
        out.values[acf2ar_v].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[runif_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[rnorm_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[rpois_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    assert_eq!(
        out.values[rbinom_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    );
    for vid in [
        rgamma_v, rbeta_v, rt_v, rf_v, rchisq_v, rexp_v, rlnorm_v, rweibull_v, rcauchy_v, rlogis_v,
    ] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    for vid in [rgeom_v, rhyper_v, rnbinom_v, rsignrank_v, rwilcox_v] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        );
    }
    assert_eq!(
        out.values[rsmirnov_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
}

#[test]
fn stats_misc_helpers_have_direct_types() {
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
    let half = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let hundredth = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Float(0.01)),
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
    let three_i = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Int(3)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xs = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let probs = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![hundredth, half, half],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat_data = fn_ir.add_value(
        ValueKind::Call {
            callee: "c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mat = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::matrix".to_string(),
            args: vec![mat_data, two_i],
            names: vec![None, Some("ncol".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let padj_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::p.adjust".to_string(),
            args: vec![probs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let padj_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::p.adjust".to_string(),
            args: vec![hundredth],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ppoints_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ppoints".to_string(),
            args: vec![three_i],
            names: vec![None],
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
    let density_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::density".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qqnorm_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qqnorm".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qqplot_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qqplot".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let qqline_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::qqline".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let interaction_plot_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::interaction.plot".to_string(),
            args: vec![xs, xs, xs],
            names: vec![None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let lag_plot_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::lag.plot".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let monthplot_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::monthplot".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let scatter_smooth_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::scatter.smooth".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let poly_m = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::poly".to_string(),
            args: vec![xs, two_i],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let prcomp_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::prcomp".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ecdf_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ecdf".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let biplot_n = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::biplot".to_string(),
            args: vec![prcomp_v],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cov_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cov".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let cor_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::cor".to_string(),
            args: vec![xs, xs],
            names: vec![None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let var_m = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::var".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let iqr_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::IQR".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let mad_s = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::mad".to_string(),
            args: vec![xs],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(padj_s));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    assert_eq!(out.values[padj_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[padj_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[ppoints_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[dist_v].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[cov_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[cor_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[var_m].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[poly_m].value_ty.shape, ShapeTy::Matrix);
    assert_eq!(out.values[poly_m].value_ty.prim, PrimTy::Double);
    assert_eq!(out.values[qqline_n].value_ty, RR::typeck::TypeState::null());
    assert_eq!(out.values[qqplot_v].value_ty.prim, PrimTy::Any);
    assert_eq!(out.values[qqplot_v].value_ty.shape, ShapeTy::Vector);
    for vid in [
        interaction_plot_n,
        lag_plot_n,
        monthplot_n,
        scatter_smooth_n,
        biplot_n,
    ] {
        assert_eq!(out.values[vid].value_ty, RR::typeck::TypeState::null());
    }
    assert_eq!(out.values[ecdf_v].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[iqr_s].value_ty.shape, ShapeTy::Scalar);
    assert_eq!(out.values[mad_s].value_ty.shape, ShapeTy::Scalar);

    assert_eq!(
        out.values[padj_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[padj_s].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[ppoints_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[dist_v].value_term,
        TypeTerm::Vector(Box::new(TypeTerm::Double))
    );
    assert_eq!(out.values[cov_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[cor_s].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[var_m].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[density_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            ("bw".to_string(), TypeTerm::Double),
            ("n".to_string(), TypeTerm::Int),
            ("old.coords".to_string(), TypeTerm::Logical),
            ("call".to_string(), TypeTerm::Any),
            ("data.name".to_string(), TypeTerm::Char),
            ("has.na".to_string(), TypeTerm::Logical),
        ])
    );
    assert_eq!(
        out.values[qqnorm_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(
        out.values[qqplot_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(out.values[qqline_n].value_term, TypeTerm::Null);
    for vid in [
        interaction_plot_n,
        lag_plot_n,
        monthplot_n,
        scatter_smooth_n,
        biplot_n,
    ] {
        assert_eq!(out.values[vid].value_term, TypeTerm::Null);
    }
    assert_eq!(out.values[iqr_s].value_term, TypeTerm::Double);
    assert_eq!(out.values[mad_s].value_term, TypeTerm::Double);
    assert_eq!(
        out.values[poly_m].value_term,
        TypeTerm::Matrix(Box::new(TypeTerm::Double))
    );
    assert_eq!(
        out.values[prcomp_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "sdev".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "rotation".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
            ("center".to_string(), TypeTerm::Any),
            ("scale".to_string(), TypeTerm::Any),
            (
                "x".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double))
            ),
        ])
    );
    assert_eq!(out.values[ecdf_v].value_term, TypeTerm::Any);
}

#[test]
fn stats_table_helpers_have_direct_types() {
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
        ValueKind::Const(RR::syntax::ast::Lit::Int(2)),
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
    let titanic = fn_ir.add_value(
        ValueKind::Load {
            var: "datasets::Titanic".to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let freq = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![one, two, three, four],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_a = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("A".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_b = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("B".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let yes = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("Yes".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let no = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str("No".to_string())),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let class_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![class_a, class_a, class_b, class_b],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let surv_vals = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::c".to_string(),
            args: vec![yes, no, yes, no],
            names: vec![None, None, None, None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tab_df = fn_ir.add_value(
        ValueKind::Call {
            callee: "base::data.frame".to_string(),
            args: vec![freq, class_vals, surv_vals],
            names: vec![
                Some("Freq".to_string()),
                Some("Class".to_string()),
                Some("Survived".to_string()),
            ],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xtabs_formula_src = fn_ir.add_value(
        ValueKind::Const(RR::syntax::ast::Lit::Str(
            "Freq ~ Class + Survived".to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xtabs_formula = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::as.formula".to_string(),
            args: vec![xtabs_formula_src],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let addmargins_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::addmargins".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let ftable_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::ftable".to_string(),
            args: vec![titanic],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let xtabs_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::xtabs".to_string(),
            args: vec![xtabs_formula, tab_df],
            names: vec![None, Some("data".to_string())],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let isoreg_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::isoreg".to_string(),
            args: vec![mat_data],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let smooth_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::smooth".to_string(),
            args: vec![mat_data],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let smooth_ends_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::smoothEnds".to_string(),
            args: vec![mat_data],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let line_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::line".to_string(),
            args: vec![mat_data],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let varimax_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::varimax".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let promax_v = fn_ir.add_value(
        ValueKind::Call {
            callee: "stats::promax".to_string(),
            args: vec![mat],
            names: vec![None],
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[b0].term = Terminator::Return(Some(addmargins_v));

    let mut all = FxHashMap::default();
    all.insert("Sym_main".to_string(), fn_ir);
    analyze_program(&mut all, TypeConfig::default()).expect("type analysis");

    let out = all.get("Sym_main").expect("fn");
    for vid in [addmargins_v, ftable_v, xtabs_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Matrix);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Matrix(Box::new(TypeTerm::Double))
        );
    }
    for vid in [smooth_v, smooth_ends_v] {
        assert_eq!(out.values[vid].value_ty.prim, PrimTy::Double);
        assert_eq!(out.values[vid].value_ty.shape, ShapeTy::Vector);
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::Vector(Box::new(TypeTerm::Double))
        );
    }
    assert_eq!(
        out.values[isoreg_v].value_term,
        TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "yf".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "yc".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double))
            ),
            (
                "iKnots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int))
            ),
            ("isOrd".to_string(), TypeTerm::Logical),
            ("ord".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
        ])
    );
    assert_eq!(
        out.values[line_v].value_term,
        TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
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
        ])
    );
    for vid in [varimax_v, promax_v] {
        assert_eq!(
            out.values[vid].value_term,
            TypeTerm::NamedList(vec![
                (
                    "loadings".to_string(),
                    TypeTerm::Matrix(Box::new(TypeTerm::Double))
                ),
                (
                    "rotmat".to_string(),
                    TypeTerm::Matrix(Box::new(TypeTerm::Double))
                ),
            ])
        );
    }
}
