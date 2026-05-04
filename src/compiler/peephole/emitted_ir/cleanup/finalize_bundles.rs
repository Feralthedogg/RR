use super::*;
pub(crate) fn run_secondary_finalize_cleanup_bundle_ir(
    lines: Vec<String>,
    preserve_all_defs: bool,
) -> Vec<String> {
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs = SecondaryFinalizeCleanupNeeds {
        dead_eval: scan.needs_dead_eval,
        noop_assign: scan.needs_noop_assign,
        unreachable: !preserve_all_defs && has_unreachable_sym_helper_candidates_ir(&lines),
        tail_assign: has_tail_assign_slice_return_candidates_ir(&lines),
    };
    if !needs.any() {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_secondary_finalize_cleanup_bundle_ir(&mut program, needs);
    program.into_lines()
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct SecondaryFinalizeCleanupNeeds {
    pub(crate) dead_eval: bool,
    pub(crate) noop_assign: bool,
    pub(crate) unreachable: bool,
    pub(crate) tail_assign: bool,
}

impl SecondaryFinalizeCleanupNeeds {
    pub(crate) fn any(self) -> bool {
        self.dead_eval || self.noop_assign || self.unreachable || self.tail_assign
    }
}

pub(crate) fn apply_secondary_finalize_cleanup_bundle_ir(
    program: &mut EmittedProgram,
    needs: SecondaryFinalizeCleanupNeeds,
) {
    if needs.dead_eval {
        apply_strip_dead_simple_eval_lines_ir(program);
    }
    if needs.noop_assign {
        apply_strip_noop_self_assignments_ir(program);
    }
    if needs.unreachable {
        apply_strip_unreachable_sym_helpers_ir(program);
    }
    if needs.tail_assign {
        apply_strip_redundant_tail_assign_slice_return_ir(program);
    }
}

pub(crate) fn run_secondary_empty_else_finalize_bundle_ir(
    lines: Vec<String>,
    preserve_all_defs: bool,
) -> Vec<String> {
    let needs_empty_else = has_empty_else_block_candidates_ir(&lines);
    let needs_match_phi = has_restore_empty_match_single_bind_candidates_ir(&lines);
    let scan = scan_basic_cleanup_candidates_ir(&lines);
    let needs = SecondaryFinalizeCleanupNeeds {
        dead_eval: scan.needs_dead_eval,
        noop_assign: scan.needs_noop_assign,
        unreachable: !preserve_all_defs && has_unreachable_sym_helper_candidates_ir(&lines),
        tail_assign: has_tail_assign_slice_return_candidates_ir(&lines),
    };
    if !needs_empty_else && !needs_match_phi && !needs.any() {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    if needs_match_phi {
        apply_restore_empty_match_single_bind_arms_ir(&mut program);
    }
    if needs_empty_else {
        apply_strip_empty_else_blocks_ir(&mut program);
    }
    apply_secondary_finalize_cleanup_bundle_ir(&mut program, needs);
    program.into_lines()
}
