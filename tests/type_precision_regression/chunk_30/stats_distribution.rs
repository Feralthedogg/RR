use crate::type_precision_regression_common::*;

#[test]
pub(crate) fn stats_distribution_helpers_have_direct_types() {
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
    let ten = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(10.0)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let half = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Float(0.5)),
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
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(5)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let seven_i = fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Int(7)),
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
