use super::*;
use std::time::Instant;

fn compare_exact_block_ir(
    stage: &str,
    input: &[String],
    legacy: &[String],
    pure_user_calls: &FxHashSet<String>,
) {
    let Some(mode) = std::env::var_os("RR_COMPARE_EXACT_BLOCK_IR") else {
        return;
    };
    let mut ir = input.to_vec();
    match stage {
        "exact_pre" => {
            ir = rewrite_forward_exact_expr_reuse_ir(ir);
            ir = strip_redundant_identical_pure_rebinds_ir(ir, pure_user_calls);
        }
        "exact_reuse" => {
            ir = strip_dead_simple_eval_lines(ir);
            ir = strip_noop_self_assignments(ir);
            ir = strip_redundant_nested_temp_reassigns(ir);
            ir = rewrite_forward_exact_pure_call_reuse_ir(ir, pure_user_calls);
            ir = rewrite_forward_exact_expr_reuse_ir(ir);
            ir = hoist_repeated_vector_helper_calls_within_lines(ir);
            ir = rewrite_forward_exact_vector_helper_reuse(ir);
            ir = rewrite_forward_temp_aliases(ir);
            ir = strip_redundant_identical_pure_rebinds_ir(ir, pure_user_calls);
        }
        _ => return,
    }
    if ir == legacy {
        return;
    }
    let mismatch_idx = legacy
        .iter()
        .zip(ir.iter())
        .position(|(lhs, rhs)| lhs != rhs)
        .unwrap_or_else(|| legacy.len().min(ir.len()));
    let legacy_line = legacy
        .get(mismatch_idx)
        .map(|line| line.trim())
        .unwrap_or("<eof>");
    let ir_line = ir
        .get(mismatch_idx)
        .map(|line| line.trim())
        .unwrap_or("<eof>");
    eprintln!(
        "RR_COMPARE_EXACT_BLOCK_IR diff stage={stage} legacy_lines={} ir_lines={} first_mismatch={} legacy=`{}` ir=`{}`",
        legacy.len(),
        ir.len(),
        mismatch_idx + 1,
        legacy_line,
        ir_line
    );
    if mode == "verbose" {
        let start = mismatch_idx.saturating_sub(2);
        let end = (mismatch_idx + 3)
            .max(start)
            .min(legacy.len().max(ir.len()));
        for idx in start..end {
            let legacy_line = legacy.get(idx).map(|line| line.trim()).unwrap_or("<eof>");
            let ir_line = ir.get(idx).map(|line| line.trim()).unwrap_or("<eof>");
            eprintln!(
                "RR_COMPARE_EXACT_BLOCK_IR ctx line={} legacy=`{}` ir=`{}`",
                idx + 1,
                legacy_line,
                ir_line
            );
        }
    }
}

fn compare_exact_reuse_substep(step: &str, input: &[String], legacy: &[String], ir: &[String]) {
    let Some(mode) = std::env::var_os("RR_COMPARE_EXACT_REUSE_STEPS") else {
        return;
    };
    if legacy == ir {
        return;
    }
    let mismatch_idx = legacy
        .iter()
        .zip(ir.iter())
        .position(|(lhs, rhs)| lhs != rhs)
        .unwrap_or_else(|| legacy.len().min(ir.len()));
    let legacy_line = legacy
        .get(mismatch_idx)
        .map(|line| line.trim())
        .unwrap_or("<eof>");
    let ir_line = ir
        .get(mismatch_idx)
        .map(|line| line.trim())
        .unwrap_or("<eof>");
    eprintln!(
        "RR_COMPARE_EXACT_REUSE_STEPS diff step={step} legacy_lines={} ir_lines={} first_mismatch={} legacy=`{}` ir=`{}`",
        legacy.len(),
        ir.len(),
        mismatch_idx + 1,
        legacy_line,
        ir_line
    );
    if mode == "verbose" {
        let start = mismatch_idx.saturating_sub(2);
        let end = (mismatch_idx + 3)
            .max(start)
            .min(legacy.len().max(ir.len()));
        for idx in start..end {
            let input_line = input.get(idx).map(|line| line.as_str()).unwrap_or("<eof>");
            let legacy_line = legacy.get(idx).map(|line| line.as_str()).unwrap_or("<eof>");
            let ir_line = ir.get(idx).map(|line| line.as_str()).unwrap_or("<eof>");
            eprintln!(
                "RR_COMPARE_EXACT_REUSE_STEPS ctx line={} input={:?} legacy={:?} ir={:?}",
                idx + 1,
                input_line,
                legacy_line,
                ir_line
            );
        }
    }
}

fn compare_exact_reuse_steps_enabled() -> bool {
    std::env::var_os("RR_COMPARE_EXACT_REUSE_STEPS").is_some()
}

fn compare_exact_block_enabled() -> bool {
    std::env::var_os("RR_COMPARE_EXACT_BLOCK_IR").is_some()
}

#[derive(Default)]
struct ExactFixpointProfile {
    prepare_elapsed_ns: u128,
    forward_elapsed_ns: u128,
    pure_call_elapsed_ns: u128,
    expr_elapsed_ns: u128,
    rebind_elapsed_ns: u128,
    rounds: usize,
}

fn run_exact_cleanup_fixpoint_rounds_with_profile(
    mut lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    max_rounds: usize,
) -> (Vec<String>, ExactFixpointProfile) {
    let mut profile = ExactFixpointProfile::default();
    for _ in 0..max_rounds {
        profile.rounds += 1;
        let before = lines.clone();
        let started = Instant::now();
        lines = strip_noop_self_assignments(lines);
        lines = strip_redundant_nested_temp_reassigns(lines);
        profile.prepare_elapsed_ns += started.elapsed().as_nanos();
        if lines == before {
            break;
        }
        if compare_exact_reuse_steps_enabled() {
            let after_prepare = lines.clone();
            let started = Instant::now();
            lines = rewrite_forward_exact_pure_call_reuse(lines, pure_user_calls);
            let pure_call_elapsed_ns = started.elapsed().as_nanos();
            profile.pure_call_elapsed_ns += pure_call_elapsed_ns;
            if lines == after_prepare {
                let might_need_more = lines.iter().any(|line| {
                    line.contains(".__rr_cse_")
                        || line.contains("rr_parallel_typed_vec_call(")
                        || line.contains("Sym_")
                });
                if !might_need_more {
                    break;
                }
            }
            let after_pure_call = lines.clone();
            let started = Instant::now();
            lines = rewrite_forward_exact_expr_reuse(lines);
            let expr_elapsed_ns = started.elapsed().as_nanos();
            profile.expr_elapsed_ns += expr_elapsed_ns;
            profile.forward_elapsed_ns += pure_call_elapsed_ns + expr_elapsed_ns;
            let after_expr = lines.clone();
            let started = Instant::now();
            lines = strip_redundant_identical_pure_rebinds(lines, pure_user_calls);
            profile.rebind_elapsed_ns += started.elapsed().as_nanos();
            if lines == before || (lines == after_expr && after_expr == after_pure_call) {
                break;
            }
        } else {
            let after_prepare = lines.clone();
            let (next_lines, exact_reuse_profile) =
                run_exact_reuse_ir_bundle(lines, pure_user_calls);
            profile.pure_call_elapsed_ns += exact_reuse_profile.pure_call_elapsed_ns;
            profile.expr_elapsed_ns += exact_reuse_profile.expr_elapsed_ns;
            profile.rebind_elapsed_ns += exact_reuse_profile.rebind_elapsed_ns;
            profile.forward_elapsed_ns +=
                exact_reuse_profile.pure_call_elapsed_ns + exact_reuse_profile.expr_elapsed_ns;
            lines = next_lines;
            if lines == before || lines == after_prepare {
                break;
            }
        }
    }
    (lines, profile)
}

pub(super) fn optimize_emitted_r_pipeline_impl(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
) -> (String, Vec<u32>) {
    optimize_emitted_r_pipeline_impl_with_profile(
        code,
        direct_builtin_call_map,
        pure_user_calls,
        fresh_user_calls,
        preserve_all_defs,
        false,
    )
    .0
}

pub(super) fn optimize_emitted_r_pipeline_impl_with_profile(
    code: &str,
    direct_builtin_call_map: bool,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    preserve_all_defs: bool,
    fast_dev: bool,
) -> ((String, Vec<u32>), PeepholeProfile) {
    let mut analysis_cache = PeepholeAnalysisCache::default();
    let mut repeat_loop_cache = RepeatLoopAnalysisCache::default();
    let reusable_pure_user_calls: FxHashSet<String> = pure_user_calls
        .iter()
        .filter(|name| !fresh_user_calls.contains(*name))
        .cloned()
        .collect();
    let mut scalar_consts: FxHashMap<String, String> = FxHashMap::default();
    let mut vector_lens: FxHashMap<String, String> = FxHashMap::default();
    let mut identity_indices: FxHashMap<String, String> = FxHashMap::default();
    let mut aliases: FxHashMap<String, String> = FxHashMap::default();
    let mut no_na_vars: FxHashSet<String> = FxHashSet::default();
    let mut helper_heavy_vars: FxHashSet<String> = FxHashSet::default();
    let mut fresh_expr_for_var: FxHashMap<String, String> = FxHashMap::default();
    let mut pure_call_bindings: Vec<PureCallBinding> = Vec::new();
    let mut last_rhs_for_var: FxHashMap<String, String> = FxHashMap::default();
    let mut out_lines = Vec::new();
    let mut conditional_depth = 0usize;
    let mutated_arg_aliases = collect_mutated_arg_aliases(code);
    let linear_started = Instant::now();

    for line in code.lines() {
        let trimmed_line = line.trim();
        let closes_conditional = trimmed_line == "}";
        let else_boundary = trimmed_line.starts_with("} else {");
        let opens_conditional = trimmed_line.starts_with("if ") && trimmed_line.ends_with('{');
        if closes_conditional && !else_boundary && conditional_depth > 0 {
            conditional_depth -= 1;
        }
        if trimmed_line.starts_with("if ")
            && trimmed_line.ends_with(" break")
            && trimmed_line.contains("rr_truthy1(")
        {
            let rewritten = rewrite_guard_truthy_line(line, &no_na_vars, &scalar_consts);
            out_lines.push(rewritten);
            continue;
        }
        if trimmed_line.starts_with("if ")
            && trimmed_line.ends_with('{')
            && trimmed_line.contains("rr_truthy1(")
        {
            let rewritten = rewrite_if_truthy_line(line, &no_na_vars, &scalar_consts);
            out_lines.push(rewritten);
            if opens_conditional {
                conditional_depth += 1;
            }
            continue;
        }

        if line.contains("<- function") {
            clear_linear_facts(
                &mut scalar_consts,
                &mut vector_lens,
                &mut identity_indices,
                &mut aliases,
                &mut no_na_vars,
                &mut helper_heavy_vars,
            );
            fresh_expr_for_var.clear();
            last_rhs_for_var.clear();
            pure_call_bindings.clear();
            out_lines.push(line.to_string());
            if opens_conditional {
                conditional_depth += 1;
            }
            continue;
        }

        if is_control_flow_boundary(line) {
            let mut rewritten_line = line.to_string();
            if rewritten_line.trim().starts_with("if ")
                && rewritten_line.trim().ends_with('{')
                && rewritten_line.contains("rr_truthy1(")
            {
                rewritten_line =
                    rewrite_if_truthy_line(&rewritten_line, &no_na_vars, &scalar_consts);
            }
            if trimmed_line == "repeat {" {
                clear_loop_boundary_facts(
                    &mut identity_indices,
                    &mut aliases,
                    &mut no_na_vars,
                    &mut helper_heavy_vars,
                );
            } else {
                clear_linear_facts(
                    &mut scalar_consts,
                    &mut vector_lens,
                    &mut identity_indices,
                    &mut aliases,
                    &mut no_na_vars,
                    &mut helper_heavy_vars,
                );
            }
            fresh_expr_for_var.clear();
            last_rhs_for_var.clear();
            pure_call_bindings.clear();
            out_lines.push(rewritten_line);
            continue;
        }

        if let Some(base) = indexed_store_base_re()
            .and_then(|re| re.captures(line))
            .and_then(|caps| caps.name("base").map(|m| m.as_str().trim().to_string()))
        {
            fresh_expr_for_var.remove(&base);
            scalar_consts.remove(&base);
            vector_lens.remove(&base);
            identity_indices.remove(&base);
            no_na_vars.remove(&base);
            helper_heavy_vars.remove(&base);
            last_rhs_for_var.remove(&base);
            invalidate_aliases_for_write(&base, &mut aliases);
            pure_call_bindings
                .retain(|binding| binding.var != base && !binding.deps.contains(&base));
            out_lines.push(line.to_string());
            continue;
        }

        let Some(caps) = assign_re().and_then(|re| re.captures(line)) else {
            let rewritten_line = rewrite_return_expr_line(line, &last_rhs_for_var);
            out_lines.push(rewritten_line);
            continue;
        };

        let indent = caps.name("indent").map(|m| m.as_str()).unwrap_or("");
        let lhs = caps.name("lhs").map(|m| m.as_str()).unwrap_or("");
        let rhs = caps.name("rhs").map(|m| m.as_str()).unwrap_or("").trim();

        let rewritten_rhs = if let Some(re) = read_vec_re() {
            re.replace_all(rhs, |caps: &Captures<'_>| {
                let base = caps.name("base").map(|m| m.as_str()).unwrap_or("");
                let idx = caps.name("idx").map(|m| m.as_str()).unwrap_or("");
                match (
                    identity_index_end_expr(idx, &identity_indices, &scalar_consts),
                    vector_lens.get(base),
                ) {
                    (Some(end), Some(base_len)) if &end == base_len => base.to_string(),
                    _ => caps.get(0).map(|m| m.as_str()).unwrap_or("").to_string(),
                }
            })
            .to_string()
        } else {
            rhs.to_string()
        };
        let rewritten_rhs = rewrite_known_length_calls(&rewritten_rhs, &vector_lens);
        let rewritten_rhs = rewrite_known_aliases(&rewritten_rhs, &aliases);
        let rewritten_rhs = rewrite_direct_vec_helper_expr(&rewritten_rhs, direct_builtin_call_map);
        let rewritten_rhs = rewrite_pure_call_reuse(&rewritten_rhs, &pure_call_bindings);

        let rewritten_rhs =
            if let Some(caps) = call_map_slice_re().and_then(|re| re.captures(&rewritten_rhs)) {
                let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
                let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
                let end = caps.name("end").map(|m| m.as_str()).unwrap_or("").trim();
                let rest = caps.name("rest").map(|m| m.as_str()).unwrap_or("").trim();
                let end = normalize_expr(end, &scalar_consts);
                if is_one(start, &scalar_consts)
                    && vector_lens
                        .get(dest)
                        .is_some_and(|dest_len| dest_len == &end)
                {
                    format!("rr_call_map_whole_auto({dest}, {rest})")
                } else {
                    rewritten_rhs
                }
            } else {
                rewritten_rhs
            };

        let rewritten_rhs = if direct_builtin_call_map {
            if let Some(caps) =
                call_map_whole_builtin_re().and_then(|re| re.captures(&rewritten_rhs))
            {
                let callee = caps.name("callee").map(|m| m.as_str()).unwrap_or("");
                let slots = caps.name("slots").map(|m| m.as_str()).unwrap_or("").trim();
                let args = caps.name("args").map(|m| m.as_str()).unwrap_or("").trim();
                if (slots == "1L" || slots == "1")
                    && !helper_heavy_runtime_auto_args_with_temps(args, &helper_heavy_vars)
                {
                    match callee {
                        "abs" | "sqrt" | "log" => format!("{callee}({args})"),
                        "pmax" | "pmin" => format!("{callee}({args})"),
                        _ => rewritten_rhs,
                    }
                } else {
                    rewritten_rhs
                }
            } else {
                rewritten_rhs
            }
        } else {
            rewritten_rhs
        };

        let rewritten_rhs = if preserve_all_defs {
            rewritten_rhs
        } else {
            rewrite_strict_ifelse_expr(&rewritten_rhs, &no_na_vars, &scalar_consts)
        };

        let rewritten_rhs =
            if let Some(caps) = assign_slice_re().and_then(|re| re.captures(&rewritten_rhs)) {
                let dest = caps.name("dest").map(|m| m.as_str()).unwrap_or("").trim();
                let start = caps.name("start").map(|m| m.as_str()).unwrap_or("").trim();
                let end = normalize_expr(
                    caps.name("end").map(|m| m.as_str()).unwrap_or("").trim(),
                    &scalar_consts,
                );
                let rest = caps.name("rest").map(|m| m.as_str()).unwrap_or("").trim();
                if is_one(start, &scalar_consts)
                    && vector_lens
                        .get(dest)
                        .is_some_and(|dest_len| dest_len == &end)
                    && infer_len_from_expr(rest, &vector_lens, &scalar_consts)
                        .is_some_and(|len| len == end)
                {
                    rest.to_string()
                } else {
                    rewritten_rhs
                }
            } else {
                rewritten_rhs
            };

        let rewritten_rhs = rewrite_known_length_calls(&rewritten_rhs, &vector_lens);
        let rewritten_rhs = rewrite_known_aliases(&rewritten_rhs, &aliases);
        let rewritten_rhs = rewrite_direct_vec_helper_expr(&rewritten_rhs, direct_builtin_call_map);
        let rewritten_rhs = rewrite_pure_call_reuse(&rewritten_rhs, &pure_call_bindings);
        let rewritten_rhs = if preserve_all_defs {
            rewritten_rhs
        } else {
            rewrite_strict_ifelse_expr(&rewritten_rhs, &no_na_vars, &scalar_consts)
        };
        let rewritten_rhs = if !last_rhs_for_var.contains_key(lhs) {
            maybe_expand_fresh_alias_rhs(&rewritten_rhs, &fresh_expr_for_var)
                .unwrap_or(rewritten_rhs)
        } else {
            rewritten_rhs
        };

        if scalar_lit_re().is_some_and(|re| re.is_match(rewritten_rhs.trim())) {
            scalar_consts.insert(lhs.to_string(), rewritten_rhs.trim().to_string());
        } else {
            scalar_consts.remove(lhs);
        }

        if let Some(base) = written_base_var(lhs) {
            fresh_expr_for_var.remove(base);
            scalar_consts.remove(base);
            vector_lens.remove(base);
            identity_indices.remove(base);
            no_na_vars.remove(base);
            helper_heavy_vars.remove(base);
            last_rhs_for_var.retain(|var, rhs| {
                var != base && !expr_idents(rhs).iter().any(|ident| ident == base)
            });
            invalidate_aliases_for_write(base, &mut aliases);
            pure_call_bindings
                .retain(|binding| binding.var != base && !binding.deps.contains(base));
        }
        invalidate_aliases_for_write(lhs, &mut aliases);
        pure_call_bindings.retain(|binding| binding.var != lhs && !binding.deps.contains(lhs));
        last_rhs_for_var
            .retain(|var, rhs| var != lhs && !expr_idents(rhs).iter().any(|ident| ident == lhs));
        let rhs_ident = rewritten_rhs.trim();
        let allow_simple_alias = !preserve_all_defs
            && conditional_depth == 0
            && plain_ident_re().is_some_and(|re| re.is_match(lhs))
            && plain_ident_re().is_some_and(|re| re.is_match(rhs_ident))
            && !mutated_arg_aliases.contains(lhs)
            && !mutated_arg_aliases.contains(rhs_ident)
            && !fresh_expr_for_var.contains_key(rhs_ident);
        if (is_peephole_temp(lhs) || allow_simple_alias) && rhs_ident != lhs {
            aliases.insert(lhs.to_string(), resolve_alias(rhs_ident, &aliases));
        }

        if let Some(caps) = range_re().and_then(|re| re.captures(rewritten_rhs.trim())) {
            let start = caps.name("start").map(|m| m.as_str()).unwrap_or("");
            if is_one(start, &scalar_consts) {
                identity_indices.insert(
                    lhs.to_string(),
                    normalize_expr(
                        caps.name("end").map(|m| m.as_str()).unwrap_or(""),
                        &scalar_consts,
                    ),
                );
            } else {
                identity_indices.remove(lhs);
            }
        } else if let Some(caps) = floor_re().and_then(|re| re.captures(rewritten_rhs.trim())) {
            let src = caps.name("src").map(|m| m.as_str()).unwrap_or("");
            if let Some(end) = identity_indices.get(src).cloned() {
                identity_indices.insert(lhs.to_string(), end);
            } else {
                identity_indices.remove(lhs);
            }
        } else {
            identity_indices.remove(lhs);
        }

        if let Some(len) = infer_len_from_expr(&rewritten_rhs, &vector_lens, &scalar_consts) {
            vector_lens.insert(lhs.to_string(), len);
        } else {
            vector_lens.remove(lhs);
        }

        if expr_proven_no_na(&rewritten_rhs, &no_na_vars, &scalar_consts) {
            no_na_vars.insert(lhs.to_string());
        } else {
            no_na_vars.remove(lhs);
        }

        if helper_heavy_runtime_auto_args(&rewritten_rhs) {
            helper_heavy_vars.insert(lhs.to_string());
        } else {
            helper_heavy_vars.remove(lhs);
        }

        if expr_is_fresh_allocation_like(&rewritten_rhs, fresh_user_calls) {
            fresh_expr_for_var.insert(lhs.to_string(), rewritten_rhs.clone());
        }

        if conditional_depth == 0
            && !is_peephole_temp(lhs)
            && let Some(binding) =
                extract_pure_call_binding(lhs, &rewritten_rhs, &reusable_pure_user_calls)
        {
            pure_call_bindings.push(binding);
        }
        last_rhs_for_var.insert(lhs.to_string(), rewritten_rhs.clone());

        out_lines.push(format!("{indent}{lhs} <- {rewritten_rhs}"));
        if opens_conditional {
            conditional_depth += 1;
        }
    }

    let linear_scan_elapsed_ns = linear_started.elapsed().as_nanos();
    let primary_started = Instant::now();
    let primary_flow_started = Instant::now();
    let out_lines = collapse_common_if_else_tail_assignments(out_lines);
    let out_lines = rewrite_full_range_conditional_scalar_loops(out_lines);
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        rewrite_inline_full_range_slice_ops(out_lines, direct_builtin_call_map)
    };
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        rewrite_one_based_full_range_index_alias_reads(out_lines)
    };
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        rewrite_forward_simple_alias_guards(out_lines)
    };
    let out_lines = rewrite_loop_index_alias_ii(out_lines);
    let out_lines = rewrite_safe_loop_index_write_calls(out_lines);
    let out_lines = rewrite_safe_loop_neighbor_read_calls(out_lines);
    let primary_flow_elapsed_ns = primary_flow_started.elapsed().as_nanos();

    let primary_inline_started = Instant::now();
    let mut primary_inline_cache = PeepholeAnalysisCache::default();
    let out_lines =
        rewrite_temp_uses_after_named_copy_with_cache(out_lines, &mut primary_inline_cache);
    let out_lines = hoist_branch_local_named_scalar_assigns_used_after_branch_with_cache(
        out_lines,
        pure_user_calls,
        &mut primary_inline_cache,
    );
    let (out_lines, _primary_immediate_profile) = run_immediate_single_use_inline_bundle_with_cache(
        out_lines,
        pure_user_calls,
        &mut primary_inline_cache,
    );
    let out_lines =
        inline_one_or_two_use_named_scalar_index_reads_within_straight_line_region_with_cache(
            out_lines,
            pure_user_calls,
            &mut primary_inline_cache,
        );
    let out_lines = inline_one_or_two_use_scalar_temps_within_straight_line_region_with_cache(
        out_lines,
        &mut primary_inline_cache,
    );
    let primary_inline_elapsed_ns = primary_inline_started.elapsed().as_nanos();

    let primary_reuse_started = Instant::now();
    let out_lines = hoist_repeated_vector_helper_calls_within_lines(out_lines);
    let out_lines = rewrite_forward_exact_vector_helper_reuse(out_lines);
    let out_lines = rewrite_forward_temp_aliases(out_lines);
    let out_lines = rewrite_forward_exact_pure_call_reuse(out_lines, pure_user_calls);
    let out_lines = rewrite_adjacent_duplicate_assignments(out_lines, pure_user_calls);
    let out_lines = collapse_trivial_dot_product_wrappers(out_lines);
    let primary_reuse_elapsed_ns = primary_reuse_started.elapsed().as_nanos();

    let primary_loop_cleanup_started = Instant::now();
    let (
        out_lines,
        line_map,
        primary_loop_dead_zero_elapsed_ns,
        primary_loop_normalize_elapsed_ns,
        primary_loop_hoist_elapsed_ns,
        primary_loop_repeat_to_for_elapsed_ns,
        primary_loop_tail_cleanup_elapsed_ns,
        primary_loop_guard_cleanup_elapsed_ns,
        primary_loop_helper_cleanup_elapsed_ns,
        primary_loop_exact_cleanup_elapsed_ns,
        primary_loop_exact_pre_elapsed_ns,
        primary_loop_exact_reuse_elapsed_ns,
        primary_loop_exact_reuse_prepare_elapsed_ns,
        primary_loop_exact_reuse_forward_elapsed_ns,
        primary_loop_exact_reuse_pure_call_elapsed_ns,
        primary_loop_exact_reuse_expr_elapsed_ns,
        primary_loop_exact_reuse_vector_alias_elapsed_ns,
        primary_loop_exact_reuse_rebind_elapsed_ns,
        primary_loop_exact_fixpoint_elapsed_ns,
        primary_loop_exact_fixpoint_prepare_elapsed_ns,
        primary_loop_exact_fixpoint_forward_elapsed_ns,
        primary_loop_exact_fixpoint_pure_call_elapsed_ns,
        primary_loop_exact_fixpoint_expr_elapsed_ns,
        primary_loop_exact_fixpoint_rebind_elapsed_ns,
        primary_loop_exact_fixpoint_rounds,
        primary_loop_exact_finalize_elapsed_ns,
        primary_loop_dead_temp_cleanup_elapsed_ns,
    ) = if fast_dev {
        let step_started = Instant::now();
        let out_lines = rewrite_dead_zero_loop_seeds_before_for(out_lines);
        let primary_loop_dead_zero_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let out_lines =
            normalize_repeat_loop_counters_with_cache(out_lines, &mut repeat_loop_cache);
        let primary_loop_normalize_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let out_lines = hoist_loop_invariant_pure_assignments_from_counted_repeat_loops_with_cache(
            out_lines,
            pure_user_calls,
            &mut repeat_loop_cache,
        );
        let primary_loop_hoist_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let out_lines = rewrite_canonical_counted_repeat_loops_to_for_with_cache(
            out_lines,
            &mut repeat_loop_cache,
        );
        let primary_loop_repeat_to_for_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let out_lines = strip_terminal_repeat_nexts(out_lines);
        let out_lines = simplify_same_var_is_na_or_not_finite_guards(out_lines);
        let out_lines = simplify_not_finite_or_zero_guard_parens(out_lines);
        let out_lines = simplify_wrapped_not_finite_parens(out_lines);
        let out_lines = run_empty_else_match_cleanup_bundle_ir(out_lines);
        let primary_loop_guard_cleanup_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let out_lines = strip_noop_self_assignments(out_lines);
        let out_lines = strip_redundant_tail_assign_slice_return(out_lines);
        let primary_loop_exact_cleanup_elapsed_ns = step_started.elapsed().as_nanos();
        let primary_loop_helper_cleanup_elapsed_ns = 0;
        let step_started = Instant::now();
        let (out_lines, line_map) =
            strip_dead_temps_with_cache(out_lines, pure_user_calls, &mut analysis_cache);
        let primary_loop_dead_temp_cleanup_elapsed_ns = step_started.elapsed().as_nanos();
        let primary_loop_tail_cleanup_elapsed_ns = primary_loop_guard_cleanup_elapsed_ns
            + primary_loop_exact_cleanup_elapsed_ns
            + primary_loop_dead_temp_cleanup_elapsed_ns;
        (
            out_lines,
            line_map,
            primary_loop_dead_zero_elapsed_ns,
            primary_loop_normalize_elapsed_ns,
            primary_loop_hoist_elapsed_ns,
            primary_loop_repeat_to_for_elapsed_ns,
            primary_loop_tail_cleanup_elapsed_ns,
            primary_loop_guard_cleanup_elapsed_ns,
            primary_loop_helper_cleanup_elapsed_ns,
            primary_loop_exact_cleanup_elapsed_ns,
            primary_loop_exact_cleanup_elapsed_ns,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            0,
            primary_loop_dead_temp_cleanup_elapsed_ns,
        )
    } else {
        let step_started = Instant::now();
        let out_lines = rewrite_dead_zero_loop_seeds_before_for(out_lines);
        let primary_loop_dead_zero_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let out_lines =
            normalize_repeat_loop_counters_with_cache(out_lines, &mut repeat_loop_cache);
        let primary_loop_normalize_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let out_lines = hoist_loop_invariant_pure_assignments_from_counted_repeat_loops_with_cache(
            out_lines,
            pure_user_calls,
            &mut repeat_loop_cache,
        );
        let primary_loop_hoist_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let out_lines = rewrite_canonical_counted_repeat_loops_to_for_with_cache(
            out_lines,
            &mut repeat_loop_cache,
        );
        let primary_loop_repeat_to_for_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let out_lines = strip_terminal_repeat_nexts(out_lines);
        let out_lines = simplify_same_var_is_na_or_not_finite_guards(out_lines);
        let out_lines = simplify_not_finite_or_zero_guard_parens(out_lines);
        let out_lines = simplify_wrapped_not_finite_parens(out_lines);
        let out_lines = run_empty_else_match_cleanup_bundle_ir(out_lines);
        let primary_loop_guard_cleanup_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let (out_lines, _primary_metric_bundle_profile) =
            run_post_passthrough_metric_bundle_ir(out_lines);
        let out_lines = collapse_inlined_copy_vec_sequences(out_lines);
        let out_lines =
            run_simple_expr_cleanup_bundle_ir(out_lines, pure_user_calls, None, !preserve_all_defs);
        let primary_loop_helper_cleanup_elapsed_ns = step_started.elapsed().as_nanos();
        let compare_exact_block = compare_exact_block_enabled();
        let exact_pre_input = compare_exact_block.then(|| out_lines.clone());
        let (
            out_lines,
            primary_loop_exact_pre_elapsed_ns,
            primary_loop_exact_reuse_prepare_elapsed_ns,
        ) = if compare_exact_block {
            let step_started = Instant::now();
            let out_lines = run_exact_pre_ir_bundle(out_lines, pure_user_calls);
            if let Some(exact_pre_input) = exact_pre_input.as_ref() {
                compare_exact_block_ir("exact_pre", exact_pre_input, &out_lines, pure_user_calls);
            }
            let exact_pre_elapsed_ns = step_started.elapsed().as_nanos();
            let step_started = Instant::now();
            let out_lines = run_exact_pre_cleanup_bundle_ir(out_lines);
            let exact_prepare_elapsed_ns = step_started.elapsed().as_nanos();
            (out_lines, exact_pre_elapsed_ns, exact_prepare_elapsed_ns)
        } else {
            let (out_lines, exact_pre_profile) =
                run_exact_pre_full_ir_bundle(out_lines, pure_user_calls);
            (
                out_lines,
                exact_pre_profile.pre_elapsed_ns,
                exact_pre_profile.cleanup_elapsed_ns,
            )
        };
        let compare_exact_reuse_steps = compare_exact_block && compare_exact_reuse_steps_enabled();
        let exact_reuse_input = compare_exact_reuse_steps.then(|| out_lines.clone());
        let (
            out_lines,
            primary_loop_exact_reuse_pure_call_elapsed_ns,
            primary_loop_exact_reuse_expr_elapsed_ns,
            primary_loop_exact_reuse_rebind_elapsed_ns,
        ) = if let (true, Some(exact_reuse_input)) =
            (compare_exact_reuse_steps, exact_reuse_input.as_ref())
        {
            let step_started = Instant::now();
            let out_lines = rewrite_forward_exact_pure_call_reuse_with_cache(
                out_lines,
                pure_user_calls,
                &mut analysis_cache,
            );
            let exact_reuse_ir_pure_call = rewrite_forward_exact_pure_call_reuse_ir(
                exact_reuse_input.clone(),
                pure_user_calls,
            );
            compare_exact_reuse_substep(
                "pure_call",
                exact_reuse_input,
                &out_lines,
                &exact_reuse_ir_pure_call,
            );
            let pure_call_elapsed_ns = step_started.elapsed().as_nanos();

            let step_started = Instant::now();
            let exact_reuse_expr_input = out_lines.clone();
            let out_lines = rewrite_forward_exact_expr_reuse(out_lines);
            let exact_reuse_ir_expr =
                rewrite_forward_exact_expr_reuse_ir(exact_reuse_expr_input.clone());
            compare_exact_reuse_substep(
                "exact_expr",
                &exact_reuse_expr_input,
                &out_lines,
                &exact_reuse_ir_expr,
            );
            let expr_elapsed_ns = step_started.elapsed().as_nanos();

            let step_started = Instant::now();
            let out_lines = strip_redundant_identical_pure_rebinds_with_cache(
                out_lines,
                pure_user_calls,
                &mut analysis_cache,
            );
            let rebind_elapsed_ns = step_started.elapsed().as_nanos();
            (
                out_lines,
                pure_call_elapsed_ns,
                expr_elapsed_ns,
                rebind_elapsed_ns,
            )
        } else {
            let (out_lines, exact_reuse_profile) =
                run_exact_reuse_ir_bundle(out_lines, pure_user_calls);
            (
                out_lines,
                exact_reuse_profile.pure_call_elapsed_ns,
                exact_reuse_profile.expr_elapsed_ns,
                exact_reuse_profile.rebind_elapsed_ns,
            )
        };
        let primary_loop_exact_reuse_forward_elapsed_ns =
            primary_loop_exact_reuse_pure_call_elapsed_ns
                + primary_loop_exact_reuse_expr_elapsed_ns;
        let step_started = Instant::now();
        let out_lines = hoist_repeated_vector_helper_calls_within_lines(out_lines);
        let out_lines = rewrite_forward_exact_vector_helper_reuse(out_lines);
        let out_lines = rewrite_forward_temp_aliases(out_lines);
        let primary_loop_exact_reuse_vector_alias_elapsed_ns = step_started.elapsed().as_nanos();
        if let Some(exact_reuse_input) = exact_reuse_input.as_ref() {
            compare_exact_block_ir(
                "exact_reuse",
                exact_reuse_input,
                &out_lines,
                pure_user_calls,
            );
        }
        let primary_loop_exact_reuse_elapsed_ns = primary_loop_exact_reuse_prepare_elapsed_ns
            + primary_loop_exact_reuse_forward_elapsed_ns
            + primary_loop_exact_reuse_vector_alias_elapsed_ns
            + primary_loop_exact_reuse_rebind_elapsed_ns;
        let step_started = Instant::now();
        let (out_lines, exact_fixpoint_profile) =
            run_exact_cleanup_fixpoint_rounds_with_profile(out_lines, pure_user_calls, 2);
        let out_lines = rewrite_shifted_square_scalar_reuse(out_lines);
        let primary_loop_exact_fixpoint_elapsed_ns = step_started.elapsed().as_nanos();
        let primary_loop_exact_fixpoint_prepare_elapsed_ns =
            exact_fixpoint_profile.prepare_elapsed_ns;
        let primary_loop_exact_fixpoint_forward_elapsed_ns =
            exact_fixpoint_profile.forward_elapsed_ns;
        let primary_loop_exact_fixpoint_pure_call_elapsed_ns =
            exact_fixpoint_profile.pure_call_elapsed_ns;
        let primary_loop_exact_fixpoint_expr_elapsed_ns = exact_fixpoint_profile.expr_elapsed_ns;
        let primary_loop_exact_fixpoint_rebind_elapsed_ns =
            exact_fixpoint_profile.rebind_elapsed_ns;
        let primary_loop_exact_fixpoint_rounds = exact_fixpoint_profile.rounds;
        let step_started = Instant::now();
        let out_lines = run_exact_finalize_cleanup_bundle_ir(out_lines);
        let primary_loop_exact_finalize_elapsed_ns = step_started.elapsed().as_nanos();
        let primary_loop_exact_cleanup_elapsed_ns = primary_loop_exact_pre_elapsed_ns
            + primary_loop_exact_reuse_elapsed_ns
            + primary_loop_exact_fixpoint_elapsed_ns
            + primary_loop_exact_finalize_elapsed_ns;
        let step_started = Instant::now();
        let (out_lines, line_map) =
            strip_dead_temps_with_cache(out_lines, pure_user_calls, &mut analysis_cache);
        let primary_loop_dead_temp_cleanup_elapsed_ns = step_started.elapsed().as_nanos();
        let primary_loop_tail_cleanup_elapsed_ns = primary_loop_guard_cleanup_elapsed_ns
            + primary_loop_helper_cleanup_elapsed_ns
            + primary_loop_exact_cleanup_elapsed_ns
            + primary_loop_dead_temp_cleanup_elapsed_ns;
        (
            out_lines,
            line_map,
            primary_loop_dead_zero_elapsed_ns,
            primary_loop_normalize_elapsed_ns,
            primary_loop_hoist_elapsed_ns,
            primary_loop_repeat_to_for_elapsed_ns,
            primary_loop_tail_cleanup_elapsed_ns,
            primary_loop_guard_cleanup_elapsed_ns,
            primary_loop_helper_cleanup_elapsed_ns,
            primary_loop_exact_cleanup_elapsed_ns,
            primary_loop_exact_pre_elapsed_ns,
            primary_loop_exact_reuse_elapsed_ns,
            primary_loop_exact_reuse_prepare_elapsed_ns,
            primary_loop_exact_reuse_forward_elapsed_ns,
            primary_loop_exact_reuse_pure_call_elapsed_ns,
            primary_loop_exact_reuse_expr_elapsed_ns,
            primary_loop_exact_reuse_vector_alias_elapsed_ns,
            primary_loop_exact_reuse_rebind_elapsed_ns,
            primary_loop_exact_fixpoint_elapsed_ns,
            primary_loop_exact_fixpoint_prepare_elapsed_ns,
            primary_loop_exact_fixpoint_forward_elapsed_ns,
            primary_loop_exact_fixpoint_pure_call_elapsed_ns,
            primary_loop_exact_fixpoint_expr_elapsed_ns,
            primary_loop_exact_fixpoint_rebind_elapsed_ns,
            primary_loop_exact_fixpoint_rounds,
            primary_loop_exact_finalize_elapsed_ns,
            primary_loop_dead_temp_cleanup_elapsed_ns,
        )
    };
    let primary_loop_cleanup_elapsed_ns = primary_loop_cleanup_started.elapsed().as_nanos();
    let primary_rewrite_elapsed_ns = primary_started.elapsed().as_nanos();
    if fast_dev {
        let finalize_started = Instant::now();
        let mut out = out_lines.join("\n");
        if code.ends_with('\n') {
            out.push('\n');
        }
        let finalize_elapsed_ns = finalize_started.elapsed().as_nanos();
        return (
            (out, line_map),
            PeepholeProfile {
                linear_scan_elapsed_ns,
                primary_rewrite_elapsed_ns,
                primary_flow_elapsed_ns,
                primary_inline_elapsed_ns,
                primary_reuse_elapsed_ns,
                primary_loop_cleanup_elapsed_ns,
                primary_loop_dead_zero_elapsed_ns,
                primary_loop_normalize_elapsed_ns,
                primary_loop_hoist_elapsed_ns,
                primary_loop_repeat_to_for_elapsed_ns,
                primary_loop_tail_cleanup_elapsed_ns,
                primary_loop_guard_cleanup_elapsed_ns,
                primary_loop_helper_cleanup_elapsed_ns,
                primary_loop_exact_cleanup_elapsed_ns,
                primary_loop_exact_pre_elapsed_ns,
                primary_loop_exact_reuse_elapsed_ns,
                primary_loop_exact_reuse_prepare_elapsed_ns,
                primary_loop_exact_reuse_forward_elapsed_ns,
                primary_loop_exact_reuse_pure_call_elapsed_ns,
                primary_loop_exact_reuse_expr_elapsed_ns,
                primary_loop_exact_reuse_vector_alias_elapsed_ns,
                primary_loop_exact_reuse_rebind_elapsed_ns,
                primary_loop_exact_fixpoint_elapsed_ns,
                primary_loop_exact_fixpoint_prepare_elapsed_ns,
                primary_loop_exact_fixpoint_forward_elapsed_ns,
                primary_loop_exact_fixpoint_pure_call_elapsed_ns,
                primary_loop_exact_fixpoint_expr_elapsed_ns,
                primary_loop_exact_fixpoint_rebind_elapsed_ns,
                primary_loop_exact_fixpoint_rounds,
                primary_loop_exact_finalize_elapsed_ns,
                primary_loop_dead_temp_cleanup_elapsed_ns,
                secondary_rewrite_elapsed_ns: 0,
                secondary_inline_elapsed_ns: 0,
                secondary_inline_branch_hoist_elapsed_ns: 0,
                secondary_inline_immediate_scalar_elapsed_ns: 0,
                secondary_inline_named_index_elapsed_ns: 0,
                secondary_inline_named_expr_elapsed_ns: 0,
                secondary_inline_scalar_region_elapsed_ns: 0,
                secondary_inline_immediate_index_elapsed_ns: 0,
                secondary_inline_adjacent_dedup_elapsed_ns: 0,
                secondary_exact_elapsed_ns: 0,
                secondary_helper_cleanup_elapsed_ns: 0,
                secondary_helper_wrapper_elapsed_ns: 0,
                secondary_helper_metric_elapsed_ns: 0,
                secondary_helper_alias_elapsed_ns: 0,
                secondary_helper_simple_expr_elapsed_ns: 0,
                secondary_helper_full_range_elapsed_ns: 0,
                secondary_helper_named_copy_elapsed_ns: 0,
                secondary_finalize_cleanup_elapsed_ns: 0,
                secondary_finalize_bundle_elapsed_ns: 0,
                secondary_finalize_dead_temp_elapsed_ns: 0,
                secondary_finalize_dead_temp_facts_elapsed_ns: 0,
                secondary_finalize_dead_temp_mark_elapsed_ns: 0,
                secondary_finalize_dead_temp_reverse_elapsed_ns: 0,
                secondary_finalize_dead_temp_compact_elapsed_ns: 0,
                finalize_elapsed_ns,
            },
        );
    }
    let secondary_started = Instant::now();
    let mut secondary_inline_cache = PeepholeAnalysisCache::default();
    let step_started = Instant::now();
    let out_lines = hoist_branch_local_named_scalar_assigns_used_after_branch_with_cache(
        out_lines,
        pure_user_calls,
        &mut secondary_inline_cache,
    );
    let secondary_inline_branch_hoist_elapsed_ns = step_started.elapsed().as_nanos();
    let step_started = Instant::now();
    let (out_lines, immediate_inline_profile) = run_immediate_single_use_inline_bundle_with_cache(
        out_lines,
        pure_user_calls,
        &mut secondary_inline_cache,
    );
    let secondary_inline_immediate_bundle_elapsed_ns = step_started.elapsed().as_nanos();
    let secondary_inline_immediate_scalar_elapsed_ns =
        immediate_inline_profile.immediate_scalar_elapsed_ns;
    let (out_lines, secondary_straight_line_profile) =
        run_named_index_scalar_region_inline_bundle_with_cache(
            out_lines,
            pure_user_calls,
            &mut secondary_inline_cache,
        );
    let secondary_inline_named_index_elapsed_ns =
        secondary_straight_line_profile.named_index_elapsed_ns;
    let secondary_inline_named_expr_elapsed_ns = immediate_inline_profile.named_expr_elapsed_ns;
    let secondary_inline_scalar_region_elapsed_ns =
        secondary_straight_line_profile.scalar_region_elapsed_ns;
    let secondary_inline_immediate_index_elapsed_ns =
        immediate_inline_profile.immediate_index_elapsed_ns;
    let step_started = Instant::now();
    let out_lines = rewrite_adjacent_duplicate_assignments(out_lines, pure_user_calls);
    let secondary_inline_adjacent_dedup_elapsed_ns = step_started.elapsed().as_nanos();
    let secondary_inline_elapsed_ns = secondary_inline_branch_hoist_elapsed_ns
        + secondary_inline_immediate_bundle_elapsed_ns
        + secondary_inline_named_index_elapsed_ns
        + secondary_inline_scalar_region_elapsed_ns
        + secondary_inline_adjacent_dedup_elapsed_ns;
    let step_started = Instant::now();
    let out_lines = run_secondary_exact_bundle_ir(out_lines);
    let secondary_exact_elapsed_ns = step_started.elapsed().as_nanos();
    let (out_lines, secondary_helper_ir_profile) =
        run_secondary_helper_ir_bundle(out_lines, pure_user_calls);
    let secondary_helper_wrapper_elapsed_ns = secondary_helper_ir_profile.post_wrapper_elapsed_ns;
    let secondary_helper_metric_elapsed_ns = secondary_helper_ir_profile.metric_elapsed_ns;
    let secondary_helper_alias_elapsed_ns = secondary_helper_ir_profile.alias_elapsed_ns;
    let secondary_helper_simple_expr_elapsed_ns = secondary_helper_ir_profile
        .simple_expr_elapsed_ns
        + secondary_helper_ir_profile.tail_elapsed_ns;
    let step_started = Instant::now();
    let out_lines = if preserve_all_defs {
        out_lines
    } else {
        run_secondary_full_range_bundle(out_lines, direct_builtin_call_map)
    };
    let secondary_helper_full_range_elapsed_ns = step_started.elapsed().as_nanos();
    let secondary_helper_named_copy_elapsed_ns = 0;
    let secondary_helper_cleanup_elapsed_ns = secondary_helper_wrapper_elapsed_ns
        + secondary_helper_metric_elapsed_ns
        + secondary_helper_alias_elapsed_ns
        + secondary_helper_simple_expr_elapsed_ns
        + secondary_helper_full_range_elapsed_ns
        + secondary_helper_named_copy_elapsed_ns;
    let step_started = Instant::now();
    let out_lines = crate::compiler::pipeline::rewrite_static_record_scalarization_lines(out_lines);
    let secondary_record_sroa_elapsed_ns = step_started.elapsed().as_nanos();
    let secondary_helper_cleanup_elapsed_ns =
        secondary_helper_cleanup_elapsed_ns + secondary_record_sroa_elapsed_ns;
    let step_started = Instant::now();
    let out_lines = run_secondary_empty_else_finalize_bundle_ir(out_lines, preserve_all_defs);
    let secondary_finalize_bundle_elapsed_ns = step_started.elapsed().as_nanos();
    let step_started = Instant::now();
    let ((out_lines, final_compact_map), secondary_dead_temp_profile) =
        strip_dead_temps_with_cache_and_profile(out_lines, pure_user_calls, &mut analysis_cache);
    let secondary_finalize_dead_temp_elapsed_ns = step_started.elapsed().as_nanos();
    let secondary_finalize_cleanup_elapsed_ns =
        secondary_finalize_bundle_elapsed_ns + secondary_finalize_dead_temp_elapsed_ns;
    let secondary_rewrite_elapsed_ns = secondary_started.elapsed().as_nanos();
    let finalize_started = Instant::now();
    let line_map = compose_line_maps(&line_map, &final_compact_map);
    let mut out = out_lines.join("\n");
    if code.ends_with('\n') {
        out.push('\n');
    }
    out = crate::compiler::pipeline::repair_missing_cse_range_aliases_in_raw_emitted_r(&out);
    let finalize_elapsed_ns = finalize_started.elapsed().as_nanos();
    (
        (out, line_map),
        PeepholeProfile {
            linear_scan_elapsed_ns,
            primary_rewrite_elapsed_ns,
            primary_flow_elapsed_ns,
            primary_inline_elapsed_ns,
            primary_reuse_elapsed_ns,
            primary_loop_cleanup_elapsed_ns,
            primary_loop_dead_zero_elapsed_ns,
            primary_loop_normalize_elapsed_ns,
            primary_loop_hoist_elapsed_ns,
            primary_loop_repeat_to_for_elapsed_ns,
            primary_loop_tail_cleanup_elapsed_ns,
            primary_loop_guard_cleanup_elapsed_ns,
            primary_loop_helper_cleanup_elapsed_ns,
            primary_loop_exact_cleanup_elapsed_ns,
            primary_loop_exact_pre_elapsed_ns,
            primary_loop_exact_reuse_elapsed_ns,
            primary_loop_exact_reuse_prepare_elapsed_ns,
            primary_loop_exact_reuse_forward_elapsed_ns,
            primary_loop_exact_reuse_pure_call_elapsed_ns,
            primary_loop_exact_reuse_expr_elapsed_ns,
            primary_loop_exact_reuse_vector_alias_elapsed_ns,
            primary_loop_exact_reuse_rebind_elapsed_ns,
            primary_loop_exact_fixpoint_elapsed_ns,
            primary_loop_exact_fixpoint_prepare_elapsed_ns,
            primary_loop_exact_fixpoint_forward_elapsed_ns,
            primary_loop_exact_fixpoint_pure_call_elapsed_ns,
            primary_loop_exact_fixpoint_expr_elapsed_ns,
            primary_loop_exact_fixpoint_rebind_elapsed_ns,
            primary_loop_exact_fixpoint_rounds,
            primary_loop_exact_finalize_elapsed_ns,
            primary_loop_dead_temp_cleanup_elapsed_ns,
            secondary_rewrite_elapsed_ns,
            secondary_inline_elapsed_ns,
            secondary_inline_branch_hoist_elapsed_ns,
            secondary_inline_immediate_scalar_elapsed_ns,
            secondary_inline_named_index_elapsed_ns,
            secondary_inline_named_expr_elapsed_ns,
            secondary_inline_scalar_region_elapsed_ns,
            secondary_inline_immediate_index_elapsed_ns,
            secondary_inline_adjacent_dedup_elapsed_ns,
            secondary_exact_elapsed_ns,
            secondary_helper_cleanup_elapsed_ns,
            secondary_helper_wrapper_elapsed_ns,
            secondary_helper_metric_elapsed_ns,
            secondary_helper_alias_elapsed_ns,
            secondary_helper_simple_expr_elapsed_ns,
            secondary_helper_full_range_elapsed_ns,
            secondary_helper_named_copy_elapsed_ns,
            secondary_finalize_cleanup_elapsed_ns,
            secondary_finalize_bundle_elapsed_ns,
            secondary_finalize_dead_temp_elapsed_ns,
            secondary_finalize_dead_temp_facts_elapsed_ns: secondary_dead_temp_profile
                .facts_elapsed_ns,
            secondary_finalize_dead_temp_mark_elapsed_ns: secondary_dead_temp_profile
                .mark_elapsed_ns,
            secondary_finalize_dead_temp_reverse_elapsed_ns: secondary_dead_temp_profile
                .reverse_elapsed_ns,
            secondary_finalize_dead_temp_compact_elapsed_ns: secondary_dead_temp_profile
                .compact_elapsed_ns,
            finalize_elapsed_ns,
        },
    )
}
