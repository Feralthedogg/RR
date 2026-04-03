//! Call-target validation and suggestion helpers for the semantics layer.
//!
//! This module keeps user-facing call diagnostics deterministic by centralizing
//! the known callable surface, fallback rules, and spelling suggestions.

use crate::error::{RR, RRCode, RRException, Stage};
use crate::utils::Span;
use rustc_hash::FxHashMap;

#[path = "call_model_suggest.rs"]
mod call_model_suggest;
#[path = "call_model_surfaces.rs"]
mod call_model_surfaces;

use self::call_model_suggest::function_name_suggestion_candidates;
pub(super) use self::call_model_surfaces::{
    builtin_arity, is_runtime_helper, is_runtime_reserved_symbol,
};
pub(crate) use self::call_model_surfaces::{
    is_dynamic_fallback_builtin, is_namespaced_r_call, is_supported_package_call,
    is_supported_tidy_helper_call, is_tidy_data_mask_call, is_tidy_helper_call,
};

#[derive(Debug, Clone)]
pub(super) struct UserFnSignature {
    pub param_names: Vec<String>,
    pub has_default: Vec<bool>,
}

pub(super) fn suggest_function_name(
    callee: &str,
    user_signatures: &FxHashMap<String, UserFnSignature>,
) -> Option<String> {
    super::suggest_name(
        callee,
        user_signatures.keys().cloned().chain(
            function_name_suggestion_candidates()
                .iter()
                .map(|name| (*name).to_string()),
        ),
    )
}

pub(super) fn validate_call_target(
    callee: &str,
    argc: usize,
    names: &[Option<String>],
    span: Span,
    user_signatures: &FxHashMap<String, UserFnSignature>,
) -> RR<()> {
    if let Some(signature) = user_signatures.get(callee) {
        let mut bound = vec![false; signature.param_names.len()];
        let mut next_positional = 0usize;

        for name in names {
            if let Some(name) = name {
                let Some(index) = signature.param_names.iter().position(|param| param == name)
                else {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Mir,
                        format!("function '{}' has no parameter named '{}'", callee, name),
                    )
                    .at(span)
                    .push_frame("mir::semantics::validate_call_target/5", Some(span)));
                };
                if bound[index] {
                    return Err(RRException::new(
                        "RR.SemanticError",
                        RRCode::E1002,
                        Stage::Mir,
                        format!(
                            "function '{}' received duplicate argument '{}'",
                            callee, name
                        ),
                    )
                    .at(span)
                    .push_frame("mir::semantics::validate_call_target/5", Some(span)));
                }
                bound[index] = true;
                continue;
            }

            while next_positional < bound.len() && bound[next_positional] {
                next_positional += 1;
            }
            if next_positional >= bound.len() {
                return Err(RRException::new(
                    "RR.SemanticError",
                    RRCode::E1002,
                    Stage::Mir,
                    format!(
                        "function '{}' expects at most {} argument(s), got {}",
                        callee,
                        signature.param_names.len(),
                        argc
                    ),
                )
                .at(span)
                .push_frame("mir::semantics::validate_call_target/5", Some(span)));
            }
            bound[next_positional] = true;
            next_positional += 1;
        }

        let missing_required = signature
            .param_names
            .iter()
            .zip(signature.has_default.iter())
            .zip(bound.iter())
            .filter_map(|((name, has_default), is_bound)| {
                (!*is_bound && !*has_default).then_some(name.clone())
            })
            .collect::<Vec<_>>();
        if !missing_required.is_empty() {
            let legacy_exact_arity = signature
                .has_default
                .iter()
                .all(|has_default| !*has_default)
                && names.iter().all(|name| name.is_none());
            let message = if legacy_exact_arity {
                format!(
                    "function '{}' expects {} argument(s), got {}",
                    callee,
                    signature.param_names.len(),
                    argc
                )
            } else {
                format!(
                    "function '{}' is missing required argument(s): {}",
                    callee,
                    missing_required.join(", ")
                )
            };
            return Err(
                RRException::new("RR.SemanticError", RRCode::E1002, Stage::Mir, message)
                    .at(span)
                    .push_frame("mir::semantics::validate_call_target/5", Some(span)),
            );
        }
        return Ok(());
    }

    if let Some((min, max)) = builtin_arity(callee) {
        if argc < min || max.is_some_and(|m| argc > m) {
            let upper = max
                .map(|m| m.to_string())
                .unwrap_or_else(|| "inf".to_string());
            return Err(RRException::new(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Mir,
                format!(
                    "builtin '{}' expects {}..{} argument(s), got {}",
                    callee, min, upper, argc
                ),
            )
            .at(span)
            .push_frame("mir::semantics::validate_call_target/4", Some(span)));
        }
        return Ok(());
    }

    if is_dynamic_fallback_builtin(callee)
        || is_namespaced_r_call(callee)
        || is_supported_package_call(callee)
        || is_tidy_helper_call(callee)
        || is_supported_tidy_helper_call(callee)
        || is_runtime_helper(callee)
    {
        return Ok(());
    }

    let mut err = RRException::new(
        "RR.SemanticError",
        RRCode::E1001,
        Stage::Mir,
        format!("undefined function '{}'", callee),
    )
    .at(span)
    .push_frame("mir::semantics::validate_call_target/5", Some(span))
    .note("Define the function before calling it, or import the module that provides it.");
    if let Some(suggestion) = suggest_function_name(callee, user_signatures) {
        err = err.help(suggestion);
    }
    Err(err)
}

#[cfg(test)]
mod tests {
    use super::is_supported_package_call;

    #[test]
    fn base_prefix_fallback_accepts_named_exports() {
        for name in ["base::R.home", "base::gcinfo", "base::findRestart"] {
            assert!(
                is_supported_package_call(name),
                "expected direct support for {name}"
            );
        }
    }

    #[test]
    fn base_prefix_fallback_accepts_operator_and_assignment_exports() {
        for name in [
            "base::+",
            "base::[",
            "base::[[",
            "base::$",
            "base::[[<-",
            "base::attr<-",
            "base::environment<-",
        ] {
            assert!(
                is_supported_package_call(name),
                "expected direct support for {name}"
            );
        }
    }

    #[test]
    fn base_prefix_fallback_does_not_open_other_packages() {
        assert!(!is_supported_package_call("unknownpkg::foo"));
    }
}
