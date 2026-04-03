use crate::typeck::lattice::{PrimTy, TypeState};

pub(super) fn infer_ggplot2_state(callee: &str, _arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
        "ggplot2::ggsave" => Some(TypeState::scalar(PrimTy::Char, false)),
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
        | "ggplot2::theme_minimal" => Some(TypeState::vector(PrimTy::Any, false)),
        callee if callee.starts_with("ggplot2::") => Some(TypeState::vector(PrimTy::Any, false)),
        _ => None,
    }
}
