pub(in super::super) fn run_secondary_finalize_cleanup_bundle_ir(
    lines: Vec<String>,
    preserve_all_defs: bool,
) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_tail_assign = has_tail_assign_slice_return_candidates_ir(&lines);
    let needs_unreachable = !preserve_all_defs && has_unreachable_sym_helper_candidates_ir(&lines);
    if !needs_dead_eval && !needs_noop_assign && !needs_tail_assign && !needs_unreachable {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_secondary_finalize_cleanup_bundle_ir(
        &mut program,
        needs_dead_eval,
        needs_noop_assign,
        needs_unreachable,
        needs_tail_assign,
    );
    program.into_lines()
}

fn apply_secondary_finalize_cleanup_bundle_ir(
    program: &mut EmittedProgram,
    needs_dead_eval: bool,
    needs_noop_assign: bool,
    needs_unreachable: bool,
    needs_tail_assign: bool,
) {
    if needs_dead_eval {
        apply_strip_dead_simple_eval_lines_ir(program);
    }
    if needs_noop_assign {
        apply_strip_noop_self_assignments_ir(program);
    }
    if needs_unreachable {
        apply_strip_unreachable_sym_helpers_ir(program);
    }
    if needs_tail_assign {
        apply_strip_redundant_tail_assign_slice_return_ir(program);
    }
}

pub(in super::super) fn run_secondary_empty_else_finalize_bundle_ir(
    lines: Vec<String>,
    preserve_all_defs: bool,
) -> Vec<String> {
    let needs_empty_else = has_empty_else_block_candidates_ir(&lines);
    let needs_match_phi = has_restore_empty_match_single_bind_candidates_ir(&lines);
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs_dead_eval = scan.needs_dead_eval;
    let needs_noop_assign = scan.needs_noop_assign;
    let needs_tail_assign = has_tail_assign_slice_return_candidates_ir(&lines);
    let needs_unreachable = !preserve_all_defs && has_unreachable_sym_helper_candidates_ir(&lines);
    if !needs_empty_else
        && !needs_match_phi
        && !needs_dead_eval
        && !needs_noop_assign
        && !needs_tail_assign
        && !needs_unreachable
    {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_match_phi {
        apply_restore_empty_match_single_bind_arms_ir(&mut program);
    }
    if needs_empty_else {
        apply_strip_empty_else_blocks_ir(&mut program);
    }
    apply_secondary_finalize_cleanup_bundle_ir(
        &mut program,
        needs_dead_eval,
        needs_noop_assign,
        needs_unreachable,
        needs_tail_assign,
    );
    program.into_lines()
}
