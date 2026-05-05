use super::*;
pub(crate) struct PipelineLinearScan<'a> {
    pub(crate) direct_builtin_call_map: bool,
    pub(crate) preserve_all_defs: bool,
    pub(crate) fresh_user_calls: &'a FxHashSet<String>,
    pub(crate) reusable_pure_user_calls: FxHashSet<String>,
    pub(crate) mutated_arg_aliases: FxHashSet<String>,
    pub(crate) scalar_consts: FxHashMap<String, String>,
    pub(crate) vector_lens: FxHashMap<String, String>,
    pub(crate) identity_indices: FxHashMap<String, String>,
    pub(crate) aliases: FxHashMap<String, String>,
    pub(crate) no_na_vars: FxHashSet<String>,
    pub(crate) helper_heavy_vars: FxHashSet<String>,
    pub(crate) fresh_expr_for_var: FxHashMap<String, String>,
    pub(crate) pure_call_bindings: Vec<PureCallBinding>,
    pub(crate) last_rhs_for_var: FxHashMap<String, String>,
    pub(crate) out_lines: Vec<String>,
    pub(crate) conditional_depth: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct PipelineLinearScanConfig {
    pub(crate) direct_builtin_call_map: bool,
    pub(crate) preserve_all_defs: bool,
}

impl<'a> PipelineLinearScan<'a> {
    pub(crate) fn new(
        code: &str,
        config: PipelineLinearScanConfig,
        pure_user_calls: &FxHashSet<String>,
        fresh_user_calls: &'a FxHashSet<String>,
    ) -> Self {
        let reusable_pure_user_calls = pure_user_calls
            .iter()
            .filter(|name| !fresh_user_calls.contains(*name))
            .cloned()
            .collect();
        Self {
            direct_builtin_call_map: config.direct_builtin_call_map,
            preserve_all_defs: config.preserve_all_defs,
            fresh_user_calls,
            reusable_pure_user_calls,
            mutated_arg_aliases: collect_mutated_arg_aliases(code),
            scalar_consts: FxHashMap::default(),
            vector_lens: FxHashMap::default(),
            identity_indices: FxHashMap::default(),
            aliases: FxHashMap::default(),
            no_na_vars: FxHashSet::default(),
            helper_heavy_vars: FxHashSet::default(),
            fresh_expr_for_var: FxHashMap::default(),
            pure_call_bindings: Vec::new(),
            last_rhs_for_var: FxHashMap::default(),
            out_lines: Vec::new(),
            conditional_depth: 0,
        }
    }

    pub(crate) fn into_lines(self) -> Vec<String> {
        self.out_lines
    }

    pub(crate) fn process_code(&mut self, code: &str) {
        for line in code.lines() {
            self.process_line(line);
        }
    }

    pub(crate) fn process_line(&mut self, line: &str) {
        let trimmed_line = line.trim();
        let opens_conditional = trimmed_line.starts_with("if ") && trimmed_line.ends_with('{');
        self.track_conditional_close(trimmed_line);
        if self.try_process_truthy_guard(line, trimmed_line, opens_conditional)
            || self.try_process_function_boundary(line, opens_conditional)
            || self.try_process_control_flow_boundary(line)
            || self.try_process_indexed_store(line)
        {
            return;
        }

        let Some(caps) = assign_re().and_then(|re| re.captures(line)) else {
            self.out_lines
                .push(rewrite_return_expr_line(line, &self.last_rhs_for_var));
            return;
        };
        self.process_assignment(line, &caps);
        if opens_conditional {
            self.conditional_depth += 1;
        }
    }

    pub(crate) fn track_conditional_close(&mut self, trimmed_line: &str) {
        let closes_conditional = trimmed_line == "}";
        let else_boundary = trimmed_line.starts_with("} else {");
        if closes_conditional && !else_boundary && self.conditional_depth > 0 {
            self.conditional_depth -= 1;
        }
    }

    pub(crate) fn try_process_truthy_guard(
        &mut self,
        line: &str,
        trimmed_line: &str,
        opens_conditional: bool,
    ) -> bool {
        if trimmed_line.starts_with("if ")
            && trimmed_line.ends_with(" break")
            && trimmed_line.contains("rr_truthy1(")
        {
            self.out_lines.push(rewrite_guard_truthy_line(
                line,
                &self.no_na_vars,
                &self.scalar_consts,
            ));
            return true;
        }
        if trimmed_line.starts_with("if ")
            && trimmed_line.ends_with('{')
            && trimmed_line.contains("rr_truthy1(")
        {
            self.out_lines.push(rewrite_if_truthy_line(
                line,
                &self.no_na_vars,
                &self.scalar_consts,
            ));
            if opens_conditional {
                self.conditional_depth += 1;
            }
            return true;
        }
        false
    }

    pub(crate) fn try_process_function_boundary(
        &mut self,
        line: &str,
        opens_conditional: bool,
    ) -> bool {
        if !line.contains("<- function") {
            return false;
        }
        self.clear_all_linear_state();
        self.out_lines.push(line.to_string());
        if opens_conditional {
            self.conditional_depth += 1;
        }
        true
    }

    pub(crate) fn try_process_control_flow_boundary(&mut self, line: &str) -> bool {
        if !is_control_flow_boundary(line) {
            return false;
        }
        let mut rewritten_line = line.to_string();
        if rewritten_line.trim().starts_with("if ")
            && rewritten_line.trim().ends_with('{')
            && rewritten_line.contains("rr_truthy1(")
        {
            rewritten_line =
                rewrite_if_truthy_line(&rewritten_line, &self.no_na_vars, &self.scalar_consts);
        }
        if line.trim() == "repeat {" {
            clear_loop_boundary_facts(
                &mut self.identity_indices,
                &mut self.aliases,
                &mut self.no_na_vars,
                &mut self.helper_heavy_vars,
            );
        } else {
            self.clear_facts();
        }
        self.clear_transient_bindings();
        self.out_lines.push(rewritten_line);
        true
    }

    pub(crate) fn try_process_indexed_store(&mut self, line: &str) -> bool {
        let Some(base) = indexed_store_base_re()
            .and_then(|re| re.captures(line))
            .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
        else {
            return false;
        };
        self.invalidate_written_base(&base);
        self.out_lines.push(line.to_string());
        true
    }

    pub(crate) fn process_assignment(&mut self, _line: &str, caps: &Captures<'_>) {
        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();
        let rewritten_rhs = self.rewrite_assignment_rhs(lhs, rhs);

        self.record_scalar_const_fact(lhs, &rewritten_rhs);
        self.invalidate_written_targets(lhs, &rewritten_rhs);
        self.record_alias_fact(lhs, &rewritten_rhs);
        self.record_identity_index_fact(lhs, &rewritten_rhs);
        self.record_len_fact(lhs, &rewritten_rhs);
        self.record_no_na_fact(lhs, &rewritten_rhs);
        self.record_helper_heavy_fact(lhs, &rewritten_rhs);
        self.record_fresh_alloc_fact(lhs, &rewritten_rhs);
        self.record_pure_call_fact(lhs, &rewritten_rhs);
        self.last_rhs_for_var
            .insert(lhs.to_string(), rewritten_rhs.clone());
        self.out_lines
            .push(format!("{indent}{lhs} <- {rewritten_rhs}"));
    }

    pub(crate) fn rewrite_assignment_rhs(&mut self, lhs: &str, rhs: &str) -> String {
        let rewritten_rhs = self.rewrite_vector_read_alias(rhs);
        let rewritten_rhs = self.rewrite_common_rhs_aliases(&rewritten_rhs);
        let rewritten_rhs = self.rewrite_call_map_slice_rhs(&rewritten_rhs);
        let rewritten_rhs = self.rewrite_direct_builtin_call_map_rhs(&rewritten_rhs);
        let rewritten_rhs = self.rewrite_strict_ifelse_rhs(rewritten_rhs);
        let rewritten_rhs = self.rewrite_whole_assign_slice_rhs(rewritten_rhs);
        let rewritten_rhs = self.rewrite_common_rhs_aliases(&rewritten_rhs);
        let rewritten_rhs = self.rewrite_strict_ifelse_rhs(rewritten_rhs);
        self.expand_fresh_alias_rhs(lhs, rewritten_rhs)
    }

    pub(crate) fn rewrite_vector_read_alias(&self, rhs: &str) -> String {
        let Some(re) = read_vec_re() else {
            return rhs.to_string();
        };
        re.replace_all(rhs, |caps: &Captures<'_>| {
            let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
            let idx = caps.name("idx").map(|m| m.as_str()).unwrap_or("");
            match (
                identity_index_end_expr(idx, &self.identity_indices, &self.scalar_consts),
                self.vector_lens.get(base),
            ) {
                (Some(end), Some(base_len)) if &end == base_len => base.to_string(),
                _ => caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string(),
            }
        })
        .to_string()
    }

    pub(crate) fn rewrite_common_rhs_aliases(&self, rhs: &str) -> String {
        let rhs = rewrite_known_length_calls(rhs, &self.vector_lens);
        let rhs = rewrite_known_aliases(&rhs, &self.aliases);
        let rhs = rewrite_direct_vec_helper_expr(&rhs, self.direct_builtin_call_map);
        rewrite_pure_call_reuse(&rhs, &self.pure_call_bindings)
    }

    pub(crate) fn rewrite_call_map_slice_rhs(&self, rhs: &str) -> String {
        let Some(caps) = call_map_slice_re().and_then(|re| re.captures(rhs)) else {
            return rhs.to_string();
        };
        let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
        let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
        let end = normalize_expr(
            caps.name("end").map(|m| m.as_str()).unwrap_or("").trim(),
            &self.scalar_consts,
        );
        let rest = caps.name("rest").map(|m| m.as_str()).unwrap_or("").trim();
        if is_one(start, &self.scalar_consts)
            && self
                .vector_lens
                .get(dest)
                .is_some_and(|dest_len| dest_len == &end)
        {
            format!("rr_call_map_whole_auto({dest}, {rest})")
        } else {
            rhs.to_string()
        }
    }

    pub(crate) fn rewrite_direct_builtin_call_map_rhs(&self, rhs: &str) -> String {
        if !self.direct_builtin_call_map {
            return rhs.to_string();
        }
        let Some(caps) = call_map_whole_builtin_re().and_then(|re| re.captures(rhs)) else {
            return rhs.to_string();
        };
        let callee = caps.name("callee").map(|m| m.as_str()).unwrap_or("");
        let slots = caps.name("slots").map(|m| m.as_str()).unwrap_or("").trim();
        let args = caps.name("args").map(|m| m.as_str()).unwrap_or("").trim();
        if (slots == "1L" || slots == "1")
            && !helper_heavy_runtime_auto_args_with_temps(args, &self.helper_heavy_vars)
        {
            match callee {
                "abs" | "sqrt" | "log" | "pmax" | "pmin" => format!("{callee}({args})"),
                _ => rhs.to_string(),
            }
        } else {
            rhs.to_string()
        }
    }

    pub(crate) fn rewrite_strict_ifelse_rhs(&self, rhs: String) -> String {
        if self.preserve_all_defs {
            rhs
        } else {
            rewrite_strict_ifelse_expr(&rhs, &self.no_na_vars, &self.scalar_consts)
        }
    }

    pub(crate) fn rewrite_whole_assign_slice_rhs(&self, rhs: String) -> String {
        let Some(caps) = assign_slice_re().and_then(|re| re.captures(&rhs)) else {
            return rhs;
        };
        let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
        let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
        let end = normalize_expr(
            caps.name("end").map(|m| m.as_str()).unwrap_or("").trim(),
            &self.scalar_consts,
        );
        let rest = caps.name("rest").map(|m| m.as_str()).unwrap_or("").trim();
        if is_one(start, &self.scalar_consts)
            && self
                .vector_lens
                .get(dest)
                .is_some_and(|dest_len| dest_len == &end)
            && infer_len_from_expr(rest, &self.vector_lens, &self.scalar_consts)
                .is_some_and(|len| len == end)
        {
            rest.to_string()
        } else {
            rhs
        }
    }

    pub(crate) fn expand_fresh_alias_rhs(&self, lhs: &str, rhs: String) -> String {
        if self.last_rhs_for_var.contains_key(lhs) {
            rhs
        } else {
            maybe_expand_fresh_alias_rhs(&rhs, &self.fresh_expr_for_var).unwrap_or(rhs)
        }
    }

    pub(crate) fn record_scalar_const_fact(&mut self, lhs: &str, rhs: &str) {
        if scalar_lit_re().is_some_and(|re| re.is_match(rhs.trim())) {
            self.scalar_consts
                .insert(lhs.to_string(), rhs.trim().to_string());
        } else {
            self.scalar_consts.remove(lhs);
        }
    }

    pub(crate) fn invalidate_written_targets(&mut self, lhs: &str, rewritten_rhs: &str) {
        if let Some(base) = written_base_var(lhs) {
            self.invalidate_written_base(base);
        }
        invalidate_aliases_for_write(lhs, &mut self.aliases);
        self.pure_call_bindings
            .retain(|binding| binding.var != lhs && !binding.deps.contains(lhs));
        self.last_rhs_for_var
            .retain(|var, rhs| var != lhs && !expr_idents(rhs).iter().any(|ident| ident == lhs));
        let _ = rewritten_rhs;
    }

    pub(crate) fn invalidate_written_base(&mut self, base: &str) {
        self.fresh_expr_for_var.remove(base);
        self.scalar_consts.remove(base);
        self.vector_lens.remove(base);
        self.identity_indices.remove(base);
        self.no_na_vars.remove(base);
        self.helper_heavy_vars.remove(base);
        self.last_rhs_for_var
            .retain(|var, rhs| var != base && !expr_idents(rhs).iter().any(|ident| ident == base));
        invalidate_aliases_for_write(base, &mut self.aliases);
        self.pure_call_bindings
            .retain(|binding| binding.var != base && !binding.deps.contains(base));
    }

    pub(crate) fn record_alias_fact(&mut self, lhs: &str, rhs: &str) {
        let rhs_ident = rhs.trim();
        let allow_simple_alias = !self.preserve_all_defs
            && self.conditional_depth == 0
            && plain_ident_re().is_some_and(|re| re.is_match(lhs))
            && plain_ident_re().is_some_and(|re| re.is_match(rhs_ident))
            && !self.mutated_arg_aliases.contains(lhs)
            && !self.mutated_arg_aliases.contains(rhs_ident)
            && !self.fresh_expr_for_var.contains_key(rhs_ident);
        if (is_peephole_temp(lhs) || allow_simple_alias) && rhs_ident != lhs {
            self.aliases
                .insert(lhs.to_string(), resolve_alias(rhs_ident, &self.aliases));
        }
    }

    pub(crate) fn record_identity_index_fact(&mut self, lhs: &str, rhs: &str) {
        if let Some(caps) = range_re().and_then(|re| re.captures(rhs.trim())) {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            if is_one(start, &self.scalar_consts) {
                self.identity_indices.insert(
                    lhs.to_string(),
                    normalize_expr(
                        caps.name("end").map(|m| m.as_str()).unwrap_or(""),
                        &self.scalar_consts,
                    ),
                );
                return;
            }
        } else if let Some(caps) = floor_re().and_then(|re| re.captures(rhs.trim())) {
            let src = caps.name("src").map(|m| m.as_str()).unwrap_or("");
            if let Some(end) = self.identity_indices.get(src).cloned() {
                self.identity_indices.insert(lhs.to_string(), end);
                return;
            }
        }
        self.identity_indices.remove(lhs);
    }

    pub(crate) fn record_len_fact(&mut self, lhs: &str, rhs: &str) {
        if let Some(len) = infer_len_from_expr(rhs, &self.vector_lens, &self.scalar_consts) {
            self.vector_lens.insert(lhs.to_string(), len);
        } else {
            self.vector_lens.remove(lhs);
        }
    }

    pub(crate) fn record_no_na_fact(&mut self, lhs: &str, rhs: &str) {
        if expr_proven_no_na(rhs, &self.no_na_vars, &self.scalar_consts) {
            self.no_na_vars.insert(lhs.to_string());
        } else {
            self.no_na_vars.remove(lhs);
        }
    }

    pub(crate) fn record_helper_heavy_fact(&mut self, lhs: &str, rhs: &str) {
        if helper_heavy_runtime_auto_args(rhs) {
            self.helper_heavy_vars.insert(lhs.to_string());
        } else {
            self.helper_heavy_vars.remove(lhs);
        }
    }

    pub(crate) fn record_fresh_alloc_fact(&mut self, lhs: &str, rhs: &str) {
        if expr_is_fresh_allocation_like(rhs, self.fresh_user_calls) {
            self.fresh_expr_for_var
                .insert(lhs.to_string(), rhs.to_string());
        }
    }

    pub(crate) fn record_pure_call_fact(&mut self, lhs: &str, rhs: &str) {
        if self.conditional_depth == 0
            && !is_peephole_temp(lhs)
            && let Some(binding) =
                extract_pure_call_binding(lhs, rhs, &self.reusable_pure_user_calls)
        {
            self.pure_call_bindings.push(binding);
        }
    }

    pub(crate) fn clear_all_linear_state(&mut self) {
        self.clear_facts();
        self.clear_transient_bindings();
    }

    pub(crate) fn clear_facts(&mut self) {
        clear_linear_facts(
            &mut self.scalar_consts,
            &mut self.vector_lens,
            &mut self.identity_indices,
            &mut self.aliases,
            &mut self.no_na_vars,
            &mut self.helper_heavy_vars,
        );
    }

    pub(crate) fn clear_transient_bindings(&mut self) {
        self.fresh_expr_for_var.clear();
        self.last_rhs_for_var.clear();
        self.pure_call_bindings.clear();
    }
}

pub(crate) fn run_pipeline_linear_scan_stage(
    pass_manager: &PeepholePassManager,
    code: &str,
    config: PipelineLinearScanConfig,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
) -> PipelineLinearScanOutcome {
    let mut scanner = PipelineLinearScan::new(code, config, pure_user_calls, fresh_user_calls);
    let ((), elapsed_ns) = pass_manager.run(PeepholeStageId::LinearScan, || {
        scanner.process_code(code);
    });
    PipelineLinearScanOutcome {
        lines: scanner.into_lines(),
        elapsed_ns,
    }
}

pub(crate) fn run_primary_pipeline_stages(
    pass_manager: &PeepholePassManager,
    out_lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    config: PrimaryPipelineConfig,
    analysis_cache: &mut PeepholeAnalysisCache,
    repeat_loop_cache: &mut RepeatLoopAnalysisCache,
) -> PrimaryPipelineOutcome {
    let started = Instant::now();
    let primary_flow = run_primary_flow_stage(
        pass_manager,
        out_lines,
        PrimaryFlowStageConfig {
            direct_builtin_call_map: config.direct_builtin_call_map,
            preserve_all_defs: config.preserve_all_defs,
        },
    );
    let flow_elapsed_ns = primary_flow.elapsed_ns;
    let primary_inline =
        run_primary_inline_stage(pass_manager, primary_flow.lines, pure_user_calls);
    let inline_elapsed_ns = primary_inline.elapsed_ns;
    let primary_reuse = run_primary_reuse_stage(
        pass_manager,
        primary_inline.lines,
        pure_user_calls,
        PrimaryReuseStageConfig {
            aggressive_o3: config.aggressive_o3,
            expression_controlled: config.expression_controlled,
        },
    );
    let reuse_elapsed_ns = primary_reuse.elapsed_ns;
    let loop_cleanup = run_primary_loop_cleanup_stage(
        pass_manager,
        primary_reuse.lines,
        PrimaryLoopCleanupOptions {
            fast_dev: config.fast_dev,
            preserve_all_defs: config.preserve_all_defs,
            size_controlled_simple_expr: config.expression_controlled,
        },
        pure_user_calls,
        analysis_cache,
        repeat_loop_cache,
    );
    build_primary_pipeline_outcome(
        started,
        flow_elapsed_ns,
        inline_elapsed_ns,
        reuse_elapsed_ns,
        loop_cleanup,
    )
}

pub(crate) fn run_secondary_pipeline_stages(
    pass_manager: &PeepholePassManager,
    out_lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    config: SecondaryPipelineConfig,
    analysis_cache: &mut PeepholeAnalysisCache,
) -> SecondaryPipelineOutcome {
    let started = Instant::now();
    let inline = run_secondary_inline_stage(pass_manager, out_lines, pure_user_calls);
    let inline_elapsed_ns = inline.elapsed_ns;
    let inline_branch_hoist_elapsed_ns = inline.branch_hoist_elapsed_ns;
    let inline_immediate_scalar_elapsed_ns = inline.immediate_scalar_elapsed_ns;
    let inline_named_index_elapsed_ns = inline.named_index_elapsed_ns;
    let inline_named_expr_elapsed_ns = inline.named_expr_elapsed_ns;
    let inline_scalar_region_elapsed_ns = inline.scalar_region_elapsed_ns;
    let inline_immediate_index_elapsed_ns = inline.immediate_index_elapsed_ns;
    let inline_adjacent_dedup_elapsed_ns = inline.adjacent_dedup_elapsed_ns;
    let exact = run_secondary_exact_stage(pass_manager, inline.lines);
    let exact_elapsed_ns = exact.elapsed_ns;
    let helper = run_secondary_helper_cleanup_stage(
        pass_manager,
        exact.lines,
        SecondaryHelperCleanupConfig {
            direct_builtin_call_map: config.direct_builtin_call_map,
            preserve_all_defs: config.preserve_all_defs,
            size_controlled_simple_expr: config.expression_controlled,
        },
        pure_user_calls,
    );
    let helper_cleanup_elapsed_ns = helper.elapsed_ns;
    let helper_wrapper_elapsed_ns = helper.wrapper_elapsed_ns;
    let helper_metric_elapsed_ns = helper.metric_elapsed_ns;
    let helper_alias_elapsed_ns = helper.alias_elapsed_ns;
    let helper_simple_expr_elapsed_ns = helper.simple_expr_elapsed_ns;
    let helper_full_range_elapsed_ns = helper.full_range_elapsed_ns;
    let helper_named_copy_elapsed_ns = helper.named_copy_elapsed_ns;
    let record_sroa_elapsed_ns = helper.record_sroa_elapsed_ns;
    let finalize = run_secondary_finalize_cleanup_stage(
        pass_manager,
        helper.lines,
        SecondaryFinalizeCleanupConfig {
            preserve_all_defs: config.preserve_all_defs,
            aggressive_o3: config.aggressive_o3,
            expression_controlled: config.expression_controlled,
        },
        pure_user_calls,
        analysis_cache,
    );
    let inline_profile = SecondaryInlineProfile {
        elapsed_ns: inline_elapsed_ns,
        branch_hoist_elapsed_ns: inline_branch_hoist_elapsed_ns,
        immediate_scalar_elapsed_ns: inline_immediate_scalar_elapsed_ns,
        named_index_elapsed_ns: inline_named_index_elapsed_ns,
        named_expr_elapsed_ns: inline_named_expr_elapsed_ns,
        scalar_region_elapsed_ns: inline_scalar_region_elapsed_ns,
        immediate_index_elapsed_ns: inline_immediate_index_elapsed_ns,
        adjacent_dedup_elapsed_ns: inline_adjacent_dedup_elapsed_ns,
    };
    let helper_profile = SecondaryHelperProfile {
        cleanup_elapsed_ns: helper_cleanup_elapsed_ns,
        wrapper_elapsed_ns: helper_wrapper_elapsed_ns,
        metric_elapsed_ns: helper_metric_elapsed_ns,
        alias_elapsed_ns: helper_alias_elapsed_ns,
        simple_expr_elapsed_ns: helper_simple_expr_elapsed_ns,
        full_range_elapsed_ns: helper_full_range_elapsed_ns,
        named_copy_elapsed_ns: helper_named_copy_elapsed_ns,
        record_sroa_elapsed_ns,
    };
    build_secondary_pipeline_outcome(
        started,
        inline_profile,
        exact_elapsed_ns,
        helper_profile,
        finalize,
    )
}

pub(crate) fn optimize_emitted_r_pipeline_impl_with_profile(
    code: &str,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    options: PeepholeOptions,
) -> ((String, Vec<u32>), PeepholeProfile) {
    debug_assert!(peephole_stage_catalog_is_well_formed());
    let pass_manager = PeepholePassManager::for_fast_dev(options.fast_dev);
    debug_assert!(pass_manager.validate_sequence(&[
        PeepholeStageId::LinearScan,
        PeepholeStageId::PrimaryFlow,
        PeepholeStageId::PrimaryInline,
        PeepholeStageId::PrimaryReuse,
        PeepholeStageId::PrimaryLoopCleanup,
        PeepholeStageId::Finalize,
    ]));
    if !options.fast_dev {
        debug_assert!(pass_manager.validate_sequence(&[
            PeepholeStageId::SecondaryInline,
            PeepholeStageId::SecondaryExact,
            PeepholeStageId::SecondaryHelperCleanup,
            PeepholeStageId::SecondaryRecordSroa,
            PeepholeStageId::SecondaryFinalizeCleanup,
            PeepholeStageId::Finalize,
        ]));
    }
    let mut analysis_cache = PeepholeAnalysisCache::default();
    let mut repeat_loop_cache = RepeatLoopAnalysisCache::default();
    let aggressive_o3 = matches!(options.opt_level, crate::compiler::OptLevel::O3);
    let expression_controlled = matches!(
        options.opt_level,
        crate::compiler::OptLevel::O3 | crate::compiler::OptLevel::Oz
    );
    let primary_config = PrimaryPipelineConfig {
        fast_dev: options.fast_dev,
        direct_builtin_call_map: options.direct_builtin_call_map,
        preserve_all_defs: options.preserve_all_defs,
        aggressive_o3,
        expression_controlled,
    };
    let secondary_config = SecondaryPipelineConfig {
        direct_builtin_call_map: options.direct_builtin_call_map,
        preserve_all_defs: options.preserve_all_defs,
        aggressive_o3,
        expression_controlled,
    };
    let linear_scan = run_pipeline_linear_scan_stage(
        &pass_manager,
        code,
        PipelineLinearScanConfig {
            direct_builtin_call_map: options.direct_builtin_call_map,
            preserve_all_defs: options.preserve_all_defs,
        },
        pure_user_calls,
        fresh_user_calls,
    );
    let mut primary = run_primary_pipeline_stages(
        &pass_manager,
        linear_scan.lines,
        pure_user_calls,
        primary_config,
        &mut analysis_cache,
        &mut repeat_loop_cache,
    );

    if options.fast_dev {
        let primary_lines = std::mem::take(&mut primary.lines);
        let line_map = std::mem::take(&mut primary.line_map);
        let (out, finalize_elapsed_ns) =
            run_fast_dev_finalize_stage(&pass_manager, primary_lines, code);
        let profile =
            build_pipeline_profile(linear_scan.elapsed_ns, &primary, None, finalize_elapsed_ns);
        return ((out, line_map), profile);
    }

    let primary_lines = std::mem::take(&mut primary.lines);
    let secondary = run_secondary_pipeline_stages(
        &pass_manager,
        primary_lines,
        pure_user_calls,
        secondary_config,
        &mut analysis_cache,
    );
    let profile = build_pipeline_profile(linear_scan.elapsed_ns, &primary, Some(&secondary), 0);
    let mut profile = profile;
    let line_map = std::mem::take(&mut primary.line_map);
    let (out, line_map, finalize_elapsed_ns) = run_standard_finalize_stage(
        &pass_manager,
        secondary.lines,
        line_map,
        &secondary.final_compact_map,
        code,
    );
    profile.finalize_elapsed_ns = finalize_elapsed_ns;
    ((out, line_map), profile)
}
