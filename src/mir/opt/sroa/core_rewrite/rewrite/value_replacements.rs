use super::*;
pub(crate) fn apply_value_replacements(
    fn_ir: &mut FnIR,
    replacements: &FxHashMap<ValueId, ValueId>,
) -> bool {
    let mut changed = false;

    for value in &mut fn_ir.values {
        changed |= rewrite_value_kind_refs(&mut value.kind, replacements);
    }

    for block in &mut fn_ir.blocks {
        for instr in &mut block.instrs {
            changed |= rewrite_instr_refs(instr, replacements);
        }
        changed |= rewrite_terminator_refs(&mut block.term, replacements);
    }

    changed
}
