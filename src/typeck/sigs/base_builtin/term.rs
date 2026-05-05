use super::*;

pub(crate) fn infer_builtin_term(callee: &str, arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    match callee {
        "length" | "nrow" | "ncol" => Some(TypeTerm::Int),
        "seq" => Some(seq_output_term(arg_terms)),
        "seq_len" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "seq_along" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "names" => Some(names_like_output_term(first_arg_term(arg_terms))),
        "rownames" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "colnames" => Some(names_like_output_term(first_arg_term(arg_terms))),
        "order" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "any" | "all" => Some(TypeTerm::Logical),
        "cat" => Some(TypeTerm::Null),
        "which" => Some(vector_index_or_scalar_term(first_arg_term(arg_terms))),
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
        "regexpr" | "agrep" => Some(int_like_first_arg_term(second_or_first_term(arg_terms))),
        "agrepl" => Some(logical_like_first_arg_term(second_or_first_term(arg_terms))),
        "gregexpr" | "regexec" => Some(TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(
            TypeTerm::Int,
        ))))),
        "strsplit" => Some(TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(
            TypeTerm::Char,
        ))))),
        "paste" | "paste0" | "sprintf" => Some(text_join_output_term(arg_terms)),
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
        "nzchar" | "grepl" | "startsWith" | "endsWith" => Some(logical_like_first_arg_term(
            string_predicate_subject_term(callee, arg_terms),
        )),
        "grep" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "union" | "intersect" | "setdiff" => {
            Some(TypeTerm::Vector(Box::new(joined_general_term(arg_terms))))
        }
        "sort" | "unique" => Some(vectorized_first_arg_term(first_arg_term(arg_terms))),
        "duplicated" => Some(logical_vector_or_scalar_term(first_arg_term(arg_terms))),
        "match" => Some(vector_index_or_scalar_term(first_arg_term(arg_terms))),
        "anyDuplicated" => Some(TypeTerm::Int),
        "dim" => Some(dim_output_term(first_arg_term(arg_terms))),
        "dimnames" => Some(TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(
            TypeTerm::Char,
        ))))),
        "rr_i0" | "rr_i1" | "rr_index1_read_idx" => Some(TypeTerm::Int),
        "rr_index_vec_floor" => Some(rr_index_vec_floor_output_term(arg_terms)),
        "c" => Some(concat_output_term(arg_terms)),
        "list" => Some(list_output_term(arg_terms)),
        "box" => Some(TypeTerm::Boxed(Box::new(first_arg_term(arg_terms)))),
        "unbox" => Some(
            arg_terms
                .first()
                .map(TypeTerm::unbox)
                .unwrap_or(TypeTerm::Any),
        ),
        "abs" | "pmax" | "pmin" => numeric_vectorized_output_term(arg_terms),
        "min" | "max" | "sum" => numeric_scalar_summary_term(arg_terms),
        "prod" | "var" | "sd" => Some(TypeTerm::Double),
        "mean" => {
            if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
                return None;
            }
            Some(TypeTerm::Double)
        }
        "sign" => numeric_vectorized_output_term(arg_terms),
        "sqrt" | "log" | "log10" | "log2" | "exp" | "atan" | "atan2" | "asin" | "acos" | "sin"
        | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "gamma" | "lgamma" | "floor" | "ceiling"
        | "trunc" | "round" => numeric_double_vectorized_output_term(arg_terms),
        "is.na" | "is.finite" => logical_vectorized_output_term(arg_terms),
        "numeric" | "double" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "integer" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "logical" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "character" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "rep" | "rep.int" => Some(rep_output_term(arg_terms)),
        "matrix" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "t" => transpose_output_term(arg_terms),
        "diag" => diag_output_term(arg_terms),
        "rowSums" | "colSums" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "crossprod" => crossprod_output_term(arg_terms),
        "tcrossprod" => tcrossprod_output_term(arg_terms),
        "rbind" => bind_output_term(arg_terms, BindAxis::Rows),
        "cbind" => bind_output_term(arg_terms, BindAxis::Cols),
        _ => None,
    }
}

enum BindAxis {
    Rows,
    Cols,
}

fn seq_output_term(arg_terms: &[TypeTerm]) -> TypeTerm {
    let prim = if arg_terms
        .iter()
        .any(|t| matches!(shallow_elem_term(t), TypeTerm::Double))
    {
        TypeTerm::Double
    } else {
        TypeTerm::Int
    };
    TypeTerm::Vector(Box::new(prim))
}

fn names_like_output_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::DataFrame(_) | TypeTerm::DataFrameNamed(_) => {
            TypeTerm::VectorLen(Box::new(TypeTerm::Char), dataframe_col_count(&first))
        }
        _ => TypeTerm::Vector(Box::new(TypeTerm::Char)),
    }
}

fn second_or_first_term(arg_terms: &[TypeTerm]) -> TypeTerm {
    arg_terms
        .get(1)
        .cloned()
        .unwrap_or_else(|| first_arg_term(arg_terms))
}

fn string_predicate_subject_term(callee: &str, arg_terms: &[TypeTerm]) -> TypeTerm {
    if callee == "grepl" {
        second_or_first_term(arg_terms)
    } else {
        first_arg_term(arg_terms)
    }
}

fn text_join_output_term(arg_terms: &[TypeTerm]) -> TypeTerm {
    if arg_terms.is_empty() || any_vector_term(arg_terms) {
        TypeTerm::Vector(Box::new(TypeTerm::Char))
    } else {
        TypeTerm::Char
    }
}

fn vector_index_or_scalar_term(first: TypeTerm) -> TypeTerm {
    if is_shaped_term(&first) {
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    } else {
        TypeTerm::Int
    }
}

fn logical_vector_or_scalar_term(first: TypeTerm) -> TypeTerm {
    if is_shaped_term(&first) {
        TypeTerm::Vector(Box::new(TypeTerm::Logical))
    } else {
        TypeTerm::Logical
    }
}

fn rr_index_vec_floor_output_term(arg_terms: &[TypeTerm]) -> TypeTerm {
    if arg_terms.iter().any(is_shaped_term) {
        TypeTerm::Vector(Box::new(TypeTerm::Int))
    } else {
        TypeTerm::Int
    }
}

fn is_shaped_term(term: &TypeTerm) -> bool {
    matches!(
        term,
        TypeTerm::Vector(_)
            | TypeTerm::VectorLen(_, _)
            | TypeTerm::Matrix(_)
            | TypeTerm::MatrixDim(_, _, _)
            | TypeTerm::ArrayDim(_, _)
    )
}

fn dim_output_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::MatrixDim(_, _, _) => TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(2)),
        TypeTerm::ArrayDim(_, dims) => {
            TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(dims.len() as i64))
        }
        TypeTerm::DataFrame(_) | TypeTerm::DataFrameNamed(_) => {
            TypeTerm::VectorLen(Box::new(TypeTerm::Int), Some(2))
        }
        _ => TypeTerm::Vector(Box::new(TypeTerm::Int)),
    }
}

fn concat_output_term(arg_terms: &[TypeTerm]) -> TypeTerm {
    let mut elem = TypeTerm::Any;
    for term in arg_terms {
        elem = elem.join(&container_element_or_self(term));
    }
    TypeTerm::Vector(Box::new(elem))
}

fn list_output_term(arg_terms: &[TypeTerm]) -> TypeTerm {
    let mut elem = TypeTerm::Any;
    for term in arg_terms {
        elem = elem.join(term);
    }
    TypeTerm::List(Box::new(elem))
}

fn container_element_or_self(term: &TypeTerm) -> TypeTerm {
    match term {
        TypeTerm::Vector(inner)
        | TypeTerm::VectorLen(inner, _)
        | TypeTerm::Matrix(inner)
        | TypeTerm::MatrixDim(inner, _, _)
        | TypeTerm::ArrayDim(inner, _) => inner.as_ref().clone(),
        _ => term.clone(),
    }
}

fn numeric_vectorized_output_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    let prim = numeric_promoted_primitive(arg_terms)?;
    if any_vector_term(arg_terms) {
        Some(TypeTerm::Vector(Box::new(prim)))
    } else {
        Some(prim)
    }
}

fn numeric_scalar_summary_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    numeric_promoted_primitive(arg_terms)
}

fn numeric_promoted_primitive(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
        return None;
    }
    match promoted_numeric_term(arg_terms) {
        TypeTerm::Int => Some(TypeTerm::Int),
        TypeTerm::Double => Some(TypeTerm::Double),
        _ => None,
    }
}

fn numeric_double_vectorized_output_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
        return None;
    }
    if any_vector_term(arg_terms) {
        Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
    } else {
        Some(TypeTerm::Double)
    }
}

fn logical_vectorized_output_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
        return None;
    }
    if any_vector_term(arg_terms) {
        Some(TypeTerm::Vector(Box::new(TypeTerm::Logical)))
    } else {
        Some(TypeTerm::Logical)
    }
}

fn rep_output_term(arg_terms: &[TypeTerm]) -> TypeTerm {
    TypeTerm::Vector(Box::new(
        arg_terms
            .first()
            .map(container_element_or_self)
            .unwrap_or(TypeTerm::Any),
    ))
}

fn transpose_output_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    let (elem, rows, cols) = matrix_term_parts(arg_terms.first()?)?;
    Some(matrix_term_with_dims(elem, cols, rows))
}

fn diag_output_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
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

fn crossprod_output_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    let (elem, _rows, cols) = matrix_term_parts(arg_terms.first()?)?;
    Some(matrix_term_with_dims(double_matrix_elem(elem), cols, cols))
}

fn tcrossprod_output_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    let (elem, rows, _cols) = matrix_term_parts(arg_terms.first()?)?;
    Some(matrix_term_with_dims(double_matrix_elem(elem), rows, rows))
}

fn double_matrix_elem(_elem: TypeTerm) -> TypeTerm {
    TypeTerm::Double
}

fn bind_output_term(arg_terms: &[TypeTerm], axis: BindAxis) -> Option<TypeTerm> {
    let mut elem = TypeTerm::Any;
    let mut rows = bind_initial_rows(&axis);
    let mut cols = bind_initial_cols(&axis);
    for term in arg_terms {
        match term {
            TypeTerm::Vector(inner) | TypeTerm::VectorLen(inner, _) => {
                elem = elem.join(inner);
                bind_vector_shape(&axis, &mut rows, &mut cols);
            }
            TypeTerm::Matrix(_) | TypeTerm::MatrixDim(_, _, _) | TypeTerm::ArrayDim(_, _) => {
                let (inner, term_rows, term_cols) = matrix_term_parts(term)?;
                elem = elem.join(&inner);
                bind_matrix_shape(&axis, &mut rows, &mut cols, term_rows, term_cols);
            }
            _ => return Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        }
    }
    Some(matrix_term_with_dims(elem, rows, cols))
}

fn bind_initial_rows(axis: &BindAxis) -> Option<i64> {
    match axis {
        BindAxis::Rows => Some(0),
        BindAxis::Cols => None,
    }
}

fn bind_initial_cols(axis: &BindAxis) -> Option<i64> {
    match axis {
        BindAxis::Rows => None,
        BindAxis::Cols => Some(0),
    }
}

fn bind_vector_shape(axis: &BindAxis, rows: &mut Option<i64>, cols: &mut Option<i64>) {
    match axis {
        BindAxis::Rows => {
            *rows = rows.map(|r| r + 1);
            *cols = None;
        }
        BindAxis::Cols => {
            *cols = cols.map(|c| c + 1);
            *rows = None;
        }
    }
}

fn bind_matrix_shape(
    axis: &BindAxis,
    rows: &mut Option<i64>,
    cols: &mut Option<i64>,
    term_rows: Option<i64>,
    term_cols: Option<i64>,
) {
    match axis {
        BindAxis::Rows => {
            *rows = add_optional_dim(*rows, term_rows);
            *cols = merge_equal_dim(*cols, term_cols);
        }
        BindAxis::Cols => {
            *rows = merge_equal_dim(*rows, term_rows);
            *cols = add_optional_dim(*cols, term_cols);
        }
    }
}

fn add_optional_dim(acc: Option<i64>, next: Option<i64>) -> Option<i64> {
    match (acc, next) {
        (Some(acc), Some(next)) => Some(acc + next),
        _ => None,
    }
}

fn merge_equal_dim(acc: Option<i64>, next: Option<i64>) -> Option<i64> {
    match (acc, next) {
        (None, x) => x,
        (Some(a), Some(b)) if a == b => Some(a),
        _ => None,
    }
}
