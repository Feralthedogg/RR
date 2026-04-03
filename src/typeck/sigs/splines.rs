use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_splines_package_call(
    callee: &str,
    _arg_tys: &[TypeState],
) -> Option<TypeState> {
    match callee {
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
        _ => None,
    }
}

pub(crate) fn infer_splines_package_call_term(
    callee: &str,
    _arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    match callee {
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
        _ => None,
    }
}
