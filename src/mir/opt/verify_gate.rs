use super::*;
use crate::{error::InternalCompilerError, error::Stage};

impl TachyonEngine {
    pub(crate) fn verify_or_panic(fn_ir: &FnIR, stage: &str) {
        if let Err(e) = crate::mir::verify::verify_ir(fn_ir) {
            Self::dump_verify_failure(fn_ir, stage, &e.to_string());
            InternalCompilerError::new(
                Stage::Opt,
                format!(
                    "MIR verification failed at {} for function '{}': {}",
                    stage, fn_ir.name, e
                ),
            )
            .into_exception()
            .display(None, None);
        }
    }

    pub(crate) fn verify_or_reject(fn_ir: &mut FnIR, stage: &str) -> bool {
        Self::clear_stale_phi_owner_metadata(fn_ir);
        match crate::mir::verify::verify_ir(fn_ir) {
            Ok(()) => true,
            Err(e) => {
                Self::dump_verify_failure(fn_ir, stage, &e.to_string());
                let reason = format!("invalid MIR at {}: {}", stage, e);
                fn_ir.mark_unsupported_dynamic(reason);
                false
            }
        }
    }

    pub(crate) fn verify_pre_dessa_end(fn_ir: &mut FnIR) -> bool {
        Self::clear_stale_phi_owner_metadata(fn_ir);
        match crate::mir::verify::verify_ir(fn_ir) {
            Ok(()) => true,
            Err(e) => {
                // Pre-deSSA final polish can still contain transient Phi shapes
                // that are resolved by the required de-SSA/codegen-prep stage.
                // Keep this as a debug/audit boundary; post-deSSA verification
                // remains the semantic enforcement point.
                Self::dump_verify_failure(fn_ir, "End/PreDeSSA", &e.to_string());
                false
            }
        }
    }

    pub(crate) fn clear_stale_phi_owner_metadata(fn_ir: &mut FnIR) {
        for value in &mut fn_ir.values {
            if !matches!(value.kind, ValueKind::Phi { .. }) {
                value.phi_block = None;
            }
        }
    }

    pub(crate) fn debug_stage_dump(fn_ir: &FnIR, stage: &str) {
        let Some(names) = std::env::var_os("RR_DEBUG_STAGE_FN") else {
            return;
        };
        let names = names.to_string_lossy().into_owned();
        let wanted: std::collections::HashSet<&str> = names
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .collect();
        if !wanted.contains(fn_ir.name.as_str()) {
            return;
        }
        if let Some(filter) = std::env::var_os("RR_DEBUG_STAGE_MATCH") {
            let filter = filter.to_string_lossy();
            if !stage.contains(filter.as_ref()) {
                return;
            }
        }
        eprintln!(
            "=== RR_DEBUG_STAGE {} :: {} ===\n{:#?}",
            fn_ir.name, stage, fn_ir
        );
    }
}
