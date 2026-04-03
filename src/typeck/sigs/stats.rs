//! Top-level router for `stats::*` signature inference.
//!
//! Larger families such as model helpers, htests, optimizers, smoothing, and
//! analysis utilities live in dedicated child modules so this file can focus on
//! dispatch and the remaining small residual table.

use crate::typeck::builtin_sigs::{
    first_arg_term, first_arg_type_state, first_numeric_prim, first_numeric_term,
    preserved_first_arg_type_without_len, preserved_head_tail_term, scalar_or_matrix_double_term,
    scalar_or_matrix_double_type, ts_like_output_term, ts_like_output_type,
    vectorized_first_arg_term, vectorized_first_arg_type, vectorized_scalar_or_vector_double_term,
    vectorized_scalar_or_vector_double_type,
};
use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

mod analysis;
mod htest;
mod model;
mod optimizer;
mod smoothing;

pub(crate) fn infer_stats_package_call(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
    if let Some(inferred) = model::infer_stats_model_package_call(callee) {
        return Some(inferred);
    }
    if let Some(inferred) = htest::infer_stats_htest_package_call(callee) {
        return Some(inferred);
    }
    if let Some(inferred) = optimizer::infer_stats_optimizer_package_call(callee) {
        return Some(inferred);
    }
    if let Some(inferred) = smoothing::infer_stats_smoothing_package_call(callee) {
        return Some(inferred);
    }
    if let Some(inferred) = analysis::infer_stats_analysis_package_call(callee) {
        return Some(inferred);
    }
    match callee {
        "stats::dnorm" | "stats::pnorm" | "stats::qnorm" | "stats::dbinom" | "stats::pbinom"
        | "stats::qbinom" | "stats::dpois" | "stats::ppois" | "stats::qpois" | "stats::dunif"
        | "stats::punif" | "stats::qunif" | "stats::dgamma" | "stats::pgamma" | "stats::qgamma"
        | "stats::dbeta" | "stats::pbeta" | "stats::qbeta" | "stats::dt" | "stats::pt"
        | "stats::qt" | "stats::df" | "stats::pf" | "stats::qf" | "stats::dchisq"
        | "stats::pchisq" | "stats::qchisq" | "stats::dexp" | "stats::pexp" | "stats::qexp"
        | "stats::dlnorm" | "stats::plnorm" | "stats::qlnorm" | "stats::dweibull"
        | "stats::pweibull" | "stats::qweibull" | "stats::dcauchy" | "stats::pcauchy"
        | "stats::qcauchy" | "stats::dgeom" | "stats::pgeom" | "stats::qgeom" | "stats::dhyper"
        | "stats::phyper" | "stats::qhyper" | "stats::dnbinom" | "stats::pnbinom"
        | "stats::qnbinom" | "stats::dlogis" | "stats::plogis" | "stats::qlogis"
        | "stats::pbirthday" | "stats::qbirthday" | "stats::ptukey" | "stats::qtukey"
        | "stats::psmirnov" | "stats::qsmirnov" | "stats::dsignrank" | "stats::psignrank"
        | "stats::qsignrank" | "stats::dwilcox" | "stats::pwilcox" | "stats::qwilcox" => {
            vectorized_scalar_or_vector_double_type(arg_tys)
        }
        "stats::rnorm" | "stats::runif" | "stats::rgamma" | "stats::rbeta" | "stats::rt"
        | "stats::rf" | "stats::rchisq" | "stats::rexp" | "stats::rlnorm" | "stats::rweibull"
        | "stats::rcauchy" | "stats::rlogis" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::rbinom" | "stats::rpois" | "stats::rgeom" | "stats::rhyper" | "stats::rnbinom"
        | "stats::rsignrank" | "stats::rwilcox" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::rsmirnov" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::acf2AR" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::p.adjust" => vectorized_scalar_or_vector_double_type(arg_tys),
        "stats::ppoints" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::dist" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::isoreg" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::toeplitz" | "stats::toeplitz2" | "stats::polym" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "stats::diffinv" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::addmargins" | "stats::ftable" | "stats::xtabs" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "stats::.vcov.aliased" | "stats::estVar" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::smooth" | "stats::smoothEnds" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::aggregate" | "stats::aggregate.data.frame" => {
            Some(TypeState::matrix(PrimTy::Any, false))
        }
        "stats::expand.model.frame" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::read.ftable" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::aggregate.ts" => Some(ts_like_output_type(first_arg_type_state(arg_tys))),
        "stats::reshape" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::get_all_vars" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::tsSmooth" => Some(ts_like_output_type(first_arg_type_state(arg_tys))),
        "stats::ave" => Some(vectorized_first_arg_type(first_arg_type_state(arg_tys))),
        "stats::reorder" | "stats::relevel" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::terms.formula" | "stats::delete.response" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats::simulate" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::DF2formula"
        | "stats::power"
        | "stats::C"
        | "stats::Pair"
        | "stats::preplot"
        | "stats::profile"
        | "stats::ppr"
        | "stats::KalmanLike"
        | "stats::makeARIMA"
        | "stats::eff.aovlist"
        | "stats::stat.anova"
        | "stats::Gamma"
        | "stats::.checkMFClasses"
        | "stats::.getXlevels"
        | "stats::.lm.fit"
        | "stats::.MFclass"
        | "stats::.preformat.ts" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::glm.fit" | "stats::lm.fit" | "stats::lm.wfit" | "stats::lsfit"
        | "stats::ls.diag" | "stats::line" | "stats::glm.control" | "stats::varimax"
        | "stats::promax" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::influence"
        | "stats::influence.measures"
        | "stats::qr.influence"
        | "stats::lm.influence" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::makepredictcall" => Some(TypeState::scalar(PrimTy::Any, false)),
        "stats::coefficients"
        | "stats::fitted.values"
        | "stats::resid"
        | "stats::predict.lm"
        | "stats::predict.glm"
        | "stats::residuals.lm"
        | "stats::residuals.glm"
        | "stats::hatvalues"
        | "stats::hat"
        | "stats::cooks.distance"
        | "stats::covratio"
        | "stats::dffits"
        | "stats::rstandard"
        | "stats::rstudent"
        | "stats::weighted.residuals"
        | "stats::na.contiguous" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::confint.lm" | "stats::confint.default" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "stats::dfbeta" | "stats::dfbetas" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::loadings" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::variable.names" => Some(TypeState::null()),
        "stats::is.empty.model" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "stats::naprint" => Some(TypeState::scalar(PrimTy::Char, false)),
        "stats::model.response" | "stats::model.extract" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats::summary.lm" | "stats::summary.glm" | "stats::summary.aov" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats::weights" | "stats::model.weights" | "stats::model.offset" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "stats::na.action" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::napredict" | "stats::naresid" => {
            Some(arg_tys.get(1).copied().unwrap_or_else(TypeState::unknown))
        }
        "stats::offset" | "stats::na.omit" | "stats::na.exclude" | "stats::na.pass"
        | "stats::na.fail" => Some(vectorized_first_arg_type(first_arg_type_state(arg_tys))),
        "stats::model.matrix.default" | "stats::model.matrix.lm" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "stats::case.names" => Some(TypeState::vector(PrimTy::Char, false)),
        "stats::complete.cases" => Some(TypeState::vector(PrimTy::Logical, false)),
        "stats::replications" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::.nknots.smspl" => Some(TypeState::scalar(PrimTy::Int, false)),
        "stats::fivenum" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::knots" | "stats::se.contrast" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::p.adjust.methods" => Some(TypeState::vector(PrimTy::Char, false)),
        "stats::sortedXyData" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::SSasymp" | "stats::SSasympOff" | "stats::SSasympOrig" | "stats::SSbiexp"
        | "stats::SSfol" | "stats::SSfpl" | "stats::SSgompertz" | "stats::SSlogis"
        | "stats::SSmicmen" | "stats::SSweibull" => {
            vectorized_scalar_or_vector_double_type(arg_tys)
        }
        "stats::NLSstAsymptotic"
        | "stats::NLSstClosestX"
        | "stats::NLSstLfAsymptote"
        | "stats::NLSstRtAsymptote" => vectorized_scalar_or_vector_double_type(arg_tys),
        "stats::splinefunH" | "stats::SSD" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::selfStart" | "stats::deriv" | "stats::deriv3" => {
            Some(TypeState::scalar(PrimTy::Any, false))
        }
        "stats::D" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::numericDeriv" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::cov" | "stats::cor" | "stats::var" => scalar_or_matrix_double_type(arg_tys),
        "stats::cov.wt" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::cov2cor" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::mahalanobis" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::rWishart" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::r2dtable" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::dmultinom" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::rmultinom" => Some(TypeState::matrix(PrimTy::Int, false)),
        "stats::ts" | "stats::as.ts" | "stats::hasTsp" | "stats::window" | "stats::window<-"
        | "stats::lag" | "stats::tsp<-" => Some(ts_like_output_type(first_arg_type_state(arg_tys))),
        "stats::contrasts<-" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "stats::ts.intersect" | "stats::ts.union" => {
            Some(TypeState::matrix(first_numeric_prim(arg_tys), false))
        }
        "stats::frequency" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::is.ts" | "stats::is.mts" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "stats::tsp" | "stats::start" | "stats::end" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "stats::deltat" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::time" | "stats::cycle" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::embed" => Some(TypeState::matrix(first_numeric_prim(arg_tys), false)),
        "stats::median" | "stats::median.default" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::weighted.mean" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::runmed" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::filter" => Some(ts_like_output_type(first_arg_type_state(arg_tys))),
        "stats::spec.taper" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "stats::arima0.diag"
        | "stats::cpgram"
        | "stats::plclust"
        | "stats::ts.plot"
        | "stats::write.ftable" => Some(TypeState::null()),
        "stats::stepfun" | "stats::as.stepfun" => Some(TypeState::scalar(PrimTy::Any, false)),
        "stats::is.stepfun" | "stats::is.leaf" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "stats::plot.ecdf" | "stats::plot.ts" | "stats::screeplot" => Some(TypeState::null()),
        "stats::plot.stepfun" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::setNames" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "stats::printCoefmat" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::summary.stepfun" => Some(TypeState::null()),
        _ => None,
    }
}

pub(crate) fn infer_stats_package_call_term(
    callee: &str,
    arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    if let Some(inferred) = model::infer_stats_model_package_call_term(callee) {
        return Some(inferred);
    }
    if let Some(inferred) = htest::infer_stats_htest_package_call_term(callee) {
        return Some(inferred);
    }
    if let Some(inferred) = optimizer::infer_stats_optimizer_package_call_term(callee) {
        return Some(inferred);
    }
    if let Some(inferred) = smoothing::infer_stats_smoothing_package_call_term(callee) {
        return Some(inferred);
    }
    if let Some(inferred) = analysis::infer_stats_analysis_package_call_term(callee) {
        return Some(inferred);
    }
    match callee {
        "stats::dnorm" | "stats::pnorm" | "stats::qnorm" | "stats::dbinom" | "stats::pbinom"
        | "stats::qbinom" | "stats::dpois" | "stats::ppois" | "stats::qpois" | "stats::dunif"
        | "stats::punif" | "stats::qunif" | "stats::dgamma" | "stats::pgamma" | "stats::qgamma"
        | "stats::dbeta" | "stats::pbeta" | "stats::qbeta" | "stats::dt" | "stats::pt"
        | "stats::qt" | "stats::df" | "stats::pf" | "stats::qf" | "stats::dchisq"
        | "stats::pchisq" | "stats::qchisq" | "stats::dexp" | "stats::pexp" | "stats::qexp"
        | "stats::dlnorm" | "stats::plnorm" | "stats::qlnorm" | "stats::dweibull"
        | "stats::pweibull" | "stats::qweibull" | "stats::dcauchy" | "stats::pcauchy"
        | "stats::qcauchy" | "stats::dgeom" | "stats::pgeom" | "stats::qgeom" | "stats::dhyper"
        | "stats::phyper" | "stats::qhyper" | "stats::dnbinom" | "stats::pnbinom"
        | "stats::qnbinom" | "stats::dlogis" | "stats::plogis" | "stats::qlogis"
        | "stats::pbirthday" | "stats::qbirthday" | "stats::ptukey" | "stats::qtukey"
        | "stats::psmirnov" | "stats::qsmirnov" | "stats::dsignrank" | "stats::psignrank"
        | "stats::qsignrank" | "stats::dwilcox" | "stats::pwilcox" | "stats::qwilcox" => {
            vectorized_scalar_or_vector_double_term(arg_terms)
        }
        "stats::rnorm" | "stats::runif" | "stats::rgamma" | "stats::rbeta" | "stats::rt"
        | "stats::rf" | "stats::rchisq" | "stats::rexp" | "stats::rlnorm" | "stats::rweibull"
        | "stats::rcauchy" | "stats::rlogis" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::rbinom" | "stats::rpois" | "stats::rgeom" | "stats::rhyper" | "stats::rnbinom"
        | "stats::rsignrank" | "stats::rwilcox" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "stats::rsmirnov" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::acf2AR" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::p.adjust" => vectorized_scalar_or_vector_double_term(arg_terms),
        "stats::ppoints" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::dist" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::isoreg" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "yf".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "yc".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "iKnots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("isOrd".to_string(), TypeTerm::Logical),
            ("ord".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::addmargins" | "stats::ftable" | "stats::xtabs" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "stats::.vcov.aliased" | "stats::estVar" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "stats::smooth" | "stats::smoothEnds" | "stats::diffinv" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "stats::aggregate" | "stats::aggregate.data.frame" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::expand.model.frame" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::read.ftable" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::aggregate.ts" => Some(ts_like_output_term(first_arg_term(arg_terms))),
        "stats::reshape" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::terms.formula" | "stats::delete.response" => Some(TypeTerm::Any),
        "stats::get_all_vars" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::tsSmooth" => Some(ts_like_output_term(first_arg_term(arg_terms))),
        "stats::simulate" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::ave" => Some(vectorized_first_arg_term(first_arg_term(arg_terms))),
        "stats::reorder" | "stats::relevel" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "stats::DF2formula"
        | "stats::power"
        | "stats::C"
        | "stats::Pair"
        | "stats::preplot"
        | "stats::profile"
        | "stats::ppr"
        | "stats::KalmanLike"
        | "stats::makeARIMA"
        | "stats::eff.aovlist"
        | "stats::stat.anova"
        | "stats::Gamma"
        | "stats::.checkMFClasses"
        | "stats::.getXlevels"
        | "stats::.lm.fit"
        | "stats::.MFclass"
        | "stats::.preformat.ts" => Some(TypeTerm::Any),
        "stats::glm.fit" => Some(TypeTerm::NamedList(vec![
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "fitted.values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "effects".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "R".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("rank".to_string(), TypeTerm::Int),
            ("qr".to_string(), TypeTerm::Any),
            ("family".to_string(), TypeTerm::Any),
            (
                "linear.predictors".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("deviance".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            ("null.deviance".to_string(), TypeTerm::Double),
            ("iter".to_string(), TypeTerm::Int),
            (
                "weights".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "prior.weights".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("df.residual".to_string(), TypeTerm::Int),
            ("df.null".to_string(), TypeTerm::Int),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("converged".to_string(), TypeTerm::Logical),
            ("boundary".to_string(), TypeTerm::Logical),
        ])),
        "stats::lm.fit" | "stats::lm.wfit" | "stats::lsfit" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "stats::makepredictcall" => Some(TypeTerm::Any),
        "stats::coefficients"
        | "stats::fitted.values"
        | "stats::resid"
        | "stats::predict.lm"
        | "stats::predict.glm"
        | "stats::residuals.lm"
        | "stats::residuals.glm"
        | "stats::hatvalues"
        | "stats::hat"
        | "stats::cooks.distance"
        | "stats::covratio"
        | "stats::dffits"
        | "stats::rstandard"
        | "stats::rstudent"
        | "stats::weighted.residuals"
        | "stats::na.contiguous" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::confint.lm" | "stats::confint.default" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "stats::dfbeta" | "stats::dfbetas" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::ls.diag" => Some(TypeTerm::NamedList(vec![
            ("std.dev".to_string(), TypeTerm::Double),
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "std.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "stud.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "cooks".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "dfits".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "correlation".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "std.err".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "cov.scaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "cov.unscaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::line" => Some(TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "fitted.values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::glm.control" => Some(TypeTerm::NamedList(vec![
            ("epsilon".to_string(), TypeTerm::Double),
            ("maxit".to_string(), TypeTerm::Int),
            ("trace".to_string(), TypeTerm::Logical),
        ])),
        "stats::influence" | "stats::lm.influence" => Some(TypeTerm::NamedList(vec![
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "sigma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "wt.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::influence.measures" => Some(TypeTerm::NamedList(vec![
            (
                "infmat".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "is.inf".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Logical)),
            ),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::qr.influence" => Some(TypeTerm::NamedList(vec![
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "sigma".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::varimax" | "stats::promax" => Some(TypeTerm::NamedList(vec![
            (
                "loadings".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "rotmat".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::loadings" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::variable.names" => Some(TypeTerm::Null),
        "stats::is.empty.model" => Some(TypeTerm::Logical),
        "stats::naprint" => Some(TypeTerm::Char),
        "stats::model.response" | "stats::model.extract" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Any)))
        }
        "stats::summary.lm" => Some(TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
            (
                "residuals".to_string(),
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
            ("sigma".to_string(), TypeTerm::Double),
            ("df".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            ("r.squared".to_string(), TypeTerm::Double),
            ("adj.r.squared".to_string(), TypeTerm::Double),
            (
                "fstatistic".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "cov.unscaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::summary.glm" => Some(TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
            (
                "family".to_string(),
                TypeTerm::NamedList(vec![
                    ("family".to_string(), TypeTerm::Char),
                    ("link".to_string(), TypeTerm::Char),
                    ("linkfun".to_string(), TypeTerm::Any),
                    ("linkinv".to_string(), TypeTerm::Any),
                    ("variance".to_string(), TypeTerm::Any),
                    ("dev.resids".to_string(), TypeTerm::Any),
                    ("aic".to_string(), TypeTerm::Any),
                    ("mu.eta".to_string(), TypeTerm::Any),
                    ("initialize".to_string(), TypeTerm::Any),
                    ("validmu".to_string(), TypeTerm::Any),
                    ("valideta".to_string(), TypeTerm::Any),
                    ("dispersion".to_string(), TypeTerm::Double),
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
        ])),
        "stats::summary.aov" => Some(TypeTerm::List(Box::new(TypeTerm::DataFrame(Vec::new())))),
        "stats::weights" | "stats::model.weights" | "stats::model.offset" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "stats::na.action" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "stats::napredict" | "stats::naresid" => {
            Some(arg_terms.get(1).cloned().unwrap_or(TypeTerm::Any))
        }
        "stats::offset" | "stats::na.omit" | "stats::na.exclude" | "stats::na.pass"
        | "stats::na.fail" => Some(vectorized_first_arg_term(first_arg_term(arg_terms))),
        "stats::case.names" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "stats::complete.cases" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "stats::replications" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "stats::.nknots.smspl" => Some(TypeTerm::Int),
        "stats::fivenum" => Some(TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(5))),
        "stats::knots" | "stats::se.contrast" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::p.adjust.methods" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "stats::sortedXyData" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::SSasymp" | "stats::SSasympOff" | "stats::SSasympOrig" | "stats::SSbiexp"
        | "stats::SSfol" | "stats::SSfpl" | "stats::SSgompertz" | "stats::SSlogis"
        | "stats::SSmicmen" | "stats::SSweibull" => {
            vectorized_scalar_or_vector_double_term(arg_terms)
        }
        "stats::NLSstAsymptotic"
        | "stats::NLSstClosestX"
        | "stats::NLSstLfAsymptote"
        | "stats::NLSstRtAsymptote" => vectorized_scalar_or_vector_double_term(arg_terms),
        "stats::splinefunH" | "stats::SSD" => Some(TypeTerm::Any),
        "stats::selfStart" | "stats::deriv" | "stats::deriv3" => Some(TypeTerm::Any),
        "stats::D" => Some(TypeTerm::Any),
        "stats::numericDeriv" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::cov" | "stats::cor" | "stats::var" => scalar_or_matrix_double_term(arg_terms),
        "stats::cov.wt" => Some(TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "center".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("n.obs".to_string(), TypeTerm::Int),
        ])),
        "stats::cov2cor" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::mahalanobis" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::rWishart" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![None, None, None],
        )),
        "stats::r2dtable" => Some(TypeTerm::List(Box::new(TypeTerm::Matrix(Box::new(
            TypeTerm::Int,
        ))))),
        "stats::dmultinom" => Some(TypeTerm::Double),
        "stats::rmultinom" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Int))),
        "stats::ts" | "stats::as.ts" | "stats::hasTsp" | "stats::window" | "stats::window<-"
        | "stats::lag" | "stats::tsp<-" => Some(ts_like_output_term(first_arg_term(arg_terms))),
        "stats::contrasts<-" => Some(preserved_head_tail_term(first_arg_term(arg_terms))),
        "stats::ts.intersect" | "stats::ts.union" => {
            Some(TypeTerm::Matrix(Box::new(first_numeric_term(arg_terms))))
        }
        "stats::frequency" => Some(TypeTerm::Double),
        "stats::is.ts" | "stats::is.mts" => Some(TypeTerm::Logical),
        "stats::tsp" => Some(TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(3))),
        "stats::start" | "stats::end" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::deltat" => Some(TypeTerm::Double),
        "stats::time" | "stats::cycle" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::embed" => Some(TypeTerm::Matrix(Box::new(first_numeric_term(arg_terms)))),
        "stats::median" | "stats::median.default" => Some(TypeTerm::Double),
        "stats::weighted.mean" => Some(TypeTerm::Double),
        "stats::runmed" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::filter" => Some(ts_like_output_term(first_arg_term(arg_terms))),
        "stats::spec.taper" => Some(preserved_head_tail_term(first_arg_term(arg_terms))),
        "stats::arima0.diag"
        | "stats::cpgram"
        | "stats::plclust"
        | "stats::ts.plot"
        | "stats::write.ftable" => Some(TypeTerm::Null),
        "stats::stepfun" | "stats::as.stepfun" => Some(TypeTerm::Any),
        "stats::is.stepfun" | "stats::is.leaf" => Some(TypeTerm::Logical),
        "stats::plot.ecdf" | "stats::plot.ts" | "stats::screeplot" | "stats::summary.stepfun" => {
            Some(TypeTerm::Null)
        }
        "stats::plot.stepfun" => Some(TypeTerm::NamedList(vec![
            (
                "t".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::toeplitz" | "stats::toeplitz2" | "stats::polym" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "stats::printCoefmat" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::setNames" => Some(preserved_head_tail_term(first_arg_term(arg_terms))),
        _ => None,
    }
}
