use super::*;
pub(crate) fn materialized_aggregate_values(fn_ir: &FnIR) -> FxHashSet<ValueId> {
    let unique_assignments = unique_var_assignments(fn_ir);
    let mut values = FxHashSet::default();

    for boundary in collect_materialization_boundaries(fn_ir) {
        add_materialized_value(fn_ir, &unique_assignments, &mut values, boundary.value);
    }

    values
}

pub(crate) fn collect_materialization_boundaries(fn_ir: &FnIR) -> Vec<SroaMaterializationBoundary> {
    let live_values = collect_non_alias_live_value_ids(fn_ir);
    let mut boundaries = Vec::new();

    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { val, .. } => {
                    boundaries.push(SroaMaterializationBoundary {
                        value: *val,
                        kind: SroaMaterializationBoundaryKind::Eval,
                    });
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    for value in [*base, *idx, *val] {
                        boundaries.push(SroaMaterializationBoundary {
                            value,
                            kind: SroaMaterializationBoundaryKind::StoreIndexOperand,
                        });
                    }
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    for value in [*base, *r, *c, *val] {
                        boundaries.push(SroaMaterializationBoundary {
                            value,
                            kind: SroaMaterializationBoundaryKind::StoreIndexOperand,
                        });
                    }
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    for value in [*base, *i, *j, *k, *val] {
                        boundaries.push(SroaMaterializationBoundary {
                            value,
                            kind: SroaMaterializationBoundaryKind::StoreIndexOperand,
                        });
                    }
                }
                Instr::UnsafeRBlock { .. } => {}
            }
        }
    }

    for value in &fn_ir.values {
        if !live_values.contains(&value.id) {
            continue;
        }
        match &value.kind {
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                boundaries.push(SroaMaterializationBoundary {
                    value: *base,
                    kind: SroaMaterializationBoundaryKind::ConcreteBase,
                });
            }
            ValueKind::Call { args, .. } => {
                for arg in args {
                    boundaries.push(SroaMaterializationBoundary {
                        value: *arg,
                        kind: SroaMaterializationBoundaryKind::CallArg,
                    });
                }
            }
            ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    boundaries.push(SroaMaterializationBoundary {
                        value: *arg,
                        kind: SroaMaterializationBoundaryKind::IntrinsicArg,
                    });
                }
            }
            ValueKind::RecordLit { fields } => {
                for (_, field_value) in fields {
                    boundaries.push(SroaMaterializationBoundary {
                        value: *field_value,
                        kind: SroaMaterializationBoundaryKind::RecordField,
                    });
                }
            }
            ValueKind::FieldSet { base, value, .. } => {
                boundaries.push(SroaMaterializationBoundary {
                    value: *base,
                    kind: SroaMaterializationBoundaryKind::FieldSetBase,
                });
                boundaries.push(SroaMaterializationBoundary {
                    value: *value,
                    kind: SroaMaterializationBoundaryKind::FieldSetValue,
                });
            }
            ValueKind::Index1D { base, .. }
            | ValueKind::Index2D { base, .. }
            | ValueKind::Index3D { base, .. } => {
                boundaries.push(SroaMaterializationBoundary {
                    value: *base,
                    kind: SroaMaterializationBoundaryKind::ConcreteBase,
                });
            }
            _ => {}
        }
    }

    for block in &fn_ir.blocks {
        if let Terminator::Return(Some(value)) = block.term {
            boundaries.push(SroaMaterializationBoundary {
                value,
                kind: SroaMaterializationBoundaryKind::Return,
            });
        }
    }

    boundaries
}

pub(crate) fn add_materialized_value(
    fn_ir: &FnIR,
    unique_assignments: &FxHashMap<String, ValueId>,
    values: &mut FxHashSet<ValueId>,
    value: ValueId,
) {
    let mut stack = vec![value];
    while let Some(current) = stack.pop() {
        if !values.insert(current) {
            continue;
        }
        if let ValueKind::Load { var } = &fn_ir.values[current].kind
            && let Some(src) = unique_assignments.get(var)
        {
            stack.push(*src);
        }
        match &fn_ir.values[current].kind {
            ValueKind::RecordLit { fields } => {
                stack.extend(fields.iter().map(|(_, field_value)| *field_value));
            }
            ValueKind::FieldSet { base, value, .. } => {
                stack.extend([*base, *value]);
            }
            ValueKind::Len { base }
            | ValueKind::Indices { base }
            | ValueKind::Index1D { base, .. }
            | ValueKind::Index2D { base, .. }
            | ValueKind::Index3D { base, .. } => {
                stack.push(*base);
            }
            _ => {}
        }
    }
}

pub(crate) fn shared_field_map_shape(
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
    args: &[(ValueId, BlockId)],
) -> Option<Vec<String>> {
    let (first, _) = args.first()?;
    let mut fields: Vec<String> = field_maps.get(first)?.keys().cloned().collect();
    fields.sort();

    for (arg, _) in args.iter().skip(1) {
        let field_map = field_maps.get(arg)?;
        if field_map.len() != fields.len()
            || !fields
                .iter()
                .all(|field| field_map.contains_key(field.as_str()))
        {
            return None;
        }
    }

    Some(fields)
}
