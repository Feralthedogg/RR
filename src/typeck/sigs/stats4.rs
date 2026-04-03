use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_stats4_package_call(callee: &str, _arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
        "stats4::mle"
        | "stats4::update"
        | "stats4::summary"
        | "stats4::profile"
        | "stats4::plot"
        | "stats4::.__C__mle"
        | "stats4::.__C__profile.mle"
        | "stats4::.__C__summary.mle"
        | "stats4::.__T__AIC:stats"
        | "stats4::.__T__BIC:stats"
        | "stats4::.__T__coef:stats"
        | "stats4::.__T__confint:stats"
        | "stats4::.__T__logLik:stats"
        | "stats4::.__T__nobs:stats"
        | "stats4::.__T__plot:base"
        | "stats4::.__T__profile:stats"
        | "stats4::.__T__show:methods"
        | "stats4::.__T__summary:base"
        | "stats4::.__T__update:stats"
        | "stats4::.__T__vcov:stats" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats4::coef" | "stats4::confint" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats4::vcov" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats4::logLik" | "stats4::AIC" | "stats4::BIC" => {
            Some(TypeState::scalar(PrimTy::Double, false))
        }
        "stats4::show" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats4::nobs" => Some(TypeState::scalar(PrimTy::Int, false)),
        _ => None,
    }
}

pub(crate) fn infer_stats4_package_call_term(
    callee: &str,
    _arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    match callee {
        "stats4::mle"
        | "stats4::update"
        | "stats4::summary"
        | "stats4::profile"
        | "stats4::plot"
        | "stats4::.__C__mle"
        | "stats4::.__C__profile.mle"
        | "stats4::.__C__summary.mle"
        | "stats4::.__T__AIC:stats"
        | "stats4::.__T__BIC:stats"
        | "stats4::.__T__coef:stats"
        | "stats4::.__T__confint:stats"
        | "stats4::.__T__logLik:stats"
        | "stats4::.__T__nobs:stats"
        | "stats4::.__T__plot:base"
        | "stats4::.__T__profile:stats"
        | "stats4::.__T__show:methods"
        | "stats4::.__T__summary:base"
        | "stats4::.__T__update:stats"
        | "stats4::.__T__vcov:stats" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "stats4::coef" | "stats4::confint" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats4::vcov" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats4::logLik" | "stats4::AIC" | "stats4::BIC" | "stats4::show" => Some(TypeTerm::Double),
        "stats4::nobs" => Some(TypeTerm::Int),
        _ => None,
    }
}
