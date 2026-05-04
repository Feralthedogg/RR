use super::*;
impl MirSCCP {
    pub(crate) fn build_user_map(&self, fn_ir: &FnIR, users: &mut FxHashMap<ValueId, Vec<User>>) {
        for blk in &fn_ir.blocks {
            match &blk.term {
                Terminator::If { cond, .. } => {
                    users.entry(*cond).or_default().push(User::Block(blk.id));
                }
                Terminator::Return(Some(v)) => {
                    users.entry(*v).or_default().push(User::Block(blk.id));
                }
                _ => {}
            }
            for instr in &blk.instrs {
                match instr {
                    Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                        users.entry(*src).or_default().push(User::Block(blk.id));
                    }
                    Instr::StoreIndex1D { base, idx, val, .. } => {
                        users.entry(*base).or_default().push(User::Block(blk.id));
                        users.entry(*idx).or_default().push(User::Block(blk.id));
                        users.entry(*val).or_default().push(User::Block(blk.id));
                    }
                    Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        users.entry(*base).or_default().push(User::Block(blk.id));
                        users.entry(*r).or_default().push(User::Block(blk.id));
                        users.entry(*c).or_default().push(User::Block(blk.id));
                        users.entry(*val).or_default().push(User::Block(blk.id));
                    }
                    Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        users.entry(*base).or_default().push(User::Block(blk.id));
                        users.entry(*i).or_default().push(User::Block(blk.id));
                        users.entry(*j).or_default().push(User::Block(blk.id));
                        users.entry(*k).or_default().push(User::Block(blk.id));
                        users.entry(*val).or_default().push(User::Block(blk.id));
                    }
                    Instr::UnsafeRBlock { .. } => {}
                }
            }
        }
        for (id, val) in fn_ir.values.iter().enumerate() {
            match &val.kind {
                ValueKind::Binary { lhs, rhs, .. } => {
                    users.entry(*lhs).or_default().push(User::Value(id));
                    users.entry(*rhs).or_default().push(User::Value(id));
                }
                ValueKind::Unary { rhs, .. } => {
                    users.entry(*rhs).or_default().push(User::Value(id));
                }
                ValueKind::Phi { args } => {
                    for (arg, _) in args {
                        users.entry(*arg).or_default().push(User::Value(id));
                    }
                }
                ValueKind::Len { base } | ValueKind::Indices { base } => {
                    users.entry(*base).or_default().push(User::Value(id));
                }
                ValueKind::Call { args, .. } => {
                    for arg in args {
                        users.entry(*arg).or_default().push(User::Value(id));
                    }
                }
                ValueKind::Range { start, end } => {
                    users.entry(*start).or_default().push(User::Value(id));
                    users.entry(*end).or_default().push(User::Value(id));
                }
                ValueKind::Index1D { base, idx, .. } => {
                    users.entry(*base).or_default().push(User::Value(id));
                    users.entry(*idx).or_default().push(User::Value(id));
                }
                ValueKind::Index2D { base, r, c } => {
                    users.entry(*base).or_default().push(User::Value(id));
                    users.entry(*r).or_default().push(User::Value(id));
                    users.entry(*c).or_default().push(User::Value(id));
                }
                ValueKind::Index3D { base, i, j, k } => {
                    users.entry(*base).or_default().push(User::Value(id));
                    users.entry(*i).or_default().push(User::Value(id));
                    users.entry(*j).or_default().push(User::Value(id));
                    users.entry(*k).or_default().push(User::Value(id));
                }
                _ => {}
            }
        }
    }
}
