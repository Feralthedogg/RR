use super::*;
pub(crate) fn candidate_source(
    fn_ir: &FnIR,
    value: ValueId,
    shapes: &FxHashMap<ValueId, Vec<String>>,
) -> Option<SroaCandidateSource> {
    match &fn_ir.values[value].kind {
        ValueKind::RecordLit { .. } => Some(SroaCandidateSource::RecordLit),
        ValueKind::FieldSet { .. } => Some(SroaCandidateSource::FieldSet),
        ValueKind::Phi { .. } if shapes.contains_key(&value) => Some(SroaCandidateSource::Phi),
        ValueKind::Load { .. } if shapes.contains_key(&value) => {
            Some(SroaCandidateSource::LoadAlias)
        }
        _ => None,
    }
}

pub(crate) fn infer_candidate_shapes(
    fn_ir: &FnIR,
) -> (
    FxHashMap<ValueId, Vec<String>>,
    FxHashMap<ValueId, Vec<SroaRejectReason>>,
) {
    let mut shapes = FxHashMap::default();
    let mut rejects: FxHashMap<ValueId, Vec<SroaRejectReason>> = FxHashMap::default();
    let mut var_shapes: FxHashMap<String, Vec<String>> = FxHashMap::default();

    for value in &fn_ir.values {
        if let ValueKind::RecordLit { fields } = &value.kind {
            match record_shape(fields) {
                Ok(shape) => {
                    shapes.insert(value.id, shape);
                }
                Err(reasons) => {
                    rejects.insert(value.id, reasons);
                }
            }
        }
    }

    let mut changed = true;
    while changed {
        changed = false;

        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                if let Instr::Assign { dst, src, .. } = instr
                    && let Some(shape) = shapes.get(src)
                    && var_shapes.get(dst) != Some(shape)
                {
                    var_shapes.insert(dst.clone(), shape.clone());
                    changed = true;
                }
            }
        }

        for value in &fn_ir.values {
            if shapes.contains_key(&value.id) || rejects.contains_key(&value.id) {
                continue;
            }
            let inferred = match &value.kind {
                ValueKind::FieldSet { base, field, .. } => {
                    let Some(shape) = shapes.get(base) else {
                        rejects
                            .entry(value.id)
                            .or_default()
                            .push(SroaRejectReason::MissingBaseShape);
                        continue;
                    };
                    if shape.iter().any(|name| name == field) {
                        Some(shape.clone())
                    } else {
                        rejects
                            .entry(value.id)
                            .or_default()
                            .push(SroaRejectReason::ShapeChangingFieldSet(field.clone()));
                        continue;
                    }
                }
                ValueKind::Phi { args } => infer_phi_shape(args, &shapes, &mut rejects, value.id),
                ValueKind::Load { var } => var_shapes.get(var).cloned(),
                _ => None,
            };

            if let Some(shape) = inferred {
                shapes.insert(value.id, shape);
                changed = true;
            }
        }
    }

    (shapes, rejects)
}

pub(crate) fn record_shape(
    fields: &[(String, ValueId)],
) -> Result<Vec<String>, Vec<SroaRejectReason>> {
    if fields.is_empty() {
        return Err(vec![SroaRejectReason::EmptyRecord]);
    }

    let mut seen = FxHashSet::default();
    let mut reasons = Vec::new();
    let mut shape = Vec::with_capacity(fields.len());
    for (field, _) in fields {
        if !seen.insert(field.clone()) {
            reasons.push(SroaRejectReason::DuplicateField(field.clone()));
        }
        shape.push(field.clone());
    }

    if reasons.is_empty() {
        Ok(shape)
    } else {
        Err(reasons)
    }
}

pub(crate) fn infer_phi_shape(
    args: &[(ValueId, BlockId)],
    shapes: &FxHashMap<ValueId, Vec<String>>,
    rejects: &mut FxHashMap<ValueId, Vec<SroaRejectReason>>,
    value: ValueId,
) -> Option<Vec<String>> {
    if args.is_empty() {
        rejects
            .entry(value)
            .or_default()
            .push(SroaRejectReason::EmptyPhi);
        return None;
    }

    let mut arg_shapes = Vec::with_capacity(args.len());
    for (arg, _) in args {
        let shape = shapes.get(arg)?;
        arg_shapes.push(shape);
    }

    let first = arg_shapes.first()?;
    if arg_shapes.iter().all(|shape| *shape == *first) {
        Some((*first).clone())
    } else {
        rejects
            .entry(value)
            .or_default()
            .push(SroaRejectReason::InconsistentPhiShape);
        None
    }
}
