use super::*;
impl<'a> MirLowerer<'a> {
    pub(crate) fn is_dynamic_fallback_builtin(name: &str) -> bool {
        crate::mir::semantics::call_model::is_dynamic_fallback_builtin(name)
    }

    pub(crate) fn is_namespaced_r_call(name: &str) -> bool {
        crate::mir::semantics::call_model::is_namespaced_r_call(name)
    }

    pub(crate) fn is_tidy_data_mask_call(name: &str) -> bool {
        crate::mir::semantics::call_model::is_tidy_data_mask_call(name)
    }

    pub(crate) fn is_tidy_helper_call(name: &str) -> bool {
        crate::mir::semantics::call_model::is_tidy_helper_call(name)
    }

    pub(crate) fn is_supported_package_call(name: &str) -> bool {
        crate::mir::semantics::call_model::is_supported_package_call(name)
    }

    pub(crate) fn is_supported_tidy_helper_call(name: &str) -> bool {
        crate::mir::semantics::call_model::is_supported_tidy_helper_call(name)
    }

    pub(crate) fn should_lower_as_tidy_symbol(name: &str) -> bool {
        !name.starts_with("rr_")
            && !Self::is_namespaced_r_call(name)
            && !Self::is_dynamic_fallback_builtin(name)
            && !Self::is_tidy_helper_call(name)
            && !matches!(
                name,
                "seq_along"
                    | "seq_len"
                    | "c"
                    | "list"
                    | "sum"
                    | "mean"
                    | "var"
                    | "prod"
                    | "min"
                    | "max"
                    | "abs"
                    | "sqrt"
                    | "sin"
                    | "cos"
                    | "tan"
                    | "asin"
                    | "acos"
                    | "atan"
                    | "atan2"
                    | "sinh"
                    | "cosh"
                    | "tanh"
                    | "log"
                    | "log10"
                    | "log2"
                    | "exp"
                    | "sign"
                    | "gamma"
                    | "lgamma"
                    | "floor"
                    | "ceiling"
                    | "trunc"
                    | "round"
                    | "pmax"
                    | "pmin"
                    | "print"
                    | "paste"
                    | "paste0"
                    | "sprintf"
                    | "cat"
                    | "names"
                    | "rownames"
                    | "colnames"
                    | "sort"
                    | "order"
                    | "match"
                    | "unique"
                    | "duplicated"
                    | "anyDuplicated"
                    | "any"
                    | "all"
                    | "which"
                    | "is.na"
                    | "is.finite"
                    | "numeric"
                    | "character"
                    | "logical"
                    | "integer"
                    | "double"
                    | "rep"
                    | "rep.int"
                    | "vector"
                    | "matrix"
                    | "dim"
                    | "dimnames"
                    | "nrow"
                    | "ncol"
                    | "colSums"
                    | "rowSums"
                    | "crossprod"
                    | "tcrossprod"
                    | "t"
                    | "diag"
                    | "rbind"
                    | "cbind"
            )
    }

    pub(crate) fn hybrid_interop_reason(name: &str) -> InteropReason {
        let (why, suggestion) = match name {
            "library" | "require" => (
                "package attachment mutates the runtime search path and cannot be proven stable at compile-time",
                Some(
                    "prefer `import r \"pkg\"`, `import r { ... } from \"pkg\"`, or `import r * as ns from \"pkg\"` for namespace-only access",
                ),
            ),
            "plot" | "lines" | "legend" | "png" | "dev.off" => (
                "unqualified plotting call depends on runtime package attachment and search-path resolution",
                Some(
                    "prefer namespaced R imports so the call lowers to `pkg::symbol(...)` directly",
                ),
            ),
            "eval" | "parse" => (
                "call evaluates code dynamically, so RR cannot stabilize the callee or its argument semantics ahead of time",
                Some(
                    "avoid runtime code construction or isolate it behind a dedicated dynamic boundary",
                ),
            ),
            "get" | "assign" | "exists" | "mget" | "rm" | "ls" => (
                "call reads or mutates environments dynamically, so symbol resolution depends on runtime state",
                Some(
                    "prefer explicit RR bindings or namespaced package imports when the target is known",
                ),
            ),
            "parent.frame" | "environment" | "sys.frame" | "sys.call" | "do.call" => (
                "call depends on runtime stack or environment state that RR cannot model statically",
                Some("pass the callee and arguments explicitly through RR values where possible"),
            ),
            _ => (
                "call uses a dynamic runtime feature that RR cannot reduce to stable direct interop",
                None,
            ),
        };
        InteropReason::new(
            InteropTier::Hybrid,
            InteropReasonKind::DynamicBuiltin,
            name,
            why,
            suggestion,
        )
    }

    pub(crate) fn opaque_package_reason(name: &str) -> InteropReason {
        InteropReason::new(
            InteropTier::Opaque,
            InteropReasonKind::PackageCall,
            name,
            "package call is preserved exactly, but RR has no dedicated semantic model for this symbol",
            Some(
                "keep the call namespaced or add this symbol to the direct interop surface if RR should reason about it",
            ),
        )
    }

    pub(crate) fn opaque_tidy_helper_reason(name: &str) -> InteropReason {
        InteropReason::new(
            InteropTier::Opaque,
            InteropReasonKind::TidyHelper,
            name,
            "tidy helper is forwarded as-is because RR does not model its selector semantics directly",
            Some(
                "prefer supported tidy helpers or add this helper to the direct tidy interop surface",
            ),
        )
    }
}
