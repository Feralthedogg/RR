use super::outputs::shallow_elem_term;
use crate::typeck::lattice::{LenSym, NaTy, PrimTy, ShapeTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn promoted_numeric_prim(args: &[TypeState]) -> PrimTy {
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

pub(crate) fn promoted_numeric_term(args: &[TypeTerm]) -> TypeTerm {
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

pub(crate) fn shared_vector_len_sym(args: &[TypeState]) -> Option<LenSym> {
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

pub(crate) fn any_vector_shape(args: &[TypeState]) -> bool {
    args.iter()
        .any(|t| matches!(t.shape, ShapeTy::Vector | ShapeTy::Matrix))
}

pub(crate) fn all_known_scalar_shape(args: &[TypeState]) -> bool {
    !args.is_empty() && args.iter().all(|t| t.shape == ShapeTy::Scalar)
}

pub(crate) fn all_known_numeric_prim(args: &[TypeState]) -> bool {
    !args.is_empty()
        && args
            .iter()
            .all(|t| matches!(t.prim, PrimTy::Int | PrimTy::Double))
}

pub(crate) fn any_vector_term(args: &[TypeTerm]) -> bool {
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

pub(crate) fn all_known_scalar_term(args: &[TypeTerm]) -> bool {
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

pub(crate) fn first_numeric_prim(args: &[TypeState]) -> PrimTy {
    args.iter()
        .find_map(|ty| matches!(ty.prim, PrimTy::Int | PrimTy::Double).then_some(ty.prim))
        .unwrap_or(PrimTy::Any)
}

pub(crate) fn first_numeric_term(args: &[TypeTerm]) -> TypeTerm {
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

pub(crate) fn first_arg_type_state(args: &[TypeState]) -> TypeState {
    args.first().copied().unwrap_or(TypeState::unknown())
}

pub(crate) fn second_arg_type_state(args: &[TypeState]) -> TypeState {
    args.get(1).copied().unwrap_or(TypeState::unknown())
}

pub(crate) fn first_arg_term(args: &[TypeTerm]) -> TypeTerm {
    args.first().cloned().unwrap_or(TypeTerm::Any)
}

pub(crate) fn second_arg_term(args: &[TypeTerm]) -> TypeTerm {
    args.get(1).cloned().unwrap_or(TypeTerm::Any)
}

pub(crate) fn vectorized_first_arg_type(first: TypeState) -> TypeState {
    let prim = match first.shape {
        ShapeTy::Scalar | ShapeTy::Vector | ShapeTy::Matrix => first.prim,
        ShapeTy::Unknown => PrimTy::Any,
    };
    TypeState::vector(prim, false).with_len(first.len_sym)
}

pub(crate) fn vectorized_first_arg_term(first: TypeTerm) -> TypeTerm {
    match first {
        TypeTerm::Vector(inner)
        | TypeTerm::VectorLen(inner, _)
        | TypeTerm::Matrix(inner)
        | TypeTerm::MatrixDim(inner, _, _)
        | TypeTerm::ArrayDim(inner, _) => TypeTerm::Vector(inner),
        term => TypeTerm::Vector(Box::new(term)),
    }
}

pub(crate) fn preserved_first_arg_type_without_len(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Scalar => TypeState::scalar(first.prim, first.na == NaTy::Never),
        ShapeTy::Vector => TypeState::vector(first.prim, first.na == NaTy::Never),
        ShapeTy::Matrix => TypeState::matrix(first.prim, first.na == NaTy::Never),
        ShapeTy::Unknown => TypeState::unknown(),
    }
}

pub(crate) fn matrix_like_first_arg_type(first: TypeState) -> TypeState {
    TypeState::matrix(first.prim, first.na == NaTy::Never)
}

pub(crate) fn preserved_second_arg_type_without_len(second: TypeState) -> TypeState {
    match second.shape {
        ShapeTy::Scalar => TypeState::scalar(second.prim, second.na == NaTy::Never),
        ShapeTy::Vector => TypeState::vector(second.prim, second.na == NaTy::Never),
        ShapeTy::Matrix => TypeState::matrix(second.prim, second.na == NaTy::Never),
        ShapeTy::Unknown => TypeState::unknown(),
    }
}

pub(crate) fn preserved_head_tail_term(first: TypeTerm) -> TypeTerm {
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

pub(crate) fn matrix_like_first_arg_term(first: TypeTerm) -> TypeTerm {
    TypeTerm::Matrix(Box::new(shallow_elem_term(&first)))
}

pub(crate) fn char_like_first_arg_type(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Scalar => TypeState::scalar(PrimTy::Char, false),
        ShapeTy::Vector | ShapeTy::Matrix => {
            TypeState::vector(PrimTy::Char, false).with_len(first.len_sym)
        }
        ShapeTy::Unknown => TypeState::vector(PrimTy::Char, false),
    }
}

pub(crate) fn int_like_first_arg_type(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Scalar => TypeState::scalar(PrimTy::Int, false),
        ShapeTy::Vector | ShapeTy::Matrix => {
            TypeState::vector(PrimTy::Int, false).with_len(first.len_sym)
        }
        ShapeTy::Unknown => TypeState::vector(PrimTy::Int, false),
    }
}

pub(crate) fn logical_like_first_arg_type(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Scalar => TypeState::scalar(PrimTy::Logical, false),
        ShapeTy::Vector | ShapeTy::Matrix => {
            TypeState::vector(PrimTy::Logical, false).with_len(first.len_sym)
        }
        ShapeTy::Unknown => TypeState::vector(PrimTy::Logical, false),
    }
}

pub(crate) fn double_like_first_arg_type(first: TypeState) -> TypeState {
    match first.shape {
        ShapeTy::Scalar => TypeState::scalar(PrimTy::Double, false),
        ShapeTy::Vector | ShapeTy::Matrix => {
            TypeState::vector(PrimTy::Double, false).with_len(first.len_sym)
        }
        ShapeTy::Unknown => TypeState::vector(PrimTy::Double, false),
    }
}

pub(crate) fn char_like_first_arg_term(first: TypeTerm) -> TypeTerm {
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

pub(crate) fn int_like_first_arg_term(first: TypeTerm) -> TypeTerm {
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

pub(crate) fn logical_like_first_arg_term(first: TypeTerm) -> TypeTerm {
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

pub(crate) fn double_like_first_arg_term(first: TypeTerm) -> TypeTerm {
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
