use super::super::*;
use super::analysis::OutlineCandidate;
use super::rewrite;
use crate::utils::Span;

pub(crate) fn extract_helper(
    parent: &mut FnIR,
    candidate: &OutlineCandidate,
    helper_name: String,
) -> Option<FnIR> {
    let helper = build_helper(parent, candidate, helper_name)?;
    rewrite::replace_region_with_call(parent, candidate, &helper.name);
    Some(helper)
}

fn build_helper(parent: &FnIR, candidate: &OutlineCandidate, helper_name: String) -> Option<FnIR> {
    let source_block = parent.blocks.get(candidate.block)?;
    let mut helper = FnIR::new(helper_name, candidate.live_ins.clone());
    helper.user_name = None;
    helper.span = parent.span;

    let entry = helper.add_block();
    let body = helper.add_block();
    helper.entry = entry;
    helper.body_head = body;
    helper.blocks[entry].term = Terminator::Goto(body);

    let mut value_map = FxHashMap::default();
    for instr in &source_block.instrs[candidate.start..candidate.end] {
        let cloned = clone_instr(parent, &mut helper, instr, &mut value_map)?;
        helper.blocks[body].instrs.push(cloned);
    }

    let ret = helper_return_value(&mut helper, &candidate.live_outs);
    helper.blocks[body].term = Terminator::Return(Some(ret));
    Some(helper)
}

fn helper_return_value(helper: &mut FnIR, live_outs: &[VarId]) -> ValueId {
    match live_outs {
        [] => helper.add_value(
            ValueKind::Const(Lit::Null),
            Span::default(),
            Facts::empty(),
            None,
        ),
        [var] => helper.add_value(
            ValueKind::Load { var: var.clone() },
            Span::default(),
            Facts::empty(),
            Some(var.clone()),
        ),
        vars => {
            let fields = vars
                .iter()
                .map(|var| {
                    let value = helper.add_value(
                        ValueKind::Load { var: var.clone() },
                        Span::default(),
                        Facts::empty(),
                        Some(var.clone()),
                    );
                    (var.clone(), value)
                })
                .collect();
            helper.add_value(
                ValueKind::RecordLit { fields },
                Span::default(),
                Facts::empty(),
                None,
            )
        }
    }
}

fn clone_instr(
    parent: &FnIR,
    helper: &mut FnIR,
    instr: &Instr,
    value_map: &mut FxHashMap<ValueId, ValueId>,
) -> Option<Instr> {
    match instr {
        Instr::Assign { dst, src, span } => {
            let cloned_src = clone_value(parent, helper, *src, value_map)?;
            Some(Instr::Assign {
                dst: dst.clone(),
                src: cloned_src,
                span: *span,
            })
        }
        _ => None,
    }
}

fn clone_value(
    parent: &FnIR,
    helper: &mut FnIR,
    root: ValueId,
    value_map: &mut FxHashMap<ValueId, ValueId>,
) -> Option<ValueId> {
    if let Some(&mapped) = value_map.get(&root) {
        return Some(mapped);
    }

    let mut needed: Vec<_> = worklist::collect_value_dependencies_iterative(parent, root)
        .into_iter()
        .collect();
    needed.sort_unstable();

    for value_id in needed {
        if value_map.contains_key(&value_id) {
            continue;
        }
        let value = parent.values.get(value_id)?;
        let kind = clone_value_kind(&value.kind, value_map)?;
        let cloned = helper.add_value(kind, value.span, value.facts, value.origin_var.clone());
        helper.values[cloned].value_ty = value.value_ty;
        helper.values[cloned].value_term = value.value_term.clone();
        helper.values[cloned].escape = value.escape;
        value_map.insert(value_id, cloned);
    }
    value_map.get(&root).copied()
}

fn clone_value_kind(
    kind: &ValueKind,
    value_map: &FxHashMap<ValueId, ValueId>,
) -> Option<ValueKind> {
    let map = |value| value_map.get(&value).copied();
    Some(match kind {
        ValueKind::Const(lit) => ValueKind::Const(lit.clone()),
        ValueKind::Param { index } => ValueKind::Param { index: *index },
        ValueKind::Load { var } => ValueKind::Load { var: var.clone() },
        ValueKind::Len { base } => ValueKind::Len { base: map(*base)? },
        ValueKind::Indices { base } => ValueKind::Indices { base: map(*base)? },
        ValueKind::Range { start, end } => ValueKind::Range {
            start: map(*start)?,
            end: map(*end)?,
        },
        ValueKind::Binary { op, lhs, rhs } => ValueKind::Binary {
            op: *op,
            lhs: map(*lhs)?,
            rhs: map(*rhs)?,
        },
        ValueKind::Unary { op, rhs } => ValueKind::Unary {
            op: *op,
            rhs: map(*rhs)?,
        },
        ValueKind::RecordLit { fields } => ValueKind::RecordLit {
            fields: fields
                .iter()
                .map(|(field, value)| Some((field.clone(), map(*value)?)))
                .collect::<Option<_>>()?,
        },
        ValueKind::FieldGet { base, field } => ValueKind::FieldGet {
            base: map(*base)?,
            field: field.clone(),
        },
        ValueKind::FieldSet { base, field, value } => ValueKind::FieldSet {
            base: map(*base)?,
            field: field.clone(),
            value: map(*value)?,
        },
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => ValueKind::Index1D {
            base: map(*base)?,
            idx: map(*idx)?,
            is_safe: *is_safe,
            is_na_safe: *is_na_safe,
        },
        ValueKind::Index2D { base, r, c } => ValueKind::Index2D {
            base: map(*base)?,
            r: map(*r)?,
            c: map(*c)?,
        },
        ValueKind::Index3D { base, i, j, k } => ValueKind::Index3D {
            base: map(*base)?,
            i: map(*i)?,
            j: map(*j)?,
            k: map(*k)?,
        },
        ValueKind::Phi { .. }
        | ValueKind::Call { .. }
        | ValueKind::Intrinsic { .. }
        | ValueKind::RSymbol { .. } => return None,
    })
}
