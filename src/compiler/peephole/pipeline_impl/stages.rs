use super::*;
use crate::compiler::peephole::stage_catalog::{PeepholePassManager, PeepholeStageId};
use std::time::Instant;

#[path = "debug_compare.rs"]
pub(crate) mod debug_compare;

use self::debug_compare::*;

#[derive(Clone, Copy)]
pub(crate) struct PrimaryFlowStageConfig {
    pub(crate) direct_builtin_call_map: bool,
    pub(crate) preserve_all_defs: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct PrimaryReuseStageConfig {
    pub(crate) aggressive_o3: bool,
    pub(crate) expression_controlled: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct HelperCleanupPolicy {
    pub(crate) preserve_all_defs: bool,
    pub(crate) size_controlled_simple_expr: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct SecondaryHelperCleanupConfig {
    pub(crate) direct_builtin_call_map: bool,
    pub(crate) preserve_all_defs: bool,
    pub(crate) size_controlled_simple_expr: bool,
}

#[derive(Clone, Copy)]
pub(crate) struct SecondaryFinalizeCleanupConfig {
    pub(crate) preserve_all_defs: bool,
    pub(crate) aggressive_o3: bool,
    pub(crate) expression_controlled: bool,
}

pub(crate) fn run_primary_flow_stage(
    pass_manager: &PeepholePassManager,
    mut lines: Vec<String>,
    config: PrimaryFlowStageConfig,
) -> PeepholeLineStageOutcome {
    let (lines, elapsed_ns) = pass_manager.run(PeepholeStageId::PrimaryFlow, || {
        lines = collapse_common_if_else_tail_assignments(lines);
        lines = rewrite_full_range_conditional_scalar_loops(lines);
        if !config.preserve_all_defs {
            lines = rewrite_inline_full_range_slice_ops(lines, config.direct_builtin_call_map);
            lines = rewrite_one_based_full_range_index_alias_reads(lines);
            lines = rewrite_forward_simple_alias_guards(lines);
        }
        lines = rewrite_loop_index_alias_ii(lines);
        lines = rewrite_safe_loop_index_write_calls(lines);
        rewrite_safe_loop_neighbor_read_calls(lines)
    });
    PeepholeLineStageOutcome { lines, elapsed_ns }
}

pub(crate) fn run_primary_inline_stage(
    pass_manager: &PeepholePassManager,
    mut lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> PeepholeLineStageOutcome {
    let (lines, elapsed_ns) = pass_manager.run(PeepholeStageId::PrimaryInline, || {
        let mut cache = PeepholeAnalysisCache::default();
        lines = rewrite_temp_uses_after_named_copy_with_cache(lines, &mut cache);
        lines = hoist_branch_local_named_scalar_assigns_used_after_branch_with_cache(
            lines,
            pure_user_calls,
            &mut cache,
        );
        let (next_lines, _primary_immediate_profile) =
            run_immediate_single_use_inline_bundle_with_cache(lines, pure_user_calls, &mut cache);
        lines =
            inline_one_or_two_use_named_scalar_index_reads_within_straight_line_region_with_cache(
                next_lines,
                pure_user_calls,
                &mut cache,
            );
        inline_one_or_two_use_scalar_temps_within_straight_line_region_with_cache(lines, &mut cache)
    });
    PeepholeLineStageOutcome { lines, elapsed_ns }
}

pub(crate) fn run_primary_reuse_stage(
    pass_manager: &PeepholePassManager,
    mut lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    config: PrimaryReuseStageConfig,
) -> PeepholeLineStageOutcome {
    let (lines, elapsed_ns) = pass_manager.run(PeepholeStageId::PrimaryReuse, || {
        lines = hoist_repeated_vector_helper_calls_within_lines(lines);
        lines = rewrite_forward_exact_vector_helper_reuse(lines);
        if config.aggressive_o3 {
            lines = hoist_aggressive_repeated_vector_helper_calls_within_lines(lines);
            lines = hoist_o3_repeated_index_vec_floor_calls_across_lines(lines);
            lines = hoist_loop_invariant_indexed_gathers(lines);
            lines = inline_o3_leaf_kernel_helpers(lines);
            lines = rewrite_forward_exact_vector_helper_reuse_aggressive(lines);
        }
        if config.expression_controlled {
            lines = materialize_o3_semantic_gather_subexpressions(lines);
            lines = materialize_o3_large_ifelse_branches(lines);
            lines = materialize_o3_large_repeated_arithmetic_subexpressions(lines);
            lines = materialize_loop_indexed_vector_helper_calls(lines, pure_user_calls);
            lines = semanticize_o3_vector_cse_temp_names(lines);
            lines = materialize_o3_exact_gather_index_temps(lines);
        }
        lines = rewrite_forward_temp_aliases(lines);
        lines = rewrite_forward_exact_pure_call_reuse(lines, pure_user_calls);
        lines = rewrite_adjacent_duplicate_assignments(lines, pure_user_calls);
        collapse_trivial_dot_product_wrappers(lines)
    });
    PeepholeLineStageOutcome { lines, elapsed_ns }
}

pub(crate) fn inline_o3_leaf_kernel_helpers(lines: Vec<String>) -> Vec<String> {
    const O3_INLINE_HELPERS: [&str; 3] = ["Sym_244", "Sym_156", "Sym_171"];
    if !lines.iter().any(|line| {
        O3_INLINE_HELPERS
            .iter()
            .any(|helper| line.contains(&format!("{helper}(")))
    }) {
        return lines;
    }
    let original = lines.join("\n");
    let mut code = original.clone();
    for helper in O3_INLINE_HELPERS {
        if !code.contains(&format!("{helper}(")) {
            continue;
        }
        let rewritten = rewrite_selected_simple_expr_helper_calls_in_text(&code, &[helper]);
        if rewritten == code {
            continue;
        }
        if o3_leaf_inline_within_budget_for_helper(helper, &code, &rewritten) {
            code = rewritten;
        }
    }
    if code == original {
        return lines;
    }
    code.lines().map(str::to_string).collect()
}

pub(crate) fn o3_leaf_inline_within_budget_for_helper(
    helper: &str,
    original: &str,
    rewritten: &str,
) -> bool {
    let growth = rewritten.len().saturating_sub(original.len());
    let (growth_env, growth_default, line_env, line_default, depth_env, depth_default) =
        if helper == "Sym_244" {
            (
                "RR_O3_LEAF_INLINE_MAX_GROWTH_BYTES",
                2048,
                "RR_O3_LEAF_INLINE_MAX_LINE_CHARS",
                1200,
                "RR_O3_LEAF_INLINE_MAX_PAREN_DEPTH",
                16,
            )
        } else {
            (
                "RR_O3_KERNEL_INLINE_MAX_GROWTH_BYTES",
                12288,
                "RR_O3_KERNEL_INLINE_MAX_LINE_CHARS",
                2400,
                "RR_O3_KERNEL_INLINE_MAX_PAREN_DEPTH",
                28,
            )
        };
    let max_growth = o3_leaf_inline_env_usize(growth_env, growth_default);
    if growth > max_growth {
        return false;
    }

    let max_line = rewritten.lines().map(str::len).max().unwrap_or(0);
    let max_line_limit = o3_leaf_inline_env_usize(line_env, line_default);
    if max_line > max_line_limit {
        return false;
    }

    let max_depth = o3_leaf_inline_env_usize(depth_env, depth_default);
    rewritten
        .lines()
        .all(|line| o3_leaf_inline_paren_depth(line) <= max_depth)
}

pub(crate) fn o3_leaf_inline_env_usize(name: &str, default: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|raw| raw.trim().parse::<usize>().ok())
        .unwrap_or(default)
}

pub(crate) fn o3_leaf_inline_paren_depth(expr: &str) -> usize {
    let mut depth = 0usize;
    let mut max_depth = 0usize;
    let mut in_single = false;
    let mut in_double = false;
    for ch in expr.chars() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '(' if !in_single && !in_double => {
                depth = depth.saturating_add(1);
                max_depth = max_depth.max(depth);
            }
            ')' if !in_single && !in_double => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    max_depth
}

pub(crate) fn join_peephole_lines(lines: Vec<String>, preserve_trailing_newline: bool) -> String {
    let mut out = lines.join("\n");
    if preserve_trailing_newline {
        out.push('\n');
    }
    out
}

pub(crate) fn run_timed_peephole_stage<T>(
    pass_manager: &PeepholePassManager,
    id: PeepholeStageId,
    run: impl FnOnce() -> T,
) -> (T, u128) {
    pass_manager.run(id, run)
}

pub(crate) fn run_fast_dev_finalize_stage(
    pass_manager: &PeepholePassManager,
    lines: Vec<String>,
    code: &str,
) -> (String, u128) {
    run_timed_peephole_stage(pass_manager, PeepholeStageId::Finalize, || {
        join_peephole_lines(lines, code.ends_with('\n'))
    })
}

pub(crate) fn run_standard_finalize_stage(
    pass_manager: &PeepholePassManager,
    lines: Vec<String>,
    primary_line_map: Vec<u32>,
    final_compact_map: &[u32],
    code: &str,
) -> (String, Vec<u32>, u128) {
    let ((out, line_map), elapsed_ns) =
        run_timed_peephole_stage(pass_manager, PeepholeStageId::Finalize, || {
            let line_map = compose_line_maps(&primary_line_map, final_compact_map);
            let mut out = join_peephole_lines(lines, code.ends_with('\n'));
            out =
                crate::compiler::pipeline::repair_missing_cse_range_aliases_in_raw_emitted_r(&out);
            (out, line_map)
        });
    (out, line_map, elapsed_ns)
}

#[derive(Clone, Copy)]
pub(crate) struct PrimaryLoopCleanupOptions {
    pub(crate) fast_dev: bool,
    pub(crate) preserve_all_defs: bool,
    pub(crate) size_controlled_simple_expr: bool,
}

pub(crate) fn run_primary_loop_cleanup_stage(
    pass_manager: &PeepholePassManager,
    lines: Vec<String>,
    options: PrimaryLoopCleanupOptions,
    pure_user_calls: &FxHashSet<String>,
    analysis_cache: &mut PeepholeAnalysisCache,
    repeat_loop_cache: &mut RepeatLoopAnalysisCache,
) -> PrimaryLoopCleanupStageOutcome {
    let (mut outcome, elapsed_ns) = pass_manager.run(PeepholeStageId::PrimaryLoopCleanup, || {
        if options.fast_dev {
            run_fast_dev_primary_loop_cleanup_stage(
                lines,
                pure_user_calls,
                analysis_cache,
                repeat_loop_cache,
            )
        } else {
            run_standard_primary_loop_cleanup_stage(
                lines,
                options,
                pure_user_calls,
                analysis_cache,
                repeat_loop_cache,
            )
        }
    });
    outcome.elapsed_ns = elapsed_ns;
    outcome
}

pub(crate) fn run_loop_canonicalization_substage(
    mut lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    repeat_loop_cache: &mut RepeatLoopAnalysisCache,
) -> LoopCanonicalizationSubstageOutcome {
    let step_started = Instant::now();
    lines = rewrite_dead_zero_loop_seeds_before_for(lines);
    let dead_zero_elapsed_ns = step_started.elapsed().as_nanos();

    let step_started = Instant::now();
    lines = normalize_repeat_loop_counters_with_cache(lines, repeat_loop_cache);
    let normalize_elapsed_ns = step_started.elapsed().as_nanos();

    let step_started = Instant::now();
    lines = hoist_loop_invariant_pure_assignments_from_counted_repeat_loops_with_cache(
        lines,
        pure_user_calls,
        repeat_loop_cache,
    );
    let hoist_elapsed_ns = step_started.elapsed().as_nanos();

    let step_started = Instant::now();
    lines = rewrite_canonical_counted_repeat_loops_to_for_with_cache(lines, repeat_loop_cache);
    let repeat_to_for_elapsed_ns = step_started.elapsed().as_nanos();

    LoopCanonicalizationSubstageOutcome {
        lines,
        dead_zero_elapsed_ns,
        normalize_elapsed_ns,
        hoist_elapsed_ns,
        repeat_to_for_elapsed_ns,
    }
}

pub(crate) fn run_primary_guard_cleanup_substage(
    mut lines: Vec<String>,
) -> PeepholeLineStageOutcome {
    let started = Instant::now();
    lines = strip_terminal_repeat_nexts(lines);
    lines = simplify_same_var_is_na_or_not_finite_guards(lines);
    lines = simplify_not_finite_or_zero_guard_parens(lines);
    lines = simplify_wrapped_not_finite_parens(lines);
    lines = run_empty_else_match_cleanup_bundle_ir(lines);
    PeepholeLineStageOutcome {
        lines,
        elapsed_ns: started.elapsed().as_nanos(),
    }
}

pub(crate) fn run_fast_primary_exact_cleanup_substage(
    mut lines: Vec<String>,
) -> PeepholeLineStageOutcome {
    let started = Instant::now();
    lines = strip_noop_self_assignments(lines);
    lines = strip_redundant_tail_assign_slice_return(lines);
    PeepholeLineStageOutcome {
        lines,
        elapsed_ns: started.elapsed().as_nanos(),
    }
}

pub(crate) fn run_primary_dead_temp_cleanup_substage(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    analysis_cache: &mut PeepholeAnalysisCache,
) -> (Vec<String>, Vec<u32>, u128) {
    let started = Instant::now();
    let (lines, line_map) = strip_dead_temps_with_cache(lines, pure_user_calls, analysis_cache);
    (lines, line_map, started.elapsed().as_nanos())
}

pub(crate) fn run_standard_primary_helper_cleanup_substage(
    lines: Vec<String>,
    policy: HelperCleanupPolicy,
    pure_user_calls: &FxHashSet<String>,
) -> PeepholeLineStageOutcome {
    let started = Instant::now();
    let (lines, _primary_metric_bundle_profile) = run_post_passthrough_metric_bundle_ir(lines);
    let lines = collapse_inlined_copy_vec_sequences(lines);
    let lines = run_simple_expr_cleanup_bundle_ir(
        lines,
        pure_user_calls,
        SimpleExprCleanupConfig {
            allowed_helpers: None,
            rewrite_full_range_alias_reads: !policy.preserve_all_defs,
            size_controlled: policy.size_controlled_simple_expr,
        },
    );
    PeepholeLineStageOutcome {
        lines,
        elapsed_ns: started.elapsed().as_nanos(),
    }
}

pub(crate) fn run_primary_exact_pre_substage(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> PrimaryExactPreSubstageOutcome {
    let compare_exact_block = compare_exact_block_enabled();
    let exact_pre_input = compare_exact_block.then(|| lines.clone());
    let (lines, pre_elapsed_ns, reuse_prepare_elapsed_ns) = if compare_exact_block {
        let step_started = Instant::now();
        let lines = run_exact_pre_ir_bundle(lines, pure_user_calls);
        if let Some(exact_pre_input) = exact_pre_input.as_ref() {
            compare_exact_block_ir("exact_pre", exact_pre_input, &lines, pure_user_calls);
        }
        let pre_elapsed_ns = step_started.elapsed().as_nanos();
        let step_started = Instant::now();
        let lines = run_exact_pre_cleanup_bundle_ir(lines);
        let prepare_elapsed_ns = step_started.elapsed().as_nanos();
        (lines, pre_elapsed_ns, prepare_elapsed_ns)
    } else {
        let (lines, exact_pre_profile) = run_exact_pre_full_ir_bundle(lines, pure_user_calls);
        (
            lines,
            exact_pre_profile.pre_elapsed_ns,
            exact_pre_profile.cleanup_elapsed_ns,
        )
    };

    PrimaryExactPreSubstageOutcome {
        lines,
        pre_elapsed_ns,
        reuse_prepare_elapsed_ns,
    }
}

pub(crate) fn run_primary_exact_reuse_substage(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    analysis_cache: &mut PeepholeAnalysisCache,
) -> PrimaryExactReuseSubstageOutcome {
    let compare_exact_block = compare_exact_block_enabled();
    let compare_exact_reuse_steps = compare_exact_block && compare_exact_reuse_steps_enabled();
    let exact_reuse_input = compare_exact_reuse_steps.then(|| lines.clone());
    let (mut lines, pure_call_elapsed_ns, expr_elapsed_ns, rebind_elapsed_ns) =
        if let (true, Some(exact_reuse_input)) =
            (compare_exact_reuse_steps, exact_reuse_input.as_ref())
        {
            let step_started = Instant::now();
            let lines = rewrite_forward_exact_pure_call_reuse_with_cache(
                lines,
                pure_user_calls,
                analysis_cache,
            );
            let exact_reuse_ir_pure_call = rewrite_forward_exact_pure_call_reuse_ir(
                exact_reuse_input.clone(),
                pure_user_calls,
            );
            compare_exact_reuse_substep(
                "pure_call",
                exact_reuse_input,
                &lines,
                &exact_reuse_ir_pure_call,
            );
            let pure_call_elapsed_ns = step_started.elapsed().as_nanos();

            let step_started = Instant::now();
            let exact_reuse_expr_input = lines.clone();
            let lines = rewrite_forward_exact_expr_reuse(lines);
            let exact_reuse_ir_expr =
                rewrite_forward_exact_expr_reuse_ir(exact_reuse_expr_input.clone());
            compare_exact_reuse_substep(
                "exact_expr",
                &exact_reuse_expr_input,
                &lines,
                &exact_reuse_ir_expr,
            );
            let expr_elapsed_ns = step_started.elapsed().as_nanos();

            let step_started = Instant::now();
            let lines = strip_redundant_identical_pure_rebinds_with_cache(
                lines,
                pure_user_calls,
                analysis_cache,
            );
            let rebind_elapsed_ns = step_started.elapsed().as_nanos();
            (
                lines,
                pure_call_elapsed_ns,
                expr_elapsed_ns,
                rebind_elapsed_ns,
            )
        } else {
            let (lines, exact_reuse_profile) = run_exact_reuse_ir_bundle(lines, pure_user_calls);
            (
                lines,
                exact_reuse_profile.pure_call_elapsed_ns,
                exact_reuse_profile.expr_elapsed_ns,
                exact_reuse_profile.rebind_elapsed_ns,
            )
        };
    let forward_elapsed_ns = pure_call_elapsed_ns + expr_elapsed_ns;

    let step_started = Instant::now();
    lines = hoist_repeated_vector_helper_calls_within_lines(lines);
    lines = rewrite_forward_exact_vector_helper_reuse(lines);
    lines = rewrite_forward_temp_aliases(lines);
    let vector_alias_elapsed_ns = step_started.elapsed().as_nanos();
    if let Some(exact_reuse_input) = exact_reuse_input.as_ref() {
        compare_exact_block_ir("exact_reuse", exact_reuse_input, &lines, pure_user_calls);
    }

    PrimaryExactReuseSubstageOutcome {
        lines,
        elapsed_ns: forward_elapsed_ns + vector_alias_elapsed_ns + rebind_elapsed_ns,
        prepare_elapsed_ns: 0,
        forward_elapsed_ns,
        pure_call_elapsed_ns,
        expr_elapsed_ns,
        vector_alias_elapsed_ns,
        rebind_elapsed_ns,
    }
}

pub(crate) fn run_fast_dev_primary_loop_cleanup_stage(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
    analysis_cache: &mut PeepholeAnalysisCache,
    repeat_loop_cache: &mut RepeatLoopAnalysisCache,
) -> PrimaryLoopCleanupStageOutcome {
    let canonical = run_loop_canonicalization_substage(lines, pure_user_calls, repeat_loop_cache);
    let guard = run_primary_guard_cleanup_substage(canonical.lines);
    let exact = run_fast_primary_exact_cleanup_substage(guard.lines);
    let (lines, line_map, dead_temp_cleanup_elapsed_ns) =
        run_primary_dead_temp_cleanup_substage(exact.lines, pure_user_calls, analysis_cache);
    let tail_cleanup_elapsed_ns =
        guard.elapsed_ns + exact.elapsed_ns + dead_temp_cleanup_elapsed_ns;

    PrimaryLoopCleanupStageOutcome {
        lines,
        line_map,
        elapsed_ns: 0,
        dead_zero_elapsed_ns: canonical.dead_zero_elapsed_ns,
        normalize_elapsed_ns: canonical.normalize_elapsed_ns,
        hoist_elapsed_ns: canonical.hoist_elapsed_ns,
        repeat_to_for_elapsed_ns: canonical.repeat_to_for_elapsed_ns,
        tail_cleanup_elapsed_ns,
        guard_cleanup_elapsed_ns: guard.elapsed_ns,
        helper_cleanup_elapsed_ns: 0,
        exact_cleanup_elapsed_ns: exact.elapsed_ns,
        exact_pre_elapsed_ns: exact.elapsed_ns,
        exact_reuse_elapsed_ns: 0,
        exact_reuse_prepare_elapsed_ns: 0,
        exact_reuse_forward_elapsed_ns: 0,
        exact_reuse_pure_call_elapsed_ns: 0,
        exact_reuse_expr_elapsed_ns: 0,
        exact_reuse_vector_alias_elapsed_ns: 0,
        exact_reuse_rebind_elapsed_ns: 0,
        exact_fixpoint_elapsed_ns: 0,
        exact_fixpoint_prepare_elapsed_ns: 0,
        exact_fixpoint_forward_elapsed_ns: 0,
        exact_fixpoint_pure_call_elapsed_ns: 0,
        exact_fixpoint_expr_elapsed_ns: 0,
        exact_fixpoint_rebind_elapsed_ns: 0,
        exact_fixpoint_rounds: 0,
        exact_finalize_elapsed_ns: 0,
        dead_temp_cleanup_elapsed_ns,
    }
}

pub(crate) fn run_standard_primary_loop_cleanup_stage(
    lines: Vec<String>,
    options: PrimaryLoopCleanupOptions,
    pure_user_calls: &FxHashSet<String>,
    analysis_cache: &mut PeepholeAnalysisCache,
    repeat_loop_cache: &mut RepeatLoopAnalysisCache,
) -> PrimaryLoopCleanupStageOutcome {
    let canonical = run_loop_canonicalization_substage(lines, pure_user_calls, repeat_loop_cache);
    let guard = run_primary_guard_cleanup_substage(canonical.lines);
    let helper = run_standard_primary_helper_cleanup_substage(
        guard.lines,
        HelperCleanupPolicy {
            preserve_all_defs: options.preserve_all_defs,
            size_controlled_simple_expr: options.size_controlled_simple_expr,
        },
        pure_user_calls,
    );
    let exact_pre = run_primary_exact_pre_substage(helper.lines, pure_user_calls);
    let exact_reuse =
        run_primary_exact_reuse_substage(exact_pre.lines, pure_user_calls, analysis_cache);
    let exact_reuse_prepare_elapsed_ns =
        exact_pre.reuse_prepare_elapsed_ns + exact_reuse.prepare_elapsed_ns;
    let exact_reuse_elapsed_ns = exact_reuse_prepare_elapsed_ns + exact_reuse.elapsed_ns;

    let step_started = Instant::now();
    let (lines, exact_fixpoint_profile) =
        run_exact_cleanup_fixpoint_rounds_with_profile(exact_reuse.lines, pure_user_calls, 2);
    let mut lines = rewrite_shifted_square_scalar_reuse(lines);
    let exact_fixpoint_elapsed_ns = step_started.elapsed().as_nanos();

    let step_started = Instant::now();
    lines = run_exact_finalize_cleanup_bundle_ir(lines);
    let exact_finalize_elapsed_ns = step_started.elapsed().as_nanos();

    let exact_cleanup_elapsed_ns = exact_pre.pre_elapsed_ns
        + exact_reuse_elapsed_ns
        + exact_fixpoint_elapsed_ns
        + exact_finalize_elapsed_ns;

    let (lines, line_map, dead_temp_cleanup_elapsed_ns) =
        run_primary_dead_temp_cleanup_substage(lines, pure_user_calls, analysis_cache);
    let tail_cleanup_elapsed_ns = guard.elapsed_ns
        + helper.elapsed_ns
        + exact_cleanup_elapsed_ns
        + dead_temp_cleanup_elapsed_ns;

    PrimaryLoopCleanupStageOutcome {
        lines,
        line_map,
        elapsed_ns: 0,
        dead_zero_elapsed_ns: canonical.dead_zero_elapsed_ns,
        normalize_elapsed_ns: canonical.normalize_elapsed_ns,
        hoist_elapsed_ns: canonical.hoist_elapsed_ns,
        repeat_to_for_elapsed_ns: canonical.repeat_to_for_elapsed_ns,
        tail_cleanup_elapsed_ns,
        guard_cleanup_elapsed_ns: guard.elapsed_ns,
        helper_cleanup_elapsed_ns: helper.elapsed_ns,
        exact_cleanup_elapsed_ns,
        exact_pre_elapsed_ns: exact_pre.pre_elapsed_ns,
        exact_reuse_elapsed_ns,
        exact_reuse_prepare_elapsed_ns,
        exact_reuse_forward_elapsed_ns: exact_reuse.forward_elapsed_ns,
        exact_reuse_pure_call_elapsed_ns: exact_reuse.pure_call_elapsed_ns,
        exact_reuse_expr_elapsed_ns: exact_reuse.expr_elapsed_ns,
        exact_reuse_vector_alias_elapsed_ns: exact_reuse.vector_alias_elapsed_ns,
        exact_reuse_rebind_elapsed_ns: exact_reuse.rebind_elapsed_ns,
        exact_fixpoint_elapsed_ns,
        exact_fixpoint_prepare_elapsed_ns: exact_fixpoint_profile.prepare_elapsed_ns,
        exact_fixpoint_forward_elapsed_ns: exact_fixpoint_profile.forward_elapsed_ns,
        exact_fixpoint_pure_call_elapsed_ns: exact_fixpoint_profile.pure_call_elapsed_ns,
        exact_fixpoint_expr_elapsed_ns: exact_fixpoint_profile.expr_elapsed_ns,
        exact_fixpoint_rebind_elapsed_ns: exact_fixpoint_profile.rebind_elapsed_ns,
        exact_fixpoint_rounds: exact_fixpoint_profile.rounds,
        exact_finalize_elapsed_ns,
        dead_temp_cleanup_elapsed_ns,
    }
}

pub(crate) fn run_secondary_inline_stage(
    pass_manager: &PeepholePassManager,
    mut lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> SecondaryInlineStageOutcome {
    let (outcome, _stage_elapsed_ns) = pass_manager.run(PeepholeStageId::SecondaryInline, || {
        let mut cache = PeepholeAnalysisCache::default();

        let step_started = Instant::now();
        lines = hoist_branch_local_named_scalar_assigns_used_after_branch_with_cache(
            lines,
            pure_user_calls,
            &mut cache,
        );
        let branch_hoist_elapsed_ns = step_started.elapsed().as_nanos();

        let step_started = Instant::now();
        let (next_lines, immediate_inline_profile) =
            run_immediate_single_use_inline_bundle_with_cache(lines, pure_user_calls, &mut cache);
        let immediate_bundle_elapsed_ns = step_started.elapsed().as_nanos();
        let immediate_scalar_elapsed_ns = immediate_inline_profile.immediate_scalar_elapsed_ns;
        let named_expr_elapsed_ns = immediate_inline_profile.named_expr_elapsed_ns;
        let immediate_index_elapsed_ns = immediate_inline_profile.immediate_index_elapsed_ns;

        let (mut lines, straight_line_profile) =
            run_named_index_scalar_region_inline_bundle_with_cache(
                next_lines,
                pure_user_calls,
                &mut cache,
            );
        let named_index_elapsed_ns = straight_line_profile.named_index_elapsed_ns;
        let scalar_region_elapsed_ns = straight_line_profile.scalar_region_elapsed_ns;

        let step_started = Instant::now();
        lines = rewrite_adjacent_duplicate_assignments(lines, pure_user_calls);
        let adjacent_dedup_elapsed_ns = step_started.elapsed().as_nanos();

        let substep_elapsed_ns = branch_hoist_elapsed_ns
            + immediate_bundle_elapsed_ns
            + named_index_elapsed_ns
            + scalar_region_elapsed_ns
            + adjacent_dedup_elapsed_ns;

        SecondaryInlineStageOutcome {
            lines,
            elapsed_ns: substep_elapsed_ns,
            branch_hoist_elapsed_ns,
            immediate_scalar_elapsed_ns,
            named_index_elapsed_ns,
            named_expr_elapsed_ns,
            scalar_region_elapsed_ns,
            immediate_index_elapsed_ns,
            adjacent_dedup_elapsed_ns,
        }
    });
    outcome
}

pub(crate) fn run_secondary_exact_stage(
    pass_manager: &PeepholePassManager,
    lines: Vec<String>,
) -> PeepholeLineStageOutcome {
    let (lines, elapsed_ns) =
        run_timed_peephole_stage(pass_manager, PeepholeStageId::SecondaryExact, || {
            run_secondary_exact_bundle_ir(lines)
        });
    PeepholeLineStageOutcome { lines, elapsed_ns }
}

pub(crate) fn run_secondary_helper_cleanup_stage(
    pass_manager: &PeepholePassManager,
    lines: Vec<String>,
    config: SecondaryHelperCleanupConfig,
    pure_user_calls: &FxHashSet<String>,
) -> SecondaryHelperCleanupStageOutcome {
    let (helper_outcome, _helper_stage_elapsed_ns) =
        pass_manager.run(PeepholeStageId::SecondaryHelperCleanup, || {
            let (mut lines, helper_ir_profile) = run_secondary_helper_ir_bundle(
                lines,
                pure_user_calls,
                config.size_controlled_simple_expr,
            );
            let wrapper_elapsed_ns = helper_ir_profile.post_wrapper_elapsed_ns;
            let metric_elapsed_ns = helper_ir_profile.metric_elapsed_ns;
            let alias_elapsed_ns = helper_ir_profile.alias_elapsed_ns;
            let simple_expr_elapsed_ns =
                helper_ir_profile.simple_expr_elapsed_ns + helper_ir_profile.tail_elapsed_ns;

            let step_started = Instant::now();
            if !config.preserve_all_defs {
                lines = run_secondary_full_range_bundle(lines, config.direct_builtin_call_map);
            }
            let full_range_elapsed_ns = step_started.elapsed().as_nanos();

            let named_copy_elapsed_ns = 0;
            let profiled_helper_elapsed_ns = wrapper_elapsed_ns
                + metric_elapsed_ns
                + alias_elapsed_ns
                + simple_expr_elapsed_ns
                + full_range_elapsed_ns
                + named_copy_elapsed_ns;

            SecondaryHelperCleanupStageOutcome {
                lines,
                elapsed_ns: profiled_helper_elapsed_ns,
                wrapper_elapsed_ns,
                metric_elapsed_ns,
                alias_elapsed_ns,
                simple_expr_elapsed_ns,
                full_range_elapsed_ns,
                named_copy_elapsed_ns,
                record_sroa_elapsed_ns: 0,
            }
        });
    let (lines, record_sroa_elapsed_ns) =
        run_timed_peephole_stage(pass_manager, PeepholeStageId::SecondaryRecordSroa, || {
            crate::compiler::pipeline::rewrite_static_record_scalarization_lines(
                helper_outcome.lines,
            )
        });

    SecondaryHelperCleanupStageOutcome {
        lines,
        elapsed_ns: helper_outcome.elapsed_ns + record_sroa_elapsed_ns,
        wrapper_elapsed_ns: helper_outcome.wrapper_elapsed_ns,
        metric_elapsed_ns: helper_outcome.metric_elapsed_ns,
        alias_elapsed_ns: helper_outcome.alias_elapsed_ns,
        simple_expr_elapsed_ns: helper_outcome.simple_expr_elapsed_ns,
        full_range_elapsed_ns: helper_outcome.full_range_elapsed_ns,
        named_copy_elapsed_ns: helper_outcome.named_copy_elapsed_ns,
        record_sroa_elapsed_ns,
    }
}

pub(crate) fn run_secondary_finalize_cleanup_stage(
    pass_manager: &PeepholePassManager,
    mut lines: Vec<String>,
    config: SecondaryFinalizeCleanupConfig,
    pure_user_calls: &FxHashSet<String>,
    analysis_cache: &mut PeepholeAnalysisCache,
) -> SecondaryFinalizeCleanupStageOutcome {
    let (outcome, _stage_elapsed_ns) =
        pass_manager.run(PeepholeStageId::SecondaryFinalizeCleanup, || {
            let step_started = Instant::now();
            lines = run_secondary_empty_else_finalize_bundle_ir(lines, config.preserve_all_defs);
            if config.aggressive_o3 {
                lines = hoist_aggressive_repeated_vector_helper_calls_within_lines(lines);
                lines = hoist_o3_repeated_index_vec_floor_calls_across_lines(lines);
                lines = hoist_loop_invariant_indexed_gathers(lines);
                lines = inline_o3_leaf_kernel_helpers(lines);
                lines = rewrite_forward_exact_vector_helper_reuse_aggressive(lines);
            }
            if config.expression_controlled {
                lines = materialize_o3_semantic_gather_subexpressions(lines);
                lines = materialize_o3_large_ifelse_branches(lines);
                lines = materialize_o3_large_repeated_arithmetic_subexpressions(lines);
                lines = materialize_loop_indexed_vector_helper_calls(lines, pure_user_calls);
                lines = semanticize_o3_vector_cse_temp_names(lines);
                lines = materialize_o3_exact_gather_index_temps(lines);
            }
            let bundle_elapsed_ns = step_started.elapsed().as_nanos();

            let step_started = Instant::now();
            let (lines_after_mark_strip, mark_strip_map) =
                if config.aggressive_o3 && !config.preserve_all_defs {
                    strip_debug_mark_lines_with_map(lines)
                } else {
                    let line_map = (1..=lines.len() as u32).collect::<Vec<_>>();
                    (lines, line_map)
                };
            lines = lines_after_mark_strip;
            let index_hoist_map = if config.aggressive_o3 {
                let (gather_lines, gather_map) =
                    hoist_aggressive_repeated_vector_helper_calls_within_lines_with_map(lines);
                let (next_lines, index_map) =
                    hoist_o3_repeated_index_vec_floor_calls_across_lines_with_map(gather_lines);
                let semantic_lines = semanticize_o3_vector_cse_temp_names(next_lines);
                let (repaired_lines, repair_map) =
                    define_missing_o3_semantic_index_temps_with_map(semantic_lines);
                lines = repaired_lines;
                let index_repair_map = compose_line_maps(&index_map, &repair_map);
                compose_line_maps(&gather_map, &index_repair_map)
            } else {
                (1..=lines.len() as u32).collect::<Vec<_>>()
            };
            lines = mark_unused_tachyon_exprmap_temps(lines);
            let ((lines, dead_temp_map), dead_temp_profile) =
                strip_dead_temps_with_cache_and_profile(lines, pure_user_calls, analysis_cache);
            let post_mark_compact_map = compose_line_maps(&index_hoist_map, &dead_temp_map);
            let final_compact_map = compose_line_maps(&mark_strip_map, &post_mark_compact_map);
            let dead_temp_elapsed_ns = step_started.elapsed().as_nanos();
            let substep_elapsed_ns = bundle_elapsed_ns + dead_temp_elapsed_ns;

            SecondaryFinalizeCleanupStageOutcome {
                lines,
                final_compact_map,
                elapsed_ns: substep_elapsed_ns,
                bundle_elapsed_ns,
                dead_temp_elapsed_ns,
                dead_temp_facts_elapsed_ns: dead_temp_profile.facts_elapsed_ns,
                dead_temp_mark_elapsed_ns: dead_temp_profile.mark_elapsed_ns,
                dead_temp_reverse_elapsed_ns: dead_temp_profile.reverse_elapsed_ns,
                dead_temp_compact_elapsed_ns: dead_temp_profile.compact_elapsed_ns,
            }
        });
    outcome
}

pub(crate) fn strip_debug_mark_lines_with_map(lines: Vec<String>) -> (Vec<String>, Vec<u32>) {
    if !lines
        .iter()
        .any(|line| line.trim_start().starts_with("rr_mark("))
    {
        let line_map = (1..=lines.len() as u32).collect::<Vec<_>>();
        return (lines, line_map);
    }

    let mut compacted = Vec::with_capacity(lines.len());
    let mut line_map = vec![0u32; lines.len()];
    let mut new_line = 0u32;
    for (idx, line) in lines.into_iter().enumerate() {
        let trimmed = line.trim();
        let is_mark =
            trimmed.starts_with("rr_mark(") && (trimmed.ends_with(");") || trimmed.ends_with(')'));
        if is_mark {
            line_map[idx] = new_line.max(1);
            continue;
        }
        new_line += 1;
        line_map[idx] = new_line;
        compacted.push(line);
    }
    (compacted, line_map)
}

pub(crate) fn run_exact_cleanup_fixpoint_rounds_with_profile(
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

pub(crate) fn optimize_emitted_r_pipeline_impl(
    code: &str,
    pure_user_calls: &FxHashSet<String>,
    fresh_user_calls: &FxHashSet<String>,
    options: PeepholeOptions,
) -> (String, Vec<u32>) {
    optimize_emitted_r_pipeline_impl_with_profile(code, pure_user_calls, fresh_user_calls, options)
        .0
}
