pub(crate) use rr::Span;
pub(crate) use rr::compiler::internal::hir::def::Ty;
pub(crate) use rr::compiler::internal::mir::Facts;
pub(crate) use rr::compiler::internal::mir::{FnIR, Terminator, ValueId, ValueKind};
pub(crate) use rr::compiler::internal::typeck::solver::{
    TypeConfig, analyze_program, hir_ty_to_type_state, hir_ty_to_type_term,
};
pub(crate) use rr::compiler::internal::typeck::{NaTy, PrimTy, ShapeTy, TypeTerm};
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

pub(crate) fn add_load(fn_ir: &mut FnIR, var: &str) -> ValueId {
    fn_ir.add_value(
        ValueKind::Load {
            var: var.to_string(),
        },
        Span::dummy(),
        Facts::empty(),
        None,
    )
}

pub(crate) fn add_str(fn_ir: &mut FnIR, value: &str) -> ValueId {
    fn_ir.add_value(
        ValueKind::Const(rr::compiler::internal::syntax::ast::Lit::Str(
            value.to_string(),
        )),
        Span::dummy(),
        Facts::empty(),
        None,
    )
}

pub(crate) fn add_call(fn_ir: &mut FnIR, callee: &str, args: Vec<ValueId>) -> ValueId {
    let names = vec![None; args.len()];
    fn_ir.add_value(
        ValueKind::Call {
            callee: callee.to_string(),
            args,
            names,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    )
}
