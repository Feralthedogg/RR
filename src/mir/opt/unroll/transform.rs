use super::analysis::{UnrollCandidate, UnrollMode};
use super::*;

pub(crate) fn apply(fn_ir: &mut FnIR, candidate: &UnrollCandidate) -> bool {
    match candidate.mode {
        UnrollMode::Full => apply_full_unroll(fn_ir, candidate),
        UnrollMode::Partial { factor } => apply_partial_unroll(fn_ir, candidate.body, factor),
    }
}

fn apply_full_unroll(fn_ir: &mut FnIR, candidate: &UnrollCandidate) -> bool {
    let template = fn_ir.blocks[candidate.body].instrs.clone();
    if template.is_empty() {
        return false;
    }

    let mut unrolled_blocks = Vec::with_capacity(candidate.trip_count);
    for _ in 0..candidate.trip_count {
        let block = fn_ir.add_block();
        fn_ir.blocks[block].instrs = clone_template_instrs(fn_ir, &template);
        unrolled_blocks.push(block);
    }
    for pair in unrolled_blocks.windows(2) {
        fn_ir.blocks[pair[0]].term = Terminator::Goto(pair[1]);
    }
    let first = unrolled_blocks[0];
    let last = *unrolled_blocks.last().unwrap_or(&first);
    fn_ir.blocks[last].term = Terminator::Goto(candidate.exit);

    for pred in &candidate.outside_preds {
        retarget_successor(&mut fn_ir.blocks[*pred].term, candidate.header, first);
    }
    fn_ir.blocks[candidate.header].term = Terminator::Unreachable;
    fn_ir.blocks[candidate.body].term = Terminator::Unreachable;
    true
}

fn apply_partial_unroll(fn_ir: &mut FnIR, body: BlockId, factor: usize) -> bool {
    if factor < 2 {
        return false;
    }
    let template = fn_ir.blocks[body].instrs.clone();
    if template.is_empty() {
        return false;
    }

    let mut expanded = Vec::with_capacity(template.len().saturating_mul(factor));
    for _ in 0..factor {
        expanded.extend(clone_template_instrs(fn_ir, &template));
    }
    fn_ir.blocks[body].instrs = expanded;
    true
}

fn clone_template_instrs(fn_ir: &mut FnIR, template: &[Instr]) -> Vec<Instr> {
    let mut remap = FxHashMap::default();
    template
        .iter()
        .map(|instr| clone_instr(fn_ir, instr, &mut remap))
        .collect()
}

fn clone_instr(fn_ir: &mut FnIR, instr: &Instr, remap: &mut FxHashMap<ValueId, ValueId>) -> Instr {
    match instr {
        Instr::Assign { dst, src, span } => Instr::Assign {
            dst: dst.clone(),
            src: clone_value(fn_ir, *src, remap),
            span: *span,
        },
        Instr::Eval { val, span } => Instr::Eval {
            val: clone_value(fn_ir, *val, remap),
            span: *span,
        },
        Instr::StoreIndex1D {
            base,
            idx,
            val,
            is_safe,
            is_na_safe,
            is_vector,
            span,
        } => Instr::StoreIndex1D {
            base: clone_value(fn_ir, *base, remap),
            idx: clone_value(fn_ir, *idx, remap),
            val: clone_value(fn_ir, *val, remap),
            is_safe: *is_safe,
            is_na_safe: *is_na_safe,
            is_vector: *is_vector,
            span: *span,
        },
        Instr::StoreIndex2D {
            base,
            r,
            c,
            val,
            span,
        } => Instr::StoreIndex2D {
            base: clone_value(fn_ir, *base, remap),
            r: clone_value(fn_ir, *r, remap),
            c: clone_value(fn_ir, *c, remap),
            val: clone_value(fn_ir, *val, remap),
            span: *span,
        },
        Instr::StoreIndex3D {
            base,
            i,
            j,
            k,
            val,
            span,
        } => Instr::StoreIndex3D {
            base: clone_value(fn_ir, *base, remap),
            i: clone_value(fn_ir, *i, remap),
            j: clone_value(fn_ir, *j, remap),
            k: clone_value(fn_ir, *k, remap),
            val: clone_value(fn_ir, *val, remap),
            span: *span,
        },
        Instr::UnsafeRBlock {
            code,
            read_only,
            span,
        } => Instr::UnsafeRBlock {
            code: code.clone(),
            read_only: *read_only,
            span: *span,
        },
    }
}

fn clone_value(
    fn_ir: &mut FnIR,
    value: ValueId,
    remap: &mut FxHashMap<ValueId, ValueId>,
) -> ValueId {
    if let Some(cloned) = remap.get(&value) {
        return *cloned;
    }

    let original = fn_ir.values[value].clone();
    let kind = clone_value_kind(fn_ir, original.kind.clone(), remap);
    let cloned = fn_ir.add_value(
        kind,
        original.span,
        original.facts,
        original.origin_var.clone(),
    );
    fn_ir.values[cloned].value_ty = original.value_ty;
    fn_ir.values[cloned].value_term = original.value_term;
    fn_ir.values[cloned].phi_block = original.phi_block;
    fn_ir.values[cloned].escape = original.escape;
    if let Some(semantics) = fn_ir.call_semantics.get(&value).copied() {
        fn_ir.call_semantics.insert(cloned, semantics);
    }
    if let Some(layout) = fn_ir.memory_layout_hints.get(&value).copied() {
        fn_ir.memory_layout_hints.insert(cloned, layout);
    }
    remap.insert(value, cloned);
    cloned
}

fn clone_value_kind(
    fn_ir: &mut FnIR,
    kind: ValueKind,
    remap: &mut FxHashMap<ValueId, ValueId>,
) -> ValueKind {
    match kind {
        ValueKind::Const(lit) => ValueKind::Const(lit),
        ValueKind::Param { index } => ValueKind::Param { index },
        ValueKind::Load { var } => ValueKind::Load { var },
        ValueKind::RSymbol { name } => ValueKind::RSymbol { name },
        ValueKind::Phi { args } => ValueKind::Phi {
            args: args
                .into_iter()
                .map(|(arg, block)| (clone_value(fn_ir, arg, remap), block))
                .collect(),
        },
        ValueKind::Len { base } => ValueKind::Len {
            base: clone_value(fn_ir, base, remap),
        },
        ValueKind::Indices { base } => ValueKind::Indices {
            base: clone_value(fn_ir, base, remap),
        },
        ValueKind::Range { start, end } => ValueKind::Range {
            start: clone_value(fn_ir, start, remap),
            end: clone_value(fn_ir, end, remap),
        },
        ValueKind::Binary { op, lhs, rhs } => ValueKind::Binary {
            op,
            lhs: clone_value(fn_ir, lhs, remap),
            rhs: clone_value(fn_ir, rhs, remap),
        },
        ValueKind::Unary { op, rhs } => ValueKind::Unary {
            op,
            rhs: clone_value(fn_ir, rhs, remap),
        },
        ValueKind::Call {
            callee,
            args,
            names,
        } => ValueKind::Call {
            callee,
            args: args
                .into_iter()
                .map(|arg| clone_value(fn_ir, arg, remap))
                .collect(),
            names,
        },
        ValueKind::RecordLit { fields } => ValueKind::RecordLit {
            fields: fields
                .into_iter()
                .map(|(name, value)| (name, clone_value(fn_ir, value, remap)))
                .collect(),
        },
        ValueKind::FieldGet { base, field } => ValueKind::FieldGet {
            base: clone_value(fn_ir, base, remap),
            field,
        },
        ValueKind::FieldSet { base, field, value } => ValueKind::FieldSet {
            base: clone_value(fn_ir, base, remap),
            field,
            value: clone_value(fn_ir, value, remap),
        },
        ValueKind::Intrinsic { op, args } => ValueKind::Intrinsic {
            op,
            args: args
                .into_iter()
                .map(|arg| clone_value(fn_ir, arg, remap))
                .collect(),
        },
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => ValueKind::Index1D {
            base: clone_value(fn_ir, base, remap),
            idx: clone_value(fn_ir, idx, remap),
            is_safe,
            is_na_safe,
        },
        ValueKind::Index2D { base, r, c } => ValueKind::Index2D {
            base: clone_value(fn_ir, base, remap),
            r: clone_value(fn_ir, r, remap),
            c: clone_value(fn_ir, c, remap),
        },
        ValueKind::Index3D { base, i, j, k } => ValueKind::Index3D {
            base: clone_value(fn_ir, base, remap),
            i: clone_value(fn_ir, i, remap),
            j: clone_value(fn_ir, j, remap),
            k: clone_value(fn_ir, k, remap),
        },
    }
}

fn retarget_successor(term: &mut Terminator, old: BlockId, new: BlockId) {
    match term {
        Terminator::Goto(target) if *target == old => *target = new,
        Terminator::If {
            then_bb, else_bb, ..
        } => {
            if *then_bb == old {
                *then_bb = new;
            }
            if *else_bb == old {
                *else_bb = new;
            }
        }
        _ => {}
    }
}
