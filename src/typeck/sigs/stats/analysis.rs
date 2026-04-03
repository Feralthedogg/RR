use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_stats_analysis_package_call(callee: &str) -> Option<TypeState> {
    match callee {
        "stats::cmdscale" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::pairwise.table" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::symnum" => Some(TypeState::matrix(PrimTy::Char, false)),
        "stats::add1" | "stats::drop1" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::extractAIC" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::add.scope" | "stats::drop.scope" => Some(TypeState::vector(PrimTy::Char, false)),
        "stats::medpolish" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::ls.print" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::dendrapply" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::order.dendrogram" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::as.dist" | "stats::cophenetic" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::as.hclust" | "stats::as.dendrogram" | "stats::rect.hclust" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats::summary.manova" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::cutree" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::aov"
        | "stats::manova"
        | "stats::alias"
        | "stats::model.tables"
        | "stats::factanal"
        | "stats::heatmap"
        | "stats::loglin"
        | "stats::kmeans"
        | "stats::hclust"
        | "stats::acf"
        | "stats::pacf"
        | "stats::ccf"
        | "stats::termplot"
        | "stats::factor.scope"
        | "stats::dummy.coef"
        | "stats::dummy.coef.lm"
        | "stats::TukeyHSD" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::effects" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::proj" => Some(TypeState::matrix(PrimTy::Double, false)),
        _ => None,
    }
}

pub(crate) fn infer_stats_analysis_package_call_term(callee: &str) -> Option<TypeTerm> {
    match callee {
        "stats::cmdscale" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::pairwise.table" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::symnum" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "stats::aov" => Some(TypeTerm::NamedList(vec![
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "effects".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("rank".to_string(), TypeTerm::Int),
            (
                "fitted.values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "assign".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("df.residual".to_string(), TypeTerm::Int),
            ("contrasts".to_string(), TypeTerm::Any),
            ("xlevels".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
            ("model".to_string(), TypeTerm::Any),
            ("qr".to_string(), TypeTerm::Any),
        ])),
        "stats::manova" => Some(TypeTerm::NamedList(vec![
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "effects".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("rank".to_string(), TypeTerm::Int),
            (
                "fitted.values".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "assign".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("qr".to_string(), TypeTerm::Any),
            ("df.residual".to_string(), TypeTerm::Int),
            ("contrasts".to_string(), TypeTerm::Any),
            ("xlevels".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
            ("model".to_string(), TypeTerm::Any),
        ])),
        "stats::TukeyHSD" => Some(TypeTerm::List(Box::new(TypeTerm::Matrix(Box::new(
            TypeTerm::Double,
        ))))),
        "stats::proj" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::loglin" => Some(TypeTerm::NamedList(vec![
            ("lrt".to_string(), TypeTerm::Double),
            ("pearson".to_string(), TypeTerm::Double),
            ("df".to_string(), TypeTerm::Double),
            (
                "margin".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Int)),
            ),
            (
                "fit".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("param".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
        ])),
        "stats::alias" => Some(TypeTerm::NamedList(vec![(
            "Model".to_string(),
            TypeTerm::Any,
        )])),
        "stats::add1" | "stats::drop1" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::extractAIC" => Some(TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(2))),
        "stats::add.scope" | "stats::drop.scope" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "stats::medpolish" => Some(TypeTerm::NamedList(vec![
            ("overall".to_string(), TypeTerm::Double),
            (
                "row".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "col".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("name".to_string(), TypeTerm::Char),
        ])),
        "stats::ls.print" => Some(TypeTerm::NamedList(vec![
            (
                "summary".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Char)),
            ),
            (
                "coef.table".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Matrix(Box::new(TypeTerm::Double)))),
            ),
        ])),
        "stats::termplot" => Some(TypeTerm::List(Box::new(TypeTerm::DataFrameNamed(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])))),
        "stats::factor.scope" => Some(TypeTerm::NamedList(vec![
            (
                "drop".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "add".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "stats::dummy.coef" | "stats::dummy.coef.lm" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Double)))
        }
        "stats::effects" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::model.tables" => Some(TypeTerm::NamedList(vec![
            (
                "tables".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any)),
            ),
            ("n".to_string(), TypeTerm::Int),
        ])),
        "stats::factanal" => Some(TypeTerm::NamedList(vec![
            ("converged".to_string(), TypeTerm::Logical),
            (
                "loadings".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "uniquenesses".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "correlation".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "criteria".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("factors".to_string(), TypeTerm::Double),
            ("dof".to_string(), TypeTerm::Double),
            ("method".to_string(), TypeTerm::Char),
            ("STATISTIC".to_string(), TypeTerm::Double),
            ("PVAL".to_string(), TypeTerm::Double),
            ("n.obs".to_string(), TypeTerm::Int),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::heatmap" => Some(TypeTerm::NamedList(vec![
            (
                "rowInd".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "colInd".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("Rowv".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
            ("Colv".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
        ])),
        "stats::kmeans" => Some(TypeTerm::NamedList(vec![
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
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("iter".to_string(), TypeTerm::Int),
            ("ifault".to_string(), TypeTerm::Int),
        ])),
        "stats::hclust" => Some(TypeTerm::NamedList(vec![
            (
                "merge".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Int)),
            ),
            (
                "height".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "order".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("labels".to_string(), TypeTerm::Any),
            ("method".to_string(), TypeTerm::Char),
            ("call".to_string(), TypeTerm::Any),
            ("dist.method".to_string(), TypeTerm::Char),
        ])),
        "stats::acf" | "stats::pacf" | "stats::ccf" => Some(TypeTerm::NamedList(vec![
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
        ])),
        "stats::dendrapply" => Some(TypeTerm::Any),
        "stats::order.dendrogram" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "stats::as.dist" | "stats::cophenetic" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "stats::as.hclust" => Some(TypeTerm::NamedList(vec![
            (
                "merge".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Int)),
            ),
            (
                "height".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "order".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("labels".to_string(), TypeTerm::Any),
            ("method".to_string(), TypeTerm::Char),
            ("call".to_string(), TypeTerm::Any),
            ("dist.method".to_string(), TypeTerm::Char),
        ])),
        "stats::as.dendrogram" => Some(TypeTerm::Any),
        "stats::rect.hclust" => Some(TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(
            TypeTerm::Int,
        ))))),
        "stats::summary.manova" => Some(TypeTerm::NamedList(vec![
            (
                "row.names".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "SS".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Matrix(Box::new(TypeTerm::Double)))),
            ),
            (
                "Eigenvalues".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "stats".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::cutree" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        _ => None,
    }
}
