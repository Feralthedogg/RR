use crate::typeck::lattice::{PrimTy, TypeState};
use crate::typeck::term::TypeTerm;

pub(crate) fn infer_tcltk_package_call(callee: &str, _arg_tys: &[TypeState]) -> Option<TypeState> {
    match callee {
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
        _ => None,
    }
}

pub(crate) fn infer_tcltk_package_call_term(
    callee: &str,
    _arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    match callee {
        "tcltk::tclObj" | "tcltk::as.tclObj" | "tcltk::tclVar" => Some(TypeTerm::Any),
        "tcltk::tclvalue" | "tcltk::addTclPath" | "tcltk::tclRequire" => Some(TypeTerm::Any),
        "tcltk::tclVersion" | "tcltk::tclfile.dir" | "tcltk::tclfile.tail" => Some(TypeTerm::Char),
        "tcltk::tkProgressBar" => Some(TypeTerm::Any),
        "tcltk::getTkProgressBar" | "tcltk::setTkProgressBar" => Some(TypeTerm::Double),
        "tcltk::is.tclObj" | "tcltk::is.tkwin" => Some(TypeTerm::Logical),
        callee
            if callee.starts_with("tcltk::tk")
                || callee.starts_with("tcltk::ttk")
                || callee.starts_with("tcltk::tcl")
                || callee.starts_with("tcltk::.Tcl")
                || callee.starts_with("tcltk::.Tk") =>
        {
            Some(TypeTerm::Any)
        }
        _ => None,
    }
}
