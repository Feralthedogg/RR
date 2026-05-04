use super::*;
pub(crate) fn vector_apply_site(fn_ir: &FnIR, lp: &LoopInfo) -> Option<VectorApplySite> {
    let preds = build_pred_map(fn_ir);
    let outer_preds: Vec<BlockId> = preds
        .get(&lp.header)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|b| !lp.body.contains(b))
        .collect();

    if outer_preds.len() != 1 {
        return None;
    }
    if lp.exits.len() != 1 {
        return None;
    }
    if !matches!(fn_ir.blocks[outer_preds[0]].term, Terminator::Goto(next) if next == lp.header) {
        return None;
    }

    Some(VectorApplySite {
        preheader: outer_preds[0],
        exit_bb: lp.exits[0],
    })
}

pub(crate) fn finish_vector_assignment(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    dest_var: VarId,
    out_val: ValueId,
) -> bool {
    finish_vector_assignment_with_shadow_states(fn_ir, site, dest_var, out_val, &[], None)
}

pub(crate) fn finish_vector_assignments(
    fn_ir: &mut FnIR,
    site: VectorApplySite,
    assignments: Vec<PreparedVectorAssignment>,
) -> bool {
    emit_prepared_vector_assignments(fn_ir, site, assignments)
}
