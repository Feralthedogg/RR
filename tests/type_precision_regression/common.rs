pub(crate) use RR::hir::def::Ty;
pub(crate) use RR::mir::{FnIR, Terminator, ValueKind};
pub(crate) use RR::typeck::solver::{
    TypeConfig, analyze_program, hir_ty_to_type_state, hir_ty_to_type_term,
};
pub(crate) use RR::typeck::{NaTy, PrimTy, ShapeTy, TypeTerm};
pub(crate) use RR::{mir::Facts, utils::Span};
pub(crate) use rustc_hash::FxHashMap;

pub(crate) fn fn_ir_param_df_term() -> TypeTerm {
    TypeTerm::DataFrameNamed(vec![
        (
            "x".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Double)),
        ),
        ("y".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Char))),
    ])
}
