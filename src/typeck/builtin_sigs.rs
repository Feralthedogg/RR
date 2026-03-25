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
    args.iter().fold(TypeTerm::Any, |acc, ty| match (&acc, ty) {
        (TypeTerm::Any, other) => other.clone(),
        (other, TypeTerm::Any) => other.clone(),
        (TypeTerm::Int, TypeTerm::Int) => TypeTerm::Int,
        (TypeTerm::Int, TypeTerm::Double)
        | (TypeTerm::Double, TypeTerm::Int)
        | (TypeTerm::Double, TypeTerm::Double) => TypeTerm::Double,
        _ => TypeTerm::Any,
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
            TypeTerm::Vector(_) | TypeTerm::Matrix(_) | TypeTerm::MatrixDim(_, _, _)
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
            | TypeTerm::Matrix(inner)
            | TypeTerm::MatrixDim(inner, _, _)
                if matches!(inner.as_ref(), TypeTerm::Int | TypeTerm::Double) =>
            {
                Some((**inner).clone())
            }
            _ => None,
        })
        .unwrap_or(TypeTerm::Any)
}

fn matrix_term_parts(term: &TypeTerm) -> Option<(TypeTerm, Option<i64>, Option<i64>)> {
    match term {
        TypeTerm::Matrix(inner) => Some((inner.as_ref().clone(), None, None)),
        TypeTerm::MatrixDim(inner, rows, cols) => Some((inner.as_ref().clone(), *rows, *cols)),
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

pub fn infer_builtin(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
        "length" | "seq_len" | "nrow" | "ncol" => Some(TypeState::scalar(PrimTy::Int, true)),
        "seq_along" => {
            Some(TypeState::vector(PrimTy::Int, true).with_len(shared_vector_len_sym(arg_tys)))
        }
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
        "abs" | "min" | "max" | "pmax" | "pmin" => {
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
        "mean" | "sqrt" | "log" | "log10" | "log2" | "exp" | "atan2" | "sin" | "cos" | "tan"
        | "floor" | "ceiling" | "trunc" | "round" => {
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

pub fn infer_builtin_term(callee: &str, arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    match callee {
        "length" | "nrow" | "ncol" => Some(TypeTerm::Int),
        "seq_len" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "seq_along" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "dim" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "dimnames" => Some(TypeTerm::List(Box::new(TypeTerm::Vector(Box::new(
            TypeTerm::Char,
        ))))),
        "rr_i0" | "rr_i1" | "rr_index1_read_idx" => Some(TypeTerm::Int),
        "rr_index_vec_floor" => {
            if arg_terms.iter().any(|t| {
                matches!(
                    t,
                    TypeTerm::Vector(_) | TypeTerm::Matrix(_) | TypeTerm::MatrixDim(_, _, _)
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
                    | TypeTerm::Matrix(inner)
                    | TypeTerm::MatrixDim(inner, _, _) => inner.as_ref().clone(),
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
        "abs" | "min" | "max" | "pmax" | "pmin" => {
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
        "mean" | "sqrt" | "log" | "log10" | "log2" | "exp" | "atan2" | "sin" | "cos" | "tan"
        | "floor" | "ceiling" | "trunc" | "round" => {
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
                    TypeTerm::Vector(inner) => {
                        elem = elem.join(inner);
                        rows = rows.map(|r| r + 1);
                        cols = None;
                    }
                    TypeTerm::Matrix(_) | TypeTerm::MatrixDim(_, _, _) => {
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
                    TypeTerm::Vector(inner) => {
                        elem = elem.join(inner);
                        cols = cols.map(|c| c + 1);
                        rows = None;
                    }
                    TypeTerm::Matrix(_) | TypeTerm::MatrixDim(_, _, _) => {
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
