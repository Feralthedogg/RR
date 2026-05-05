use crate::mir::{BlockId, FnIR, Terminator};
use rustc_hash::FxHashSet;

pub(super) fn reachable_blocks(fn_ir: &FnIR) -> FxHashSet<BlockId> {
    let mut reachable = FxHashSet::default();
    let mut stack = vec![fn_ir.entry];
    reachable.insert(fn_ir.entry);

    while let Some(bb) = stack.pop() {
        match &fn_ir.blocks[bb].term {
            Terminator::Goto(target) => {
                if reachable.insert(*target) {
                    stack.push(*target);
                }
            }
            Terminator::If {
                then_bb, else_bb, ..
            } => {
                if reachable.insert(*then_bb) {
                    stack.push(*then_bb);
                }
                if reachable.insert(*else_bb) {
                    stack.push(*else_bb);
                }
            }
            Terminator::Return(_) | Terminator::Unreachable => {}
        }
    }

    reachable
}
