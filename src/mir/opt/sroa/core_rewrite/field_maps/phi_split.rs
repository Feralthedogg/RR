use super::*;
#[derive(Debug, Clone)]
pub(crate) struct RecordPhiSplitCandidate {
    pub(crate) value: ValueId,
    pub(crate) args: Vec<(ValueId, BlockId)>,
    pub(crate) phi_block: BlockId,
    pub(crate) span: Span,
}

pub(crate) fn split_demanded_record_phis(
    fn_ir: &mut FnIR,
    field_maps: &mut FxHashMap<ValueId, SroaFieldMap>,
) -> bool {
    let mut demanded_fields = demanded_aggregate_fields(fn_ir);
    let materialized_values = materialized_aggregate_values(fn_ir);
    let candidates = collect_record_phi_split_candidates(fn_ir, field_maps);
    for candidate in &candidates {
        if !materialized_values.contains(&candidate.value) {
            continue;
        }
        let Some(fields) = shared_field_map_shape(field_maps, &candidate.args) else {
            continue;
        };
        demanded_fields
            .entry(candidate.value)
            .or_default()
            .extend(fields);
    }

    if demanded_fields.is_empty() {
        return false;
    }

    let mut changed = false;
    for candidate in candidates {
        if field_maps.contains_key(&candidate.value) {
            continue;
        }
        let Some(requested_fields) = demanded_fields.get(&candidate.value) else {
            continue;
        };
        let Some(fields) = shared_field_map_shape(field_maps, &candidate.args) else {
            continue;
        };
        if !requested_fields.iter().any(|field| {
            fields
                .iter()
                .any(|candidate_field| candidate_field == field)
        }) {
            continue;
        }

        let mut scalar_fields = FxHashMap::default();
        for field in fields {
            let mut args = Vec::with_capacity(candidate.args.len());
            for (arg, pred) in &candidate.args {
                let Some(field_value) = field_maps
                    .get(arg)
                    .and_then(|field_map| field_map.get(&field))
                    .copied()
                else {
                    args.clear();
                    break;
                };
                args.push((field_value, *pred));
            }
            if args.is_empty() {
                scalar_fields.clear();
                break;
            }

            let field_phi = fn_ir.add_value(
                ValueKind::Phi { args },
                candidate.span,
                Facts::empty(),
                None,
            );
            fn_ir.values[field_phi].phi_block = Some(candidate.phi_block);
            scalar_fields.insert(field, field_phi);
        }

        if !scalar_fields.is_empty() {
            field_maps.insert(candidate.value, scalar_fields);
            changed = true;
        }
    }

    changed
}

pub(crate) fn collect_record_phi_split_candidates(
    fn_ir: &FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
) -> Vec<RecordPhiSplitCandidate> {
    fn_ir
        .values
        .iter()
        .filter_map(|value| {
            if field_maps.contains_key(&value.id) {
                return None;
            }
            let ValueKind::Phi { args } = &value.kind else {
                return None;
            };
            let phi_block = value.phi_block?;
            if args.is_empty() || !args.iter().all(|(arg, _)| field_maps.contains_key(arg)) {
                return None;
            }
            Some(RecordPhiSplitCandidate {
                value: value.id,
                args: args.clone(),
                phi_block,
                span: value.span,
            })
        })
        .collect()
}

pub(crate) fn demanded_aggregate_fields(fn_ir: &FnIR) -> FxHashMap<ValueId, FxHashSet<String>> {
    let live_values = collect_live_value_ids(fn_ir);
    let unique_assignments = unique_var_assignments(fn_ir);
    let mut demanded: FxHashMap<ValueId, FxHashSet<String>> = FxHashMap::default();
    for value in &fn_ir.values {
        if !live_values.contains(&value.id) {
            continue;
        }
        let ValueKind::FieldGet { base, field } = &value.kind else {
            continue;
        };
        add_demanded_alias_field(fn_ir, &unique_assignments, &mut demanded, *base, field);
    }
    demanded
}

pub(crate) fn add_demanded_alias_field(
    fn_ir: &FnIR,
    unique_assignments: &FxHashMap<String, ValueId>,
    demanded: &mut FxHashMap<ValueId, FxHashSet<String>>,
    value: ValueId,
    field: &str,
) {
    let mut stack = vec![value];
    let mut seen = FxHashSet::default();
    while let Some(current) = stack.pop() {
        if !seen.insert(current) {
            continue;
        }
        demanded
            .entry(current)
            .or_default()
            .insert(field.to_string());
        if let ValueKind::Load { var } = &fn_ir.values[current].kind
            && let Some(src) = unique_assignments.get(var)
        {
            stack.push(*src);
        }
    }
}
