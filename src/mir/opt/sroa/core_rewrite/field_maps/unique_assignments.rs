use super::*;
pub(crate) fn unique_var_assignments(fn_ir: &FnIR) -> FxHashMap<String, ValueId> {
    let mut counts: FxHashMap<String, usize> = FxHashMap::default();
    let mut sources = FxHashMap::default();
    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            if let Instr::Assign { dst, src, .. } = instr {
                *counts.entry(dst.clone()).or_default() += 1;
                sources.insert(dst.clone(), *src);
            }
        }
    }

    sources
        .into_iter()
        .filter(|(var, _)| counts.get(var).copied() == Some(1))
        .collect()
}

pub(crate) fn unique_var_assignment_instr(
    fn_ir: &FnIR,
    var: &str,
    src: ValueId,
) -> Option<(BlockId, usize)> {
    let mut found = None;
    for block in &fn_ir.blocks {
        for (instr_index, instr) in block.instrs.iter().enumerate() {
            let Instr::Assign {
                dst,
                src: assigned_src,
                ..
            } = instr
            else {
                continue;
            };
            if dst != var {
                continue;
            }
            if *assigned_src != src || found.replace((block.id, instr_index)).is_some() {
                return None;
            }
        }
    }
    found
}
