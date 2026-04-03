use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_stats_optimizer_package_call(callee: &str) -> Option<TypeState> {
    match callee {
        "stats::step"
        | "stats::optim"
        | "stats::optimize"
        | "stats::optimise"
        | "stats::nlm"
        | "stats::nlminb"
        | "stats::constrOptim"
        | "stats::uniroot"
        | "stats::integrate"
        | "stats::HoltWinters"
        | "stats::StructTS"
        | "stats::KalmanForecast"
        | "stats::KalmanRun"
        | "stats::KalmanSmooth"
        | "stats::arima"
        | "stats::arima0"
        | "stats::ar"
        | "stats::ar.yw"
        | "stats::ar.mle"
        | "stats::ar.burg"
        | "stats::ar.ols"
        | "stats::spec.ar"
        | "stats::kernel"
        | "stats::nls"
        | "stats::nls.control" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::optimHess" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::getInitial" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::is.tskernel" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "stats::df.kernel" | "stats::bandwidth.kernel" => {
            Some(TypeState::scalar(PrimTy::Double, false))
        }
        "stats::kernapply" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::arima.sim" | "stats::ARMAacf" | "stats::ARMAtoMA" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "stats::convolve" | "stats::fft" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::mvfft" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::nextn" => Some(TypeState::scalar(PrimTy::Int, false)),
        "stats::tsdiag" => Some(TypeState::null()),
        _ => None,
    }
}

pub(crate) fn infer_stats_optimizer_package_call_term(callee: &str) -> Option<TypeTerm> {
    match callee {
        "stats::optim" => Some(TypeTerm::NamedList(vec![
            (
                "par".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("value".to_string(), TypeTerm::Double),
            (
                "counts".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("convergence".to_string(), TypeTerm::Int),
            ("message".to_string(), TypeTerm::Any),
        ])),
        "stats::optimHess" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::optimize" | "stats::optimise" => Some(TypeTerm::NamedList(vec![
            ("minimum".to_string(), TypeTerm::Double),
            ("objective".to_string(), TypeTerm::Double),
        ])),
        "stats::nlm" => Some(TypeTerm::NamedList(vec![
            ("minimum".to_string(), TypeTerm::Double),
            (
                "estimate".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "gradient".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("code".to_string(), TypeTerm::Int),
            ("iterations".to_string(), TypeTerm::Int),
        ])),
        "stats::nlminb" => Some(TypeTerm::NamedList(vec![
            (
                "par".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("objective".to_string(), TypeTerm::Double),
            ("convergence".to_string(), TypeTerm::Int),
            ("iterations".to_string(), TypeTerm::Int),
            (
                "evaluations".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("message".to_string(), TypeTerm::Char),
        ])),
        "stats::constrOptim" => Some(TypeTerm::NamedList(vec![
            (
                "par".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("value".to_string(), TypeTerm::Double),
            (
                "counts".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("convergence".to_string(), TypeTerm::Int),
            ("message".to_string(), TypeTerm::Any),
            ("outer.iterations".to_string(), TypeTerm::Int),
            ("barrier.value".to_string(), TypeTerm::Double),
        ])),
        "stats::uniroot" => Some(TypeTerm::NamedList(vec![
            ("root".to_string(), TypeTerm::Double),
            ("f.root".to_string(), TypeTerm::Double),
            ("iter".to_string(), TypeTerm::Int),
            ("init.it".to_string(), TypeTerm::Int),
            ("estim.prec".to_string(), TypeTerm::Double),
        ])),
        "stats::integrate" => Some(TypeTerm::NamedList(vec![
            ("value".to_string(), TypeTerm::Double),
            ("abs.error".to_string(), TypeTerm::Double),
            ("subdivisions".to_string(), TypeTerm::Int),
            ("message".to_string(), TypeTerm::Char),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::HoltWinters" => Some(TypeTerm::NamedList(vec![
            (
                "fitted".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("alpha".to_string(), TypeTerm::Double),
            ("beta".to_string(), TypeTerm::Any),
            ("gamma".to_string(), TypeTerm::Any),
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("seasonal".to_string(), TypeTerm::Char),
            ("SSE".to_string(), TypeTerm::Double),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::StructTS" => Some(TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("loglik".to_string(), TypeTerm::Double),
            ("loglik0".to_string(), TypeTerm::Double),
            (
                "data".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "fitted".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("call".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("code".to_string(), TypeTerm::Int),
            ("model".to_string(), TypeTerm::Any),
            (
                "model0".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any)),
            ),
            (
                "xtsp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::KalmanForecast" => Some(TypeTerm::NamedList(vec![
            (
                "pred".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "var".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::KalmanRun" => Some(TypeTerm::NamedList(vec![
            (
                "values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "resid".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "states".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::KalmanSmooth" => Some(TypeTerm::NamedList(vec![
            (
                "smooth".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "var".to_string(),
                TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, None, None]),
            ),
        ])),
        "stats::arima0" => Some(TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("sigma2".to_string(), TypeTerm::Double),
            (
                "var.coef".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("mask".to_string(), TypeTerm::Any),
            ("loglik".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            (
                "arma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("call".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("code".to_string(), TypeTerm::Int),
            ("n.cond".to_string(), TypeTerm::Int),
        ])),
        "stats::arima" => Some(TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("sigma2".to_string(), TypeTerm::Double),
            (
                "var.coef".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("mask".to_string(), TypeTerm::Any),
            ("loglik".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            (
                "arma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("call".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("code".to_string(), TypeTerm::Int),
            ("n.cond".to_string(), TypeTerm::Int),
            ("nobs".to_string(), TypeTerm::Int),
            ("model".to_string(), TypeTerm::Any),
        ])),
        "stats::ar" | "stats::ar.yw" | "stats::ar.mle" | "stats::ar.burg" => {
            Some(TypeTerm::NamedList(vec![
                ("order".to_string(), TypeTerm::Int),
                (
                    "ar".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double)),
                ),
                ("var.pred".to_string(), TypeTerm::Double),
                ("x.mean".to_string(), TypeTerm::Double),
                (
                    "aic".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double)),
                ),
                ("n.used".to_string(), TypeTerm::Int),
                ("n.obs".to_string(), TypeTerm::Int),
                ("order.max".to_string(), TypeTerm::Double),
                (
                    "partialacf".to_string(),
                    TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, Some(1), Some(1)]),
                ),
                (
                    "resid".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double)),
                ),
                ("method".to_string(), TypeTerm::Char),
                ("series".to_string(), TypeTerm::Char),
                ("frequency".to_string(), TypeTerm::Double),
                ("call".to_string(), TypeTerm::Any),
                (
                    "asy.var.coef".to_string(),
                    TypeTerm::Matrix(Box::new(TypeTerm::Double)),
                ),
            ]))
        }
        "stats::ar.ols" => Some(TypeTerm::NamedList(vec![
            ("order".to_string(), TypeTerm::Int),
            (
                "ar".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("var.pred".to_string(), TypeTerm::Double),
            ("x.mean".to_string(), TypeTerm::Double),
            (
                "aic".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("n.used".to_string(), TypeTerm::Int),
            ("n.obs".to_string(), TypeTerm::Int),
            ("order.max".to_string(), TypeTerm::Double),
            (
                "partialacf".to_string(),
                TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, Some(1), Some(1)]),
            ),
            (
                "resid".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("method".to_string(), TypeTerm::Char),
            ("series".to_string(), TypeTerm::Char),
            ("frequency".to_string(), TypeTerm::Double),
            ("call".to_string(), TypeTerm::Any),
            ("x.intercept".to_string(), TypeTerm::Double),
            (
                "asy.se.coef".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any)),
            ),
        ])),
        "stats::kernel" => Some(TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("m".to_string(), TypeTerm::Int),
        ])),
        "stats::is.tskernel" => Some(TypeTerm::Logical),
        "stats::df.kernel" | "stats::bandwidth.kernel" => Some(TypeTerm::Double),
        "stats::kernapply" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::convolve" | "stats::fft" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "stats::mvfft" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Any))),
        "stats::nextn" => Some(TypeTerm::Int),
        "stats::spec.ar" => Some(TypeTerm::NamedList(vec![
            (
                "freq".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "spec".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("coh".to_string(), TypeTerm::Any),
            ("phase".to_string(), TypeTerm::Any),
            ("n.used".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("snames".to_string(), TypeTerm::Any),
        ])),
        "stats::nls" => Some(TypeTerm::NamedList(vec![
            ("m".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
            (
                "convInfo".to_string(),
                TypeTerm::NamedList(vec![
                    ("isConv".to_string(), TypeTerm::Logical),
                    ("finIter".to_string(), TypeTerm::Int),
                    ("finTol".to_string(), TypeTerm::Double),
                    ("stopCode".to_string(), TypeTerm::Int),
                    ("stopMessage".to_string(), TypeTerm::Char),
                ]),
            ),
            ("data".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
            (
                "dataClasses".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "control".to_string(),
                TypeTerm::NamedList(vec![
                    ("maxiter".to_string(), TypeTerm::Double),
                    ("tol".to_string(), TypeTerm::Double),
                    ("minFactor".to_string(), TypeTerm::Double),
                    ("printEval".to_string(), TypeTerm::Logical),
                    ("warnOnly".to_string(), TypeTerm::Logical),
                    ("scaleOffset".to_string(), TypeTerm::Double),
                    ("nDcentral".to_string(), TypeTerm::Logical),
                ]),
            ),
        ])),
        "stats::nls.control" => Some(TypeTerm::NamedList(vec![
            ("maxiter".to_string(), TypeTerm::Double),
            ("tol".to_string(), TypeTerm::Double),
            ("minFactor".to_string(), TypeTerm::Double),
            ("printEval".to_string(), TypeTerm::Logical),
            ("warnOnly".to_string(), TypeTerm::Logical),
            ("scaleOffset".to_string(), TypeTerm::Double),
            ("nDcentral".to_string(), TypeTerm::Logical),
        ])),
        "stats::getInitial" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::arima.sim" | "stats::ARMAacf" | "stats::ARMAtoMA" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "stats::tsdiag" => Some(TypeTerm::Null),
        _ => None,
    }
}
