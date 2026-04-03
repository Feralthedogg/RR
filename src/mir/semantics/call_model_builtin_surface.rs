pub(crate) fn builtin_arity(name: &str) -> Option<(usize, Option<usize>)> {
    match name {
        "length" | "seq_len" | "seq_along" | "abs" | "sqrt" | "sin" | "cos" | "tan" | "asin"
        | "acos" | "atan" | "sinh" | "cosh" | "tanh" | "log10" | "log2" | "exp" | "sign"
        | "gamma" | "lgamma" | "floor" | "ceiling" | "trunc" | "colSums" | "rowSums" | "is.na"
        | "is.finite" | "dim" | "dimnames" | "nrow" | "ncol" | "t" => Some((1, Some(1))),
        "atan2" => Some((2, Some(2))),
        "round" | "log" => Some((1, Some(2))),
        "pmax" | "pmin" => Some((2, None)),
        "sum" | "mean" | "var" | "sd" | "min" | "max" | "print" | "c" | "list" => Some((1, None)),
        "paste" | "paste0" | "cat" => Some((0, None)),
        "sprintf" => Some((1, None)),
        "names" | "rownames" | "colnames" => Some((1, Some(1))),
        "sort" | "unique" | "duplicated" | "anyDuplicated" => Some((1, None)),
        "match" => Some((2, Some(4))),
        "order" => Some((0, None)),
        "prod" | "any" | "all" => Some((0, None)),
        "which" => Some((1, Some(3))),
        "numeric" => Some((1, Some(1))),
        "character" | "logical" | "integer" | "double" => Some((0, Some(1))),
        "rep" => Some((1, None)),
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
