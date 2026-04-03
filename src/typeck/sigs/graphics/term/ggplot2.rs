use crate::typeck::term::TypeTerm;

pub(super) fn infer_ggplot2_term(callee: &str, _arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    match callee {
        "ggplot2::ggsave" => Some(TypeTerm::Char),
        "ggplot2::aes"
        | "ggplot2::ggplot"
        | "ggplot2::geom_col"
        | "ggplot2::geom_bar"
        | "ggplot2::facet_grid"
        | "ggplot2::geom_line"
        | "ggplot2::geom_point"
        | "ggplot2::ggtitle"
        | "ggplot2::facet_wrap"
        | "ggplot2::labs"
        | "ggplot2::theme_bw"
        | "ggplot2::theme_minimal" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        callee if callee.starts_with("ggplot2::") => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        _ => None,
    }
}
