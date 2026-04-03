use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_stats_smoothing_package_call(callee: &str) -> Option<TypeState> {
    match callee {
        "stats::poly" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::qqline"
        | "stats::interaction.plot"
        | "stats::lag.plot"
        | "stats::monthplot"
        | "stats::scatter.smooth"
        | "stats::biplot"
        | "stats::plot.spec.coherency"
        | "stats::plot.spec.phase" => Some(TypeState::null()),
        "stats::IQR" | "stats::mad" | "stats::bw.bcv" | "stats::bw.nrd" | "stats::bw.nrd0"
        | "stats::bw.SJ" | "stats::bw.ucv" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::qqnorm"
        | "stats::qqplot"
        | "stats::density"
        | "stats::density.default"
        | "stats::prcomp"
        | "stats::princomp"
        | "stats::cancor"
        | "stats::power.anova.test"
        | "stats::power.prop.test"
        | "stats::power.t.test"
        | "stats::decompose"
        | "stats::spec.pgram"
        | "stats::spectrum"
        | "stats::stl"
        | "stats::approx"
        | "stats::ksmooth"
        | "stats::lowess"
        | "stats::loess"
        | "stats::loess.control"
        | "stats::loess.smooth"
        | "stats::spline"
        | "stats::smooth.spline"
        | "stats::supsmu" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::approxfun" | "stats::splinefun" => Some(TypeState::scalar(PrimTy::Any, false)),
        "stats::ecdf" => Some(TypeState::scalar(PrimTy::Any, false)),
        _ => None,
    }
}

pub(crate) fn infer_stats_smoothing_package_call_term(callee: &str) -> Option<TypeTerm> {
    match callee {
        "stats::poly" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::qqline"
        | "stats::interaction.plot"
        | "stats::lag.plot"
        | "stats::monthplot"
        | "stats::scatter.smooth"
        | "stats::biplot" => Some(TypeTerm::Null),
        "stats::IQR" | "stats::mad" | "stats::bw.bcv" | "stats::bw.nrd" | "stats::bw.nrd0"
        | "stats::bw.SJ" | "stats::bw.ucv" => Some(TypeTerm::Double),
        "stats::qqnorm" | "stats::qqplot" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::density" | "stats::density.default" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("bw".to_string(), TypeTerm::Double),
            ("n".to_string(), TypeTerm::Int),
            ("old.coords".to_string(), TypeTerm::Logical),
            ("call".to_string(), TypeTerm::Any),
            ("data.name".to_string(), TypeTerm::Char),
            ("has.na".to_string(), TypeTerm::Logical),
        ])),
        "stats::prcomp" => Some(TypeTerm::NamedList(vec![
            (
                "sdev".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "rotation".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("center".to_string(), TypeTerm::Any),
            ("scale".to_string(), TypeTerm::Any),
            (
                "x".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::princomp" => Some(TypeTerm::NamedList(vec![
            (
                "sdev".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "loadings".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "center".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "scale".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("n.obs".to_string(), TypeTerm::Int),
            (
                "scores".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::cancor" => Some(TypeTerm::NamedList(vec![
            (
                "cor".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "xcoef".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "ycoef".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "xcenter".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "ycenter".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::power.anova.test" => Some(TypeTerm::NamedList(vec![
            ("groups".to_string(), TypeTerm::Int),
            ("n".to_string(), TypeTerm::Double),
            ("between.var".to_string(), TypeTerm::Double),
            ("within.var".to_string(), TypeTerm::Double),
            ("sig.level".to_string(), TypeTerm::Double),
            ("power".to_string(), TypeTerm::Double),
            ("note".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
        ])),
        "stats::power.prop.test" => Some(TypeTerm::NamedList(vec![
            ("n".to_string(), TypeTerm::Double),
            ("p1".to_string(), TypeTerm::Double),
            ("p2".to_string(), TypeTerm::Double),
            ("sig.level".to_string(), TypeTerm::Double),
            ("power".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("note".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
        ])),
        "stats::power.t.test" => Some(TypeTerm::NamedList(vec![
            ("n".to_string(), TypeTerm::Double),
            ("delta".to_string(), TypeTerm::Double),
            ("sd".to_string(), TypeTerm::Double),
            ("sig.level".to_string(), TypeTerm::Double),
            ("power".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("note".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
        ])),
        "stats::approx"
        | "stats::ksmooth"
        | "stats::lowess"
        | "stats::loess.smooth"
        | "stats::spline"
        | "stats::supsmu" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::loess" => Some(TypeTerm::NamedList(vec![
            ("n".to_string(), TypeTerm::Int),
            (
                "fitted".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("enp".to_string(), TypeTerm::Double),
            ("s".to_string(), TypeTerm::Double),
            ("one.delta".to_string(), TypeTerm::Double),
            ("two.delta".to_string(), TypeTerm::Double),
            ("trace.hat".to_string(), TypeTerm::Double),
            ("divisor".to_string(), TypeTerm::Double),
            (
                "xnames".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "weights".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
        ])),
        "stats::loess.control" => Some(TypeTerm::NamedList(vec![
            ("surface".to_string(), TypeTerm::Char),
            ("statistics".to_string(), TypeTerm::Char),
            ("trace.hat".to_string(), TypeTerm::Char),
            ("cell".to_string(), TypeTerm::Double),
            ("iterations".to_string(), TypeTerm::Int),
            ("iterTrace".to_string(), TypeTerm::Logical),
        ])),
        "stats::smooth.spline" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "w".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "yin".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("df".to_string(), TypeTerm::Double),
            ("lambda".to_string(), TypeTerm::Double),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::decompose" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "seasonal".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "trend".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "random".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "figure".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("type".to_string(), TypeTerm::Char),
        ])),
        "stats::spec.pgram" => Some(TypeTerm::NamedList(vec![
            (
                "freq".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "spec".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("coh".to_string(), TypeTerm::Any),
            ("phase".to_string(), TypeTerm::Any),
            ("kernel".to_string(), TypeTerm::Any),
            ("df".to_string(), TypeTerm::Double),
            ("bandwidth".to_string(), TypeTerm::Double),
            ("n.used".to_string(), TypeTerm::Int),
            ("orig.n".to_string(), TypeTerm::Int),
            ("series".to_string(), TypeTerm::Char),
            ("snames".to_string(), TypeTerm::Any),
            ("method".to_string(), TypeTerm::Char),
            ("taper".to_string(), TypeTerm::Double),
            ("pad".to_string(), TypeTerm::Int),
            ("detrend".to_string(), TypeTerm::Logical),
            ("demean".to_string(), TypeTerm::Logical),
        ])),
        "stats::spectrum" => Some(TypeTerm::NamedList(vec![
            (
                "freq".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "spec".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("df".to_string(), TypeTerm::Double),
            ("bandwidth".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("series".to_string(), TypeTerm::Char),
            ("snames".to_string(), TypeTerm::Any),
        ])),
        "stats::plot.spec.coherency" | "stats::plot.spec.phase" => Some(TypeTerm::Null),
        "stats::stl" => Some(TypeTerm::NamedList(vec![
            (
                "time.series".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "weights".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("call".to_string(), TypeTerm::Any),
            (
                "win".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("deg".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            (
                "jump".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("inner".to_string(), TypeTerm::Int),
            ("outer".to_string(), TypeTerm::Int),
        ])),
        "stats::approxfun" | "stats::splinefun" => Some(TypeTerm::Any),
        "stats::ecdf" => Some(TypeTerm::Any),
        _ => None,
    }
}
