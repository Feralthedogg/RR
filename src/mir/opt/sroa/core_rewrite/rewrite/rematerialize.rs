use super::*;
pub(crate) fn rematerialize_aggregate_boundaries(
    fn_ir: &mut FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
) -> bool {
    let (shapes, _) = infer_candidate_shapes(fn_ir);
    let live_values = collect_non_alias_live_value_ids(fn_ir);
    let mut materialized = FxHashMap::default();
    let mut changed = false;

    let concrete_base_rewrites: Vec<_> = fn_ir
        .values
        .iter()
        .filter(|value| live_values.contains(&value.id))
        .filter_map(|value| match &value.kind {
            ValueKind::Len { base }
            | ValueKind::Indices { base }
            | ValueKind::Index1D { base, .. }
            | ValueKind::Index2D { base, .. }
            | ValueKind::Index3D { base, .. }
                if should_rematerialize_boundary_value(fn_ir, field_maps, *base) =>
            {
                Some((value.id, *base))
            }
            _ => None,
        })
        .collect();
    for (consumer, old_base) in concrete_base_rewrites {
        let Some(replacement) =
            rematerialize_value(fn_ir, field_maps, &shapes, &mut materialized, old_base)
        else {
            continue;
        };
        changed |=
            rewrite_concrete_base_consumer(&mut fn_ir.values[consumer].kind, old_base, replacement);
    }

    let record_field_rewrites: Vec<_> = fn_ir
        .values
        .iter()
        .filter(|value| live_values.contains(&value.id))
        .filter_map(|value| {
            let ValueKind::RecordLit { fields } = &value.kind else {
                return None;
            };
            let rewrites: Vec<_> = fields
                .iter()
                .enumerate()
                .filter_map(|(field_index, (_, field_value))| {
                    should_rematerialize_boundary_value(fn_ir, field_maps, *field_value)
                        .then_some((field_index, *field_value))
                })
                .collect();
            (!rewrites.is_empty()).then_some((value.id, rewrites))
        })
        .collect();
    for (record, rewrites) in record_field_rewrites {
        for (field_index, field_value) in rewrites {
            let Some(replacement) =
                rematerialize_value(fn_ir, field_maps, &shapes, &mut materialized, field_value)
            else {
                continue;
            };
            if let ValueKind::RecordLit { fields } = &mut fn_ir.values[record].kind
                && fields
                    .get(field_index)
                    .map(|(_, current)| *current == field_value)
                    .unwrap_or(false)
            {
                fields[field_index].1 = replacement;
                changed = true;
            }
        }
    }

    let field_set_base_rewrites: Vec<_> = fn_ir
        .values
        .iter()
        .filter(|value| live_values.contains(&value.id))
        .filter_map(|value| match &value.kind {
            ValueKind::FieldSet { base, .. }
                if should_rematerialize_boundary_value(fn_ir, field_maps, *base) =>
            {
                Some((value.id, *base))
            }
            _ => None,
        })
        .collect();
    for (field_set, field_base) in field_set_base_rewrites {
        let Some(replacement) =
            rematerialize_value(fn_ir, field_maps, &shapes, &mut materialized, field_base)
        else {
            continue;
        };
        if let ValueKind::FieldSet { base, .. } = &mut fn_ir.values[field_set].kind
            && *base == field_base
        {
            *base = replacement;
            changed = true;
        }
    }

    let field_set_value_rewrites: Vec<_> = fn_ir
        .values
        .iter()
        .filter(|value| live_values.contains(&value.id))
        .filter_map(|value| match &value.kind {
            ValueKind::FieldSet {
                value: field_value, ..
            } if should_rematerialize_boundary_value(fn_ir, field_maps, *field_value) => {
                Some((value.id, *field_value))
            }
            _ => None,
        })
        .collect();
    for (field_set, field_value) in field_set_value_rewrites {
        let Some(replacement) =
            rematerialize_value(fn_ir, field_maps, &shapes, &mut materialized, field_value)
        else {
            continue;
        };
        if let ValueKind::FieldSet { value, .. } = &mut fn_ir.values[field_set].kind
            && *value == field_value
        {
            *value = replacement;
            changed = true;
        }
    }

    let eval_rewrites: Vec<_> = fn_ir
        .blocks
        .iter()
        .flat_map(|block| {
            block
                .instrs
                .iter()
                .enumerate()
                .filter_map(|(instr_index, instr)| match instr {
                    Instr::Eval { val, .. }
                        if should_rematerialize_boundary_value(fn_ir, field_maps, *val) =>
                    {
                        Some((block.id, instr_index, *val))
                    }
                    _ => None,
                })
        })
        .collect();
    for (block, instr_index, value) in eval_rewrites {
        let Some(replacement) =
            rematerialize_value(fn_ir, field_maps, &shapes, &mut materialized, value)
        else {
            continue;
        };
        if let Some(Instr::Eval { val, .. }) = fn_ir.blocks[block].instrs.get_mut(instr_index)
            && *val == value
        {
            *val = replacement;
            changed = true;
        }
    }

    let mut store_rewrites = Vec::new();
    for block in &fn_ir.blocks {
        for (instr_index, instr) in block.instrs.iter().enumerate() {
            for (operand, value) in store_index_operands(instr) {
                if should_rematerialize_boundary_value(fn_ir, field_maps, value) {
                    store_rewrites.push((block.id, instr_index, operand, value));
                }
            }
        }
    }
    for (block, instr_index, operand, value) in store_rewrites {
        let Some(replacement) =
            rematerialize_value(fn_ir, field_maps, &shapes, &mut materialized, value)
        else {
            continue;
        };
        if let Some(instr) = fn_ir.blocks[block].instrs.get_mut(instr_index) {
            changed |= rewrite_store_index_operand(instr, operand, value, replacement);
        }
    }

    let return_rewrites: Vec<_> = fn_ir
        .blocks
        .iter()
        .filter_map(|block| match block.term {
            Terminator::Return(Some(value))
                if should_rematerialize_boundary_value(fn_ir, field_maps, value) =>
            {
                Some((block.id, value))
            }
            _ => None,
        })
        .collect();
    for (block, value) in return_rewrites {
        let Some(replacement) =
            rematerialize_value(fn_ir, field_maps, &shapes, &mut materialized, value)
        else {
            continue;
        };
        if let Terminator::Return(Some(ret)) = &mut fn_ir.blocks[block].term
            && *ret == value
        {
            *ret = replacement;
            changed = true;
        }
    }

    let call_rewrites: Vec<_> = fn_ir
        .values
        .iter()
        .filter(|value| live_values.contains(&value.id))
        .filter_map(|value| {
            let ValueKind::Call { args, .. } = &value.kind else {
                return None;
            };
            let rewrites: Vec<_> = args
                .iter()
                .enumerate()
                .filter_map(|(arg_index, arg)| {
                    should_rematerialize_boundary_value(fn_ir, field_maps, *arg)
                        .then_some((arg_index, *arg))
                })
                .collect();
            (!rewrites.is_empty()).then_some((value.id, rewrites))
        })
        .collect();
    for (call, rewrites) in call_rewrites {
        for (arg_index, arg) in rewrites {
            let Some(replacement) =
                rematerialize_value(fn_ir, field_maps, &shapes, &mut materialized, arg)
            else {
                continue;
            };
            if let ValueKind::Call { args, .. } = &mut fn_ir.values[call].kind
                && args.get(arg_index).copied() == Some(arg)
            {
                args[arg_index] = replacement;
                changed = true;
            }
        }
    }

    let intrinsic_rewrites: Vec<_> = fn_ir
        .values
        .iter()
        .filter(|value| live_values.contains(&value.id))
        .filter_map(|value| {
            let ValueKind::Intrinsic { args, .. } = &value.kind else {
                return None;
            };
            let rewrites: Vec<_> = args
                .iter()
                .enumerate()
                .filter_map(|(arg_index, arg)| {
                    should_rematerialize_boundary_value(fn_ir, field_maps, *arg)
                        .then_some((arg_index, *arg))
                })
                .collect();
            (!rewrites.is_empty()).then_some((value.id, rewrites))
        })
        .collect();
    for (intrinsic, rewrites) in intrinsic_rewrites {
        for (arg_index, arg) in rewrites {
            let Some(replacement) =
                rematerialize_value(fn_ir, field_maps, &shapes, &mut materialized, arg)
            else {
                continue;
            };
            if let ValueKind::Intrinsic { args, .. } = &mut fn_ir.values[intrinsic].kind
                && args.get(arg_index).copied() == Some(arg)
            {
                args[arg_index] = replacement;
                changed = true;
            }
        }
    }

    changed
}
