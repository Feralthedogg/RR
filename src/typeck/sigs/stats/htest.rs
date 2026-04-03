use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_stats_htest_package_call(callee: &str) -> Option<TypeState> {
    match callee {
        "stats::PP.test"
        | "stats::t.test"
        | "stats::wilcox.test"
        | "stats::binom.test"
        | "stats::prop.test"
        | "stats::poisson.test"
        | "stats::chisq.test"
        | "stats::fisher.test"
        | "stats::cor.test"
        | "stats::ks.test"
        | "stats::shapiro.test"
        | "stats::ansari.test"
        | "stats::bartlett.test"
        | "stats::Box.test"
        | "stats::fligner.test"
        | "stats::friedman.test"
        | "stats::kruskal.test"
        | "stats::mauchly.test"
        | "stats::mantelhaen.test"
        | "stats::mcnemar.test"
        | "stats::mood.test"
        | "stats::oneway.test"
        | "stats::prop.trend.test"
        | "stats::quade.test"
        | "stats::var.test"
        | "stats::pairwise.t.test"
        | "stats::pairwise.wilcox.test"
        | "stats::pairwise.prop.test" => Some(TypeState::vector(PrimTy::Any, false)),
        _ => None,
    }
}

pub(crate) fn infer_stats_htest_package_call_term(callee: &str) -> Option<TypeTerm> {
    match callee {
        "stats::t.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "estimate".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("null.value".to_string(), TypeTerm::Double),
            ("stderr".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::wilcox.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Any),
            ("p.value".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::binom.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("estimate".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::prop.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "estimate".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("null.value".to_string(), TypeTerm::Any),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::poisson.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "estimate".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::chisq.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
            (
                "observed".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "expected".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "stdres".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::fisher.test" => Some(TypeTerm::NamedList(vec![
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("estimate".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::cor.test" => Some(TypeTerm::NamedList(vec![
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
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::ks.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
            ("exact".to_string(), TypeTerm::Logical),
        ])),
        "stats::shapiro.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::ansari.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::bartlett.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("data.name".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
        ])),
        "stats::mauchly.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::PP.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::Box.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::fligner.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::friedman.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::kruskal.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Int),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::mantelhaen.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("estimate".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::mcnemar.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::mood.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::oneway.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            (
                "parameter".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::prop.trend.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            ("parameter".to_string(), TypeTerm::Double),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::quade.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            (
                "parameter".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
            ("p.value".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::var.test" => Some(TypeTerm::NamedList(vec![
            ("statistic".to_string(), TypeTerm::Double),
            (
                "parameter".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("p.value".to_string(), TypeTerm::Double),
            (
                "conf.int".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("estimate".to_string(), TypeTerm::Double),
            ("null.value".to_string(), TypeTerm::Double),
            ("alternative".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("data.name".to_string(), TypeTerm::Char),
        ])),
        "stats::pairwise.t.test" | "stats::pairwise.wilcox.test" | "stats::pairwise.prop.test" => {
            Some(TypeTerm::NamedList(vec![
                ("method".to_string(), TypeTerm::Char),
                ("data.name".to_string(), TypeTerm::Char),
                (
                    "p.value".to_string(),
                    TypeTerm::Matrix(Box::new(TypeTerm::Double)),
                ),
                ("p.adjust.method".to_string(), TypeTerm::Char),
            ]))
        }
        _ => None,
    }
}
