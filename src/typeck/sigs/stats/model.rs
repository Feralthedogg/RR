use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_stats_model_package_call(callee: &str) -> Option<TypeState> {
    match callee {
        "stats::lm" | "stats::glm" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::as.formula" | "stats::formula" | "stats::reformulate" => Some(TypeState::unknown()),
        "stats::contrasts"
        | "stats::contr.treatment"
        | "stats::contr.sum"
        | "stats::contr.helmert"
        | "stats::contr.SAS"
        | "stats::contr.poly" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::binomial"
        | "stats::gaussian"
        | "stats::poisson"
        | "stats::family"
        | "stats::make.link"
        | "stats::quasi"
        | "stats::quasibinomial"
        | "stats::quasipoisson"
        | "stats::inverse.gaussian" => Some(TypeState::vector(PrimTy::Any, false)),
        _ => None,
    }
}

pub(crate) fn infer_stats_model_package_call_term(callee: &str) -> Option<TypeTerm> {
    match callee {
        "stats::lm" | "stats::glm" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "stats::as.formula" | "stats::formula" | "stats::reformulate" => Some(TypeTerm::Any),
        "stats::contrasts"
        | "stats::contr.treatment"
        | "stats::contr.sum"
        | "stats::contr.helmert"
        | "stats::contr.SAS"
        | "stats::contr.poly" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::make.link" => Some(TypeTerm::NamedList(vec![
            ("linkfun".to_string(), TypeTerm::Any),
            ("linkinv".to_string(), TypeTerm::Any),
            ("mu.eta".to_string(), TypeTerm::Any),
            ("valideta".to_string(), TypeTerm::Any),
            ("name".to_string(), TypeTerm::Char),
        ])),
        "stats::family"
        | "stats::binomial"
        | "stats::gaussian"
        | "stats::poisson"
        | "stats::quasibinomial"
        | "stats::quasipoisson"
        | "stats::inverse.gaussian" => Some(TypeTerm::NamedList(vec![
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
        ])),
        "stats::quasi" => Some(TypeTerm::NamedList(vec![
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
            ("varfun".to_string(), TypeTerm::Char),
            ("dispersion".to_string(), TypeTerm::Double),
        ])),
        _ => None,
    }
}
