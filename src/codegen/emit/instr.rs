use super::*;

impl RBackend {
    pub(super) fn emit_instr(
        &mut self,
        instr: &Instr,
        values: &[Value],
        params: &[String],
    ) -> Result<(), crate::error::RRException> {
        match instr {
            Instr::Assign { dst, src, span } => {
                let label = format!("assign {}", dst);
                self.emit_mark(*span, Some(label.as_str()));
                self.emit_scratch.emitted_temp_names.clear();
                if let Some(ValueKind::FieldSet { base, field, value }) =
                    values.get(*src).map(|value| &value.kind)
                    && let Some(base_var) =
                        self.resolve_named_mutable_base_var(*base, values, params)
                    && base_var == *dst
                {
                    let rhs = self.resolve_val(*value, values, params, false);
                    self.record_span(*span);
                    self.write_stmt(&format!(r#"{dst}[["{field}"]] <- {rhs}"#));
                    self.note_var_write(dst);
                    self.bind_value_to_var(*src, dst);
                    self.bind_var_to_value(dst, *src);
                    self.log_last_assigned_value_change(dst);
                    self.value_tracker
                        .last_assigned_value_ids
                        .insert(dst.clone(), *src);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if let Some(ValueKind::FieldGet { base, field }) =
                    values.get(*src).map(|value| &value.kind)
                    && values[*src].origin_var.as_deref() == Some(dst.as_str())
                    && self.resolve_bound_value_id(dst) != Some(*src)
                {
                    let base_expr = self.resolve_val(*base, values, params, false);
                    let rendered = format!(r#"{base_expr}[["{field}"]]"#);
                    self.record_span(*span);
                    self.write_stmt(&format!("{dst} <- {rendered}"));
                    self.note_var_write(dst);
                    self.bind_value_to_var(*src, dst);
                    self.bind_var_to_value(dst, *src);
                    self.log_last_assigned_value_change(dst);
                    self.value_tracker
                        .last_assigned_value_ids
                        .insert(dst.clone(), *src);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if is_generated_poly_loop_var_name(dst) {
                    let rendered = self.resolve_raw_generated_loop_expr(
                        *src,
                        values,
                        params,
                        &mut FxHashSet::default(),
                    );
                    if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_some() {
                        eprintln!(
                            "RR_DEBUG_EMIT_ASSIGN generated_loop fn={} dst={} src={} rendered={}",
                            self.current_fn_name, dst, src, rendered
                        );
                    }
                    self.record_span(*span);
                    self.write_stmt(&format!("{dst} <- {rendered}"));
                    self.note_var_write(dst);
                    self.bind_value_to_var(*src, dst);
                    self.bind_var_to_value(dst, *src);
                    self.log_last_assigned_value_change(dst);
                    self.value_tracker
                        .last_assigned_value_ids
                        .insert(dst.clone(), *src);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if let Some(partial_slice_stmt) =
                    self.try_render_constant_safe_partial_self_assign(dst, *src, values, params)
                {
                    self.record_span(*span);
                    self.write_stmt(&partial_slice_stmt);
                    self.note_var_write(dst);
                    self.invalidate_var_binding(dst);
                    self.value_tracker.last_assigned_value_ids.remove(dst);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if let Some(row_slice_stmt) =
                    self.try_render_safe_idx_cube_row_slice_assign(dst, *src, values, params)
                {
                    self.record_span(*span);
                    self.write_stmt(&row_slice_stmt);
                    self.note_var_write(dst);
                    self.invalidate_var_binding(dst);
                    self.value_tracker.last_assigned_value_ids.remove(dst);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if let Some(whole_range_rhs) =
                    self.try_resolve_whole_range_self_assign_rhs(dst, *src, values, params)
                {
                    if whole_range_rhs == *dst {
                        self.invalidate_emitted_cse_temps();
                        return Ok(());
                    }
                    self.record_span(*span);
                    self.write_stmt(&format!("{dst} <- {whole_range_rhs}"));
                    self.note_var_write(dst);
                    self.loop_analysis
                        .recent_whole_assign_bases
                        .insert(dst.clone());
                    self.bind_value_to_var(*src, dst);
                    self.bind_var_to_value(dst, *src);
                    self.remember_known_full_end_expr(dst, *src, values, params);
                    self.log_last_assigned_value_change(dst);
                    self.value_tracker
                        .last_assigned_value_ids
                        .insert(dst.clone(), *src);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if let Some(whole_range_rhs) =
                    self.try_resolve_whole_range_call_map_rhs(dst, *src, values, params)
                {
                    self.record_span(*span);
                    self.write_stmt(&format!("{dst} <- {whole_range_rhs}"));
                    self.note_var_write(dst);
                    self.loop_analysis
                        .recent_whole_assign_bases
                        .insert(dst.clone());
                    self.bind_value_to_var(*src, dst);
                    self.bind_var_to_value(dst, *src);
                    self.remember_known_full_end_expr(dst, *src, values, params);
                    self.log_last_assigned_value_change(dst);
                    self.value_tracker
                        .last_assigned_value_ids
                        .insert(dst.clone(), *src);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if let Some(whole_range_rhs) =
                    self.try_resolve_whole_auto_call_map_rhs(dst, *src, values, params)
                {
                    self.record_span(*span);
                    self.write_stmt(&format!("{dst} <- {whole_range_rhs}"));
                    self.note_var_write(dst);
                    self.loop_analysis
                        .recent_whole_assign_bases
                        .insert(dst.clone());
                    self.bind_value_to_var(*src, dst);
                    self.bind_var_to_value(dst, *src);
                    self.remember_known_full_end_expr(dst, *src, values, params);
                    self.log_last_assigned_value_change(dst);
                    self.value_tracker
                        .last_assigned_value_ids
                        .insert(dst.clone(), *src);
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                let preserve_loop_seed = self.in_active_loop_mutated_context(dst);
                let same_origin_self_assign = values[*src].origin_var.as_deref()
                    == Some(dst.as_str())
                    && self.resolve_bound_value_id(dst) != Some(*src);
                let stale_origin_probe = if same_origin_self_assign {
                    None
                } else {
                    self.resolve_stale_origin_var(*src, &values[*src], values)
                };
                let bound_probe = if same_origin_self_assign {
                    None
                } else {
                    self.resolve_bound_value(*src)
                };
                let mutated_whole_range_copy_probe =
                    self.try_resolve_mutated_whole_range_copy_alias(*src, values, params);
                let allow_last_assigned_skip = !matches!(
                    values[*src].kind,
                    ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. }
                );
                let stale_self_fresh_replay = !preserve_loop_seed
                    && self.is_fresh_mutable_aggregate_value(&values[*src])
                    && values[*src].origin_var.as_deref() == Some(dst.as_str())
                    && self.resolve_bound_value_id(dst) != Some(*src)
                    && (self.value_tracker.value_bindings.get(src).is_some_and(
                        |(bound_var, version)| {
                            bound_var == dst && self.current_var_version(dst) != *version
                        },
                    ) || self.resolve_bound_value_id(dst).is_some_and(|current| {
                        !self.is_fresh_mutable_aggregate_value(&values[current])
                    }));
                let stale_same_origin_fresh_without_live_binding = !preserve_loop_seed
                    && self.is_fresh_mutable_aggregate_value(&values[*src])
                    && values[*src].origin_var.as_deref() == Some(dst.as_str())
                    && self.resolve_bound_value_id(dst).is_none()
                    && self.current_var_version(dst) > 0;
                if stale_self_fresh_replay {
                    if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_some() {
                        let binding = self.value_tracker.value_bindings.get(src).cloned();
                        eprintln!(
                            "RR_DEBUG_EMIT_ASSIGN skip=stale_self_fresh_replay fn={} dst={} src={} kind={:?} current_bound={:?} origin={:?} binding={:?} current_version={}",
                            self.current_fn_name,
                            dst,
                            src,
                            values[*src].kind,
                            self.resolve_bound_value_id(dst),
                            values[*src].origin_var,
                            binding,
                            self.current_var_version(dst),
                        );
                    }
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if stale_same_origin_fresh_without_live_binding {
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if allow_last_assigned_skip
                    && let Some(prev_src) =
                        self.value_tracker.last_assigned_value_ids.get(dst).copied()
                    && values[prev_src].kind == values[*src].kind
                {
                    if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_some() {
                        eprintln!(
                            "RR_DEBUG_EMIT_ASSIGN skip=last_assigned_same_kind fn={} dst={} src={} prev_src={} kind={:?}",
                            self.current_fn_name, dst, src, prev_src, values[*src].kind,
                        );
                    }
                    self.invalidate_emitted_cse_temps();
                    return Ok(());
                }
                if !preserve_loop_seed {
                    if matches!(
                        values[*src].kind,
                        ValueKind::Call { ref callee, .. }
                            if self.call_is_known_fresh_allocation(callee)
                    ) && values[*src].origin_var.as_deref() == Some(dst.as_str())
                        && self
                            .resolve_stale_origin_var(*src, &values[*src], values)
                            .as_deref()
                            == Some(dst.as_str())
                        && self
                            .resolve_bound_value_id(dst)
                            .is_some_and(|current_val_id| {
                                values[current_val_id].kind == values[*src].kind
                            })
                    {
                        self.invalidate_emitted_cse_temps();
                        return Ok(());
                    }
                    if self.resolve_bound_value_id(dst) == Some(*src) {
                        self.invalidate_emitted_cse_temps();
                        return Ok(());
                    }
                    if let Some(current_val_id) = self.resolve_bound_value_id(dst)
                        && current_val_id != *src
                    {
                        if values[current_val_id].kind == values[*src].kind {
                            self.invalidate_emitted_cse_temps();
                            return Ok(());
                        }
                        let src_expr = self.resolve_val(*src, values, params, true);
                        let current_expr = self.resolve_val(current_val_id, values, params, true);
                        if src_expr == current_expr {
                            self.invalidate_emitted_cse_temps();
                            return Ok(());
                        }
                    }
                }
                let v = if let Some(alias_var) = mutated_whole_range_copy_probe.clone() {
                    alias_var
                } else if !matches!(
                    values[*src].kind,
                    ValueKind::Const(_) | ValueKind::Load { .. } | ValueKind::Param { .. }
                ) && let Some(origin_var) = stale_origin_probe.clone()
                {
                    origin_var
                } else if !matches!(values[*src].kind, ValueKind::Const(_))
                    && let Some(bound) = bound_probe.clone()
                {
                    bound
                } else {
                    let preview = self.resolve_val(*src, values, params, true);
                    if preview == *dst {
                        preview
                    } else {
                        self.emit_common_subexpr_temps(*src, values, params);
                        self.resolve_val(*src, values, params, true)
                    }
                };
                let v =
                    self.rewrite_known_one_based_full_range_alias_reads(v.as_str(), values, params);
                if std::env::var_os("RR_DEBUG_EMIT_ASSIGN").is_some() {
                    eprintln!(
                        "RR_DEBUG_EMIT_ASSIGN fn={} dst={} src={} kind={:?} rendered={} skip={}",
                        self.current_fn_name,
                        dst,
                        src,
                        values[*src].kind,
                        v,
                        v == *dst
                    );
                }
                if v != *dst {
                    self.record_span(*span);
                    self.write_stmt(&format!("{} <- {}", dst, v));
                    self.note_var_write(dst);
                    self.loop_analysis
                        .recent_whole_assign_bases
                        .insert(dst.clone());
                    if !matches!(&values[*src].kind, ValueKind::Load { var } if var != dst) {
                        self.bind_value_to_var(*src, dst);
                    }
                    if !matches!(&values[*src].kind, ValueKind::Load { .. }) {
                        self.bind_var_to_value(dst, *src);
                    }
                    self.remember_known_full_end_expr(dst, *src, values, params);
                    self.log_last_assigned_value_change(dst);
                    self.value_tracker
                        .last_assigned_value_ids
                        .insert(dst.clone(), *src);
                }
                self.invalidate_emitted_cse_temps();
            }
            Instr::Eval { val, span } => {
                self.emit_mark(*span, Some("eval"));
                self.record_span(*span);
                let v = self.resolve_val(*val, values, params, false);
                self.write_stmt(&v);
            }
            Instr::StoreIndex1D {
                base,
                idx,
                val,
                is_vector,
                is_safe,
                is_na_safe,
                span,
            } => {
                self.emit_mark(*span, Some("store"));
                self.record_span(*span);
                let base_val = self.resolve_mutable_base(*base, values, params);
                let idx_val = self.resolve_val(*idx, values, params, false);
                let src_val = self.resolve_val(*val, values, params, false);

                if *is_vector {
                    self.write_stmt(&format!("{} <- {}", base_val, src_val));
                    self.bump_base_version_if_named(*base, values);
                    if let Some(base_name) = Self::named_mutable_base_expr(
                        *base,
                        values,
                        &self.value_tracker.value_bindings,
                        &self.value_tracker.var_versions,
                    ) {
                        self.bind_value_to_var(*val, &base_name);
                        if !matches!(&values[*val].kind, ValueKind::Load { .. }) {
                            self.bind_var_to_value(&base_name, *val);
                        }
                        self.loop_analysis
                            .recent_whole_assign_bases
                            .insert(base_name.clone());
                        self.remember_known_full_end_expr(&base_name, *val, values, params);
                    }
                } else {
                    let idx_elidable = self.can_elide_index_expr(*idx, values, params);
                    if (*is_safe && *is_na_safe) || idx_elidable {
                        self.write_stmt(&format!("{}[{}] <- {}", base_val, idx_val, src_val));
                    } else {
                        let idx_expr = format!("rr_index1_write({}, \"index\")", idx_val);
                        self.write_stmt(&format!("{}[{}] <- {}", base_val, idx_expr, src_val));
                    }
                    self.bump_base_version_if_named(*base, values);
                }
            }
            Instr::StoreIndex2D {
                base,
                r,
                c,
                val,
                span,
            } => {
                self.emit_mark(*span, Some("store2d"));
                self.record_span(*span);
                let base_val = self.resolve_mutable_base(*base, values, params);
                let r_val = self.resolve_val(*r, values, params, false);
                let c_val = self.resolve_val(*c, values, params, false);
                let src_val = self.resolve_val(*val, values, params, false);
                let r_idx = if self.can_elide_index_expr(*r, values, params) {
                    r_val
                } else {
                    format!("rr_index1_write({}, \"row\")", r_val)
                };
                let c_idx = if self.can_elide_index_expr(*c, values, params) {
                    c_val
                } else {
                    format!("rr_index1_write({}, \"col\")", c_val)
                };
                self.write_stmt(&format!(
                    "{}[{}, {}] <- {}",
                    base_val, r_idx, c_idx, src_val
                ));
                self.bump_base_version_if_named(*base, values);
            }
            Instr::StoreIndex3D {
                base,
                i,
                j,
                k,
                val,
                span,
            } => {
                self.emit_mark(*span, Some("store3d"));
                self.record_span(*span);
                let base_val = self.resolve_mutable_base(*base, values, params);
                let i_val = self.resolve_val(*i, values, params, false);
                let j_val = self.resolve_val(*j, values, params, false);
                let k_val = self.resolve_val(*k, values, params, false);
                let src_val = self.resolve_val(*val, values, params, false);
                let i_idx = if self.can_elide_index_expr(*i, values, params) {
                    i_val
                } else {
                    format!("rr_index1_write({}, \"dim1\")", i_val)
                };
                let j_idx = if self.can_elide_index_expr(*j, values, params) {
                    j_val
                } else {
                    format!("rr_index1_write({}, \"dim2\")", j_val)
                };
                let k_idx = if self.can_elide_index_expr(*k, values, params) {
                    k_val
                } else {
                    format!("rr_index1_write({}, \"dim3\")", k_val)
                };
                self.write_stmt(&format!(
                    "{}[{}, {}, {}] <- {}",
                    base_val, i_idx, j_idx, k_idx, src_val
                ));
                self.bump_base_version_if_named(*base, values);
            }
        }
        Ok(())
    }

    pub(super) fn emit_term(
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
                let c = self.resolve_cond(*cond, values, params);
                self.write_stmt(&format!("if ({}) {{ # goto {}/{}", c, then_bb, else_bb));
                self.write_stmt("}");
            }
            Terminator::Return(Some(v)) => {
                let val = self.resolve_val(*v, values, params, false);
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
