use super::*;

pub(crate) fn joined_general_prim(args: &[TypeState]) -> PrimTy {
    args.iter()
        .fold(PrimTy::Any, |acc, ty| match (acc, ty.prim) {
            (PrimTy::Any, other) => other,
            (other, PrimTy::Any) => other,
            (a, b) if a == b => a,
            (PrimTy::Int, PrimTy::Double) | (PrimTy::Double, PrimTy::Int) => PrimTy::Double,
            _ => PrimTy::Any,
        })
}

pub(crate) fn shallow_elem_term(term: &TypeTerm) -> TypeTerm {
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

pub(crate) fn joined_general_term(args: &[TypeTerm]) -> TypeTerm {
    args.iter().fold(TypeTerm::Any, |acc, term| {
        acc.join(&shallow_elem_term(term))
    })
}

pub(crate) fn rank_output_type(first: TypeState) -> TypeState {
    if matches!(first.shape, ShapeTy::Scalar) {
        TypeState::scalar(PrimTy::Double, false)
    } else {
        TypeState::vector(PrimTy::Double, false).with_len(first.len_sym)
    }
}

pub(crate) fn rank_output_term(first: TypeTerm) -> TypeTerm {
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

pub(crate) fn sample_output_type(first: TypeState) -> TypeState {
    let prim = match (first.shape, first.prim) {
        (ShapeTy::Scalar, PrimTy::Int | PrimTy::Double) => PrimTy::Int,
        (_, prim) => prim,
    };
    TypeState::vector(prim, false)
}

pub(crate) fn sample_output_term(first: TypeTerm) -> TypeTerm {
    let elem = match first {
        TypeTerm::Int | TypeTerm::Double => TypeTerm::Int,
        other => shallow_elem_term(&other),
    };
    TypeTerm::Vector(Box::new(elem))
}

pub(crate) fn ts_like_output_type(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Matrix => TypeState::matrix(first.prim, false),
        _ => TypeState::vector(first.prim, false).with_len(first.len_sym),
    }
}

pub(crate) fn ts_like_output_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::Matrix(inner)
        | TypeTerm::MatrixDim(inner, _, _)
        | TypeTerm::ArrayDim(inner, _) => TypeTerm::Matrix(inner),
        other => TypeTerm::Vector(Box::new(shallow_elem_term(&other))),
    }
}

pub(crate) fn ifelse_output_type(arg_tys: &[TypeState]) -> Option<TypeState> {
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

pub(crate) fn ifelse_output_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
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

pub(crate) fn vectorized_scalar_or_vector_double_type(arg_tys: &[TypeState]) -> Option<TypeState> {
    if !any_vector_shape(arg_tys) && !all_known_scalar_shape(arg_tys) {
        return None;
    }
    if any_vector_shape(arg_tys) {
        Some(TypeState::vector(PrimTy::Double, false).with_len(shared_vector_len_sym(arg_tys)))
    } else {
        Some(TypeState::scalar(PrimTy::Double, false))
    }
}

pub(crate) fn vectorized_scalar_or_vector_double_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    if !any_vector_term(arg_terms) && !all_known_scalar_term(arg_terms) {
        return None;
    }
    if any_vector_term(arg_terms) {
        Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
    } else {
        Some(TypeTerm::Double)
    }
}

pub(crate) fn scalar_or_matrix_double_type(arg_tys: &[TypeState]) -> Option<TypeState> {
    if arg_tys.iter().any(|t| matches!(t.shape, ShapeTy::Matrix)) {
        Some(TypeState::matrix(PrimTy::Double, false))
    } else {
        Some(TypeState::scalar(PrimTy::Double, false))
    }
}

pub(crate) fn scalar_or_matrix_double_term(arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
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

pub(crate) fn matrix_term_parts(term: &TypeTerm) -> Option<(TypeTerm, Option<i64>, Option<i64>)> {
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

pub(crate) fn matrix_term_with_dims(
    elem: TypeTerm,
    rows: Option<i64>,
    cols: Option<i64>,
) -> TypeTerm {
    if rows.is_none() && cols.is_none() {
        TypeTerm::Matrix(Box::new(elem))
    } else {
        TypeTerm::MatrixDim(Box::new(elem), rows, cols)
    }
}

pub(crate) fn dataframe_col_count(term: &TypeTerm) -> Option<i64> {
    match term {
        TypeTerm::DataFrame(cols) => Some(cols.len() as i64),
        TypeTerm::DataFrameNamed(cols) => Some(cols.len() as i64),
        _ => None,
    }
}
