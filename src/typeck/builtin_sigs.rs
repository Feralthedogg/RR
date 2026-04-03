//! Routing layer for builtin and package signature inference.
//!
//! Domain-specific signature tables live under `typeck/sigs/*`. This file keeps
//! the high-level dispatch order and small residual fallbacks that still span
//! multiple packages.

use super::lattice::{PrimTy, TypeState};
use super::term::TypeTerm;

pub(crate) use crate::typeck::sigs::base_builtin::*;

pub fn infer_package_call(callee: &str, arg_tys: &[TypeState]) -> Option<TypeState> {
    if let Some(inferred) = crate::typeck::sigs::base::infer_base_package_call(callee, arg_tys) {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::base_extra::infer_base_extra_package_call(callee, arg_tys)
    {
        return Some(inferred);
    }
    if let Some(inferred) = crate::typeck::sigs::utils::infer_utils_package_call(callee, arg_tys) {
        return Some(inferred);
    }
    if let Some(inferred) = crate::typeck::sigs::tools::infer_tools_package_call(callee, arg_tys) {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::parallel::infer_parallel_package_call(callee, arg_tys)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::splines::infer_splines_package_call(callee, arg_tys)
    {
        return Some(inferred);
    }
    if let Some(inferred) = crate::typeck::sigs::tcltk::infer_tcltk_package_call(callee, arg_tys) {
        return Some(inferred);
    }
    if let Some(inferred) = crate::typeck::sigs::stats4::infer_stats4_package_call(callee, arg_tys)
    {
        return Some(inferred);
    }
    if let Some(inferred) = crate::typeck::sigs::stats::infer_stats_package_call(callee, arg_tys) {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::graphics::infer_graphics_package_call(callee, arg_tys)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::methods::infer_methods_package_call(callee, arg_tys)
    {
        return Some(inferred);
    }
    match callee {
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
        _ => None,
    }
}

pub fn infer_package_binding(var: &str) -> Option<TypeState> {
    if let Some(inferred) = crate::typeck::sigs::datasets::infer_datasets_package_binding(var) {
        return Some(inferred);
    }
    match var {
        callee if callee.starts_with("base::") => Some(TypeState::unknown()),
        _ => None,
    }
}

pub fn infer_package_call_term(callee: &str, arg_terms: &[TypeTerm]) -> Option<TypeTerm> {
    if let Some(inferred) =
        crate::typeck::sigs::base::infer_base_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::base_extra::infer_base_extra_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::utils::infer_utils_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::tools::infer_tools_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::parallel::infer_parallel_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::splines::infer_splines_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::tcltk::infer_tcltk_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::stats4::infer_stats4_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::stats::infer_stats_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::graphics::infer_graphics_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    if let Some(inferred) =
        crate::typeck::sigs::methods::infer_methods_package_call_term(callee, arg_terms)
    {
        return Some(inferred);
    }
    match callee {
        "compiler::enableJIT" => Some(TypeTerm::Int),
        "compiler::compilePKGS" => Some(TypeTerm::Logical),
        "compiler::getCompilerOption" => Some(TypeTerm::Any),
        "compiler::setCompilerOptions" | "compiler::compile" | "compiler::disassemble" => {
            Some(TypeTerm::List(Box::new(TypeTerm::Any)))
        }
        "compiler::cmpfile" | "compiler::loadcmp" => Some(TypeTerm::Null),
        "compiler::cmpfun" => Some(TypeTerm::Any),
        _ => None,
    }
}

pub fn infer_package_binding_term(var: &str) -> Option<TypeTerm> {
    if let Some(inferred) = crate::typeck::sigs::datasets::infer_datasets_package_binding_term(var)
    {
        return Some(inferred);
    }
    match var {
        callee if callee.starts_with("base::") => Some(TypeTerm::Any),
        _ => None,
    }
}
