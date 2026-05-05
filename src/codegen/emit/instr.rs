use super::*;
use crate::mir::analyze::effects::call_is_pure;

pub(crate) struct AssignEmit<'a> {
    pub(crate) dst: &'a str,
    pub(crate) src: usize,
    pub(crate) span: Span,
    pub(crate) values: &'a [Value],
    pub(crate) params: &'a [String],
}

pub(crate) struct GeneralAssignProbe {
    pub(crate) preserve_loop_seed: bool,
    pub(crate) stale_origin: Option<String>,
    pub(crate) bound: Option<String>,
    pub(crate) mutated_whole_range_copy: Option<String>,
    pub(crate) allow_last_assigned_skip: bool,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct AssignTracking {
    pub(crate) remember_full_end: bool,
    pub(crate) track_whole_range: bool,
}

pub(crate) const NORMAL_ASSIGN_TRACKING: AssignTracking = AssignTracking {
    remember_full_end: false,
    track_whole_range: false,
};

pub(crate) const WHOLE_RANGE_ASSIGN_TRACKING: AssignTracking = AssignTracking {
    remember_full_end: true,
    track_whole_range: true,
};

pub(crate) struct StoreIndex1DArgs {
    pub(crate) base: usize,
    pub(crate) idx: usize,
    pub(crate) val: usize,
    pub(crate) is_vector: bool,
    pub(crate) is_safe: bool,
    pub(crate) is_na_safe: bool,
    pub(crate) span: Span,
}

pub(crate) struct StoreIndex2DArgs {
    pub(crate) base: usize,
    pub(crate) row: usize,
    pub(crate) col: usize,
    pub(crate) val: usize,
    pub(crate) span: Span,
}

pub(crate) struct StoreIndex3DArgs {
    pub(crate) base: usize,
    pub(crate) dim1: usize,
    pub(crate) dim2: usize,
    pub(crate) dim3: usize,
    pub(crate) val: usize,
    pub(crate) span: Span,
}

pub(crate) struct StoreIndex1DScalarEmit<'a> {
    pub(crate) args: &'a StoreIndex1DArgs,
    pub(crate) base_val: &'a str,
    pub(crate) idx_val: &'a str,
    pub(crate) src_val: &'a str,
    pub(crate) mutated_base_name: Option<String>,
    pub(crate) values: &'a [Value],
    pub(crate) params: &'a [String],
}

impl<'a> AssignEmit<'a> {
    pub(crate) fn from_instr(instr: &'a Instr, values: &'a [Value], params: &'a [String]) -> Self {
        let Instr::Assign { dst, src, span } = instr else {
            unreachable!("AssignEmit requires Instr::Assign");
        };
        Self {
            dst,
            src: *src,
            span: *span,
            values,
            params,
        }
    }
}

impl StoreIndex1DArgs {
    pub(crate) fn from_instr(instr: &Instr) -> Self {
        let Instr::StoreIndex1D {
            base,
            idx,
            val,
            is_vector,
            is_safe,
            is_na_safe,
            span,
        } = instr
        else {
            unreachable!("StoreIndex1DArgs requires Instr::StoreIndex1D");
        };
        Self {
            base: *base,
            idx: *idx,
            val: *val,
            is_vector: *is_vector,
            is_safe: *is_safe,
            is_na_safe: *is_na_safe,
            span: *span,
        }
    }
}

impl StoreIndex2DArgs {
    pub(crate) fn from_instr(instr: &Instr) -> Self {
        let Instr::StoreIndex2D {
            base,
            r,
            c,
            val,
            span,
        } = instr
        else {
            unreachable!("StoreIndex2DArgs requires Instr::StoreIndex2D");
        };
        Self {
            base: *base,
            row: *r,
            col: *c,
            val: *val,
            span: *span,
        }
    }
}

impl StoreIndex3DArgs {
    pub(crate) fn from_instr(instr: &Instr) -> Self {
        let Instr::StoreIndex3D {
            base,
            i,
            j,
            k,
            val,
            span,
        } = instr
        else {
            unreachable!("StoreIndex3DArgs requires Instr::StoreIndex3D");
        };
        Self {
            base: *base,
            dim1: *i,
            dim2: *j,
            dim3: *k,
            val: *val,
            span: *span,
        }
    }
}

impl RBackend {
    pub(crate) fn clear_unsafe_r_emit_assumptions(&mut self) {
        self.value_tracker.value_bindings.clear();
        self.value_tracker.var_value_bindings.clear();
        self.value_tracker.last_assigned_value_ids.clear();
        self.emit_scratch.clear();
        self.loop_analysis.recent_whole_assign_bases.clear();
        self.invalidate_emitted_cse_temps();
    }

    pub(crate) fn emit_unsafe_r_block(&mut self, code: &str, read_only: bool, span: Span) {
        self.emit_mark(span, Some("unsafe-r"));
        self.record_span(span);
        let label = if read_only {
            "rr-unsafe-r-read"
        } else {
            "rr-unsafe-r"
        };
        self.write_stmt(&format!("# {label}-begin"));
        for line in code
            .trim_matches('\n')
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .lines()
        {
            if line.trim().is_empty() {
                self.newline();
            } else {
                self.write_indent();
                self.write(line.trim_end());
                self.newline();
            }
        }
        self.write_stmt(&format!("# {label}-end"));
        self.clear_unsafe_r_emit_assumptions();
    }

    pub(crate) fn can_reuse_live_expr_alias(&self, val_id: usize, values: &[Value]) -> bool {
        match values.get(val_id).map(|value| &value.kind) {
            Some(ValueKind::Binary { .. })
            | Some(ValueKind::Unary { .. })
            | Some(ValueKind::RecordLit { .. })
            | Some(ValueKind::FieldGet { .. })
            | Some(ValueKind::Len { .. })
            | Some(ValueKind::Indices { .. })
            | Some(ValueKind::Range { .. }) => true,
            Some(ValueKind::Call { callee, .. }) => {
                call_is_pure(callee) || self.analysis.known_pure_user_calls.contains(callee)
            }
            _ => false,
        }
    }

    pub(crate) fn resolve_preferred_live_operand(
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
        if let Some(origin_var) = self.resolve_live_const_origin_var(val_id, values) {
            return origin_var;
        }
        if let Some(alias) = self.resolve_live_same_kind_scalar_alias(val_id, values) {
            return alias;
        }
        if self.can_reuse_live_expr_alias(val_id, values) {
            let preferred = self.resolve_preferred_live_expr_alias(val_id, values, params);
            if Self::is_plain_symbol_expr(preferred.as_str()) {
                return preferred;
            }
        }
        self.resolve_val(val_id, values, params, false)
    }
}

impl RBackend {
    pub(crate) fn emit_instr(
        &mut self,
        instr: &Instr,
        values: &[Value],
        params: &[String],
    ) -> Result<(), crate::error::RRException> {
        match instr {
            Instr::UnsafeRBlock {
                code,
                read_only,
                span,
            } => {
                self.emit_unsafe_r_block(code, *read_only, *span);
            }
            Instr::Assign { .. } => {
                self.emit_assign_instr(AssignEmit::from_instr(instr, values, params));
            }
            Instr::Eval { val, span } => {
                self.emit_eval_instr(*val, *span, values, params);
            }
            Instr::StoreIndex1D { .. } => {
                self.emit_store_index1d_instr(StoreIndex1DArgs::from_instr(instr), values, params);
            }
            Instr::StoreIndex2D { .. } => {
                self.emit_store_index2d_instr(StoreIndex2DArgs::from_instr(instr), values, params);
            }
            Instr::StoreIndex3D { .. } => {
                self.emit_store_index3d_instr(StoreIndex3DArgs::from_instr(instr), values, params);
            }
        }
        Ok(())
    }

    pub(crate) fn emit_assign_instr(&mut self, ctx: AssignEmit<'_>) {
        self.prepare_assign_emit(&ctx);

        if self.try_emit_field_set_assign(&ctx)
            || self.try_emit_field_get_self_assign(&ctx)
            || self.try_emit_generated_poly_loop_assign(&ctx)
            || self.try_emit_slice_assign(&ctx)
            || self.try_emit_whole_range_assign(&ctx)
        {
            return;
        }

        let probe = self.collect_general_assign_probe(&ctx);
        if self.should_skip_general_assign(&ctx, &probe) {
            self.invalidate_emitted_cse_temps();
            return;
        }

        self.emit_general_assign(&ctx, &probe);
    }

    pub(crate) fn prepare_assign_emit(&mut self, ctx: &AssignEmit<'_>) {
        let label = format!("assign {}", ctx.dst);
        self.emit_mark(ctx.span, Some(label.as_str()));
        self.emit_scratch.emitted_temp_names.clear();
    }

    pub(crate) fn try_emit_field_set_assign(&mut self, ctx: &AssignEmit<'_>) -> bool {
        let Some(ValueKind::FieldSet { base, field, value }) =
            ctx.values.get(ctx.src).map(|value| &value.kind)
        else {
            return false;
        };
        let Some(base_var) = self.resolve_named_mutable_base_var(*base, ctx.values, ctx.params)
        else {
            return false;
        };
        if base_var != ctx.dst {
            return false;
        }

        let rhs = self.resolve_plain_reusable_value(*value, ctx.values, ctx.params, true);
        self.record_span(ctx.span);
        self.write_stmt(&format!(r#"{}[["{}"]] <- {}"#, ctx.dst, field, rhs));
        self.note_var_write(ctx.dst);
        self.remember_completed_assign(ctx.dst, ctx.src);
        self.invalidate_emitted_cse_temps();
        true
    }

    pub(crate) fn try_emit_field_get_self_assign(&mut self, ctx: &AssignEmit<'_>) -> bool {
        let Some(ValueKind::FieldGet { base, field }) =
            ctx.values.get(ctx.src).map(|value| &value.kind)
        else {
            return false;
        };
        if ctx.values[ctx.src].origin_var.as_deref() != Some(ctx.dst)
            || self.resolve_bound_value_id(ctx.dst) == Some(ctx.src)
        {
            return false;
        }

        if self.skip_if_current_value_expanded_equivalent(ctx) {
            return true;
        }
        if self.try_emit_field_get_alias_assign(ctx) {
            return true;
        }

        let base_expr = self.resolve_plain_reusable_value(*base, ctx.values, ctx.params, false);
        let rendered = format!(r#"{base_expr}[["{field}"]]"#);
        self.emit_tracked_assign(ctx, &rendered, NORMAL_ASSIGN_TRACKING);
        true
    }

    pub(crate) fn try_emit_field_get_alias_assign(&mut self, ctx: &AssignEmit<'_>) -> bool {
        let preferred_alias = self
            .resolve_live_same_kind_scalar_alias(ctx.src, ctx.values)
            .unwrap_or_else(|| {
                self.resolve_preferred_live_expr_alias(ctx.src, ctx.values, ctx.params)
            });
        if !Self::is_plain_symbol_expr(preferred_alias.as_str()) || preferred_alias == ctx.dst {
            return false;
        }

        self.emit_tracked_assign(ctx, &preferred_alias, NORMAL_ASSIGN_TRACKING);
        true
    }

    pub(crate) fn skip_if_current_value_expanded_equivalent(
        &mut self,
        ctx: &AssignEmit<'_>,
    ) -> bool {
        let Some(current_val_id) = self.resolve_bound_value_id(ctx.dst) else {
            return false;
        };
        if current_val_id == ctx.src {
            return false;
        }
        if ctx.values[current_val_id].kind == ctx.values[ctx.src].kind {
            self.invalidate_emitted_cse_temps();
            return true;
        }

        let expanded_src_expr = self.resolve_expanded_scalar_expr_for_equivalence(
            ctx.src,
            ctx.values,
            ctx.params,
            &mut FxHashSet::default(),
        );
        let expanded_current_expr = self.resolve_expanded_scalar_expr_for_equivalence(
            current_val_id,
            ctx.values,
            ctx.params,
            &mut FxHashSet::default(),
        );
        if expanded_src_expr != expanded_current_expr {
            return false;
        }

        self.invalidate_emitted_cse_temps();
        true
    }

    pub(crate) fn try_emit_generated_poly_loop_assign(&mut self, ctx: &AssignEmit<'_>) -> bool {
        if !is_generated_poly_loop_var_name(ctx.dst) {
            return false;
        }

        let rendered = self.resolve_raw_generated_loop_expr(
            ctx.src,
            ctx.values,
            ctx.params,
            &mut FxHashSet::default(),
        );
        if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_some() {
            eprintln!(
                "RR_DEBUG_EMIT_ASSIGN generated_loop fn={} dst={} src={} rendered={}",
                self.current_fn_name, ctx.dst, ctx.src, rendered
            );
        }
        self.emit_tracked_assign(ctx, &rendered, NORMAL_ASSIGN_TRACKING);
        true
    }

    pub(crate) fn try_emit_slice_assign(&mut self, ctx: &AssignEmit<'_>) -> bool {
        if let Some(partial_slice_stmt) = self
            .try_render_constant_safe_partial_self_assign(ctx.dst, ctx.src, ctx.values, ctx.params)
        {
            self.emit_untracked_mutation_assign(ctx, &partial_slice_stmt);
            return true;
        }
        if let Some(row_slice_stmt) =
            self.try_render_safe_idx_cube_row_slice_assign(ctx.dst, ctx.src, ctx.values, ctx.params)
        {
            self.emit_untracked_mutation_assign(ctx, &row_slice_stmt);
            return true;
        }
        false
    }

    pub(crate) fn try_emit_whole_range_assign(&mut self, ctx: &AssignEmit<'_>) -> bool {
        if let Some(whole_range_rhs) =
            self.try_resolve_whole_range_self_assign_rhs(ctx.dst, ctx.src, ctx.values, ctx.params)
        {
            if whole_range_rhs == ctx.dst {
                self.invalidate_emitted_cse_temps();
            } else {
                self.emit_tracked_assign(ctx, &whole_range_rhs, WHOLE_RANGE_ASSIGN_TRACKING);
            }
            return true;
        }
        if let Some(whole_range_rhs) =
            self.try_resolve_whole_range_call_map_rhs(ctx.dst, ctx.src, ctx.values, ctx.params)
        {
            self.emit_tracked_assign(ctx, &whole_range_rhs, WHOLE_RANGE_ASSIGN_TRACKING);
            return true;
        }
        if let Some(whole_range_rhs) =
            self.try_resolve_whole_auto_call_map_rhs(ctx.dst, ctx.src, ctx.values, ctx.params)
        {
            self.emit_tracked_assign(ctx, &whole_range_rhs, WHOLE_RANGE_ASSIGN_TRACKING);
            return true;
        }
        false
    }

    pub(crate) fn collect_general_assign_probe(
        &mut self,
        ctx: &AssignEmit<'_>,
    ) -> GeneralAssignProbe {
        let preserve_loop_seed = self.in_active_loop_mutated_context(ctx.dst);
        let same_origin_self_assign = ctx.values[ctx.src].origin_var.as_deref() == Some(ctx.dst)
            && self.resolve_bound_value_id(ctx.dst) != Some(ctx.src);
        let stale_origin = if same_origin_self_assign {
            None
        } else {
            self.resolve_stale_origin_var(ctx.src, &ctx.values[ctx.src], ctx.values)
        };
        let bound = if same_origin_self_assign {
            None
        } else {
            self.resolve_bound_value(ctx.src)
        };
        let mutated_whole_range_copy =
            self.try_resolve_mutated_whole_range_copy_alias(ctx.src, ctx.values, ctx.params);
        let allow_last_assigned_skip = !self.is_direct_assign_source(ctx);

        GeneralAssignProbe {
            preserve_loop_seed,
            stale_origin,
            bound,
            mutated_whole_range_copy,
            allow_last_assigned_skip,
        }
    }

    pub(crate) fn should_skip_general_assign(
        &mut self,
        ctx: &AssignEmit<'_>,
        probe: &GeneralAssignProbe,
    ) -> bool {
        self.should_skip_stale_self_fresh_replay(ctx, probe.preserve_loop_seed)
            || self
                .should_skip_stale_same_origin_without_live_binding(ctx, probe.preserve_loop_seed)
            || self.should_skip_last_assigned_same_kind(ctx, probe.allow_last_assigned_skip)
            || (!probe.preserve_loop_seed && self.should_skip_non_loop_duplicate_assign(ctx))
    }

    pub(crate) fn should_skip_stale_self_fresh_replay(
        &self,
        ctx: &AssignEmit<'_>,
        preserve_loop_seed: bool,
    ) -> bool {
        let should_skip = !preserve_loop_seed
            && self.is_fresh_mutable_aggregate_value(&ctx.values[ctx.src])
            && ctx.values[ctx.src].origin_var.as_deref() == Some(ctx.dst)
            && self.resolve_bound_value_id(ctx.dst) != Some(ctx.src)
            && (self.value_tracker.value_bindings.get(&ctx.src).is_some_and(
                |(bound_var, version)| {
                    bound_var == ctx.dst && self.current_var_version(ctx.dst) != *version
                },
            ) || self.resolve_bound_value_id(ctx.dst).is_some_and(|current| {
                !self.is_fresh_mutable_aggregate_value(&ctx.values[current])
            }));
        if should_skip {
            self.log_stale_self_fresh_replay_skip(ctx);
        }
        should_skip
    }

    pub(crate) fn log_stale_self_fresh_replay_skip(&self, ctx: &AssignEmit<'_>) {
        if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_none() {
            return;
        }

        let binding = self.value_tracker.value_bindings.get(&ctx.src).cloned();
        eprintln!(
            "RR_DEBUG_EMIT_ASSIGN skip=stale_self_fresh_replay fn={} dst={} src={} kind={:?} current_bound={:?} origin={:?} binding={:?} current_version={}",
            self.current_fn_name,
            ctx.dst,
            ctx.src,
            ctx.values[ctx.src].kind,
            self.resolve_bound_value_id(ctx.dst),
            ctx.values[ctx.src].origin_var,
            binding,
            self.current_var_version(ctx.dst),
        );
    }

    pub(crate) fn should_skip_stale_same_origin_without_live_binding(
        &self,
        ctx: &AssignEmit<'_>,
        preserve_loop_seed: bool,
    ) -> bool {
        !preserve_loop_seed
            && self.is_fresh_mutable_aggregate_value(&ctx.values[ctx.src])
            && ctx.values[ctx.src].origin_var.as_deref() == Some(ctx.dst)
            && self.resolve_bound_value_id(ctx.dst).is_none()
            && self.current_var_version(ctx.dst) > 0
    }

    pub(crate) fn should_skip_last_assigned_same_kind(
        &self,
        ctx: &AssignEmit<'_>,
        allow_last_assigned_skip: bool,
    ) -> bool {
        let should_skip = allow_last_assigned_skip
            && self
                .value_tracker
                .last_assigned_value_ids
                .get(ctx.dst)
                .is_some_and(|prev_src| ctx.values[*prev_src].kind == ctx.values[ctx.src].kind);
        if should_skip {
            self.log_last_assigned_same_kind_skip(ctx);
        }
        should_skip
    }

    pub(crate) fn log_last_assigned_same_kind_skip(&self, ctx: &AssignEmit<'_>) {
        if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_none() {
            return;
        }

        if let Some(prev_src) = self.value_tracker.last_assigned_value_ids.get(ctx.dst) {
            eprintln!(
                "RR_DEBUG_EMIT_ASSIGN skip=last_assigned_same_kind fn={} dst={} src={} prev_src={} kind={:?}",
                self.current_fn_name, ctx.dst, ctx.src, prev_src, ctx.values[ctx.src].kind,
            );
        }
    }

    pub(crate) fn should_skip_non_loop_duplicate_assign(&mut self, ctx: &AssignEmit<'_>) -> bool {
        self.should_skip_fresh_allocation_replay(ctx)
            || self.resolve_bound_value_id(ctx.dst) == Some(ctx.src)
            || self.should_skip_equivalent_current_assignment(ctx)
    }

    pub(crate) fn should_skip_fresh_allocation_replay(&self, ctx: &AssignEmit<'_>) -> bool {
        matches!(
            ctx.values[ctx.src].kind,
            ValueKind::Call { ref callee, .. } if self.call_is_known_fresh_allocation(callee)
        ) && ctx.values[ctx.src].origin_var.as_deref() == Some(ctx.dst)
            && self
                .resolve_stale_origin_var(ctx.src, &ctx.values[ctx.src], ctx.values)
                .as_deref()
                == Some(ctx.dst)
            && self
                .resolve_bound_value_id(ctx.dst)
                .is_some_and(|current_val_id| {
                    ctx.values[current_val_id].kind == ctx.values[ctx.src].kind
                })
    }

    pub(crate) fn should_skip_equivalent_current_assignment(
        &mut self,
        ctx: &AssignEmit<'_>,
    ) -> bool {
        let Some(current_val_id) = self.resolve_bound_value_id(ctx.dst) else {
            return false;
        };
        if current_val_id == ctx.src {
            return false;
        }
        if ctx.values[current_val_id].kind == ctx.values[ctx.src].kind {
            return true;
        }

        let src_expr = self.resolve_val(ctx.src, ctx.values, ctx.params, true);
        let current_expr = self.resolve_val(current_val_id, ctx.values, ctx.params, true);
        if src_expr == current_expr {
            return true;
        }

        let expanded_src_expr = self.resolve_expanded_scalar_expr_for_equivalence(
            ctx.src,
            ctx.values,
            ctx.params,
            &mut FxHashSet::default(),
        );
        let expanded_current_expr = self.resolve_expanded_scalar_expr_for_equivalence(
            current_val_id,
            ctx.values,
            ctx.params,
            &mut FxHashSet::default(),
        );
        expanded_src_expr == expanded_current_expr
    }

    pub(crate) fn emit_general_assign(&mut self, ctx: &AssignEmit<'_>, probe: &GeneralAssignProbe) {
        let rendered = self.resolve_general_assign_rhs(ctx, probe);
        let rendered =
            self.rewrite_known_one_based_full_range_alias_reads(&rendered, ctx.values, ctx.params);
        self.log_general_assign_emit(ctx, &rendered);

        if rendered != ctx.dst {
            self.record_span(ctx.span);
            self.write_stmt(&format!("{} <- {}", ctx.dst, rendered));
            self.note_var_write(ctx.dst);
            self.loop_analysis
                .recent_whole_assign_bases
                .insert(ctx.dst.to_string());
            if !matches!(&ctx.values[ctx.src].kind, ValueKind::Load { var } if var != ctx.dst) {
                self.bind_value_to_var(ctx.src, ctx.dst);
            }
            if !matches!(&ctx.values[ctx.src].kind, ValueKind::Load { .. }) {
                self.bind_var_to_value(ctx.dst, ctx.src);
            }
            self.remember_known_full_end_expr(ctx.dst, ctx.src, ctx.values, ctx.params);
            self.remember_last_assigned_value(ctx.dst, ctx.src);
        }
        self.invalidate_emitted_cse_temps();
    }

    pub(crate) fn resolve_general_assign_rhs(
        &mut self,
        ctx: &AssignEmit<'_>,
        probe: &GeneralAssignProbe,
    ) -> String {
        if let Some(alias_var) = probe.mutated_whole_range_copy.clone() {
            return alias_var;
        }
        if !self.is_direct_assign_source(ctx)
            && let Some(origin_var) = probe.stale_origin.clone()
        {
            return origin_var;
        }
        if !matches!(ctx.values[ctx.src].kind, ValueKind::Const(_))
            && let Some(bound) = probe.bound.clone()
        {
            return bound;
        }
        if self.can_reuse_live_expr_alias(ctx.src, ctx.values) {
            let preferred = self
                .resolve_live_same_kind_scalar_alias(ctx.src, ctx.values)
                .unwrap_or_else(|| {
                    self.resolve_preferred_live_expr_alias(ctx.src, ctx.values, ctx.params)
                });
            if Self::is_plain_symbol_expr(preferred.as_str()) && preferred != ctx.dst {
                return preferred;
            }
        }

        self.resolve_materialized_assign_rhs(ctx)
    }

    pub(crate) fn resolve_materialized_assign_rhs(&mut self, ctx: &AssignEmit<'_>) -> String {
        let preview = self.resolve_val(ctx.src, ctx.values, ctx.params, true);
        if preview == ctx.dst {
            return preview;
        }

        self.emit_common_subexpr_temps(ctx.src, ctx.values, ctx.params);
        self.resolve_val(ctx.src, ctx.values, ctx.params, true)
    }

    pub(crate) fn log_general_assign_emit(&self, ctx: &AssignEmit<'_>, rendered: &str) {
        if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_none() {
            return;
        }

        eprintln!(
            "RR_DEBUG_EMIT_ASSIGN fn={} dst={} src={} kind={:?} rendered={} skip={}",
            self.current_fn_name,
            ctx.dst,
            ctx.src,
            ctx.values[ctx.src].kind,
            rendered,
            rendered == ctx.dst
        );
    }

    pub(crate) fn is_direct_assign_source(&self, ctx: &AssignEmit<'_>) -> bool {
        matches!(
            ctx.values[ctx.src].kind,
            ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. }
        )
    }

    pub(crate) fn resolve_plain_reusable_value(
        &mut self,
        val_id: usize,
        values: &[Value],
        params: &[String],
        allow_hidden_symbol: bool,
    ) -> String {
        if self.can_reuse_live_expr_alias(val_id, values) {
            let preferred = self
                .resolve_live_same_kind_scalar_alias(val_id, values)
                .unwrap_or_else(|| self.resolve_preferred_live_expr_alias(val_id, values, params));
            if Self::is_plain_symbol_expr(preferred.as_str())
                && (allow_hidden_symbol || !preferred.starts_with('.'))
            {
                return preferred;
            }
        }

        self.resolve_val(val_id, values, params, false)
    }

    pub(crate) fn emit_tracked_assign(
        &mut self,
        ctx: &AssignEmit<'_>,
        rhs: &str,
        tracking: AssignTracking,
    ) {
        self.record_span(ctx.span);
        self.write_stmt(&format!("{} <- {}", ctx.dst, rhs));
        self.note_var_write(ctx.dst);
        if tracking.track_whole_range {
            self.loop_analysis
                .recent_whole_assign_bases
                .insert(ctx.dst.to_string());
        }
        self.remember_completed_assign(ctx.dst, ctx.src);
        if tracking.remember_full_end {
            self.remember_known_full_end_expr(ctx.dst, ctx.src, ctx.values, ctx.params);
        }
        self.invalidate_emitted_cse_temps();
    }

    pub(crate) fn emit_untracked_mutation_assign(&mut self, ctx: &AssignEmit<'_>, stmt: &str) {
        self.record_span(ctx.span);
        self.write_stmt(stmt);
        self.note_var_write(ctx.dst);
        self.invalidate_var_binding(ctx.dst);
        self.value_tracker.last_assigned_value_ids.remove(ctx.dst);
        self.invalidate_emitted_cse_temps();
    }

    pub(crate) fn remember_completed_assign(&mut self, dst: &str, src: usize) {
        self.bind_value_to_var(src, dst);
        self.bind_var_to_value(dst, src);
        self.remember_last_assigned_value(dst, src);
    }

    pub(crate) fn remember_last_assigned_value(&mut self, dst: &str, src: usize) {
        self.log_last_assigned_value_change(dst);
        self.value_tracker
            .last_assigned_value_ids
            .insert(dst.to_string(), src);
    }

    pub(crate) fn emit_eval_instr(
        &mut self,
        val: usize,
        span: Span,
        values: &[Value],
        params: &[String],
    ) {
        self.emit_mark(span, Some("eval"));
        self.record_span(span);
        let rendered = self.resolve_preferred_live_operand(val, values, params);
        self.write_stmt(&rendered);
    }

    pub(crate) fn emit_store_index1d_instr(
        &mut self,
        args: StoreIndex1DArgs,
        values: &[Value],
        params: &[String],
    ) {
        self.emit_mark(args.span, Some("store"));
        self.record_span(args.span);
        let mutated_base_name = self.resolve_named_mutable_base_var(args.base, values, params);
        let base_val = self.resolve_mutable_base(args.base, values, params);
        let idx_val = self.resolve_preferred_live_operand(args.idx, values, params);
        let src_val = self.resolve_preferred_live_operand(args.val, values, params);

        if args.is_vector {
            self.emit_vector_store_index1d(
                &args,
                &base_val,
                &src_val,
                mutated_base_name,
                values,
                params,
            );
        } else {
            self.emit_scalar_store_index1d(StoreIndex1DScalarEmit {
                args: &args,
                base_val: &base_val,
                idx_val: &idx_val,
                src_val: &src_val,
                mutated_base_name,
                values,
                params,
            });
        }
    }

    pub(crate) fn emit_vector_store_index1d(
        &mut self,
        args: &StoreIndex1DArgs,
        base_val: &str,
        src_val: &str,
        mutated_base_name: Option<String>,
        values: &[Value],
        params: &[String],
    ) {
        self.write_stmt(&format!("{base_val} <- {src_val}"));
        if let Some(base_name) = mutated_base_name.as_deref() {
            self.invalidate_alias_bindings_depending_on_var(base_name, values);
            self.note_var_write(base_name);
            self.bind_value_to_var(args.val, base_name);
            if !matches!(&values[args.val].kind, ValueKind::Load { .. }) {
                self.bind_var_to_value(base_name, args.val);
            }
            self.loop_analysis
                .recent_whole_assign_bases
                .insert(base_name.to_string());
            self.remember_known_full_end_expr(base_name, args.val, values, params);
        } else {
            self.bump_base_version_if_named(args.base, values);
        }
    }

    pub(crate) fn emit_scalar_store_index1d(&mut self, ctx: StoreIndex1DScalarEmit<'_>) {
        let idx_expr = if (ctx.args.is_safe && ctx.args.is_na_safe)
            || self.can_elide_index_expr(ctx.args.idx, ctx.values, ctx.params)
        {
            ctx.idx_val.to_string()
        } else {
            format!("rr_index1_write({}, \"index\")", ctx.idx_val)
        };
        self.write_stmt(&format!(
            "{}[{}] <- {}",
            ctx.base_val, idx_expr, ctx.src_val
        ));
        self.record_index_store_mutation(
            ctx.args.base,
            ctx.mutated_base_name.as_deref(),
            ctx.values,
        );
    }

    pub(crate) fn emit_store_index2d_instr(
        &mut self,
        args: StoreIndex2DArgs,
        values: &[Value],
        params: &[String],
    ) {
        self.emit_mark(args.span, Some("store2d"));
        self.record_span(args.span);
        let mutated_base_name = self.resolve_named_mutable_base_var(args.base, values, params);
        let base_val = self.resolve_mutable_base(args.base, values, params);
        let row_val = self.resolve_preferred_live_operand(args.row, values, params);
        let col_val = self.resolve_preferred_live_operand(args.col, values, params);
        let src_val = self.resolve_preferred_live_operand(args.val, values, params);
        let row_idx = self.index_write_expr(args.row, row_val, "row", values, params);
        let col_idx = self.index_write_expr(args.col, col_val, "col", values, params);

        self.write_stmt(&format!("{base_val}[{row_idx}, {col_idx}] <- {src_val}"));
        self.record_index_store_mutation(args.base, mutated_base_name.as_deref(), values);
    }

    pub(crate) fn emit_store_index3d_instr(
        &mut self,
        args: StoreIndex3DArgs,
        values: &[Value],
        params: &[String],
    ) {
        self.emit_mark(args.span, Some("store3d"));
        self.record_span(args.span);
        let mutated_base_name = self.resolve_named_mutable_base_var(args.base, values, params);
        let base_val = self.resolve_mutable_base(args.base, values, params);
        let dim1_val = self.resolve_preferred_live_operand(args.dim1, values, params);
        let dim2_val = self.resolve_preferred_live_operand(args.dim2, values, params);
        let dim3_val = self.resolve_preferred_live_operand(args.dim3, values, params);
        let src_val = self.resolve_preferred_live_operand(args.val, values, params);
        let dim1_idx = self.index_write_expr(args.dim1, dim1_val, "dim1", values, params);
        let dim2_idx = self.index_write_expr(args.dim2, dim2_val, "dim2", values, params);
        let dim3_idx = self.index_write_expr(args.dim3, dim3_val, "dim3", values, params);

        self.write_stmt(&format!(
            "{base_val}[{dim1_idx}, {dim2_idx}, {dim3_idx}] <- {src_val}"
        ));
        self.record_index_store_mutation(args.base, mutated_base_name.as_deref(), values);
    }

    pub(crate) fn index_write_expr(
        &self,
        val_id: usize,
        rendered: String,
        label: &str,
        values: &[Value],
        params: &[String],
    ) -> String {
        if self.can_elide_index_expr(val_id, values, params) {
            rendered
        } else {
            format!("rr_index1_write({rendered}, \"{label}\")")
        }
    }

    pub(crate) fn record_index_store_mutation(
        &mut self,
        base: usize,
        mutated_base_name: Option<&str>,
        values: &[Value],
    ) {
        if let Some(base_name) = mutated_base_name {
            self.invalidate_alias_bindings_depending_on_var(base_name, values);
            self.note_var_write(base_name);
        } else {
            self.bump_base_version_if_named(base, values);
        }
    }

    pub(crate) fn emit_term(
        &mut self,
        term: &Terminator,
        values: &[Value],
        params: &[String],
    ) -> Result<(), crate::error::RRException> {
        match term {
            Terminator::Goto(t) => {
                self.write_stmt(&format!("break; # goto {}", t));
            }
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => {
                let c = self.resolve_cond(*cond, values, params, &[], &[]);
                self.write_stmt(&format!("if ({}) {{ # goto {}/{}", c, then_bb, else_bb));
                self.write_stmt("}");
            }
            Terminator::Return(Some(v)) => {
                let val = self.resolve_preferred_live_operand(*v, values, params);
                self.write_stmt(&format!("return({})", val));
            }
            Terminator::Return(None) => {
                self.write_stmt("return(NULL)");
            }
            Terminator::Unreachable => {
                self.write_stmt("rr_fail(\"RR.RuntimeError\", \"ICE9001\", \"unreachable code reached\", \"control flow\")");
            }
        }
        Ok(())
    }
}
