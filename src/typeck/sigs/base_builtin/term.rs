use super::*;

pub(crate) fn infer_builtin_term(callee: &str, arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    match callee {
        "length" | "nrow" | "ncol" => Some(TypeTerm::Int),
        "seq" => {
            let prim = if arg_terms
                .iter()
                .any(|t| matches!(shallow_elem_term(t), TypeTerm::Double))
            {
                TypeTerm::Double
            } else {
                TypeTerm::Int
            };
            Some(TypeTerm::Vector(Box::new(prim)))
        }
        "seq_len" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "seq_along" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "names" => match first_arg_term(arg_terms) {
            TypeTerm::DataFrame(_) | TypeTerm::DataFrameNamed(_) => Some(TypeTerm::VectorLen(
                Box::new(TypeTerm::Char),
                dataframe_col_count(&first_arg_term(arg_terms)),
            )),
            _ => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        },
        "rownames" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "colnames" => match first_arg_term(arg_terms) {
            TypeTerm::DataFrame(_) | TypeTerm::DataFrameNamed(_) => Some(TypeTerm::VectorLen(
                Box::new(TypeTerm::Char),
                dataframe_col_count(&first_arg_term(arg_terms)),
            )),
            _ => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        },
        "order" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "any" | "all" => Some(TypeTerm::Logical),
        "cat" => Some(TypeTerm::Null),
        "which" => match first_arg_term(arg_terms) {
            TypeTerm::Vector(_)
            | TypeTerm::VectorLen(_, _)
            | TypeTerm::Matrix(_)
            | TypeTerm::MatrixDim(_, _, _)
            | TypeTerm::ArrayDim(_, _) => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
            _ => Some(TypeTerm::Int),
        },
        "which.min" | "which.max" => Some(TypeTerm::Int),
        "isTRUE" | "isFALSE" => Some(TypeTerm::Logical),
        "lengths" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "sample" => Some(sample_output_term(first_arg_term(arg_terms))),
        "sample.int" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "rank" => Some(rank_output_term(first_arg_term(arg_terms))),
        "aggregate" => Some(TypeTerm::DataFrame(Vec::new())),
        "ave" => Some(vectorized_first_arg_term(first_arg_term(arg_terms))),
        "reorder" | "relevel" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "factor" | "cut" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "table" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "ifelse" => ifelse_output_term(arg_terms),
        "ts" | "window" | "lag" => Some(ts_like_output_term(first_arg_term(arg_terms))),
        "frequency" => Some(TypeTerm::Double),
        "time" | "cycle" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "embed" => Some(TypeTerm::Matrix(Box::new(first_numeric_term(arg_terms)))),
        "trimws" => Some(char_like_first_arg_term(first_arg_term(arg_terms))),
        "chartr" => Some(char_like_first_arg_term(
            arg_terms
                .get(2)
                .cloned()
                .unwrap_or_else(|| first_arg_term(arg_terms)),
        )),
        "regexpr" | "agrep" => Some(int_like_first_arg_term(
            arg_terms
                .get(1)
                .cloned()
                .unwrap_or_else(|| first_arg_term(arg_terms)),
        )),
        "agrepl" => Some(logical_like_first_arg_term(
            arg_terms
                .get(1)
                .cloned()
                .unwrap_or_else(|| first_arg_term(arg_terms)),
        )),
        "gregexpr" | "regexec" => Some(TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(
            TypeTerm::Int,
        ))))),
        "strsplit" => Some(TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(
            TypeTerm::Char,
        ))))),
        "paste" | "paste0" | "sprintf" => {
            if arg_terms.is_empty() || any_vector_term(arg_terms) {
                Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
            } else {
                Some(TypeTerm::Char)
            }
        }
        "tolower" | "toupper" | "substr" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "sub" | "gsub" => Some(char_like_first_arg_term(
            arg_terms
                .get(2)
                .cloned()
                .unwrap_or_else(|| first_arg_term(arg_terms)),
        )),
        "nchar" => Some(int_like_first_arg_term(first_arg_term(arg_terms))),
        "nzchar" | "grepl" | "startsWith" | "endsWith" => {
            Some(logical_like_first_arg_term(if matches!(callee, "grepl") {
                arg_terms
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| first_arg_term(arg_terms))
            } else {
                first_arg_term(arg_terms)
            }))
        }
        "grep" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "union" | "intersect" | "setdiff" => {
            Some(TypeTerm::Vector(Box::new(joined_general_term(arg_terms))))
        }
        "sort" | "unique" => Some(vectorized_first_arg_term(first_arg_term(arg_terms))),
        "duplicated" => match first_arg_term(arg_terms) {
            TypeTerm::Vector(_)
            | TypeTerm::VectorLen(_, _)
            | TypeTerm::Matrix(_)
            | TypeTerm::MatrixDim(_, _, _)
            | TypeTerm::ArrayDim(_, _) => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
            _ => Some(TypeTerm::Logical),
        },
        "match" => match first_arg_term(arg_terms) {
            TypeTerm::Vector(_)
            | TypeTerm::VectorLen(_, _)
            | TypeTerm::Matrix(_)
            | TypeTerm::MatrixDim(_, _, _)
            | TypeTerm::ArrayDim(_, _) => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
            _ => Some(TypeTerm::Int),
        },
        "anyDuplicated" => Some(TypeTerm::Int),
        "dim" => match first_arg_term(arg_terms) {
            TypeTerm::MatrixDim(_, _, _) => {
                Some(TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(2)))
            }
            TypeTerm::ArrayDim(_, dims) => Some(TypeTerm::VectorLen(
                Box::new(TypeTerm::Int),
                Some(dims.len() as i64),
            )),
            TypeTerm::DataFrame(_) | TypeTerm::DataFrameNamed(_) => {
                Some(TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(2)))
            }
            _ => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        },
        "dimnames" => Some(TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(
            TypeTerm::Char,
        ))))),
        "rr_i0" | "rr_i1" | "rr_index1_read_idx" => Some(TypeTerm::Int),
        "rr_index_vec_floor" => {
            if arg_terms.iter().any(|t| {
                matches!(
                    t,
                    TypeTerm::Vector(_)
                        | TypeTerm::VectorLen(_, _)
                        | TypeTerm::Matrix(_)
                        | TypeTerm::MatrixDim(_, _, _)
                        | TypeTerm::ArrayDim(_, _)
                )
            }) {
                Some(TypeTerm::Vector(Box::new(TypeTerm::Int)))
            } else {
                Some(TypeTerm::Int)
            }
        }
        "c" => {
            let mut elem = TypeTerm::Any;
            for t in arg_terms {
                let promoted = match t {
                    TypeTerm::Vector(inner)
                    | TypeTerm::VectorLen(inner, _)
                    | TypeTerm::Matrix(inner)
                    | TypeTerm::MatrixDim(inner, _, _)
                    | TypeTerm::ArrayDim(inner, _) => inner.as_ref().clone(),
                    _ => t.clone(),
                };
                elem = elem.join(&promoted);
            }
            Some(TypeTerm::Vector(Box::new(elem)))
        }
        "list" => {
            let mut elem = TypeTerm::Any;
            for t in arg_terms {
                elem = elem.join(t);
            }
            Some(TypeTerm::List(Box::new(elem)))
        }
        "box" => {
            let inner = arg_terms.first().cloned().unwrap_or(TypeTerm::Any);
            Some(TypeTerm::Boxed(Box::new(inner)))
        }
        "unbox" => {
            let inner = arg_terms
                .first()
                .map(TypeTerm::unbox)
                .unwrap_or(TypeTerm::Any);
            Some(inner)
        }
        "abs" | "pmax" | "pmin" => {
            let prim = match promoted_numeric_term(arg_terms) {
                TypeTerm::Int => TypeTerm::Int,
                TypeTerm::Double => TypeTerm::Double,
                _ => return None,
            };
            if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
                return None;
            }
            if any_vector_term(arg_terms) {
                Some(TypeTerm::Vector(Box::new(prim)))
            } else {
                Some(prim)
            }
        }
        "min" | "max" => {
            if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
                return None;
            }
            Some(match promoted_numeric_term(arg_terms) {
                TypeTerm::Int => TypeTerm::Int,
                TypeTerm::Double => TypeTerm::Double,
                _ => return None,
            })
        }
        "sum" => {
            if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
                return None;
            }
            Some(match promoted_numeric_term(arg_terms) {
                TypeTerm::Int => TypeTerm::Int,
                TypeTerm::Double => TypeTerm::Double,
                _ => return None,
            })
        }
        "prod" | "var" | "sd" => Some(TypeTerm::Double),
        "mean" => {
            if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
                return None;
            }
            Some(TypeTerm::Double)
        }
        "sign" => {
            if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
                return None;
            }
            let prim = match promoted_numeric_term(arg_terms) {
                TypeTerm::Int => TypeTerm::Int,
                TypeTerm::Double => TypeTerm::Double,
                _ => return None,
            };
            if any_vector_term(arg_terms) {
                Some(TypeTerm::Vector(Box::new(prim)))
            } else {
                Some(prim)
            }
        }
        "sqrt" | "log" | "log10" | "log2" | "exp" | "atan" | "atan2" | "asin" | "acos" | "sin"
        | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "gamma" | "lgamma" | "floor" | "ceiling"
        | "trunc" | "round" => {
            if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
                return None;
            }
            if any_vector_term(arg_terms) {
                Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
            } else {
                Some(TypeTerm::Double)
            }
        }
        "is.na" | "is.finite" => {
            if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
                return None;
            }
            if any_vector_term(arg_terms) {
                Some(TypeTerm::Vector(Box::new(TypeTerm::Logical)))
            } else {
                Some(TypeTerm::Logical)
            }
        }
        "numeric" | "double" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "integer" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "logical" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "character" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "rep" | "rep.int" => {
            let elem = match arg_terms.first() {
                Some(
                    TypeTerm::Vector(inner)
                    | TypeTerm::VectorLen(inner, _)
                    | TypeTerm::Matrix(inner)
                    | TypeTerm::MatrixDim(inner, _, _)
                    | TypeTerm::ArrayDim(inner, _),
                ) => inner.as_ref().clone(),
                Some(term) => term.clone(),
                None => TypeTerm::Any,
            };
            Some(TypeTerm::Vector(Box::new(elem)))
        }
        "matrix" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "t" => {
            let (elem, rows, cols) = matrix_term_parts(arg_terms.first()?)?;
            Some(matrix_term_with_dims(elem, cols, rows))
        }
        "diag" => {
            let first = arg_terms.first()?;
            match matrix_term_parts(first) {
                Some((elem, _, _)) => Some(TypeTerm::Vector(Box::new(elem))),
                None => {
                    let elem = match first_numeric_term(arg_terms) {
                        TypeTerm::Int | TypeTerm::Double => first_numeric_term(arg_terms),
                        _ => TypeTerm::Double,
                    };
                    Some(TypeTerm::Matrix(Box::new(elem)))
                }
            }
        }
        "rowSums" | "colSums" => {
            let prim = match first_numeric_term(arg_terms) {
                TypeTerm::Int | TypeTerm::Double => TypeTerm::Double,
                _ => TypeTerm::Double,
            };
            Some(TypeTerm::Vector(Box::new(prim)))
        }
        "crossprod" => {
            let (elem, _rows, cols) = matrix_term_parts(arg_terms.first()?)?;
            let elem = match elem {
                TypeTerm::Int | TypeTerm::Double => TypeTerm::Double,
                _ => TypeTerm::Double,
            };
            Some(matrix_term_with_dims(elem, cols, cols))
        }
        "tcrossprod" => {
            let (elem, rows, _cols) = matrix_term_parts(arg_terms.first()?)?;
            let elem = match elem {
                TypeTerm::Int | TypeTerm::Double => TypeTerm::Double,
                _ => TypeTerm::Double,
            };
            Some(matrix_term_with_dims(elem, rows, rows))
        }
        "rbind" => {
            let mut elem = TypeTerm::Any;
            let mut rows = Some(0i64);
            let mut cols: Option<i64> = None;
            for term in arg_terms {
                match term {
                    TypeTerm::Vector(inner) | TypeTerm::VectorLen(inner, _) => {
                        elem = elem.join(inner);
                        rows = rows.map(|r| r + 1);
                        cols = None;
                    }
                    TypeTerm::Matrix(_)
                    | TypeTerm::MatrixDim(_, _, _)
                    | TypeTerm::ArrayDim(_, _) => {
                        let (inner, r, c) = matrix_term_parts(term)?;
                        elem = elem.join(&inner);
                        rows = match (rows, r) {
                            (Some(acc), Some(n)) => Some(acc + n),
                            _ => None,
                        };
                        cols = match (cols, c) {
                            (None, x) => x,
                            (Some(a), Some(b)) if a == b => Some(a),
                            _ => None,
                        };
                    }
                    _ => return Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
                }
            }
            Some(matrix_term_with_dims(elem, rows, cols))
        }
        "cbind" => {
            let mut elem = TypeTerm::Any;
            let mut rows: Option<i64> = None;
            let mut cols = Some(0i64);
            for term in arg_terms {
                match term {
                    TypeTerm::Vector(inner) | TypeTerm::VectorLen(inner, _) => {
                        elem = elem.join(inner);
                        cols = cols.map(|c| c + 1);
                        rows = None;
                    }
                    TypeTerm::Matrix(_)
                    | TypeTerm::MatrixDim(_, _, _)
                    | TypeTerm::ArrayDim(_, _) => {
                        let (inner, r, c) = matrix_term_parts(term)?;
                        elem = elem.join(&inner);
                        rows = match (rows, r) {
                            (None, x) => x,
                            (Some(a), Some(b)) if a == b => Some(a),
                            _ => None,
                        };
                        cols = match (cols, c) {
                            (Some(acc), Some(n)) => Some(acc + n),
                            _ => None,
                        };
                    }
                    _ => return Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
                }
            }
            Some(matrix_term_with_dims(elem, rows, cols))
        }
        _ => None,
    }
}
