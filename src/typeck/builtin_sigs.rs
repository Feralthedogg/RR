use super::lattice::{PrimTy, ShapeTy, TypeState};
use super::term::TypeTerm;

pub fn infer_builtin(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
        "length" | "seq_len" => Some(TypeState::scalar(PrimTy::Int, true)),
        "seq_along" => Some(TypeState::vector(PrimTy::Int, true)),
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
            Some(out)
        }
        "sum" | "mean" | "min" | "max" | "abs" | "sqrt" | "log" | "log10" | "log2" | "exp"
        | "pmax" | "pmin" | "atan2" | "sin" | "cos" | "tan" | "floor" | "ceiling" | "trunc"
        | "round" => {
            if arg_tys.iter().any(|t| t.shape == ShapeTy::Vector) {
                Some(TypeState::vector(PrimTy::Double, false))
            } else {
                Some(TypeState::scalar(PrimTy::Double, false))
            }
        }
        "is.na" | "is.finite" => {
            if arg_tys.iter().any(|t| t.shape == ShapeTy::Vector) {
                Some(TypeState::vector(PrimTy::Logical, false))
            } else {
                Some(TypeState::scalar(PrimTy::Logical, false))
            }
        }
        "matrix" => Some(TypeState::matrix(PrimTy::Double, false)),
        "list" => Some(TypeState::vector(PrimTy::Any, false)),
        _ => None,
    }
}

pub fn infer_builtin_term(callee: &str, arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    match callee {
        "length" | "seq_len" => Some(TypeTerm::Int),
        "seq_along" => Some(TypeTerm::Vector(Box::new(TypeTerm::Int))),
        "c" => {
            let mut elem = TypeTerm::Any;
            for t in arg_terms {
                let promoted = match t {
                    TypeTerm::Vector(inner) | TypeTerm::Matrix(inner) => inner.as_ref().clone(),
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
        "sum" | "mean" | "min" | "max" | "abs" | "sqrt" | "log" | "log10" | "log2" | "exp"
        | "pmax" | "pmin" | "atan2" | "sin" | "cos" | "tan" | "floor" | "ceiling" | "trunc"
        | "round" => {
            if arg_terms
                .iter()
                .any(|t| matches!(t, TypeTerm::Vector(_) | TypeTerm::Matrix(_)))
            {
                Some(TypeTerm::Vector(Box::new(TypeTerm::Double)))
            } else {
                Some(TypeTerm::Double)
            }
        }
        "is.na" | "is.finite" => {
            if arg_terms
                .iter()
                .any(|t| matches!(t, TypeTerm::Vector(_) | TypeTerm::Matrix(_)))
            {
                Some(TypeTerm::Vector(Box::new(TypeTerm::Logical)))
            } else {
                Some(TypeTerm::Logical)
            }
        }
        "matrix" => Some(TypeTerm::Matrix(Box::new(TypeTerm::Double))),
        _ => None,
    }
}
