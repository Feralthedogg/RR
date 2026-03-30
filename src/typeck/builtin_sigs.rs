use super::lattice::{PrimTy, ShapeTy, TypeState};
use super::term::TypeTerm;

fn promoted_numeric_prim(args: &[TypeState]) -> PrimTy {
    args.iter()
        .fold(PrimTy::Any, |acc, ty| match (acc, ty.prim) {
            (PrimTy::Any, other) => other,
            (other, PrimTy::Any) => other,
            (PrimTy::Int, PrimTy::Int) => PrimTy::Int,
            (PrimTy::Int, PrimTy::Double)
            | (PrimTy::Double, PrimTy::Int)
            | (PrimTy::Double, PrimTy::Double) => PrimTy::Double,
            _ => PrimTy::Any,
        })
}

fn promoted_numeric_term(args: &[TypeTerm]) -> TypeTerm {
    fn numeric_elem_term(ty: &TypeTerm) -> TypeTerm {
        match ty {
            TypeTerm::Vector(inner)
            | TypeTerm::VectorLen(inner, _)
            | TypeTerm::Matrix(inner)
            | TypeTerm::MatrixDim(inner, _, _)
            | TypeTerm::ArrayDim(inner, _) => numeric_elem_term(inner),
            _ => ty.clone(),
        }
    }

    args.iter().fold(TypeTerm::Any, |acc, ty| {
        match (&acc, &numeric_elem_term(ty)) {
            (TypeTerm::Any, other) => other.clone(),
            (other, TypeTerm::Any) => other.clone(),
            (TypeTerm::Int, TypeTerm::Int) => TypeTerm::Int,
            (TypeTerm::Int, TypeTerm::Double)
            | (TypeTerm::Double, TypeTerm::Int)
            | (TypeTerm::Double, TypeTerm::Double) => TypeTerm::Double,
            _ => TypeTerm::Any,
        }
    })
}

fn shared_vector_len_sym(args: &[TypeState]) -> Option<super::lattice::LenSym> {
    let mut out = None;
    for ty in args
        .iter()
        .filter(|ty| matches!(ty.shape, ShapeTy::Vector | ShapeTy::Matrix))
    {
        match (out, ty.len_sym) {
            (None, len) => out = len,
            (Some(prev), Some(len)) if prev == len => {}
            (Some(_), Some(_)) | (Some(_), None) => return None,
        }
    }
    out
}

fn any_vector_shape(args: &[TypeState]) -> bool {
    args.iter()
        .any(|t| matches!(t.shape, ShapeTy::Vector | ShapeTy::Matrix))
}

fn all_known_scalar_shape(args: &[TypeState]) -> bool {
    !args.is_empty() && args.iter().all(|t| t.shape == ShapeTy::Scalar)
}

fn all_known_numeric_prim(args: &[TypeState]) -> bool {
    !args.is_empty()
        && args
            .iter()
            .all(|t| matches!(t.prim, PrimTy::Int | PrimTy::Double))
}

fn any_vector_term(args: &[TypeTerm]) -> bool {
    args.iter().any(|t| {
        matches!(
            t,
            TypeTerm::Vector(_)
                | TypeTerm::VectorLen(_, _)
                | TypeTerm::Matrix(_)
                | TypeTerm::MatrixDim(_, _, _)
                | TypeTerm::ArrayDim(_, _)
        )
    })
}

fn all_known_scalar_term(args: &[TypeTerm]) -> bool {
    !args.is_empty()
        && args.iter().all(|t| {
            matches!(
                t,
                TypeTerm::Int
                    | TypeTerm::Double
                    | TypeTerm::Logical
                    | TypeTerm::Char
                    | TypeTerm::Null
            )
        })
}

fn first_numeric_prim(args: &[TypeState]) -> PrimTy {
    args.iter()
        .find_map(|ty| matches!(ty.prim, PrimTy::Int | PrimTy::Double).then_some(ty.prim))
        .unwrap_or(PrimTy::Any)
}

fn first_numeric_term(args: &[TypeTerm]) -> TypeTerm {
    args.iter()
        .find_map(|ty| match ty {
            TypeTerm::Int => Some(TypeTerm::Int),
            TypeTerm::Double => Some(TypeTerm::Double),
            TypeTerm::Vector(inner)
            | TypeTerm::VectorLen(inner, _)
            | TypeTerm::Matrix(inner)
            | TypeTerm::MatrixDim(inner, _, _)
            | TypeTerm::ArrayDim(inner, _)
                if matches!(inner.as_ref(), TypeTerm::Int | TypeTerm::Double) =>
            {
                Some((**inner).clone())
            }
            _ => None,
        })
        .unwrap_or(TypeTerm::Any)
}

fn first_arg_type_state(args: &[TypeState]) -> TypeState {
    args.first().copied().unwrap_or(TypeState::unknown())
}

fn second_arg_type_state(args: &[TypeState]) -> TypeState {
    args.get(1).copied().unwrap_or(TypeState::unknown())
}

fn first_arg_term(args: &[TypeTerm]) -> TypeTerm {
    args.first().cloned().unwrap_or(TypeTerm::Any)
}

fn second_arg_term(args: &[TypeTerm]) -> TypeTerm {
    args.get(1).cloned().unwrap_or(TypeTerm::Any)
}

fn vectorized_first_arg_type(first: TypeState) -> TypeState {
    let prim = match first.shape {
        ShapeTy::Scalar | ShapeTy::Vector | ShapeTy::Matrix => first.prim,
        ShapeTy::Unknown => PrimTy::Any,
    };
    TypeState::vector(prim, false).with_len(first.len_sym)
}

fn vectorized_first_arg_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::Vector(inner)
        | TypeTerm::VectorLen(inner, _)
        | TypeTerm::Matrix(inner)
        | TypeTerm::MatrixDim(inner, _, _)
        | TypeTerm::ArrayDim(inner, _) => TypeTerm::Vector(inner),
        term => TypeTerm::Vector(Box::new(term)),
    }
}

fn preserved_first_arg_type_without_len(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Scalar => TypeState::scalar(first.prim, first.na == super::lattice::NaTy::Never),
        ShapeTy::Vector => TypeState::vector(first.prim, first.na == super::lattice::NaTy::Never),
        ShapeTy::Matrix => TypeState::matrix(first.prim, first.na == super::lattice::NaTy::Never),
        ShapeTy::Unknown => TypeState::unknown(),
    }
}

fn matrix_like_first_arg_type(first: TypeState) -> TypeState {
    TypeState::matrix(first.prim, first.na == super::lattice::NaTy::Never)
}

fn preserved_second_arg_type_without_len(second: TypeState) -> TypeState {
    match second.shape {
        ShapeTy::Scalar => TypeState::scalar(second.prim, second.na == super::lattice::NaTy::Never),
        ShapeTy::Vector => TypeState::vector(second.prim, second.na == super::lattice::NaTy::Never),
        ShapeTy::Matrix => TypeState::matrix(second.prim, second.na == super::lattice::NaTy::Never),
        ShapeTy::Unknown => TypeState::unknown(),
    }
}

fn preserved_head_tail_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::Matrix(inner)
        | TypeTerm::MatrixDim(inner, _, _)
        | TypeTerm::ArrayDim(inner, _) => TypeTerm::Matrix(inner),
        TypeTerm::Vector(_)
        | TypeTerm::VectorLen(_, _)
        | TypeTerm::DataFrame(_)
        | TypeTerm::DataFrameNamed(_)
        | TypeTerm::NamedList(_)
        | TypeTerm::List(_)
        | TypeTerm::Int
        | TypeTerm::Double
        | TypeTerm::Logical
        | TypeTerm::Char
        | TypeTerm::Null
        | TypeTerm::Any
        | TypeTerm::Never
        | TypeTerm::Boxed(_)
        | TypeTerm::Option(_)
        | TypeTerm::Union(_) => first,
    }
}

fn matrix_like_first_arg_term(first: TypeTerm) -> TypeTerm {
    TypeTerm::Matrix(Box::new(shallow_elem_term(&first)))
}

fn char_like_first_arg_type(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Scalar => TypeState::scalar(PrimTy::Char, false),
        ShapeTy::Vector | ShapeTy::Matrix => {
            TypeState::vector(PrimTy::Char, false).with_len(first.len_sym)
        }
        ShapeTy::Unknown => TypeState::vector(PrimTy::Char, false),
    }
}

fn int_like_first_arg_type(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Scalar => TypeState::scalar(PrimTy::Int, false),
        ShapeTy::Vector | ShapeTy::Matrix => {
            TypeState::vector(PrimTy::Int, false).with_len(first.len_sym)
        }
        ShapeTy::Unknown => TypeState::vector(PrimTy::Int, false),
    }
}

fn logical_like_first_arg_type(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Scalar => TypeState::scalar(PrimTy::Logical, false),
        ShapeTy::Vector | ShapeTy::Matrix => {
            TypeState::vector(PrimTy::Logical, false).with_len(first.len_sym)
        }
        ShapeTy::Unknown => TypeState::vector(PrimTy::Logical, false),
    }
}

fn double_like_first_arg_type(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Scalar => TypeState::scalar(PrimTy::Double, false),
        ShapeTy::Vector | ShapeTy::Matrix => {
            TypeState::vector(PrimTy::Double, false).with_len(first.len_sym)
        }
        ShapeTy::Unknown => TypeState::vector(PrimTy::Double, false),
    }
}

fn char_like_first_arg_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::Vector(_)
        | TypeTerm::VectorLen(_, _)
        | TypeTerm::Matrix(_)
        | TypeTerm::MatrixDim(_, _, _)
        | TypeTerm::ArrayDim(_, _)
        | TypeTerm::DataFrame(_)
        | TypeTerm::DataFrameNamed(_)
        | TypeTerm::List(_) => TypeTerm::Vector(Box::new(TypeTerm::Char)),
        _ => TypeTerm::Char,
    }
}

fn int_like_first_arg_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::Vector(_)
        | TypeTerm::VectorLen(_, _)
        | TypeTerm::Matrix(_)
        | TypeTerm::MatrixDim(_, _, _)
        | TypeTerm::ArrayDim(_, _)
        | TypeTerm::DataFrame(_)
        | TypeTerm::DataFrameNamed(_)
        | TypeTerm::List(_) => TypeTerm::Vector(Box::new(TypeTerm::Int)),
        _ => TypeTerm::Int,
    }
}

fn logical_like_first_arg_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::Vector(_)
        | TypeTerm::VectorLen(_, _)
        | TypeTerm::Matrix(_)
        | TypeTerm::MatrixDim(_, _, _)
        | TypeTerm::ArrayDim(_, _)
        | TypeTerm::DataFrame(_)
        | TypeTerm::DataFrameNamed(_)
        | TypeTerm::List(_) => TypeTerm::Vector(Box::new(TypeTerm::Logical)),
        _ => TypeTerm::Logical,
    }
}

fn double_like_first_arg_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::Vector(_)
        | TypeTerm::VectorLen(_, _)
        | TypeTerm::Matrix(_)
        | TypeTerm::MatrixDim(_, _, _)
        | TypeTerm::ArrayDim(_, _)
        | TypeTerm::DataFrame(_)
        | TypeTerm::DataFrameNamed(_)
        | TypeTerm::List(_) => TypeTerm::Vector(Box::new(TypeTerm::Double)),
        _ => TypeTerm::Double,
    }
}

fn joined_general_prim(args: &[TypeState]) -> PrimTy {
    args.iter()
        .fold(PrimTy::Any, |acc, ty| match (acc, ty.prim) {
            (PrimTy::Any, other) => other,
            (other, PrimTy::Any) => other,
            (a, b) if a == b => a,
            (PrimTy::Int, PrimTy::Double) | (PrimTy::Double, PrimTy::Int) => PrimTy::Double,
            _ => PrimTy::Any,
        })
}

fn shallow_elem_term(term: &TypeTerm) -> TypeTerm {
    match term {
        TypeTerm::Vector(inner)
        | TypeTerm::VectorLen(inner, _)
        | TypeTerm::Matrix(inner)
        | TypeTerm::MatrixDim(inner, _, _)
        | TypeTerm::ArrayDim(inner, _)
        | TypeTerm::List(inner) => inner.as_ref().clone(),
        _ => term.clone(),
    }
}

fn joined_general_term(args: &[TypeTerm]) -> TypeTerm {
    args.iter().fold(TypeTerm::Any, |acc, term| {
        acc.join(&shallow_elem_term(term))
    })
}

fn rank_output_type(first: TypeState) -> TypeState {
    if matches!(first.shape, ShapeTy::Scalar) {
        TypeState::scalar(PrimTy::Double, false)
    } else {
        TypeState::vector(PrimTy::Double, false).with_len(first.len_sym)
    }
}

fn rank_output_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::Vector(_)
        | TypeTerm::VectorLen(_, _)
        | TypeTerm::Matrix(_)
        | TypeTerm::MatrixDim(_, _, _)
        | TypeTerm::ArrayDim(_, _)
        | TypeTerm::DataFrame(_)
        | TypeTerm::DataFrameNamed(_)
        | TypeTerm::List(_) => TypeTerm::Vector(Box::new(TypeTerm::Double)),
        _ => TypeTerm::Double,
    }
}

fn sample_output_type(first: TypeState) -> TypeState {
    let prim = match (first.shape, first.prim) {
        (ShapeTy::Scalar, PrimTy::Int | PrimTy::Double) => PrimTy::Int,
        (_, prim) => prim,
    };
    TypeState::vector(prim, false)
}

fn sample_output_term(first: TypeTerm) -> TypeTerm {
    let elem = match first {
        TypeTerm::Int | TypeTerm::Double => TypeTerm::Int,
        other => shallow_elem_term(&other),
    };
    TypeTerm::Vector(Box::new(elem))
}

fn ts_like_output_type(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Matrix => TypeState::matrix(first.prim, false),
        _ => TypeState::vector(first.prim, false).with_len(first.len_sym),
    }
}

fn ts_like_output_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::Matrix(inner)
        | TypeTerm::MatrixDim(inner, _, _)
        | TypeTerm::ArrayDim(inner, _) => TypeTerm::Matrix(inner),
        other => TypeTerm::Vector(Box::new(shallow_elem_term(&other))),
    }
}

fn ifelse_output_type(arg_tys: &[TypeState]) -> Option<TypeState> {
    if arg_tys.len() < 3 {
        return Some(TypeState::vector(PrimTy::Any, false));
    }
    let yes = arg_tys[1];
    let no = arg_tys[2];
    let prim = match (yes.prim, no.prim) {
        (PrimTy::Any, other) => other,
        (other, PrimTy::Any) => other,
        (a, b) if a == b => a,
        (PrimTy::Int, PrimTy::Double) | (PrimTy::Double, PrimTy::Int) => PrimTy::Double,
        _ => PrimTy::Any,
    };
    let any_vec = arg_tys
        .iter()
        .take(3)
        .any(|t| matches!(t.shape, ShapeTy::Vector | ShapeTy::Matrix));
    if any_vec {
        Some(TypeState::vector(prim, false).with_len(shared_vector_len_sym(&arg_tys[..3])))
    } else {
        Some(TypeState::scalar(prim, false))
    }
}

fn ifelse_output_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    if arg_terms.len() < 3 {
        return Some(TypeTerm::Vector(Box::new(TypeTerm::Any)));
    }
    let yes = shallow_elem_term(&arg_terms[1]);
    let no = shallow_elem_term(&arg_terms[2]);
    let joined = yes.join(&no);
    let any_vec = arg_terms.iter().take(3).any(|t| {
        matches!(
            t,
            TypeTerm::Vector(_)
                | TypeTerm::VectorLen(_, _)
                | TypeTerm::Matrix(_)
                | TypeTerm::MatrixDim(_, _, _)
                | TypeTerm::ArrayDim(_, _)
        )
    });
    if any_vec {
        Some(TypeTerm::Vector(Box::new(joined)))
    } else {
        Some(joined)
    }
}

fn vectorized_scalar_or_vector_double_type(arg_tys: &[TypeState]) -> Option<TypeState> {
    if !any_vector_shape(arg_tys) && !all_known_scalar_shape(arg_tys) {
        return None;
    }
    if any_vector_shape(arg_tys) {
        Some(TypeState::vector(PrimTy::Double, false).with_len(shared_vector_len_sym(arg_tys)))
    } else {
        Some(TypeState::scalar(PrimTy::Double, false))
    }
}

fn vectorized_scalar_or_vector_double_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
        return None;
    }
    if any_vector_term(arg_terms) {
        Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
    } else {
        Some(TypeTerm::Double)
    }
}

fn scalar_or_matrix_double_type(arg_tys: &[TypeState]) -> Option<TypeState> {
    if arg_tys.iter().any(|t| matches!(t.shape, ShapeTy::Matrix)) {
        Some(TypeState::matrix(PrimTy::Double, false))
    } else {
        Some(TypeState::scalar(PrimTy::Double, false))
    }
}

fn scalar_or_matrix_double_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    if arg_terms.iter().any(|t| {
        matches!(
            t,
            TypeTerm::Matrix(_)
                | TypeTerm::MatrixDim(_, _, _)
                | TypeTerm::ArrayDim(_, _)
                | TypeTerm::DataFrame(_)
                | TypeTerm::DataFrameNamed(_)
        )
    }) {
        Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
    } else {
        Some(TypeTerm::Double)
    }
}

fn matrix_term_parts(term: &TypeTerm) -> Option<(TypeTerm, Option<i64>, Option<i64>)> {
    match term {
        TypeTerm::Matrix(inner) => Some((inner.as_ref().clone(), None, None)),
        TypeTerm::MatrixDim(inner, rows, cols) => Some((inner.as_ref().clone(), *rows, *cols)),
        TypeTerm::ArrayDim(inner, dims) => Some((
            inner.as_ref().clone(),
            dims.first().copied().flatten(),
            dims.get(1).copied().flatten(),
        )),
        _ => None,
    }
}

fn matrix_term_with_dims(elem: TypeTerm, rows: Option<i64>, cols: Option<i64>) -> TypeTerm {
    if rows.is_none() && cols.is_none() {
        TypeTerm::Matrix(Box::new(elem))
    } else {
        TypeTerm::MatrixDim(Box::new(elem), rows, cols)
    }
}

fn dataframe_col_count(term: &TypeTerm) -> Option<i64> {
    match term {
        TypeTerm::DataFrame(cols) => Some(cols.len() as i64),
        TypeTerm::DataFrameNamed(cols) => Some(cols.len() as i64),
        _ => None,
    }
}

pub fn infer_builtin(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
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
                Some(TypeState::scalar(PrimTy::Int, false))
            } else {
                Some(TypeState::vector(PrimTy::Int, false).with_len(first.len_sym))
            }
        }
        "which.min" | "which.max" => Some(TypeState::scalar(PrimTy::Int, false)),
        "isTRUE" | "isFALSE" => Some(TypeState::scalar(PrimTy::Logical, false)),
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
            let mut out = TypeState::vector(PrimTy::Any, false);
            for t in arg_tys {
                let promoted = if t.shape == ShapeTy::Vector {
                    TypeState::vector(t.prim, t.na == super::lattice::NaTy::Never)
                } else {
                    TypeState::vector(t.prim, false)
                };
                out = out.join(promoted);
            }
            if arg_tys.len() == 1 {
                out = out.with_len(shared_vector_len_sym(arg_tys));
            }
            Some(out)
        }
        "abs" | "pmax" | "pmin" => {
            let prim = promoted_numeric_prim(arg_tys);
            if !any_vector_shape(arg_tys) && !all_known_scalar_shape(arg_tys) {
                return None;
            }
            if prim == PrimTy::Any && !all_known_numeric_prim(arg_tys) {
                return None;
            }
            let prim = if matches!(prim, PrimTy::Int | PrimTy::Double) {
                prim
            } else {
                PrimTy::Double
            };
            if any_vector_shape(arg_tys) {
                Some(TypeState::vector(prim, false).with_len(shared_vector_len_sym(arg_tys)))
            } else {
                Some(TypeState::scalar(prim, false))
            }
        }
        "min" | "max" => {
            if !any_vector_shape(arg_tys) && !all_known_scalar_shape(arg_tys) {
                return None;
            }
            let prim = promoted_numeric_prim(arg_tys);
            if prim == PrimTy::Any && !all_known_numeric_prim(arg_tys) {
                return None;
            }
            Some(TypeState::scalar(
                match prim {
                    PrimTy::Int => PrimTy::Int,
                    _ => PrimTy::Double,
                },
                false,
            ))
        }
        "sum" => {
            if !any_vector_shape(arg_tys) && !all_known_scalar_shape(arg_tys) {
                return None;
            }
            let prim = promoted_numeric_prim(arg_tys);
            if prim == PrimTy::Any && !all_known_numeric_prim(arg_tys) {
                return None;
            }
            Some(TypeState::scalar(
                match prim {
                    PrimTy::Int => PrimTy::Int,
                    _ => PrimTy::Double,
                },
                false,
            ))
        }
        "prod" | "var" | "sd" => {
            if !any_vector_shape(arg_tys) && !all_known_scalar_shape(arg_tys) && !arg_tys.is_empty()
            {
                return None;
            }
            Some(TypeState::scalar(PrimTy::Double, false))
        }
        "mean" => {
            if !any_vector_shape(arg_tys) && !all_known_scalar_shape(arg_tys) {
                return None;
            }
            Some(TypeState::scalar(PrimTy::Double, false))
        }
        "sign" => {
            if !any_vector_shape(arg_tys) && !all_known_scalar_shape(arg_tys) {
                return None;
            }
            let prim = match promoted_numeric_prim(arg_tys) {
                PrimTy::Int => PrimTy::Int,
                PrimTy::Double => PrimTy::Double,
                _ => return None,
            };
            if any_vector_shape(arg_tys) {
                Some(TypeState::vector(prim, false).with_len(shared_vector_len_sym(arg_tys)))
            } else {
                Some(TypeState::scalar(prim, false))
            }
        }
        "sqrt" | "log" | "log10" | "log2" | "exp" | "atan" | "atan2" | "asin" | "acos" | "sin"
        | "cos" | "tan" | "sinh" | "cosh" | "tanh" | "gamma" | "lgamma" | "floor" | "ceiling"
        | "trunc" | "round" => {
            if !any_vector_shape(arg_tys) && !all_known_scalar_shape(arg_tys) {
                return None;
            }
            if any_vector_shape(arg_tys) {
                Some(
                    TypeState::vector(PrimTy::Double, false)
                        .with_len(shared_vector_len_sym(arg_tys)),
                )
            } else {
                Some(TypeState::scalar(PrimTy::Double, false))
            }
        }
        "is.na" | "is.finite" => {
            if !any_vector_shape(arg_tys) && !all_known_scalar_shape(arg_tys) {
                return None;
            }
            if any_vector_shape(arg_tys) {
                Some(
                    TypeState::vector(PrimTy::Logical, false)
                        .with_len(shared_vector_len_sym(arg_tys)),
                )
            } else {
                Some(TypeState::scalar(PrimTy::Logical, false))
            }
        }
        "numeric" | "double" => Some(TypeState::vector(PrimTy::Double, false)),
        "integer" => Some(TypeState::vector(PrimTy::Int, false)),
        "logical" => Some(TypeState::vector(PrimTy::Logical, false)),
        "character" => Some(TypeState::vector(PrimTy::Char, false)),
        "rep" | "rep.int" => {
            let first = arg_tys.first().copied().unwrap_or(TypeState::unknown());
            let prim = match first.shape {
                ShapeTy::Matrix | ShapeTy::Vector | ShapeTy::Scalar => first.prim,
                ShapeTy::Unknown => PrimTy::Any,
            };
            Some(TypeState::vector(prim, false))
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

pub fn infer_package_call(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
        "base::data.frame" => {
            Some(TypeState::matrix(PrimTy::Any, false).with_len(shared_vector_len_sym(arg_tys)))
        }
        "base::globalenv" | "base::environment" => Some(TypeState::unknown()),
        "base::unlink" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::file.path" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::basename" | "base::dirname" | "base::normalizePath" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::dir.exists" | "base::file.exists" => {
            Some(logical_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::eval" | "base::evalq" | "base::do.call" | "base::parse" | "base::readRDS"
        | "base::get0" | "base::getOption" | "base::file" => Some(TypeState::unknown()),
        "base::save" => Some(TypeState::null()),
        "base::list.files" | "base::path.expand" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::getNamespace" | "base::asNamespace" => Some(TypeState::unknown()),
        "base::isNamespace" | "base::is.name" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::find.package" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::package_version" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::getElement" | "base::unname" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "base::baseenv"
        | "base::emptyenv"
        | "base::new.env"
        | "base::parent.env"
        | "base::as.environment"
        | "base::list2env"
        | "base::topenv" => Some(TypeState::unknown()),
        "base::is.environment"
        | "base::environmentIsLocked"
        | "base::isNamespaceLoaded"
        | "base::requireNamespace" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::environmentName" | "base::getNamespaceName" | "base::getNamespaceVersion" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "base::loadedNamespaces" | "base::getNamespaceExports" | "base::getNamespaceUsers" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "base::as.list.environment" | "base::getNamespaceImports" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "base::library" | "base::searchpaths" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::require" | "base::packageHasNamespace" | "base::is.loaded" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "base::identical"
        | "base::inherits"
        | "base::interactive"
        | "base::is.R"
        | "base::is.array"
        | "base::is.atomic"
        | "base::is.call"
        | "base::is.character"
        | "base::is.complex"
        | "base::is.data.frame"
        | "base::is.double"
        | "base::is.expression"
        | "base::is.factor"
        | "base::is.function"
        | "base::is.integer"
        | "base::is.language"
        | "base::is.list"
        | "base::is.logical"
        | "base::is.null"
        | "base::is.numeric"
        | "base::is.numeric.Date"
        | "base::is.numeric.POSIXt"
        | "base::is.numeric.difftime"
        | "base::is.numeric_version"
        | "base::is.object"
        | "base::is.ordered"
        | "base::is.package_version"
        | "base::is.pairlist"
        | "base::is.primitive"
        | "base::is.qr"
        | "base::is.raw"
        | "base::is.recursive"
        | "base::is.single"
        | "base::is.symbol"
        | "base::is.table"
        | "base::is.unsorted"
        | "base::is.vector" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::is.element"
        | "base::is.finite.POSIXlt"
        | "base::is.infinite"
        | "base::is.infinite.POSIXlt"
        | "base::is.na.POSIXlt"
        | "base::is.na.data.frame"
        | "base::is.na.numeric_version"
        | "base::is.nan"
        | "base::is.nan.POSIXlt" => {
            Some(logical_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::loadNamespace" | "base::getLoadedDLLs" | "base::dyn.load" => {
            Some(TypeState::unknown())
        }
        "base::dyn.unload" => Some(TypeState::null()),
        "base::readLines" | "base::Sys.getenv" | "base::Sys.which" | "base::Sys.readlink"
        | "base::Sys.info" | "base::Sys.glob" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::writeLines"
        | "base::writeChar"
        | "base::writeBin"
        | "base::flush"
        | "base::truncate.connection" => Some(TypeState::null()),
        "base::seek" => Some(TypeState::scalar(PrimTy::Double, false)),
        "base::Sys.setenv" | "base::Sys.unsetenv" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "base::Sys.getpid" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::Sys.time" | "base::Sys.Date" => Some(TypeState::scalar(PrimTy::Double, false)),
        "base::Sys.getlocale" => Some(TypeState::scalar(PrimTy::Char, false)),
        "base::system" | "base::system2" => Some(TypeState::unknown()),
        "base::system.time" => Some(TypeState::vector(PrimTy::Double, false)),
        "base::Sys.sleep" => Some(TypeState::null()),
        "base::Sys.setlocale" | "base::Sys.timezone" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "base::Sys.localeconv" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::Sys.setFileTime" | "base::Sys.chmod" => {
            Some(logical_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::Sys.umask" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::sys.parent" | "base::sys.nframe" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::sys.parents" => Some(TypeState::vector(PrimTy::Int, false)),
        "base::search" | "base::gettext" | "base::gettextf" | "base::ngettext" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "base::geterrmessage" => Some(TypeState::scalar(PrimTy::Char, false)),
        "base::message" | "base::packageStartupMessage" | "base::.packageStartupMessage" => {
            Some(TypeState::null())
        }
        "base::sys.call"
        | "base::sys.calls"
        | "base::sys.function"
        | "base::sys.frame"
        | "base::sys.frames"
        | "base::sys.status"
        | "base::sys.source"
        | "base::source"
        | "base::options"
        | "base::warning"
        | "base::warningCondition"
        | "base::packageNotFoundError"
        | "base::packageEvent" => Some(TypeState::unknown()),
        "base::stdin"
        | "base::stdout"
        | "base::stderr"
        | "base::textConnection"
        | "base::rawConnection"
        | "base::socketConnection"
        | "base::url"
        | "base::pipe"
        | "base::open"
        | "base::summary.connection" => Some(TypeState::unknown()),
        "base::textConnectionValue" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::rawConnectionValue" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::close" | "base::closeAllConnections" | "base::pushBack" | "base::clearPushBack" => {
            Some(TypeState::null())
        }
        "base::close.connection" | "base::close.srcfile" | "base::close.srcfilealias" => {
            Some(TypeState::null())
        }
        "base::isOpen" | "base::isIncomplete" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::pushBackLength" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::socketSelect" => Some(TypeState::vector(PrimTy::Logical, false)),
        "base::scan" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::read.table" | "base::read.csv" | "base::read.csv2" | "base::read.delim"
        | "base::read.delim2" => Some(TypeState::matrix(PrimTy::Any, false)),
        "base::write.table" | "base::write.csv" | "base::write.csv2" | "base::saveRDS"
        | "base::dput" | "base::dump" | "base::sink" => Some(TypeState::null()),
        "base::count.fields" => Some(TypeState::vector(PrimTy::Int, false)),
        "base::sink.number" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::capture.output" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::lapply" | "base::Map" | "base::split" | "base::by" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "base::sapply" | "base::vapply" | "base::mapply" | "base::tapply" | "base::apply" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "base::Reduce" | "base::Find" => Some(TypeState::unknown()),
        "base::Filter" | "base::unsplit" | "base::within" | "base::transform" => Some(
            preserved_first_arg_type_without_len(first_arg_type_state(arg_tys)),
        ),
        "base::Position" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::expand.grid" | "base::merge" => Some(TypeState::matrix(PrimTy::Any, false)),
        "base::as.Date"
        | "base::as.Date.character"
        | "base::as.Date.default"
        | "base::as.Date.factor"
        | "base::as.Date.numeric"
        | "base::as.Date.POSIXct"
        | "base::as.Date.POSIXlt"
        | "base::as.POSIXct"
        | "base::as.POSIXct.Date"
        | "base::as.POSIXct.default"
        | "base::as.POSIXct.numeric"
        | "base::as.POSIXct.POSIXlt"
        | "base::as.POSIXlt"
        | "base::as.POSIXlt.character"
        | "base::as.POSIXlt.Date"
        | "base::as.POSIXlt.default"
        | "base::as.POSIXlt.factor"
        | "base::as.POSIXlt.numeric"
        | "base::as.POSIXlt.POSIXct"
        | "base::as.difftime"
        | "base::as.double.difftime"
        | "base::as.double.POSIXlt"
        | "base::strptime"
        | "base::difftime"
        | "base::julian" => Some(double_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::as.character.Date"
        | "base::as.character.POSIXt"
        | "base::format.Date"
        | "base::format.POSIXct"
        | "base::format.POSIXlt"
        | "base::months"
        | "base::quarters"
        | "base::weekdays" => Some(char_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::OlsonNames" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::ISOdate" | "base::ISOdatetime" | "base::seq.Date" | "base::seq.POSIXt" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "base::all.names" | "base::all.vars" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::anyDuplicated.array"
        | "base::anyDuplicated.data.frame"
        | "base::anyDuplicated.default"
        | "base::anyDuplicated.matrix" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::anyNA" | "base::anyNA.data.frame" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::anyNA.numeric_version" | "base::anyNA.POSIXlt" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "base::addTaskCallback" => Some(TypeState::scalar(PrimTy::Int, false)),
        "base::bindingIsActive" | "base::bindingIsLocked" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "base::backsolve" => Some(double_like_first_arg_type(second_arg_type_state(arg_tys))),
        "base::balancePOSIXlt" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::besselI" | "base::besselJ" | "base::besselK" | "base::besselY" | "base::beta"
        | "base::choose" => Some(double_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::casefold" | "base::char.expand" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::charmatch" => Some(int_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::charToRaw" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::chkDots" => Some(TypeState::null()),
        "base::chol" | "base::chol.default" | "base::chol2inv" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "base::chooseOpsMethod" | "base::chooseOpsMethod.default" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "base::complete.cases" => Some(logical_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::cut.Date" | "base::cut.POSIXt" | "base::cut.default" => {
            Some(int_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::complex" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::cummax" | "base::cummin" | "base::cumsum" => {
            Some(vectorized_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::cumprod" => Some(double_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::diff" | "base::diff.default" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "base::diff.Date" | "base::diff.POSIXt" | "base::diff.difftime" => {
            Some(double_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::commandArgs" | "base::data.class" | "base::deparse" | "base::extSoftVersion" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "base::data.matrix" => Some(TypeState::matrix(PrimTy::Double, false)),
        "base::det" => Some(TypeState::scalar(PrimTy::Double, false)),
        "base::determinant" | "base::determinant.matrix" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "base::debuggingState" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::dget" => Some(TypeState::unknown()),
        "base::debug"
        | "base::debugonce"
        | "base::declare"
        | "base::delayedAssign"
        | "base::detach"
        | "base::enquote"
        | "base::env.profile"
        | "base::environment<-"
        | "base::errorCondition"
        | "base::eval.parent"
        | "base::Exec"
        | "base::expression" => Some(TypeState::unknown()),
        "base::date" | "base::deparse1" | "base::file.choose" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "base::dQuote" | "base::enc2native" | "base::enc2utf8" | "base::encodeString"
        | "base::Encoding" => Some(char_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::dontCheck" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "base::digamma" | "base::expm1" | "base::factorial" | "base::acosh" | "base::asinh"
        | "base::atanh" | "base::cospi" => {
            Some(double_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::eigen" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::exists" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "base::findInterval" => Some(int_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::file.show" => Some(TypeState::null()),
        "base::format.data.frame" | "base::format.info" => {
            Some(TypeState::matrix(PrimTy::Char, false))
        }
        "base::format"
        | "base::format.AsIs"
        | "base::format.default"
        | "base::format.difftime"
        | "base::format.factor"
        | "base::format.hexmode"
        | "base::format.libraryIQR"
        | "base::format.numeric_version"
        | "base::format.octmode"
        | "base::format.packageInfo"
        | "base::format.pval"
        | "base::format.summaryDefault"
        | "base::formatC"
        | "base::formatDL" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::drop" => Some(TypeState::unknown()),
        "base::droplevels" | "base::droplevels.data.frame" => Some(
            preserved_first_arg_type_without_len(first_arg_type_state(arg_tys)),
        ),
        "base::droplevels.factor" => Some(int_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::duplicated.default" => infer_builtin("duplicated", arg_tys),
        "base::duplicated.array"
        | "base::duplicated.data.frame"
        | "base::duplicated.matrix"
        | "base::duplicated.numeric_version"
        | "base::duplicated.POSIXlt"
        | "base::duplicated.warnings" => infer_builtin("duplicated", arg_tys),
        "base::attr<-" | "base::attributes<-" | "base::class<-" | "base::colnames<-"
        | "base::comment<-" | "base::dimnames<-" | "base::levels<-" | "base::names<-"
        | "base::row.names<-" | "base::rownames<-" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "base::body<-" => Some(TypeState::unknown()),
        "base::bindtextdomain" => Some(TypeState::scalar(PrimTy::Char, false)),
        "base::builtins" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::alist"
        | "base::as.expression"
        | "base::as.expression.default"
        | "base::as.package_version"
        | "base::as.pairlist" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::as.call"
        | "base::as.function"
        | "base::as.function.default"
        | "base::as.name"
        | "base::as.symbol"
        | "base::activeBindingFunction"
        | "base::allowInterrupts"
        | "base::attach"
        | "base::attachNamespace"
        | "base::autoload"
        | "base::autoloader"
        | "base::break"
        | "base::browser"
        | "base::browserSetDebug"
        | "base::as.qr"
        | "base::asS3"
        | "base::asS4" => Some(TypeState::unknown()),
        "base::Arg" => Some(double_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::aperm.default" | "base::aperm.table" => {
            Some(matrix_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.complex" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::as.hexmode" | "base::as.octmode" | "base::as.ordered" | "base::gl" => {
            Some(int_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.numeric_version" | "base::asplit" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::as.null" | "base::as.null.default" => Some(TypeState::null()),
        "base::as.raw" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::as.single" | "base::as.single.default" => {
            Some(double_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.table" | "base::as.table.default" => {
            Some(matrix_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::bitwAnd" | "base::bitwNot" | "base::bitwOr" | "base::bitwShiftL"
        | "base::bitwShiftR" | "base::bitwXor" => {
            Some(int_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::by.data.frame"
        | "base::by.default"
        | "base::computeRestarts"
        | "base::c.numeric_version"
        | "base::c.POSIXlt"
        | "base::c.warnings" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::c.Date" | "base::c.difftime" | "base::c.POSIXct" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "base::c.factor" => Some(TypeState::vector(PrimTy::Int, false)),
        "base::c.noquote" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::callCC"
        | "base::comment"
        | "base::conditionCall"
        | "base::conditionCall.condition"
        | "base::conflictRules" => Some(TypeState::unknown()),
        "base::cbind.data.frame" => Some(TypeState::matrix(PrimTy::Any, false)),
        "base::conditionMessage"
        | "base::conditionMessage.condition"
        | "base::conflicts"
        | "base::curlGetHeaders" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::contributors" => Some(TypeState::null()),
        "base::Cstack_info" => Some(TypeState::vector(PrimTy::Int, false)),
        "base::browserText" => Some(TypeState::scalar(PrimTy::Char, false)),
        "base::capabilities" => Some(TypeState::vector(PrimTy::Logical, false)),
        "base::Conj" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "base::abbreviate"
        | "base::as.character"
        | "base::as.character.condition"
        | "base::as.character.default"
        | "base::as.character.error"
        | "base::as.character.factor"
        | "base::as.character.hexmode"
        | "base::as.character.numeric_version"
        | "base::as.character.octmode"
        | "base::as.character.srcref" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::all.equal"
        | "base::all.equal.default"
        | "base::all.equal.character"
        | "base::all.equal.environment"
        | "base::all.equal.envRefClass"
        | "base::all.equal.factor"
        | "base::all.equal.formula"
        | "base::all.equal.function"
        | "base::all.equal.language"
        | "base::all.equal.list"
        | "base::all.equal.numeric"
        | "base::all.equal.POSIXt"
        | "base::all.equal.raw"
        | "base::args"
        | "base::body"
        | "base::call"
        | "base::bquote"
        | "base::browserCondition" => Some(TypeState::unknown()),
        "base::array"
        | "base::as.array"
        | "base::as.array.default"
        | "base::as.matrix"
        | "base::as.matrix.data.frame"
        | "base::as.matrix.default"
        | "base::as.matrix.noquote"
        | "base::as.matrix.POSIXlt"
        | "base::aperm" => Some(matrix_like_first_arg_type(first_arg_type_state(arg_tys))),
        "base::as.data.frame"
        | "base::as.data.frame.array"
        | "base::as.data.frame.AsIs"
        | "base::as.data.frame.character"
        | "base::as.data.frame.complex"
        | "base::as.data.frame.data.frame"
        | "base::as.data.frame.Date"
        | "base::as.data.frame.default"
        | "base::as.data.frame.difftime"
        | "base::as.data.frame.factor"
        | "base::as.data.frame.integer"
        | "base::as.data.frame.list"
        | "base::as.data.frame.logical"
        | "base::as.data.frame.matrix"
        | "base::as.data.frame.model.matrix"
        | "base::as.data.frame.noquote"
        | "base::as.data.frame.numeric"
        | "base::as.data.frame.numeric_version"
        | "base::as.data.frame.ordered"
        | "base::as.data.frame.POSIXct"
        | "base::as.data.frame.POSIXlt"
        | "base::as.data.frame.raw"
        | "base::as.data.frame.table"
        | "base::as.data.frame.ts"
        | "base::as.data.frame.vector"
        | "base::array2DF" => Some(TypeState::matrix(PrimTy::Any, false)),
        "base::as.list"
        | "base::as.list.data.frame"
        | "base::as.list.Date"
        | "base::as.list.default"
        | "base::as.list.difftime"
        | "base::as.list.factor"
        | "base::as.list.function"
        | "base::as.list.numeric_version"
        | "base::as.list.POSIXct"
        | "base::as.list.POSIXlt" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::arrayInd" | "base::col" | "base::row" => Some(TypeState::matrix(PrimTy::Int, false)),
        "base::colMeans" | "base::rowMeans" => Some(TypeState::vector(PrimTy::Double, false)),
        "base::append" | "base::addNA" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "base::as.double" | "base::as.numeric" => {
            Some(double_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.factor" | "base::as.integer" | "base::ordered" => {
            Some(int_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.logical" | "base::as.logical.factor" => {
            Some(logical_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::as.vector"
        | "base::as.vector.data.frame"
        | "base::as.vector.factor"
        | "base::as.vector.POSIXlt" => {
            Some(vectorized_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::class" | "base::levels" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::attr" => Some(TypeState::unknown()),
        "base::attributes" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::readBin" | "base::serialize" => Some(TypeState::vector(PrimTy::Any, false)),
        "base::readChar" | "base::load" => Some(TypeState::vector(PrimTy::Char, false)),
        "base::unserialize" | "base::fifo" | "base::gzcon" => Some(TypeState::unknown()),
        "base::getwd" | "base::tempdir" | "base::tempfile" | "base::system.file" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "base::dir" | "base::list.dirs" | "base::path.package" | "base::.packages" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "base::dir.create" | "base::file.create" | "base::file.remove" | "base::file.rename"
        | "base::file.copy" | "base::file.append" | "base::file.link" | "base::file.symlink" => {
            Some(logical_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "base::file.access" | "base::file.mode" => Some(TypeState::vector(PrimTy::Int, false)),
        "base::file.info" => Some(TypeState::matrix(PrimTy::Any, false)),
        "base::file.size" | "base::file.mtime" => Some(TypeState::vector(PrimTy::Double, false)),
        "base::length" => infer_builtin("length", arg_tys),
        "base::seq_len" => Some(TypeState::vector(PrimTy::Int, true)),
        "base::seq_along" => infer_builtin("seq_along", arg_tys),
        "base::c" => infer_builtin("c", arg_tys),
        "base::list" => infer_builtin("list", arg_tys),
        "base::sum" => infer_builtin("sum", arg_tys),
        "base::mean" => infer_builtin("mean", arg_tys),
        "base::abs" => infer_builtin("abs", arg_tys),
        "base::seq" => infer_builtin("seq", arg_tys),
        "base::ifelse" => infer_builtin("ifelse", arg_tys),
        "base::min" => infer_builtin("min", arg_tys),
        "base::max" => infer_builtin("max", arg_tys),
        "base::pmax" => infer_builtin("pmax", arg_tys),
        "base::pmin" => infer_builtin("pmin", arg_tys),
        "base::sqrt" => infer_builtin("sqrt", arg_tys),
        "base::log" => infer_builtin("log", arg_tys),
        "base::log10" => infer_builtin("log10", arg_tys),
        "base::log2" => infer_builtin("log2", arg_tys),
        "base::exp" => infer_builtin("exp", arg_tys),
        "base::atan2" => infer_builtin("atan2", arg_tys),
        "base::sin" => infer_builtin("sin", arg_tys),
        "base::cos" => infer_builtin("cos", arg_tys),
        "base::tan" => infer_builtin("tan", arg_tys),
        "base::asin" => infer_builtin("asin", arg_tys),
        "base::acos" => infer_builtin("acos", arg_tys),
        "base::atan" => infer_builtin("atan", arg_tys),
        "base::sinh" => infer_builtin("sinh", arg_tys),
        "base::cosh" => infer_builtin("cosh", arg_tys),
        "base::tanh" => infer_builtin("tanh", arg_tys),
        "base::sign" => infer_builtin("sign", arg_tys),
        "base::gamma" => infer_builtin("gamma", arg_tys),
        "base::lgamma" => infer_builtin("lgamma", arg_tys),
        "base::floor" => infer_builtin("floor", arg_tys),
        "base::ceiling" => infer_builtin("ceiling", arg_tys),
        "base::trunc" => infer_builtin("trunc", arg_tys),
        "base::round" => infer_builtin("round", arg_tys),
        "base::is.na" => infer_builtin("is.na", arg_tys),
        "base::is.finite" => infer_builtin("is.finite", arg_tys),
        "base::print" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        callee if callee.starts_with("base::print.") => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "base::numeric" => infer_builtin("numeric", arg_tys),
        "base::matrix" => infer_builtin("matrix", arg_tys),
        "base::diag" => infer_builtin("diag", arg_tys),
        "base::t" => infer_builtin("t", arg_tys),
        "base::rbind" => infer_builtin("rbind", arg_tys),
        "base::cbind" => infer_builtin("cbind", arg_tys),
        "base::rowSums" => infer_builtin("rowSums", arg_tys),
        "base::colSums" => infer_builtin("colSums", arg_tys),
        "base::crossprod" => infer_builtin("crossprod", arg_tys),
        "base::tcrossprod" => infer_builtin("tcrossprod", arg_tys),
        "base::dim" => infer_builtin("dim", arg_tys),
        "base::dimnames" => infer_builtin("dimnames", arg_tys),
        "base::nrow" => infer_builtin("nrow", arg_tys),
        "base::ncol" => infer_builtin("ncol", arg_tys),
        "base::character" => infer_builtin("character", arg_tys),
        "base::logical" => infer_builtin("logical", arg_tys),
        "base::integer" => infer_builtin("integer", arg_tys),
        "base::double" => infer_builtin("double", arg_tys),
        "base::rep" => infer_builtin("rep", arg_tys),
        "base::rep.int" => infer_builtin("rep.int", arg_tys),
        "base::any" => infer_builtin("any", arg_tys),
        "base::all" => infer_builtin("all", arg_tys),
        "base::which" => infer_builtin("which", arg_tys),
        "base::prod" => infer_builtin("prod", arg_tys),
        "base::paste" => infer_builtin("paste", arg_tys),
        "base::paste0" => infer_builtin("paste0", arg_tys),
        "base::sprintf" => infer_builtin("sprintf", arg_tys),
        "base::cat" => infer_builtin("cat", arg_tys),
        "base::tolower" => infer_builtin("tolower", arg_tys),
        "base::toupper" => infer_builtin("toupper", arg_tys),
        "base::substr" => infer_builtin("substr", arg_tys),
        "base::sub" => infer_builtin("sub", arg_tys),
        "base::gsub" => infer_builtin("gsub", arg_tys),
        "base::nchar" => infer_builtin("nchar", arg_tys),
        "base::nzchar" => infer_builtin("nzchar", arg_tys),
        "base::grepl" => infer_builtin("grepl", arg_tys),
        "base::grep" => infer_builtin("grep", arg_tys),
        "base::startsWith" => infer_builtin("startsWith", arg_tys),
        "base::endsWith" => infer_builtin("endsWith", arg_tys),
        "base::which.min" => infer_builtin("which.min", arg_tys),
        "base::which.max" => infer_builtin("which.max", arg_tys),
        "base::isTRUE" => infer_builtin("isTRUE", arg_tys),
        "base::isFALSE" => infer_builtin("isFALSE", arg_tys),
        "base::lengths" => infer_builtin("lengths", arg_tys),
        "base::union" => infer_builtin("union", arg_tys),
        "base::intersect" => infer_builtin("intersect", arg_tys),
        "base::setdiff" => infer_builtin("setdiff", arg_tys),
        "base::sample" => infer_builtin("sample", arg_tys),
        "base::sample.int" => infer_builtin("sample.int", arg_tys),
        "base::rank" => infer_builtin("rank", arg_tys),
        "base::factor" => infer_builtin("factor", arg_tys),
        "base::cut" => infer_builtin("cut", arg_tys),
        "base::table" => infer_builtin("table", arg_tys),
        "base::trimws" => infer_builtin("trimws", arg_tys),
        "base::chartr" => infer_builtin("chartr", arg_tys),
        "base::strsplit" => infer_builtin("strsplit", arg_tys),
        "base::regexpr" => infer_builtin("regexpr", arg_tys),
        "base::gregexpr" => infer_builtin("gregexpr", arg_tys),
        "base::regexec" => infer_builtin("regexec", arg_tys),
        "base::agrep" => infer_builtin("agrep", arg_tys),
        "base::agrepl" => infer_builtin("agrepl", arg_tys),
        "base::names" => infer_builtin("names", arg_tys),
        "base::rownames" => infer_builtin("rownames", arg_tys),
        "base::colnames" => infer_builtin("colnames", arg_tys),
        "base::sort" => infer_builtin("sort", arg_tys),
        "base::order" => infer_builtin("order", arg_tys),
        "base::match" => infer_builtin("match", arg_tys),
        "base::unique" => infer_builtin("unique", arg_tys),
        "base::duplicated" => infer_builtin("duplicated", arg_tys),
        "base::anyDuplicated" => infer_builtin("anyDuplicated", arg_tys),
        "base::summary" => Some(TypeState::vector(PrimTy::Any, false)),
        callee if callee.starts_with("base::summary.") => Some(TypeState::unknown()),
        "stats::dnorm" | "stats::pnorm" | "stats::qnorm" | "stats::dbinom" | "stats::pbinom"
        | "stats::qbinom" | "stats::dpois" | "stats::ppois" | "stats::qpois" | "stats::dunif"
        | "stats::punif" | "stats::qunif" | "stats::dgamma" | "stats::pgamma" | "stats::qgamma"
        | "stats::dbeta" | "stats::pbeta" | "stats::qbeta" | "stats::dt" | "stats::pt"
        | "stats::qt" | "stats::df" | "stats::pf" | "stats::qf" | "stats::dchisq"
        | "stats::pchisq" | "stats::qchisq" | "stats::dexp" | "stats::pexp" | "stats::qexp"
        | "stats::dlnorm" | "stats::plnorm" | "stats::qlnorm" | "stats::dweibull"
        | "stats::pweibull" | "stats::qweibull" | "stats::dcauchy" | "stats::pcauchy"
        | "stats::qcauchy" | "stats::dgeom" | "stats::pgeom" | "stats::qgeom" | "stats::dhyper"
        | "stats::phyper" | "stats::qhyper" | "stats::dnbinom" | "stats::pnbinom"
        | "stats::qnbinom" | "stats::dlogis" | "stats::plogis" | "stats::qlogis"
        | "stats::pbirthday" | "stats::qbirthday" | "stats::ptukey" | "stats::qtukey"
        | "stats::psmirnov" | "stats::qsmirnov" | "stats::dsignrank" | "stats::psignrank"
        | "stats::qsignrank" | "stats::dwilcox" | "stats::pwilcox" | "stats::qwilcox" => {
            vectorized_scalar_or_vector_double_type(arg_tys)
        }
        "stats::rnorm" | "stats::runif" | "stats::rgamma" | "stats::rbeta" | "stats::rt"
        | "stats::rf" | "stats::rchisq" | "stats::rexp" | "stats::rlnorm" | "stats::rweibull"
        | "stats::rcauchy" | "stats::rlogis" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::rbinom" | "stats::rpois" | "stats::rgeom" | "stats::rhyper" | "stats::rnbinom"
        | "stats::rsignrank" | "stats::rwilcox" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::rsmirnov" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::acf2AR" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::p.adjust" => vectorized_scalar_or_vector_double_type(arg_tys),
        "stats::ppoints" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::dist" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::toeplitz" | "stats::toeplitz2" | "stats::polym" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "stats::diffinv" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::addmargins" | "stats::ftable" | "stats::xtabs" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "stats::.vcov.aliased" | "stats::estVar" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::medpolish" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::symnum" => Some(TypeState::matrix(PrimTy::Char, false)),
        "stats::smooth" | "stats::smoothEnds" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::cmdscale" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::aggregate" | "stats::aggregate.data.frame" => {
            Some(TypeState::matrix(PrimTy::Any, false))
        }
        "stats::expand.model.frame" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::read.ftable" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::aggregate.ts" => Some(ts_like_output_type(first_arg_type_state(arg_tys))),
        "stats::reshape" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::get_all_vars" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::tsSmooth" => Some(ts_like_output_type(first_arg_type_state(arg_tys))),
        "stats::ave" => Some(vectorized_first_arg_type(first_arg_type_state(arg_tys))),
        "stats::reorder" | "stats::relevel" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::terms.formula" | "stats::delete.response" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats::DF2formula"
        | "stats::power"
        | "stats::C"
        | "stats::Pair"
        | "stats::preplot"
        | "stats::profile"
        | "stats::ppr"
        | "stats::KalmanLike"
        | "stats::makeARIMA"
        | "stats::eff.aovlist"
        | "stats::stat.anova"
        | "stats::Gamma"
        | "stats::.checkMFClasses"
        | "stats::.getXlevels"
        | "stats::.lm.fit"
        | "stats::.MFclass"
        | "stats::.preformat.ts" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::model.response" | "stats::model.extract" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats::case.names" => Some(TypeState::vector(PrimTy::Char, false)),
        "stats::complete.cases" => Some(TypeState::vector(PrimTy::Logical, false)),
        "stats::replications" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::ls.print" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::.nknots.smspl" => Some(TypeState::scalar(PrimTy::Int, false)),
        "stats::fivenum" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::knots" | "stats::se.contrast" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::p.adjust.methods" => Some(TypeState::vector(PrimTy::Char, false)),
        "stats::sortedXyData" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::pairwise.table" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::SSasymp" | "stats::SSasympOff" | "stats::SSasympOrig" | "stats::SSbiexp"
        | "stats::SSfol" | "stats::SSfpl" | "stats::SSgompertz" | "stats::SSlogis"
        | "stats::SSmicmen" | "stats::SSweibull" => {
            vectorized_scalar_or_vector_double_type(arg_tys)
        }
        "stats::NLSstAsymptotic"
        | "stats::NLSstClosestX"
        | "stats::NLSstLfAsymptote"
        | "stats::NLSstRtAsymptote" => vectorized_scalar_or_vector_double_type(arg_tys),
        "stats::splinefunH" | "stats::SSD" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::selfStart" | "stats::deriv" | "stats::deriv3" => {
            Some(TypeState::scalar(PrimTy::Any, false))
        }
        "stats::D" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::numericDeriv" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::cov" | "stats::cor" | "stats::var" => scalar_or_matrix_double_type(arg_tys),
        "stats::cov.wt" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::cov2cor" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::mahalanobis" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::rWishart" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::r2dtable" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::dmultinom" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::rmultinom" => Some(TypeState::matrix(PrimTy::Int, false)),
        "stats::ts" | "stats::as.ts" | "stats::hasTsp" | "stats::window" | "stats::window<-"
        | "stats::lag" | "stats::tsp<-" => Some(ts_like_output_type(first_arg_type_state(arg_tys))),
        "stats::contrasts<-" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "stats::ts.intersect" | "stats::ts.union" => {
            Some(TypeState::matrix(first_numeric_prim(arg_tys), false))
        }
        "stats::frequency" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::is.ts" | "stats::is.mts" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "stats::tsp" | "stats::start" | "stats::end" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "stats::deltat" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::time" | "stats::cycle" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::embed" => Some(TypeState::matrix(first_numeric_prim(arg_tys), false)),
        "stats::weighted.mean" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::runmed" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::filter" => Some(ts_like_output_type(first_arg_type_state(arg_tys))),
        "stats::spec.taper" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "stats::arima0.diag"
        | "stats::cpgram"
        | "stats::plclust"
        | "stats::ts.plot"
        | "stats::write.ftable" => Some(TypeState::null()),
        "stats::stepfun" | "stats::as.stepfun" => Some(TypeState::scalar(PrimTy::Any, false)),
        "stats::is.stepfun" | "stats::is.leaf" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "stats::plot.ecdf" | "stats::plot.ts" | "stats::screeplot" => Some(TypeState::null()),
        "stats::plot.stepfun" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::dendrapply" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::order.dendrogram" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::as.dist" | "stats::cophenetic" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::as.hclust" | "stats::as.dendrogram" | "stats::rect.hclust" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats::cutree" => Some(TypeState::vector(PrimTy::Int, false)),
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
        | "stats::supsmu"
        | "stats::aov"
        | "stats::manova"
        | "stats::alias"
        | "stats::model.tables"
        | "stats::factanal"
        | "stats::heatmap"
        | "stats::loglin"
        | "stats::step"
        | "stats::optim"
        | "stats::optimize"
        | "stats::optimise"
        | "stats::nlm"
        | "stats::nlminb"
        | "stats::constrOptim"
        | "stats::uniroot"
        | "stats::integrate"
        | "stats::HoltWinters"
        | "stats::StructTS"
        | "stats::KalmanForecast"
        | "stats::KalmanRun"
        | "stats::KalmanSmooth"
        | "stats::arima"
        | "stats::arima0"
        | "stats::ar"
        | "stats::ar.yw"
        | "stats::ar.mle"
        | "stats::ar.burg"
        | "stats::ar.ols"
        | "stats::spec.ar"
        | "stats::kernel"
        | "stats::nls"
        | "stats::nls.control"
        | "stats::kmeans"
        | "stats::hclust"
        | "stats::acf"
        | "stats::pacf"
        | "stats::ccf"
        | "stats::PP.test"
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
        | "stats::termplot"
        | "stats::pairwise.t.test"
        | "stats::pairwise.wilcox.test"
        | "stats::pairwise.prop.test"
        | "stats::factor.scope"
        | "stats::dummy.coef"
        | "stats::dummy.coef.lm" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::add1" | "stats::drop1" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::extractAIC" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::add.scope" | "stats::drop.scope" => Some(TypeState::vector(PrimTy::Char, false)),
        "stats::effects" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::setNames" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "stats::printCoefmat" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::optimHess" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::getInitial" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::is.tskernel" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "stats::df.kernel" | "stats::bandwidth.kernel" => {
            Some(TypeState::scalar(PrimTy::Double, false))
        }
        "stats::kernapply" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::arima.sim" | "stats::ARMAacf" | "stats::ARMAtoMA" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "stats::convolve" | "stats::fft" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::mvfft" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::nextn" => Some(TypeState::scalar(PrimTy::Int, false)),
        "stats::tsdiag" => Some(TypeState::null()),
        "stats::TukeyHSD" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::proj" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::approxfun" | "stats::splinefun" => Some(TypeState::scalar(PrimTy::Any, false)),
        "stats::ecdf" => Some(TypeState::scalar(PrimTy::Any, false)),
        "methods::isClass"
        | "methods::isGeneric"
        | "methods::hasMethod"
        | "methods::existsMethod"
        | "methods::existsFunction"
        | "methods::hasLoadAction"
        | "methods::hasArg"
        | "methods::hasMethods"
        | "methods::isGroup"
        | "methods::isGrammarSymbol"
        | "methods::isRematched"
        | "methods::isXS3Class"
        | "methods::is"
        | "methods::validObject"
        | "methods::isVirtualClass"
        | "methods::isClassUnion"
        | "methods::isSealedClass"
        | "methods::isSealedMethod"
        | "methods::isClassDef"
        | "methods::testVirtual"
        | "methods::canCoerce" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "methods::slotNames"
        | "methods::getClasses"
        | "methods::getSlots"
        | "methods::getGroupMembers"
        | "methods::formalArgs"
        | "methods::getAllSuperClasses" => Some(TypeState::vector(PrimTy::Char, false)),
        "methods::getPackageName" => Some(TypeState::scalar(PrimTy::Char, false)),
        "methods::getClass"
        | "methods::getClassDef"
        | "methods::findMethods"
        | "methods::findClass"
        | "methods::findUnique" => Some(TypeState::vector(PrimTy::Any, false)),
        "methods::classesToAM" => Some(TypeState::matrix(PrimTy::Double, false)),
        "methods::findMethodSignatures" => Some(TypeState::matrix(PrimTy::Char, false)),
        "methods::cacheMetaData" => Some(TypeState::null()),
        "methods::getLoadActions" => Some(TypeState::vector(PrimTy::Any, false)),
        "methods::extends" => Some(TypeState::vector(PrimTy::Char, false)),
        "methods::getGenerics" | "methods::getGroup" => Some(TypeState::vector(PrimTy::Any, false)),
        "methods::findMethod" | "methods::getMethodsForDispatch" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "methods::setGeneric" | "methods::setMethod" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "methods::showExtends"
        | "methods::addNextMethod"
        | "methods::makeExtends"
        | "methods::setAs"
        | "methods::resetClass"
        | "methods::assignMethodsMetaData"
        | "methods::signature"
        | "methods::setReplaceMethod"
        | "methods::removeClass"
        | "methods::implicitGeneric"
        | "methods::el"
        | "methods::callGeneric"
        | "methods::showClass"
        | "methods::Complex"
        | "methods::evalSource"
        | "methods::coerce<-"
        | "methods::kronecker"
        | "methods::resetGeneric"
        | "methods::matrixOps"
        | "methods::possibleExtends"
        | "methods::missingArg"
        | "methods::Quote"
        | "methods::registerImplicitGenerics"
        | "methods::initRefFields"
        | "methods::insertMethod"
        | "methods::externalRefMethod"
        | "methods::fixPre1.8"
        | "methods::checkAtAssignment"
        | "methods::finalDefaultMethod"
        | "methods::completeClassDefinition"
        | "methods::callNextMethod"
        | "methods::selectSuperClasses"
        | "methods::removeMethods"
        | "methods::evalOnLoad"
        | "methods::cbind2"
        | "methods::setClassUnion"
        | "methods::initialize"
        | "methods::Summary"
        | "methods::representation"
        | "methods::method.skeleton"
        | "methods::setRefClass"
        | "methods::Math2"
        | "methods::getMethods"
        | "methods::S3Part<-"
        | "methods::Logic"
        | "methods::matchSignature"
        | "methods::methodsPackageMetaName"
        | "methods::defaultDumpName"
        | "methods::substituteDirect"
        | "methods::packageSlot"
        | "methods::as<-"
        | "methods::removeMethod"
        | "methods::MethodsListSelect"
        | "methods::S3Part"
        | "methods::checkSlotAssignment"
        | "methods::classMetaName"
        | "methods::slotsFromS3"
        | "methods::promptMethods"
        | "methods::insertClassMethods"
        | "methods::packageSlot<-"
        | "methods::languageEl<-" => Some(TypeState::vector(PrimTy::Any, false)),
        "methods::.slotNames" => Some(TypeState::vector(PrimTy::Char, false)),
        "methods::.hasSlot" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "methods::validSlotNames" | "methods::inheritedSlotNames" | "methods::allNames" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "methods::classLabel" | "methods::className" | "methods::setPackageName" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "methods::.__C__signature"
        | "methods::.__C__packageInfo"
        | "methods::.__C__MethodSelectionReport"
        | "methods::.__C__uninitializedField"
        | "methods::.__C__ordered"
        | "methods::.__C__oldClass"
        | "methods::.__C__groupGenericFunction"
        | "methods::.__C__standardGeneric"
        | "methods::.__C__nonstandardGroupGenericFunction"
        | "methods::.__C__S3"
        | "methods::.__C__array"
        | "methods::.__C__S4"
        | "methods::.__C__SuperClassMethod"
        | "methods::.__C__aov"
        | "methods::.__C__integrate"
        | "methods::.__C__listOfMethods"
        | "methods::.__C__ClassUnionRepresentation"
        | "methods::.__C__refObject"
        | "methods::.__C__.Other"
        | "methods::.__C__classPrototypeDef"
        | "methods::.__C__ts"
        | "methods::.__C__table"
        | "methods::.__C__double"
        | "methods::.__C__environment"
        | "methods::.__C__Date"
        | "methods::.__C__character"
        | "methods::.__C__LinearMethodsList"
        | "methods::.__C__structure"
        | "methods::.__C__lm"
        | "methods::.__C__dump.frames"
        | "methods::.__C__density"
        | "methods::.__C__vector"
        | "methods::.__C__repeat"
        | "methods::.__C__className"
        | "methods::.__C__name"
        | "methods::.__C__ObjectsWithPackage"
        | "methods::.__C__glm.null"
        | "methods::.__C__defaultBindingFunction"
        | "methods::.__C__("
        | "methods::.__C__special"
        | "methods::.__C__SealedMethodDefinition"
        | "methods::.__C__list"
        | "methods::.__C__NULL"
        | "methods::.__C__.environment"
        | "methods::.__C__genericFunctionWithTrace"
        | "methods::.__C__anova"
        | "methods::.__C__socket"
        | "methods::.__C__refGeneratorSlot"
        | "methods::.__C__integer"
        | "methods::.__C__packageIQR"
        | "methods::.__C__envRefClass"
        | "methods::.__C__complex"
        | "methods::.__C__classRepresentation"
        | "methods::.__C__libraryIQR"
        | "methods::.__C__OptionalFunction"
        | "methods::.__C__missing"
        | "methods::.__C__refMethodDef"
        | "methods::.__C__genericFunction"
        | "methods::.__C__classGeneratorFunction"
        | "methods::.__C__raw"
        | "methods::.__C__mlm"
        | "methods::.__C__POSIXct"
        | "methods::.__C__{"
        | "methods::.__C__groupGenericFunctionWithTrace"
        | "methods::.__C__data.frameRowLabels"
        | "methods::.__C__activeBindingFunction"
        | "methods::.__C__externalptr"
        | "methods::.__C__.name"
        | "methods::.__C__recordedplot"
        | "methods::.__C__localRefClass"
        | "methods::.__C__POSIXt"
        | "methods::.__C__summary.table"
        | "methods::.__C__language"
        | "methods::.__C__refClass"
        | "methods::.__C__numeric"
        | "methods::.__C__derivedDefaultMethodWithTrace"
        | "methods::.__C__externalRefMethod"
        | "methods::.__C__MethodDefinitionWithTrace"
        | "methods::.__C__maov"
        | "methods::.__C__standardGenericWithTrace"
        | "methods::.__C__.NULL"
        | "methods::.__C__PossibleMethod"
        | "methods::.__C__functionWithTrace"
        | "methods::.__C__factor"
        | "methods::.__C__sourceEnvironment"
        | "methods::.__C__MethodDefinition"
        | "methods::.__C__mts"
        | "methods::.__C__mtable"
        | "methods::.__C__data.frame"
        | "methods::.__C__if"
        | "methods::.__C__optionalMethod"
        | "methods::.__C__ANY"
        | "methods::.__C__refClassRepresentation"
        | "methods::.__C__conditionalExtension"
        | "methods::.__C__traceable"
        | "methods::.__C__MethodWithNextWithTrace"
        | "methods::.__C__anova.glm.null"
        | "methods::.__C__.externalptr"
        | "methods::.__C__matrix"
        | "methods::.__C__hsearch"
        | "methods::.__C__function"
        | "methods::.__C__POSIXlt"
        | "methods::.__C__logical"
        | "methods::.__C__nonstandardGenericWithTrace"
        | "methods::.__C__summaryDefault"
        | "methods::.__C__derivedDefaultMethod"
        | "methods::.__C__nonstandardGeneric"
        | "methods::.__C__glm"
        | "methods::.__C__nonstandardGenericFunction"
        | "methods::.__C__refObjectGenerator"
        | "methods::.__C__builtin"
        | "methods::.__C__for"
        | "methods::.__C__internalDispatchMethod"
        | "methods::.__C__anova.glm"
        | "methods::.__C__<-"
        | "methods::.__C__nonStructure"
        | "methods::.__C__call"
        | "methods::.__C__MethodWithNext"
        | "methods::.__C__rle"
        | "methods::.__C__logLik"
        | "methods::.__C__namedList"
        | "methods::.__C__formula"
        | "methods::.__C__while"
        | "methods::.__C__expression"
        | "methods::.__C__refMethodDefWithTrace"
        | "methods::.__C__VIRTUAL"
        | "methods::.__C__SClassExtension" => Some(TypeState::vector(PrimTy::Any, false)),
        "methods::.EmptyPrimitiveSkeletons"
        | "methods::.OldClassesList"
        | "methods::.S4methods"
        | "methods::.ShortPrimitiveSkeletons"
        | "methods::.classEnv"
        | "methods::.doTracePrint"
        | "methods::.selectSuperClasses"
        | "methods::.untracedFunction"
        | "methods::.valueClassTest"
        | "methods::.__T__Logic:base"
        | "methods::.__T__loadMethod:methods"
        | "methods::.__T__[<-:base"
        | "methods::.__T__coerce:methods"
        | "methods::.__T__matrixOps:base"
        | "methods::.__T__show:methods"
        | "methods::.__T__body<-:base"
        | "methods::.__T__cbind2:methods"
        | "methods::.__T__Arith:base"
        | "methods::.__T__[[<-:base"
        | "methods::.__T__$:base"
        | "methods::.__T__Compare:methods"
        | "methods::.__T__Math2:methods"
        | "methods::.__T__slotsFromS3:methods"
        | "methods::.__T__Complex:base"
        | "methods::.__T__coerce<-:methods"
        | "methods::.__T__kronecker:base"
        | "methods::.__T__rbind2:methods"
        | "methods::.__T__initialize:methods"
        | "methods::.__T__$<-:base"
        | "methods::.__T__addNextMethod:methods"
        | "methods::.__T__Ops:base"
        | "methods::.__T__Math:base"
        | "methods::.__T__Summary:base"
        | "methods::.__T__[:base"
        | "methods::emptyMethodsList"
        | "methods::listFromMethods"
        | "methods::metaNameUndo"
        | "methods::Math"
        | "methods::makePrototypeFromClassDef"
        | "methods::reconcilePropertiesAndPrototype"
        | "methods::doPrimitiveMethod"
        | "methods::SignatureMethod"
        | "methods::setIs"
        | "methods::balanceMethodsList"
        | "methods::setLoadActions"
        | "methods::setDataPart"
        | "methods::setGroupGeneric"
        | "methods::S3Class"
        | "methods::dumpMethods"
        | "methods::sigToEnv"
        | "methods::setGenericImplicit"
        | "methods::sealClass"
        | "methods::makeStandardGeneric"
        | "methods::loadMethod"
        | "methods::prohibitGeneric"
        | "methods::substituteFunctionArgs"
        | "methods::completeExtends"
        | "methods::assignClassDef"
        | "methods::methodSignatureMatrix"
        | "methods::body<-"
        | "methods::prototype"
        | "methods::requireMethods"
        | "methods::slot<-"
        | "methods::setOldClass"
        | "methods::setPrimitiveMethods"
        | "methods::rematchDefinition"
        | "methods::MethodAddCoerce"
        | "methods::evalqOnLoad"
        | "methods::insertSource"
        | "methods::multipleClasses"
        | "methods::empty.dump"
        | "methods::el<-"
        | "methods::elNamed<-"
        | "methods::newEmptyObject"
        | "methods::Arith"
        | "methods::mergeMethods"
        | "methods::MethodsList"
        | "methods::asMethodDefinition"
        | "methods::languageEl"
        | "methods::Ops"
        | "methods::completeSubclasses"
        | "methods::cacheGenericsMetaData"
        | "methods::tryNew"
        | "methods::showDefault"
        | "methods::Compare"
        | "methods::makeClassRepresentation"
        | "methods::promptClass"
        | "methods::newClassRepresentation"
        | "methods::removeGeneric"
        | "methods::S3Class<-"
        | "methods::rbind2"
        | "methods::setValidity"
        | "methods::functionBody"
        | "methods::dumpMethod"
        | "methods::elNamed"
        | "methods::generic.skeleton"
        | "methods::makeGeneric"
        | "methods::coerce"
        | "methods::initFieldArgs"
        | "methods::unRematchDefinition"
        | "methods::defaultPrototype"
        | "methods::showMethods"
        | "methods::as"
        | "methods::conformMethod"
        | "methods::makeMethodsList"
        | "methods::newBasic"
        | "methods::functionBody<-"
        | "methods::setLoadAction"
        | "methods::superClassDepth"
        | "methods::getMethodsMetaData" => Some(TypeState::vector(PrimTy::Any, false)),
        "methods::getGeneric"
        | "methods::cacheMethod"
        | "methods::getFunction"
        | "methods::getRefClass"
        | "methods::findFunction"
        | "methods::getDataPart"
        | "methods::selectMethod"
        | "methods::new"
        | "methods::slot"
        | "methods::getMethod"
        | "methods::getValidity"
        | "methods::testInheritedMethods"
        | "methods::standardGeneric"
        | "methods::setClass" => Some(TypeState::unknown()),
        "methods::show" => Some(TypeState::null()),
        "compiler::enableJIT" => Some(TypeState::scalar(PrimTy::Int, false)),
        "compiler::compilePKGS" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "compiler::getCompilerOption" => Some(TypeState::scalar(PrimTy::Any, false)),
        "compiler::setCompilerOptions" | "compiler::compile" | "compiler::disassemble" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "compiler::cmpfile" | "compiler::loadcmp" => Some(TypeState::null()),
        "compiler::cmpfun" => Some(TypeState::unknown()),
        callee if callee.starts_with("dplyr::") => Some(TypeState::unknown()),
        callee if callee.starts_with("readr::") || callee.starts_with("tidyr::") => {
            Some(TypeState::unknown())
        }
        "utils::head" | "utils::tail" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "utils::packageVersion"
        | "utils::packageDescription"
        | "utils::sessionInfo"
        | "utils::citation"
        | "utils::person"
        | "utils::as.person"
        | "utils::as.personList"
        | "utils::getAnywhere" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::as.roman" => {
            let first = first_arg_type_state(arg_tys);
            if matches!(first.shape, ShapeTy::Scalar) {
                Some(TypeState::scalar(PrimTy::Int, false))
            } else {
                Some(TypeState::vector(PrimTy::Int, false).with_len(first.len_sym))
            }
        }
        "utils::hasName" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "utils::strcapture" => Some(
            TypeState::matrix(PrimTy::Any, false)
                .with_len(arg_tys.get(1).copied().and_then(|ty| ty.len_sym)),
        ),
        "utils::contrib.url" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::localeToCharset" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::charClass" => Some(TypeState::vector(PrimTy::Logical, false)),
        "utils::findMatches" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::fileSnapshot" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::apropos" | "utils::find" | "utils::methods" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "utils::help.search" | "utils::data" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::argsAnywhere" => Some(TypeState::unknown()),
        "utils::compareVersion" => Some(TypeState::scalar(PrimTy::Double, false)),
        "utils::capture.output" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::file_test" => {
            let probe = arg_tys.get(1).copied().unwrap_or(TypeState::unknown());
            if matches!(probe.shape, ShapeTy::Scalar) {
                Some(TypeState::scalar(PrimTy::Logical, false))
            } else {
                Some(TypeState::vector(PrimTy::Logical, false).with_len(probe.len_sym))
            }
        }
        "utils::URLencode" | "utils::URLdecode" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "utils::head.matrix" | "utils::tail.matrix" => Some(preserved_first_arg_type_without_len(
            first_arg_type_state(arg_tys),
        )),
        "utils::available.packages" => Some(TypeState::matrix(PrimTy::Char, false)),
        "utils::stack" | "utils::unstack" => Some(TypeState::matrix(PrimTy::Any, false)),
        "utils::strOptions" | "utils::txtProgressBar" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "utils::toBibtex" | "utils::toLatex" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::getTxtProgressBar" | "utils::setTxtProgressBar" => {
            Some(TypeState::scalar(PrimTy::Double, false))
        }
        "utils::modifyList"
        | "utils::relist"
        | "utils::as.relistable"
        | "utils::personList"
        | "utils::warnErrList"
        | "utils::readCitationFile"
        | "utils::bibentry"
        | "utils::citEntry"
        | "utils::citHeader"
        | "utils::citFooter" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::getSrcref" | "utils::getFromNamespace" | "utils::getS3method" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "utils::getParseData" => Some(TypeState::matrix(PrimTy::Any, false)),
        "utils::getParseText" | "utils::globalVariables" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "utils::getSrcFilename" | "utils::getSrcDirectory" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "utils::getSrcLocation" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::hashtab" | "utils::gethash" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::sethash" => Some(TypeState::unknown()),
        "utils::remhash" | "utils::is.hashtab" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "utils::clrhash" | "utils::maphash" => Some(TypeState::null()),
        "utils::numhash" => Some(TypeState::scalar(PrimTy::Int, false)),
        "utils::typhash" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::asDateBuilt" => Some(TypeState::scalar(PrimTy::Double, false)),
        "utils::findLineNum" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::getCRANmirrors" => Some(TypeState::matrix(PrimTy::Char, false)),
        "utils::findCRANmirror" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::package.skeleton" | "utils::unzip" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::zip" => Some(TypeState::scalar(PrimTy::Int, false)),
        "utils::limitedLabels"
        | "utils::formatOL"
        | "utils::formatUL"
        | "utils::ls.str"
        | "utils::lsf.str" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::news" => Some(TypeState::null()),
        "utils::vignette" | "utils::hsearch_db" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::hsearch_db_concepts" | "utils::hsearch_db_keywords" => {
            Some(TypeState::matrix(PrimTy::Any, false))
        }
        "utils::browseEnv"
        | "utils::browseURL"
        | "utils::browseVignettes"
        | "utils::bug.report"
        | "utils::checkCRAN"
        | "utils::chooseBioCmirror"
        | "utils::chooseCRANmirror"
        | "utils::create.post"
        | "utils::data.entry"
        | "utils::dataentry"
        | "utils::debugcall"
        | "utils::debugger"
        | "utils::demo"
        | "utils::dump.frames"
        | "utils::edit"
        | "utils::emacs"
        | "utils::example"
        | "utils::file.edit"
        | "utils::fix"
        | "utils::fixInNamespace"
        | "utils::flush.console"
        | "utils::help.request"
        | "utils::help.start"
        | "utils::page"
        | "utils::pico"
        | "utils::process.events"
        | "utils::prompt"
        | "utils::promptData"
        | "utils::promptImport"
        | "utils::promptPackage"
        | "utils::recover"
        | "utils::removeSource"
        | "utils::RShowDoc"
        | "utils::RSiteSearch"
        | "utils::rtags"
        | "utils::setBreakpoint"
        | "utils::suppressForeignCheck"
        | "utils::undebugcall"
        | "utils::url.show"
        | "utils::vi"
        | "utils::View"
        | "utils::xedit"
        | "utils::xemacs" => Some(TypeState::null()),
        "utils::tar" => Some(TypeState::scalar(PrimTy::Int, false)),
        "utils::untar" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::timestamp" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::Rprof" | "utils::Rprofmem" => Some(TypeState::null()),
        "utils::summaryRprof" | "utils::setRepositories" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "utils::?"
        | "utils::.AtNames"
        | "utils::.DollarNames"
        | "utils::cite"
        | "utils::citeNatbib"
        | "utils::help"
        | "utils::read.socket"
        | "utils::RweaveChunkPrefix" => Some(TypeState::vector(PrimTy::Char, false)),
        "utils::.romans" => Some(TypeState::vector(PrimTy::Int, false)),
        "utils::askYesNo" | "utils::isS3method" | "utils::isS3stdGeneric" => {
            Some(TypeState::scalar(PrimTy::Logical, false))
        }
        "utils::rc.settings" => Some(TypeState::vector(PrimTy::Logical, false)),
        "utils::download.file" => Some(TypeState::scalar(PrimTy::Int, false)),
        "utils::alarm" | "utils::rc.getOption" => Some(TypeState::scalar(PrimTy::Any, false)),
        "utils::close.socket"
        | "utils::history"
        | "utils::loadhistory"
        | "utils::savehistory"
        | "utils::write.socket" => Some(TypeState::null()),
        "utils::.checkHT"
        | "utils::.RtangleCodeLabel"
        | "utils::.S3methods"
        | "utils::aregexec"
        | "utils::aspell"
        | "utils::aspell_package_C_files"
        | "utils::aspell_package_R_files"
        | "utils::aspell_package_Rd_files"
        | "utils::aspell_package_vignettes"
        | "utils::aspell_write_personal_dictionary_file"
        | "utils::assignInMyNamespace"
        | "utils::assignInNamespace"
        | "utils::changedFiles"
        | "utils::de"
        | "utils::de.ncols"
        | "utils::de.restore"
        | "utils::de.setup"
        | "utils::download.packages"
        | "utils::install.packages"
        | "utils::make.packages.html"
        | "utils::make.socket"
        | "utils::makeRweaveLatexCodeRunner"
        | "utils::mirror2html"
        | "utils::new.packages"
        | "utils::old.packages"
        | "utils::packageStatus"
        | "utils::rc.options"
        | "utils::rc.status"
        | "utils::remove.packages"
        | "utils::Rtangle"
        | "utils::RtangleFinish"
        | "utils::RtangleRuncode"
        | "utils::RtangleSetup"
        | "utils::RtangleWritedoc"
        | "utils::RweaveEvalWithOpt"
        | "utils::RweaveLatex"
        | "utils::RweaveLatexFinish"
        | "utils::RweaveLatexOptions"
        | "utils::RweaveLatexSetup"
        | "utils::RweaveLatexWritedoc"
        | "utils::RweaveTryStop"
        | "utils::Stangle"
        | "utils::Sweave"
        | "utils::SweaveHooks"
        | "utils::SweaveSyntaxLatex"
        | "utils::SweaveSyntaxNoweb"
        | "utils::SweaveSyntConv"
        | "utils::update.packages"
        | "utils::upgrade" => Some(TypeState::vector(PrimTy::Any, false)),
        "utils::is.relistable" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "utils::packageName" | "utils::osVersion" | "utils::nsl" | "utils::select.list" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "utils::menu" => Some(TypeState::scalar(PrimTy::Int, false)),
        "utils::glob2rx" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::installed.packages" => Some(TypeState::matrix(PrimTy::Char, false)),
        "utils::maintainer" => Some(TypeState::scalar(PrimTy::Char, false)),
        "utils::packageDate"
        | "utils::object.size"
        | "utils::memory.size"
        | "utils::memory.limit" => Some(TypeState::scalar(PrimTy::Double, false)),
        "utils::read.csv"
        | "utils::read.csv2"
        | "utils::read.table"
        | "utils::read.delim"
        | "utils::read.fwf"
        | "utils::read.delim2"
        | "utils::read.DIF"
        | "utils::read.fortran" => Some(TypeState::matrix(PrimTy::Any, false)),
        "utils::write.csv" | "utils::write.csv2" | "utils::write.table" | "utils::str" => {
            Some(TypeState::null())
        }
        "utils::count.fields" => Some(TypeState::vector(PrimTy::Int, false)),
        "utils::adist" => Some(TypeState::matrix(PrimTy::Double, false)),
        "utils::combn" => {
            let first = first_arg_type_state(arg_tys);
            let prim = match first.shape {
                ShapeTy::Matrix | ShapeTy::Vector | ShapeTy::Scalar => first.prim,
                ShapeTy::Unknown => PrimTy::Any,
            };
            Some(TypeState::matrix(prim, false))
        }
        "utils::type.convert" => Some(vectorized_first_arg_type(first_arg_type_state(arg_tys))),
        "tools::toTitleCase" | "tools::file_path_as_absolute" | "tools::R_user_dir" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "tools::md5sum" | "tools::sha256sum" => Some(TypeState::vector(PrimTy::Char, false)),
        "tools::file_ext" | "tools::file_path_sans_ext" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "tools::list_files_with_exts" | "tools::list_files_with_type" | "tools::dependsOnPkgs" => {
            Some(TypeState::vector(PrimTy::Char, false))
        }
        "tools::getVignetteInfo" => Some(TypeState::matrix(PrimTy::Char, false)),
        "tools::pkgVignettes" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::delimMatch" => Some(TypeState::scalar(PrimTy::Int, false)),
        "tools::parse_URI_reference" => Some(TypeState::matrix(PrimTy::Char, false)),
        "tools::encoded_text_to_latex" => {
            Some(char_like_first_arg_type(first_arg_type_state(arg_tys)))
        }
        "tools::parse_Rd" | "tools::Rd2txt_options" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::Rd2HTML" | "tools::Rd2latex" | "tools::Rd2ex" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        "tools::Rd2txt"
        | "tools::RdTextFilter"
        | "tools::checkRd"
        | "tools::showNonASCII"
        | "tools::showNonASCIIfile" => Some(TypeState::vector(PrimTy::Char, false)),
        "tools::find_gs_cmd"
        | "tools::findHTMLlinks"
        | "tools::makevars_site"
        | "tools::makevars_user"
        | "tools::HTMLheader"
        | "tools::SweaveTeXFilter"
        | "tools::toHTML"
        | "tools::toRd"
        | "tools::charset_to_Unicode" => Some(TypeState::vector(PrimTy::Char, false)),
        "tools::Rdindex" => Some(TypeState::null()),
        "tools::read.00Index" => Some(TypeState::matrix(PrimTy::Char, false)),
        "tools::parseLatex" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::getBibstyle" => Some(TypeState::scalar(PrimTy::Char, false)),
        "tools::deparseLatex" => Some(TypeState::scalar(PrimTy::Char, false)),
        "tools::latexToUtf8" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::SIGCHLD" | "tools::SIGCONT" | "tools::SIGHUP" | "tools::SIGINT"
        | "tools::SIGKILL" | "tools::SIGQUIT" | "tools::SIGSTOP" | "tools::SIGTERM"
        | "tools::SIGTSTP" | "tools::SIGUSR1" | "tools::SIGUSR2" => {
            Some(TypeState::scalar(PrimTy::Int, false))
        }
        "tools::assertCondition"
        | "tools::assertError"
        | "tools::assertWarning"
        | "tools::add_datalist"
        | "tools::buildVignette"
        | "tools::buildVignettes"
        | "tools::compactPDF"
        | "tools::installFoundDepends"
        | "tools::make_translations_pkg"
        | "tools::package_native_routine_registration_skeleton"
        | "tools::pskill"
        | "tools::psnice"
        | "tools::resaveRdaFiles"
        | "tools::startDynamicHelp"
        | "tools::testInstalledBasic"
        | "tools::testInstalledPackage"
        | "tools::testInstalledPackages"
        | "tools::texi2dvi"
        | "tools::texi2pdf"
        | "tools::update_PACKAGES"
        | "tools::update_pkg_po"
        | "tools::write_PACKAGES"
        | "tools::xgettext"
        | "tools::xgettext2pot"
        | "tools::xngettext" => Some(TypeState::null()),
        "tools::Adobe_glyphs" => Some(TypeState::matrix(PrimTy::Char, false)),
        "tools::.print.via.format"
        | "tools::analyze_license"
        | "tools::as.Rconcordance"
        | "tools::bibstyle"
        | "tools::check_package_dois"
        | "tools::check_package_urls"
        | "tools::check_packages_in_dir"
        | "tools::check_packages_in_dir_changes"
        | "tools::check_packages_in_dir_details"
        | "tools::checkDocFiles"
        | "tools::checkDocStyle"
        | "tools::checkFF"
        | "tools::checkMD5sums"
        | "tools::checkPoFile"
        | "tools::checkPoFiles"
        | "tools::checkRdaFiles"
        | "tools::checkRdContents"
        | "tools::checkReplaceFuns"
        | "tools::checkS3methods"
        | "tools::checkTnF"
        | "tools::checkVignettes"
        | "tools::codoc"
        | "tools::codocClasses"
        | "tools::codocData"
        | "tools::followConcordance"
        | "tools::getDepList"
        | "tools::langElts"
        | "tools::loadPkgRdMacros"
        | "tools::loadRdMacros"
        | "tools::matchConcordance"
        | "tools::nonS3methods"
        | "tools::package.dependencies"
        | "tools::pkg2HTML"
        | "tools::pkgDepends"
        | "tools::R"
        | "tools::Rcmd"
        | "tools::Rdiff"
        | "tools::summarize_check_packages_in_dir_depends"
        | "tools::summarize_check_packages_in_dir_results"
        | "tools::summarize_check_packages_in_dir_timings"
        | "tools::undoc"
        | "tools::vignetteDepends"
        | "tools::vignetteEngine"
        | "tools::vignetteInfo" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::standard_package_names"
        | "tools::base_aliases_db"
        | "tools::base_rdxrefs_db"
        | "tools::CRAN_aliases_db"
        | "tools::CRAN_archive_db"
        | "tools::CRAN_rdxrefs_db" => Some(TypeState::vector(PrimTy::Any, false)),
        "tools::CRAN_package_db"
        | "tools::CRAN_authors_db"
        | "tools::CRAN_current_db"
        | "tools::CRAN_check_results"
        | "tools::CRAN_check_details"
        | "tools::CRAN_check_issues" => Some(TypeState::matrix(PrimTy::Any, false)),
        "tools::summarize_CRAN_check_status" => Some(TypeState::vector(PrimTy::Char, false)),
        "tools::package_dependencies" | "tools::Rd_db" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "parallel::detectCores" => Some(TypeState::scalar(PrimTy::Int, false)),
        "parallel::makeCluster"
        | "parallel::makeForkCluster"
        | "parallel::makePSOCKcluster"
        | "parallel::parLapply"
        | "parallel::parLapplyLB"
        | "parallel::clusterEvalQ"
        | "parallel::clusterMap"
        | "parallel::clusterApply"
        | "parallel::clusterCall"
        | "parallel::mclapply"
        | "parallel::mcMap"
        | "parallel::clusterSplit"
        | "parallel::splitIndices"
        | "parallel::getDefaultCluster"
        | "parallel::recvData"
        | "parallel::recvOneData"
        | "parallel::clusterApplyLB" => Some(TypeState::vector(PrimTy::Any, false)),
        "parallel::parSapply"
        | "parallel::parSapplyLB"
        | "parallel::parApply"
        | "parallel::parCapply"
        | "parallel::parRapply"
        | "parallel::pvec"
        | "parallel::mcmapply" => Some(TypeState::vector(PrimTy::Any, false)),
        "parallel::nextRNGStream" | "parallel::nextRNGSubStream" | "parallel::mcaffinity" => {
            Some(TypeState::vector(PrimTy::Int, false))
        }
        "parallel::mcparallel" | "parallel::mccollect" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "splines::bs" | "splines::ns" | "splines::splineDesign" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "splines::interpSpline"
        | "splines::periodicSpline"
        | "splines::backSpline"
        | "splines::xyVector" => Some(TypeState::vector(PrimTy::Any, false)),
        "splines::spline.des" | "splines::as.polySpline" | "splines::polySpline" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "splines::asVector" => Some(TypeState::vector(PrimTy::Double, false)),
        "splines::splineKnots" => Some(TypeState::vector(PrimTy::Double, false)),
        "splines::splineOrder" => Some(TypeState::scalar(PrimTy::Int, false)),
        "tcltk::tclObj" | "tcltk::as.tclObj" | "tcltk::tclVar" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "tcltk::tclvalue" => Some(TypeState::unknown()),
        "tcltk::addTclPath" | "tcltk::tclRequire" => Some(TypeState::unknown()),
        "tcltk::tclVersion" => Some(TypeState::scalar(PrimTy::Char, false)),
        "tcltk::tkProgressBar" => Some(TypeState::vector(PrimTy::Any, false)),
        "tcltk::getTkProgressBar" | "tcltk::setTkProgressBar" => {
            Some(TypeState::scalar(PrimTy::Double, false))
        }
        "tcltk::is.tclObj" | "tcltk::is.tkwin" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "tcltk::tclfile.dir" | "tcltk::tclfile.tail" => {
            Some(TypeState::scalar(PrimTy::Char, false))
        }
        callee
            if callee.starts_with("tcltk::tk")
                || callee.starts_with("tcltk::ttk")
                || callee.starts_with("tcltk::tcl")
                || callee.starts_with("tcltk::.Tcl")
                || callee.starts_with("tcltk::.Tk") =>
        {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats4::mle"
        | "stats4::update"
        | "stats4::summary"
        | "stats4::profile"
        | "stats4::plot"
        | "stats4::.__C__mle"
        | "stats4::.__C__profile.mle"
        | "stats4::.__C__summary.mle"
        | "stats4::.__T__AIC:stats"
        | "stats4::.__T__BIC:stats"
        | "stats4::.__T__coef:stats"
        | "stats4::.__T__confint:stats"
        | "stats4::.__T__logLik:stats"
        | "stats4::.__T__nobs:stats"
        | "stats4::.__T__plot:base"
        | "stats4::.__T__profile:stats"
        | "stats4::.__T__show:methods"
        | "stats4::.__T__summary:base"
        | "stats4::.__T__update:stats"
        | "stats4::.__T__vcov:stats" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats4::coef" | "stats4::confint" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats4::vcov" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats4::logLik" | "stats4::AIC" | "stats4::BIC" => {
            Some(TypeState::scalar(PrimTy::Double, false))
        }
        "stats4::show" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats4::nobs" => Some(TypeState::scalar(PrimTy::Int, false)),
        "parallel::stopCluster"
        | "parallel::clusterExport"
        | "parallel::closeNode"
        | "parallel::clusterSetRNGStream"
        | "parallel::mc.reset.stream"
        | "parallel::sendData"
        | "parallel::registerClusterType"
        | "parallel::setDefaultCluster" => Some(TypeState::null()),
        "stats::median"
        | "stats::median.default"
        | "stats::sd"
        | "stats::AIC"
        | "stats::BIC"
        | "stats::logLik"
        | "stats::deviance"
        | "stats::sigma" => Some(TypeState::scalar(PrimTy::Double, false)),
        "stats::nobs" | "stats::df.residual" => Some(TypeState::scalar(PrimTy::Int, false)),
        "stats::quantile" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::predict"
        | "stats::predict.lm"
        | "stats::predict.glm"
        | "stats::coef"
        | "stats::coefficients"
        | "stats::fitted"
        | "stats::fitted.values"
        | "stats::resid"
        | "stats::residuals"
        | "stats::residuals.lm"
        | "stats::residuals.glm"
        | "stats::hatvalues"
        | "stats::hat"
        | "stats::cooks.distance"
        | "stats::covratio"
        | "stats::dffits"
        | "stats::rstandard"
        | "stats::rstudent"
        | "stats::weighted.residuals" => Some(TypeState::vector(PrimTy::Double, false)),
        "stats::summary.lm"
        | "stats::summary.glm"
        | "stats::summary.aov"
        | "stats::summary.manova" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::summary.stepfun" => Some(TypeState::null()),
        "stats::isoreg" | "stats::line" | "stats::varimax" | "stats::promax" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats::asOneSidedFormula" => Some(TypeState::unknown()),
        "stats::variable.names" => Some(TypeState::null()),
        "stats::dfbeta" | "stats::dfbetas" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::influence" | "stats::influence.measures" | "stats::qr.influence" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats::vcov"
        | "stats::confint"
        | "stats::confint.lm"
        | "stats::confint.default"
        | "stats::model.matrix"
        | "stats::model.matrix.default"
        | "stats::model.matrix.lm" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::simulate" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::anova" => Some(TypeState::matrix(PrimTy::Any, false)),
        "stats::model.frame" | "stats::model.frame.default" => {
            Some(TypeState::matrix(PrimTy::Any, false))
        }
        "stats::glm.fit" | "stats::lm.fit" | "stats::lm.wfit" | "stats::lsfit"
        | "stats::ls.diag" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::loadings" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::makepredictcall" => Some(TypeState::scalar(PrimTy::Any, false)),
        "stats::na.contiguous" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "stats::na.action" => Some(TypeState::vector(PrimTy::Int, false)),
        "stats::napredict" | "stats::naresid" => Some(preserved_second_arg_type_without_len(
            second_arg_type_state(arg_tys),
        )),
        "stats::naprint" => Some(TypeState::scalar(PrimTy::Char, false)),
        "stats::weights" | "stats::model.weights" | "stats::model.offset" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "stats::offset" => Some(preserved_first_arg_type_without_len(first_arg_type_state(
            arg_tys,
        ))),
        "stats::na.omit" | "stats::na.exclude" | "stats::na.pass" | "stats::na.fail" => Some(
            preserved_first_arg_type_without_len(first_arg_type_state(arg_tys)),
        ),
        "stats::lm.influence" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::glm.control" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::is.empty.model" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "stats::getCall" => Some(TypeState::scalar(PrimTy::Any, false)),
        "stats::update" | "stats::update.default" | "stats::terms" | "stats::drop.terms" => {
            Some(TypeState::vector(PrimTy::Any, false))
        }
        "stats::update.formula" => Some(TypeState::unknown()),
        "grid::grid.newpage"
        | "grid::grid.draw"
        | "grid::grid.pack"
        | "grid::grid.place"
        | "grid::grid.polyline"
        | "grid::grid.raster"
        | "grid::grid.curve"
        | "grid::grid.bezier"
        | "grid::grid.path"
        | "grid::pushViewport"
        | "grid::popViewport" => Some(TypeState::null()),
        "grid::seekViewport" => Some(TypeState::scalar(PrimTy::Int, false)),
        "grid::nullGrob"
        | "grid::rectGrob"
        | "grid::circleGrob"
        | "grid::segmentsGrob"
        | "grid::pointsGrob"
        | "grid::rasterGrob"
        | "grid::bezierGrob"
        | "grid::pathGrob"
        | "grid::polygonGrob"
        | "grid::polylineGrob"
        | "grid::xsplineGrob"
        | "grid::frameGrob"
        | "grid::packGrob"
        | "grid::placeGrob"
        | "grid::roundrectGrob"
        | "grid::linesGrob"
        | "grid::curveGrob"
        | "grid::textGrob"
        | "grid::grobTree"
        | "grid::gList"
        | "grid::gpar"
        | "grid::viewport"
        | "grid::grid.layout"
        | "grid::grid.frame"
        | "grid::vpStack"
        | "grid::vpList"
        | "grid::dataViewport"
        | "grid::current.viewport"
        | "grid::upViewport"
        | "grid::grid.rect"
        | "grid::grid.text"
        | "grid::grid.circle"
        | "grid::grid.points"
        | "grid::grid.lines"
        | "grid::grid.segments"
        | "grid::grid.polygon" => Some(TypeState::vector(PrimTy::Any, false)),
        "grid::unit"
        | "grid::grobWidth"
        | "grid::grobHeight"
        | "grid::drawDetails"
        | "grid::grid.multipanel"
        | "grid::addGrob"
        | "grid::grobDescent"
        | "grid::grid.roundrect"
        | "grid::convertNative"
        | "grid::vpPath"
        | "grid::getGrob"
        | "grid::grid.grep"
        | "grid::applyEdits"
        | "grid::absolute.size"
        | "grid::explode"
        | "grid::gPath"
        | "grid::widthDetails"
        | "grid::current.transform"
        | "grid::descentDetails"
        | "grid::grid.stroke"
        | "grid::bezierPoints"
        | "grid::getNames"
        | "grid::convertUnit"
        | "grid::grid.show.layout"
        | "grid::is.grob"
        | "grid::grid.legend"
        | "grid::emptyCoords"
        | "grid::radialGradient"
        | "grid::groupShear"
        | "grid::stringDescent"
        | "grid::removeGrob"
        | "grid::grid.yaxis"
        | "grid::viewportScale"
        | "grid::grid.revert"
        | "grid::grid.grob"
        | "grid::arrow"
        | "grid::strokeGrob"
        | "grid::useRotate"
        | "grid::fillGrob"
        | "grid::grid.locator"
        | "grid::emptyGTreeCoords"
        | "grid::grid.pretty"
        | "grid::applyEdit"
        | "grid::fillStrokeGrob"
        | "grid::deviceLoc"
        | "grid::arcCurvature"
        | "grid::viewportTransform"
        | "grid::useGrob"
        | "grid::viewport.transform"
        | "grid::glyphGrob"
        | "grid::setChildren"
        | "grid::grobAscent"
        | "grid::unit.pmin"
        | "grid::grid.ls"
        | "grid::viewport.layout"
        | "grid::grid.reorder"
        | "grid::plotViewport"
        | "grid::moveToGrob"
        | "grid::preDrawDetails"
        | "grid::downViewport"
        | "grid::grid.add"
        | "grid::xsplinePoints"
        | "grid::unitType"
        | "grid::groupScale"
        | "grid::grid.xspline"
        | "grid::viewportRotate"
        | "grid::grid.plot.and.legend"
        | "grid::showGrob"
        | "grid::grid.get"
        | "grid::layout.widths"
        | "grid::grid.glyph"
        | "grid::grid.fillStroke"
        | "grid::isClosed"
        | "grid::grid.xaxis"
        | "grid::resolveRasterSize"
        | "grid::yDetails"
        | "grid::convertHeight"
        | "grid::defnRotate"
        | "grid::grid.gedit"
        | "grid::valid.just"
        | "grid::postDrawDetails"
        | "grid::convertX"
        | "grid::grid.record"
        | "grid::convertY"
        | "grid::layoutRegion"
        | "grid::grobPoints"
        | "grid::clipGrob"
        | "grid::convertWidth"
        | "grid::xDetails"
        | "grid::push.viewport"
        | "grid::pathListing"
        | "grid::as.mask"
        | "grid::resolveVJust"
        | "grid::get.gpar"
        | "grid::unit.pmax"
        | "grid::unit.psum"
        | "grid::current.vpPath"
        | "grid::ascentDetails"
        | "grid::grid.abline"
        | "grid::childNames"
        | "grid::grobPathListing"
        | "grid::delayGrob"
        | "grid::grid.move.to"
        | "grid::grid.convertHeight"
        | "grid::pattern"
        | "grid::grid.gget"
        | "grid::xaxisGrob"
        | "grid::editDetails"
        | "grid::grid.define"
        | "grid::viewportTranslate"
        | "grid::grid.DLapply"
        | "grid::grid.grill"
        | "grid::nestedListing"
        | "grid::as.path"
        | "grid::layout.heights"
        | "grid::unit.c"
        | "grid::grid.use"
        | "grid::grid.refresh"
        | "grid::resolveHJust"
        | "grid::emptyGrobCoords"
        | "grid::grobName"
        | "grid::editGrob"
        | "grid::grid.strip"
        | "grid::grid.clip"
        | "grid::arrowsGrob"
        | "grid::unit.rep"
        | "grid::grid.copy"
        | "grid::grid.fill"
        | "grid::stringAscent"
        | "grid::gridGTreeCoords"
        | "grid::legendGrob"
        | "grid::useScale"
        | "grid::grid.group"
        | "grid::recordGrob"
        | "grid::useTranslate"
        | "grid::grid.edit"
        | "grid::forceGrob"
        | "grid::grid.convertX"
        | "grid::isEmptyCoords"
        | "grid::grid.convertY"
        | "grid::grid.collection"
        | "grid::showViewport"
        | "grid::grid.set"
        | "grid::vpTree"
        | "grid::makeContent"
        | "grid::grid.display.list"
        | "grid::gTree"
        | "grid::gridGrobCoords"
        | "grid::groupGrob"
        | "grid::defineGrob"
        | "grid::groupFlip"
        | "grid::deviceDim"
        | "grid::current.rotation"
        | "grid::editViewport"
        | "grid::grid.grabExpr"
        | "grid::grob"
        | "grid::gridCoords"
        | "grid::yaxisGrob"
        | "grid::groupRotate"
        | "grid::grid.line.to"
        | "grid::reorderGrob"
        | "grid::depth"
        | "grid::defnScale"
        | "grid::groupTranslate"
        | "grid::heightDetails"
        | "grid::is.unit"
        | "grid::grobX"
        | "grid::stringHeight"
        | "grid::grid.convert"
        | "grid::makeContext"
        | "grid::grobY"
        | "grid::unit.length"
        | "grid::linearGradient"
        | "grid::grid.null"
        | "grid::grid.arrows"
        | "grid::defnTranslate"
        | "grid::grid.delay"
        | "grid::grid.cap"
        | "grid::validDetails"
        | "grid::grid.gremove"
        | "grid::layout.torture"
        | "grid::grid.show.viewport"
        | "grid::gEdit"
        | "grid::current.parent"
        | "grid::grobCoords"
        | "grid::engine.display.list"
        | "grid::grid.convertWidth"
        | "grid::grid.function"
        | "grid::gEditList"
        | "grid::calcStringMetric"
        | "grid::grid.remove"
        | "grid::grid.grab"
        | "grid::functionGrob"
        | "grid::grid.force"
        | "grid::grid.panel"
        | "grid::setGrob"
        | "grid::stringWidth"
        | "grid::lineToGrob"
        | "grid::draw.details"
        | "grid::current.vpTree" => Some(TypeState::unknown()),
        "graphics::plot"
        | "graphics::plot.default"
        | "graphics::plot.design"
        | "graphics::plot.function"
        | "graphics::plot.new"
        | "graphics::plot.window"
        | "graphics::plot.xy"
        | "graphics::lines"
        | "graphics::lines.default"
        | "graphics::points"
        | "graphics::points.default"
        | "graphics::abline"
        | "graphics::title"
        | "graphics::box"
        | "graphics::text"
        | "graphics::text.default"
        | "graphics::segments"
        | "graphics::arrows"
        | "graphics::mtext"
        | "graphics::polygon"
        | "graphics::polypath"
        | "graphics::matplot"
        | "graphics::matlines"
        | "graphics::matpoints"
        | "graphics::pairs"
        | "graphics::pairs.default"
        | "graphics::stripchart"
        | "graphics::dotchart"
        | "graphics::layout.show"
        | "graphics::pie"
        | "graphics::symbols"
        | "graphics::smoothScatter"
        | "graphics::stem"
        | "graphics::contour"
        | "graphics::contour.default"
        | "graphics::image"
        | "graphics::image.default"
        | "graphics::assocplot"
        | "graphics::mosaicplot"
        | "graphics::fourfoldplot"
        | "graphics::clip"
        | "graphics::xspline"
        | "graphics::.filled.contour"
        | "graphics::filled.contour"
        | "graphics::cdplot"
        | "graphics::coplot"
        | "graphics::curve"
        | "graphics::close.screen"
        | "graphics::co.intervals"
        | "graphics::erase.screen"
        | "graphics::frame"
        | "graphics::grid"
        | "graphics::panel.smooth"
        | "graphics::rasterImage"
        | "graphics::rect"
        | "graphics::spineplot"
        | "graphics::stars"
        | "graphics::sunflowerplot"
        | "grDevices::png"
        | "grDevices::pdf" => Some(TypeState::null()),
        "graphics::persp" => Some(TypeState::matrix(PrimTy::Double, false)),
        "graphics::hist"
        | "graphics::hist.default"
        | "graphics::boxplot"
        | "graphics::boxplot.default"
        | "graphics::boxplot.matrix"
        | "graphics::barplot"
        | "graphics::barplot.default"
        | "graphics::bxp"
        | "graphics::par"
        | "graphics::screen"
        | "graphics::split.screen" => Some(TypeState::vector(PrimTy::Any, false)),
        "graphics::layout" => Some(TypeState::scalar(PrimTy::Int, false)),
        "graphics::identify" => Some(TypeState::vector(PrimTy::Int, false)),
        "graphics::axTicks"
        | "graphics::Axis"
        | "graphics::axis.Date"
        | "graphics::axis.POSIXct"
        | "graphics::strwidth"
        | "graphics::strheight"
        | "graphics::grconvertX"
        | "graphics::grconvertY" => Some(TypeState::vector(PrimTy::Double, false)),
        "graphics::lcm" | "graphics::xinch" | "graphics::yinch" | "graphics::xyinch" => {
            vectorized_scalar_or_vector_double_type(arg_tys)
        }
        "grDevices::jpeg" | "grDevices::bmp" | "grDevices::tiff" => Some(TypeState::null()),
        "grDevices::dev.size" => Some(TypeState::vector(PrimTy::Double, false)),
        "grDevices::dev.off"
        | "grDevices::dev.cur"
        | "grDevices::dev.next"
        | "grDevices::dev.prev" => Some(TypeState::scalar(PrimTy::Int, false)),
        "graphics::axis" => Some(TypeState::vector(PrimTy::Double, false)),
        "graphics::locator" => Some(TypeState::vector(PrimTy::Any, false)),
        "graphics::rug" => Some(TypeState::vector(PrimTy::Double, false)),
        "grDevices::rgb"
        | "grDevices::hsv"
        | "grDevices::gray"
        | "grDevices::gray.colors"
        | "grDevices::palette.colors"
        | "grDevices::palette.pals"
        | "grDevices::hcl.colors"
        | "grDevices::colors"
        | "grDevices::heat.colors"
        | "grDevices::terrain.colors"
        | "grDevices::topo.colors"
        | "grDevices::cm.colors"
        | "grDevices::rainbow"
        | "grDevices::adjustcolor"
        | "grDevices::palette"
        | "grDevices::densCols" => Some(TypeState::vector(PrimTy::Char, false)),
        "grDevices::n2mfrow" => Some(TypeState::vector(PrimTy::Int, false)),
        "grDevices::col2rgb" => Some(TypeState::matrix(PrimTy::Int, false)),
        "grDevices::rgb2hsv" | "grDevices::convertColor" => {
            Some(TypeState::matrix(PrimTy::Double, false))
        }
        "grDevices::as.raster" => Some(TypeState::matrix(PrimTy::Char, false)),
        "grDevices::axisTicks" | "grDevices::extendrange" => {
            Some(TypeState::vector(PrimTy::Double, false))
        }
        "grDevices::cm" => vectorized_scalar_or_vector_double_type(arg_tys),
        "grDevices::boxplot.stats"
        | "grDevices::contourLines"
        | "grDevices::dev.capabilities"
        | "grDevices::dev.capture"
        | "grDevices::check.options"
        | "grDevices::colorConverter"
        | "grDevices::colorRamp"
        | "grDevices::colorRampPalette"
        | "grDevices::getGraphicsEvent"
        | "grDevices::getGraphicsEventEnv"
        | "grDevices::recordPlot"
        | "grDevices::as.graphicsAnnot"
        | "grDevices::make.rgb"
        | "grDevices::pdf.options"
        | "grDevices::pdfFonts"
        | "grDevices::ps.options"
        | "grDevices::postscriptFonts"
        | "grDevices::quartz.options"
        | "grDevices::quartzFont"
        | "grDevices::quartzFonts"
        | "grDevices::X11.options"
        | "grDevices::X11Font"
        | "grDevices::X11Fonts"
        | "grDevices::cairoSymbolFont"
        | "grDevices::CIDFont"
        | "grDevices::Type1Font"
        | "grDevices::Hershey"
        | "grDevices::glyphAnchor"
        | "grDevices::glyphFont"
        | "grDevices::glyphFontList"
        | "grDevices::glyphHeight"
        | "grDevices::glyphHeightBottom"
        | "grDevices::glyphInfo"
        | "grDevices::glyphJust"
        | "grDevices::glyphWidth"
        | "grDevices::glyphWidthLeft"
        | "grDevices::.axisPars"
        | "grDevices::.clipPath"
        | "grDevices::.defineGroup"
        | "grDevices::.devUp"
        | "grDevices::.linearGradientPattern"
        | "grDevices::.mask"
        | "grDevices::.opIndex"
        | "grDevices::.radialGradientPattern"
        | "grDevices::.ruleIndex"
        | "grDevices::.setClipPath"
        | "grDevices::.setMask"
        | "grDevices::.setPattern"
        | "grDevices::.tilingPattern"
        | "grDevices::.useGroup" => Some(TypeState::vector(PrimTy::Any, false)),
        "grDevices::chull" | "grDevices::dev.list" => Some(TypeState::vector(PrimTy::Int, false)),
        "grDevices::dev.set"
        | "grDevices::nclass.FD"
        | "grDevices::nclass.scott"
        | "grDevices::nclass.Sturges" => Some(TypeState::scalar(PrimTy::Int, false)),
        "grDevices::dev.interactive"
        | "grDevices::deviceIsInteractive"
        | "grDevices::is.raster" => Some(TypeState::scalar(PrimTy::Logical, false)),
        "grDevices::blues9"
        | "grDevices::grey"
        | "grDevices::grey.colors"
        | "grDevices::grSoftVersion"
        | "grDevices::hcl"
        | "grDevices::hcl.pals"
        | "grDevices::colorspaces"
        | "grDevices::colours" => Some(TypeState::vector(PrimTy::Char, false)),
        "grDevices::trans3d"
        | "grDevices::xy.coords"
        | "grDevices::xyTable"
        | "grDevices::xyz.coords" => Some(TypeState::vector(PrimTy::Any, false)),
        "grDevices::bitmap"
        | "grDevices::cairo_pdf"
        | "grDevices::cairo_ps"
        | "grDevices::dev.control"
        | "grDevices::dev.copy"
        | "grDevices::dev.copy2eps"
        | "grDevices::dev.copy2pdf"
        | "grDevices::dev.flush"
        | "grDevices::dev.hold"
        | "grDevices::dev.new"
        | "grDevices::dev.print"
        | "grDevices::devAskNewPage"
        | "grDevices::dev2bitmap"
        | "grDevices::embedFonts"
        | "grDevices::embedGlyphs"
        | "grDevices::graphics.off"
        | "grDevices::pictex"
        | "grDevices::postscript"
        | "grDevices::quartz"
        | "grDevices::quartz.save"
        | "grDevices::recordGraphics"
        | "grDevices::replayPlot"
        | "grDevices::savePlot"
        | "grDevices::setEPS"
        | "grDevices::setGraphicsEventEnv"
        | "grDevices::setGraphicsEventHandlers"
        | "grDevices::setPS"
        | "grDevices::svg"
        | "grDevices::x11"
        | "grDevices::X11"
        | "grDevices::xfig" => Some(TypeState::null()),
        "graphics::legend" => Some(TypeState::vector(PrimTy::Any, false)),
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
        "stats::lm" | "stats::glm" => Some(TypeState::vector(PrimTy::Any, false)),
        "stats::as.formula" | "stats::formula" | "stats::reformulate" => Some(TypeState::unknown()),
        "stats::contrasts"
        | "stats::contr.treatment"
        | "stats::contr.sum"
        | "stats::contr.helmert"
        | "stats::contr.SAS"
        | "stats::contr.poly" => Some(TypeState::matrix(PrimTy::Double, false)),
        "stats::binomial"
        | "stats::gaussian"
        | "stats::poisson"
        | "stats::family"
        | "stats::make.link"
        | "stats::quasi"
        | "stats::quasibinomial"
        | "stats::quasipoisson"
        | "stats::inverse.gaussian" => Some(TypeState::vector(PrimTy::Any, false)),
        _ => None,
    }
}

pub fn infer_package_binding(var: &str) -> Option<TypeState> {
    match var {
        "datasets::iris" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::mtcars" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::airquality" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::ToothGrowth" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::CO2" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::USArrests" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::cars" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::pressure" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::faithful" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::women" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::BOD" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::attitude" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::PlantGrowth" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::InsectSprays" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::sleep" => Some(TypeState::matrix(PrimTy::Char, false)),
        "datasets::Orange" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::rock" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::trees" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::esoph" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::stackloss" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::warpbreaks" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::quakes" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::LifeCycleSavings" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::ChickWeight" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::DNase" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::Formaldehyde" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::Indometh" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::Loblolly" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::Puromycin" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::USJudgeRatings" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::anscombe" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::attenu" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::chickwts" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::infert" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::longley" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::morley" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::npk" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::swiss" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::volcano" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::state.x77" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::USPersonalExpenditure" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::WorldPhones" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::EuStockMarkets" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::VADeaths" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::AirPassengers" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::JohnsonJohnson" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::Nile" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::lynx" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::nottem" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::sunspot.year" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::precip" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::islands" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::state.area" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::state.abb" => Some(TypeState::vector(PrimTy::Char, false)),
        "datasets::state.name" => Some(TypeState::vector(PrimTy::Char, false)),
        "datasets::state.region" => Some(TypeState::vector(PrimTy::Char, false)),
        "datasets::state.division" => Some(TypeState::vector(PrimTy::Char, false)),
        "datasets::airmiles" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::austres" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::co2" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::discoveries" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::fdeaths" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::ldeaths" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::mdeaths" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::nhtemp" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::sunspots" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::treering" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::uspop" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::rivers" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::UKDriverDeaths" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::UKgas" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::USAccDeaths" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::WWWusage" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::eurodist" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::UScitiesD" => Some(TypeState::vector(PrimTy::Int, false)),
        "datasets::euro" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::stack.loss" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::sunspot.m2014" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::sunspot.month" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::LakeHuron" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::lh" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::presidents" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::Seatbelts" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::OrchardSprays" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::Theoph" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::penguins" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::penguins_raw" => Some(TypeState::matrix(PrimTy::Any, false)),
        "datasets::gait" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::crimtab" => Some(TypeState::matrix(PrimTy::Int, false)),
        "datasets::occupationalStatus" => Some(TypeState::matrix(PrimTy::Int, false)),
        "datasets::ability.cov" => Some(TypeState::vector(PrimTy::Any, false)),
        "datasets::Harman23.cor" => Some(TypeState::vector(PrimTy::Any, false)),
        "datasets::Harman74.cor" => Some(TypeState::vector(PrimTy::Any, false)),
        "datasets::state.center" => Some(TypeState::vector(PrimTy::Any, false)),
        "datasets::BJsales" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::BJsales.lead" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::beaver1" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::beaver2" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::euro.cross" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::randu" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::freeny" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::stack.x" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::freeny.x" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::freeny.y" => Some(TypeState::vector(PrimTy::Double, false)),
        "datasets::iris3" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::Titanic" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::UCBAdmissions" => Some(TypeState::matrix(PrimTy::Double, false)),
        "datasets::HairEyeColor" => Some(TypeState::matrix(PrimTy::Double, false)),
        callee if callee.starts_with("base::") => Some(TypeState::unknown()),
        _ => None,
    }
}

pub fn infer_builtin_term(callee: &str, arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
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

pub fn infer_package_call_term(callee: &str, arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    match callee {
        "base::data.frame" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::globalenv" | "base::environment" => Some(TypeTerm::Any),
        "base::unlink" => Some(TypeTerm::Int),
        "base::file.path" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::basename" | "base::dirname" | "base::normalizePath" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::dir.exists" | "base::file.exists" => {
            Some(logical_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::eval" | "base::evalq" | "base::do.call" | "base::parse" | "base::readRDS"
        | "base::get0" | "base::getOption" | "base::file" => Some(TypeTerm::Any),
        "base::save" => Some(TypeTerm::Null),
        "base::list.files" | "base::path.expand" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::getNamespace" | "base::asNamespace" => Some(TypeTerm::Any),
        "base::isNamespace" | "base::is.name" => Some(TypeTerm::Logical),
        "base::find.package" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::package_version" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::getElement" | "base::unname" => Some(first_arg_term(arg_terms)),
        "base::baseenv"
        | "base::emptyenv"
        | "base::new.env"
        | "base::parent.env"
        | "base::as.environment"
        | "base::list2env"
        | "base::topenv" => Some(TypeTerm::Any),
        "base::is.environment"
        | "base::environmentIsLocked"
        | "base::isNamespaceLoaded"
        | "base::requireNamespace" => Some(TypeTerm::Logical),
        "base::environmentName" | "base::getNamespaceName" | "base::getNamespaceVersion" => {
            Some(TypeTerm::Char)
        }
        "base::loadedNamespaces" | "base::getNamespaceExports" | "base::getNamespaceUsers" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "base::as.list.environment" | "base::getNamespaceImports" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "base::library" | "base::searchpaths" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::require" | "base::packageHasNamespace" | "base::is.loaded" => {
            Some(TypeTerm::Logical)
        }
        "base::identical"
        | "base::inherits"
        | "base::interactive"
        | "base::is.R"
        | "base::is.array"
        | "base::is.atomic"
        | "base::is.call"
        | "base::is.character"
        | "base::is.complex"
        | "base::is.data.frame"
        | "base::is.double"
        | "base::is.expression"
        | "base::is.factor"
        | "base::is.function"
        | "base::is.integer"
        | "base::is.language"
        | "base::is.list"
        | "base::is.logical"
        | "base::is.null"
        | "base::is.numeric"
        | "base::is.numeric.Date"
        | "base::is.numeric.POSIXt"
        | "base::is.numeric.difftime"
        | "base::is.numeric_version"
        | "base::is.object"
        | "base::is.ordered"
        | "base::is.package_version"
        | "base::is.pairlist"
        | "base::is.primitive"
        | "base::is.qr"
        | "base::is.raw"
        | "base::is.recursive"
        | "base::is.single"
        | "base::is.symbol"
        | "base::is.table"
        | "base::is.unsorted"
        | "base::is.vector" => Some(TypeTerm::Logical),
        "base::is.element"
        | "base::is.finite.POSIXlt"
        | "base::is.infinite"
        | "base::is.infinite.POSIXlt"
        | "base::is.na.POSIXlt"
        | "base::is.na.data.frame"
        | "base::is.na.numeric_version"
        | "base::is.nan"
        | "base::is.nan.POSIXlt" => Some(logical_like_first_arg_term(first_arg_term(arg_terms))),
        "base::loadNamespace" | "base::getLoadedDLLs" | "base::dyn.load" => Some(TypeTerm::Any),
        "base::dyn.unload" => Some(TypeTerm::Null),
        "base::readLines" | "base::Sys.getenv" | "base::Sys.which" | "base::Sys.readlink"
        | "base::Sys.info" | "base::Sys.glob" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::writeLines"
        | "base::writeChar"
        | "base::writeBin"
        | "base::flush"
        | "base::truncate.connection" => Some(TypeTerm::Null),
        "base::seek" => Some(TypeTerm::Double),
        "base::Sys.setenv" | "base::Sys.unsetenv" => Some(TypeTerm::Logical),
        "base::Sys.getpid" => Some(TypeTerm::Int),
        "base::Sys.time" | "base::Sys.Date" => Some(TypeTerm::Double),
        "base::Sys.getlocale" => Some(TypeTerm::Char),
        "base::system" | "base::system2" => Some(TypeTerm::Any),
        "base::system.time" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "base::Sys.sleep" => Some(TypeTerm::Null),
        "base::Sys.setlocale" | "base::Sys.timezone" => Some(TypeTerm::Char),
        "base::Sys.localeconv" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::Sys.setFileTime" | "base::Sys.chmod" => {
            Some(logical_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::Sys.umask" => Some(TypeTerm::Int),
        "base::sys.parent" | "base::sys.nframe" => Some(TypeTerm::Int),
        "base::sys.parents" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "base::search" | "base::gettext" | "base::gettextf" | "base::ngettext" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "base::geterrmessage" => Some(TypeTerm::Char),
        "base::message" | "base::packageStartupMessage" | "base::.packageStartupMessage" => {
            Some(TypeTerm::Null)
        }
        "base::sys.call"
        | "base::sys.calls"
        | "base::sys.function"
        | "base::sys.frame"
        | "base::sys.frames"
        | "base::sys.status"
        | "base::sys.source"
        | "base::source"
        | "base::options"
        | "base::warning"
        | "base::warningCondition"
        | "base::packageNotFoundError"
        | "base::packageEvent" => Some(TypeTerm::Any),
        "base::stdin"
        | "base::stdout"
        | "base::stderr"
        | "base::textConnection"
        | "base::rawConnection"
        | "base::socketConnection"
        | "base::url"
        | "base::pipe"
        | "base::open"
        | "base::summary.connection" => Some(TypeTerm::Any),
        "base::textConnectionValue" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::rawConnectionValue" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::close" | "base::closeAllConnections" | "base::pushBack" | "base::clearPushBack" => {
            Some(TypeTerm::Null)
        }
        "base::close.connection" | "base::close.srcfile" | "base::close.srcfilealias" => {
            Some(TypeTerm::Null)
        }
        "base::isOpen" | "base::isIncomplete" => Some(TypeTerm::Logical),
        "base::pushBackLength" => Some(TypeTerm::Int),
        "base::socketSelect" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "base::scan" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::read.table" | "base::read.csv" | "base::read.csv2" | "base::read.delim"
        | "base::read.delim2" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::write.table" | "base::write.csv" | "base::write.csv2" | "base::saveRDS"
        | "base::dput" | "base::dump" | "base::sink" => Some(TypeTerm::Null),
        "base::count.fields" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "base::sink.number" => Some(TypeTerm::Int),
        "base::capture.output" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::lapply" | "base::Map" | "base::split" | "base::by" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "base::sapply" | "base::vapply" | "base::mapply" | "base::tapply" | "base::apply" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Any)))
        }
        "base::Reduce" | "base::Find" => Some(TypeTerm::Any),
        "base::Filter" | "base::unsplit" | "base::within" | "base::transform" => {
            Some(first_arg_term(arg_terms))
        }
        "base::Position" => Some(TypeTerm::Int),
        "base::expand.grid" | "base::merge" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::as.Date"
        | "base::as.Date.character"
        | "base::as.Date.default"
        | "base::as.Date.factor"
        | "base::as.Date.numeric"
        | "base::as.Date.POSIXct"
        | "base::as.Date.POSIXlt"
        | "base::as.POSIXct"
        | "base::as.POSIXct.Date"
        | "base::as.POSIXct.default"
        | "base::as.POSIXct.numeric"
        | "base::as.POSIXct.POSIXlt"
        | "base::as.POSIXlt"
        | "base::as.POSIXlt.character"
        | "base::as.POSIXlt.Date"
        | "base::as.POSIXlt.default"
        | "base::as.POSIXlt.factor"
        | "base::as.POSIXlt.numeric"
        | "base::as.POSIXlt.POSIXct"
        | "base::as.difftime"
        | "base::as.double.difftime"
        | "base::as.double.POSIXlt"
        | "base::strptime"
        | "base::difftime"
        | "base::julian" => Some(double_like_first_arg_term(first_arg_term(arg_terms))),
        "base::as.character.Date"
        | "base::as.character.POSIXt"
        | "base::format.Date"
        | "base::format.POSIXct"
        | "base::format.POSIXlt"
        | "base::months"
        | "base::quarters"
        | "base::weekdays" => Some(char_like_first_arg_term(first_arg_term(arg_terms))),
        "base::OlsonNames" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::ISOdate" | "base::ISOdatetime" | "base::seq.Date" | "base::seq.POSIXt" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "base::all.names" | "base::all.vars" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::anyDuplicated.array"
        | "base::anyDuplicated.data.frame"
        | "base::anyDuplicated.default"
        | "base::anyDuplicated.matrix" => Some(TypeTerm::Int),
        "base::anyNA" | "base::anyNA.data.frame" => Some(TypeTerm::Logical),
        "base::anyNA.numeric_version" | "base::anyNA.POSIXlt" => Some(TypeTerm::Logical),
        "base::addTaskCallback" => Some(TypeTerm::Int),
        "base::bindingIsActive" | "base::bindingIsLocked" => Some(TypeTerm::Logical),
        "base::backsolve" => Some(double_like_first_arg_term(second_arg_term(arg_terms))),
        "base::balancePOSIXlt" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::besselI" | "base::besselJ" | "base::besselK" | "base::besselY" | "base::beta"
        | "base::choose" => Some(double_like_first_arg_term(first_arg_term(arg_terms))),
        "base::casefold" | "base::char.expand" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::charmatch" => Some(int_like_first_arg_term(first_arg_term(arg_terms))),
        "base::charToRaw" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::chkDots" => Some(TypeTerm::Null),
        "base::chol" | "base::chol.default" | "base::chol2inv" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "base::chooseOpsMethod" | "base::chooseOpsMethod.default" => Some(TypeTerm::Logical),
        "base::complete.cases" => Some(logical_like_first_arg_term(first_arg_term(arg_terms))),
        "base::complex" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::cut.Date" | "base::cut.POSIXt" | "base::cut.default" => {
            Some(int_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::cummax" | "base::cummin" | "base::cumsum" => {
            Some(vectorized_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::cumprod" => Some(double_like_first_arg_term(first_arg_term(arg_terms))),
        "base::diff" | "base::diff.default" => {
            Some(preserved_head_tail_term(first_arg_term(arg_terms)))
        }
        "base::diff.Date" | "base::diff.POSIXt" | "base::diff.difftime" => {
            Some(double_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::commandArgs" | "base::data.class" | "base::deparse" | "base::extSoftVersion" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "base::data.matrix" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "base::det" => Some(TypeTerm::Double),
        "base::determinant" | "base::determinant.matrix" => Some(TypeTerm::NamedList(vec![
            ("modulus".to_string(), TypeTerm::Double),
            ("sign".to_string(), TypeTerm::Int),
        ])),
        "base::debuggingState" => Some(TypeTerm::Logical),
        "base::dget" => Some(TypeTerm::Any),
        "base::debug"
        | "base::debugonce"
        | "base::declare"
        | "base::delayedAssign"
        | "base::detach"
        | "base::enquote"
        | "base::env.profile"
        | "base::environment<-"
        | "base::errorCondition"
        | "base::eval.parent"
        | "base::Exec"
        | "base::expression" => Some(TypeTerm::Any),
        "base::date" | "base::deparse1" | "base::file.choose" => Some(TypeTerm::Char),
        "base::dQuote" | "base::enc2native" | "base::enc2utf8" | "base::encodeString"
        | "base::Encoding" => Some(char_like_first_arg_term(first_arg_term(arg_terms))),
        "base::dontCheck" => Some(first_arg_term(arg_terms)),
        "base::digamma" | "base::expm1" | "base::factorial" | "base::acosh" | "base::asinh"
        | "base::atanh" | "base::cospi" => {
            Some(double_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::eigen" => Some(TypeTerm::NamedList(vec![
            (
                "values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "vectors".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "base::exists" => Some(TypeTerm::Logical),
        "base::findInterval" => Some(int_like_first_arg_term(first_arg_term(arg_terms))),
        "base::file.show" => Some(TypeTerm::Null),
        "base::format.data.frame" | "base::format.info" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::format"
        | "base::format.AsIs"
        | "base::format.default"
        | "base::format.difftime"
        | "base::format.factor"
        | "base::format.hexmode"
        | "base::format.libraryIQR"
        | "base::format.numeric_version"
        | "base::format.octmode"
        | "base::format.packageInfo"
        | "base::format.pval"
        | "base::format.summaryDefault"
        | "base::formatC"
        | "base::formatDL" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::drop" => Some(TypeTerm::Any),
        "base::droplevels" | "base::droplevels.data.frame" => Some(first_arg_term(arg_terms)),
        "base::droplevels.factor" => Some(int_like_first_arg_term(first_arg_term(arg_terms))),
        "base::duplicated.default" => infer_builtin_term("duplicated", arg_terms),
        "base::duplicated.array"
        | "base::duplicated.data.frame"
        | "base::duplicated.matrix"
        | "base::duplicated.numeric_version"
        | "base::duplicated.POSIXlt"
        | "base::duplicated.warnings" => infer_builtin_term("duplicated", arg_terms),
        "base::attr<-" | "base::attributes<-" | "base::class<-" | "base::colnames<-"
        | "base::comment<-" | "base::dimnames<-" | "base::levels<-" | "base::names<-"
        | "base::row.names<-" | "base::rownames<-" => Some(first_arg_term(arg_terms)),
        "base::body<-" => Some(TypeTerm::Any),
        "base::bindtextdomain" => Some(TypeTerm::Char),
        "base::builtins" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::alist"
        | "base::as.expression"
        | "base::as.expression.default"
        | "base::as.package_version"
        | "base::as.pairlist" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::as.call"
        | "base::as.function"
        | "base::as.function.default"
        | "base::as.name"
        | "base::as.symbol"
        | "base::activeBindingFunction"
        | "base::allowInterrupts"
        | "base::attach"
        | "base::attachNamespace"
        | "base::autoload"
        | "base::autoloader"
        | "base::break"
        | "base::browser"
        | "base::browserSetDebug"
        | "base::as.qr"
        | "base::asS3"
        | "base::asS4" => Some(TypeTerm::Any),
        "base::Arg" => Some(double_like_first_arg_term(first_arg_term(arg_terms))),
        "base::aperm.default" | "base::aperm.table" => {
            Some(matrix_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.complex" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::as.hexmode" | "base::as.octmode" | "base::as.ordered" | "base::gl" => {
            Some(int_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.numeric_version" | "base::asplit" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "base::as.null" | "base::as.null.default" => Some(TypeTerm::Null),
        "base::as.raw" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::as.single" | "base::as.single.default" => {
            Some(double_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.table" | "base::as.table.default" => {
            Some(matrix_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::bitwAnd" | "base::bitwNot" | "base::bitwOr" | "base::bitwShiftL"
        | "base::bitwShiftR" | "base::bitwXor" => {
            Some(int_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::by.data.frame"
        | "base::by.default"
        | "base::computeRestarts"
        | "base::c.numeric_version"
        | "base::c.POSIXlt"
        | "base::c.warnings" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::c.Date" | "base::c.difftime" | "base::c.POSIXct" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "base::c.factor" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "base::c.noquote" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::callCC"
        | "base::comment"
        | "base::conditionCall"
        | "base::conditionCall.condition"
        | "base::conflictRules" => Some(TypeTerm::Any),
        "base::cbind.data.frame" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::conditionMessage"
        | "base::conditionMessage.condition"
        | "base::conflicts"
        | "base::curlGetHeaders" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::contributors" => Some(TypeTerm::Null),
        "base::Cstack_info" => Some(TypeTerm::NamedList(vec![
            ("size".to_string(), TypeTerm::Int),
            ("current".to_string(), TypeTerm::Int),
            ("direction".to_string(), TypeTerm::Int),
            ("eval_depth".to_string(), TypeTerm::Int),
        ])),
        "base::browserText" => Some(TypeTerm::Char),
        "base::capabilities" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "base::Conj" => Some(first_arg_term(arg_terms)),
        "base::abbreviate"
        | "base::as.character"
        | "base::as.character.condition"
        | "base::as.character.default"
        | "base::as.character.error"
        | "base::as.character.factor"
        | "base::as.character.hexmode"
        | "base::as.character.numeric_version"
        | "base::as.character.octmode"
        | "base::as.character.srcref" => Some(char_like_first_arg_term(first_arg_term(arg_terms))),
        "base::all.equal"
        | "base::all.equal.default"
        | "base::all.equal.character"
        | "base::all.equal.environment"
        | "base::all.equal.envRefClass"
        | "base::all.equal.factor"
        | "base::all.equal.formula"
        | "base::all.equal.function"
        | "base::all.equal.language"
        | "base::all.equal.list"
        | "base::all.equal.numeric"
        | "base::all.equal.POSIXt"
        | "base::all.equal.raw"
        | "base::args"
        | "base::body"
        | "base::call"
        | "base::bquote"
        | "base::browserCondition" => Some(TypeTerm::Any),
        "base::array"
        | "base::as.array"
        | "base::as.array.default"
        | "base::as.matrix"
        | "base::as.matrix.data.frame"
        | "base::as.matrix.default"
        | "base::as.matrix.noquote"
        | "base::as.matrix.POSIXlt"
        | "base::aperm" => Some(matrix_like_first_arg_term(first_arg_term(arg_terms))),
        "base::as.data.frame"
        | "base::as.data.frame.array"
        | "base::as.data.frame.AsIs"
        | "base::as.data.frame.character"
        | "base::as.data.frame.complex"
        | "base::as.data.frame.data.frame"
        | "base::as.data.frame.Date"
        | "base::as.data.frame.default"
        | "base::as.data.frame.difftime"
        | "base::as.data.frame.factor"
        | "base::as.data.frame.integer"
        | "base::as.data.frame.list"
        | "base::as.data.frame.logical"
        | "base::as.data.frame.matrix"
        | "base::as.data.frame.model.matrix"
        | "base::as.data.frame.noquote"
        | "base::as.data.frame.numeric"
        | "base::as.data.frame.numeric_version"
        | "base::as.data.frame.ordered"
        | "base::as.data.frame.POSIXct"
        | "base::as.data.frame.POSIXlt"
        | "base::as.data.frame.raw"
        | "base::as.data.frame.table"
        | "base::as.data.frame.ts"
        | "base::as.data.frame.vector"
        | "base::array2DF" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::as.list"
        | "base::as.list.data.frame"
        | "base::as.list.Date"
        | "base::as.list.default"
        | "base::as.list.difftime"
        | "base::as.list.factor"
        | "base::as.list.function"
        | "base::as.list.numeric_version"
        | "base::as.list.POSIXct"
        | "base::as.list.POSIXlt" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::arrayInd" | "base::col" | "base::row" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Int)))
        }
        "base::colMeans" | "base::rowMeans" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "base::append" | "base::addNA" => Some(first_arg_term(arg_terms)),
        "base::as.double" | "base::as.numeric" => {
            Some(double_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.factor" | "base::as.integer" | "base::ordered" => {
            Some(int_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.logical" | "base::as.logical.factor" => {
            Some(logical_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::as.vector"
        | "base::as.vector.data.frame"
        | "base::as.vector.factor"
        | "base::as.vector.POSIXlt" => Some(vectorized_first_arg_term(first_arg_term(arg_terms))),
        "base::class" | "base::levels" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::attr" => Some(TypeTerm::Any),
        "base::attributes" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "base::readBin" | "base::serialize" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "base::readChar" | "base::load" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "base::unserialize" | "base::fifo" | "base::gzcon" => Some(TypeTerm::Any),
        "base::getwd" | "base::tempdir" | "base::tempfile" | "base::system.file" => {
            Some(TypeTerm::Char)
        }
        "base::dir" | "base::list.dirs" | "base::path.package" | "base::.packages" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "base::dir.create" | "base::file.create" | "base::file.remove" | "base::file.rename"
        | "base::file.copy" | "base::file.append" | "base::file.link" | "base::file.symlink" => {
            Some(logical_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "base::file.access" | "base::file.mode" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "base::file.info" => Some(TypeTerm::DataFrame(Vec::new())),
        "base::file.size" | "base::file.mtime" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "base::length" => infer_builtin_term("length", arg_terms),
        "base::seq_len" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "base::seq_along" => infer_builtin_term("seq_along", arg_terms),
        "base::c" => infer_builtin_term("c", arg_terms),
        "base::list" => infer_builtin_term("list", arg_terms),
        "base::sum" => infer_builtin_term("sum", arg_terms),
        "base::mean" => infer_builtin_term("mean", arg_terms),
        "base::abs" => infer_builtin_term("abs", arg_terms),
        "base::seq" => infer_builtin_term("seq", arg_terms),
        "base::ifelse" => infer_builtin_term("ifelse", arg_terms),
        "base::min" => infer_builtin_term("min", arg_terms),
        "base::max" => infer_builtin_term("max", arg_terms),
        "base::pmax" => infer_builtin_term("pmax", arg_terms),
        "base::pmin" => infer_builtin_term("pmin", arg_terms),
        "base::sqrt" => infer_builtin_term("sqrt", arg_terms),
        "base::log" => infer_builtin_term("log", arg_terms),
        "base::log10" => infer_builtin_term("log10", arg_terms),
        "base::log2" => infer_builtin_term("log2", arg_terms),
        "base::exp" => infer_builtin_term("exp", arg_terms),
        "base::atan2" => infer_builtin_term("atan2", arg_terms),
        "base::sin" => infer_builtin_term("sin", arg_terms),
        "base::cos" => infer_builtin_term("cos", arg_terms),
        "base::tan" => infer_builtin_term("tan", arg_terms),
        "base::asin" => infer_builtin_term("asin", arg_terms),
        "base::acos" => infer_builtin_term("acos", arg_terms),
        "base::atan" => infer_builtin_term("atan", arg_terms),
        "base::sinh" => infer_builtin_term("sinh", arg_terms),
        "base::cosh" => infer_builtin_term("cosh", arg_terms),
        "base::tanh" => infer_builtin_term("tanh", arg_terms),
        "base::sign" => infer_builtin_term("sign", arg_terms),
        "base::gamma" => infer_builtin_term("gamma", arg_terms),
        "base::lgamma" => infer_builtin_term("lgamma", arg_terms),
        "base::floor" => infer_builtin_term("floor", arg_terms),
        "base::ceiling" => infer_builtin_term("ceiling", arg_terms),
        "base::trunc" => infer_builtin_term("trunc", arg_terms),
        "base::round" => infer_builtin_term("round", arg_terms),
        "base::is.na" => infer_builtin_term("is.na", arg_terms),
        "base::is.finite" => infer_builtin_term("is.finite", arg_terms),
        "base::print" => Some(first_arg_term(arg_terms)),
        callee if callee.starts_with("base::print.") => Some(first_arg_term(arg_terms)),
        "base::numeric" => infer_builtin_term("numeric", arg_terms),
        "base::matrix" => infer_builtin_term("matrix", arg_terms),
        "base::diag" => infer_builtin_term("diag", arg_terms),
        "base::t" => infer_builtin_term("t", arg_terms),
        "base::rbind" => infer_builtin_term("rbind", arg_terms),
        "base::cbind" => infer_builtin_term("cbind", arg_terms),
        "base::rowSums" => infer_builtin_term("rowSums", arg_terms),
        "base::colSums" => infer_builtin_term("colSums", arg_terms),
        "base::crossprod" => infer_builtin_term("crossprod", arg_terms),
        "base::tcrossprod" => infer_builtin_term("tcrossprod", arg_terms),
        "base::dim" => infer_builtin_term("dim", arg_terms),
        "base::dimnames" => infer_builtin_term("dimnames", arg_terms),
        "base::nrow" => infer_builtin_term("nrow", arg_terms),
        "base::ncol" => infer_builtin_term("ncol", arg_terms),
        "base::character" => infer_builtin_term("character", arg_terms),
        "base::logical" => infer_builtin_term("logical", arg_terms),
        "base::integer" => infer_builtin_term("integer", arg_terms),
        "base::double" => infer_builtin_term("double", arg_terms),
        "base::rep" => infer_builtin_term("rep", arg_terms),
        "base::rep.int" => infer_builtin_term("rep.int", arg_terms),
        "base::any" => infer_builtin_term("any", arg_terms),
        "base::all" => infer_builtin_term("all", arg_terms),
        "base::which" => infer_builtin_term("which", arg_terms),
        "base::prod" => infer_builtin_term("prod", arg_terms),
        "base::paste" => infer_builtin_term("paste", arg_terms),
        "base::paste0" => infer_builtin_term("paste0", arg_terms),
        "base::sprintf" => infer_builtin_term("sprintf", arg_terms),
        "base::cat" => infer_builtin_term("cat", arg_terms),
        "base::tolower" => infer_builtin_term("tolower", arg_terms),
        "base::toupper" => infer_builtin_term("toupper", arg_terms),
        "base::substr" => infer_builtin_term("substr", arg_terms),
        "base::sub" => infer_builtin_term("sub", arg_terms),
        "base::gsub" => infer_builtin_term("gsub", arg_terms),
        "base::nchar" => infer_builtin_term("nchar", arg_terms),
        "base::nzchar" => infer_builtin_term("nzchar", arg_terms),
        "base::grepl" => infer_builtin_term("grepl", arg_terms),
        "base::grep" => infer_builtin_term("grep", arg_terms),
        "base::startsWith" => infer_builtin_term("startsWith", arg_terms),
        "base::endsWith" => infer_builtin_term("endsWith", arg_terms),
        "base::which.min" => infer_builtin_term("which.min", arg_terms),
        "base::which.max" => infer_builtin_term("which.max", arg_terms),
        "base::isTRUE" => infer_builtin_term("isTRUE", arg_terms),
        "base::isFALSE" => infer_builtin_term("isFALSE", arg_terms),
        "base::lengths" => infer_builtin_term("lengths", arg_terms),
        "base::union" => infer_builtin_term("union", arg_terms),
        "base::intersect" => infer_builtin_term("intersect", arg_terms),
        "base::setdiff" => infer_builtin_term("setdiff", arg_terms),
        "base::sample" => infer_builtin_term("sample", arg_terms),
        "base::sample.int" => infer_builtin_term("sample.int", arg_terms),
        "base::rank" => infer_builtin_term("rank", arg_terms),
        "base::factor" => infer_builtin_term("factor", arg_terms),
        "base::cut" => infer_builtin_term("cut", arg_terms),
        "base::table" => infer_builtin_term("table", arg_terms),
        "base::trimws" => infer_builtin_term("trimws", arg_terms),
        "base::chartr" => infer_builtin_term("chartr", arg_terms),
        "base::strsplit" => infer_builtin_term("strsplit", arg_terms),
        "base::regexpr" => infer_builtin_term("regexpr", arg_terms),
        "base::gregexpr" => infer_builtin_term("gregexpr", arg_terms),
        "base::regexec" => infer_builtin_term("regexec", arg_terms),
        "base::agrep" => infer_builtin_term("agrep", arg_terms),
        "base::agrepl" => infer_builtin_term("agrepl", arg_terms),
        "base::names" => infer_builtin_term("names", arg_terms),
        "base::rownames" => infer_builtin_term("rownames", arg_terms),
        "base::colnames" => infer_builtin_term("colnames", arg_terms),
        "base::sort" => infer_builtin_term("sort", arg_terms),
        "base::order" => infer_builtin_term("order", arg_terms),
        "base::match" => infer_builtin_term("match", arg_terms),
        "base::unique" => infer_builtin_term("unique", arg_terms),
        "base::duplicated" => infer_builtin_term("duplicated", arg_terms),
        "base::anyDuplicated" => infer_builtin_term("anyDuplicated", arg_terms),
        "base::summary" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        callee if callee.starts_with("base::summary.") => Some(TypeTerm::Any),
        "stats::dnorm" | "stats::pnorm" | "stats::qnorm" | "stats::dbinom" | "stats::pbinom"
        | "stats::qbinom" | "stats::dpois" | "stats::ppois" | "stats::qpois" | "stats::dunif"
        | "stats::punif" | "stats::qunif" | "stats::dgamma" | "stats::pgamma" | "stats::qgamma"
        | "stats::dbeta" | "stats::pbeta" | "stats::qbeta" | "stats::dt" | "stats::pt"
        | "stats::qt" | "stats::df" | "stats::pf" | "stats::qf" | "stats::dchisq"
        | "stats::pchisq" | "stats::qchisq" | "stats::dexp" | "stats::pexp" | "stats::qexp"
        | "stats::dlnorm" | "stats::plnorm" | "stats::qlnorm" | "stats::dweibull"
        | "stats::pweibull" | "stats::qweibull" | "stats::dcauchy" | "stats::pcauchy"
        | "stats::qcauchy" | "stats::dgeom" | "stats::pgeom" | "stats::qgeom" | "stats::dhyper"
        | "stats::phyper" | "stats::qhyper" | "stats::dnbinom" | "stats::pnbinom"
        | "stats::qnbinom" | "stats::dlogis" | "stats::plogis" | "stats::qlogis"
        | "stats::pbirthday" | "stats::qbirthday" | "stats::ptukey" | "stats::qtukey"
        | "stats::psmirnov" | "stats::qsmirnov" | "stats::dsignrank" | "stats::psignrank"
        | "stats::qsignrank" | "stats::dwilcox" | "stats::pwilcox" | "stats::qwilcox" => {
            vectorized_scalar_or_vector_double_term(arg_terms)
        }
        "stats::rnorm" | "stats::runif" | "stats::rgamma" | "stats::rbeta" | "stats::rt"
        | "stats::rf" | "stats::rchisq" | "stats::rexp" | "stats::rlnorm" | "stats::rweibull"
        | "stats::rcauchy" | "stats::rlogis" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::rbinom" | "stats::rpois" | "stats::rgeom" | "stats::rhyper" | "stats::rnbinom"
        | "stats::rsignrank" | "stats::rwilcox" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "stats::rsmirnov" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::acf2AR" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::p.adjust" => vectorized_scalar_or_vector_double_term(arg_terms),
        "stats::ppoints" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::dist" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::addmargins" | "stats::ftable" | "stats::xtabs" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "stats::.vcov.aliased" | "stats::estVar" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "stats::smooth" | "stats::smoothEnds" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::cmdscale" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::aggregate" | "stats::aggregate.data.frame" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::expand.model.frame" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::read.ftable" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::aggregate.ts" => Some(ts_like_output_term(first_arg_term(arg_terms))),
        "stats::reshape" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::terms.formula" | "stats::delete.response" => Some(TypeTerm::Any),
        "stats::get_all_vars" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::tsSmooth" => Some(ts_like_output_term(first_arg_term(arg_terms))),
        "stats::ave" => Some(vectorized_first_arg_term(first_arg_term(arg_terms))),
        "stats::reorder" | "stats::relevel" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "stats::DF2formula"
        | "stats::power"
        | "stats::C"
        | "stats::Pair"
        | "stats::preplot"
        | "stats::profile"
        | "stats::ppr"
        | "stats::KalmanLike"
        | "stats::makeARIMA"
        | "stats::eff.aovlist"
        | "stats::stat.anova"
        | "stats::Gamma"
        | "stats::.checkMFClasses"
        | "stats::.getXlevels"
        | "stats::.lm.fit"
        | "stats::.MFclass"
        | "stats::.preformat.ts" => Some(TypeTerm::Any),
        "stats::model.response" | "stats::model.extract" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Any)))
        }
        "stats::case.names" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "stats::complete.cases" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "stats::replications" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "stats::.nknots.smspl" => Some(TypeTerm::Int),
        "stats::fivenum" => Some(TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(5))),
        "stats::knots" | "stats::se.contrast" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::p.adjust.methods" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "stats::sortedXyData" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::pairwise.table" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::symnum" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "stats::SSasymp" | "stats::SSasympOff" | "stats::SSasympOrig" | "stats::SSbiexp"
        | "stats::SSfol" | "stats::SSfpl" | "stats::SSgompertz" | "stats::SSlogis"
        | "stats::SSmicmen" | "stats::SSweibull" => {
            vectorized_scalar_or_vector_double_term(arg_terms)
        }
        "stats::NLSstAsymptotic"
        | "stats::NLSstClosestX"
        | "stats::NLSstLfAsymptote"
        | "stats::NLSstRtAsymptote" => vectorized_scalar_or_vector_double_term(arg_terms),
        "stats::splinefunH" | "stats::SSD" => Some(TypeTerm::Any),
        "stats::selfStart" | "stats::deriv" | "stats::deriv3" => Some(TypeTerm::Any),
        "stats::D" => Some(TypeTerm::Any),
        "stats::numericDeriv" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::cov" | "stats::cor" | "stats::var" => scalar_or_matrix_double_term(arg_terms),
        "stats::cov.wt" => Some(TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "center".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("n.obs".to_string(), TypeTerm::Int),
        ])),
        "stats::cov2cor" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::mahalanobis" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::rWishart" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![None, None, None],
        )),
        "stats::r2dtable" => Some(TypeTerm::List(Box::new(TypeTerm::Matrix(Box::new(
            TypeTerm::Int,
        ))))),
        "stats::dmultinom" => Some(TypeTerm::Double),
        "stats::rmultinom" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Int))),
        "stats::ts" | "stats::as.ts" | "stats::hasTsp" | "stats::window" | "stats::window<-"
        | "stats::lag" | "stats::tsp<-" => Some(ts_like_output_term(first_arg_term(arg_terms))),
        "stats::contrasts<-" => Some(preserved_head_tail_term(first_arg_term(arg_terms))),
        "stats::ts.intersect" | "stats::ts.union" => {
            Some(TypeTerm::Matrix(Box::new(first_numeric_term(arg_terms))))
        }
        "stats::frequency" => Some(TypeTerm::Double),
        "stats::is.ts" | "stats::is.mts" => Some(TypeTerm::Logical),
        "stats::tsp" => Some(TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(3))),
        "stats::start" | "stats::end" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::deltat" => Some(TypeTerm::Double),
        "stats::time" | "stats::cycle" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::embed" => Some(TypeTerm::Matrix(Box::new(first_numeric_term(arg_terms)))),
        "stats::weighted.mean" => Some(TypeTerm::Double),
        "stats::runmed" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::filter" => Some(ts_like_output_term(first_arg_term(arg_terms))),
        "stats::spec.taper" => Some(preserved_head_tail_term(first_arg_term(arg_terms))),
        "stats::arima0.diag"
        | "stats::cpgram"
        | "stats::plclust"
        | "stats::ts.plot"
        | "stats::write.ftable" => Some(TypeTerm::Null),
        "stats::stepfun" | "stats::as.stepfun" => Some(TypeTerm::Any),
        "stats::is.stepfun" | "stats::is.leaf" => Some(TypeTerm::Logical),
        "stats::plot.ecdf" | "stats::plot.ts" | "stats::screeplot" => Some(TypeTerm::Null),
        "stats::plot.stepfun" => Some(TypeTerm::NamedList(vec![
            (
                "t".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
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
        "stats::cutree" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
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
        "stats::setNames" => Some(preserved_head_tail_term(first_arg_term(arg_terms))),
        "stats::printCoefmat" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::optim" => Some(TypeTerm::NamedList(vec![
            (
                "par".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("value".to_string(), TypeTerm::Double),
            (
                "counts".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("convergence".to_string(), TypeTerm::Int),
            ("message".to_string(), TypeTerm::Any),
        ])),
        "stats::optimHess" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::optimize" | "stats::optimise" => Some(TypeTerm::NamedList(vec![
            ("minimum".to_string(), TypeTerm::Double),
            ("objective".to_string(), TypeTerm::Double),
        ])),
        "stats::nlm" => Some(TypeTerm::NamedList(vec![
            ("minimum".to_string(), TypeTerm::Double),
            (
                "estimate".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "gradient".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("code".to_string(), TypeTerm::Int),
            ("iterations".to_string(), TypeTerm::Int),
        ])),
        "stats::nlminb" => Some(TypeTerm::NamedList(vec![
            (
                "par".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("objective".to_string(), TypeTerm::Double),
            ("convergence".to_string(), TypeTerm::Int),
            ("iterations".to_string(), TypeTerm::Int),
            (
                "evaluations".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("message".to_string(), TypeTerm::Char),
        ])),
        "stats::constrOptim" => Some(TypeTerm::NamedList(vec![
            (
                "par".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("value".to_string(), TypeTerm::Double),
            (
                "counts".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("convergence".to_string(), TypeTerm::Int),
            ("message".to_string(), TypeTerm::Any),
            ("outer.iterations".to_string(), TypeTerm::Int),
            ("barrier.value".to_string(), TypeTerm::Double),
        ])),
        "stats::uniroot" => Some(TypeTerm::NamedList(vec![
            ("root".to_string(), TypeTerm::Double),
            ("f.root".to_string(), TypeTerm::Double),
            ("iter".to_string(), TypeTerm::Int),
            ("init.it".to_string(), TypeTerm::Int),
            ("estim.prec".to_string(), TypeTerm::Double),
        ])),
        "stats::integrate" => Some(TypeTerm::NamedList(vec![
            ("value".to_string(), TypeTerm::Double),
            ("abs.error".to_string(), TypeTerm::Double),
            ("subdivisions".to_string(), TypeTerm::Int),
            ("message".to_string(), TypeTerm::Char),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::HoltWinters" => Some(TypeTerm::NamedList(vec![
            (
                "fitted".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("alpha".to_string(), TypeTerm::Double),
            ("beta".to_string(), TypeTerm::Any),
            ("gamma".to_string(), TypeTerm::Any),
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("seasonal".to_string(), TypeTerm::Char),
            ("SSE".to_string(), TypeTerm::Double),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::StructTS" => Some(TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("loglik".to_string(), TypeTerm::Double),
            ("loglik0".to_string(), TypeTerm::Double),
            (
                "data".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "fitted".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("call".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("code".to_string(), TypeTerm::Int),
            ("model".to_string(), TypeTerm::Any),
            (
                "model0".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any)),
            ),
            (
                "xtsp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::KalmanForecast" => Some(TypeTerm::NamedList(vec![
            (
                "pred".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "var".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::KalmanRun" => Some(TypeTerm::NamedList(vec![
            (
                "values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "resid".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "states".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::KalmanSmooth" => Some(TypeTerm::NamedList(vec![
            (
                "smooth".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "var".to_string(),
                TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, None, None]),
            ),
        ])),
        "stats::arima0" => Some(TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("sigma2".to_string(), TypeTerm::Double),
            (
                "var.coef".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("mask".to_string(), TypeTerm::Any),
            ("loglik".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            (
                "arma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("call".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("code".to_string(), TypeTerm::Int),
            ("n.cond".to_string(), TypeTerm::Int),
        ])),
        "stats::arima" => Some(TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("sigma2".to_string(), TypeTerm::Double),
            (
                "var.coef".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("mask".to_string(), TypeTerm::Any),
            ("loglik".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            (
                "arma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("call".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("code".to_string(), TypeTerm::Int),
            ("n.cond".to_string(), TypeTerm::Int),
            ("nobs".to_string(), TypeTerm::Int),
            ("model".to_string(), TypeTerm::Any),
        ])),
        "stats::ar" | "stats::ar.yw" | "stats::ar.mle" | "stats::ar.burg" => {
            Some(TypeTerm::NamedList(vec![
                ("order".to_string(), TypeTerm::Int),
                (
                    "ar".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double)),
                ),
                ("var.pred".to_string(), TypeTerm::Double),
                ("x.mean".to_string(), TypeTerm::Double),
                (
                    "aic".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double)),
                ),
                ("n.used".to_string(), TypeTerm::Int),
                ("n.obs".to_string(), TypeTerm::Int),
                ("order.max".to_string(), TypeTerm::Double),
                (
                    "partialacf".to_string(),
                    TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, Some(1), Some(1)]),
                ),
                (
                    "resid".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double)),
                ),
                ("method".to_string(), TypeTerm::Char),
                ("series".to_string(), TypeTerm::Char),
                ("frequency".to_string(), TypeTerm::Double),
                ("call".to_string(), TypeTerm::Any),
                (
                    "asy.var.coef".to_string(),
                    TypeTerm::Matrix(Box::new(TypeTerm::Double)),
                ),
            ]))
        }
        "stats::ar.ols" => Some(TypeTerm::NamedList(vec![
            ("order".to_string(), TypeTerm::Int),
            (
                "ar".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("var.pred".to_string(), TypeTerm::Double),
            ("x.mean".to_string(), TypeTerm::Double),
            (
                "aic".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("n.used".to_string(), TypeTerm::Int),
            ("n.obs".to_string(), TypeTerm::Int),
            ("order.max".to_string(), TypeTerm::Double),
            (
                "partialacf".to_string(),
                TypeTerm::ArrayDim(Box::new(TypeTerm::Double), vec![None, Some(1), Some(1)]),
            ),
            (
                "resid".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("method".to_string(), TypeTerm::Char),
            ("series".to_string(), TypeTerm::Char),
            ("frequency".to_string(), TypeTerm::Double),
            ("call".to_string(), TypeTerm::Any),
            ("x.intercept".to_string(), TypeTerm::Double),
            (
                "asy.se.coef".to_string(),
                TypeTerm::List(Box::new(TypeTerm::Any)),
            ),
        ])),
        "stats::arima.sim" | "stats::ARMAacf" | "stats::ARMAtoMA" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "stats::tsdiag" => Some(TypeTerm::Null),
        "stats::kernel" => Some(TypeTerm::NamedList(vec![
            (
                "coef".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("m".to_string(), TypeTerm::Int),
        ])),
        "stats::is.tskernel" => Some(TypeTerm::Logical),
        "stats::df.kernel" | "stats::bandwidth.kernel" => Some(TypeTerm::Double),
        "stats::kernapply" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::convolve" | "stats::fft" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "stats::mvfft" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Any))),
        "stats::nextn" => Some(TypeTerm::Int),
        "stats::spec.ar" => Some(TypeTerm::NamedList(vec![
            (
                "freq".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "spec".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("coh".to_string(), TypeTerm::Any),
            ("phase".to_string(), TypeTerm::Any),
            ("n.used".to_string(), TypeTerm::Any),
            ("series".to_string(), TypeTerm::Char),
            ("method".to_string(), TypeTerm::Char),
            ("snames".to_string(), TypeTerm::Any),
        ])),
        "stats::nls" => Some(TypeTerm::NamedList(vec![
            ("m".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
            (
                "convInfo".to_string(),
                TypeTerm::NamedList(vec![
                    ("isConv".to_string(), TypeTerm::Logical),
                    ("finIter".to_string(), TypeTerm::Int),
                    ("finTol".to_string(), TypeTerm::Double),
                    ("stopCode".to_string(), TypeTerm::Int),
                    ("stopMessage".to_string(), TypeTerm::Char),
                ]),
            ),
            ("data".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
            (
                "dataClasses".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "control".to_string(),
                TypeTerm::NamedList(vec![
                    ("maxiter".to_string(), TypeTerm::Double),
                    ("tol".to_string(), TypeTerm::Double),
                    ("minFactor".to_string(), TypeTerm::Double),
                    ("printEval".to_string(), TypeTerm::Logical),
                    ("warnOnly".to_string(), TypeTerm::Logical),
                    ("scaleOffset".to_string(), TypeTerm::Double),
                    ("nDcentral".to_string(), TypeTerm::Logical),
                ]),
            ),
        ])),
        "stats::nls.control" => Some(TypeTerm::NamedList(vec![
            ("maxiter".to_string(), TypeTerm::Double),
            ("tol".to_string(), TypeTerm::Double),
            ("minFactor".to_string(), TypeTerm::Double),
            ("printEval".to_string(), TypeTerm::Logical),
            ("warnOnly".to_string(), TypeTerm::Logical),
            ("scaleOffset".to_string(), TypeTerm::Double),
            ("nDcentral".to_string(), TypeTerm::Logical),
        ])),
        "stats::getInitial" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
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
        "stats::ecdf" => Some(TypeTerm::Any),
        "methods::isClass"
        | "methods::isGeneric"
        | "methods::hasMethod"
        | "methods::existsMethod"
        | "methods::existsFunction"
        | "methods::hasLoadAction"
        | "methods::hasArg"
        | "methods::hasMethods"
        | "methods::isGroup"
        | "methods::isGrammarSymbol"
        | "methods::isRematched"
        | "methods::isXS3Class"
        | "methods::is"
        | "methods::validObject"
        | "methods::isVirtualClass"
        | "methods::isClassUnion"
        | "methods::isSealedClass"
        | "methods::isSealedMethod"
        | "methods::isClassDef"
        | "methods::testVirtual"
        | "methods::canCoerce" => Some(TypeTerm::Logical),
        "methods::slotNames"
        | "methods::getClasses"
        | "methods::getSlots"
        | "methods::getGroupMembers"
        | "methods::formalArgs"
        | "methods::getAllSuperClasses" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "methods::getPackageName" => Some(TypeTerm::Char),
        "methods::getClass"
        | "methods::getClassDef"
        | "methods::findMethods"
        | "methods::findClass"
        | "methods::findUnique" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "methods::classesToAM" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "methods::findMethodSignatures" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "methods::cacheMetaData" => Some(TypeTerm::Null),
        "methods::getLoadActions" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "methods::extends" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "methods::getGenerics" | "methods::getGroup" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "methods::findMethod" | "methods::getMethodsForDispatch" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "methods::setGeneric" | "methods::setMethod" => Some(TypeTerm::Char),
        "methods::showExtends"
        | "methods::addNextMethod"
        | "methods::makeExtends"
        | "methods::setAs"
        | "methods::resetClass"
        | "methods::assignMethodsMetaData"
        | "methods::signature"
        | "methods::setReplaceMethod"
        | "methods::removeClass"
        | "methods::implicitGeneric"
        | "methods::el"
        | "methods::callGeneric"
        | "methods::showClass"
        | "methods::Complex"
        | "methods::evalSource"
        | "methods::coerce<-"
        | "methods::kronecker"
        | "methods::resetGeneric"
        | "methods::matrixOps"
        | "methods::possibleExtends"
        | "methods::missingArg"
        | "methods::Quote"
        | "methods::registerImplicitGenerics"
        | "methods::initRefFields"
        | "methods::insertMethod"
        | "methods::externalRefMethod"
        | "methods::fixPre1.8"
        | "methods::checkAtAssignment"
        | "methods::finalDefaultMethod"
        | "methods::completeClassDefinition"
        | "methods::callNextMethod"
        | "methods::selectSuperClasses"
        | "methods::removeMethods"
        | "methods::evalOnLoad"
        | "methods::cbind2"
        | "methods::setClassUnion"
        | "methods::initialize"
        | "methods::Summary"
        | "methods::representation"
        | "methods::method.skeleton"
        | "methods::setRefClass"
        | "methods::Math2"
        | "methods::getMethods"
        | "methods::S3Part<-"
        | "methods::Logic"
        | "methods::matchSignature"
        | "methods::methodsPackageMetaName"
        | "methods::defaultDumpName"
        | "methods::substituteDirect"
        | "methods::packageSlot"
        | "methods::as<-"
        | "methods::removeMethod"
        | "methods::MethodsListSelect"
        | "methods::S3Part"
        | "methods::checkSlotAssignment"
        | "methods::classMetaName"
        | "methods::slotsFromS3"
        | "methods::promptMethods"
        | "methods::insertClassMethods"
        | "methods::packageSlot<-"
        | "methods::languageEl<-" => Some(TypeTerm::Any),
        "methods::.slotNames" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "methods::.hasSlot" => Some(TypeTerm::Logical),
        "methods::validSlotNames" | "methods::inheritedSlotNames" | "methods::allNames" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "methods::classLabel" | "methods::className" | "methods::setPackageName" => {
            Some(TypeTerm::Char)
        }
        "methods::.__C__signature"
        | "methods::.__C__packageInfo"
        | "methods::.__C__MethodSelectionReport"
        | "methods::.__C__uninitializedField"
        | "methods::.__C__ordered"
        | "methods::.__C__oldClass"
        | "methods::.__C__groupGenericFunction"
        | "methods::.__C__standardGeneric"
        | "methods::.__C__nonstandardGroupGenericFunction"
        | "methods::.__C__S3"
        | "methods::.__C__array"
        | "methods::.__C__S4"
        | "methods::.__C__SuperClassMethod"
        | "methods::.__C__aov"
        | "methods::.__C__integrate"
        | "methods::.__C__listOfMethods"
        | "methods::.__C__ClassUnionRepresentation"
        | "methods::.__C__refObject"
        | "methods::.__C__.Other"
        | "methods::.__C__classPrototypeDef"
        | "methods::.__C__ts"
        | "methods::.__C__table"
        | "methods::.__C__double"
        | "methods::.__C__environment"
        | "methods::.__C__Date"
        | "methods::.__C__character"
        | "methods::.__C__LinearMethodsList"
        | "methods::.__C__structure"
        | "methods::.__C__lm"
        | "methods::.__C__dump.frames"
        | "methods::.__C__density"
        | "methods::.__C__vector"
        | "methods::.__C__repeat"
        | "methods::.__C__className"
        | "methods::.__C__name"
        | "methods::.__C__ObjectsWithPackage"
        | "methods::.__C__glm.null"
        | "methods::.__C__defaultBindingFunction"
        | "methods::.__C__("
        | "methods::.__C__special"
        | "methods::.__C__SealedMethodDefinition"
        | "methods::.__C__list"
        | "methods::.__C__NULL"
        | "methods::.__C__.environment"
        | "methods::.__C__genericFunctionWithTrace"
        | "methods::.__C__anova"
        | "methods::.__C__socket"
        | "methods::.__C__refGeneratorSlot"
        | "methods::.__C__integer"
        | "methods::.__C__packageIQR"
        | "methods::.__C__envRefClass"
        | "methods::.__C__complex"
        | "methods::.__C__classRepresentation"
        | "methods::.__C__libraryIQR"
        | "methods::.__C__OptionalFunction"
        | "methods::.__C__missing"
        | "methods::.__C__refMethodDef"
        | "methods::.__C__genericFunction"
        | "methods::.__C__classGeneratorFunction"
        | "methods::.__C__raw"
        | "methods::.__C__mlm"
        | "methods::.__C__POSIXct"
        | "methods::.__C__{"
        | "methods::.__C__groupGenericFunctionWithTrace"
        | "methods::.__C__data.frameRowLabels"
        | "methods::.__C__activeBindingFunction"
        | "methods::.__C__externalptr"
        | "methods::.__C__.name"
        | "methods::.__C__recordedplot"
        | "methods::.__C__localRefClass"
        | "methods::.__C__POSIXt"
        | "methods::.__C__summary.table"
        | "methods::.__C__language"
        | "methods::.__C__refClass"
        | "methods::.__C__numeric"
        | "methods::.__C__derivedDefaultMethodWithTrace"
        | "methods::.__C__externalRefMethod"
        | "methods::.__C__MethodDefinitionWithTrace"
        | "methods::.__C__maov"
        | "methods::.__C__standardGenericWithTrace"
        | "methods::.__C__.NULL"
        | "methods::.__C__PossibleMethod"
        | "methods::.__C__functionWithTrace"
        | "methods::.__C__factor"
        | "methods::.__C__sourceEnvironment"
        | "methods::.__C__MethodDefinition"
        | "methods::.__C__mts"
        | "methods::.__C__mtable"
        | "methods::.__C__data.frame"
        | "methods::.__C__if"
        | "methods::.__C__optionalMethod"
        | "methods::.__C__ANY"
        | "methods::.__C__refClassRepresentation"
        | "methods::.__C__conditionalExtension"
        | "methods::.__C__traceable"
        | "methods::.__C__MethodWithNextWithTrace"
        | "methods::.__C__anova.glm.null"
        | "methods::.__C__.externalptr"
        | "methods::.__C__matrix"
        | "methods::.__C__hsearch"
        | "methods::.__C__function"
        | "methods::.__C__POSIXlt"
        | "methods::.__C__logical"
        | "methods::.__C__nonstandardGenericWithTrace"
        | "methods::.__C__summaryDefault"
        | "methods::.__C__derivedDefaultMethod"
        | "methods::.__C__nonstandardGeneric"
        | "methods::.__C__glm"
        | "methods::.__C__nonstandardGenericFunction"
        | "methods::.__C__refObjectGenerator"
        | "methods::.__C__builtin"
        | "methods::.__C__for"
        | "methods::.__C__internalDispatchMethod"
        | "methods::.__C__anova.glm"
        | "methods::.__C__<-"
        | "methods::.__C__nonStructure"
        | "methods::.__C__call"
        | "methods::.__C__MethodWithNext"
        | "methods::.__C__rle"
        | "methods::.__C__logLik"
        | "methods::.__C__namedList"
        | "methods::.__C__formula"
        | "methods::.__C__while"
        | "methods::.__C__expression"
        | "methods::.__C__refMethodDefWithTrace"
        | "methods::.__C__VIRTUAL"
        | "methods::.__C__SClassExtension" => Some(TypeTerm::Any),
        "methods::.EmptyPrimitiveSkeletons"
        | "methods::.OldClassesList"
        | "methods::.S4methods"
        | "methods::.ShortPrimitiveSkeletons"
        | "methods::.classEnv"
        | "methods::.doTracePrint"
        | "methods::.selectSuperClasses"
        | "methods::.untracedFunction"
        | "methods::.valueClassTest"
        | "methods::.__T__Logic:base"
        | "methods::.__T__loadMethod:methods"
        | "methods::.__T__[<-:base"
        | "methods::.__T__coerce:methods"
        | "methods::.__T__matrixOps:base"
        | "methods::.__T__show:methods"
        | "methods::.__T__body<-:base"
        | "methods::.__T__cbind2:methods"
        | "methods::.__T__Arith:base"
        | "methods::.__T__[[<-:base"
        | "methods::.__T__$:base"
        | "methods::.__T__Compare:methods"
        | "methods::.__T__Math2:methods"
        | "methods::.__T__slotsFromS3:methods"
        | "methods::.__T__Complex:base"
        | "methods::.__T__coerce<-:methods"
        | "methods::.__T__kronecker:base"
        | "methods::.__T__rbind2:methods"
        | "methods::.__T__initialize:methods"
        | "methods::.__T__$<-:base"
        | "methods::.__T__addNextMethod:methods"
        | "methods::.__T__Ops:base"
        | "methods::.__T__Math:base"
        | "methods::.__T__Summary:base"
        | "methods::.__T__[:base"
        | "methods::emptyMethodsList"
        | "methods::listFromMethods"
        | "methods::metaNameUndo"
        | "methods::Math"
        | "methods::makePrototypeFromClassDef"
        | "methods::reconcilePropertiesAndPrototype"
        | "methods::doPrimitiveMethod"
        | "methods::SignatureMethod"
        | "methods::setIs"
        | "methods::balanceMethodsList"
        | "methods::setLoadActions"
        | "methods::setDataPart"
        | "methods::setGroupGeneric"
        | "methods::S3Class"
        | "methods::dumpMethods"
        | "methods::sigToEnv"
        | "methods::setGenericImplicit"
        | "methods::sealClass"
        | "methods::makeStandardGeneric"
        | "methods::loadMethod"
        | "methods::prohibitGeneric"
        | "methods::substituteFunctionArgs"
        | "methods::completeExtends"
        | "methods::assignClassDef"
        | "methods::methodSignatureMatrix"
        | "methods::body<-"
        | "methods::prototype"
        | "methods::requireMethods"
        | "methods::slot<-"
        | "methods::setOldClass"
        | "methods::setPrimitiveMethods"
        | "methods::rematchDefinition"
        | "methods::MethodAddCoerce"
        | "methods::evalqOnLoad"
        | "methods::insertSource"
        | "methods::multipleClasses"
        | "methods::empty.dump"
        | "methods::el<-"
        | "methods::elNamed<-"
        | "methods::newEmptyObject"
        | "methods::Arith"
        | "methods::mergeMethods"
        | "methods::MethodsList"
        | "methods::asMethodDefinition"
        | "methods::languageEl"
        | "methods::Ops"
        | "methods::completeSubclasses"
        | "methods::cacheGenericsMetaData"
        | "methods::tryNew"
        | "methods::showDefault"
        | "methods::Compare"
        | "methods::makeClassRepresentation"
        | "methods::promptClass"
        | "methods::newClassRepresentation"
        | "methods::removeGeneric"
        | "methods::S3Class<-"
        | "methods::rbind2"
        | "methods::setValidity"
        | "methods::functionBody"
        | "methods::dumpMethod"
        | "methods::elNamed"
        | "methods::generic.skeleton"
        | "methods::makeGeneric"
        | "methods::coerce"
        | "methods::initFieldArgs"
        | "methods::unRematchDefinition"
        | "methods::defaultPrototype"
        | "methods::showMethods"
        | "methods::as"
        | "methods::conformMethod"
        | "methods::makeMethodsList"
        | "methods::newBasic"
        | "methods::functionBody<-"
        | "methods::setLoadAction"
        | "methods::superClassDepth"
        | "methods::getMethodsMetaData" => Some(TypeTerm::Any),
        "methods::getGeneric"
        | "methods::cacheMethod"
        | "methods::getFunction"
        | "methods::getRefClass"
        | "methods::findFunction"
        | "methods::getDataPart"
        | "methods::selectMethod"
        | "methods::new"
        | "methods::slot"
        | "methods::getMethod"
        | "methods::getValidity"
        | "methods::testInheritedMethods"
        | "methods::standardGeneric"
        | "methods::setClass" => Some(TypeTerm::Any),
        "methods::show" => Some(TypeTerm::Null),
        "compiler::enableJIT" => Some(TypeTerm::Int),
        "compiler::compilePKGS" => Some(TypeTerm::Logical),
        "compiler::getCompilerOption" => Some(TypeTerm::Any),
        "compiler::setCompilerOptions" | "compiler::compile" | "compiler::disassemble" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "compiler::cmpfile" | "compiler::loadcmp" => Some(TypeTerm::Null),
        "compiler::cmpfun" => Some(TypeTerm::Any),
        callee if callee.starts_with("dplyr::") => Some(TypeTerm::Any),
        callee if callee.starts_with("readr::") || callee.starts_with("tidyr::") => {
            Some(TypeTerm::Any)
        }
        "utils::head" | "utils::tail" => Some(preserved_head_tail_term(first_arg_term(arg_terms))),
        "utils::packageVersion"
        | "utils::citation"
        | "utils::person"
        | "utils::as.person"
        | "utils::as.personList" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "utils::getAnywhere" => Some(TypeTerm::NamedList(vec![
            ("name".to_string(), TypeTerm::Char),
            ("objs".to_string(), TypeTerm::List(Box::new(TypeTerm::Any))),
            (
                "where".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "visible".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
            (
                "dups".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
        ])),
        "utils::packageDescription" => Some(TypeTerm::NamedList(vec![
            ("Package".to_string(), TypeTerm::Char),
            ("Version".to_string(), TypeTerm::Char),
            ("Priority".to_string(), TypeTerm::Char),
            ("Title".to_string(), TypeTerm::Char),
            ("Author".to_string(), TypeTerm::Char),
            ("Maintainer".to_string(), TypeTerm::Char),
            ("Contact".to_string(), TypeTerm::Char),
            ("Description".to_string(), TypeTerm::Char),
            ("License".to_string(), TypeTerm::Char),
            ("Imports".to_string(), TypeTerm::Char),
            ("Suggests".to_string(), TypeTerm::Char),
            ("NeedsCompilation".to_string(), TypeTerm::Char),
            ("Encoding".to_string(), TypeTerm::Char),
            ("Enhances".to_string(), TypeTerm::Char),
            ("Built".to_string(), TypeTerm::Char),
        ])),
        "utils::sessionInfo" => Some(TypeTerm::NamedList(vec![
            ("R.version".to_string(), TypeTerm::Any),
            ("platform".to_string(), TypeTerm::Char),
            ("locale".to_string(), TypeTerm::Char),
            ("tzone".to_string(), TypeTerm::Char),
            ("tzcode_type".to_string(), TypeTerm::Char),
            ("running".to_string(), TypeTerm::Char),
            (
                "RNGkind".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "basePkgs".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("loadedOnly".to_string(), TypeTerm::Any),
            ("matprod".to_string(), TypeTerm::Char),
            ("BLAS".to_string(), TypeTerm::Char),
            ("LAPACK".to_string(), TypeTerm::Char),
            ("LA_version".to_string(), TypeTerm::Char),
        ])),
        "utils::as.roman" => match first_arg_term(arg_terms) {
            TypeTerm::Vector(_)
            | TypeTerm::VectorLen(_, _)
            | TypeTerm::Matrix(_)
            | TypeTerm::MatrixDim(_, _, _)
            | TypeTerm::ArrayDim(_, _) => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
            _ => Some(TypeTerm::Int),
        },
        "utils::hasName" => Some(TypeTerm::Logical),
        "utils::strcapture" => Some(TypeTerm::DataFrame(Vec::new())),
        "utils::contrib.url" => Some(TypeTerm::Char),
        "utils::localeToCharset" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::charClass" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "utils::findMatches" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::fileSnapshot" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "utils::apropos" | "utils::find" | "utils::methods" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "utils::help.search" => Some(TypeTerm::NamedList(vec![
            ("pattern".to_string(), TypeTerm::Char),
            (
                "fields".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("type".to_string(), TypeTerm::Char),
            ("agrep".to_string(), TypeTerm::Any),
            ("ignore.case".to_string(), TypeTerm::Logical),
            (
                "types".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("package".to_string(), TypeTerm::Any),
            ("lib.loc".to_string(), TypeTerm::Char),
            (
                "matches".to_string(),
                TypeTerm::DataFrameNamed(vec![
                    (
                        "Topic".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "Title".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "Name".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    ("ID".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Char))),
                    (
                        "Package".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "LibPath".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "Type".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "Field".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                    (
                        "Entry".to_string(),
                        TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    ),
                ]),
            ),
        ])),
        "utils::data" => Some(TypeTerm::NamedList(vec![
            ("title".to_string(), TypeTerm::Char),
            ("header".to_string(), TypeTerm::Any),
            (
                "results".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Char)),
            ),
            ("footer".to_string(), TypeTerm::Char),
        ])),
        "utils::argsAnywhere" => Some(TypeTerm::Any),
        "utils::compareVersion" => Some(TypeTerm::Double),
        "utils::capture.output" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::file_test" => match arg_terms.get(1).cloned().unwrap_or(TypeTerm::Any) {
            TypeTerm::Vector(_)
            | TypeTerm::VectorLen(_, _)
            | TypeTerm::Matrix(_)
            | TypeTerm::MatrixDim(_, _, _)
            | TypeTerm::ArrayDim(_, _) => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
            _ => Some(TypeTerm::Logical),
        },
        "utils::URLencode" | "utils::URLdecode" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "utils::head.matrix" | "utils::tail.matrix" => {
            Some(preserved_head_tail_term(first_arg_term(arg_terms)))
        }
        "utils::available.packages" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "utils::stack" | "utils::unstack" => Some(TypeTerm::DataFrame(Vec::new())),
        "utils::strOptions" => Some(TypeTerm::NamedList(vec![
            ("strict.width".to_string(), TypeTerm::Char),
            ("digits.d".to_string(), TypeTerm::Int),
            ("vec.len".to_string(), TypeTerm::Int),
            ("list.len".to_string(), TypeTerm::Int),
            ("deparse.lines".to_string(), TypeTerm::Any),
            ("drop.deparse.attr".to_string(), TypeTerm::Logical),
            ("formatNum".to_string(), TypeTerm::Any),
        ])),
        "utils::txtProgressBar" => Some(TypeTerm::Any),
        "utils::toBibtex" | "utils::toLatex" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::getTxtProgressBar" | "utils::setTxtProgressBar" => Some(TypeTerm::Double),
        "utils::modifyList"
        | "utils::relist"
        | "utils::as.relistable"
        | "utils::personList"
        | "utils::warnErrList"
        | "utils::readCitationFile"
        | "utils::bibentry"
        | "utils::citEntry"
        | "utils::citHeader"
        | "utils::citFooter" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "utils::getSrcref" | "utils::getFromNamespace" | "utils::getS3method" => {
            Some(TypeTerm::Any)
        }
        "utils::getParseData" => Some(TypeTerm::DataFrame(Vec::new())),
        "utils::getParseText" | "utils::globalVariables" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "utils::getSrcFilename" | "utils::getSrcDirectory" => Some(TypeTerm::Char),
        "utils::getSrcLocation" => Some(TypeTerm::Any),
        "utils::hashtab" | "utils::gethash" => Some(TypeTerm::Any),
        "utils::sethash" => Some(TypeTerm::Any),
        "utils::remhash" | "utils::is.hashtab" => Some(TypeTerm::Logical),
        "utils::clrhash" | "utils::maphash" => Some(TypeTerm::Null),
        "utils::numhash" => Some(TypeTerm::Int),
        "utils::typhash" => Some(TypeTerm::Char),
        "utils::asDateBuilt" => Some(TypeTerm::Double),
        "utils::findLineNum" => Some(TypeTerm::Any),
        "utils::getCRANmirrors" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Name".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Country".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "City".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "URL".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Host".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Maintainer".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("OK".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            (
                "CountryCode".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Comment".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "utils::findCRANmirror" => Some(TypeTerm::Char),
        "utils::package.skeleton" | "utils::unzip" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "utils::zip" => Some(TypeTerm::Int),
        "utils::limitedLabels"
        | "utils::formatOL"
        | "utils::formatUL"
        | "utils::ls.str"
        | "utils::lsf.str" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::news" => Some(TypeTerm::Null),
        "utils::vignette" => Some(TypeTerm::NamedList(vec![
            ("type".to_string(), TypeTerm::Char),
            ("title".to_string(), TypeTerm::Char),
            ("header".to_string(), TypeTerm::Any),
            ("results".to_string(), TypeTerm::DataFrame(Vec::new())),
            ("footer".to_string(), TypeTerm::Any),
        ])),
        "utils::hsearch_db" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "utils::hsearch_db_concepts" | "utils::hsearch_db_keywords" => {
            Some(TypeTerm::DataFrame(Vec::new()))
        }
        "utils::browseEnv"
        | "utils::browseURL"
        | "utils::browseVignettes"
        | "utils::bug.report"
        | "utils::checkCRAN"
        | "utils::chooseBioCmirror"
        | "utils::chooseCRANmirror"
        | "utils::create.post"
        | "utils::data.entry"
        | "utils::dataentry"
        | "utils::debugcall"
        | "utils::debugger"
        | "utils::demo"
        | "utils::dump.frames"
        | "utils::edit"
        | "utils::emacs"
        | "utils::example"
        | "utils::file.edit"
        | "utils::fix"
        | "utils::fixInNamespace"
        | "utils::flush.console"
        | "utils::help.request"
        | "utils::help.start"
        | "utils::page"
        | "utils::pico"
        | "utils::process.events"
        | "utils::prompt"
        | "utils::promptData"
        | "utils::promptImport"
        | "utils::promptPackage"
        | "utils::recover"
        | "utils::removeSource"
        | "utils::RShowDoc"
        | "utils::RSiteSearch"
        | "utils::rtags"
        | "utils::setBreakpoint"
        | "utils::suppressForeignCheck"
        | "utils::undebugcall"
        | "utils::url.show"
        | "utils::vi"
        | "utils::View"
        | "utils::xedit"
        | "utils::xemacs" => Some(TypeTerm::Null),
        "utils::tar" => Some(TypeTerm::Int),
        "utils::untar" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::timestamp" => Some(TypeTerm::Char),
        "utils::Rprof" | "utils::Rprofmem" => Some(TypeTerm::Null),
        "utils::summaryRprof" => Some(TypeTerm::NamedList(vec![
            ("by.self".to_string(), TypeTerm::Any),
            ("by.total".to_string(), TypeTerm::Any),
            ("sample.interval".to_string(), TypeTerm::Double),
            ("sampling.time".to_string(), TypeTerm::Double),
        ])),
        "utils::setRepositories" => Some(TypeTerm::NamedList(vec![(
            "repos".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Char)),
        )])),
        "utils::?"
        | "utils::.AtNames"
        | "utils::.DollarNames"
        | "utils::cite"
        | "utils::citeNatbib"
        | "utils::help"
        | "utils::read.socket"
        | "utils::RweaveChunkPrefix" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "utils::.romans" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "utils::askYesNo" | "utils::isS3method" | "utils::isS3stdGeneric" => {
            Some(TypeTerm::Logical)
        }
        "utils::rc.settings" => Some(TypeTerm::Vector(Box::new(TypeTerm::Logical))),
        "utils::download.file" => Some(TypeTerm::Int),
        "utils::alarm" | "utils::rc.getOption" => Some(TypeTerm::Any),
        "utils::close.socket"
        | "utils::history"
        | "utils::loadhistory"
        | "utils::savehistory"
        | "utils::write.socket" => Some(TypeTerm::Null),
        "utils::.checkHT"
        | "utils::.RtangleCodeLabel"
        | "utils::.S3methods"
        | "utils::aregexec"
        | "utils::aspell"
        | "utils::aspell_package_C_files"
        | "utils::aspell_package_R_files"
        | "utils::aspell_package_Rd_files"
        | "utils::aspell_package_vignettes"
        | "utils::aspell_write_personal_dictionary_file"
        | "utils::assignInMyNamespace"
        | "utils::assignInNamespace"
        | "utils::changedFiles"
        | "utils::de"
        | "utils::de.ncols"
        | "utils::de.restore"
        | "utils::de.setup"
        | "utils::download.packages"
        | "utils::install.packages"
        | "utils::make.packages.html"
        | "utils::make.socket"
        | "utils::makeRweaveLatexCodeRunner"
        | "utils::mirror2html"
        | "utils::new.packages"
        | "utils::old.packages"
        | "utils::packageStatus"
        | "utils::rc.options"
        | "utils::rc.status"
        | "utils::remove.packages"
        | "utils::Rtangle"
        | "utils::RtangleFinish"
        | "utils::RtangleRuncode"
        | "utils::RtangleSetup"
        | "utils::RtangleWritedoc"
        | "utils::RweaveEvalWithOpt"
        | "utils::RweaveLatex"
        | "utils::RweaveLatexFinish"
        | "utils::RweaveLatexOptions"
        | "utils::RweaveLatexSetup"
        | "utils::RweaveLatexWritedoc"
        | "utils::RweaveTryStop"
        | "utils::Stangle"
        | "utils::Sweave"
        | "utils::SweaveHooks"
        | "utils::SweaveSyntaxLatex"
        | "utils::SweaveSyntaxNoweb"
        | "utils::SweaveSyntConv"
        | "utils::update.packages"
        | "utils::upgrade" => Some(TypeTerm::Any),
        "utils::is.relistable" => Some(TypeTerm::Logical),
        "utils::packageName" | "utils::osVersion" | "utils::nsl" | "utils::select.list" => {
            Some(TypeTerm::Char)
        }
        "utils::menu" => Some(TypeTerm::Int),
        "utils::glob2rx" => Some(TypeTerm::Char),
        "utils::installed.packages" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "utils::maintainer" => Some(TypeTerm::Char),
        "utils::packageDate"
        | "utils::object.size"
        | "utils::memory.size"
        | "utils::memory.limit" => Some(TypeTerm::Double),
        "utils::read.csv"
        | "utils::read.csv2"
        | "utils::read.table"
        | "utils::read.delim"
        | "utils::read.fwf"
        | "utils::read.delim2"
        | "utils::read.DIF"
        | "utils::read.fortran" => Some(TypeTerm::DataFrame(Vec::new())),
        "utils::write.csv" | "utils::write.csv2" | "utils::write.table" | "utils::str" => {
            Some(TypeTerm::Null)
        }
        "utils::count.fields" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "utils::adist" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "utils::combn" => {
            let elem = match first_arg_term(arg_terms) {
                TypeTerm::Vector(inner)
                | TypeTerm::VectorLen(inner, _)
                | TypeTerm::Matrix(inner)
                | TypeTerm::MatrixDim(inner, _, _)
                | TypeTerm::ArrayDim(inner, _) => inner.as_ref().clone(),
                term => term,
            };
            Some(TypeTerm::Matrix(Box::new(elem)))
        }
        "utils::type.convert" => Some(vectorized_first_arg_term(first_arg_term(arg_terms))),
        "tools::toTitleCase" | "tools::file_path_as_absolute" | "tools::R_user_dir" => {
            Some(TypeTerm::Char)
        }
        "tools::md5sum" | "tools::sha256sum" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "tools::file_ext" | "tools::file_path_sans_ext" => {
            Some(char_like_first_arg_term(first_arg_term(arg_terms)))
        }
        "tools::list_files_with_exts" | "tools::list_files_with_type" | "tools::dependsOnPkgs" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Char)))
        }
        "tools::getVignetteInfo" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "tools::pkgVignettes" => Some(TypeTerm::NamedList(vec![
            (
                "docs".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "names".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "engines".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "patterns".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "encodings".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("dir".to_string(), TypeTerm::Char),
            ("pkgdir".to_string(), TypeTerm::Char),
            (
                "msg".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "tools::delimMatch" => Some(TypeTerm::Int),
        "tools::parse_URI_reference" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "scheme".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "authority".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "path".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "query".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "fragment".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "tools::encoded_text_to_latex" => Some(char_like_first_arg_term(first_arg_term(arg_terms))),
        "tools::parse_Rd" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "tools::Rd2txt_options" => Some(TypeTerm::NamedList(vec![
            ("width".to_string(), TypeTerm::Int),
            ("minIndent".to_string(), TypeTerm::Int),
            ("extraIndent".to_string(), TypeTerm::Int),
            ("sectionIndent".to_string(), TypeTerm::Int),
            ("sectionExtra".to_string(), TypeTerm::Int),
            ("itemBullet".to_string(), TypeTerm::Char),
            ("enumFormat".to_string(), TypeTerm::Any),
            ("showURLs".to_string(), TypeTerm::Logical),
            ("code_quote".to_string(), TypeTerm::Logical),
            ("underline_titles".to_string(), TypeTerm::Logical),
        ])),
        "tools::Rd2HTML" | "tools::Rd2latex" | "tools::Rd2ex" => Some(TypeTerm::Char),
        "tools::Rd2txt"
        | "tools::RdTextFilter"
        | "tools::checkRd"
        | "tools::showNonASCII"
        | "tools::showNonASCIIfile" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "tools::find_gs_cmd"
        | "tools::findHTMLlinks"
        | "tools::makevars_site"
        | "tools::makevars_user"
        | "tools::HTMLheader"
        | "tools::SweaveTeXFilter"
        | "tools::toHTML"
        | "tools::toRd"
        | "tools::charset_to_Unicode" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "tools::Rdindex" => Some(TypeTerm::Null),
        "tools::read.00Index" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "tools::parseLatex" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "tools::getBibstyle" => Some(TypeTerm::Char),
        "tools::deparseLatex" => Some(TypeTerm::Char),
        "tools::latexToUtf8" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "tools::SIGCHLD" | "tools::SIGCONT" | "tools::SIGHUP" | "tools::SIGINT"
        | "tools::SIGKILL" | "tools::SIGQUIT" | "tools::SIGSTOP" | "tools::SIGTERM"
        | "tools::SIGTSTP" | "tools::SIGUSR1" | "tools::SIGUSR2" => Some(TypeTerm::Int),
        "tools::assertCondition"
        | "tools::assertError"
        | "tools::assertWarning"
        | "tools::add_datalist"
        | "tools::buildVignette"
        | "tools::buildVignettes"
        | "tools::compactPDF"
        | "tools::installFoundDepends"
        | "tools::make_translations_pkg"
        | "tools::package_native_routine_registration_skeleton"
        | "tools::pskill"
        | "tools::psnice"
        | "tools::resaveRdaFiles"
        | "tools::startDynamicHelp"
        | "tools::testInstalledBasic"
        | "tools::testInstalledPackage"
        | "tools::testInstalledPackages"
        | "tools::texi2dvi"
        | "tools::texi2pdf"
        | "tools::update_PACKAGES"
        | "tools::update_pkg_po"
        | "tools::write_PACKAGES"
        | "tools::xgettext"
        | "tools::xgettext2pot"
        | "tools::xngettext" => Some(TypeTerm::Null),
        "tools::Adobe_glyphs" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "adobe".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "unicode".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "tools::.print.via.format"
        | "tools::analyze_license"
        | "tools::as.Rconcordance"
        | "tools::bibstyle"
        | "tools::check_package_dois"
        | "tools::check_package_urls"
        | "tools::check_packages_in_dir"
        | "tools::check_packages_in_dir_changes"
        | "tools::check_packages_in_dir_details"
        | "tools::checkDocFiles"
        | "tools::checkDocStyle"
        | "tools::checkFF"
        | "tools::checkMD5sums"
        | "tools::checkPoFile"
        | "tools::checkPoFiles"
        | "tools::checkRdaFiles"
        | "tools::checkRdContents"
        | "tools::checkReplaceFuns"
        | "tools::checkS3methods"
        | "tools::checkTnF"
        | "tools::checkVignettes"
        | "tools::codoc"
        | "tools::codocClasses"
        | "tools::codocData"
        | "tools::followConcordance"
        | "tools::getDepList"
        | "tools::langElts"
        | "tools::loadPkgRdMacros"
        | "tools::loadRdMacros"
        | "tools::matchConcordance"
        | "tools::nonS3methods"
        | "tools::package.dependencies"
        | "tools::pkg2HTML"
        | "tools::pkgDepends"
        | "tools::R"
        | "tools::Rcmd"
        | "tools::Rdiff"
        | "tools::summarize_check_packages_in_dir_depends"
        | "tools::summarize_check_packages_in_dir_results"
        | "tools::summarize_check_packages_in_dir_timings"
        | "tools::undoc"
        | "tools::vignetteDepends"
        | "tools::vignetteEngine"
        | "tools::vignetteInfo" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "tools::standard_package_names"
        | "tools::base_aliases_db"
        | "tools::base_rdxrefs_db"
        | "tools::CRAN_aliases_db"
        | "tools::CRAN_archive_db"
        | "tools::CRAN_rdxrefs_db" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "tools::CRAN_package_db"
        | "tools::CRAN_authors_db"
        | "tools::CRAN_current_db"
        | "tools::CRAN_check_results"
        | "tools::CRAN_check_details"
        | "tools::CRAN_check_issues" => Some(TypeTerm::DataFrame(Vec::new())),
        "tools::summarize_CRAN_check_status" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "tools::package_dependencies" | "tools::Rd_db" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "parallel::detectCores" => Some(TypeTerm::Int),
        "parallel::makeCluster"
        | "parallel::makeForkCluster"
        | "parallel::makePSOCKcluster"
        | "parallel::parLapply"
        | "parallel::parLapplyLB"
        | "parallel::clusterEvalQ"
        | "parallel::clusterMap"
        | "parallel::clusterApply"
        | "parallel::clusterCall"
        | "parallel::mclapply"
        | "parallel::mcMap"
        | "parallel::clusterSplit"
        | "parallel::splitIndices"
        | "parallel::getDefaultCluster"
        | "parallel::recvData"
        | "parallel::recvOneData"
        | "parallel::clusterApplyLB" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "parallel::parSapply"
        | "parallel::parSapplyLB"
        | "parallel::parApply"
        | "parallel::parCapply"
        | "parallel::parRapply"
        | "parallel::pvec"
        | "parallel::mcmapply" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "parallel::nextRNGStream" | "parallel::nextRNGSubStream" | "parallel::mcaffinity" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Int)))
        }
        "parallel::mcparallel" | "parallel::mccollect" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "splines::bs" | "splines::ns" | "splines::splineDesign" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "splines::interpSpline" | "splines::periodicSpline" | "splines::backSpline" => {
            Some(TypeTerm::NamedList(vec![
                (
                    "knots".to_string(),
                    TypeTerm::Vector(Box::new(TypeTerm::Double)),
                ),
                ("coefficients".to_string(), TypeTerm::Any),
            ]))
        }
        "splines::xyVector" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "splines::spline.des" => Some(TypeTerm::NamedList(vec![
            (
                "knots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("order".to_string(), TypeTerm::Double),
            (
                "derivs".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "design".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "splines::as.polySpline" => Some(TypeTerm::NamedList(vec![
            (
                "knots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("coefficients".to_string(), TypeTerm::Any),
        ])),
        "splines::polySpline" => Some(TypeTerm::NamedList(vec![
            (
                "knots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("coefficients".to_string(), TypeTerm::Any),
            ("period".to_string(), TypeTerm::Double),
        ])),
        "splines::asVector" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "splines::splineKnots" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "splines::splineOrder" => Some(TypeTerm::Int),
        "tcltk::tclObj" | "tcltk::as.tclObj" | "tcltk::tclVar" => Some(TypeTerm::Any),
        "tcltk::tclvalue" => Some(TypeTerm::Any),
        "tcltk::addTclPath" | "tcltk::tclRequire" => Some(TypeTerm::Any),
        "tcltk::tclVersion" => Some(TypeTerm::Char),
        "tcltk::tkProgressBar" => Some(TypeTerm::Any),
        "tcltk::getTkProgressBar" | "tcltk::setTkProgressBar" => Some(TypeTerm::Double),
        "tcltk::is.tclObj" | "tcltk::is.tkwin" => Some(TypeTerm::Logical),
        "tcltk::tclfile.dir" | "tcltk::tclfile.tail" => Some(TypeTerm::Char),
        callee
            if callee.starts_with("tcltk::tk")
                || callee.starts_with("tcltk::ttk")
                || callee.starts_with("tcltk::tcl")
                || callee.starts_with("tcltk::.Tcl")
                || callee.starts_with("tcltk::.Tk") =>
        {
            Some(TypeTerm::Any)
        }
        "stats4::mle"
        | "stats4::update"
        | "stats4::summary"
        | "stats4::profile"
        | "stats4::plot"
        | "stats4::.__C__mle"
        | "stats4::.__C__profile.mle"
        | "stats4::.__C__summary.mle"
        | "stats4::.__T__AIC:stats"
        | "stats4::.__T__BIC:stats"
        | "stats4::.__T__coef:stats"
        | "stats4::.__T__confint:stats"
        | "stats4::.__T__logLik:stats"
        | "stats4::.__T__nobs:stats"
        | "stats4::.__T__plot:base"
        | "stats4::.__T__profile:stats"
        | "stats4::.__T__show:methods"
        | "stats4::.__T__summary:base"
        | "stats4::.__T__update:stats"
        | "stats4::.__T__vcov:stats" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "stats4::coef" | "stats4::confint" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats4::vcov" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats4::logLik" | "stats4::AIC" | "stats4::BIC" => Some(TypeTerm::Double),
        "stats4::show" => Some(TypeTerm::Double),
        "stats4::nobs" => Some(TypeTerm::Int),
        "parallel::stopCluster"
        | "parallel::clusterExport"
        | "parallel::closeNode"
        | "parallel::clusterSetRNGStream"
        | "parallel::mc.reset.stream"
        | "parallel::sendData"
        | "parallel::registerClusterType"
        | "parallel::setDefaultCluster" => Some(TypeTerm::Null),
        "stats::median"
        | "stats::median.default"
        | "stats::sd"
        | "stats::AIC"
        | "stats::BIC"
        | "stats::logLik"
        | "stats::deviance"
        | "stats::sigma" => Some(TypeTerm::Double),
        "stats::nobs" | "stats::df.residual" => Some(TypeTerm::Int),
        "stats::quantile" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::predict"
        | "stats::predict.lm"
        | "stats::predict.glm"
        | "stats::coef"
        | "stats::coefficients"
        | "stats::fitted"
        | "stats::fitted.values"
        | "stats::resid"
        | "stats::residuals"
        | "stats::residuals.lm"
        | "stats::residuals.glm"
        | "stats::hatvalues"
        | "stats::hat"
        | "stats::cooks.distance"
        | "stats::covratio"
        | "stats::dffits"
        | "stats::rstandard"
        | "stats::rstudent"
        | "stats::weighted.residuals" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::simulate" => Some(TypeTerm::DataFrame(Vec::new())),
        "stats::summary.lm" => Some(TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "aliased".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
            ("sigma".to_string(), TypeTerm::Double),
            ("df".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            ("r.squared".to_string(), TypeTerm::Double),
            ("adj.r.squared".to_string(), TypeTerm::Double),
            (
                "fstatistic".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "cov.unscaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::summary.glm" => Some(TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            ("terms".to_string(), TypeTerm::Any),
            (
                "family".to_string(),
                TypeTerm::NamedList(vec![
                    ("family".to_string(), TypeTerm::Char),
                    ("link".to_string(), TypeTerm::Char),
                    ("linkfun".to_string(), TypeTerm::Any),
                    ("linkinv".to_string(), TypeTerm::Any),
                    ("variance".to_string(), TypeTerm::Any),
                    ("dev.resids".to_string(), TypeTerm::Any),
                    ("aic".to_string(), TypeTerm::Any),
                    ("mu.eta".to_string(), TypeTerm::Any),
                    ("initialize".to_string(), TypeTerm::Any),
                    ("validmu".to_string(), TypeTerm::Any),
                    ("valideta".to_string(), TypeTerm::Any),
                    ("dispersion".to_string(), TypeTerm::Double),
                ]),
            ),
            ("deviance".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            ("contrasts".to_string(), TypeTerm::Any),
            ("df.residual".to_string(), TypeTerm::Int),
            ("null.deviance".to_string(), TypeTerm::Double),
            ("df.null".to_string(), TypeTerm::Int),
            ("iter".to_string(), TypeTerm::Int),
            (
                "deviance.resid".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "aliased".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Logical)),
            ),
            ("dispersion".to_string(), TypeTerm::Double),
            ("df".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            (
                "cov.unscaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "cov.scaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::summary.aov" => Some(TypeTerm::List(Box::new(TypeTerm::DataFrame(Vec::new())))),
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
        "stats::summary.stepfun" => Some(TypeTerm::Null),
        "stats::toeplitz" | "stats::toeplitz2" | "stats::polym" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "stats::diffinv" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "stats::isoreg" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "yf".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "yc".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "iKnots".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("isOrd".to_string(), TypeTerm::Logical),
            ("ord".to_string(), TypeTerm::Any),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::line" => Some(TypeTerm::NamedList(vec![
            ("call".to_string(), TypeTerm::Any),
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "fitted.values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::varimax" | "stats::promax" => Some(TypeTerm::NamedList(vec![
            (
                "loadings".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "rotmat".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::asOneSidedFormula" => Some(TypeTerm::Any),
        "stats::variable.names" => Some(TypeTerm::Null),
        "stats::dfbeta" | "stats::dfbetas" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::influence" => Some(TypeTerm::NamedList(vec![
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "sigma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "wt.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::influence.measures" => Some(TypeTerm::NamedList(vec![
            (
                "infmat".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "is.inf".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Logical)),
            ),
            ("call".to_string(), TypeTerm::Any),
        ])),
        "stats::qr.influence" => Some(TypeTerm::NamedList(vec![
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "sigma".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::vcov"
        | "stats::confint"
        | "stats::confint.lm"
        | "stats::confint.default"
        | "stats::model.matrix"
        | "stats::model.matrix.default"
        | "stats::model.matrix.lm" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::anova" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Any))),
        "stats::model.frame" | "stats::model.frame.default" => {
            Some(TypeTerm::DataFrame(Vec::new()))
        }
        "stats::glm.fit" => Some(TypeTerm::NamedList(vec![
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "fitted.values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "effects".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "R".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            ("rank".to_string(), TypeTerm::Int),
            ("qr".to_string(), TypeTerm::Any),
            ("family".to_string(), TypeTerm::Any),
            (
                "linear.predictors".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("deviance".to_string(), TypeTerm::Double),
            ("aic".to_string(), TypeTerm::Double),
            ("null.deviance".to_string(), TypeTerm::Double),
            ("iter".to_string(), TypeTerm::Int),
            (
                "weights".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "prior.weights".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("df.residual".to_string(), TypeTerm::Int),
            ("df.null".to_string(), TypeTerm::Int),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("converged".to_string(), TypeTerm::Logical),
            ("boundary".to_string(), TypeTerm::Logical),
        ])),
        "stats::lm.fit" => Some(TypeTerm::NamedList(vec![
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
            ("assign".to_string(), TypeTerm::Any),
            ("qr".to_string(), TypeTerm::Any),
            ("df.residual".to_string(), TypeTerm::Int),
        ])),
        "stats::lm.wfit" => Some(TypeTerm::NamedList(vec![
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "fitted.values".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "effects".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "weights".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("rank".to_string(), TypeTerm::Int),
            ("assign".to_string(), TypeTerm::Any),
            ("qr".to_string(), TypeTerm::Any),
            ("df.residual".to_string(), TypeTerm::Int),
        ])),
        "stats::lsfit" => Some(TypeTerm::NamedList(vec![
            (
                "coefficients".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "residuals".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("intercept".to_string(), TypeTerm::Logical),
            ("qr".to_string(), TypeTerm::Any),
        ])),
        "stats::ls.diag" => Some(TypeTerm::NamedList(vec![
            ("std.dev".to_string(), TypeTerm::Double),
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "std.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "stud.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "cooks".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "dfits".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "correlation".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "std.err".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "cov.scaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "cov.unscaled".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::loadings" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::makepredictcall" => Some(TypeTerm::Any),
        "stats::na.contiguous" => Some(preserved_head_tail_term(first_arg_term(arg_terms))),
        "stats::na.action" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "stats::napredict" | "stats::naresid" => {
            Some(preserved_head_tail_term(second_arg_term(arg_terms)))
        }
        "stats::naprint" => Some(TypeTerm::Char),
        "stats::weights" | "stats::model.weights" | "stats::model.offset" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "stats::offset" => Some(preserved_head_tail_term(first_arg_term(arg_terms))),
        "stats::na.omit" | "stats::na.exclude" | "stats::na.pass" | "stats::na.fail" => {
            Some(preserved_head_tail_term(first_arg_term(arg_terms)))
        }
        "stats::lm.influence" => Some(TypeTerm::NamedList(vec![
            (
                "hat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "coefficients".to_string(),
                TypeTerm::Matrix(Box::new(TypeTerm::Double)),
            ),
            (
                "sigma".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "wt.res".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "stats::glm.control" => Some(TypeTerm::NamedList(vec![
            ("epsilon".to_string(), TypeTerm::Double),
            ("maxit".to_string(), TypeTerm::Int),
            ("trace".to_string(), TypeTerm::Logical),
        ])),
        "stats::is.empty.model" => Some(TypeTerm::Logical),
        "stats::getCall" => Some(TypeTerm::Any),
        "stats::update" | "stats::update.default" | "stats::terms" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "stats::update.formula" | "stats::drop.terms" => Some(TypeTerm::Any),
        "grid::grid.newpage"
        | "grid::grid.draw"
        | "grid::grid.pack"
        | "grid::grid.place"
        | "grid::grid.polyline"
        | "grid::grid.raster"
        | "grid::grid.curve"
        | "grid::grid.bezier"
        | "grid::grid.path"
        | "grid::pushViewport"
        | "grid::popViewport" => Some(TypeTerm::Null),
        "grid::seekViewport" => Some(TypeTerm::Int),
        "grid::nullGrob"
        | "grid::rectGrob"
        | "grid::circleGrob"
        | "grid::segmentsGrob"
        | "grid::pointsGrob"
        | "grid::rasterGrob"
        | "grid::bezierGrob"
        | "grid::pathGrob"
        | "grid::polygonGrob"
        | "grid::polylineGrob"
        | "grid::xsplineGrob"
        | "grid::frameGrob"
        | "grid::packGrob"
        | "grid::placeGrob"
        | "grid::roundrectGrob"
        | "grid::linesGrob"
        | "grid::curveGrob"
        | "grid::textGrob"
        | "grid::grobTree"
        | "grid::gList"
        | "grid::gpar"
        | "grid::viewport"
        | "grid::grid.layout"
        | "grid::grid.frame"
        | "grid::vpStack"
        | "grid::vpList"
        | "grid::dataViewport"
        | "grid::current.viewport"
        | "grid::upViewport"
        | "grid::grid.rect"
        | "grid::grid.text"
        | "grid::grid.circle"
        | "grid::grid.points"
        | "grid::grid.lines"
        | "grid::grid.segments"
        | "grid::grid.polygon" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "grid::unit"
        | "grid::grobWidth"
        | "grid::grobHeight"
        | "grid::drawDetails"
        | "grid::grid.multipanel"
        | "grid::addGrob"
        | "grid::grobDescent"
        | "grid::grid.roundrect"
        | "grid::convertNative"
        | "grid::vpPath"
        | "grid::getGrob"
        | "grid::grid.grep"
        | "grid::applyEdits"
        | "grid::absolute.size"
        | "grid::explode"
        | "grid::gPath"
        | "grid::widthDetails"
        | "grid::current.transform"
        | "grid::descentDetails"
        | "grid::grid.stroke"
        | "grid::bezierPoints"
        | "grid::getNames"
        | "grid::convertUnit"
        | "grid::grid.show.layout"
        | "grid::is.grob"
        | "grid::grid.legend"
        | "grid::emptyCoords"
        | "grid::radialGradient"
        | "grid::groupShear"
        | "grid::stringDescent"
        | "grid::removeGrob"
        | "grid::grid.yaxis"
        | "grid::viewportScale"
        | "grid::grid.revert"
        | "grid::grid.grob"
        | "grid::arrow"
        | "grid::strokeGrob"
        | "grid::useRotate"
        | "grid::fillGrob"
        | "grid::grid.locator"
        | "grid::emptyGTreeCoords"
        | "grid::grid.pretty"
        | "grid::applyEdit"
        | "grid::fillStrokeGrob"
        | "grid::deviceLoc"
        | "grid::arcCurvature"
        | "grid::viewportTransform"
        | "grid::useGrob"
        | "grid::viewport.transform"
        | "grid::glyphGrob"
        | "grid::setChildren"
        | "grid::grobAscent"
        | "grid::unit.pmin"
        | "grid::grid.ls"
        | "grid::viewport.layout"
        | "grid::grid.reorder"
        | "grid::plotViewport"
        | "grid::moveToGrob"
        | "grid::preDrawDetails"
        | "grid::downViewport"
        | "grid::grid.add"
        | "grid::xsplinePoints"
        | "grid::unitType"
        | "grid::groupScale"
        | "grid::grid.xspline"
        | "grid::viewportRotate"
        | "grid::grid.plot.and.legend"
        | "grid::showGrob"
        | "grid::grid.get"
        | "grid::layout.widths"
        | "grid::grid.glyph"
        | "grid::grid.fillStroke"
        | "grid::isClosed"
        | "grid::grid.xaxis"
        | "grid::resolveRasterSize"
        | "grid::yDetails"
        | "grid::convertHeight"
        | "grid::defnRotate"
        | "grid::grid.gedit"
        | "grid::valid.just"
        | "grid::postDrawDetails"
        | "grid::convertX"
        | "grid::grid.record"
        | "grid::convertY"
        | "grid::layoutRegion"
        | "grid::grobPoints"
        | "grid::clipGrob"
        | "grid::convertWidth"
        | "grid::xDetails"
        | "grid::push.viewport"
        | "grid::pathListing"
        | "grid::as.mask"
        | "grid::resolveVJust"
        | "grid::get.gpar"
        | "grid::unit.pmax"
        | "grid::unit.psum"
        | "grid::current.vpPath"
        | "grid::ascentDetails"
        | "grid::grid.abline"
        | "grid::childNames"
        | "grid::grobPathListing"
        | "grid::delayGrob"
        | "grid::grid.move.to"
        | "grid::grid.convertHeight"
        | "grid::pattern"
        | "grid::grid.gget"
        | "grid::xaxisGrob"
        | "grid::editDetails"
        | "grid::grid.define"
        | "grid::viewportTranslate"
        | "grid::grid.DLapply"
        | "grid::grid.grill"
        | "grid::nestedListing"
        | "grid::as.path"
        | "grid::layout.heights"
        | "grid::unit.c"
        | "grid::grid.use"
        | "grid::grid.refresh"
        | "grid::resolveHJust"
        | "grid::emptyGrobCoords"
        | "grid::grobName"
        | "grid::editGrob"
        | "grid::grid.strip"
        | "grid::grid.clip"
        | "grid::arrowsGrob"
        | "grid::unit.rep"
        | "grid::grid.copy"
        | "grid::grid.fill"
        | "grid::stringAscent"
        | "grid::gridGTreeCoords"
        | "grid::legendGrob"
        | "grid::useScale"
        | "grid::grid.group"
        | "grid::recordGrob"
        | "grid::useTranslate"
        | "grid::grid.edit"
        | "grid::forceGrob"
        | "grid::grid.convertX"
        | "grid::isEmptyCoords"
        | "grid::grid.convertY"
        | "grid::grid.collection"
        | "grid::showViewport"
        | "grid::grid.set"
        | "grid::vpTree"
        | "grid::makeContent"
        | "grid::grid.display.list"
        | "grid::gTree"
        | "grid::gridGrobCoords"
        | "grid::groupGrob"
        | "grid::defineGrob"
        | "grid::groupFlip"
        | "grid::deviceDim"
        | "grid::current.rotation"
        | "grid::editViewport"
        | "grid::grid.grabExpr"
        | "grid::grob"
        | "grid::gridCoords"
        | "grid::yaxisGrob"
        | "grid::groupRotate"
        | "grid::grid.line.to"
        | "grid::reorderGrob"
        | "grid::depth"
        | "grid::defnScale"
        | "grid::groupTranslate"
        | "grid::heightDetails"
        | "grid::is.unit"
        | "grid::grobX"
        | "grid::stringHeight"
        | "grid::grid.convert"
        | "grid::makeContext"
        | "grid::grobY"
        | "grid::unit.length"
        | "grid::linearGradient"
        | "grid::grid.null"
        | "grid::grid.arrows"
        | "grid::defnTranslate"
        | "grid::grid.delay"
        | "grid::grid.cap"
        | "grid::validDetails"
        | "grid::grid.gremove"
        | "grid::layout.torture"
        | "grid::grid.show.viewport"
        | "grid::gEdit"
        | "grid::current.parent"
        | "grid::grobCoords"
        | "grid::engine.display.list"
        | "grid::grid.convertWidth"
        | "grid::grid.function"
        | "grid::gEditList"
        | "grid::calcStringMetric"
        | "grid::grid.remove"
        | "grid::grid.grab"
        | "grid::functionGrob"
        | "grid::grid.force"
        | "grid::grid.panel"
        | "grid::setGrob"
        | "grid::stringWidth"
        | "grid::lineToGrob"
        | "grid::draw.details"
        | "grid::current.vpTree" => Some(TypeTerm::Any),
        "graphics::plot"
        | "graphics::plot.default"
        | "graphics::plot.design"
        | "graphics::plot.function"
        | "graphics::plot.new"
        | "graphics::plot.window"
        | "graphics::plot.xy"
        | "graphics::lines"
        | "graphics::lines.default"
        | "graphics::points"
        | "graphics::points.default"
        | "graphics::abline"
        | "graphics::title"
        | "graphics::box"
        | "graphics::text"
        | "graphics::text.default"
        | "graphics::segments"
        | "graphics::arrows"
        | "graphics::mtext"
        | "graphics::polygon"
        | "graphics::polypath"
        | "graphics::matplot"
        | "graphics::matlines"
        | "graphics::matpoints"
        | "graphics::pairs"
        | "graphics::pairs.default"
        | "graphics::stripchart"
        | "graphics::dotchart"
        | "graphics::layout.show"
        | "graphics::pie"
        | "graphics::symbols"
        | "graphics::smoothScatter"
        | "graphics::stem"
        | "graphics::contour"
        | "graphics::contour.default"
        | "graphics::image"
        | "graphics::image.default"
        | "graphics::assocplot"
        | "graphics::mosaicplot"
        | "graphics::fourfoldplot"
        | "graphics::clip"
        | "graphics::xspline"
        | "graphics::.filled.contour"
        | "graphics::filled.contour"
        | "graphics::cdplot"
        | "graphics::coplot"
        | "graphics::curve"
        | "graphics::close.screen"
        | "graphics::co.intervals"
        | "graphics::erase.screen"
        | "graphics::frame"
        | "graphics::grid"
        | "graphics::panel.smooth"
        | "graphics::rasterImage"
        | "graphics::rect"
        | "graphics::spineplot"
        | "graphics::stars"
        | "graphics::sunflowerplot"
        | "grDevices::png"
        | "grDevices::pdf" => Some(TypeTerm::Null),
        "graphics::persp" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "graphics::hist"
        | "graphics::hist.default"
        | "graphics::boxplot"
        | "graphics::boxplot.default"
        | "graphics::boxplot.matrix"
        | "graphics::barplot"
        | "graphics::barplot.default"
        | "graphics::bxp"
        | "graphics::par"
        | "graphics::screen"
        | "graphics::split.screen" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "graphics::layout" => Some(TypeTerm::Int),
        "graphics::axTicks"
        | "graphics::Axis"
        | "graphics::axis.Date"
        | "graphics::axis.POSIXct"
        | "graphics::strwidth"
        | "graphics::strheight"
        | "graphics::grconvertX"
        | "graphics::grconvertY" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "graphics::lcm" | "graphics::xinch" | "graphics::yinch" | "graphics::xyinch" => {
            vectorized_scalar_or_vector_double_term(arg_terms)
        }
        "grDevices::jpeg" | "grDevices::bmp" | "grDevices::tiff" => Some(TypeTerm::Null),
        "grDevices::dev.size" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "grDevices::dev.off"
        | "grDevices::dev.cur"
        | "grDevices::dev.next"
        | "grDevices::dev.prev" => Some(TypeTerm::Int),
        "graphics::axis" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "graphics::identify" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "graphics::locator" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "graphics::rug" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "grDevices::rgb"
        | "grDevices::hsv"
        | "grDevices::gray"
        | "grDevices::gray.colors"
        | "grDevices::palette.colors"
        | "grDevices::palette.pals"
        | "grDevices::hcl.colors"
        | "grDevices::colors"
        | "grDevices::heat.colors"
        | "grDevices::terrain.colors"
        | "grDevices::topo.colors"
        | "grDevices::cm.colors"
        | "grDevices::rainbow"
        | "grDevices::adjustcolor"
        | "grDevices::palette"
        | "grDevices::densCols" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "grDevices::n2mfrow" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "grDevices::col2rgb" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Int))),
        "grDevices::rgb2hsv" | "grDevices::convertColor" => {
            Some(TypeTerm::Matrix(Box::new(TypeTerm::Double)))
        }
        "grDevices::as.raster" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Char))),
        "grDevices::axisTicks" | "grDevices::extendrange" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
        }
        "grDevices::cm" => vectorized_scalar_or_vector_double_term(arg_terms),
        "grDevices::boxplot.stats" => Some(TypeTerm::NamedList(vec![
            (
                "stats".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("n".to_string(), TypeTerm::Int),
            (
                "conf".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "out".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "grDevices::chull" | "grDevices::dev.list" => {
            Some(TypeTerm::Vector(Box::new(TypeTerm::Int)))
        }
        "grDevices::dev.set"
        | "grDevices::nclass.FD"
        | "grDevices::nclass.scott"
        | "grDevices::nclass.Sturges" => Some(TypeTerm::Int),
        "grDevices::dev.interactive"
        | "grDevices::deviceIsInteractive"
        | "grDevices::is.raster" => Some(TypeTerm::Logical),
        "grDevices::blues9"
        | "grDevices::grey"
        | "grDevices::grey.colors"
        | "grDevices::grSoftVersion"
        | "grDevices::hcl"
        | "grDevices::hcl.pals"
        | "grDevices::colorspaces"
        | "grDevices::colours" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "grDevices::contourLines"
        | "grDevices::dev.capabilities"
        | "grDevices::dev.capture"
        | "grDevices::check.options"
        | "grDevices::colorConverter"
        | "grDevices::colorRamp"
        | "grDevices::colorRampPalette"
        | "grDevices::getGraphicsEvent"
        | "grDevices::getGraphicsEventEnv"
        | "grDevices::recordPlot"
        | "grDevices::as.graphicsAnnot"
        | "grDevices::make.rgb"
        | "grDevices::pdf.options"
        | "grDevices::pdfFonts"
        | "grDevices::ps.options"
        | "grDevices::postscriptFonts"
        | "grDevices::quartz.options"
        | "grDevices::quartzFont"
        | "grDevices::quartzFonts"
        | "grDevices::X11.options"
        | "grDevices::X11Font"
        | "grDevices::X11Fonts"
        | "grDevices::cairoSymbolFont"
        | "grDevices::CIDFont"
        | "grDevices::Type1Font"
        | "grDevices::Hershey"
        | "grDevices::glyphAnchor"
        | "grDevices::glyphFont"
        | "grDevices::glyphFontList"
        | "grDevices::glyphHeight"
        | "grDevices::glyphHeightBottom"
        | "grDevices::glyphInfo"
        | "grDevices::glyphJust"
        | "grDevices::glyphWidth"
        | "grDevices::glyphWidthLeft"
        | "grDevices::.axisPars"
        | "grDevices::.clipPath"
        | "grDevices::.defineGroup"
        | "grDevices::.devUp"
        | "grDevices::.linearGradientPattern"
        | "grDevices::.mask"
        | "grDevices::.opIndex"
        | "grDevices::.radialGradientPattern"
        | "grDevices::.ruleIndex"
        | "grDevices::.setClipPath"
        | "grDevices::.setMask"
        | "grDevices::.setPattern"
        | "grDevices::.tilingPattern"
        | "grDevices::.useGroup" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "grDevices::trans3d" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "grDevices::xy.coords" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("xlab".to_string(), TypeTerm::Any),
            ("ylab".to_string(), TypeTerm::Any),
        ])),
        "grDevices::xyTable" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "number".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
        ])),
        "grDevices::xyz.coords" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "z".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            ("xlab".to_string(), TypeTerm::Any),
            ("ylab".to_string(), TypeTerm::Any),
            ("zlab".to_string(), TypeTerm::Any),
        ])),
        "grDevices::bitmap"
        | "grDevices::cairo_pdf"
        | "grDevices::cairo_ps"
        | "grDevices::dev.control"
        | "grDevices::dev.copy"
        | "grDevices::dev.copy2eps"
        | "grDevices::dev.copy2pdf"
        | "grDevices::dev.flush"
        | "grDevices::dev.hold"
        | "grDevices::dev.new"
        | "grDevices::dev.print"
        | "grDevices::devAskNewPage"
        | "grDevices::dev2bitmap"
        | "grDevices::embedFonts"
        | "grDevices::embedGlyphs"
        | "grDevices::graphics.off"
        | "grDevices::pictex"
        | "grDevices::postscript"
        | "grDevices::quartz"
        | "grDevices::quartz.save"
        | "grDevices::recordGraphics"
        | "grDevices::replayPlot"
        | "grDevices::savePlot"
        | "grDevices::setEPS"
        | "grDevices::setGraphicsEventEnv"
        | "grDevices::setGraphicsEventHandlers"
        | "grDevices::setPS"
        | "grDevices::svg"
        | "grDevices::x11"
        | "grDevices::X11"
        | "grDevices::xfig" => Some(TypeTerm::Null),
        "graphics::legend" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
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
        "stats::lm" | "stats::glm" => Some(TypeTerm::List(Box::new(TypeTerm::Any))),
        "stats::as.formula" | "stats::formula" | "stats::reformulate" => Some(TypeTerm::Any),
        "stats::contrasts"
        | "stats::contr.treatment"
        | "stats::contr.sum"
        | "stats::contr.helmert"
        | "stats::contr.SAS"
        | "stats::contr.poly" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        "stats::make.link" => Some(TypeTerm::NamedList(vec![
            ("linkfun".to_string(), TypeTerm::Any),
            ("linkinv".to_string(), TypeTerm::Any),
            ("mu.eta".to_string(), TypeTerm::Any),
            ("valideta".to_string(), TypeTerm::Any),
            ("name".to_string(), TypeTerm::Char),
        ])),
        "stats::family"
        | "stats::binomial"
        | "stats::gaussian"
        | "stats::poisson"
        | "stats::quasibinomial"
        | "stats::quasipoisson"
        | "stats::inverse.gaussian" => Some(TypeTerm::NamedList(vec![
            ("family".to_string(), TypeTerm::Char),
            ("link".to_string(), TypeTerm::Char),
            ("linkfun".to_string(), TypeTerm::Any),
            ("linkinv".to_string(), TypeTerm::Any),
            ("variance".to_string(), TypeTerm::Any),
            ("dev.resids".to_string(), TypeTerm::Any),
            ("aic".to_string(), TypeTerm::Any),
            ("mu.eta".to_string(), TypeTerm::Any),
            ("initialize".to_string(), TypeTerm::Any),
            ("validmu".to_string(), TypeTerm::Any),
            ("valideta".to_string(), TypeTerm::Any),
            ("dispersion".to_string(), TypeTerm::Double),
        ])),
        "stats::quasi" => Some(TypeTerm::NamedList(vec![
            ("family".to_string(), TypeTerm::Char),
            ("link".to_string(), TypeTerm::Char),
            ("linkfun".to_string(), TypeTerm::Any),
            ("linkinv".to_string(), TypeTerm::Any),
            ("variance".to_string(), TypeTerm::Any),
            ("dev.resids".to_string(), TypeTerm::Any),
            ("aic".to_string(), TypeTerm::Any),
            ("mu.eta".to_string(), TypeTerm::Any),
            ("initialize".to_string(), TypeTerm::Any),
            ("validmu".to_string(), TypeTerm::Any),
            ("valideta".to_string(), TypeTerm::Any),
            ("varfun".to_string(), TypeTerm::Char),
            ("dispersion".to_string(), TypeTerm::Double),
        ])),
        _ => None,
    }
}

pub fn infer_package_binding_term(var: &str) -> Option<TypeTerm> {
    match var {
        "datasets::iris" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Sepal.Length".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Sepal.Width".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Petal.Length".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Petal.Width".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Species".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "datasets::mtcars" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "mpg".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "cyl".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "disp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "hp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "drat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "wt".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "qsec".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "vs".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "am".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "gear".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "carb".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::airquality" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Ozone".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "Solar.R".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "Wind".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Temp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "Month".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("Day".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
        ])),
        "datasets::ToothGrowth" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "len".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "supp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "dose".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::CO2" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Plant".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Type".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Treatment".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "conc".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "uptake".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::USArrests" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Murder".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Assault".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "UrbanPop".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "Rape".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::cars" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "speed".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "dist".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::pressure" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "temperature".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "pressure".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::faithful" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "eruptions".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "waiting".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::women" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "height".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "weight".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::BOD" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Time".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "demand".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::attitude" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "rating".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "complaints".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "privileges".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "learning".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "raises".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "critical".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "advance".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::PlantGrowth" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "weight".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "group".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "datasets::InsectSprays" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "count".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "spray".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "datasets::sleep" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "extra".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "group".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("ID".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Char))),
        ])),
        "datasets::Orange" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Tree".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "age".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "circumference".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::rock" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "area".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "peri".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "shape".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "perm".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::trees" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Girth".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Height".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Volume".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::esoph" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "agegp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "alcgp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "tobgp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "ncases".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "ncontrols".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::stackloss" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Air.Flow".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Water.Temp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Acid.Conc.".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "stack.loss".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::warpbreaks" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "breaks".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "wool".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "tension".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "datasets::quakes" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "lat".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "long".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "depth".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "mag".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "stations".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::LifeCycleSavings" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "sr".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "pop15".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "pop75".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "dpi".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "ddpi".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::ChickWeight" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "weight".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Time".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Chick".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Diet".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "datasets::DNase" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Run".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "conc".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "density".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::Formaldehyde" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "carb".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "optden".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::Indometh" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Subject".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "time".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "conc".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::Loblolly" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "height".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "age".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Seed".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "datasets::Puromycin" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "conc".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "rate".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "state".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "datasets::USJudgeRatings" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "CONT".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "INTG".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "DMNR".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "DILG".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "CFMG".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "DECI".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "PREP".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "FAMI".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "ORAL".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "WRIT".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "PHYS".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "RTEN".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::anscombe" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "x1".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "x2".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "x3".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "x4".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y1".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y2".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y3".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y4".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::attenu" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "event".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "mag".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "station".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "dist".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "accel".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::chickwts" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "weight".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "feed".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "datasets::infert" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "education".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "age".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "parity".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "induced".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "case".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "spontaneous".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "stratum".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "pooled.stratum".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::longley" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "GNP.deflator".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "GNP".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Unemployed".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Armed.Forces".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Population".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Year".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "Employed".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::morley" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Expt".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            ("Run".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
            (
                "Speed".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
        ])),
        "datasets::npk" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "block".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            ("N".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Char))),
            ("P".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Char))),
            ("K".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Char))),
            (
                "yield".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::swiss" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Fertility".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Agriculture".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Examination".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "Education".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "Catholic".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Infant.Mortality".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::volcano" => Some(TypeTerm::MatrixDim(
            Box::new(TypeTerm::Double),
            Some(87),
            Some(61),
        )),
        "datasets::state.x77" => Some(TypeTerm::MatrixDim(
            Box::new(TypeTerm::Double),
            Some(50),
            Some(8),
        )),
        "datasets::USPersonalExpenditure" => Some(TypeTerm::MatrixDim(
            Box::new(TypeTerm::Double),
            Some(5),
            Some(5),
        )),
        "datasets::WorldPhones" => Some(TypeTerm::MatrixDim(
            Box::new(TypeTerm::Double),
            Some(7),
            Some(7),
        )),
        "datasets::EuStockMarkets" => Some(TypeTerm::MatrixDim(
            Box::new(TypeTerm::Double),
            Some(1860),
            Some(4),
        )),
        "datasets::VADeaths" => Some(TypeTerm::MatrixDim(
            Box::new(TypeTerm::Double),
            Some(5),
            Some(4),
        )),
        "datasets::AirPassengers" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::JohnsonJohnson" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::Nile" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::lynx" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::nottem" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::sunspot.year" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::precip" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::islands" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::state.area" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::state.abb" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "datasets::state.name" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "datasets::state.region" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "datasets::state.division" => Some(TypeTerm::Vector(Box::new(TypeTerm::Char))),
        "datasets::airmiles" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::austres" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::co2" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::discoveries" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::fdeaths" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::ldeaths" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::mdeaths" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::nhtemp" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::sunspots" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::treering" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::uspop" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::rivers" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::UKDriverDeaths" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::UKgas" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::USAccDeaths" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::WWWusage" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::eurodist" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::UScitiesD" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "datasets::euro" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::stack.loss" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::sunspot.m2014" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::sunspot.month" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::LakeHuron" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::lh" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::presidents" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::Seatbelts" => Some(TypeTerm::MatrixDim(
            Box::new(TypeTerm::Double),
            Some(192),
            Some(8),
        )),
        "datasets::OrchardSprays" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "decrease".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "rowpos".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "colpos".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "treatment".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "datasets::Theoph" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "Subject".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Wt".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Dose".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Time".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "conc".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::penguins" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "species".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "island".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "bill_len".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "bill_dep".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "flipper_len".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "body_mass".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
            (
                "sex".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "year".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Int)),
            ),
        ])),
        "datasets::penguins_raw" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "studyName".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Sample Number".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Species".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Region".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Island".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Stage".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Individual ID".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Clutch Completion".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Date Egg".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Culmen Length (mm)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Culmen Depth (mm)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Flipper Length (mm)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Body Mass (g)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Sex".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
            (
                "Delta 15 N (o/oo)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Delta 13 C (o/oo)".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "Comments".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Char)),
            ),
        ])),
        "datasets::gait" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(20), Some(39), Some(2)],
        )),
        "datasets::crimtab" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Int),
            vec![Some(42), Some(22)],
        )),
        "datasets::occupationalStatus" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Int),
            vec![Some(8), Some(8)],
        )),
        "datasets::ability.cov" => Some(TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(6), Some(6)),
            ),
            (
                "center".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(6)),
            ),
            ("n.obs".to_string(), TypeTerm::Double),
        ])),
        "datasets::Harman23.cor" => Some(TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(8), Some(8)),
            ),
            (
                "center".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(8)),
            ),
            ("n.obs".to_string(), TypeTerm::Double),
        ])),
        "datasets::Harman74.cor" => Some(TypeTerm::NamedList(vec![
            (
                "cov".to_string(),
                TypeTerm::MatrixDim(Box::new(TypeTerm::Double), Some(24), Some(24)),
            ),
            (
                "center".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(24)),
            ),
            ("n.obs".to_string(), TypeTerm::Double),
        ])),
        "datasets::state.center" => Some(TypeTerm::NamedList(vec![
            (
                "x".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(50)),
            ),
            (
                "y".to_string(),
                TypeTerm::VectorLen(Box::new(TypeTerm::Double), Some(50)),
            ),
        ])),
        "datasets::BJsales" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::BJsales.lead" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::beaver1" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "day".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "time".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "temp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "activ".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::beaver2" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "day".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "time".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "temp".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "activ".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::euro.cross" => Some(TypeTerm::MatrixDim(
            Box::new(TypeTerm::Double),
            Some(11),
            Some(11),
        )),
        "datasets::randu" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "x".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "z".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::freeny" => Some(TypeTerm::DataFrameNamed(vec![
            (
                "y".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "lag.quarterly.revenue".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "price.index".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "income.level".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
            (
                "market.potential".to_string(),
                TypeTerm::Vector(Box::new(TypeTerm::Double)),
            ),
        ])),
        "datasets::stack.x" => Some(TypeTerm::MatrixDim(
            Box::new(TypeTerm::Double),
            Some(21),
            Some(3),
        )),
        "datasets::freeny.x" => Some(TypeTerm::MatrixDim(
            Box::new(TypeTerm::Double),
            Some(39),
            Some(4),
        )),
        "datasets::freeny.y" => Some(TypeTerm::Vector(Box::new(TypeTerm::Double))),
        "datasets::iris3" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(50), Some(4), Some(3)],
        )),
        "datasets::Titanic" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(4), Some(2), Some(2), Some(2)],
        )),
        "datasets::UCBAdmissions" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(2), Some(2), Some(6)],
        )),
        "datasets::HairEyeColor" => Some(TypeTerm::ArrayDim(
            Box::new(TypeTerm::Double),
            vec![Some(4), Some(4), Some(2)],
        )),
        callee if callee.starts_with("base::") => Some(TypeTerm::Any),
        _ => None,
    }
}
