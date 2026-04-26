use super::*;
use crate::mir::analyze::effects;
use crate::utils::Span;

type SroaFieldMap = FxHashMap<String, ValueId>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SroaAnalysis {
    pub(super) candidates: Vec<SroaCandidate>,
}

impl SroaAnalysis {
    pub(super) fn counts(&self) -> SroaAnalysisCounts {
        let mut counts = SroaAnalysisCounts::default();
        counts.candidates = self.candidates.len();
        for candidate in &self.candidates {
            match candidate.source {
                SroaCandidateSource::RecordLit => counts.record_lits += 1,
                SroaCandidateSource::FieldSet => counts.field_sets += 1,
                SroaCandidateSource::Phi => counts.phis += 1,
                SroaCandidateSource::LoadAlias => counts.load_aliases += 1,
            }
            match candidate.status {
                SroaCandidateStatus::ScalarOnly => counts.scalar_only += 1,
                SroaCandidateStatus::NeedsRematerialization => counts.needs_rematerialization += 1,
                SroaCandidateStatus::Rejected => counts.rejected += 1,
            }
        }
        counts
    }

    pub(super) fn candidate(&self, value: ValueId) -> Option<&SroaCandidate> {
        self.candidates
            .iter()
            .find(|candidate| candidate.value == value)
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(super) struct SroaAnalysisCounts {
    pub(super) candidates: usize,
    pub(super) record_lits: usize,
    pub(super) field_sets: usize,
    pub(super) phis: usize,
    pub(super) load_aliases: usize,
    pub(super) scalar_only: usize,
    pub(super) needs_rematerialization: usize,
    pub(super) rejected: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SroaCandidate {
    pub(super) value: ValueId,
    pub(super) source: SroaCandidateSource,
    pub(super) shape: Option<Vec<String>>,
    pub(super) uses: Vec<SroaUse>,
    pub(super) status: SroaCandidateStatus,
    pub(super) reject_reasons: Vec<SroaRejectReason>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SroaCandidateSource {
    RecordLit,
    FieldSet,
    Phi,
    LoadAlias,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SroaCandidateStatus {
    ScalarOnly,
    NeedsRematerialization,
    Rejected,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SroaUse {
    pub(super) user: SroaUser,
    pub(super) kind: SroaUseKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SroaUseKind {
    Projection,
    Update,
    Alias,
    Phi,
    Materialize,
    Reject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum SroaUser {
    Value(ValueId),
    Instr { block: BlockId, instr: usize },
    Terminator { block: BlockId },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SroaMaterializationBoundaryKind {
    Eval,
    Return,
    CallArg,
    IntrinsicArg,
    RecordField,
    FieldSetBase,
    FieldSetValue,
    ConcreteBase,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct SroaMaterializationBoundary {
    value: ValueId,
    kind: SroaMaterializationBoundaryKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum SroaRejectReason {
    EmptyRecord,
    DuplicateField(String),
    MissingBaseShape,
    ShapeChangingFieldSet(String),
    EmptyPhi,
    InconsistentPhiShape,
    UnsupportedUse(SroaUseKind),
}

pub(super) fn analyze_function(fn_ir: &FnIR) -> SroaAnalysis {
    let uses = build_use_graph(fn_ir);
    let (shapes, base_rejects) = infer_candidate_shapes(fn_ir);
    let mut candidates = Vec::new();

    for value in &fn_ir.values {
        let Some(source) = candidate_source(fn_ir, value.id, &shapes) else {
            continue;
        };
        let mut reject_reasons = base_rejects.get(&value.id).cloned().unwrap_or_default();
        let shape = shapes.get(&value.id).cloned();
        let value_uses = uses.get(&value.id).cloned().unwrap_or_default();

        for value_use in &value_uses {
            if value_use.kind == SroaUseKind::Reject {
                reject_reasons.push(SroaRejectReason::UnsupportedUse(value_use.kind));
            }
        }

        let status = if !reject_reasons.is_empty() || shape.is_none() {
            SroaCandidateStatus::Rejected
        } else if value_uses
            .iter()
            .any(|value_use| value_use.kind == SroaUseKind::Materialize)
        {
            SroaCandidateStatus::NeedsRematerialization
        } else {
            SroaCandidateStatus::ScalarOnly
        };

        candidates.push(SroaCandidate {
            value: value.id,
            source,
            shape,
            uses: value_uses,
            status,
            reject_reasons,
        });
    }

    SroaAnalysis { candidates }
}

pub(super) fn optimize(fn_ir: &mut FnIR) -> bool {
    if fn_ir.requires_conservative_optimization() || !is_control_flow_rewrite_candidate(fn_ir) {
        return false;
    }

    let mut changed = false;
    let max_rounds = fn_ir.values.len().saturating_add(1).max(1);
    for _ in 0..max_rounds {
        let round_changed = optimize_once(fn_ir);
        changed |= round_changed;
        if !round_changed {
            break;
        }
    }

    changed
}

fn optimize_once(fn_ir: &mut FnIR) -> bool {
    let snapshot_changed = snapshot_record_alias_fields(fn_ir);
    let field_maps = infer_rewrite_field_maps(fn_ir);
    if field_maps.is_empty() {
        return snapshot_changed;
    }

    let mut replacements = FxHashMap::default();
    for value in &fn_ir.values {
        let ValueKind::FieldGet { base, field } = &value.kind else {
            continue;
        };
        let Some(replacement) = field_maps.get(base).and_then(|fields| fields.get(field)) else {
            continue;
        };
        if *replacement != value.id {
            replacements.insert(value.id, *replacement);
        }
    }

    let mut changed = snapshot_changed;
    if !replacements.is_empty() {
        changed |= apply_value_replacements(fn_ir, &replacements);
    }

    changed |= rematerialize_aggregate_boundaries(fn_ir, &field_maps);
    changed |= remove_dead_scalarized_aggregate_assigns(fn_ir, &field_maps);
    changed
}

fn is_control_flow_rewrite_candidate(fn_ir: &FnIR) -> bool {
    for block in &fn_ir.blocks {
        if block.instrs.iter().any(|instr| {
            matches!(
                instr,
                Instr::StoreIndex1D { .. }
                    | Instr::StoreIndex2D { .. }
                    | Instr::StoreIndex3D { .. }
            )
        }) {
            return false;
        }
    }

    true
}

fn infer_rewrite_field_maps(fn_ir: &mut FnIR) -> FxHashMap<ValueId, SroaFieldMap> {
    let snapshot_vars = sroa_snapshot_vars(fn_ir);
    let mut field_maps = FxHashMap::default();
    for value in &fn_ir.values {
        if let ValueKind::RecordLit { fields } = &value.kind
            && let Some(field_map) = scalarizable_record_field_map(fn_ir, fields, &snapshot_vars)
        {
            field_maps.insert(value.id, field_map);
        }
    }

    propagate_rewrite_field_maps(fn_ir, &mut field_maps, &snapshot_vars);
    split_demanded_record_phis(fn_ir, &mut field_maps);
    propagate_rewrite_field_maps(fn_ir, &mut field_maps, &snapshot_vars);

    field_maps
}

fn propagate_rewrite_field_maps(
    fn_ir: &FnIR,
    field_maps: &mut FxHashMap<ValueId, SroaFieldMap>,
    snapshot_vars: &FxHashSet<String>,
) {
    let unique_assignments = unique_var_assignments(fn_ir);
    let mut var_maps: FxHashMap<String, SroaFieldMap> = FxHashMap::default();
    let mut changed = true;
    while changed {
        changed = false;

        for (var, src) in &unique_assignments {
            let Some(field_map) = field_maps.get(src) else {
                continue;
            };
            if var_maps.get(var) != Some(field_map) {
                var_maps.insert(var.clone(), field_map.clone());
                changed = true;
            }
        }

        for value in &fn_ir.values {
            let inferred = match &value.kind {
                ValueKind::Load { var } => var_maps.get(var).cloned(),
                ValueKind::FieldSet { base, field, value } => {
                    if let Some(base_map) = field_maps.get(base) {
                        if !base_map.contains_key(field)
                            || !sroa_value_is_scalarizable_field(
                                fn_ir,
                                *value,
                                snapshot_vars,
                                &mut FxHashSet::default(),
                            )
                        {
                            None
                        } else {
                            let mut updated_map = base_map.clone();
                            updated_map.insert(field.clone(), *value);
                            Some(updated_map)
                        }
                    } else {
                        None
                    }
                }
                _ => None,
            };

            let Some(field_map) = inferred else {
                continue;
            };
            if field_maps.get(&value.id) != Some(&field_map) {
                field_maps.insert(value.id, field_map);
                changed = true;
            }
        }
    }
}

#[derive(Debug, Clone)]
struct RecordPhiSplitCandidate {
    value: ValueId,
    args: Vec<(ValueId, BlockId)>,
    phi_block: BlockId,
    span: Span,
}

fn split_demanded_record_phis(
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

fn collect_record_phi_split_candidates(
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

fn demanded_aggregate_fields(fn_ir: &FnIR) -> FxHashMap<ValueId, FxHashSet<String>> {
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

fn add_demanded_alias_field(
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

fn materialized_aggregate_values(fn_ir: &FnIR) -> FxHashSet<ValueId> {
    let unique_assignments = unique_var_assignments(fn_ir);
    let mut values = FxHashSet::default();

    for boundary in collect_materialization_boundaries(fn_ir) {
        add_materialized_value(fn_ir, &unique_assignments, &mut values, boundary.value);
    }

    values
}

fn collect_materialization_boundaries(fn_ir: &FnIR) -> Vec<SroaMaterializationBoundary> {
    let live_values = collect_non_alias_live_value_ids(fn_ir);
    let mut boundaries = Vec::new();

    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            if let Instr::Eval { val, .. } = instr {
                boundaries.push(SroaMaterializationBoundary {
                    value: *val,
                    kind: SroaMaterializationBoundaryKind::Eval,
                });
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

fn add_materialized_value(
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

fn shared_field_map_shape(
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

fn unique_var_assignments(fn_ir: &FnIR) -> FxHashMap<String, ValueId> {
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

#[derive(Debug, Clone)]
struct RecordFieldSnapshotPlan {
    block: BlockId,
    instr_index: usize,
    record: ValueId,
    inserted_instrs: Vec<Instr>,
    field_replacements: Vec<(usize, ValueId)>,
}

fn snapshot_record_alias_fields(fn_ir: &mut FnIR) -> bool {
    let unique_assignments = unique_var_assignments(fn_ir);
    if unique_assignments.is_empty() {
        return false;
    }

    let existing_snapshot_vars = sroa_snapshot_vars(fn_ir);
    let mut candidates = Vec::new();
    for block in &fn_ir.blocks {
        for (instr_index, instr) in block.instrs.iter().enumerate() {
            let Instr::Assign { dst, src, .. } = instr else {
                continue;
            };
            if unique_assignments.get(dst).copied() != Some(*src) {
                continue;
            }
            let Some(snapshot_fields) =
                collect_record_alias_field_snapshots(fn_ir, *src, dst, &existing_snapshot_vars)
            else {
                continue;
            };
            if !snapshot_fields.is_empty() {
                candidates.push((block.id, instr_index, *src, dst.clone(), snapshot_fields));
            }
        }
    }
    if candidates.is_empty() {
        return false;
    }

    let mut used_vars = used_var_names(fn_ir);
    let mut plans = Vec::new();
    for (block, instr_index, record, alias, snapshot_fields) in candidates {
        let mut inserted_instrs = Vec::with_capacity(snapshot_fields.len());
        let mut field_replacements = Vec::with_capacity(snapshot_fields.len());
        for (field_index, field_name, field_value) in snapshot_fields {
            let temp_var = unique_sroa_snapshot_temp_var(&alias, &field_name, &mut used_vars);
            let span = fn_ir.values[field_value].span;
            let temp_load = fn_ir.add_value(
                ValueKind::Load {
                    var: temp_var.clone(),
                },
                span,
                Facts::empty(),
                Some(temp_var.clone()),
            );
            inserted_instrs.push(Instr::Assign {
                dst: temp_var,
                src: field_value,
                span,
            });
            field_replacements.push((field_index, temp_load));
        }
        plans.push(RecordFieldSnapshotPlan {
            block,
            instr_index,
            record,
            inserted_instrs,
            field_replacements,
        });
    }

    plans.sort_by(|left, right| {
        right
            .block
            .cmp(&left.block)
            .then_with(|| right.instr_index.cmp(&left.instr_index))
    });

    let mut changed = false;
    for plan in plans {
        if let ValueKind::RecordLit { fields } = &mut fn_ir.values[plan.record].kind {
            for (field_index, replacement) in plan.field_replacements {
                if let Some((_, field_value)) = fields.get_mut(field_index) {
                    *field_value = replacement;
                    changed = true;
                }
            }
        }
        if !plan.inserted_instrs.is_empty() {
            let insert_at = plan.instr_index.min(fn_ir.blocks[plan.block].instrs.len());
            fn_ir.blocks[plan.block]
                .instrs
                .splice(insert_at..insert_at, plan.inserted_instrs);
            changed = true;
        }
    }

    changed
}

fn collect_record_alias_field_snapshots(
    fn_ir: &FnIR,
    record: ValueId,
    alias: &str,
    snapshot_vars: &FxHashSet<String>,
) -> Option<Vec<(usize, String, ValueId)>> {
    let ValueKind::RecordLit { fields } = &fn_ir.values[record].kind else {
        return None;
    };
    if record_shape(fields).is_err() {
        return None;
    }

    let mut snapshots = Vec::new();
    for (field_index, (field_name, field_value)) in fields.iter().enumerate() {
        if value_loads_var(fn_ir, *field_value, alias) {
            return None;
        }
        if sroa_value_is_scalarizable_field(
            fn_ir,
            *field_value,
            snapshot_vars,
            &mut FxHashSet::default(),
        ) {
            continue;
        }
        if !sroa_value_is_snapshot_safe(fn_ir, *field_value, &mut FxHashSet::default()) {
            return None;
        }
        snapshots.push((field_index, field_name.clone(), *field_value));
    }

    Some(snapshots)
}

fn sroa_snapshot_vars(fn_ir: &FnIR) -> FxHashSet<String> {
    let unique_assignments = unique_var_assignments(fn_ir);
    unique_assignments
        .keys()
        .filter(|var| var.contains("__rr_sroa_snap_"))
        .cloned()
        .collect()
}

fn unique_sroa_snapshot_temp_var(
    alias: &str,
    field: &str,
    used_vars: &mut FxHashSet<String>,
) -> String {
    let alias = sanitize_symbol_segment(alias);
    let field = sanitize_symbol_segment(field);
    let seed = format!("{alias}__rr_sroa_snap_{field}");
    if used_vars.insert(seed.clone()) {
        return seed;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{seed}_{suffix}");
        if used_vars.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn value_loads_var(fn_ir: &FnIR, value: ValueId, target: &str) -> bool {
    loaded_vars_in_values(fn_ir, [value]).contains(target)
}

fn scalarizable_record_field_map(
    fn_ir: &FnIR,
    fields: &[(String, ValueId)],
    snapshot_vars: &FxHashSet<String>,
) -> Option<SroaFieldMap> {
    if record_shape(fields).is_err() {
        return None;
    }
    let mut field_map = FxHashMap::default();
    for (field, value) in fields {
        if !sroa_value_is_scalarizable_field(
            fn_ir,
            *value,
            snapshot_vars,
            &mut FxHashSet::default(),
        ) {
            return None;
        }
        field_map.insert(field.clone(), *value);
    }
    Some(field_map)
}

fn sroa_value_is_pure(fn_ir: &FnIR, value: ValueId, visiting: &mut FxHashSet<ValueId>) -> bool {
    sroa_value_is_scalarizable_field(fn_ir, value, &FxHashSet::default(), visiting)
}

fn sroa_value_is_scalarizable_field(
    fn_ir: &FnIR,
    value: ValueId,
    snapshot_vars: &FxHashSet<String>,
    visiting: &mut FxHashSet<ValueId>,
) -> bool {
    if !visiting.insert(value) {
        return true;
    }

    let pure = match &fn_ir.values[value].kind {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => true,
        ValueKind::Load { var } => snapshot_vars.contains(var),
        ValueKind::Binary { lhs, rhs, .. } => {
            sroa_value_is_scalarizable_field(fn_ir, *lhs, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *rhs, snapshot_vars, visiting)
        }
        ValueKind::Unary { rhs, .. }
        | ValueKind::Len { base: rhs }
        | ValueKind::Indices { base: rhs }
        | ValueKind::FieldGet { base: rhs, .. } => {
            sroa_value_is_scalarizable_field(fn_ir, *rhs, snapshot_vars, visiting)
        }
        ValueKind::Range { start, end } => {
            sroa_value_is_scalarizable_field(fn_ir, *start, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *end, snapshot_vars, visiting)
        }
        ValueKind::Call { callee, args, .. } => {
            effects::call_is_pure(callee)
                && args.iter().all(|arg| {
                    sroa_value_is_scalarizable_field(fn_ir, *arg, snapshot_vars, visiting)
                })
        }
        ValueKind::RecordLit { fields } => fields.iter().all(|(_, value)| {
            sroa_value_is_scalarizable_field(fn_ir, *value, snapshot_vars, visiting)
        }),
        ValueKind::FieldSet { base, value, .. } => {
            sroa_value_is_scalarizable_field(fn_ir, *base, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *value, snapshot_vars, visiting)
        }
        ValueKind::Intrinsic { args, .. } => args
            .iter()
            .all(|arg| sroa_value_is_scalarizable_field(fn_ir, *arg, snapshot_vars, visiting)),
        ValueKind::Index1D { base, idx, .. } => {
            sroa_value_is_scalarizable_field(fn_ir, *base, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *idx, snapshot_vars, visiting)
        }
        ValueKind::Index2D { base, r, c } => {
            sroa_value_is_scalarizable_field(fn_ir, *base, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *r, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *c, snapshot_vars, visiting)
        }
        ValueKind::Index3D { base, i, j, k } => {
            sroa_value_is_scalarizable_field(fn_ir, *base, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *i, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *j, snapshot_vars, visiting)
                && sroa_value_is_scalarizable_field(fn_ir, *k, snapshot_vars, visiting)
        }
        ValueKind::Phi { args } => args
            .iter()
            .all(|(arg, _)| sroa_value_is_scalarizable_field(fn_ir, *arg, snapshot_vars, visiting)),
    };

    visiting.remove(&value);
    pure
}

fn sroa_value_is_snapshot_safe(
    fn_ir: &FnIR,
    value: ValueId,
    visiting: &mut FxHashSet<ValueId>,
) -> bool {
    if !visiting.insert(value) {
        return true;
    }

    let safe = match &fn_ir.values[value].kind {
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => true,
        ValueKind::Binary { lhs, rhs, .. } => {
            sroa_value_is_snapshot_safe(fn_ir, *lhs, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *rhs, visiting)
        }
        ValueKind::Unary { rhs, .. }
        | ValueKind::Len { base: rhs }
        | ValueKind::Indices { base: rhs }
        | ValueKind::FieldGet { base: rhs, .. } => {
            sroa_value_is_snapshot_safe(fn_ir, *rhs, visiting)
        }
        ValueKind::Range { start, end } => {
            sroa_value_is_snapshot_safe(fn_ir, *start, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *end, visiting)
        }
        ValueKind::Call { callee, args, .. } => {
            effects::call_is_pure(callee)
                && args
                    .iter()
                    .all(|arg| sroa_value_is_snapshot_safe(fn_ir, *arg, visiting))
        }
        ValueKind::RecordLit { fields } => fields
            .iter()
            .all(|(_, value)| sroa_value_is_snapshot_safe(fn_ir, *value, visiting)),
        ValueKind::FieldSet { base, value, .. } => {
            sroa_value_is_snapshot_safe(fn_ir, *base, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *value, visiting)
        }
        ValueKind::Intrinsic { args, .. } => args
            .iter()
            .all(|arg| sroa_value_is_snapshot_safe(fn_ir, *arg, visiting)),
        ValueKind::Index1D { base, idx, .. } => {
            sroa_value_is_snapshot_safe(fn_ir, *base, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *idx, visiting)
        }
        ValueKind::Index2D { base, r, c } => {
            sroa_value_is_snapshot_safe(fn_ir, *base, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *r, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *c, visiting)
        }
        ValueKind::Index3D { base, i, j, k } => {
            sroa_value_is_snapshot_safe(fn_ir, *base, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *i, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *j, visiting)
                && sroa_value_is_snapshot_safe(fn_ir, *k, visiting)
        }
        ValueKind::Phi { .. } => false,
    };

    visiting.remove(&value);
    safe
}

fn apply_value_replacements(fn_ir: &mut FnIR, replacements: &FxHashMap<ValueId, ValueId>) -> bool {
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

fn rematerialize_aggregate_boundaries(
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

fn rewrite_concrete_base_consumer(
    kind: &mut ValueKind,
    old_base: ValueId,
    replacement: ValueId,
) -> bool {
    let base = match kind {
        ValueKind::Len { base }
        | ValueKind::Indices { base }
        | ValueKind::Index1D { base, .. }
        | ValueKind::Index2D { base, .. }
        | ValueKind::Index3D { base, .. } => base,
        _ => return false,
    };
    if *base != old_base {
        return false;
    }
    *base = replacement;
    true
}

fn should_rematerialize_boundary_value(
    fn_ir: &FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
    value: ValueId,
) -> bool {
    field_maps.contains_key(&value)
        && !matches!(fn_ir.values[value].kind, ValueKind::RecordLit { .. })
}

fn rematerialize_value(
    fn_ir: &mut FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
    shapes: &FxHashMap<ValueId, Vec<String>>,
    materialized: &mut FxHashMap<ValueId, ValueId>,
    value: ValueId,
) -> Option<ValueId> {
    if let Some(existing) = materialized.get(&value).copied() {
        return Some(existing);
    }

    if matches!(fn_ir.values[value].kind, ValueKind::RecordLit { .. }) {
        return Some(value);
    }

    let field_map = field_maps.get(&value)?;
    let shape = materialization_shape(fn_ir, field_maps, shapes, value)?;
    let mut fields = Vec::with_capacity(shape.len());
    for field in shape {
        let mut field_value = *field_map.get(&field)?;
        if should_rematerialize_boundary_value(fn_ir, field_maps, field_value) {
            field_value =
                rematerialize_value(fn_ir, field_maps, shapes, materialized, field_value)?;
        }
        fields.push((field, field_value));
    }

    let rematerialized = fn_ir.add_value(
        ValueKind::RecordLit { fields },
        fn_ir.values[value].span,
        Facts::empty(),
        None,
    );
    materialized.insert(value, rematerialized);
    Some(rematerialized)
}

fn materialization_shape(
    fn_ir: &FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
    shapes: &FxHashMap<ValueId, Vec<String>>,
    value: ValueId,
) -> Option<Vec<String>> {
    if let Some(shape) = shapes.get(&value) {
        return Some(shape.clone());
    }
    if let ValueKind::RecordLit { fields } = &fn_ir.values[value].kind {
        return record_shape(fields).ok();
    }
    let mut fields: Vec<_> = field_maps.get(&value)?.keys().cloned().collect();
    fields.sort();
    Some(fields)
}

fn remove_dead_scalarized_aggregate_assigns(
    fn_ir: &mut FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
) -> bool {
    let unique_assignments = unique_var_assignments(fn_ir);
    let required_loaded_vars = scalarized_loaded_vars_required_by_non_alias_uses(fn_ir, field_maps);
    let mut changed = false;

    for value in &mut fn_ir.values {
        let ValueKind::Load { var } = &value.kind else {
            continue;
        };
        let Some(src) = unique_assignments.get(var) else {
            continue;
        };
        if field_maps.contains_key(src) && !required_loaded_vars.contains(var) {
            value.kind = ValueKind::Const(Lit::Null);
            value.origin_var = None;
            changed = true;
        }
    }

    for block in &mut fn_ir.blocks {
        let old_len = block.instrs.len();
        block.instrs.retain(|instr| {
            let Instr::Assign { dst, src, .. } = instr else {
                return true;
            };
            !field_maps.contains_key(src) || required_loaded_vars.contains(dst)
        });
        changed |= block.instrs.len() != old_len;
    }
    changed
}

fn scalarized_loaded_vars_required_by_non_alias_uses(
    fn_ir: &FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
) -> FxHashSet<String> {
    let mut roots = Vec::new();
    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { val, .. } => roots.push(*val),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    roots.extend([*base, *idx, *val]);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    roots.extend([*base, *r, *c, *val]);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    roots.extend([*base, *i, *j, *k, *val]);
                }
            }
        }
        match &block.term {
            Terminator::If { cond, .. } => roots.push(*cond),
            Terminator::Return(Some(value)) => roots.push(*value),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    let unique_assignments = unique_var_assignments(fn_ir);
    let mut required = loaded_vars_in_values(fn_ir, roots);
    let mut changed = true;
    while changed {
        changed = false;
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                let Instr::Assign { dst, src, .. } = instr else {
                    continue;
                };
                if !required.contains(dst) {
                    continue;
                }
                for var in loaded_vars_in_values(fn_ir, [*src]) {
                    if let Some(src) = unique_assignments.get(&var)
                        && field_maps.contains_key(src)
                        && required.insert(var)
                    {
                        changed = true;
                    }
                }
            }
        }
    }

    required
}

fn loaded_vars_in_values(
    fn_ir: &FnIR,
    roots: impl IntoIterator<Item = ValueId>,
) -> FxHashSet<String> {
    let mut vars = FxHashSet::default();
    let mut seen = FxHashSet::default();
    let mut stack: Vec<_> = roots.into_iter().collect();

    while let Some(value) = stack.pop() {
        if !seen.insert(value) {
            continue;
        }
        if let ValueKind::Load { var } = &fn_ir.values[value].kind {
            vars.insert(var.clone());
        }
        stack.extend(value_dependencies(&fn_ir.values[value].kind));
    }

    vars
}

fn collect_non_alias_live_value_ids(fn_ir: &FnIR) -> FxHashSet<ValueId> {
    let mut live = FxHashSet::default();
    let mut stack = Vec::new();

    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Assign { .. } => {}
                Instr::Eval { val, .. } => stack.push(*val),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    stack.extend([*base, *idx, *val]);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    stack.extend([*base, *r, *c, *val]);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    stack.extend([*base, *i, *j, *k, *val]);
                }
            }
        }
        match &block.term {
            Terminator::If { cond, .. } => stack.push(*cond),
            Terminator::Return(Some(value)) => stack.push(*value),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    while let Some(value) = stack.pop() {
        if !live.insert(value) {
            continue;
        }
        stack.extend(value_dependencies(&fn_ir.values[value].kind));
    }

    live
}

fn collect_live_value_ids(fn_ir: &FnIR) -> FxHashSet<ValueId> {
    let mut live = FxHashSet::default();
    let mut stack = Vec::new();

    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Assign { src, .. } => stack.push(*src),
                Instr::Eval { val, .. } => stack.push(*val),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    stack.extend([*base, *idx, *val]);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    stack.extend([*base, *r, *c, *val]);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    stack.extend([*base, *i, *j, *k, *val]);
                }
            }
        }
        match &block.term {
            Terminator::If { cond, .. } => stack.push(*cond),
            Terminator::Return(Some(value)) => stack.push(*value),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    while let Some(value) = stack.pop() {
        if !live.insert(value) {
            continue;
        }
        stack.extend(value_dependencies(&fn_ir.values[value].kind));
    }

    live
}

fn rewrite_value_kind_refs(
    kind: &mut ValueKind,
    replacements: &FxHashMap<ValueId, ValueId>,
) -> bool {
    let mut changed = false;
    match kind {
        ValueKind::Binary { lhs, rhs, .. } => {
            changed |= rewrite_value_ref(lhs, replacements);
            changed |= rewrite_value_ref(rhs, replacements);
        }
        ValueKind::Unary { rhs, .. }
        | ValueKind::Len { base: rhs }
        | ValueKind::Indices { base: rhs }
        | ValueKind::FieldGet { base: rhs, .. } => {
            changed |= rewrite_value_ref(rhs, replacements);
        }
        ValueKind::Phi { args } => {
            for (arg, _) in args {
                changed |= rewrite_value_ref(arg, replacements);
            }
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
            for arg in args {
                changed |= rewrite_value_ref(arg, replacements);
            }
        }
        ValueKind::RecordLit { fields } => {
            for (_, value) in fields {
                changed |= rewrite_value_ref(value, replacements);
            }
        }
        ValueKind::FieldSet { base, value, .. } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(value, replacements);
        }
        ValueKind::Index1D { base, idx, .. } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(idx, replacements);
        }
        ValueKind::Index2D { base, r, c } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(r, replacements);
            changed |= rewrite_value_ref(c, replacements);
        }
        ValueKind::Index3D { base, i, j, k } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(i, replacements);
            changed |= rewrite_value_ref(j, replacements);
            changed |= rewrite_value_ref(k, replacements);
        }
        ValueKind::Range { start, end } => {
            changed |= rewrite_value_ref(start, replacements);
            changed |= rewrite_value_ref(end, replacements);
        }
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => {}
    }
    changed
}

fn rewrite_instr_refs(instr: &mut Instr, replacements: &FxHashMap<ValueId, ValueId>) -> bool {
    let mut changed = false;
    match instr {
        Instr::Assign { src, .. } => {
            changed |= rewrite_value_ref(src, replacements);
        }
        Instr::Eval { val, .. } => {
            changed |= rewrite_value_ref(val, replacements);
        }
        Instr::StoreIndex1D { base, idx, val, .. } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(idx, replacements);
            changed |= rewrite_value_ref(val, replacements);
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(r, replacements);
            changed |= rewrite_value_ref(c, replacements);
            changed |= rewrite_value_ref(val, replacements);
        }
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => {
            changed |= rewrite_value_ref(base, replacements);
            changed |= rewrite_value_ref(i, replacements);
            changed |= rewrite_value_ref(j, replacements);
            changed |= rewrite_value_ref(k, replacements);
            changed |= rewrite_value_ref(val, replacements);
        }
    }
    changed
}

fn rewrite_terminator_refs(
    term: &mut Terminator,
    replacements: &FxHashMap<ValueId, ValueId>,
) -> bool {
    match term {
        Terminator::If { cond, .. } => rewrite_value_ref(cond, replacements),
        Terminator::Return(Some(value)) => rewrite_value_ref(value, replacements),
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => false,
    }
}

fn rewrite_value_ref(value: &mut ValueId, replacements: &FxHashMap<ValueId, ValueId>) -> bool {
    let replacement = resolve_replacement(*value, replacements);
    if replacement == *value {
        false
    } else {
        *value = replacement;
        true
    }
}

fn resolve_replacement(value: ValueId, replacements: &FxHashMap<ValueId, ValueId>) -> ValueId {
    let mut current = value;
    let mut seen = FxHashSet::default();
    while let Some(next) = replacements.get(&current).copied() {
        if !seen.insert(current) || next == current {
            break;
        }
        current = next;
    }
    current
}

fn candidate_source(
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

fn infer_candidate_shapes(
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

fn record_shape(fields: &[(String, ValueId)]) -> Result<Vec<String>, Vec<SroaRejectReason>> {
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

fn infer_phi_shape(
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
        let Some(shape) = shapes.get(arg) else {
            return None;
        };
        arg_shapes.push(shape);
    }

    let Some(first) = arg_shapes.first() else {
        return None;
    };
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

fn build_use_graph(fn_ir: &FnIR) -> FxHashMap<ValueId, Vec<SroaUse>> {
    let mut uses = FxHashMap::default();

    for value in &fn_ir.values {
        match &value.kind {
            ValueKind::Phi { args } => {
                for (arg, _) in args {
                    add_use(&mut uses, *arg, SroaUser::Value(value.id), SroaUseKind::Phi);
                }
            }
            ValueKind::FieldGet { base, .. } => {
                add_use(
                    &mut uses,
                    *base,
                    SroaUser::Value(value.id),
                    SroaUseKind::Projection,
                );
            }
            ValueKind::FieldSet {
                base,
                value: field_value,
                ..
            } => {
                add_use(
                    &mut uses,
                    *base,
                    SroaUser::Value(value.id),
                    SroaUseKind::Update,
                );
                add_use(
                    &mut uses,
                    *field_value,
                    SroaUser::Value(value.id),
                    SroaUseKind::Materialize,
                );
            }
            ValueKind::RecordLit { fields } => {
                for (_, field_value) in fields {
                    add_use(
                        &mut uses,
                        *field_value,
                        SroaUser::Value(value.id),
                        SroaUseKind::Materialize,
                    );
                }
            }
            ValueKind::Call { args, .. } => {
                for arg in args {
                    add_use(
                        &mut uses,
                        *arg,
                        SroaUser::Value(value.id),
                        SroaUseKind::Materialize,
                    );
                }
            }
            ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    add_use(
                        &mut uses,
                        *arg,
                        SroaUser::Value(value.id),
                        SroaUseKind::Materialize,
                    );
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                add_use(
                    &mut uses,
                    *base,
                    SroaUser::Value(value.id),
                    SroaUseKind::Materialize,
                );
            }
            ValueKind::Unary { rhs: base, .. } => {
                add_use(
                    &mut uses,
                    *base,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
            }
            ValueKind::Range { start, end } => {
                add_use(
                    &mut uses,
                    *start,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
                add_use(
                    &mut uses,
                    *end,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                add_use(
                    &mut uses,
                    *lhs,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
                add_use(
                    &mut uses,
                    *rhs,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
            }
            ValueKind::Index1D { base, idx, .. } => {
                add_use(
                    &mut uses,
                    *base,
                    SroaUser::Value(value.id),
                    SroaUseKind::Materialize,
                );
                add_use(
                    &mut uses,
                    *idx,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
            }
            ValueKind::Index2D { base, r, c } => {
                add_use(
                    &mut uses,
                    *base,
                    SroaUser::Value(value.id),
                    SroaUseKind::Materialize,
                );
                add_use(
                    &mut uses,
                    *r,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
                add_use(
                    &mut uses,
                    *c,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
            }
            ValueKind::Index3D { base, i, j, k } => {
                add_use(
                    &mut uses,
                    *base,
                    SroaUser::Value(value.id),
                    SroaUseKind::Materialize,
                );
                add_use(
                    &mut uses,
                    *i,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
                add_use(
                    &mut uses,
                    *j,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
                add_use(
                    &mut uses,
                    *k,
                    SroaUser::Value(value.id),
                    SroaUseKind::Reject,
                );
            }
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => {}
        }
    }

    for block in &fn_ir.blocks {
        for (instr_index, instr) in block.instrs.iter().enumerate() {
            match instr {
                Instr::Assign { src, .. } => add_use(
                    &mut uses,
                    *src,
                    SroaUser::Instr {
                        block: block.id,
                        instr: instr_index,
                    },
                    SroaUseKind::Alias,
                ),
                Instr::Eval { val, .. } => add_use(
                    &mut uses,
                    *val,
                    SroaUser::Instr {
                        block: block.id,
                        instr: instr_index,
                    },
                    SroaUseKind::Materialize,
                ),
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    add_store_uses(&mut uses, block.id, instr_index, &[*base, *idx, *val]);
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    add_store_uses(&mut uses, block.id, instr_index, &[*base, *r, *c, *val]);
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    add_store_uses(&mut uses, block.id, instr_index, &[*base, *i, *j, *k, *val]);
                }
            }
        }

        match &block.term {
            Terminator::If { cond, .. } => add_use(
                &mut uses,
                *cond,
                SroaUser::Terminator { block: block.id },
                SroaUseKind::Reject,
            ),
            Terminator::Return(Some(value)) => add_use(
                &mut uses,
                *value,
                SroaUser::Terminator { block: block.id },
                SroaUseKind::Materialize,
            ),
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    uses
}

fn add_store_uses(
    uses: &mut FxHashMap<ValueId, Vec<SroaUse>>,
    block: BlockId,
    instr: usize,
    values: &[ValueId],
) {
    for value in values {
        add_use(
            uses,
            *value,
            SroaUser::Instr { block, instr },
            SroaUseKind::Reject,
        );
    }
}

fn add_use(
    uses: &mut FxHashMap<ValueId, Vec<SroaUse>>,
    value: ValueId,
    user: SroaUser,
    kind: SroaUseKind,
) {
    uses.entry(value).or_default().push(SroaUse { user, kind });
}

#[derive(Debug, Clone)]
struct RecordCallArgSpec {
    arg_index: usize,
    fields: Vec<RecordCallFieldArg>,
}

#[derive(Debug, Clone)]
struct RecordCallFieldArg {
    name: String,
    value: ValueId,
}

#[derive(Debug, Clone)]
struct RecordReturnFieldUse {
    field_get: ValueId,
    base_call: Option<ValueId>,
    field: String,
    callee: String,
    args: Vec<ValueId>,
    names: Vec<Option<String>>,
    alias_var: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RecordReturnAliasTempKey {
    alias_var: String,
    field: String,
    callee: String,
    args: Vec<ValueId>,
    names: Vec<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RecordReturnAliasCallKey {
    alias_var: String,
    callee: String,
    args: Vec<ValueId>,
    names: Vec<Option<String>>,
}

#[derive(Debug, Clone)]
struct RecordReturnAliasInlineState {
    block: BlockId,
    next_insert_index: usize,
    value_map: FxHashMap<ValueId, ValueId>,
    temp_by_field: FxHashMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct RecordReturnDirectCallKey {
    base_call: ValueId,
    callee: String,
    args: Vec<ValueId>,
    names: Vec<Option<String>>,
}

#[derive(Debug, Default, Clone)]
struct RecordReturnDirectInlineState {
    value_map: FxHashMap<ValueId, ValueId>,
    replacement_by_field: FxHashMap<String, ValueId>,
}

pub(super) fn specialize_record_field_calls(all_fns: &mut FxHashMap<String, FnIR>) -> bool {
    let mut ordered_names: Vec<_> = all_fns.keys().cloned().collect();
    ordered_names.sort();

    let mut reserved_names: FxHashSet<_> = all_fns.keys().cloned().collect();
    let mut generated_fns = Vec::new();
    let mut changed = false;

    for caller_name in ordered_names {
        let Some(mut caller) = all_fns.remove(&caller_name) else {
            continue;
        };
        let caller_changed = specialize_record_field_calls_in_caller(
            &mut caller,
            all_fns,
            &mut reserved_names,
            &mut generated_fns,
        );
        if caller_changed {
            changed = true;
            let _ = optimize(&mut caller);
        }
        all_fns.insert(caller_name, caller);
    }

    for (name, fn_ir) in generated_fns {
        all_fns.insert(name, fn_ir);
    }

    changed
}

pub(super) fn specialize_record_return_field_calls(all_fns: &mut FxHashMap<String, FnIR>) -> bool {
    let mut ordered_names: Vec<_> = all_fns.keys().cloned().collect();
    ordered_names.sort();

    let mut reserved_names: FxHashSet<_> = all_fns.keys().cloned().collect();
    let mut generated_fns = Vec::new();
    let mut specialized_names = FxHashMap::default();
    let mut pure_cache = FxHashMap::default();
    let mut changed = false;

    for caller_name in ordered_names {
        let Some(mut caller) = all_fns.remove(&caller_name) else {
            continue;
        };
        let caller_changed = specialize_record_return_fields_in_caller(
            &mut caller,
            all_fns,
            &mut reserved_names,
            &mut generated_fns,
            &mut specialized_names,
            &mut pure_cache,
        );
        if caller_changed {
            changed = true;
            let _ = optimize(&mut caller);
        }
        all_fns.insert(caller_name, caller);
    }

    for (name, fn_ir) in generated_fns {
        all_fns.insert(name, fn_ir);
    }

    changed
}

fn specialize_record_field_calls_in_caller(
    caller: &mut FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    reserved_names: &mut FxHashSet<String>,
    generated_fns: &mut Vec<(String, FnIR)>,
) -> bool {
    if caller.requires_conservative_optimization() || !is_control_flow_rewrite_candidate(caller) {
        return false;
    }

    let before_value_count = caller.values.len();
    let field_maps = infer_rewrite_field_maps(caller);
    let analysis_changed = caller.values.len() != before_value_count;
    if field_maps.is_empty() {
        return analysis_changed;
    }
    let (shapes, _) = infer_candidate_shapes(caller);
    let live_values = collect_live_value_ids(caller);
    let call_sites: Vec<_> = caller
        .values
        .iter()
        .filter(|value| live_values.contains(&value.id))
        .filter_map(|value| {
            let ValueKind::Call {
                callee,
                args,
                names,
            } = &value.kind
            else {
                return None;
            };
            Some((value.id, callee.clone(), args.clone(), names.clone()))
        })
        .collect();

    let mut changed = analysis_changed;
    for (call, callee_name, args, names) in call_sites {
        if names.iter().any(Option::is_some) {
            continue;
        }
        let Some(callee) = all_fns.get(&callee_name) else {
            continue;
        };
        if callee.requires_conservative_optimization()
            || args.len() != callee.params.len()
            || callee.name == caller.name
        {
            continue;
        }
        let arg_specs = collect_record_call_arg_specs(caller, &field_maps, &shapes, callee, &args);
        if arg_specs.is_empty() {
            continue;
        }

        let new_name =
            unique_sroa_specialized_name(&caller.name, &callee_name, call, reserved_names);
        let Some(specialized) = build_record_arg_specialized_callee(callee, &new_name, &arg_specs)
        else {
            continue;
        };
        if apply_record_call_specialization(caller, call, &callee_name, &new_name, &arg_specs) {
            reserved_names.insert(new_name.clone());
            generated_fns.push((new_name, specialized));
            changed = true;
        }
    }

    changed
}

fn specialize_record_return_fields_in_caller(
    caller: &mut FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    reserved_names: &mut FxHashSet<String>,
    generated_fns: &mut Vec<(String, FnIR)>,
    specialized_names: &mut FxHashMap<(String, String), String>,
    pure_cache: &mut FxHashMap<String, bool>,
) -> bool {
    if caller.requires_conservative_optimization() || !is_control_flow_rewrite_candidate(caller) {
        return false;
    }

    let uses = collect_record_return_field_uses(caller, all_fns);
    if uses.is_empty() {
        return false;
    }

    let mut changed = false;
    let mut scalarized_alias_vars = FxHashSet::default();
    let mut alias_temp_vars = FxHashMap::default();
    let mut alias_inline_states = FxHashMap::default();
    let mut direct_inline_states = FxHashMap::default();
    for record_use in uses {
        if !function_is_effect_free(&record_use.callee, all_fns, pure_cache) {
            continue;
        }
        let Some(callee) = all_fns.get(&record_use.callee) else {
            continue;
        };
        if record_use.alias_var.is_none()
            && rewrite_direct_record_return_field_use_with_inlined_value(
                caller,
                all_fns,
                &record_use,
                &mut direct_inline_states,
            )
        {
            changed = true;
            continue;
        }
        if record_use.alias_var.is_some()
            && rewrite_record_return_alias_field_use_with_inlined_temp(
                caller,
                all_fns,
                &record_use,
                &mut alias_inline_states,
            )
        {
            if let Some(alias_var) = record_use.alias_var.as_ref() {
                scalarized_alias_vars.insert(alias_var.clone());
            }
            changed = true;
            continue;
        }
        let cache_key = (record_use.callee.clone(), record_use.field.clone());
        let specialized_name = if let Some(name) = specialized_names.get(&cache_key).cloned() {
            name
        } else {
            let new_name = unique_sroa_return_specialized_name(
                &record_use.callee,
                &record_use.field,
                reserved_names,
            );
            let Some(specialized) =
                build_record_return_field_specialized_callee(callee, &new_name, &record_use.field)
            else {
                continue;
            };
            reserved_names.insert(new_name.clone());
            specialized_names.insert(cache_key, new_name.clone());
            generated_fns.push((new_name.clone(), specialized));
            new_name
        };

        let rewritten = if record_use.alias_var.is_some() {
            rewrite_record_return_alias_field_use_with_shared_temp(
                caller,
                &record_use,
                &specialized_name,
                &mut alias_temp_vars,
            )
        } else {
            rewrite_record_return_field_use(caller, &record_use, &specialized_name)
        };
        if rewritten {
            if let Some(alias_var) = record_use.alias_var.as_ref() {
                scalarized_alias_vars.insert(alias_var.clone());
            }
            changed = true;
        }
    }

    if !scalarized_alias_vars.is_empty() {
        changed |= remove_scalarized_return_aliases(caller, &scalarized_alias_vars);
    }

    changed
}

fn collect_record_call_arg_specs(
    caller: &FnIR,
    field_maps: &FxHashMap<ValueId, SroaFieldMap>,
    shapes: &FxHashMap<ValueId, Vec<String>>,
    callee: &FnIR,
    args: &[ValueId],
) -> Vec<RecordCallArgSpec> {
    let mut specs = Vec::new();
    for (arg_index, arg) in args.iter().copied().enumerate() {
        if arg_index >= callee.params.len() || !field_maps.contains_key(&arg) {
            continue;
        }
        let Some(shape) = materialization_shape(caller, field_maps, shapes, arg) else {
            continue;
        };
        if shape.is_empty() {
            continue;
        }
        let Some(field_map) = field_maps.get(&arg) else {
            continue;
        };
        let mut fields = Vec::with_capacity(shape.len());
        for field in shape {
            let Some(value) = field_map.get(&field).copied() else {
                fields.clear();
                break;
            };
            fields.push(RecordCallFieldArg { name: field, value });
        }
        if !fields.is_empty() {
            specs.push(RecordCallArgSpec { arg_index, fields });
        }
    }
    specs
}

fn collect_record_return_field_uses(
    caller: &FnIR,
    all_fns: &FxHashMap<String, FnIR>,
) -> Vec<RecordReturnFieldUse> {
    let live_values = collect_live_value_ids(caller);
    let unique_assignments = unique_var_assignments(caller);
    let uses = build_use_graph(caller);
    let scalarizable_alias_vars = scalarizable_return_alias_vars(caller, all_fns, &uses);
    let mut out = Vec::new();

    for value in &caller.values {
        if !live_values.contains(&value.id) {
            continue;
        }
        let ValueKind::FieldGet { base, field } = &value.kind else {
            continue;
        };
        if let Some((callee, args, names)) = call_parts(caller, *base)
            && all_fns.contains_key(&callee)
            && callee != caller.name
        {
            out.push(RecordReturnFieldUse {
                field_get: value.id,
                base_call: Some(*base),
                field: field.clone(),
                callee,
                args,
                names,
                alias_var: None,
            });
            continue;
        }
        if let ValueKind::Load { var } = &caller.values[*base].kind {
            if !scalarizable_alias_vars.contains(var) {
                continue;
            }
            let Some(src) = unique_assignments.get(var).copied() else {
                continue;
            };
            let Some((callee, args, names)) = call_parts(caller, src) else {
                continue;
            };
            if all_fns.contains_key(&callee) && callee != caller.name {
                out.push(RecordReturnFieldUse {
                    field_get: value.id,
                    base_call: None,
                    field: field.clone(),
                    callee,
                    args,
                    names,
                    alias_var: Some(var.clone()),
                });
            }
        }
    }

    out
}

fn scalarizable_return_alias_vars(
    caller: &FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    uses: &FxHashMap<ValueId, Vec<SroaUse>>,
) -> FxHashSet<String> {
    let unique_assignments = unique_var_assignments(caller);
    let mut out = FxHashSet::default();

    for (var, src) in unique_assignments {
        let Some((callee, _, _)) = call_parts(caller, src) else {
            continue;
        };
        if !all_fns.contains_key(&callee) || callee == caller.name {
            continue;
        }
        let load_ids: Vec<_> = caller
            .values
            .iter()
            .filter_map(|value| match &value.kind {
                ValueKind::Load { var: load_var } if load_var == &var => Some(value.id),
                _ => None,
            })
            .collect();
        if load_ids.is_empty() {
            continue;
        }
        let all_load_uses_are_field_gets = load_ids.iter().all(|load| {
            uses.get(load).is_some_and(|load_uses| {
                !load_uses.is_empty()
                    && load_uses.iter().all(|load_use| match load_use.user {
                        SroaUser::Value(user) => matches!(
                            &caller.values[user].kind,
                            ValueKind::FieldGet { base, .. } if base == load
                        ),
                        SroaUser::Instr { .. } | SroaUser::Terminator { .. } => false,
                    })
            })
        });
        if all_load_uses_are_field_gets {
            out.insert(var);
        }
    }

    out
}

fn call_parts(fn_ir: &FnIR, value: ValueId) -> Option<(String, Vec<ValueId>, Vec<Option<String>>)> {
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &fn_ir.values.get(value)?.kind
    else {
        return None;
    };
    Some((callee.clone(), args.clone(), names.clone()))
}

fn build_record_return_field_specialized_callee(
    callee: &FnIR,
    new_name: &str,
    field: &str,
) -> Option<FnIR> {
    if callee.requires_conservative_optimization() || !is_control_flow_rewrite_candidate(callee) {
        return None;
    }

    let mut specialized = callee.clone();
    specialized.name = new_name.to_string();
    specialized.user_name = None;
    let field_maps = infer_rewrite_field_maps(&mut specialized);
    let mut rewrites = Vec::new();
    for block in &specialized.blocks {
        match block.term {
            Terminator::Return(Some(ret)) => {
                let replacement = field_maps.get(&ret)?.get(field).copied()?;
                rewrites.push((block.id, replacement));
            }
            Terminator::Return(None) => return None,
            Terminator::Goto(_) | Terminator::If { .. } | Terminator::Unreachable => {}
        }
    }
    if rewrites.is_empty() {
        return None;
    }

    for (block, replacement) in rewrites {
        specialized.blocks[block].term = Terminator::Return(Some(replacement));
    }
    let _ = optimize(&mut specialized);
    Some(specialized)
}

fn rewrite_record_return_field_use(
    caller: &mut FnIR,
    record_use: &RecordReturnFieldUse,
    specialized_name: &str,
) -> bool {
    let Some(value) = caller.values.get_mut(record_use.field_get) else {
        return false;
    };
    value.kind = ValueKind::Call {
        callee: specialized_name.to_string(),
        args: record_use.args.clone(),
        names: record_use.names.clone(),
    };
    caller.set_call_semantics(record_use.field_get, CallSemantics::UserDefined);
    true
}

fn rewrite_direct_record_return_field_use_with_inlined_value(
    caller: &mut FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    record_use: &RecordReturnFieldUse,
    direct_inline_states: &mut FxHashMap<RecordReturnDirectCallKey, RecordReturnDirectInlineState>,
) -> bool {
    let Some(base_call) = record_use.base_call else {
        return false;
    };
    if record_use.names.iter().any(Option::is_some)
        || caller.values.get(record_use.field_get).is_none()
    {
        return false;
    }

    let key = RecordReturnDirectCallKey {
        base_call,
        callee: record_use.callee.clone(),
        args: record_use.args.clone(),
        names: record_use.names.clone(),
    };
    let mut state = direct_inline_states.remove(&key).unwrap_or_default();

    let replacement =
        if let Some(replacement) = state.replacement_by_field.get(&record_use.field).copied() {
            replacement
        } else {
            let Some(callee) = all_fns.get(&record_use.callee) else {
                direct_inline_states.insert(key, state);
                return false;
            };
            let Some((scalarized, field_value)) =
                scalarizable_single_record_return_field(callee, &record_use.field)
            else {
                direct_inline_states.insert(key, state);
                return false;
            };
            let Some(cloned_value) = clone_scalarizable_callee_value(
                caller,
                &scalarized,
                &record_use.args,
                field_value,
                &mut state.value_map,
            ) else {
                direct_inline_states.insert(key, state);
                return false;
            };
            state
                .replacement_by_field
                .insert(record_use.field.clone(), cloned_value);
            cloned_value
        };

    let rewritten =
        rewrite_record_return_field_use_to_value(caller, record_use.field_get, replacement);
    direct_inline_states.insert(key, state);
    rewritten
}

fn rewrite_record_return_alias_field_use_with_inlined_temp(
    caller: &mut FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    record_use: &RecordReturnFieldUse,
    alias_inline_states: &mut FxHashMap<RecordReturnAliasCallKey, RecordReturnAliasInlineState>,
) -> bool {
    let Some(alias_var) = record_use.alias_var.as_ref() else {
        return false;
    };
    if caller.values.get(record_use.field_get).is_none() {
        return false;
    }
    let key = RecordReturnAliasCallKey {
        alias_var: alias_var.clone(),
        callee: record_use.callee.clone(),
        args: record_use.args.clone(),
        names: record_use.names.clone(),
    };
    let Some(mut state) = alias_inline_states
        .remove(&key)
        .or_else(|| create_record_return_alias_inline_state(caller, record_use))
    else {
        return false;
    };

    if let Some(temp_var) = state.temp_by_field.get(&record_use.field).cloned() {
        let rewritten =
            rewrite_record_return_field_use_to_temp_load(caller, record_use.field_get, &temp_var);
        alias_inline_states.insert(key, state);
        return rewritten;
    }

    let Some(callee) = all_fns.get(&record_use.callee) else {
        alias_inline_states.insert(key, state);
        return false;
    };
    let Some((scalarized, field_value)) =
        scalarizable_single_record_return_field(callee, &record_use.field)
    else {
        alias_inline_states.insert(key, state);
        return false;
    };
    let Some(cloned_value) = clone_scalarizable_callee_value(
        caller,
        &scalarized,
        &record_use.args,
        field_value,
        &mut state.value_map,
    ) else {
        alias_inline_states.insert(key, state);
        return false;
    };

    let span = caller.values[record_use.field_get].span;
    let temp_var = unique_sroa_return_temp_var(caller, alias_var, &record_use.field);
    let insert_at = state
        .next_insert_index
        .min(caller.blocks[state.block].instrs.len());
    caller.blocks[state.block].instrs.insert(
        insert_at,
        Instr::Assign {
            dst: temp_var.clone(),
            src: cloned_value,
            span,
        },
    );
    state.next_insert_index = insert_at + 1;
    state
        .temp_by_field
        .insert(record_use.field.clone(), temp_var.clone());

    let rewritten =
        rewrite_record_return_field_use_to_temp_load(caller, record_use.field_get, &temp_var);
    alias_inline_states.insert(key, state);
    rewritten
}

fn create_record_return_alias_inline_state(
    caller: &FnIR,
    record_use: &RecordReturnFieldUse,
) -> Option<RecordReturnAliasInlineState> {
    if record_use.names.iter().any(Option::is_some) {
        return None;
    }
    let (block, instr_index) = find_record_return_alias_assignment(caller, record_use)?;
    Some(RecordReturnAliasInlineState {
        block,
        next_insert_index: instr_index + 1,
        value_map: FxHashMap::default(),
        temp_by_field: FxHashMap::default(),
    })
}

fn scalarizable_single_record_return_field(callee: &FnIR, field: &str) -> Option<(FnIR, ValueId)> {
    if callee.requires_conservative_optimization()
        || !is_control_flow_rewrite_candidate(callee)
        || callee.blocks.len() != 1
    {
        return None;
    }

    let mut scalarized = callee.clone();
    let field_maps = infer_rewrite_field_maps(&mut scalarized);
    let block = scalarized.blocks.get(scalarized.entry)?;
    let Terminator::Return(Some(ret)) = block.term else {
        return None;
    };
    let replacement = field_maps.get(&ret)?.get(field).copied()?;
    Some((scalarized, replacement))
}

fn clone_scalarizable_callee_value(
    caller: &mut FnIR,
    callee: &FnIR,
    args: &[ValueId],
    value: ValueId,
    value_map: &mut FxHashMap<ValueId, ValueId>,
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
            lhs: clone_scalarizable_callee_value(caller, callee, args, lhs, value_map)?,
            rhs: clone_scalarizable_callee_value(caller, callee, args, rhs, value_map)?,
        },
        ValueKind::Unary { op, rhs } => ValueKind::Unary {
            op,
            rhs: clone_scalarizable_callee_value(caller, callee, args, rhs, value_map)?,
        },
        ValueKind::Len { base } => ValueKind::Len {
            base: clone_scalarizable_callee_value(caller, callee, args, base, value_map)?,
        },
        ValueKind::Indices { base } => ValueKind::Indices {
            base: clone_scalarizable_callee_value(caller, callee, args, base, value_map)?,
        },
        ValueKind::Range { start, end } => ValueKind::Range {
            start: clone_scalarizable_callee_value(caller, callee, args, start, value_map)?,
            end: clone_scalarizable_callee_value(caller, callee, args, end, value_map)?,
        },
        ValueKind::RecordLit { fields } => {
            let mut cloned_fields = Vec::with_capacity(fields.len());
            for (field, field_value) in fields {
                cloned_fields.push((
                    field,
                    clone_scalarizable_callee_value(caller, callee, args, field_value, value_map)?,
                ));
            }
            ValueKind::RecordLit {
                fields: cloned_fields,
            }
        }
        ValueKind::FieldGet { base, field } => ValueKind::FieldGet {
            base: clone_scalarizable_callee_value(caller, callee, args, base, value_map)?,
            field,
        },
        ValueKind::FieldSet { base, field, value } => ValueKind::FieldSet {
            base: clone_scalarizable_callee_value(caller, callee, args, base, value_map)?,
            field,
            value: clone_scalarizable_callee_value(caller, callee, args, value, value_map)?,
        },
        ValueKind::Intrinsic { op, args: inputs } => {
            let mut cloned_args = Vec::with_capacity(inputs.len());
            for input in inputs {
                cloned_args.push(clone_scalarizable_callee_value(
                    caller, callee, args, input, value_map,
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
            base: clone_scalarizable_callee_value(caller, callee, args, base, value_map)?,
            idx: clone_scalarizable_callee_value(caller, callee, args, idx, value_map)?,
            is_safe,
            is_na_safe,
        },
        ValueKind::Index2D { base, r, c } => ValueKind::Index2D {
            base: clone_scalarizable_callee_value(caller, callee, args, base, value_map)?,
            r: clone_scalarizable_callee_value(caller, callee, args, r, value_map)?,
            c: clone_scalarizable_callee_value(caller, callee, args, c, value_map)?,
        },
        ValueKind::Index3D { base, i, j, k } => ValueKind::Index3D {
            base: clone_scalarizable_callee_value(caller, callee, args, base, value_map)?,
            i: clone_scalarizable_callee_value(caller, callee, args, i, value_map)?,
            j: clone_scalarizable_callee_value(caller, callee, args, j, value_map)?,
            k: clone_scalarizable_callee_value(caller, callee, args, k, value_map)?,
        },
        ValueKind::RSymbol { name } => ValueKind::RSymbol { name },
        ValueKind::Phi { .. } | ValueKind::Call { .. } | ValueKind::Load { .. } => return None,
    };

    let cloned = caller.add_value(cloned_kind, source.span, source.facts, None);
    caller.values[cloned].value_ty = source.value_ty;
    caller.values[cloned].value_term = source.value_term;
    caller.values[cloned].escape = source.escape;
    value_map.insert(value, cloned);
    Some(cloned)
}

fn rewrite_record_return_field_use_to_value(
    caller: &mut FnIR,
    field_get: ValueId,
    replacement: ValueId,
) -> bool {
    if caller.values.get(field_get).is_none() || caller.values.get(replacement).is_none() {
        return false;
    }
    let mut replacements = FxHashMap::default();
    replacements.insert(field_get, replacement);
    let changed = apply_value_replacements(caller, &replacements);
    if changed && let Some(value) = caller.values.get_mut(field_get) {
        value.kind = ValueKind::Const(Lit::Null);
        value.origin_var = None;
    }
    changed
}

fn rewrite_record_return_field_use_to_temp_load(
    caller: &mut FnIR,
    field_get: ValueId,
    temp_var: &str,
) -> bool {
    let Some(value) = caller.values.get_mut(field_get) else {
        return false;
    };
    value.kind = ValueKind::Load {
        var: temp_var.to_string(),
    };
    value.origin_var = Some(temp_var.to_string());
    true
}

fn rewrite_record_return_alias_field_use_with_shared_temp(
    caller: &mut FnIR,
    record_use: &RecordReturnFieldUse,
    specialized_name: &str,
    alias_temp_vars: &mut FxHashMap<RecordReturnAliasTempKey, String>,
) -> bool {
    let Some(alias_var) = record_use.alias_var.as_ref() else {
        return rewrite_record_return_field_use(caller, record_use, specialized_name);
    };
    let key = RecordReturnAliasTempKey {
        alias_var: alias_var.clone(),
        field: record_use.field.clone(),
        callee: record_use.callee.clone(),
        args: record_use.args.clone(),
        names: record_use.names.clone(),
    };
    let temp_var = if let Some(temp_var) = alias_temp_vars.get(&key).cloned() {
        temp_var
    } else {
        let Some(temp_var) =
            insert_scalarized_return_alias_temp(caller, record_use, specialized_name)
        else {
            return rewrite_record_return_field_use(caller, record_use, specialized_name);
        };
        alias_temp_vars.insert(key, temp_var.clone());
        temp_var
    };

    rewrite_record_return_field_use_to_temp_load(caller, record_use.field_get, &temp_var)
}

fn insert_scalarized_return_alias_temp(
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

fn find_record_return_alias_assignment(
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

fn remove_scalarized_return_aliases(fn_ir: &mut FnIR, vars: &FxHashSet<String>) -> bool {
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

fn function_is_effect_free(
    name: &str,
    all_fns: &FxHashMap<String, FnIR>,
    cache: &mut FxHashMap<String, bool>,
) -> bool {
    if effects::call_is_pure(name) {
        return true;
    }
    if let Some(cached) = cache.get(name).copied() {
        return cached;
    }
    let mut visiting = FxHashSet::default();
    let pure = function_is_effect_free_inner(name, all_fns, cache, &mut visiting);
    cache.insert(name.to_string(), pure);
    pure
}

fn function_is_effect_free_inner(
    name: &str,
    all_fns: &FxHashMap<String, FnIR>,
    cache: &mut FxHashMap<String, bool>,
    visiting: &mut FxHashSet<String>,
) -> bool {
    if effects::call_is_pure(name) {
        return true;
    }
    if let Some(cached) = cache.get(name).copied() {
        return cached;
    }
    if !visiting.insert(name.to_string()) {
        return false;
    }
    let Some(fn_ir) = all_fns.get(name) else {
        visiting.remove(name);
        return false;
    };
    if fn_ir.requires_conservative_optimization() {
        visiting.remove(name);
        return false;
    }

    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Assign { src, .. } => {
                    if !value_is_effect_free_in_program(*src, fn_ir, all_fns, cache, visiting) {
                        visiting.remove(name);
                        return false;
                    }
                }
                Instr::Eval { .. }
                | Instr::StoreIndex1D { .. }
                | Instr::StoreIndex2D { .. }
                | Instr::StoreIndex3D { .. } => {
                    visiting.remove(name);
                    return false;
                }
            }
        }
        match &block.term {
            Terminator::If { cond, .. } | Terminator::Return(Some(cond)) => {
                if !value_is_effect_free_in_program(*cond, fn_ir, all_fns, cache, visiting) {
                    visiting.remove(name);
                    return false;
                }
            }
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    visiting.remove(name);
    cache.insert(name.to_string(), true);
    true
}

fn value_is_effect_free_in_program(
    value: ValueId,
    fn_ir: &FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    cache: &mut FxHashMap<String, bool>,
    visiting_fns: &mut FxHashSet<String>,
) -> bool {
    fn rec(
        value: ValueId,
        fn_ir: &FnIR,
        all_fns: &FxHashMap<String, FnIR>,
        cache: &mut FxHashMap<String, bool>,
        visiting_fns: &mut FxHashSet<String>,
        visiting_values: &mut FxHashSet<ValueId>,
    ) -> bool {
        if !visiting_values.insert(value) {
            return false;
        }
        let pure = match &fn_ir.values[value].kind {
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => true,
            ValueKind::Call { callee, args, .. } => {
                args.iter()
                    .all(|arg| rec(*arg, fn_ir, all_fns, cache, visiting_fns, visiting_values))
                    && (effects::call_is_pure(callee)
                        || function_is_effect_free_inner(callee, all_fns, cache, visiting_fns))
            }
            _ => value_dependencies(&fn_ir.values[value].kind)
                .into_iter()
                .all(|dep| rec(dep, fn_ir, all_fns, cache, visiting_fns, visiting_values)),
        };
        visiting_values.remove(&value);
        pure
    }

    rec(
        value,
        fn_ir,
        all_fns,
        cache,
        visiting_fns,
        &mut FxHashSet::default(),
    )
}

fn build_record_arg_specialized_callee(
    callee: &FnIR,
    new_name: &str,
    specs: &[RecordCallArgSpec],
) -> Option<FnIR> {
    let spec_by_param: FxHashMap<_, _> = specs.iter().map(|spec| (spec.arg_index, spec)).collect();
    let param_value_owner = specialized_param_value_owners(callee, specs);
    if !callee_record_param_uses_are_specializable(callee, specs, &param_value_owner) {
        return None;
    }

    let mut used_params = FxHashSet::default();
    let mut new_params = Vec::new();
    let mut new_param_defaults = Vec::new();
    let mut new_param_spans = Vec::new();
    let mut new_param_ty_hints = Vec::new();
    let mut new_param_term_hints = Vec::new();
    let mut new_param_hint_spans = Vec::new();
    let mut old_param_to_new = FxHashMap::default();
    let mut field_param_indices: FxHashMap<(usize, String), usize> = FxHashMap::default();

    for old_index in 0..callee.params.len() {
        if let Some(spec) = spec_by_param.get(&old_index).copied() {
            for (field_index, field) in spec.fields.iter().enumerate() {
                let param_name = unique_field_param_name(
                    callee,
                    old_index,
                    field_index,
                    &field.name,
                    &mut used_params,
                );
                let new_index = new_params.len();
                new_params.push(param_name);
                new_param_defaults.push(None);
                new_param_spans.push(param_span_at(callee, old_index));
                new_param_ty_hints.push(TypeState::unknown());
                new_param_term_hints.push(TypeTerm::Any);
                new_param_hint_spans.push(None);
                field_param_indices.insert((old_index, field.name.clone()), new_index);
            }
        } else {
            let new_index = new_params.len();
            let param_name = callee.params[old_index].clone();
            used_params.insert(param_name.clone());
            new_params.push(param_name);
            new_param_defaults.push(param_default_at(callee, old_index));
            new_param_spans.push(param_span_at(callee, old_index));
            new_param_ty_hints.push(param_ty_hint_at(callee, old_index));
            new_param_term_hints.push(param_term_hint_at(callee, old_index));
            new_param_hint_spans.push(param_hint_span_at(callee, old_index));
            old_param_to_new.insert(old_index, new_index);
        }
    }

    let mut field_get_param_indices = FxHashMap::default();
    for value in &callee.values {
        let ValueKind::FieldGet { base, field } = &value.kind else {
            continue;
        };
        let Some(param_index) = param_value_owner.get(base).copied() else {
            continue;
        };
        let new_index = field_param_indices
            .get(&(param_index, field.clone()))
            .copied()?;
        field_get_param_indices.insert(value.id, new_index);
    }

    let mut specialized = callee.clone();
    specialized.name = new_name.to_string();
    specialized.user_name = None;
    specialized.params = new_params;
    specialized.param_default_r_exprs = new_param_defaults;
    specialized.param_spans = new_param_spans;
    specialized.param_ty_hints = new_param_ty_hints;
    specialized.param_term_hints = new_param_term_hints;
    specialized.param_hint_spans = new_param_hint_spans;

    for value in &mut specialized.values {
        if let Some(param_index) = field_get_param_indices.get(&value.id).copied() {
            value.kind = ValueKind::Param { index: param_index };
            value.origin_var = specialized.params.get(param_index).cloned();
            continue;
        }
        if let ValueKind::Param { index } = value.kind {
            if spec_by_param.contains_key(&index) {
                value.kind = ValueKind::Const(Lit::Null);
                value.origin_var = None;
            } else if let Some(new_index) = old_param_to_new.get(&index).copied() {
                value.kind = ValueKind::Param { index: new_index };
                value.origin_var = specialized.params.get(new_index).cloned();
            } else {
                return None;
            }
        }
    }

    Some(specialized)
}

fn specialized_param_value_owners(
    callee: &FnIR,
    specs: &[RecordCallArgSpec],
) -> FxHashMap<ValueId, usize> {
    let specialized_params: FxHashSet<_> = specs.iter().map(|spec| spec.arg_index).collect();
    callee
        .values
        .iter()
        .filter_map(|value| match value.kind {
            ValueKind::Param { index } if specialized_params.contains(&index) => {
                Some((value.id, index))
            }
            _ => None,
        })
        .collect()
}

fn callee_record_param_uses_are_specializable(
    callee: &FnIR,
    specs: &[RecordCallArgSpec],
    param_value_owner: &FxHashMap<ValueId, usize>,
) -> bool {
    if param_value_owner.is_empty() {
        return true;
    }

    let allowed_fields: FxHashMap<usize, FxHashSet<String>> = specs
        .iter()
        .map(|spec| {
            (
                spec.arg_index,
                spec.fields.iter().map(|field| field.name.clone()).collect(),
            )
        })
        .collect();

    for value in &callee.values {
        if let ValueKind::FieldGet { base, field } = &value.kind
            && let Some(param_index) = param_value_owner.get(base)
            && allowed_fields
                .get(param_index)
                .is_some_and(|fields| fields.contains(field))
        {
            continue;
        }
        if value_dependencies(&value.kind)
            .iter()
            .any(|dep| param_value_owner.contains_key(dep))
        {
            return false;
        }
    }

    let specialized_param_names: FxHashSet<_> = specs
        .iter()
        .filter_map(|spec| callee.params.get(spec.arg_index).cloned())
        .collect();
    for block in &callee.blocks {
        for instr in &block.instrs {
            if instr_assigns_any(instr, &specialized_param_names)
                || instr_refs_any(instr, param_value_owner)
            {
                return false;
            }
        }
        if terminator_refs_any(&block.term, param_value_owner) {
            return false;
        }
    }

    true
}

fn instr_assigns_any(instr: &Instr, vars: &FxHashSet<String>) -> bool {
    matches!(instr, Instr::Assign { dst, .. } if vars.contains(dst))
}

fn instr_refs_any(instr: &Instr, values: &FxHashMap<ValueId, usize>) -> bool {
    match instr {
        Instr::Assign { src, .. } => values.contains_key(src),
        Instr::Eval { val, .. } => values.contains_key(val),
        Instr::StoreIndex1D { base, idx, val, .. } => {
            values.contains_key(base) || values.contains_key(idx) || values.contains_key(val)
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            values.contains_key(base)
                || values.contains_key(r)
                || values.contains_key(c)
                || values.contains_key(val)
        }
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => {
            values.contains_key(base)
                || values.contains_key(i)
                || values.contains_key(j)
                || values.contains_key(k)
                || values.contains_key(val)
        }
    }
}

fn terminator_refs_any(term: &Terminator, values: &FxHashMap<ValueId, usize>) -> bool {
    match term {
        Terminator::If { cond, .. } => values.contains_key(cond),
        Terminator::Return(Some(value)) => values.contains_key(value),
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => false,
    }
}

fn apply_record_call_specialization(
    caller: &mut FnIR,
    call: ValueId,
    old_callee: &str,
    new_callee: &str,
    specs: &[RecordCallArgSpec],
) -> bool {
    let spec_by_arg: FxHashMap<_, _> = specs.iter().map(|spec| (spec.arg_index, spec)).collect();
    let Some(value) = caller.values.get_mut(call) else {
        return false;
    };
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &mut value.kind
    else {
        return false;
    };
    if callee != old_callee {
        return false;
    }

    let old_args = args.clone();
    let mut new_args = Vec::new();
    for (arg_index, arg) in old_args.iter().copied().enumerate() {
        if let Some(spec) = spec_by_arg.get(&arg_index).copied() {
            new_args.extend(spec.fields.iter().map(|field| field.value));
        } else {
            new_args.push(arg);
        }
    }
    *callee = new_callee.to_string();
    *args = new_args;
    *names = vec![None; args.len()];
    true
}

fn unique_sroa_specialized_name(
    caller_name: &str,
    callee_name: &str,
    call: ValueId,
    reserved_names: &FxHashSet<String>,
) -> String {
    let base = format!(
        "{}__rr_sroa_{}__call{}",
        sanitize_symbol_segment(callee_name),
        sanitize_symbol_segment(caller_name),
        call
    );
    if !reserved_names.contains(&base) {
        return base;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}_{suffix}");
        if !reserved_names.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn unique_sroa_return_specialized_name(
    callee_name: &str,
    field: &str,
    reserved_names: &FxHashSet<String>,
) -> String {
    let base = format!(
        "{}__rr_sroa_ret_{}",
        sanitize_symbol_segment(callee_name),
        sanitize_symbol_segment(field)
    );
    if !reserved_names.contains(&base) {
        return base;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}_{suffix}");
        if !reserved_names.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn unique_sroa_return_temp_var(fn_ir: &FnIR, alias_var: &str, field: &str) -> String {
    let used = used_var_names(fn_ir);
    let base = format!(
        "{}__rr_sroa_ret_{}",
        sanitize_symbol_segment(alias_var),
        sanitize_symbol_segment(field)
    );
    if !used.contains(&base) {
        return base;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{base}_{suffix}");
        if !used.contains(&candidate) {
            return candidate;
        }
        suffix += 1;
    }
}

fn used_var_names(fn_ir: &FnIR) -> FxHashSet<String> {
    let mut used: FxHashSet<String> = fn_ir.params.iter().cloned().collect();
    for value in &fn_ir.values {
        if let Some(origin) = value.origin_var.as_ref() {
            used.insert(origin.clone());
        }
        if let ValueKind::Load { var } = &value.kind {
            used.insert(var.clone());
        }
    }
    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            if let Instr::Assign { dst, .. } = instr {
                used.insert(dst.clone());
            }
        }
    }
    used
}

fn unique_field_param_name(
    callee: &FnIR,
    param_index: usize,
    field_index: usize,
    field: &str,
    used_params: &mut FxHashSet<String>,
) -> String {
    let base = callee
        .params
        .get(param_index)
        .map(|param| sanitize_symbol_segment(param))
        .unwrap_or_else(|| format!("arg{param_index}"));
    let field = sanitize_symbol_segment(field);
    let seed = format!("{base}__rr_sroa_{field_index}_{field}");
    if used_params.insert(seed.clone()) {
        return seed;
    }
    let mut suffix = 1usize;
    loop {
        let candidate = format!("{seed}_{suffix}");
        if used_params.insert(candidate.clone()) {
            return candidate;
        }
        suffix += 1;
    }
}

fn sanitize_symbol_segment(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        out.push_str("sym");
    }
    if out
        .as_bytes()
        .first()
        .is_some_and(|first| first.is_ascii_digit())
    {
        out.insert(0, '_');
    }
    out
}

fn param_default_at(fn_ir: &FnIR, index: usize) -> Option<String> {
    fn_ir
        .param_default_r_exprs
        .get(index)
        .cloned()
        .unwrap_or(None)
}

fn param_span_at(fn_ir: &FnIR, index: usize) -> Span {
    fn_ir.param_spans.get(index).copied().unwrap_or_default()
}

fn param_ty_hint_at(fn_ir: &FnIR, index: usize) -> TypeState {
    fn_ir
        .param_ty_hints
        .get(index)
        .cloned()
        .unwrap_or_else(TypeState::unknown)
}

fn param_term_hint_at(fn_ir: &FnIR, index: usize) -> TypeTerm {
    fn_ir
        .param_term_hints
        .get(index)
        .cloned()
        .unwrap_or(TypeTerm::Any)
}

fn param_hint_span_at(fn_ir: &FnIR, index: usize) -> Option<Span> {
    fn_ir.param_hint_spans.get(index).copied().unwrap_or(None)
}

impl TachyonEngine {
    pub(super) fn sroa_trace_enabled() -> bool {
        std::env::var_os("RR_SROA_TRACE").is_some()
    }

    pub(super) fn sroa_trace_verbose() -> bool {
        std::env::var("RR_SROA_TRACE")
            .map(|raw| {
                matches!(
                    raw.trim().to_ascii_lowercase().as_str(),
                    "2" | "detail" | "debug" | "verbose"
                )
            })
            .unwrap_or(false)
    }

    pub(super) fn debug_sroa_candidates(all_fns: &FxHashMap<String, FnIR>) {
        if !Self::sroa_trace_enabled() {
            return;
        }

        for name in Self::sorted_fn_names(all_fns) {
            let Some(fn_ir) = all_fns.get(&name) else {
                continue;
            };
            let analysis = analyze_function(fn_ir);
            let counts = analysis.counts();
            if counts.candidates == 0 {
                continue;
            }
            eprintln!(
                "   [sroa-cand] {} candidates={} record={} field-set={} phi={} alias={} scalar={} remat={} reject={}",
                name,
                counts.candidates,
                counts.record_lits,
                counts.field_sets,
                counts.phis,
                counts.load_aliases,
                counts.scalar_only,
                counts.needs_rematerialization,
                counts.rejected
            );
            if Self::sroa_trace_verbose() {
                for candidate in analysis.candidates {
                    eprintln!(
                        "      value={} source={:?} shape={:?} status={:?} uses={} rejects={:?}",
                        candidate.value,
                        candidate.source,
                        candidate.shape,
                        candidate.status,
                        candidate.uses.len(),
                        candidate.reject_reasons
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::Span;

    fn test_fn() -> FnIR {
        let mut fn_ir = FnIR::new("sroa_test".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
        fn_ir
    }

    fn int_value(fn_ir: &mut FnIR, value: i64) -> ValueId {
        fn_ir.add_value(
            ValueKind::Const(Lit::Int(value)),
            Span::default(),
            Facts::empty(),
            None,
        )
    }

    fn record_xy(fn_ir: &mut FnIR, x: ValueId, y: ValueId) -> ValueId {
        fn_ir.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), x), ("y".to_string(), y)],
            },
            Span::default(),
            Facts::empty(),
            None,
        )
    }

    fn binary_value(fn_ir: &mut FnIR, op: BinOp, lhs: ValueId, rhs: ValueId) -> ValueId {
        fn_ir.add_value(
            ValueKind::Binary { op, lhs, rhs },
            Span::default(),
            Facts::empty(),
            None,
        )
    }

    fn record_pos_mass(fn_ir: &mut FnIR, pos: ValueId, mass: ValueId) -> ValueId {
        fn_ir.add_value(
            ValueKind::RecordLit {
                fields: vec![("pos".to_string(), pos), ("mass".to_string(), mass)],
            },
            Span::default(),
            Facts::empty(),
            None,
        )
    }

    fn sum_xy_fn() -> FnIR {
        let mut fn_ir = FnIR::new("sum_xy".to_string(), vec!["p".to_string()]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
        let p = fn_ir.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("p".to_string()),
        );
        let x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: p,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let y = fn_ir.add_value(
            ValueKind::FieldGet {
                base: p,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let sum = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: x,
                rhs: y,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::Return(Some(sum));
        fn_ir
    }

    fn make_xy_fn() -> FnIR {
        let mut fn_ir = FnIR::new("make_xy".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[entry].term = Terminator::Return(Some(record));
        fn_ir
    }

    fn branch_make_xy_fn() -> FnIR {
        let mut fn_ir = FnIR::new("branch_make_xy".to_string(), vec![]);
        let entry = fn_ir.add_block();
        let then_bb = fn_ir.add_block();
        let else_bb = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let then_x = int_value(&mut fn_ir, 1);
        let then_y = int_value(&mut fn_ir, 2);
        let then_record = record_xy(&mut fn_ir, then_x, then_y);
        let else_x = int_value(&mut fn_ir, 3);
        let else_y = int_value(&mut fn_ir, 4);
        let else_record = record_xy(&mut fn_ir, else_x, else_y);

        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb,
            else_bb,
        };
        fn_ir.blocks[then_bb].term = Terminator::Return(Some(then_record));
        fn_ir.blocks[else_bb].term = Terminator::Return(Some(else_record));
        fn_ir
    }

    fn impure_make_xy_fn() -> FnIR {
        let mut fn_ir = FnIR::new("impure_make_xy".to_string(), vec![]);
        let entry = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;
        let side_effect = fn_ir.add_value(
            ValueKind::Call {
                callee: "print".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].instrs.push(Instr::Eval {
            val: side_effect,
            span: Span::default(),
        });
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[entry].term = Terminator::Return(Some(record));
        fn_ir
    }

    #[test]
    fn sroa_rewrites_direct_record_field_get() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: record,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

        assert!(optimize(&mut fn_ir), "expected SROA rewrite");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].term,
            Terminator::Return(Some(ret)) if ret == x
        ));
    }

    #[test]
    fn sroa_rewrites_nested_record_field_get_in_one_pass() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let mass = int_value(&mut fn_ir, 3);
        let pos = record_xy(&mut fn_ir, x, y);
        let body = record_pos_mass(&mut fn_ir, pos, mass);
        let get_pos = fn_ir.add_value(
            ValueKind::FieldGet {
                base: body,
                field: "pos".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: get_pos,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

        assert!(optimize(&mut fn_ir), "expected nested SROA rewrite");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].term,
            Terminator::Return(Some(ret)) if ret == x
        ));
    }

    #[test]
    fn sroa_scalarizes_straight_line_record_chain_to_projected_scalar() {
        let mut fn_ir = test_fn();
        let ax = int_value(&mut fn_ir, 10);
        let ay = int_value(&mut fn_ir, 15);
        let vx = int_value(&mut fn_ir, 2);
        let vy = int_value(&mut fn_ir, -3);
        let dt = int_value(&mut fn_ir, 2);

        let moved_x = binary_value(&mut fn_ir, BinOp::Add, ax, vx);
        let moved_y = binary_value(&mut fn_ir, BinOp::Add, ay, vy);
        let moved = record_xy(&mut fn_ir, moved_x, moved_y);
        let rebound_x = fn_ir.add_value(
            ValueKind::Unary {
                op: UnaryOp::Neg,
                rhs: vx,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rebound_y = fn_ir.add_value(
            ValueKind::Unary {
                op: UnaryOp::Neg,
                rhs: vy,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rebound = record_xy(&mut fn_ir, rebound_x, rebound_y);
        let moved_get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: moved,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rebound_get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: rebound,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let translated_x = binary_value(&mut fn_ir, BinOp::Add, moved_get_x, rebound_get_x);
        let moved_get_y = fn_ir.add_value(
            ValueKind::FieldGet {
                base: moved,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rebound_get_y = fn_ir.add_value(
            ValueKind::FieldGet {
                base: rebound,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let translated_y = binary_value(&mut fn_ir, BinOp::Add, moved_get_y, rebound_get_y);
        let translated = record_xy(&mut fn_ir, translated_x, translated_y);
        let translated_get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: translated,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let final_x = binary_value(&mut fn_ir, BinOp::Mul, translated_get_x, dt);
        let translated_get_y = fn_ir.add_value(
            ValueKind::FieldGet {
                base: translated,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let final_y = binary_value(&mut fn_ir, BinOp::Mul, translated_get_y, dt);
        let final_record = record_xy(&mut fn_ir, final_x, final_y);
        let projected = fn_ir.add_value(
            ValueKind::FieldGet {
                base: final_record,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(projected));

        assert!(optimize(&mut fn_ir), "expected chained SROA rewrite");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].term,
            Terminator::Return(Some(ret)) if ret == final_x
        ));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_rewrites_single_load_alias_field_get() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let get_y = fn_ir.add_value(
            ValueKind::FieldGet {
                base: load,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_y));

        assert!(optimize(&mut fn_ir), "expected SROA alias rewrite");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].term,
            Terminator::Return(Some(ret)) if ret == y
        ));
        assert!(
            fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
            "dead scalarized aggregate assignment should be removed"
        );
        assert!(
            matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
            "dead aggregate load alias should be neutralized with its assignment"
        );
    }

    #[test]
    fn sroa_rematerializes_returned_alias_and_removes_dead_assignment() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(load));

        assert!(optimize(&mut fn_ir), "expected return rematerialization");
        let Terminator::Return(Some(ret)) = fn_ir.blocks[fn_ir.entry].term else {
            panic!("entry block should return a rematerialized record");
        };
        assert_ne!(ret, load);
        assert!(matches!(
            &fn_ir.values[ret].kind,
            ValueKind::RecordLit { fields }
                if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
        ));
        assert!(
            fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
            "dead aggregate alias assignment should be removed after rematerialization"
        );
        assert!(
            matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
            "dead return alias load should be neutralized with its assignment"
        );
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_rematerializes_unknown_call_alias_arg_and_removes_dead_assignment() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let call = fn_ir.add_value(
            ValueKind::Call {
                callee: "opaque_helper".to_string(),
                args: vec![load],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(call));

        assert!(
            optimize(&mut fn_ir),
            "expected call argument rematerialization"
        );
        let ValueKind::Call { args, .. } = &fn_ir.values[call].kind else {
            panic!("call value should remain a call");
        };
        assert_eq!(args.len(), 1);
        assert_ne!(args[0], load);
        assert!(matches!(
            &fn_ir.values[args[0]].kind,
            ValueKind::RecordLit { fields }
                if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
        ));
        assert!(
            fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
            "dead aggregate alias assignment should be removed after call rematerialization"
        );
        assert!(
            matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
            "dead call alias load should be neutralized with its assignment"
        );
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_rematerializes_intrinsic_alias_arg_and_removes_dead_assignment() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let intrinsic = fn_ir.add_value(
            ValueKind::Intrinsic {
                op: IntrinsicOp::VecMeanF64,
                args: vec![load],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(intrinsic));

        assert!(
            optimize(&mut fn_ir),
            "expected intrinsic argument rematerialization"
        );
        let ValueKind::Intrinsic { args, .. } = &fn_ir.values[intrinsic].kind else {
            panic!("intrinsic value should remain an intrinsic");
        };
        assert_eq!(args.len(), 1);
        assert_ne!(args[0], load);
        assert!(matches!(
            &fn_ir.values[args[0]].kind,
            ValueKind::RecordLit { fields }
                if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
        ));
        assert!(
            fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
            "dead aggregate alias assignment should be removed after intrinsic rematerialization"
        );
        assert!(matches!(
            fn_ir.values[load].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_rematerializes_eval_alias_and_removes_dead_assignment() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let done = int_value(&mut fn_ir, 0);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
            val: load,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(done));

        assert!(
            optimize(&mut fn_ir),
            "expected eval boundary rematerialization"
        );
        assert_eq!(fn_ir.blocks[fn_ir.entry].instrs.len(), 1);
        let Instr::Eval { val, .. } = fn_ir.blocks[fn_ir.entry].instrs[0] else {
            panic!("aggregate assignment should be removed and eval should remain");
        };
        assert_ne!(val, load);
        assert!(matches!(
            &fn_ir.values[val].kind,
            ValueKind::RecordLit { fields }
                if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
        ));
        assert!(
            matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
            "dead eval alias load should be neutralized with its assignment"
        );
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_rematerializes_nested_record_alias_field_and_removes_dead_assignment() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let mass = int_value(&mut fn_ir, 3);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "pos".to_string(),
            src: record,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "pos".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("pos".to_string()),
        );
        let body = record_pos_mass(&mut fn_ir, load, mass);
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(body));

        assert!(
            optimize(&mut fn_ir),
            "expected nested record field rematerialization"
        );
        let ValueKind::RecordLit { fields } = &fn_ir.values[body].kind else {
            panic!("outer record should remain a record literal");
        };
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "pos");
        assert_ne!(fields[0].1, load);
        assert!(matches!(
            &fn_ir.values[fields[0].1].kind,
            ValueKind::RecordLit { fields }
                if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
        ));
        assert_eq!(fields[1], ("mass".to_string(), mass));
        assert!(
            fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
            "dead nested aggregate alias assignment should be removed"
        );
        assert!(
            matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
            "dead nested alias load should be neutralized with its assignment"
        );
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_rematerializes_index_base_alias_and_removes_dead_assignment() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let idx = int_value(&mut fn_ir, 1);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let indexed = fn_ir.add_value(
            ValueKind::Index1D {
                base: load,
                idx,
                is_safe: false,
                is_na_safe: false,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(indexed));

        assert!(
            optimize(&mut fn_ir),
            "expected index base rematerialization"
        );
        let ValueKind::Index1D {
            base, idx: got_idx, ..
        } = &fn_ir.values[indexed].kind
        else {
            panic!("indexed value should remain an Index1D");
        };
        assert_ne!(*base, load);
        assert_eq!(*got_idx, idx);
        assert!(matches!(
            &fn_ir.values[*base].kind,
            ValueKind::RecordLit { fields }
                if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
        ));
        assert!(
            fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
            "dead index base alias assignment should be removed"
        );
        assert!(matches!(
            fn_ir.values[load].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_rematerializes_len_base_alias_and_removes_dead_assignment() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let len = fn_ir.add_value(
            ValueKind::Len { base: load },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(len));

        assert!(optimize(&mut fn_ir), "expected len base rematerialization");
        let ValueKind::Len { base } = &fn_ir.values[len].kind else {
            panic!("len value should remain a Len");
        };
        assert_ne!(*base, load);
        assert!(matches!(
            &fn_ir.values[*base].kind,
            ValueKind::RecordLit { fields }
                if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
        ));
        assert!(
            fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
            "dead len base alias assignment should be removed"
        );
        assert!(matches!(
            fn_ir.values[load].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_does_not_drop_impure_unused_record_fields() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let record = record_xy(&mut fn_ir, x, impure);
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: record,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

        assert!(
            !optimize(&mut fn_ir),
            "SROA must not remove record construction when it would drop an impure field"
        );
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].term,
            Terminator::Return(Some(ret)) if ret == get_x
        ));
    }

    #[test]
    fn sroa_snapshots_record_field_load_before_reassignment() {
        let mut fn_ir = test_fn();
        let initial = int_value(&mut fn_ir, 1);
        let replacement = int_value(&mut fn_ir, 2);
        let y = int_value(&mut fn_ir, 3);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: initial,
            span: Span::default(),
        });
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let record = record_xy(&mut fn_ir, load_x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: replacement,
            span: Span::default(),
        });
        let load_point = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: load_point,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

        assert!(
            optimize(&mut fn_ir),
            "SROA should snapshot load fields at the aggregate alias assignment"
        );
        let Terminator::Return(Some(ret)) = fn_ir.blocks[fn_ir.entry].term else {
            panic!("entry block should still return a value");
        };
        let ValueKind::Load { var } = &fn_ir.values[ret].kind else {
            panic!("projected field should load the snapshot temp");
        };
        assert!(var.contains("__rr_sroa_snap_x"));
        assert!(matches!(
            &fn_ir.blocks[fn_ir.entry].instrs[..],
            [
                Instr::Assign {
                    dst: initial_dst,
                    src: initial_src,
                    ..
                },
                Instr::Assign { dst, src, .. },
                Instr::Assign {
                    dst: reassigned,
                    src: reassigned_src,
                    ..
                },
            ] if initial_dst == "x"
                && *initial_src == initial
                && dst == var
                && *src == load_x
                && reassigned == "x"
                && *reassigned_src == replacement
        ));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_snapshots_record_field_expression_before_reassignment() {
        let mut fn_ir = test_fn();
        let initial = int_value(&mut fn_ir, 1);
        let replacement = int_value(&mut fn_ir, 2);
        let y = int_value(&mut fn_ir, 3);
        let one = int_value(&mut fn_ir, 1);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: initial,
            span: Span::default(),
        });
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let computed_x = binary_value(&mut fn_ir, BinOp::Add, load_x, one);
        let record = record_xy(&mut fn_ir, computed_x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: replacement,
            span: Span::default(),
        });
        let load_point = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: load_point,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

        assert!(
            optimize(&mut fn_ir),
            "SROA should snapshot pure load-dependent field expressions"
        );
        let Terminator::Return(Some(ret)) = fn_ir.blocks[fn_ir.entry].term else {
            panic!("entry block should still return a value");
        };
        let ValueKind::Load { var } = &fn_ir.values[ret].kind else {
            panic!("projected field should load the expression snapshot temp");
        };
        assert!(var.contains("__rr_sroa_snap_x"));
        assert!(matches!(
            &fn_ir.blocks[fn_ir.entry].instrs[..],
            [
                Instr::Assign {
                    dst: initial_dst,
                    src: initial_src,
                    ..
                },
                Instr::Assign { dst, src, .. },
                Instr::Assign {
                    dst: reassigned,
                    src: reassigned_src,
                    ..
                },
            ] if initial_dst == "x"
                && *initial_src == initial
                && dst == var
                && *src == computed_x
                && reassigned == "x"
                && *reassigned_src == replacement
        ));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_rematerializes_snapshot_record_alias_return_after_reassignment() {
        let mut fn_ir = test_fn();
        let initial = int_value(&mut fn_ir, 1);
        let replacement = int_value(&mut fn_ir, 2);
        let y = int_value(&mut fn_ir, 3);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: initial,
            span: Span::default(),
        });
        let load_x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let record = record_xy(&mut fn_ir, load_x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: replacement,
            span: Span::default(),
        });
        let load_point = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(load_point));

        assert!(
            optimize(&mut fn_ir),
            "SROA should rematerialize a returned alias from snapshot fields"
        );
        let Terminator::Return(Some(ret)) = fn_ir.blocks[fn_ir.entry].term else {
            panic!("entry block should return a rematerialized record");
        };
        let ValueKind::RecordLit { fields } = &fn_ir.values[ret].kind else {
            panic!("returned value should rematerialize as a record literal");
        };
        assert_eq!(fields.len(), 2);
        let ValueKind::Load { var } = &fn_ir.values[fields[0].1].kind else {
            panic!("x field should be loaded from its snapshot temp");
        };
        assert!(var.contains("__rr_sroa_snap_x"));
        assert_eq!(fields[1], ("y".to_string(), y));
        assert!(fn_ir.blocks[fn_ir.entry].instrs.iter().any(
            |instr| matches!(instr, Instr::Assign { dst, src, .. } if dst == var && *src == load_x)
        ));
        assert!(matches!(
            fn_ir.values[load_point].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_rewrites_field_set_updated_field() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let replacement = int_value(&mut fn_ir, 3);
        let record = record_xy(&mut fn_ir, x, y);
        let updated = fn_ir.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "x".to_string(),
                value: replacement,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: updated,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

        assert!(optimize(&mut fn_ir), "expected FieldSet SROA rewrite");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].term,
            Terminator::Return(Some(ret)) if ret == replacement
        ));
    }

    #[test]
    fn sroa_rewrites_field_set_unchanged_field() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let replacement = int_value(&mut fn_ir, 3);
        let record = record_xy(&mut fn_ir, x, y);
        let updated = fn_ir.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "x".to_string(),
                value: replacement,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_y = fn_ir.add_value(
            ValueKind::FieldGet {
                base: updated,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_y));

        assert!(optimize(&mut fn_ir), "expected FieldSet SROA rewrite");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].term,
            Terminator::Return(Some(ret)) if ret == y
        ));
    }

    #[test]
    fn sroa_rewrites_field_set_alias_and_removes_dead_assignment() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let replacement = int_value(&mut fn_ir, 4);
        let record = record_xy(&mut fn_ir, x, y);
        let updated = fn_ir.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "y".to_string(),
                value: replacement,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: updated,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let get_y = fn_ir.add_value(
            ValueKind::FieldGet {
                base: load,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_y));

        assert!(optimize(&mut fn_ir), "expected FieldSet alias SROA rewrite");
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].term,
            Terminator::Return(Some(ret)) if ret == replacement
        ));
        assert!(
            fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
            "dead scalarized FieldSet assignment should be removed"
        );
    }

    #[test]
    fn sroa_does_not_drop_impure_field_set_update() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let record = record_xy(&mut fn_ir, x, y);
        let updated = fn_ir.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "x".to_string(),
                value: impure,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_y = fn_ir.add_value(
            ValueKind::FieldGet {
                base: updated,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_y));

        assert!(
            !optimize(&mut fn_ir),
            "SROA must not remove an impure FieldSet update even when reading another field"
        );
        assert!(matches!(
            fn_ir.blocks[fn_ir.entry].term,
            Terminator::Return(Some(ret)) if ret == get_y
        ));
    }

    #[test]
    fn sroa_rematerializes_field_set_alias_base_with_impure_update() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let updated = fn_ir.add_value(
            ValueKind::FieldSet {
                base: load,
                field: "x".to_string(),
                value: impure,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(updated));

        assert!(
            optimize(&mut fn_ir),
            "expected FieldSet base rematerialization"
        );
        let ValueKind::FieldSet { base, value, .. } = &fn_ir.values[updated].kind else {
            panic!("updated value should remain a functional FieldSet");
        };
        assert_ne!(*base, load);
        assert_eq!(*value, impure);
        assert!(matches!(
            &fn_ir.values[*base].kind,
            ValueKind::RecordLit { fields }
                if fields == &vec![("x".to_string(), x), ("y".to_string(), y)]
        ));
        assert!(
            fn_ir.blocks[fn_ir.entry].instrs.is_empty(),
            "dead FieldSet base alias assignment should be removed"
        );
        assert!(
            matches!(fn_ir.values[load].kind, ValueKind::Const(Lit::Null)),
            "dead FieldSet base alias load should be neutralized"
        );
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_splits_and_rematerializes_field_set_phi_base_with_impure_update() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let impure = fn_ir.add_value(
            ValueKind::Call {
                callee: "impure_helper".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let left_record = record_xy(&mut fn_ir, x1, y1);
        let right_record = record_xy(&mut fn_ir, x2, y2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        let updated = fn_ir.add_value(
            ValueKind::FieldSet {
                base: record_phi,
                field: "x".to_string(),
                value: impure,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(updated));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(
            optimize(&mut fn_ir),
            "expected FieldSet phi base rematerialization"
        );
        let ValueKind::FieldSet { base, value, .. } = &fn_ir.values[updated].kind else {
            panic!("updated value should remain a functional FieldSet");
        };
        assert_ne!(*base, record_phi);
        assert_eq!(*value, impure);
        let ValueKind::RecordLit { fields } = &fn_ir.values[*base].kind else {
            panic!("FieldSet base should rematerialize as a record literal");
        };
        assert_eq!(fields.len(), 2);
        assert!(matches!(
            &fn_ir.values[fields[0].1].kind,
            ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
        ));
        assert_eq!(fn_ir.values[fields[0].1].phi_block, Some(merge_bb));
        assert!(matches!(
            &fn_ir.values[fields[1].1].kind,
            ValueKind::Phi { args } if *args == vec![(y1, left_bb), (y2, right_bb)]
        ));
        assert_eq!(fn_ir.values[fields[1].1].phi_block, Some(merge_bb));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_splits_branch_record_phi_for_projected_field() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let left_record = record_xy(&mut fn_ir, x1, y1);
        let right_record = record_xy(&mut fn_ir, x2, y2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: record_phi,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(get_x));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(optimize(&mut fn_ir), "expected branch phi SROA rewrite");
        let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
            panic!("merge block should still return a value");
        };
        assert_ne!(ret, get_x);
        assert!(matches!(
            &fn_ir.values[ret].kind,
            ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
        ));
        assert_eq!(fn_ir.values[ret].phi_block, Some(merge_bb));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_splits_aliased_branch_record_phi_for_projected_field() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let left_record = record_xy(&mut fn_ir, x1, y1);
        let right_record = record_xy(&mut fn_ir, x2, y2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record_phi,
            span: Span::default(),
        });
        let load = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let get_y = fn_ir.add_value(
            ValueKind::FieldGet {
                base: load,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(get_y));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(
            optimize(&mut fn_ir),
            "expected aliased branch phi SROA rewrite"
        );
        let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
            panic!("merge block should still return a value");
        };
        assert!(matches!(
            &fn_ir.values[ret].kind,
            ValueKind::Phi { args } if *args == vec![(y1, left_bb), (y2, right_bb)]
        ));
        assert_eq!(fn_ir.values[ret].phi_block, Some(merge_bb));
        assert!(
            fn_ir.blocks[merge_bb].instrs.is_empty(),
            "dead aliased aggregate phi assignment should be removed"
        );
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_splits_transitive_aliased_branch_record_phi_for_projected_field() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let left_record = record_xy(&mut fn_ir, x1, y1);
        let right_record = record_xy(&mut fn_ir, x2, y2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record_phi,
            span: Span::default(),
        });
        let load_point = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        fn_ir.blocks[merge_bb].instrs.push(Instr::Assign {
            dst: "alias".to_string(),
            src: load_point,
            span: Span::default(),
        });
        let load_alias = fn_ir.add_value(
            ValueKind::Load {
                var: "alias".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("alias".to_string()),
        );
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: load_alias,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(get_x));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(
            optimize(&mut fn_ir),
            "expected transitive aliased branch phi SROA rewrite"
        );
        let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
            panic!("merge block should still return a value");
        };
        assert!(matches!(
            &fn_ir.values[ret].kind,
            ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
        ));
        assert_eq!(fn_ir.values[ret].phi_block, Some(merge_bb));
        assert!(
            fn_ir.blocks[merge_bb].instrs.is_empty(),
            "dead transitive aggregate aliases should be removed"
        );
        assert!(matches!(
            fn_ir.values[load_point].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(matches!(
            fn_ir.values[load_alias].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_splits_nested_branch_record_phi_for_projected_field_in_one_pass() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let mass1 = int_value(&mut fn_ir, 10);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let mass2 = int_value(&mut fn_ir, 20);
        let left_pos = record_xy(&mut fn_ir, x1, y1);
        let right_pos = record_xy(&mut fn_ir, x2, y2);
        let left_record = record_pos_mass(&mut fn_ir, left_pos, mass1);
        let right_record = record_pos_mass(&mut fn_ir, right_pos, mass2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        let get_pos = fn_ir.add_value(
            ValueKind::FieldGet {
                base: record_phi,
                field: "pos".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: get_pos,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(get_x));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(
            optimize(&mut fn_ir),
            "expected nested branch phi SROA rewrite"
        );
        let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
            panic!("merge block should still return a value");
        };
        assert_ne!(ret, get_x);
        assert!(matches!(
            &fn_ir.values[ret].kind,
            ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
        ));
        assert_eq!(fn_ir.values[ret].phi_block, Some(merge_bb));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());

        let value_count = fn_ir.values.len();
        assert!(
            !optimize(&mut fn_ir),
            "nested phi scalarization should be idempotent"
        );
        assert_eq!(fn_ir.values.len(), value_count);
    }

    #[test]
    fn sroa_splits_and_rematerializes_branch_record_phi_return() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let left_record = record_xy(&mut fn_ir, x1, y1);
        let right_record = record_xy(&mut fn_ir, x2, y2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(record_phi));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(
            optimize(&mut fn_ir),
            "expected aggregate phi return rematerialization"
        );
        let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
            panic!("merge block should return a rematerialized record");
        };
        assert_ne!(ret, record_phi);
        let ValueKind::RecordLit { fields } = &fn_ir.values[ret].kind else {
            panic!("returned value should rematerialize as a record literal");
        };
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "x");
        assert_eq!(fields[1].0, "y");
        assert!(matches!(
            &fn_ir.values[fields[0].1].kind,
            ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
        ));
        assert_eq!(fn_ir.values[fields[0].1].phi_block, Some(merge_bb));
        assert!(matches!(
            &fn_ir.values[fields[1].1].kind,
            ValueKind::Phi { args } if *args == vec![(y1, left_bb), (y2, right_bb)]
        ));
        assert_eq!(fn_ir.values[fields[1].1].phi_block, Some(merge_bb));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_splits_and_rematerializes_transitive_alias_branch_record_phi_return() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let left_record = record_xy(&mut fn_ir, x1, y1);
        let right_record = record_xy(&mut fn_ir, x2, y2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record_phi,
            span: Span::default(),
        });
        let load_point = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        fn_ir.blocks[merge_bb].instrs.push(Instr::Assign {
            dst: "alias".to_string(),
            src: load_point,
            span: Span::default(),
        });
        let load_alias = fn_ir.add_value(
            ValueKind::Load {
                var: "alias".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("alias".to_string()),
        );
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(load_alias));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(
            optimize(&mut fn_ir),
            "expected transitive alias aggregate phi return rematerialization"
        );
        let Terminator::Return(Some(ret)) = fn_ir.blocks[merge_bb].term else {
            panic!("merge block should return a rematerialized record");
        };
        assert_ne!(ret, load_alias);
        let ValueKind::RecordLit { fields } = &fn_ir.values[ret].kind else {
            panic!("returned value should rematerialize as a record literal");
        };
        assert_eq!(fields.len(), 2);
        assert!(matches!(
            &fn_ir.values[fields[0].1].kind,
            ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
        ));
        assert_eq!(fn_ir.values[fields[0].1].phi_block, Some(merge_bb));
        assert!(matches!(
            &fn_ir.values[fields[1].1].kind,
            ValueKind::Phi { args } if *args == vec![(y1, left_bb), (y2, right_bb)]
        ));
        assert_eq!(fn_ir.values[fields[1].1].phi_block, Some(merge_bb));
        assert!(
            fn_ir.blocks[merge_bb].instrs.is_empty(),
            "dead transitive aggregate aliases should be removed"
        );
        assert!(matches!(
            fn_ir.values[load_point].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(matches!(
            fn_ir.values[load_alias].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_splits_and_rematerializes_branch_record_phi_nested_record_field() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let mass = int_value(&mut fn_ir, 5);
        let left_record = record_xy(&mut fn_ir, x1, y1);
        let right_record = record_xy(&mut fn_ir, x2, y2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        let body = record_pos_mass(&mut fn_ir, record_phi, mass);
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(body));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(
            optimize(&mut fn_ir),
            "expected nested record field aggregate phi rematerialization"
        );
        let ValueKind::RecordLit { fields } = &fn_ir.values[body].kind else {
            panic!("outer record should remain a record literal");
        };
        assert_eq!(fields.len(), 2);
        let ValueKind::RecordLit { fields: pos_fields } = &fn_ir.values[fields[0].1].kind else {
            panic!("nested phi field should rematerialize as a record literal");
        };
        assert_eq!(pos_fields.len(), 2);
        assert!(matches!(
            &fn_ir.values[pos_fields[0].1].kind,
            ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
        ));
        assert_eq!(fn_ir.values[pos_fields[0].1].phi_block, Some(merge_bb));
        assert!(matches!(
            &fn_ir.values[pos_fields[1].1].kind,
            ValueKind::Phi { args } if *args == vec![(y1, left_bb), (y2, right_bb)]
        ));
        assert_eq!(fn_ir.values[pos_fields[1].1].phi_block, Some(merge_bb));
        assert_eq!(fields[1], ("mass".to_string(), mass));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_ignores_dead_nested_record_phi_materialization_demand() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let done = int_value(&mut fn_ir, 0);
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let mass = int_value(&mut fn_ir, 5);
        let left_record = record_xy(&mut fn_ir, x1, y1);
        let right_record = record_xy(&mut fn_ir, x2, y2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        let _dead_body = record_pos_mass(&mut fn_ir, record_phi, mass);
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(done));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        let value_count = fn_ir.values.len();
        assert!(
            !optimize(&mut fn_ir),
            "dead nested aggregate values must not create SROA materialization demand"
        );
        assert_eq!(fn_ir.values.len(), value_count);
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_splits_and_rematerializes_branch_record_phi_index_base() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let idx = int_value(&mut fn_ir, 1);
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let left_record = record_xy(&mut fn_ir, x1, y1);
        let right_record = record_xy(&mut fn_ir, x2, y2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        let indexed = fn_ir.add_value(
            ValueKind::Index1D {
                base: record_phi,
                idx,
                is_safe: false,
                is_na_safe: false,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(indexed));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(
            optimize(&mut fn_ir),
            "expected index base aggregate phi rematerialization"
        );
        let ValueKind::Index1D {
            base, idx: got_idx, ..
        } = &fn_ir.values[indexed].kind
        else {
            panic!("indexed value should remain an Index1D");
        };
        assert_ne!(*base, record_phi);
        assert_eq!(*got_idx, idx);
        let ValueKind::RecordLit { fields } = &fn_ir.values[*base].kind else {
            panic!("index base should rematerialize as a record literal");
        };
        assert_eq!(fields.len(), 2);
        assert!(matches!(
            &fn_ir.values[fields[0].1].kind,
            ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
        ));
        assert_eq!(fn_ir.values[fields[0].1].phi_block, Some(merge_bb));
        assert!(matches!(
            &fn_ir.values[fields[1].1].kind,
            ValueKind::Phi { args } if *args == vec![(y1, left_bb), (y2, right_bb)]
        ));
        assert_eq!(fn_ir.values[fields[1].1].phi_block, Some(merge_bb));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_splits_and_rematerializes_branch_record_phi_eval() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let left_bb = fn_ir.add_block();
        let right_bb = fn_ir.add_block();
        let merge_bb = fn_ir.add_block();
        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let done = int_value(&mut fn_ir, 0);
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let left_record = record_xy(&mut fn_ir, x1, y1);
        let right_record = record_xy(&mut fn_ir, x2, y2);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left_record, left_bb), (right_record, right_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(merge_bb);
        fn_ir.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left_bb,
            else_bb: right_bb,
        };
        fn_ir.blocks[left_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[right_bb].term = Terminator::Goto(merge_bb);
        fn_ir.blocks[merge_bb].instrs.push(Instr::Eval {
            val: record_phi,
            span: Span::default(),
        });
        fn_ir.blocks[merge_bb].term = Terminator::Return(Some(done));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(
            optimize(&mut fn_ir),
            "expected aggregate phi eval rematerialization"
        );
        let [Instr::Eval { val, .. }] = fn_ir.blocks[merge_bb].instrs.as_slice() else {
            panic!("merge block should keep one rematerialized eval");
        };
        assert_ne!(*val, record_phi);
        let ValueKind::RecordLit { fields } = &fn_ir.values[*val].kind else {
            panic!("eval value should rematerialize as a record literal");
        };
        assert_eq!(fields.len(), 2);
        assert!(matches!(
            &fn_ir.values[fields[0].1].kind,
            ValueKind::Phi { args } if *args == vec![(x1, left_bb), (x2, right_bb)]
        ));
        assert_eq!(fn_ir.values[fields[0].1].phi_block, Some(merge_bb));
        assert!(matches!(
            &fn_ir.values[fields[1].1].kind,
            ValueKind::Phi { args } if *args == vec![(y1, left_bb), (y2, right_bb)]
        ));
        assert_eq!(fn_ir.values[fields[1].1].phi_block, Some(merge_bb));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
    }

    #[test]
    fn sroa_splits_loop_carried_record_phi_for_projected_field() {
        let mut fn_ir = test_fn();
        let entry = fn_ir.entry;
        let header_bb = fn_ir.add_block();
        let body_bb = fn_ir.add_block();
        let exit_bb = fn_ir.add_block();
        fn_ir.body_head = header_bb;

        let cond = fn_ir.add_value(
            ValueKind::Const(Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let x0 = int_value(&mut fn_ir, 0);
        let y0 = int_value(&mut fn_ir, 10);
        let one = int_value(&mut fn_ir, 1);
        let seed = record_xy(&mut fn_ir, x0, y0);
        let record_phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(seed, entry), (seed, body_bb)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[record_phi].phi_block = Some(header_bb);
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: record_phi,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let next_x = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: get_x,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_y = fn_ir.add_value(
            ValueKind::FieldGet {
                base: record_phi,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let updated = record_xy(&mut fn_ir, next_x, get_y);
        if let ValueKind::Phi { args } = &mut fn_ir.values[record_phi].kind {
            args[1] = (updated, body_bb);
        }

        fn_ir.blocks[entry].term = Terminator::Goto(header_bb);
        fn_ir.blocks[header_bb].term = Terminator::If {
            cond,
            then_bb: body_bb,
            else_bb: exit_bb,
        };
        fn_ir.blocks[body_bb].term = Terminator::Goto(header_bb);
        fn_ir.blocks[exit_bb].term = Terminator::Return(Some(get_x));

        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());
        assert!(
            optimize(&mut fn_ir),
            "expected loop-carried record phi SROA rewrite"
        );
        let Terminator::Return(Some(ret)) = fn_ir.blocks[exit_bb].term else {
            panic!("exit block should still return a value");
        };
        assert_ne!(ret, get_x);
        assert!(matches!(
            &fn_ir.values[ret].kind,
            ValueKind::Phi { args } if *args == vec![(x0, entry), (next_x, body_bb)]
        ));
        assert_eq!(fn_ir.values[ret].phi_block, Some(header_bb));
        assert!(matches!(
            &fn_ir.values[next_x].kind,
            ValueKind::Binary { lhs, rhs, .. } if *lhs == ret && *rhs == one
        ));
        assert!(crate::mir::verify::verify_ir(&fn_ir).is_ok());

        let value_count = fn_ir.values.len();
        assert!(
            !optimize(&mut fn_ir),
            "dead projections must not keep growing scalar phi values"
        );
        assert_eq!(fn_ir.values.len(), value_count);
    }

    #[test]
    fn sroa_analysis_accepts_straight_line_record_projection() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: record,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

        let analysis = analyze_function(&fn_ir);
        let candidate = analysis.candidate(record).expect("record candidate");

        assert_eq!(
            candidate.shape.as_deref(),
            Some(&["x".to_string(), "y".to_string()][..])
        );
        assert_eq!(candidate.status, SroaCandidateStatus::ScalarOnly);
        assert!(
            candidate
                .uses
                .iter()
                .any(|value_use| value_use.kind == SroaUseKind::Projection)
        );
    }

    #[test]
    fn sroa_analysis_marks_returned_record_for_rematerialization() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(record));

        let analysis = analyze_function(&fn_ir);
        let candidate = analysis.candidate(record).expect("record candidate");

        assert_eq!(
            candidate.status,
            SroaCandidateStatus::NeedsRematerialization
        );
        assert!(
            candidate
                .uses
                .iter()
                .any(|value_use| value_use.kind == SroaUseKind::Materialize)
        );
    }

    #[test]
    fn sroa_analysis_marks_eval_record_for_rematerialization() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let done = int_value(&mut fn_ir, 0);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
            val: record,
            span: Span::default(),
        });
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(done));

        let analysis = analyze_function(&fn_ir);
        let candidate = analysis.candidate(record).expect("record candidate");

        assert_eq!(
            candidate.status,
            SroaCandidateStatus::NeedsRematerialization
        );
        assert!(
            candidate
                .uses
                .iter()
                .any(|value_use| value_use.kind == SroaUseKind::Materialize)
        );
    }

    #[test]
    fn sroa_escape_analysis_classifies_materialization_boundaries() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let record = record_xy(&mut fn_ir, x, y);
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        let load_for_eval = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        fn_ir.blocks[fn_ir.entry].instrs.push(Instr::Eval {
            val: load_for_eval,
            span: Span::default(),
        });
        let load_for_call = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let call = fn_ir.add_value(
            ValueKind::Call {
                callee: "opaque_helper".to_string(),
                args: vec![load_for_call],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_for_intrinsic = fn_ir.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let intrinsic = fn_ir.add_value(
            ValueKind::Intrinsic {
                op: IntrinsicOp::VecMeanF64,
                args: vec![load_for_intrinsic],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let sum = binary_value(&mut fn_ir, BinOp::Add, call, intrinsic);
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(sum));

        let boundaries = collect_materialization_boundaries(&fn_ir);

        assert!(boundaries.contains(&SroaMaterializationBoundary {
            value: load_for_eval,
            kind: SroaMaterializationBoundaryKind::Eval,
        }));
        assert!(boundaries.contains(&SroaMaterializationBoundary {
            value: load_for_call,
            kind: SroaMaterializationBoundaryKind::CallArg,
        }));
        assert!(boundaries.contains(&SroaMaterializationBoundary {
            value: load_for_intrinsic,
            kind: SroaMaterializationBoundaryKind::IntrinsicArg,
        }));
        assert!(boundaries.contains(&SroaMaterializationBoundary {
            value: sum,
            kind: SroaMaterializationBoundaryKind::Return,
        }));
    }

    #[test]
    fn sroa_analysis_tracks_field_set_shape() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let replacement = int_value(&mut fn_ir, 3);
        let record = record_xy(&mut fn_ir, x, y);
        let updated = fn_ir.add_value(
            ValueKind::FieldSet {
                base: record,
                field: "x".to_string(),
                value: replacement,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_x = fn_ir.add_value(
            ValueKind::FieldGet {
                base: updated,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(get_x));

        let analysis = analyze_function(&fn_ir);
        let candidate = analysis.candidate(updated).expect("fieldset candidate");

        assert_eq!(candidate.source, SroaCandidateSource::FieldSet);
        assert_eq!(
            candidate.shape.as_deref(),
            Some(&["x".to_string(), "y".to_string()][..])
        );
        assert_eq!(candidate.status, SroaCandidateStatus::ScalarOnly);
    }

    #[test]
    fn sroa_analysis_tracks_same_shape_phi() {
        let mut fn_ir = test_fn();
        let x1 = int_value(&mut fn_ir, 1);
        let y1 = int_value(&mut fn_ir, 2);
        let x2 = int_value(&mut fn_ir, 3);
        let y2 = int_value(&mut fn_ir, 4);
        let left = record_xy(&mut fn_ir, x1, y1);
        let right = record_xy(&mut fn_ir, x2, y2);
        let phi = fn_ir.add_value(
            ValueKind::Phi {
                args: vec![(left, fn_ir.entry), (right, fn_ir.entry)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[phi].phi_block = Some(fn_ir.entry);
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(phi));

        let analysis = analyze_function(&fn_ir);
        let candidate = analysis.candidate(phi).expect("phi candidate");

        assert_eq!(candidate.source, SroaCandidateSource::Phi);
        assert_eq!(
            candidate.shape.as_deref(),
            Some(&["x".to_string(), "y".to_string()][..])
        );
        assert_eq!(
            candidate.status,
            SroaCandidateStatus::NeedsRematerialization
        );
    }

    #[test]
    fn sroa_analysis_rejects_unsupported_index_use() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let y = int_value(&mut fn_ir, 2);
        let idx = int_value(&mut fn_ir, 0);
        let record = record_xy(&mut fn_ir, x, y);
        let indexed = fn_ir.add_value(
            ValueKind::Index1D {
                base: record,
                idx,
                is_safe: false,
                is_na_safe: false,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(indexed));

        let analysis = analyze_function(&fn_ir);
        let candidate = analysis.candidate(record).expect("record candidate");

        assert_eq!(
            candidate.status,
            SroaCandidateStatus::NeedsRematerialization
        );
        assert!(
            candidate
                .uses
                .iter()
                .any(|value_use| value_use.kind == SroaUseKind::Materialize)
        );
    }

    #[test]
    fn sroa_analysis_rejects_duplicate_fields() {
        let mut fn_ir = test_fn();
        let x = int_value(&mut fn_ir, 1);
        let duplicate = fn_ir.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), x), ("x".to_string(), x)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[fn_ir.entry].term = Terminator::Return(Some(duplicate));

        let analysis = analyze_function(&fn_ir);
        let candidate = analysis.candidate(duplicate).expect("record candidate");

        assert_eq!(candidate.status, SroaCandidateStatus::Rejected);
        assert!(candidate.reject_reasons.iter().any(
            |reason| matches!(reason, SroaRejectReason::DuplicateField(field) if field == "x")
        ));
    }

    #[test]
    fn sroa_specializes_known_record_field_call_argument() {
        let mut caller = FnIR::new("caller".to_string(), vec![]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let x = int_value(&mut caller, 1);
        let y = int_value(&mut caller, 2);
        let record = record_xy(&mut caller, x, y);
        caller.blocks[entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: record,
            span: Span::default(),
        });
        let load = caller.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let call = caller.add_value(
            ValueKind::Call {
                callee: "sum_xy".to_string(),
                args: vec![load],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(call));

        let mut all_fns = FxHashMap::default();
        all_fns.insert("caller".to_string(), caller);
        all_fns.insert("sum_xy".to_string(), sum_xy_fn());

        assert!(specialize_record_field_calls(&mut all_fns));
        let caller = all_fns.get("caller").expect("caller");
        let ValueKind::Call {
            callee,
            args,
            names,
        } = &caller.values[call].kind
        else {
            panic!("call should remain a direct call");
        };
        assert_ne!(callee, "sum_xy");
        assert_eq!(args, &vec![x, y]);
        assert_eq!(names, &vec![None, None]);
        assert!(
            caller.blocks[entry].instrs.is_empty(),
            "record alias should become dead once call args are scalarized"
        );

        let specialized = all_fns.get(callee).expect("specialized callee");
        assert_eq!(specialized.params.len(), 2);
        assert!(
            specialized
                .params
                .iter()
                .all(|param| param.contains("__rr_sroa_"))
        );
        assert!(
            specialized
                .values
                .iter()
                .any(|value| matches!(value.kind, ValueKind::Param { index: 0 }))
        );
        assert!(
            specialized
                .values
                .iter()
                .any(|value| matches!(value.kind, ValueKind::Param { index: 1 }))
        );
        assert!(crate::mir::verify::verify_ir(caller).is_ok());
        assert!(crate::mir::verify::verify_ir(specialized).is_ok());
    }

    #[test]
    fn sroa_does_not_specialize_record_call_when_param_escapes() {
        let mut callee = FnIR::new("escape_record".to_string(), vec!["p".to_string()]);
        let entry = callee.add_block();
        callee.entry = entry;
        callee.body_head = entry;
        let p = callee.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some("p".to_string()),
        );
        callee.blocks[entry].term = Terminator::Return(Some(p));

        let mut caller = FnIR::new("caller".to_string(), vec![]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let x = int_value(&mut caller, 1);
        let y = int_value(&mut caller, 2);
        let record = record_xy(&mut caller, x, y);
        let call = caller.add_value(
            ValueKind::Call {
                callee: "escape_record".to_string(),
                args: vec![record],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(call));

        let mut all_fns = FxHashMap::default();
        all_fns.insert("caller".to_string(), caller);
        all_fns.insert("escape_record".to_string(), callee);

        assert!(!specialize_record_field_calls(&mut all_fns));
        let caller = all_fns.get("caller").expect("caller");
        assert!(matches!(
            &caller.values[call].kind,
            ValueKind::Call { callee, args, .. } if callee == "escape_record" && args == &vec![record]
        ));
        assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_")));
    }

    #[test]
    fn sroa_specializes_direct_record_return_field_call() {
        let mut caller = FnIR::new("caller".to_string(), vec![]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let call = caller.add_value(
            ValueKind::Call {
                callee: "make_xy".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_x = caller.add_value(
            ValueKind::FieldGet {
                base: call,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(get_x));

        let mut all_fns = FxHashMap::default();
        all_fns.insert("caller".to_string(), caller);
        all_fns.insert("make_xy".to_string(), make_xy_fn());

        assert!(specialize_record_return_field_calls(&mut all_fns));
        let caller = all_fns.get("caller").expect("caller");
        assert!(matches!(
            caller.values[get_x].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(matches!(
            caller.blocks[entry].term,
            Terminator::Return(Some(ret)) if matches!(caller.values[ret].kind, ValueKind::Const(Lit::Int(1)))
        ));
        assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_ret_")));
        assert!(crate::mir::verify::verify_ir(caller).is_ok());
    }

    #[test]
    fn sroa_keeps_scalar_return_helper_for_direct_branch_record_return() {
        let mut caller = FnIR::new("caller".to_string(), vec![]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let call = caller.add_value(
            ValueKind::Call {
                callee: "branch_make_xy".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_x = caller.add_value(
            ValueKind::FieldGet {
                base: call,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(get_x));

        let mut all_fns = FxHashMap::default();
        all_fns.insert("caller".to_string(), caller);
        all_fns.insert("branch_make_xy".to_string(), branch_make_xy_fn());

        assert!(specialize_record_return_field_calls(&mut all_fns));
        let caller = all_fns.get("caller").expect("caller");
        let ValueKind::Call { callee, args, .. } = &caller.values[get_x].kind else {
            panic!("branching record-return projection should use the scalar helper fallback");
        };
        assert!(callee.contains("__rr_sroa_ret_x"));
        assert!(args.is_empty());
        let specialized = all_fns.get(callee).expect("specialized return callee");
        assert!(specialized.blocks.iter().any(|block| matches!(
            block.term,
            Terminator::Return(Some(ret)) if matches!(specialized.values[ret].kind, ValueKind::Const(Lit::Int(1)))
        )));
        assert!(specialized.blocks.iter().any(|block| matches!(
            block.term,
            Terminator::Return(Some(ret)) if matches!(specialized.values[ret].kind, ValueKind::Const(Lit::Int(3)))
        )));
        assert!(crate::mir::verify::verify_ir(caller).is_ok());
        assert!(crate::mir::verify::verify_ir(specialized).is_ok());
    }

    #[test]
    fn sroa_inlines_aliased_record_return_field_call_and_removes_alias() {
        let mut caller = FnIR::new("caller".to_string(), vec![]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let call = caller.add_value(
            ValueKind::Call {
                callee: "make_xy".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: call,
            span: Span::default(),
        });
        let load = caller.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let get_y = caller.add_value(
            ValueKind::FieldGet {
                base: load,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(get_y));

        let mut all_fns = FxHashMap::default();
        all_fns.insert("caller".to_string(), caller);
        all_fns.insert("make_xy".to_string(), make_xy_fn());

        assert!(specialize_record_return_field_calls(&mut all_fns));
        let caller = all_fns.get("caller").expect("caller");
        let ValueKind::Load { var: temp_var } = &caller.values[get_y].kind else {
            panic!("aliased field projection should become a scalar temp load");
        };
        assert!(temp_var.contains("__rr_sroa_ret_y"));
        assert!(
            matches!(
                &caller.blocks[entry].instrs[..],
                [Instr::Assign { dst, src, .. }]
                    if dst == temp_var
                        && matches!(caller.values[*src].kind, ValueKind::Const(Lit::Int(2)))
            ),
            "pure record-return alias assignment should be replaced by one inlined scalar temp"
        );
        assert!(matches!(
            caller.values[load].kind,
            ValueKind::Const(Lit::Null)
        ));
        let Instr::Assign { src, .. } = &caller.blocks[entry].instrs[0] else {
            panic!("expected scalar temp assignment");
        };
        assert!(matches!(
            caller.values[*src].kind,
            ValueKind::Const(Lit::Int(2))
        ));
        assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_ret_")));
        assert!(crate::mir::verify::verify_ir(caller).is_ok());
    }

    #[test]
    fn sroa_shares_aliased_record_return_inline_temp_for_repeated_projection() {
        let mut caller = FnIR::new("caller".to_string(), vec![]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let call = caller.add_value(
            ValueKind::Call {
                callee: "make_xy".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: call,
            span: Span::default(),
        });
        let load_a = caller.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let get_x_a = caller.add_value(
            ValueKind::FieldGet {
                base: load_a,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_b = caller.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let get_x_b = caller.add_value(
            ValueKind::FieldGet {
                base: load_b,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let sum = caller.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: get_x_a,
                rhs: get_x_b,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(sum));

        let mut all_fns = FxHashMap::default();
        all_fns.insert("caller".to_string(), caller);
        all_fns.insert("make_xy".to_string(), make_xy_fn());

        assert!(specialize_record_return_field_calls(&mut all_fns));
        let caller = all_fns.get("caller").expect("caller");
        let ValueKind::Load { var: first_temp } = &caller.values[get_x_a].kind else {
            panic!("first repeated projection should load the shared scalar temp");
        };
        let ValueKind::Load { var: second_temp } = &caller.values[get_x_b].kind else {
            panic!("second repeated projection should load the shared scalar temp");
        };
        assert_eq!(first_temp, second_temp);
        assert!(first_temp.contains("__rr_sroa_ret_x"));
        let ret_x_calls: Vec<_> = caller
            .values
            .iter()
            .filter_map(|value| match &value.kind {
                ValueKind::Call { callee, .. } if callee.contains("__rr_sroa_ret_x") => {
                    Some(value.id)
                }
                _ => None,
            })
            .collect();
        assert_eq!(
            ret_x_calls.len(),
            0,
            "inlineable repeated field projection should not need scalar-return calls"
        );
        assert_eq!(
            caller.blocks[entry].instrs.len(),
            1,
            "record alias assignment should be replaced by one scalar temp assignment"
        );
        assert!(matches!(
            &caller.blocks[entry].instrs[0],
            Instr::Assign { dst, src, .. }
                if dst == first_temp && matches!(caller.values[*src].kind, ValueKind::Const(Lit::Int(1)))
        ));
        assert!(matches!(
            caller.values[load_a].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(matches!(
            caller.values[load_b].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(crate::mir::verify::verify_ir(caller).is_ok());
    }

    #[test]
    fn sroa_inlines_aliased_record_return_different_fields_without_scalar_calls() {
        let mut caller = FnIR::new("caller".to_string(), vec![]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let call = caller.add_value(
            ValueKind::Call {
                callee: "make_xy".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].instrs.push(Instr::Assign {
            dst: "point".to_string(),
            src: call,
            span: Span::default(),
        });
        let load_x = caller.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let get_x = caller.add_value(
            ValueKind::FieldGet {
                base: load_x,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_y = caller.add_value(
            ValueKind::Load {
                var: "point".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("point".to_string()),
        );
        let get_y = caller.add_value(
            ValueKind::FieldGet {
                base: load_y,
                field: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let sum = caller.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: get_x,
                rhs: get_y,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(sum));

        let mut all_fns = FxHashMap::default();
        all_fns.insert("caller".to_string(), caller);
        all_fns.insert("make_xy".to_string(), make_xy_fn());

        assert!(specialize_record_return_field_calls(&mut all_fns));
        let caller = all_fns.get("caller").expect("caller");
        let ValueKind::Load { var: x_temp } = &caller.values[get_x].kind else {
            panic!("x projection should load an inlined scalar temp");
        };
        let ValueKind::Load { var: y_temp } = &caller.values[get_y].kind else {
            panic!("y projection should load an inlined scalar temp");
        };
        assert_ne!(x_temp, y_temp);
        assert!(x_temp.contains("__rr_sroa_ret_x"));
        assert!(y_temp.contains("__rr_sroa_ret_y"));
        assert_eq!(
            caller.blocks[entry].instrs.len(),
            2,
            "record alias assignment should be replaced by fieldwise scalar temps"
        );
        assert!(matches!(
            &caller.blocks[entry].instrs[0],
            Instr::Assign { dst, src, .. }
                if dst == x_temp && matches!(caller.values[*src].kind, ValueKind::Const(Lit::Int(1)))
        ));
        assert!(matches!(
            &caller.blocks[entry].instrs[1],
            Instr::Assign { dst, src, .. }
                if dst == y_temp && matches!(caller.values[*src].kind, ValueKind::Const(Lit::Int(2)))
        ));
        assert!(matches!(
            caller.values[load_x].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(matches!(
            caller.values[load_y].kind,
            ValueKind::Const(Lit::Null)
        ));
        assert!(!caller.values.iter().any(|value| {
            matches!(
                &value.kind,
                ValueKind::Call { callee, .. } if callee.contains("__rr_sroa_ret_")
            )
        }));
        assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_ret_")));
        assert!(crate::mir::verify::verify_ir(caller).is_ok());
    }

    #[test]
    fn sroa_does_not_specialize_impure_record_return_call() {
        let mut caller = FnIR::new("caller".to_string(), vec![]);
        let entry = caller.add_block();
        caller.entry = entry;
        caller.body_head = entry;
        let call = caller.add_value(
            ValueKind::Call {
                callee: "impure_make_xy".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let get_x = caller.add_value(
            ValueKind::FieldGet {
                base: call,
                field: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        caller.blocks[entry].term = Terminator::Return(Some(get_x));

        let mut all_fns = FxHashMap::default();
        all_fns.insert("caller".to_string(), caller);
        all_fns.insert("impure_make_xy".to_string(), impure_make_xy_fn());

        assert!(!specialize_record_return_field_calls(&mut all_fns));
        let caller = all_fns.get("caller").expect("caller");
        assert!(matches!(
            &caller.values[get_x].kind,
            ValueKind::FieldGet { base, field } if *base == call && field == "x"
        ));
        assert!(!all_fns.keys().any(|name| name.contains("__rr_sroa_ret_")));
    }
}
