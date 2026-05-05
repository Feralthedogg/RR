use super::*;
pub(crate) fn strip_unreachable_sym_helpers_ir(lines: Vec<String>) -> Vec<String> {
    if !has_unreachable_sym_helper_candidates_ir(&lines) {
        return lines;
    }
    let mut program = EmittedProgram::parse(&lines);
    apply_strip_unreachable_sym_helpers_ir(&mut program);
    program.into_lines()
}
