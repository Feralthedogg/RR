use super::*;
pub(crate) fn clone_scalarizable_callee_value(
    caller: &mut FnIR,
    callee: &FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    args: &[ValueId],
    value: ValueId,
    value_map: &mut FxHashMap<ValueId, ValueId>,
    visiting: &mut FxHashSet<String>,
) -> Option<ValueId> {
    if let Some(mapped) = value_map.get(&value).copied() {
        return Some(mapped);
    }

    let source = callee.values.get(value)?.clone();
    let cloned_kind = match source.kind {
        ValueKind::Const(lit) => ValueKind::Const(lit),
        ValueKind::Param { index } => {
            let mapped = args.get(index).copied()?;
            value_map.insert(value, mapped);
            return Some(mapped);
        }
        ValueKind::Binary { op, lhs, rhs } => ValueKind::Binary {
            op,
            lhs: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, lhs, value_map, visiting,
            )?,
            rhs: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, rhs, value_map, visiting,
            )?,
        },
        ValueKind::Unary { op, rhs } => ValueKind::Unary {
            op,
            rhs: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, rhs, value_map, visiting,
            )?,
        },
        ValueKind::Len { base } => ValueKind::Len {
            base: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, base, value_map, visiting,
            )?,
        },
        ValueKind::Indices { base } => ValueKind::Indices {
            base: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, base, value_map, visiting,
            )?,
        },
        ValueKind::Range { start, end } => ValueKind::Range {
            start: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, start, value_map, visiting,
            )?,
            end: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, end, value_map, visiting,
            )?,
        },
        ValueKind::RecordLit { fields } => {
            let mut cloned_fields = Vec::with_capacity(fields.len());
            for (field, field_value) in fields {
                cloned_fields.push((
                    field,
                    clone_scalarizable_callee_value(
                        caller,
                        callee,
                        all_fns,
                        args,
                        field_value,
                        value_map,
                        visiting,
                    )?,
                ));
            }
            ValueKind::RecordLit {
                fields: cloned_fields,
            }
        }
        ValueKind::FieldGet { base, field } => {
            let cloned_base = clone_scalarizable_callee_value(
                caller, callee, all_fns, args, base, value_map, visiting,
            )?;
            if let Some(projected) = scalarized_field_projection_from_value(
                caller,
                all_fns,
                cloned_base,
                &field,
                visiting,
            ) {
                value_map.insert(value, projected);
                return Some(projected);
            }
            ValueKind::FieldGet {
                base: cloned_base,
                field,
            }
        }
        ValueKind::FieldSet { base, field, value } => ValueKind::FieldSet {
            base: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, base, value_map, visiting,
            )?,
            field,
            value: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, value, value_map, visiting,
            )?,
        },
        ValueKind::Intrinsic { op, args: inputs } => {
            let mut cloned_args = Vec::with_capacity(inputs.len());
            for input in inputs {
                cloned_args.push(clone_scalarizable_callee_value(
                    caller, callee, all_fns, args, input, value_map, visiting,
                )?);
            }
            ValueKind::Intrinsic {
                op,
                args: cloned_args,
            }
        }
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => ValueKind::Index1D {
            base: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, base, value_map, visiting,
            )?,
            idx: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, idx, value_map, visiting,
            )?,
            is_safe,
            is_na_safe,
        },
        ValueKind::Index2D { base, r, c } => ValueKind::Index2D {
            base: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, base, value_map, visiting,
            )?,
            r: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, r, value_map, visiting,
            )?,
            c: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, c, value_map, visiting,
            )?,
        },
        ValueKind::Index3D { base, i, j, k } => ValueKind::Index3D {
            base: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, base, value_map, visiting,
            )?,
            i: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, i, value_map, visiting,
            )?,
            j: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, j, value_map, visiting,
            )?,
            k: clone_scalarizable_callee_value(
                caller, callee, all_fns, args, k, value_map, visiting,
            )?,
        },
        ValueKind::Call {
            callee: call_callee,
            args: inputs,
            names,
        } => {
            let mut cloned_args = Vec::with_capacity(inputs.len());
            for input in inputs {
                cloned_args.push(clone_scalarizable_callee_value(
                    caller, callee, all_fns, args, input, value_map, visiting,
                )?);
            }
            ValueKind::Call {
                callee: call_callee,
                args: cloned_args,
                names,
            }
        }
        ValueKind::RSymbol { name } => ValueKind::RSymbol { name },
        ValueKind::Phi { .. } | ValueKind::Load { .. } => return None,
    };

    let cloned = caller.add_value(cloned_kind, source.span, source.facts, None);
    caller.values[cloned].value_ty = source.value_ty;
    caller.values[cloned].value_term = source.value_term;
    caller.values[cloned].escape = source.escape;
    if let ValueKind::Call { callee, .. } = &caller.values[cloned].kind
        && all_fns.contains_key(callee)
    {
        caller.set_call_semantics(cloned, CallSemantics::UserDefined);
    }
    value_map.insert(value, cloned);
    Some(cloned)
}
