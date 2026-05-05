use super::*;
pub fn build_pred_map(fn_ir: &FnIR) -> FxHashMap<BlockId, Vec<BlockId>> {
    let mut map = FxHashMap::default();
    for (src, blk) in fn_ir.blocks.iter().enumerate() {
        let targets = match &blk.term {
            Terminator::Goto(t) => vec![*t],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![*then_bb, *else_bb],
            _ => vec![],
        };
        for t in targets {
            map.entry(t).or_insert_with(Vec::new).push(src);
        }
    }
    map
}
