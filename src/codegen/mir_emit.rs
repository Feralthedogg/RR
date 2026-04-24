//! MIR-to-R emission coordinator.
//!
//! The heavy lifting is delegated to `codegen/emit/*` and `codegen/backend/*`.
//! This file keeps the entrypoints, shared regex helpers, and wiring that tie
//! those child modules into a single deterministic emission pipeline.

use crate::codegen::backend::state::{
    ActiveScalarLoopIndex, BranchSnapshot, LastAssignedValueUndo, RBackend, ScalarLoopCmp,
    ValueBindingUndo, VarValueBindingUndo, VarVersionUndo,
};
use crate::mir::def::{
    BinOp, BlockId, FnIR, Instr, IntrinsicOp, Lit, Terminator, UnaryOp, Value, ValueKind,
};
use crate::mir::flow::Facts;
use crate::mir::opt::poly::is_generated_poly_loop_var_name;
use crate::mir::structurizer::{StructuredBlock, Structurizer};
use crate::typeck::{PrimTy, ShapeTy, TypeTerm};
use crate::utils::Span;
use regex::{Captures, Regex};
use rustc_hash::{FxHashMap, FxHashSet};

pub use crate::codegen::backend::state::MapEntry;

const IDENT_PATTERN: &str = r"(?:[A-Za-z_][A-Za-z0-9._]*|\.[A-Za-z_][A-Za-z0-9._]*)";
const GENERATED_POLY_LOOP_IV_PREFIX: &str = ".__poly_gen_iv_";

#[path = "emit/assign.rs"]
mod assign;
#[path = "emit/bindings.rs"]
mod bindings_emit;
#[path = "emit/branches.rs"]
mod branches_emit;
#[path = "emit/cse.rs"]
mod cse_emit;
#[path = "emit/cse_prune.rs"]
mod cse_prune_emit;
#[path = "emit/index.rs"]
mod index_emit;
#[path = "emit/instr.rs"]
mod instr_emit;
#[path = "emit/render.rs"]
mod render_emit;
#[path = "emit/resolve.rs"]
mod resolve_emit;
#[path = "emit/rewrite.rs"]
mod rewrite_emit;
#[path = "emit/structured_analysis.rs"]
mod structured_analysis_emit;
#[path = "emit/structured.rs"]
mod structured_emit;

fn compile_regex(pattern: String) -> Option<Regex> {
    Regex::new(&pattern).ok()
}

pub struct MirEmitter {
    pub(crate) backend: RBackend,
}

fn is_recognized_loop_index_name(name: &str) -> bool {
    matches!(name, "i" | "j" | "k")
        || name.starts_with("i_")
        || name.starts_with("j_")
        || name.starts_with("k_")
        || is_generated_poly_loop_var_name(name)
}

impl RBackend {
    fn emitted_callee_name(callee: &str) -> String {
        match callee {
            // Some `tools` namespace database helpers have shifted between exported
            // and namespace-internal across R releases. Emitting `:::` keeps the
            // RR surface stable for package-alias calls while remaining compatible
            // with versions where `::` no longer resolves them.
            "tools::standard_package_names" => "tools:::standard_package_names".to_string(),
            "tools::base_aliases_db" => "tools:::base_aliases_db".to_string(),
            "tools::base_rdxrefs_db" => "tools:::base_rdxrefs_db".to_string(),
            "tools::CRAN_aliases_db" => "tools:::CRAN_aliases_db".to_string(),
            "tools::CRAN_archive_db" => "tools:::CRAN_archive_db".to_string(),
            "tools::CRAN_package_db" => "tools:::CRAN_package_db".to_string(),
            "tools::CRAN_authors_db" => "tools:::CRAN_authors_db".to_string(),
            "tools::CRAN_current_db" => "tools:::CRAN_current_db".to_string(),
            "tools::CRAN_check_results" => "tools:::CRAN_check_results".to_string(),
            "tools::CRAN_check_details" => "tools:::CRAN_check_details".to_string(),
            "tools::CRAN_check_issues" => "tools:::CRAN_check_issues".to_string(),
            "tools::CRAN_rdxrefs_db" => "tools:::CRAN_rdxrefs_db".to_string(),
            // `qr.influence` is namespace-internal on older R releases used in CI.
            "stats::qr.influence" => "stats:::qr.influence".to_string(),
            _ => callee.to_string(),
        }
    }

    fn in_active_loop_mutated_context(&self, var: &str) -> bool {
        self.loop_analysis
            .active_loop_mutated_vars
            .iter()
            .rev()
            .any(|vars| vars.contains(var))
    }

    pub fn emit_function(
        &mut self,
        fn_ir: &FnIR,
    ) -> Result<(String, Vec<MapEntry>), crate::error::RRException> {
        if let Err(err) = crate::mir::verify::verify_emittable_ir(fn_ir) {
            return Err(crate::error::RRException::new(
                "codegen",
                crate::error::RRCode::ICE9001,
                crate::error::Stage::Codegen,
                err.to_string(),
            ));
        }
        self.reset_emit_output_state();

        let wrapper_plan = Self::typed_parallel_wrapper_plan(fn_ir);
        if let Some(plan) = wrapper_plan.as_ref() {
            self.emit_function_named(fn_ir, &plan.impl_name)?;
            self.newline();
            self.emit_typed_parallel_wrapper(fn_ir, plan);
        } else {
            self.emit_function_named(fn_ir, fn_ir.name.as_str())?;
        }
        if std::env::var_os("RR_DEBUG_EMIT_PRE_REWRITE").is_some() {
            eprintln!(
                "=== RR_DEBUG_EMIT_PRE_REWRITE {} ===\n{}",
                fn_ir.name, self.output
            );
        }
        Self::rewrite_safe_scalar_loop_index_helpers(&mut self.output);
        Self::rewrite_branch_local_identical_alloc_rebinds(&mut self.output);
        Self::hoist_branch_local_pure_scalar_assigns_used_after_branch(&mut self.output);
        Self::rewrite_single_use_scalar_index_aliases(&mut self.output);
        Self::rewrite_immediate_and_guard_named_scalar_exprs(&mut self.output);
        Self::rewrite_two_use_named_scalar_exprs(&mut self.output);
        Self::rewrite_small_multiuse_scalar_index_aliases(&mut self.output);
        Self::rewrite_one_or_two_use_named_scalar_index_reads_in_straight_line_region(
            &mut self.output,
        );
        Self::rewrite_named_scalar_pure_call_aliases(&mut self.output);
        Self::rewrite_loop_index_alias_ii(&mut self.output);
        Self::strip_dead_zero_seed_ii(&mut self.output);
        Self::rewrite_slice_bound_aliases(&mut self.output);
        Self::rewrite_particle_idx_alias(&mut self.output);
        Self::rewrite_adjacent_duplicate_symbol_assignments(&mut self.output);
        Self::rewrite_duplicate_pure_call_assignments(
            &mut self.output,
            self.analysis.known_pure_user_calls.as_ref(),
        );
        Self::strip_noop_self_assignments(&mut self.output);
        Self::rewrite_temp_uses_after_named_copy(&mut self.output);
        Self::strip_noop_temp_copy_roundtrips(&mut self.output);
        Self::strip_dead_simple_scalar_assigns(&mut self.output);
        Self::strip_shadowed_simple_scalar_seed_assigns(&mut self.output);
        Self::strip_dead_seq_len_locals(&mut self.output);
        Self::strip_redundant_branch_local_vec_fill_rebinds(&mut self.output);
        Self::strip_unused_raw_arg_aliases(&mut self.output);
        Self::rewrite_readonly_raw_arg_aliases(&mut self.output);
        Self::strip_empty_else_blocks(&mut self.output);
        Self::collapse_nested_else_if_blocks(&mut self.output);
        Self::rewrite_guard_scalar_literals(&mut self.output);
        Self::rewrite_loop_guard_scalar_literals(&mut self.output);
        Self::rewrite_single_assignment_loop_seed_literals(&mut self.output);
        Self::rewrite_sym210_loop_seed(&mut self.output);
        Self::rewrite_seq_len_full_overwrite_inits(&mut self.output);
        Self::restore_missing_repeat_loop_counter_updates(&mut self.output);
        Self::rewrite_hoisted_loop_counter_aliases(&mut self.output);
        Self::repair_missing_cse_range_aliases(&mut self.output);
        Self::restore_constant_one_guard_repeat_loop_counters(&mut self.output);
        Self::rewrite_literal_named_list_calls(&mut self.output);
        Self::rewrite_literal_field_get_calls(&mut self.output);
        Self::strip_redundant_tail_assign_slice_return(&mut self.output);
        Self::strip_unreachable_sym_helpers(&mut self.output);
        if std::env::var_os("RR_DEBUG_EMIT_POST_SAFE_REWRITE").is_some() {
            eprintln!(
                "=== RR_DEBUG_EMIT_POST_SAFE_REWRITE {} ===\n{}",
                fn_ir.name, self.output
            );
        }
        Self::restore_missing_generated_poly_loop_steps(&mut self.output);
        Self::repair_missing_cse_range_aliases(&mut self.output);
        if std::env::var_os("RR_DEBUG_EMIT_POST_STEP_RESTORE").is_some() {
            eprintln!(
                "=== RR_DEBUG_EMIT_POST_STEP_RESTORE {} ===\n{}",
                fn_ir.name, self.output
            );
        }
        Self::strip_terminal_repeat_nexts(&mut self.output);
        Self::prune_dead_cse_temps(&mut self.output);
        Self::strip_orphan_rr_cse_pruned_markers(&mut self.output);
        Self::strip_single_blank_spacers(&mut self.output);
        Self::compact_blank_lines(&mut self.output);
        if std::env::var_os("RR_DEBUG_EMIT_POST_PRUNE").is_some() {
            eprintln!(
                "=== RR_DEBUG_EMIT_POST_PRUNE {} ===\n{}",
                fn_ir.name, self.output
            );
        }

        Ok((
            std::mem::take(&mut self.output),
            std::mem::take(&mut self.source_map),
        ))
    }

    fn emit_function_named(
        &mut self,
        fn_ir: &FnIR,
        emitted_name: &str,
    ) -> Result<(), crate::error::RRException> {
        self.prepare_function_emit_state(fn_ir);

        self.write(emitted_name);
        self.write(" <- function(");
        for (idx, param) in fn_ir.params.iter().enumerate() {
            if idx > 0 {
                self.write(", ");
            }
            self.write(param);
            if let Some(Some(default_expr)) = fn_ir.param_default_r_exprs.get(idx) {
                self.write(" = ");
                self.write(default_expr);
            }
        }
        self.write(") ");
        self.newline();
        self.write_indent();
        self.write("{");
        self.newline();
        self.indent += 1;

        if fn_ir.unsupported_dynamic {
            self.write_stmt(&format!(
                "# rr-hybrid-fallback: {}",
                if fn_ir.fallback_reasons.is_empty() {
                    "dynamic runtime feature detected".to_string()
                } else {
                    fn_ir.fallback_reasons.join(", ")
                }
            ));
        }
        if fn_ir.opaque_interop {
            self.write_stmt(&format!(
                "# rr-opaque-interop: {}",
                if fn_ir.opaque_reasons.is_empty() {
                    "package/runtime interop requires conservative optimization".to_string()
                } else {
                    fn_ir.opaque_reasons.join(", ")
                }
            ));
        }

        // Proof correspondence:
        // `proof/lean/RRProofs/CodegenSubset.lean`,
        // `proof/lean/RRProofs/PipelineBlockEnvSubset.lean`,
        // `proof/lean/RRProofs/PipelineFnEnvSubset.lean`,
        // `proof/lean/RRProofs/PipelineFnCfgSubset.lean`, and the Coq
        // `Codegen*` / `Pipeline*Subset` files approximate reduced slices of
        // the semantics under this structured MIR emission path.
        let structured = Structurizer::new(fn_ir).build();
        if std::env::var_os("RR_DEBUG_STRUCTURED").is_some() {
            eprintln!(
                "=== RR_DEBUG_STRUCTURED {} ===\n{:#?}",
                fn_ir.name, structured
            );
        }
        self.emit_structured(&structured, fn_ir)?;

        self.indent -= 1;
        self.write_indent();
        self.write("}");
        self.newline();
        Ok(())
    }

    fn record_span(&mut self, span: Span) {
        if span.start_line != 0 {
            self.source_map.push(MapEntry {
                r_line: self.current_line,
                rr_span: span,
            });
        }
    }

    fn try_resolve_whole_range_self_assign_rhs(
        &self,
        dst: &str,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        assign::try_resolve_whole_range_self_assign_rhs(self, dst, src, values, params)
    }

    fn try_render_constant_safe_partial_self_assign(
        &self,
        dst: &str,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        assign::try_render_constant_safe_partial_self_assign(self, dst, src, values, params)
    }

    fn try_render_safe_idx_cube_row_slice_assign(
        &self,
        dst: &str,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        assign::try_render_safe_idx_cube_row_slice_assign(self, dst, src, values, params)
    }

    fn resolve_preferred_live_expr_alias(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        if let Some(ValueKind::Load { var }) = values.get(val_id).map(|v| &v.kind) {
            if let Some(alias) = self.resolve_readonly_arg_alias_name(var, values) {
                return alias;
            }
            if !var.starts_with('.') {
                return var.clone();
            }
        }
        if let Some(bound) = self.resolve_bound_value(val_id)
            && !bound.starts_with('.')
        {
            return bound;
        }
        if let Some(alias) = self.resolve_live_same_kind_scalar_alias(val_id, values) {
            return alias;
        }
        let rendered =
            self.resolve_bound_temp_expr(val_id, values, params, &mut FxHashSet::default());
        if Self::is_plain_symbol_expr(rendered.as_str()) {
            return rendered;
        }
        self.find_live_plain_symbol_for_exact_expr(val_id, rendered.as_str(), values, params)
            .unwrap_or(rendered)
    }

    fn find_live_plain_symbol_for_exact_expr(
        &self,
        val_id: usize,
        expr: &str,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        let mut candidate: Option<String> = None;
        for (var, (bound_val_id, version)) in &self.value_tracker.var_value_bindings {
            if var.starts_with('.') || self.current_var_version(var) != *version {
                continue;
            }
            let bound_expr = self.rewrite_live_readonly_arg_aliases(
                self.resolve_val(*bound_val_id, values, params, true),
                values,
            );
            if bound_expr != expr {
                continue;
            }
            if candidate.is_some() {
                return None;
            }
            candidate = Some(var.clone());
        }
        if candidate.is_some() {
            return candidate;
        }

        let target_expanded = self.resolve_expanded_scalar_expr_for_equivalence(
            val_id,
            values,
            params,
            &mut FxHashSet::default(),
        );
        let mut expanded_candidate: Option<String> = None;
        for (var, (bound_val_id, version)) in &self.value_tracker.var_value_bindings {
            if var.starts_with('.') || self.current_var_version(var) != *version {
                continue;
            }
            let bound_expanded = self.resolve_expanded_scalar_expr_for_equivalence(
                *bound_val_id,
                values,
                params,
                &mut FxHashSet::default(),
            );
            if bound_expanded != target_expanded {
                continue;
            }
            if expanded_candidate.is_some() {
                return None;
            }
            expanded_candidate = Some(var.clone());
        }
        expanded_candidate
    }

    fn resolve_live_same_kind_scalar_alias(
        &self,
        val_id: usize,
        values: &[Value],
    ) -> Option<String> {
        if values
            .get(val_id)
            .is_some_and(|value| self.is_fresh_mutable_aggregate_value(value))
        {
            return None;
        }
        if !self.can_reuse_live_expr_alias(val_id, values) {
            return None;
        }
        if !matches!(
            values.get(val_id).map(|value| &value.kind),
            Some(
                ValueKind::Binary { .. }
                    | ValueKind::Unary { .. }
                    | ValueKind::FieldGet { .. }
                    | ValueKind::Len { .. }
                    | ValueKind::Indices { .. }
                    | ValueKind::Range { .. }
                    | ValueKind::Call { .. }
            )
        ) {
            return None;
        }

        let mut candidate: Option<String> = None;
        for (var, (bound_val_id, version)) in &self.value_tracker.var_value_bindings {
            if var.starts_with('.') || self.current_var_version(var) != *version {
                continue;
            }
            let same_value = *bound_val_id == val_id;
            let same_kind = values.get(*bound_val_id).map(|value| &value.kind)
                == values.get(val_id).map(|value| &value.kind);
            if !same_value && !same_kind {
                continue;
            }
            if candidate.is_some() {
                return None;
            }
            candidate = Some(var.clone());
        }
        candidate
    }

    fn idx_cube_row_size_expr(
        &self,
        start: usize,
        end: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        assign::idx_cube_row_size_expr(self, start, end, values, params)
    }

    fn value_matches_known_length_expr(
        &self,
        val_id: usize,
        target_end_expr: &str,
        values: &[Value],
        params: &[String],
    ) -> bool {
        assign::value_matches_known_length_expr(self, val_id, target_end_expr, values, params)
    }

    fn rep_int_matches_slice_len(
        &self,
        val_id: usize,
        start: i64,
        end: i64,
        values: &[Value],
    ) -> bool {
        assign::rep_int_matches_slice_len(self, val_id, start, end, values)
    }

    fn try_resolve_whole_range_call_map_rhs(
        &self,
        dst: &str,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        assign::try_resolve_whole_range_call_map_rhs(self, dst, src, values, params)
    }

    fn try_resolve_whole_auto_call_map_rhs(
        &self,
        dst: &str,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        assign::try_resolve_whole_auto_call_map_rhs(self, dst, src, values, params)
    }

    fn try_resolve_mutated_whole_range_copy_alias(
        &self,
        src: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        assign::try_resolve_mutated_whole_range_copy_alias(self, src, values, params)
    }

    fn resolve_bound_temp_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> String {
        assign::resolve_bound_temp_expr(self, val_id, values, params, seen)
    }

    fn resolve_expanded_scalar_expr_for_equivalence(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> String {
        assign::resolve_expanded_scalar_expr_for_equivalence(self, val_id, values, params, seen)
    }

    fn resolve_named_mutable_base_var(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> Option<String> {
        assign::resolve_named_mutable_base_var(self, val_id, values, params)
    }

    fn resolve_mutated_descendant_var(&self, val_id: usize) -> Option<String> {
        assign::resolve_mutated_descendant_var(self, val_id)
    }

    fn is_plain_symbol_expr(expr: &str) -> bool {
        assign::is_plain_symbol_expr(expr)
    }

    fn direct_call_map_slots_supported(
        &self,
        callee_name: &str,
        arg_count: usize,
        vector_slots_val: usize,
        values: &[Value],
    ) -> bool {
        assign::direct_call_map_slots_supported(
            self,
            callee_name,
            arg_count,
            vector_slots_val,
            values,
        )
    }

    fn const_int_vector_values(&self, val_id: usize, values: &[Value]) -> Option<Vec<i64>> {
        assign::const_int_vector_values(self, val_id, values)
    }

    fn const_int_value(&self, val_id: usize, values: &[Value]) -> Option<i64> {
        assign::const_int_value(self, val_id, values)
    }

    fn const_int_value_impl(
        &self,
        val_id: usize,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> Option<i64> {
        assign::const_int_value_impl(self, val_id, values, seen)
    }

    fn const_index_int_value(&self, val_id: usize, values: &[Value]) -> Option<i64> {
        assign::const_index_int_value(self, val_id, values)
    }

    fn value_requires_runtime_auto_profit_guard(
        &self,
        val_id: usize,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        assign::value_requires_runtime_auto_profit_guard(self, val_id, values, seen)
    }

    fn direct_whole_range_call_map_expr(
        &self,
        callee_name: &str,
        rendered_args: &[String],
    ) -> Option<String> {
        assign::direct_whole_range_call_map_expr(self, callee_name, rendered_args)
    }

    fn render_call_map_whole_auto_expr(
        &self,
        dest: &str,
        callee_name: &str,
        helper_cost: &str,
        vector_slots: &str,
        rendered_args: &[String],
    ) -> String {
        assign::render_call_map_whole_auto_expr(
            self,
            dest,
            callee_name,
            helper_cost,
            vector_slots,
            rendered_args,
        )
    }

    fn const_string_value(&self, val_id: usize, values: &[Value]) -> Option<String> {
        assign::const_string_value(self, val_id, values)
    }

    fn normalize_whole_range_vector_expr(
        &self,
        expr: String,
        start: usize,
        end: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        assign::normalize_whole_range_vector_expr(self, expr, start, end, values, params)
    }

    fn wrap_backend_builtin_expr(&self, expr: &str) -> String {
        assign::wrap_backend_builtin_expr(self, expr)
    }

    fn rewrite_known_one_based_full_range_alias_reads(
        &self,
        expr: &str,
        values: &[Value],
        params: &[String],
    ) -> String {
        assign::rewrite_known_one_based_full_range_alias_reads(self, expr, values, params)
    }

    fn expr_is_one_based_full_range_for_end(idx_expr: &str, end_expr: &str) -> bool {
        assign::expr_is_one_based_full_range_for_end(idx_expr, end_expr)
    }

    fn extract_one_based_alias_name(idx_expr: &str) -> Option<String> {
        assign::extract_one_based_alias_name(idx_expr)
    }

    fn value_is_full_dest_end(
        &self,
        base: usize,
        end: usize,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        assign::value_is_full_dest_end(self, base, end, values, params, seen)
    }

    fn rewrite_known_full_range_index_reads(
        &self,
        expr: &str,
        start: usize,
        end: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        let mut out = expr.to_string();

        let start_exprs = self.full_range_start_spellings(start, values, params);
        let end_expr = regex::escape(self.resolve_val(end, values, params, false).trim());
        for start_expr in start_exprs {
            let escaped_start = regex::escape(start_expr.trim());
            for pattern in [
                format!(
                    r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*:\s*{}\)",
                    IDENT_PATTERN, escaped_start, end_expr
                ),
                format!(
                    r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*rr_index_vec_floor\(\s*{}\s*:\s*{}\s*\)\)",
                    IDENT_PATTERN, escaped_start, end_expr
                ),
            ] {
                if let Some(re) = compile_regex(pattern) {
                    out = re
                        .replace_all(&out, |caps: &Captures<'_>| {
                            caps.name("base")
                                .map(|m| m.as_str())
                                .unwrap_or("")
                                .to_string()
                        })
                        .to_string();
                }
            }
        }

        for (var, (val_id, version)) in &self.value_tracker.var_value_bindings {
            let temp_stale_ok =
                var.starts_with(".__rr_cse_") || var.starts_with(".tachyon_exprmap");
            if !var.starts_with('.')
                || (self.current_var_version(var) != *version && !temp_stale_ok)
            {
                continue;
            }
            if !self.value_is_full_range_alias(*val_id, start, end, values, params) {
                continue;
            }
            let pattern = format!(
                r"rr_index1_read_vec(?:_floor)?\((?P<base>{}),\s*{}\s*\)",
                IDENT_PATTERN,
                regex::escape(var),
            );
            if let Some(re) = compile_regex(pattern) {
                out = re
                    .replace_all(&out, |caps: &Captures<'_>| {
                        caps.name("base")
                            .map(|m| m.as_str())
                            .unwrap_or("")
                            .to_string()
                    })
                    .to_string();
            }
        }

        out
    }

    fn full_range_start_spellings(
        &self,
        start: usize,
        values: &[Value],
        params: &[String],
    ) -> Vec<String> {
        let mut out = Vec::new();
        let rendered = self.resolve_val(start, values, params, false);
        out.push(rendered);
        for one in ["1L", "1", "1.0"] {
            if !out.iter().any(|s| s == one) && self.value_is_known_one(start, values) {
                out.push(one.to_string());
            }
        }
        out
    }

    fn value_is_known_one(&self, val_id: usize, values: &[Value]) -> bool {
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Const(Lit::Int(1))) => true,
            Some(ValueKind::Const(Lit::Float(f))) if (*f - 1.0).abs() <= f64::EPSILON => true,
            Some(ValueKind::Load { var }) => self
                .resolve_bound_value_id(var)
                .is_some_and(|bound| self.value_is_known_one(bound, values)),
            _ => false,
        }
    }

    fn value_is_full_range_alias(
        &self,
        val_id: usize,
        start: usize,
        end: usize,
        values: &[Value],
        params: &[String],
    ) -> bool {
        match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Range { start: s, end: e }) => {
                self.value_is_known_one(*s, values)
                    && self.resolve_val(*e, values, params, false)
                        == self.resolve_val(end, values, params, false)
                    && self.value_is_known_one(start, values)
            }
            Some(ValueKind::Call { callee, args, .. })
                if callee == "rr_index_vec_floor" && args.len() == 1 =>
            {
                self.value_is_full_range_alias(args[0], start, end, values, params)
            }
            Some(ValueKind::Load { var }) => {
                self.resolve_bound_value_id(var).is_some_and(|bound| {
                    self.value_is_full_range_alias(bound, start, end, values, params)
                })
            }
            _ => false,
        }
    }

    fn value_is_one_based_full_range_alias(
        &self,
        val_id: usize,
        end_expr: &str,
        values: &[Value],
        params: &[String],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(val_id) {
            return false;
        }
        let ok = match values.get(val_id).map(|v| &v.kind) {
            Some(ValueKind::Range { start, end }) => {
                self.value_is_known_one(*start, values)
                    && self.resolve_val(*end, values, params, false) == end_expr
            }
            Some(ValueKind::Call { callee, args, .. })
                if callee == "rr_index_vec_floor" && args.len() == 1 =>
            {
                self.value_is_one_based_full_range_alias(args[0], end_expr, values, params, seen)
            }
            Some(ValueKind::Load { var }) => {
                self.resolve_temp_bound_value_id(var).is_some_and(|bound| {
                    self.value_is_one_based_full_range_alias(bound, end_expr, values, params, seen)
                })
            }
            _ => false,
        };
        seen.remove(&val_id);
        ok
    }

    fn resolve_index1d_expr(
        &self,
        base: usize,
        idx: usize,
        is_safe: bool,
        is_na_safe: bool,
        values: &[Value],
        params: &[String],
    ) -> String {
        index_emit::resolve_index1d_expr(self, base, idx, is_safe, is_na_safe, values, params)
    }

    fn resolve_index2d_expr(
        &self,
        base: usize,
        r: usize,
        c: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        index_emit::resolve_index2d_expr(self, base, r, c, values, params)
    }

    fn resolve_index3d_expr(
        &self,
        base: usize,
        i: usize,
        j: usize,
        k: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        index_emit::resolve_index3d_expr(self, base, i, j, k, values, params)
    }

    fn build_named_arg_list(
        &self,
        args: &[usize],
        names: &[Option<String>],
        values: &[Value],
        params: &[String],
    ) -> String {
        render_emit::build_named_arg_list(self, args, names, values, params)
    }

    fn build_plain_arg_list(&self, args: &[usize], values: &[Value], params: &[String]) -> String {
        render_emit::build_plain_arg_list(self, args, values, params)
    }

    fn resolve_preferred_plain_symbol_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        if let Some(bound) = self.resolve_bound_value(val_id)
            && Self::is_plain_symbol_expr(bound.as_str())
            && !bound.starts_with('.')
        {
            return bound;
        }
        if let Some(alias) = self.resolve_live_same_kind_scalar_alias(val_id, values) {
            return alias;
        }
        self.resolve_val(val_id, values, params, false)
    }

    fn resolve_preferred_scalar_call_arg_expr(
        &self,
        val_id: usize,
        values: &[Value],
        params: &[String],
    ) -> String {
        self.resolve_preferred_plain_symbol_expr(val_id, values, params)
    }

    fn intrinsic_helper(op: IntrinsicOp) -> &'static str {
        render_emit::intrinsic_helper(op)
    }

    fn binary_op_str(op: BinOp) -> &'static str {
        render_emit::binary_op_str(op)
    }

    fn unary_op_str(op: UnaryOp) -> &'static str {
        render_emit::unary_op_str(op)
    }

    fn resolve_cond(&self, cond: usize, values: &[Value], params: &[String]) -> String {
        index_emit::resolve_cond(self, cond, values, params)
    }

    fn comparison_is_scalar_non_na(&self, cond: usize, values: &[Value]) -> bool {
        index_emit::comparison_is_scalar_non_na(self, cond, values)
    }

    fn value_is_scalar_non_na(&self, value_id: usize, values: &[Value]) -> bool {
        index_emit::value_is_scalar_non_na(self, value_id, values)
    }

    fn value_is_scalar_non_na_impl(
        &self,
        value_id: usize,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        index_emit::value_is_scalar_non_na_impl(self, value_id, values, seen)
    }

    fn value_is_proven_non_zero(&self, value_id: usize, values: &[Value]) -> bool {
        index_emit::value_is_proven_non_zero(self, value_id, values)
    }

    fn value_is_proven_non_zero_impl(
        &self,
        value_id: usize,
        values: &[Value],
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        index_emit::value_is_proven_non_zero_impl(self, value_id, values, seen)
    }

    fn can_elide_identity_floor_call(
        callee: &str,
        args: &[usize],
        names: &[Option<String>],
        values: &[Value],
    ) -> bool {
        index_emit::can_elide_identity_floor_call(callee, args, names, values)
    }

    fn floor_index_read_components(
        callee: &str,
        args: &[usize],
        names: &[Option<String>],
        values: &[Value],
    ) -> Option<(usize, usize)> {
        index_emit::floor_index_read_components(callee, args, names, values)
    }

    fn can_elide_index_wrapper(idx: usize, values: &[Value]) -> bool {
        index_emit::can_elide_index_wrapper(idx, values)
    }

    fn emit_lit(&self, lit: &Lit) -> String {
        render_emit::emit_lit(self, lit)
    }

    fn emit_lit_with_value(&self, lit: &Lit, value: &Value) -> String {
        render_emit::emit_lit_with_value(self, lit, value)
    }

    fn emit_float_lit(&self, value: f64) -> String {
        render_emit::emit_float_lit(self, value)
    }

    pub(crate) fn write(&mut self, s: &str) {
        self.output.push_str(s);
    }

    pub(crate) fn write_indent(&mut self) {
        for _ in 0..self.indent {
            self.output.push_str("  ");
        }
    }

    pub(crate) fn newline(&mut self) {
        self.output.push('\n');
        self.current_line += 1;
    }

    pub(crate) fn write_stmt(&mut self, s: &str) {
        self.write_indent();
        self.write(s);
        self.newline();
    }

    fn rewrite_safe_scalar_loop_index_helpers(output: &mut String) {
        rewrite_emit::rewrite_safe_scalar_loop_index_helpers(output)
    }

    fn rewrite_branch_local_identical_alloc_rebinds(output: &mut String) {
        rewrite_emit::rewrite_branch_local_identical_alloc_rebinds(output)
    }

    fn hoist_branch_local_pure_scalar_assigns_used_after_branch(output: &mut String) {
        rewrite_emit::hoist_branch_local_pure_scalar_assigns_used_after_branch(output)
    }

    fn rewrite_single_use_scalar_index_aliases(output: &mut String) {
        rewrite_emit::rewrite_single_use_scalar_index_aliases(output)
    }

    fn rewrite_immediate_and_guard_named_scalar_exprs(output: &mut String) {
        rewrite_emit::rewrite_immediate_and_guard_named_scalar_exprs(output)
    }

    fn rewrite_two_use_named_scalar_exprs(output: &mut String) {
        rewrite_emit::rewrite_two_use_named_scalar_exprs(output)
    }

    fn rewrite_small_multiuse_scalar_index_aliases(output: &mut String) {
        rewrite_emit::rewrite_small_multiuse_scalar_index_aliases(output)
    }

    fn rewrite_one_or_two_use_named_scalar_index_reads_in_straight_line_region(
        output: &mut String,
    ) {
        rewrite_emit::rewrite_one_or_two_use_named_scalar_index_reads_in_straight_line_region(
            output,
        )
    }

    fn rewrite_named_scalar_pure_call_aliases(output: &mut String) {
        rewrite_emit::rewrite_named_scalar_pure_call_aliases(output)
    }

    fn rewrite_loop_index_alias_ii(output: &mut String) {
        rewrite_emit::rewrite_loop_index_alias_ii(output)
    }

    fn strip_dead_zero_seed_ii(output: &mut String) {
        rewrite_emit::strip_dead_zero_seed_ii(output)
    }

    fn rewrite_slice_bound_aliases(output: &mut String) {
        rewrite_emit::rewrite_slice_bound_aliases(output)
    }

    fn rewrite_particle_idx_alias(output: &mut String) {
        rewrite_emit::rewrite_particle_idx_alias(output)
    }

    fn rewrite_adjacent_duplicate_symbol_assignments(output: &mut String) {
        rewrite_emit::rewrite_adjacent_duplicate_symbol_assignments(output)
    }

    fn rewrite_duplicate_pure_call_assignments(
        output: &mut String,
        pure_user_calls: &FxHashSet<String>,
    ) {
        rewrite_emit::rewrite_duplicate_pure_call_assignments(output, pure_user_calls)
    }

    fn strip_noop_self_assignments(output: &mut String) {
        rewrite_emit::strip_noop_self_assignments(output)
    }

    fn rewrite_temp_uses_after_named_copy(output: &mut String) {
        rewrite_emit::rewrite_temp_uses_after_named_copy(output)
    }

    fn strip_noop_temp_copy_roundtrips(output: &mut String) {
        rewrite_emit::strip_noop_temp_copy_roundtrips(output)
    }

    fn strip_dead_simple_scalar_assigns(output: &mut String) {
        rewrite_emit::strip_dead_simple_scalar_assigns(output)
    }

    fn strip_shadowed_simple_scalar_seed_assigns(output: &mut String) {
        rewrite_emit::strip_shadowed_simple_scalar_seed_assigns(output)
    }

    fn strip_dead_seq_len_locals(output: &mut String) {
        rewrite_emit::strip_dead_seq_len_locals(output)
    }

    fn strip_redundant_branch_local_vec_fill_rebinds(output: &mut String) {
        rewrite_emit::strip_redundant_branch_local_vec_fill_rebinds(output)
    }

    fn strip_unused_raw_arg_aliases(output: &mut String) {
        rewrite_emit::strip_unused_raw_arg_aliases(output)
    }

    fn rewrite_readonly_raw_arg_aliases(output: &mut String) {
        rewrite_emit::rewrite_readonly_raw_arg_aliases(output)
    }

    fn strip_empty_else_blocks(output: &mut String) {
        rewrite_emit::strip_empty_else_blocks(output)
    }

    fn collapse_nested_else_if_blocks(output: &mut String) {
        rewrite_emit::collapse_nested_else_if_blocks(output)
    }

    fn rewrite_guard_scalar_literals(output: &mut String) {
        rewrite_emit::rewrite_guard_scalar_literals(output)
    }

    fn rewrite_loop_guard_scalar_literals(output: &mut String) {
        rewrite_emit::rewrite_loop_guard_scalar_literals(output)
    }

    fn rewrite_single_assignment_loop_seed_literals(output: &mut String) {
        rewrite_emit::rewrite_single_assignment_loop_seed_literals(output)
    }

    fn rewrite_sym210_loop_seed(output: &mut String) {
        rewrite_emit::rewrite_sym210_loop_seed(output)
    }

    fn rewrite_seq_len_full_overwrite_inits(output: &mut String) {
        rewrite_emit::rewrite_seq_len_full_overwrite_inits(output)
    }

    fn restore_missing_repeat_loop_counter_updates(output: &mut String) {
        rewrite_emit::restore_missing_repeat_loop_counter_updates(output)
    }

    fn rewrite_hoisted_loop_counter_aliases(output: &mut String) {
        rewrite_emit::rewrite_hoisted_loop_counter_aliases(output)
    }

    fn repair_missing_cse_range_aliases(output: &mut String) {
        rewrite_emit::repair_missing_cse_range_aliases(output)
    }

    fn restore_constant_one_guard_repeat_loop_counters(output: &mut String) {
        rewrite_emit::restore_constant_one_guard_repeat_loop_counters(output)
    }

    fn rewrite_literal_named_list_calls(output: &mut String) {
        rewrite_emit::rewrite_literal_named_list_calls(output)
    }

    fn rewrite_literal_field_get_calls(output: &mut String) {
        rewrite_emit::rewrite_literal_field_get_calls(output)
    }

    fn strip_unreachable_sym_helpers(output: &mut String) {
        rewrite_emit::strip_unreachable_sym_helpers(output)
    }

    fn strip_redundant_tail_assign_slice_return(output: &mut String) {
        rewrite_emit::strip_redundant_tail_assign_slice_return(output)
    }

    fn strip_single_blank_spacers(output: &mut String) {
        rewrite_emit::strip_single_blank_spacers(output)
    }

    fn compact_blank_lines(output: &mut String) {
        rewrite_emit::compact_blank_lines(output)
    }

    fn strip_terminal_repeat_nexts(output: &mut String) {
        rewrite_emit::strip_terminal_repeat_nexts(output)
    }

    fn strip_orphan_rr_cse_pruned_markers(output: &mut String) {
        rewrite_emit::strip_orphan_rr_cse_pruned_markers(output)
    }

    fn infer_generated_poly_loop_step(
        lines: &[String],
        body_start: usize,
        body_end: usize,
        var: &str,
    ) -> i64 {
        rewrite_emit::infer_generated_poly_loop_step(lines, body_start, body_end, var)
    }

    fn first_generated_poly_loop_var_in_line(line: &str) -> Option<String> {
        rewrite_emit::first_generated_poly_loop_var_in_line(line)
    }

    fn restore_missing_generated_poly_loop_steps(output: &mut String) {
        rewrite_emit::restore_missing_generated_poly_loop_steps(output)
    }

    fn emit_mark(&mut self, span: Span, label: Option<&str>) {
        render_emit::emit_mark(self, span, label)
    }
}

#[cfg(test)]
#[path = "emit/tests.rs"]
mod tests;
