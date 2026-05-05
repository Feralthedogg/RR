use super::*;
pub(crate) fn has_exact_expr_candidates_ir(lines: &[String]) -> bool {
    lines.iter().any(|line| {
        assign_re()
            .and_then(|re| re.captures(line.trim()))
            .and_then(|caps| caps.name("rhs").map(|m| m.as_str()))
            .is_some_and(expr_is_exact_reusable_scalar)
    })
}

#[derive(Default)]
pub(crate) struct ExactPreBundleProfile {
    pub(crate) pre_elapsed_ns: u128,
    pub(crate) cleanup_elapsed_ns: u128,
}

pub(crate) fn run_exact_pre_full_ir_bundle(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> (Vec<String>, ExactPreBundleProfile) {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_nested_temp = scan.needs_nested_temp;
    let needs_exact_expr = has_exact_expr_candidates_ir(&lines);
    if !needs_exact_expr && !needs_dead_eval && !needs_noop_assign && !needs_nested_temp {
        return (lines, ExactPreBundleProfile::default());
    }

    let mut profile = ExactPreBundleProfile::default();
    let mut program = EmittedProgram::parse(&lines);

    let started = std::time::Instant::now();
    if needs_exact_expr {
        apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
        apply_strip_redundant_identical_pure_rebinds_ir(&mut program, pure_user_calls);
    }
    profile.pre_elapsed_ns = started.elapsed().as_nanos();

    let started = std::time::Instant::now();
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    if needs_nested_temp {
        apply_strip_redundant_nested_temp_reassigns_ir(&mut program);
    }
    profile.cleanup_elapsed_ns = started.elapsed().as_nanos();

    (program.into_lines(), profile)
}

pub(crate) fn run_secondary_exact_expr_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let needs_exact_expr = has_exact_expr_candidates_ir(&lines);
    let needs_noop_assign = scan_basic_cleanup_candidates_ir(&lines).needs_noop_assign;
    if !needs_exact_expr && !needs_noop_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_exact_expr {
        apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    program.into_lines()
}

pub(crate) fn run_secondary_exact_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let needs_dead_zero = has_dead_zero_loop_seed_candidates_ir(&lines);
    let needs_terminal_next = has_terminal_repeat_next_candidates_ir(&lines);
    let needs_exact_expr = has_exact_expr_candidates_ir(&lines);
    let needs_noop_assign = scan_basic_cleanup_candidates_ir(&lines).needs_noop_assign;
    if !needs_dead_zero && !needs_terminal_next && !needs_exact_expr && !needs_noop_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_dead_zero {
        apply_rewrite_dead_zero_loop_seeds_before_for_ir(&mut program);
    }
    if needs_terminal_next {
        apply_strip_terminal_repeat_nexts_ir(&mut program);
    }
    if needs_exact_expr {
        apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    run_secondary_exact_local_scalar_bundle(program.into_lines())
}

pub(crate) fn run_exact_finalize_cleanup_bundle_ir(lines: Vec<String>) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_nested_temp = scan.needs_nested_temp;
    let needs_tail_assign = has_tail_assign_slice_return_candidates_ir(&lines);
    if !needs_dead_eval && !needs_noop_assign && !needs_nested_temp && !needs_tail_assign {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(&mut program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(&mut program);
    }
    if needs_nested_temp {
        apply_strip_redundant_nested_temp_reassigns_ir(&mut program);
    }
    if needs_tail_assign {
        apply_strip_redundant_tail_assign_slice_return_ir(&mut program);
    }
    program.into_lines()
}

pub(crate) fn collapse_identical_if_else_tail_assignments_late_ir(
    lines: Vec<String>,
) -> Vec<String> {
    let mut program = EmittedProgram::parse(&lines);
    apply_collapse_identical_if_else_tail_assignments_late_ir(&mut program);
    program.into_lines()
}

#[derive(Default)]
pub(crate) struct ExactReuseBundleProfile {
    pub(crate) pure_call_elapsed_ns: u128,
    pub(crate) expr_elapsed_ns: u128,
    pub(crate) rebind_elapsed_ns: u128,
}

pub(crate) fn run_exact_reuse_ir_bundle(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> (Vec<String>, ExactReuseBundleProfile) {
    if !lines.iter().any(|line| line.contains("<-")) {
        return (lines, ExactReuseBundleProfile::default());
    }
    let mut profile = ExactReuseBundleProfile::default();
    let mut program = EmittedProgram::parse(&lines);

    let started = std::time::Instant::now();
    apply_rewrite_forward_exact_pure_call_reuse_ir(&mut program, pure_user_calls);
    profile.pure_call_elapsed_ns = started.elapsed().as_nanos();

    let started = std::time::Instant::now();
    apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
    profile.expr_elapsed_ns = started.elapsed().as_nanos();

    let started = std::time::Instant::now();
    apply_strip_redundant_identical_pure_rebinds_ir(&mut program, pure_user_calls);
    profile.rebind_elapsed_ns = started.elapsed().as_nanos();

    (program.into_lines(), profile)
}

pub(crate) fn run_exact_pre_ir_bundle(
    lines: Vec<String>,
    pure_user_calls: &FxHashSet<String>,
) -> Vec<String> {
    if !lines.iter().any(|line| line.contains("<-")) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_rewrite_forward_exact_expr_reuse_ir(&mut program);
    apply_strip_redundant_identical_pure_rebinds_ir(&mut program, pure_user_calls);
    program.into_lines()
}
