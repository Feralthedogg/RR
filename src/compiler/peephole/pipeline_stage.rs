use super::*;

#[allow(dead_code)]
pub(super) struct EmittedROptimizationConfig<'a> {
    pub(super) direct_builtin_call_map: bool,
    pub(super) pure_user_calls: &'a FxHashSet<String>,
    pub(super) fresh_user_calls: &'a FxHashSet<String>,
    pub(super) reusable_pure_user_calls: &'a FxHashSet<String>,
    pub(super) preserve_all_defs: bool,
}

#[allow(dead_code)]
pub(super) fn run_initial_emitted_r_rewrite_pass(
    code: &str,
    cfg: &EmittedROptimizationConfig<'_>,
) -> Vec<String> {
    let direct_builtin_call_map = cfg.direct_builtin_call_map;
    let fresh_user_calls = cfg.fresh_user_calls;
    let reusable_pure_user_calls = cfg.reusable_pure_user_calls;
    let preserve_all_defs = cfg.preserve_all_defs;

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
                extract_pure_call_binding(lhs, &rewritten_rhs, reusable_pure_user_calls)
        {
            pure_call_bindings.push(binding);
        }
        last_rhs_for_var.insert(lhs.to_string(), rewritten_rhs.clone());

        out_lines.push(format!("{indent}{lhs} <- {rewritten_rhs}"));
        if opens_conditional {
            conditional_depth += 1;
        }
    }

    out_lines
}

#[allow(dead_code)]
pub(super) fn run_post_linear_peephole_passes(
    out_lines: Vec<String>,
    _cfg: &EmittedROptimizationConfig<'_>,
) -> (Vec<String>, Vec<u32>) {
    (out_lines, Vec::new())
}

pub(super) fn compose_line_maps(first: &[u32], second: &[u32]) -> Vec<u32> {
    first
        .iter()
        .map(|line| {
            if *line == 0 {
                return 0;
            }
            let idx = (*line as usize).saturating_sub(1);
            second.get(idx).copied().unwrap_or(*line)
        })
        .collect()
}

pub(super) fn run_exact_expr_cleanup_rounds(
    mut lines: Vec<String>,
    max_rounds: usize,
) -> Vec<String> {
    for _ in 0..max_rounds {
        let before = lines.clone();
        lines = run_secondary_exact_expr_bundle_ir(lines);
        lines = rewrite_temp_minus_one_scaled_to_named_scalar(lines);
        if lines == before {
            break;
        }
    }
    lines
}
