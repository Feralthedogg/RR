use super::*;
impl MirInliner {
    pub(crate) fn remap_value_kind(&self, kind: &mut ValueKind, map: &mut InlineMap) {
        match kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                if let Some(&n) = map.v.get(lhs) {
                    *lhs = n;
                }
                if let Some(&n) = map.v.get(rhs) {
                    *rhs = n;
                }
            }
            ValueKind::Unary { rhs, .. } => {
                if let Some(&n) = map.v.get(rhs) {
                    *rhs = n;
                }
            }
            ValueKind::Call { args, .. } => {
                for a in args {
                    if let Some(&n) = map.v.get(a) {
                        *a = n;
                    }
                }
            }
            ValueKind::Intrinsic { args, .. } => {
                for a in args {
                    if let Some(&n) = map.v.get(a) {
                        *a = n;
                    }
                }
            }
            ValueKind::Phi { args } => {
                for (v, b) in args {
                    if let Some(&n) = map.v.get(v) {
                        *v = n;
                    }
                    if let Some(&n) = map.b.get(b) {
                        *b = n;
                    }
                }
            }
            ValueKind::Index1D { base, idx, .. } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(idx) {
                    *idx = n;
                }
            }
            ValueKind::Index2D { base, r, c } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(r) {
                    *r = n;
                }
                if let Some(&n) = map.v.get(c) {
                    *c = n;
                }
            }
            ValueKind::Index3D { base, i, j, k } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(i) {
                    *i = n;
                }
                if let Some(&n) = map.v.get(j) {
                    *j = n;
                }
                if let Some(&n) = map.v.get(k) {
                    *k = n;
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
            }
            ValueKind::Range { start, end } => {
                if let Some(&n) = map.v.get(start) {
                    *start = n;
                }
                if let Some(&n) = map.v.get(end) {
                    *end = n;
                }
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    if let Some(&n) = map.v.get(value) {
                        *value = n;
                    }
                }
            }
            ValueKind::FieldGet { base, .. } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
            }
            ValueKind::FieldSet { base, value, .. } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(value) {
                    *value = n;
                }
            }
            ValueKind::Load { var } => {
                let mapped = map.map_var(var);
                *var = mapped;
            }
            _ => {}
        }
    }

    pub(crate) fn remap_instr(&self, instr: &mut Instr, map: &mut InlineMap) {
        match instr {
            Instr::Assign { dst, src, .. } => {
                if let Some(&n) = map.v.get(src) {
                    *src = n;
                }
                let mapped = map.map_var(dst);
                *dst = mapped;
            }
            Instr::Eval { val, .. } => {
                if let Some(&n) = map.v.get(val) {
                    *val = n;
                }
            }
            Instr::StoreIndex1D { base, idx, val, .. } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(idx) {
                    *idx = n;
                }
                if let Some(&n) = map.v.get(val) {
                    *val = n;
                }
            }
            Instr::StoreIndex2D {
                base, r, c, val, ..
            } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(r) {
                    *r = n;
                }
                if let Some(&n) = map.v.get(c) {
                    *c = n;
                }
                if let Some(&n) = map.v.get(val) {
                    *val = n;
                }
            }
            Instr::StoreIndex3D {
                base, i, j, k, val, ..
            } => {
                if let Some(&n) = map.v.get(base) {
                    *base = n;
                }
                if let Some(&n) = map.v.get(i) {
                    *i = n;
                }
                if let Some(&n) = map.v.get(j) {
                    *j = n;
                }
                if let Some(&n) = map.v.get(k) {
                    *k = n;
                }
                if let Some(&n) = map.v.get(val) {
                    *val = n;
                }
            }
            Instr::UnsafeRBlock { .. } => {}
        }
    }

    pub(crate) fn remap_term(&self, term: &mut Terminator, map: &InlineMap) {
        match term {
            Terminator::Goto(b) => {
                if let Some(&n) = map.b.get(b) {
                    *b = n;
                }
            }
            Terminator::If {
                cond,
                then_bb,
                else_bb,
            } => {
                if let Some(&n) = map.v.get(cond) {
                    *cond = n;
                }
                if let Some(&n) = map.b.get(then_bb) {
                    *then_bb = n;
                }
                if let Some(&n) = map.b.get(else_bb) {
                    *else_bb = n;
                }
            }
            Terminator::Return(Some(v)) => {
                if let Some(&n) = map.v.get(v) {
                    *v = n;
                }
            }
            _ => {}
        }
    }

    pub(crate) fn replace_value_use(value: &mut ValueId, old: ValueId, new: ValueId) {
        if *value == old {
            *value = new;
        }
    }

    pub(crate) fn replace_value_uses<'a>(
        values: impl IntoIterator<Item = &'a mut ValueId>,
        old: ValueId,
        new: ValueId,
    ) {
        for value in values {
            Self::replace_value_use(value, old, new);
        }
    }

    pub(crate) fn replace_value_kind_uses(kind: &mut ValueKind, old: ValueId, new: ValueId) {
        match kind {
            ValueKind::Binary { lhs, rhs, .. } => Self::replace_value_uses([lhs, rhs], old, new),
            ValueKind::Unary { rhs, .. } => Self::replace_value_use(rhs, old, new),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                Self::replace_value_uses(args.iter_mut(), old, new);
            }
            ValueKind::Phi { args } => {
                for (value, _) in args {
                    Self::replace_value_use(value, old, new);
                }
            }
            ValueKind::Index1D { base, idx, .. } => {
                Self::replace_value_uses([base, idx], old, new);
            }
            ValueKind::Index2D { base, r, c } => {
                Self::replace_value_uses([base, r, c], old, new);
            }
            ValueKind::Index3D { base, i, j, k } => {
                Self::replace_value_uses([base, i, j, k], old, new);
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                Self::replace_value_use(base, old, new);
            }
            ValueKind::Range { start, end } => Self::replace_value_uses([start, end], old, new),
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    Self::replace_value_use(value, old, new);
                }
            }
            ValueKind::FieldGet { base, .. } => Self::replace_value_use(base, old, new),
            ValueKind::FieldSet { base, value, .. } => {
                Self::replace_value_uses([base, value], old, new);
            }
            _ => {}
        }
    }

    pub(crate) fn replace_instr_uses(instr: &mut Instr, old: ValueId, new: ValueId) {
        match instr {
            Instr::Assign { src, .. } => Self::replace_value_use(src, old, new),
            Instr::Eval { val, .. } => Self::replace_value_use(val, old, new),
            Instr::StoreIndex1D { base, idx, val, .. } => {
                Self::replace_value_uses([base, idx, val], old, new);
            }
            Instr::StoreIndex2D {
                base, r, c, val, ..
            } => {
                Self::replace_value_uses([base, r, c, val], old, new);
            }
            Instr::StoreIndex3D {
                base, i, j, k, val, ..
            } => {
                Self::replace_value_uses([base, i, j, k, val], old, new);
            }
            Instr::UnsafeRBlock { .. } => {}
        }
    }

    pub(crate) fn replace_terminator_uses(term: &mut Terminator, old: ValueId, new: ValueId) {
        match term {
            Terminator::If { cond, .. } => Self::replace_value_use(cond, old, new),
            Terminator::Return(Some(value)) => Self::replace_value_use(value, old, new),
            _ => {}
        }
    }

    pub(crate) fn replace_uses(&self, fn_ir: &mut FnIR, old: ValueId, new: ValueId) {
        for val in &mut fn_ir.values {
            Self::replace_value_kind_uses(&mut val.kind, old, new);
        }

        for blk in &mut fn_ir.blocks {
            for instr in &mut blk.instrs {
                Self::replace_instr_uses(instr, old, new);
            }
            Self::replace_terminator_uses(&mut blk.term, old, new);
        }
    }
}
