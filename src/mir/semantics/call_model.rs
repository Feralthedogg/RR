use crate::error::{RR, RRCode, RRException, Stage};
use crate::utils::Span;
use rustc_hash::FxHashMap;

pub(super) fn function_name_suggestion_candidates() -> &'static [&'static str] {
    &[
        "length",
        "seq_len",
        "seq_along",
        "abs",
        "sqrt",
        "sin",
        "cos",
        "tan",
        "asin",
        "acos",
        "atan",
        "atan2",
        "sinh",
        "cosh",
        "tanh",
        "log10",
        "log2",
        "exp",
        "sign",
        "gamma",
        "lgamma",
        "floor",
        "ceiling",
        "trunc",
        "colSums",
        "rowSums",
        "is.na",
        "is.finite",
        "round",
        "log",
        "pmax",
        "pmin",
        "sum",
        "mean",
        "var",
        "sd",
        "min",
        "max",
        "prod",
        "print",
        "c",
        "list",
        "numeric",
        "rep.int",
        "vector",
        "matrix",
        "dim",
        "dimnames",
        "nrow",
        "ncol",
        "crossprod",
        "tcrossprod",
        "t",
        "diag",
        "rbind",
        "cbind",
        "eval",
        "parse",
        "get",
        "assign",
        "exists",
        "mget",
        "rm",
        "ls",
        "library",
        "require",
        "parent.frame",
        "environment",
        "sys.frame",
        "sys.call",
        "do.call",
    ]
}

pub(super) fn suggest_function_name(
    callee: &str,
    user_arities: &FxHashMap<String, usize>,
) -> Option<String> {
    super::suggest_name(
        callee,
        user_arities.keys().cloned().chain(
            function_name_suggestion_candidates()
                .iter()
                .map(|name| (*name).to_string()),
        ),
    )
}

pub(super) fn validate_call_target(
    callee: &str,
    argc: usize,
    span: Span,
    user_arities: &FxHashMap<String, usize>,
) -> RR<()> {
    if let Some(expected) = user_arities.get(callee) {
        if *expected != argc {
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Mir,
                format!(
                    "function '{}' expects {} argument(s), got {}",
                    callee, expected, argc
                ),
            )
            .at(span)
            .push_frame("mir::semantics::validate_call_target/4", Some(span)));
        }
        return Ok(());
    }

    if let Some((min, max)) = builtin_arity(callee) {
        if argc < min || max.is_some_and(|m| argc > m) {
            let upper = max
                .map(|m| m.to_string())
                .unwrap_or_else(|| "inf".to_string());
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Mir,
                format!(
                    "builtin '{}' expects {}..{} argument(s), got {}",
                    callee, min, upper, argc
                ),
            )
            .at(span)
            .push_frame("mir::semantics::validate_call_target/4", Some(span)));
        }
        return Ok(());
    }

    if is_dynamic_fallback_builtin(callee)
        || is_namespaced_r_call(callee)
        || is_supported_package_call(callee)
        || is_tidy_helper_call(callee)
        || is_supported_tidy_helper_call(callee)
        || is_runtime_helper(callee)
    {
        return Ok(());
    }

    let mut err = RRException::new(
        "RR.SemanticError",
        RRCode::E1001,
        Stage::Mir,
        format!("undefined function '{}'", callee),
    )
    .at(span)
    .push_frame("mir::semantics::validate_call_target/4", Some(span))
    .note("Define the function before calling it, or import the module that provides it.");
    if let Some(suggestion) = suggest_function_name(callee, user_arities) {
        err = err.help(suggestion);
    }
    Err(err)
}

pub(super) fn builtin_arity(name: &str) -> Option<(usize, Option<usize>)> {
    match name {
        "length" | "seq_len" | "seq_along" | "abs" | "sqrt" | "sin" | "cos" | "tan" | "asin"
        | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "log10" | "log2" | "exp" | "sign"
        | "gamma" | "lgamma" | "floor" | "ceiling" | "trunc" | "colSums" | "rowSums" | "is.na"
        | "is.finite" | "dim" | "dimnames" | "nrow" | "ncol" | "t" => Some((1, Some(1))),
        "atan2" => Some((2, Some(2))),
        "round" | "log" => Some((1, Some(2))),
        "pmax" | "pmin" => Some((2, None)),
        "sum" | "mean" | "var" | "sd" | "min" | "max" | "prod" | "print" | "c" | "list" => {
            Some((1, None))
        }
        "numeric" => Some((1, Some(1))),
        "rep.int" => Some((2, Some(2))),
        "vector" => Some((1, Some(2))),
        "matrix" => Some((1, Some(4))),
        "diag" => Some((1, Some(4))),
        "rbind" | "cbind" => Some((1, None)),
        "crossprod" | "tcrossprod" => Some((1, Some(2))),
        _ => None,
    }
}

pub(crate) fn is_dynamic_fallback_builtin(name: &str) -> bool {
    matches!(
        name,
        "eval"
            | "parse"
            | "get"
            | "assign"
            | "exists"
            | "mget"
            | "rm"
            | "ls"
            | "parent.frame"
            | "environment"
            | "sys.frame"
            | "sys.call"
            | "do.call"
            | "library"
            | "require"
            | "png"
            | "plot"
            | "lines"
            | "legend"
            | "dev.off"
    )
}

pub(crate) fn is_namespaced_r_call(name: &str) -> bool {
    let Some((pkg, sym)) = name.split_once("::") else {
        return false;
    };
    !pkg.is_empty() && !sym.is_empty() && !pkg.contains(':') && !sym.contains(':')
}

pub(crate) fn is_tidy_helper_call(name: &str) -> bool {
    matches!(
        name,
        "starts_with"
            | "ends_with"
            | "contains"
            | "matches"
            | "everything"
            | "all_of"
            | "any_of"
            | "where"
            | "desc"
            | "between"
            | "n"
            | "row_number"
    )
}

pub(crate) fn is_tidy_data_mask_call(name: &str) -> bool {
    matches!(
        name,
        "ggplot2::aes"
            | "dplyr::mutate"
            | "dplyr::filter"
            | "dplyr::select"
            | "dplyr::summarise"
            | "dplyr::arrange"
            | "dplyr::group_by"
            | "dplyr::rename"
            | "tidyr::separate"
            | "tidyr::pivot_longer"
            | "tidyr::pivot_wider"
            | "tidyr::unite"
    )
}

pub(crate) fn is_supported_package_call(name: &str) -> bool {
    matches!(
        name,
        "base::data.frame"
            | "stats::median"
            | "stats::sd"
            | "stats::lm"
            | "stats::predict"
            | "stats::quantile"
            | "stats::glm"
            | "stats::as.formula"
            | "readr::read_csv"
            | "readr::read_delim"
            | "readr::read_rds"
            | "readr::read_tsv"
            | "readr::write_csv"
            | "readr::write_delim"
            | "readr::write_rds"
            | "readr::write_tsv"
            | "tidyr::separate"
            | "tidyr::pivot_longer"
            | "tidyr::pivot_wider"
            | "tidyr::unite"
            | "graphics::plot"
            | "graphics::lines"
            | "graphics::legend"
            | "grDevices::png"
            | "grDevices::dev.off"
            | "ggplot2::aes"
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
            | "ggplot2::theme_minimal"
            | "ggplot2::ggsave"
            | "dplyr::mutate"
            | "dplyr::filter"
            | "dplyr::full_join"
            | "dplyr::inner_join"
            | "dplyr::right_join"
            | "dplyr::select"
            | "dplyr::summarise"
            | "dplyr::arrange"
            | "dplyr::anti_join"
            | "dplyr::bind_rows"
            | "dplyr::group_by"
            | "dplyr::left_join"
            | "dplyr::rename"
            | "dplyr::semi_join"
    )
}

pub(crate) fn is_supported_tidy_helper_call(name: &str) -> bool {
    is_tidy_helper_call(name)
}

pub(super) fn is_runtime_helper(name: &str) -> bool {
    name.starts_with("rr_")
}

pub(super) fn is_runtime_reserved_symbol(name: &str) -> bool {
    name.starts_with(".phi_")
        || name.starts_with(".tachyon_")
        || name.starts_with("Sym_")
        || name.starts_with("__lambda_")
        || name.starts_with("rr_")
}
