use super::*;
pub(crate) fn build_use_graph(fn_ir: &FnIR) -> FxHashMap<ValueId, Vec<SroaUse>> {
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
                Instr::UnsafeRBlock { .. } => {}
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

pub(crate) fn add_store_uses(
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
            SroaUseKind::Materialize,
        );
    }
}

pub(crate) fn add_use(
    uses: &mut FxHashMap<ValueId, Vec<SroaUse>>,
    value: ValueId,
    user: SroaUser,
    kind: SroaUseKind,
) {
    uses.entry(value).or_default().push(SroaUse { user, kind });
}
