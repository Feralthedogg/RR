use super::*;
pub(crate) fn insert_scalarized_return_alias_temp(
    caller: &mut FnIR,
    record_use: &RecordReturnFieldUse,
    specialized_name: &str,
) -> Option<String> {
    let alias_var = record_use.alias_var.as_ref()?;
    let (block, instr_index) = find_record_return_alias_assignment(caller, record_use)?;
    let span = caller.values.get(record_use.field_get)?.span;
    let temp_var = unique_sroa_return_temp_var(caller, alias_var, &record_use.field);
    let scalar_call = caller.add_value(
        ValueKind::Call {
            callee: specialized_name.to_string(),
            args: record_use.args.clone(),
            names: record_use.names.clone(),
        },
        span,
        Facts::empty(),
        None,
    );
    caller.set_call_semantics(scalar_call, CallSemantics::UserDefined);
    caller.blocks[block].instrs.insert(
        instr_index + 1,
        Instr::Assign {
            dst: temp_var.clone(),
            src: scalar_call,
            span,
        },
    );
    Some(temp_var)
}

pub(crate) fn find_record_return_alias_assignment(
    fn_ir: &FnIR,
    record_use: &RecordReturnFieldUse,
) -> Option<(BlockId, usize)> {
    let alias_var = record_use.alias_var.as_ref()?;
    for block in &fn_ir.blocks {
        for (instr_index, instr) in block.instrs.iter().enumerate() {
            let Instr::Assign { dst, src, .. } = instr else {
                continue;
            };
            if dst != alias_var {
                continue;
            }
            let Some((callee, args, names)) = call_parts(fn_ir, *src) else {
                continue;
            };
            if callee == record_use.callee && args == record_use.args && names == record_use.names {
                return Some((block.id, instr_index));
            }
        }
    }
    None
}

pub(crate) fn remove_scalarized_return_aliases(fn_ir: &mut FnIR, vars: &FxHashSet<String>) -> bool {
    let mut changed = false;
    for value in &mut fn_ir.values {
        if let ValueKind::Load { var } = &value.kind
            && vars.contains(var)
        {
            value.kind = ValueKind::Const(Lit::Null);
            value.origin_var = None;
            changed = true;
        }
    }

    for block in &mut fn_ir.blocks {
        let old_len = block.instrs.len();
        block
            .instrs
            .retain(|instr| !matches!(instr, Instr::Assign { dst, .. } if vars.contains(dst)));
        changed |= block.instrs.len() != old_len;
    }

    changed
}
