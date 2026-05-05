use super::*;

pub(crate) fn infer_builtin(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
        "length" | "seq_len" | "nrow" | "ncol" => Some(TypeState::scalar(PrimTy::Int, true)),
        "seq" => {
            let prim = if arg_tys.iter().any(|t| t.prim == PrimTy::Double) {
                PrimTy::Double
            } else {
                PrimTy::Int
            };
            Some(TypeState::vector(prim, false))
        }
        "seq_along" => {
            Some(TypeState::vector(PrimTy::Int, true).with_len(shared_vector_len_sym(arg_tys)))
        }
        "names" | "rownames" | "colnames" => Some(TypeState::vector(PrimTy::Char, false)),
        "order" => Some(TypeState::vector(PrimTy::Int, false)),
        "any" | "all" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "cat" => Some(TypeState::null()),
        "which" => {
            let first = first_arg_type_state(arg_tys);
            if matches!(first.shape, ShapeTy::Scalar) {
                Some(TypeState::scalar(PrimTy::Int, true))
            } else {
                Some(TypeState::vector(PrimTy::Int, true).with_len(first.len_sym))
            }
        }
        "which.min" | "which.max" => Some(TypeState::scalar(PrimTy::Int, true)),
        "isTRUE" | "isFALSE" => Some(TypeState::scalar(PrimTy::Logical, true)),
        "lengths" => {
            let first = first_arg_type_state(arg_tys);
            Some(TypeState::vector(PrimTy::Int, false).with_len(first.len_sym))
        }
        "sample" => Some(sample_output_type(first_arg_type_state(arg_tys))),
        "sample.int" => Some(TypeState::vector(PrimTy::Int, false)),
        "rank" => Some(rank_output_type(first_arg_type_state(arg_tys))),
        "aggregate" => Some(TypeState::matrix(PrimTy::Any, false)),
        "ave" => Some(vectorized_first_arg_type(first_arg_type_state(arg_tys))),
        "reorder" | "relevel" => Some(TypeState::vector(PrimTy::Int, false)),
        "factor" | "cut" => {
            let first = first_arg_type_state(arg_tys);
            Some(TypeState::vector(PrimTy::Int, false).with_len(first.len_sym))
        }
        "table" => Some(TypeState::vector(PrimTy::Int, false)),
        "ifelse" => ifelse_output_type(arg_tys),
        "ts" | "window" | "lag" => Some(ts_like_output_type(first_arg_type_state(arg_tys))),
        "frequency" => Some(TypeState::scalar(PrimTy::Double, false)),
        "time" | "cycle" => Some(TypeState::vector(PrimTy::Double, false)),
        "embed" => Some(TypeState::matrix(first_numeric_prim(arg_tys), false)),
        "trimws" => Some(char_like_first_arg_type(first_arg_type_state(arg_tys))),
        "chartr" => Some(char_like_first_arg_type(
            arg_tys
                .get(2)
                .copied()
                .unwrap_or(first_arg_type_state(arg_tys)),
        )),
        "regexpr" | "agrep" => Some(int_like_first_arg_type(
            arg_tys
                .get(1)
                .copied()
                .unwrap_or(first_arg_type_state(arg_tys)),
        )),
        "agrepl" => Some(logical_like_first_arg_type(
            arg_tys
                .get(1)
                .copied()
                .unwrap_or(first_arg_type_state(arg_tys)),
        )),
        "gregexpr" | "regexec" | "strsplit" => Some(TypeState::vector(PrimTy::Any, false)),
        "paste" | "paste0" | "sprintf" => {
            if arg_tys.is_empty() || any_vector_shape(arg_tys) {
                Some(
                    TypeState::vector(PrimTy::Char, false).with_len(shared_vector_len_sym(arg_tys)),
                )
            } else {
                Some(TypeState::scalar(PrimTy::Char, false))
            }
        }
        "tolower" | "toupper" | "substr" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "sub" | "gsub" => Some(char_like_first_arg_type(
            arg_tys
                .get(2)
                .copied()
                .unwrap_or(first_arg_type_state(arg_tys)),
        )),
        "nchar" => Some(int_like_first_arg_type(first_arg_type_state(arg_tys))),
        "nzchar" | "grepl" | "startsWith" | "endsWith" => {
            Some(logical_like_first_arg_type(if matches!(callee, "grepl") {
                arg_tys
                    .get(1)
                    .copied()
                    .unwrap_or(first_arg_type_state(arg_tys))
            } else {
                first_arg_type_state(arg_tys)
            }))
        }
        "grep" => Some(TypeState::vector(PrimTy::Int, false)),
        "union" | "intersect" | "setdiff" => {
            Some(TypeState::vector(joined_general_prim(arg_tys), false))
        }
        "sort" | "unique" => Some(vectorized_first_arg_type(first_arg_type_state(arg_tys))),
        "duplicated" => {
            let first = first_arg_type_state(arg_tys);
            if matches!(first.shape, ShapeTy::Scalar) {
                Some(TypeState::scalar(PrimTy::Logical, false))
            } else {
                Some(TypeState::vector(PrimTy::Logical, false).with_len(first.len_sym))
            }
        }
        "match" => {
            let first = first_arg_type_state(arg_tys);
            if matches!(first.shape, ShapeTy::Scalar) {
                Some(TypeState::scalar(PrimTy::Int, false))
            } else {
                Some(TypeState::vector(PrimTy::Int, false).with_len(first.len_sym))
            }
        }
        "anyDuplicated" => Some(TypeState::scalar(PrimTy::Int, false)),
        "dim" => Some(TypeState::vector(PrimTy::Int, true)),
        "dimnames" => Some(TypeState::vector(PrimTy::Any, false)),
        "rr_i0" | "rr_i1" | "rr_index1_read_idx" => Some(TypeState::scalar(PrimTy::Int, false)),
        "rr_index_vec_floor" => {
            if arg_tys.iter().any(|t| t.shape == ShapeTy::Vector) {
                Some(TypeState::vector(PrimTy::Int, false).with_len(shared_vector_len_sym(arg_tys)))
            } else {
                Some(TypeState::scalar(PrimTy::Int, false))
            }
        }
        "c" => {
            let all_non_na = arg_tys.iter().all(|t| t.na == crate::typeck::NaTy::Never);
            let mut out = TypeState::vector(PrimTy::Any, all_non_na);
            for t in arg_tys {
                let promoted = TypeState::vector(t.prim, t.na == crate::typeck::NaTy::Never);
                out = out.join(promoted);
            }
            if arg_tys.len() == 1 {
                out = out.with_len(shared_vector_len_sym(arg_tys));
            }
            Some(out)
        }
        "abs" | "pmax" | "pmin" => numeric_shape_preserving_output_type(arg_tys),
        "min" | "max" | "sum" => scalar_numeric_reduction_output_type(arg_tys),
        "prod" => double_numeric_reduction_output_type(arg_tys, DOUBLE_PRODUCT_REDUCTION),
        "var" | "sd" => double_numeric_reduction_output_type(arg_tys, DOUBLE_VARIANCE_REDUCTION),
        "mean" => double_numeric_reduction_output_type(arg_tys, DOUBLE_MEAN_REDUCTION),
        "sign" => sign_output_type(arg_tys),
        "sqrt" | "log" | "log10" | "log2" | "exp" | "atan" | "atan2" | "asin" | "acos" | "sin"
        | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "gamma" | "lgamma" | "floor" | "ceiling"
        | "trunc" | "round" => vectorized_prim_output_type(arg_tys, PrimTy::Double, false),
        "is.na" | "is.finite" => vectorized_prim_output_type(arg_tys, PrimTy::Logical, true),
        "numeric" | "double" => Some(TypeState::vector(PrimTy::Double, true)),
        "integer" => Some(TypeState::vector(PrimTy::Int, true)),
        "logical" => Some(TypeState::vector(PrimTy::Logical, true)),
        "character" => Some(TypeState::vector(PrimTy::Char, true)),
        "rep" | "rep.int" => {
            let first = arg_tys.first().copied().unwrap_or(TypeState::unknown());
            let prim = match first.shape {
                ShapeTy::Matrix | ShapeTy::Vector | ShapeTy::Scalar => first.prim,
                ShapeTy::Unknown => PrimTy::Any,
            };
            Some(TypeState::vector(
                prim,
                first.na == crate::typeck::NaTy::Never,
            ))
        }
        "matrix" => Some(TypeState::matrix(PrimTy::Double, false)),
        "t" | "rbind" | "cbind" => Some(TypeState::matrix(first_numeric_prim(arg_tys), false)),
        "list" => Some(TypeState::vector(PrimTy::Any, false)),
        "diag" => match arg_tys.first().map(|t| t.shape) {
            Some(ShapeTy::Matrix) => Some(TypeState::vector(first_numeric_prim(arg_tys), false)),
            Some(ShapeTy::Vector) | Some(ShapeTy::Scalar) => {
                Some(TypeState::matrix(first_numeric_prim(arg_tys), false))
            }
            _ => Some(TypeState::matrix(PrimTy::Double, false)),
        },
        "rowSums" | "colSums" => {
            let prim = match first_numeric_prim(arg_tys) {
                PrimTy::Int => PrimTy::Double,
                PrimTy::Double => PrimTy::Double,
                _ => PrimTy::Double,
            };
            Some(TypeState::vector(prim, false))
        }
        "crossprod" | "tcrossprod" => Some(TypeState::matrix(PrimTy::Double, false)),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug)]
struct DoubleNumericReduction {
    allow_empty_input: bool,
    preserves_non_na_inputs: bool,
}

const DOUBLE_PRODUCT_REDUCTION: DoubleNumericReduction = DoubleNumericReduction {
    allow_empty_input: true,
    preserves_non_na_inputs: true,
};

const DOUBLE_VARIANCE_REDUCTION: DoubleNumericReduction = DoubleNumericReduction {
    allow_empty_input: true,
    preserves_non_na_inputs: false,
};

const DOUBLE_MEAN_REDUCTION: DoubleNumericReduction = DoubleNumericReduction {
    allow_empty_input: false,
    preserves_non_na_inputs: false,
};

fn numeric_shape_is_known(arg_tys: &[TypeState]) -> bool {
    any_vector_shape(arg_tys) || all_known_scalar_shape(arg_tys)
}

fn numeric_shape_is_known_or_empty(arg_tys: &[TypeState], allow_empty: bool) -> bool {
    numeric_shape_is_known(arg_tys) || (allow_empty && arg_tys.is_empty())
}

fn checked_promoted_numeric_prim(arg_tys: &[TypeState]) -> Option<PrimTy> {
    let prim = promoted_numeric_prim(arg_tys);
    if prim == PrimTy::Any && !all_known_numeric_prim(arg_tys) {
        None
    } else {
        Some(prim)
    }
}

fn numeric_shape_preserving_output_type(arg_tys: &[TypeState]) -> Option<TypeState> {
    if !numeric_shape_is_known(arg_tys) {
        return None;
    }
    let prim = match checked_promoted_numeric_prim(arg_tys)? {
        PrimTy::Int => PrimTy::Int,
        PrimTy::Double => PrimTy::Double,
        _ => PrimTy::Double,
    };
    vectorized_prim_output_type(arg_tys, prim, false)
}

fn scalar_numeric_reduction_output_type(arg_tys: &[TypeState]) -> Option<TypeState> {
    if !numeric_shape_is_known(arg_tys) {
        return None;
    }
    let prim = match checked_promoted_numeric_prim(arg_tys)? {
        PrimTy::Int => PrimTy::Int,
        _ => PrimTy::Double,
    };
    Some(TypeState::scalar(
        prim,
        arg_tys.iter().all(|t| t.na == crate::typeck::NaTy::Never),
    ))
}

fn double_numeric_reduction_output_type(
    arg_tys: &[TypeState],
    reduction: DoubleNumericReduction,
) -> Option<TypeState> {
    if !numeric_shape_is_known_or_empty(arg_tys, reduction.allow_empty_input) {
        return None;
    }
    Some(TypeState::scalar(
        PrimTy::Double,
        reduction.preserves_non_na_inputs
            && arg_tys.iter().all(|t| t.na == crate::typeck::NaTy::Never),
    ))
}

fn sign_output_type(arg_tys: &[TypeState]) -> Option<TypeState> {
    if !numeric_shape_is_known(arg_tys) {
        return None;
    }
    let prim = match promoted_numeric_prim(arg_tys) {
        PrimTy::Int => PrimTy::Int,
        PrimTy::Double => PrimTy::Double,
        _ => return None,
    };
    vectorized_prim_output_type(arg_tys, prim, false)
}

fn vectorized_prim_output_type(
    arg_tys: &[TypeState],
    prim: PrimTy,
    non_na: bool,
) -> Option<TypeState> {
    if !numeric_shape_is_known(arg_tys) {
        return None;
    }
    if any_vector_shape(arg_tys) {
        Some(TypeState::vector(prim, non_na).with_len(shared_vector_len_sym(arg_tys)))
    } else {
        Some(TypeState::scalar(prim, non_na))
    }
}
