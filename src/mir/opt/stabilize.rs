use super::*;

impl TachyonEngine {
    // Required lowering-to-codegen stabilization passes.
    // This must run even in O0, because codegen cannot emit Phi.
    fn stabilize_for_codegen_inner(
        &self,
        all_fns: &mut FxHashMap<String, FnIR>,
        verify_start: bool,
    ) {
        let floor_helpers = Self::collect_floor_helpers(all_fns);
        if !floor_helpers.is_empty() {
            let _ = Self::rewrite_floor_helper_calls(all_fns, &floor_helpers);
        }
        let abs_helpers = Self::collect_trivial_abs_helpers(all_fns);
        if !abs_helpers.is_empty() {
            let _ = Self::rewrite_trivial_abs_helper_calls(all_fns, &abs_helpers);
        }
        let unit_index_helpers = Self::collect_unit_index_helpers(all_fns);
        if !unit_index_helpers.is_empty() {
            let _ = Self::rewrite_unit_index_helper_calls(all_fns, &unit_index_helpers);
        }
        let minmax_helpers = Self::collect_trivial_minmax_helpers(all_fns);
        if !minmax_helpers.is_empty() {
            let _ = Self::rewrite_trivial_minmax_helper_calls(all_fns, &minmax_helpers);
        }
        let clamp_helpers = Self::collect_trivial_clamp_helpers(all_fns);
        if !clamp_helpers.is_empty() {
            let _ = Self::rewrite_trivial_clamp_helper_calls(all_fns, &clamp_helpers);
        }
        let wrap_index_helpers = Self::collect_wrap_index_helpers(all_fns);
        if !wrap_index_helpers.is_empty() {
            let _ = Self::rewrite_wrap_index_helper_calls(all_fns, &wrap_index_helpers);
        }
        let periodic_index_helpers = Self::collect_periodic_index_helpers(all_fns);
        if !periodic_index_helpers.is_empty() {
            let _ = Self::rewrite_periodic_index_helper_calls(all_fns, &periodic_index_helpers);
        }
        let cube_index_helpers = Self::collect_cube_index_helpers(all_fns);
        if !cube_index_helpers.is_empty() {
            let _ = Self::rewrite_cube_index_helper_calls(all_fns, &cube_index_helpers);
        }
        let ordered_names = Self::sorted_fn_names(all_fns);
        for name in &ordered_names {
            let Some(fn_ir) = all_fns.get_mut(name) else {
                continue;
            };
            if verify_start && !Self::verify_or_reject(fn_ir, "PrepareForCodegen/Start") {
                continue;
            }
            let _ = de_ssa::run(fn_ir);
            // Keep this lightweight but convergent to avoid dead temp noise after De-SSA.
            // Conservative interop functions skip cleanup to preserve package/runtime semantics.
            if !fn_ir.requires_conservative_optimization() {
                let mut changed = true;
                let mut guard = 0;
                while changed && guard < 8 {
                    guard += 1;
                    changed = false;
                    changed |= self.simplify_cfg(fn_ir);
                    changed |= self.dce(fn_ir);
                }
            }
            let _ = Self::verify_or_reject(fn_ir, "PrepareForCodegen/End");
        }
    }

    pub fn stabilize_for_codegen(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        self.stabilize_for_codegen_inner(all_fns, true);
    }

    pub fn stabilize_for_codegen_relaxed_start(&self, all_fns: &mut FxHashMap<String, FnIR>) {
        self.stabilize_for_codegen_inner(all_fns, false);
    }
}
