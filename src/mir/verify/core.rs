use super::*;
pub fn verify(fn_ir: &FnIR) -> Result<(), VerifyError> {
    verify_ir(fn_ir)
}

pub fn verify_ir(fn_ir: &FnIR) -> Result<(), VerifyError> {
    // Proof correspondence:
    // `proof/lean/RRProofs/VerifyIrStructLite.lean` and its Coq companion
    // package the reduced structural obligations approximated by this verifier.
    let reachable = compute_reachable(fn_ir);
    let used_values = collect_used_values(fn_ir, &reachable);
    let loop_headers = reachable_loop_headers(fn_ir, &reachable);

    validate_entry_points(fn_ir, &reachable)?;
    validate_value_table(fn_ir)?;
    let preds = validate_block_structure(fn_ir)?;
    validate_reachable_loop_shape(fn_ir, &reachable, &preds)?;
    validate_phi_shapes(fn_ir, &reachable, &used_values, &loop_headers, &preds)?;

    let (in_defs, out_defs) = compute_must_defined_vars(fn_ir, &reachable, &preds);
    let assigned_vars = validate_instruction_flow(fn_ir, &reachable, &preds, &in_defs)?;
    validate_phi_edge_definitions(fn_ir, &reachable, &out_defs)?;
    validate_reachable_loads(fn_ir, &reachable, &assigned_vars)
}

pub(crate) fn reachable_loop_headers(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
) -> FxHashSet<BlockId> {
    LoopAnalyzer::new(fn_ir)
        .find_loops()
        .into_iter()
        .map(|lp| lp.header)
        .filter(|header| reachable.contains(header))
        .collect()
}

pub(crate) fn validate_entry_points(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
) -> Result<(), VerifyError> {
    check_blk(fn_ir, fn_ir.entry)?;
    check_blk(fn_ir, fn_ir.body_head)?;

    if !reachable.contains(&fn_ir.body_head) {
        return Err(VerifyError::InvalidBodyHead {
            block: fn_ir.body_head,
        });
    }
    if matches!(fn_ir.blocks[fn_ir.entry].term, Terminator::Unreachable) {
        return Err(VerifyError::InvalidEntryTerminator);
    }
    if matches!(fn_ir.blocks[fn_ir.body_head].term, Terminator::Unreachable) {
        return Err(VerifyError::InvalidBodyHeadTerminator {
            block: fn_ir.body_head,
        });
    }
    if fn_ir.body_head != fn_ir.entry && fn_is_self_recursive(fn_ir) {
        validate_self_recursive_entry_edge(fn_ir)?;
        for instr in &fn_ir.blocks[fn_ir.entry].instrs {
            validate_entry_prologue_instr(fn_ir, instr)?;
        }
    }

    Ok(())
}

pub(crate) fn validate_self_recursive_entry_edge(fn_ir: &FnIR) -> Result<(), VerifyError> {
    match fn_ir.blocks[fn_ir.entry].term {
        Terminator::Goto(target) if target == fn_ir.body_head => Ok(()),
        _ => Err(VerifyError::InvalidBodyHeadEntryEdge {
            entry: fn_ir.entry,
            body_head: fn_ir.body_head,
        }),
    }
}

pub(crate) fn validate_entry_prologue_instr(
    fn_ir: &FnIR,
    instr: &Instr,
) -> Result<(), VerifyError> {
    match instr {
        Instr::Assign { dst, src, .. } => {
            let ValueKind::Param { index } = fn_ir.values[*src].kind else {
                return Err(VerifyError::InvalidEntryPrologue {
                    block: fn_ir.entry,
                    value: *src,
                });
            };
            let expected_dst = param_runtime_var_name(fn_ir, index);
            if expected_dst.as_deref() != Some(dst.as_str()) {
                return Err(VerifyError::InvalidEntryPrologue {
                    block: fn_ir.entry,
                    value: *src,
                });
            }
            Ok(())
        }
        Instr::Eval { val, .. } => Err(VerifyError::InvalidEntryPrologue {
            block: fn_ir.entry,
            value: *val,
        }),
        Instr::StoreIndex1D { base, .. }
        | Instr::StoreIndex2D { base, .. }
        | Instr::StoreIndex3D { base, .. } => Err(VerifyError::InvalidEntryPrologue {
            block: fn_ir.entry,
            value: *base,
        }),
        Instr::UnsafeRBlock { .. } => Err(VerifyError::InvalidEntryPrologue {
            block: fn_ir.entry,
            value: 0,
        }),
    }
}

pub(crate) fn validate_value_table(fn_ir: &FnIR) -> Result<(), VerifyError> {
    for (vid, value) in fn_ir.values.iter().enumerate() {
        validate_value_record(fn_ir, vid, value)?;
    }

    if let Some(value) = detect_non_phi_value_cycle(fn_ir) {
        return Err(VerifyError::NonPhiValueCycle { value });
    }

    Ok(())
}

pub(crate) fn validate_value_record(
    fn_ir: &FnIR,
    vid: ValueId,
    value: &Value,
) -> Result<(), VerifyError> {
    if value.id != vid {
        return Err(VerifyError::BadValue(vid));
    }

    if !matches!(value.kind, ValueKind::Phi { .. })
        && let Some(block) = value.phi_block
    {
        return Err(VerifyError::InvalidPhiOwner { value: vid, block });
    }
    if !matches!(value.kind, ValueKind::Phi { .. })
        && value_has_direct_self_reference(vid, &value.kind)
    {
        return Err(VerifyError::SelfReferentialValue { value: vid });
    }

    validate_value_kind_operands(fn_ir, vid, value)
}

pub(crate) fn validate_value_kind_operands(
    fn_ir: &FnIR,
    vid: ValueId,
    value: &Value,
) -> Result<(), VerifyError> {
    match &value.kind {
        ValueKind::Param { index } => validate_param_index(fn_ir, vid, *index)?,
        ValueKind::Binary { lhs, rhs, .. } => {
            check_val(fn_ir, *lhs)?;
            check_val(fn_ir, *rhs)?;
        }
        ValueKind::Unary { rhs, .. } => check_val(fn_ir, *rhs)?,
        ValueKind::Phi { args } => validate_phi_operand_refs(fn_ir, args)?,
        ValueKind::Call { args, names, .. } => validate_call_operand_refs(fn_ir, vid, args, names)?,
        ValueKind::RecordLit { fields } => {
            for (_, value) in fields {
                check_val(fn_ir, *value)?;
            }
        }
        ValueKind::FieldGet { base, .. } => check_val(fn_ir, *base)?,
        ValueKind::FieldSet { base, value, .. } => {
            check_val(fn_ir, *base)?;
            check_val(fn_ir, *value)?;
        }
        ValueKind::Intrinsic { op, args } => validate_intrinsic_operands(fn_ir, vid, op, args)?,
        ValueKind::Index1D { base, idx, .. } => {
            check_val(fn_ir, *base)?;
            check_val(fn_ir, *idx)?;
        }
        ValueKind::Index2D { base, r, c } => {
            check_val(fn_ir, *base)?;
            check_val(fn_ir, *r)?;
            check_val(fn_ir, *c)?;
        }
        ValueKind::Index3D { base, i, j, k } => {
            check_val(fn_ir, *base)?;
            check_val(fn_ir, *i)?;
            check_val(fn_ir, *j)?;
            check_val(fn_ir, *k)?;
        }
        ValueKind::Len { base } | ValueKind::Indices { base } => check_val(fn_ir, *base)?,
        ValueKind::Range { start, end } => {
            check_val(fn_ir, *start)?;
            check_val(fn_ir, *end)?;
        }
        ValueKind::RSymbol { .. } => {}
        _ => {}
    }

    Ok(())
}

pub(crate) fn validate_param_index(
    fn_ir: &FnIR,
    vid: ValueId,
    index: usize,
) -> Result<(), VerifyError> {
    if index >= fn_ir.params.len() {
        return Err(VerifyError::InvalidParamIndex {
            value: vid,
            index,
            param_count: fn_ir.params.len(),
        });
    }
    Ok(())
}

pub(crate) fn validate_phi_operand_refs(
    fn_ir: &FnIR,
    args: &[(ValueId, BlockId)],
) -> Result<(), VerifyError> {
    for (value, block) in args {
        check_val(fn_ir, *value)?;
        check_blk(fn_ir, *block)?;
    }
    Ok(())
}

pub(crate) fn validate_call_operand_refs(
    fn_ir: &FnIR,
    vid: ValueId,
    args: &[ValueId],
    names: &[Option<String>],
) -> Result<(), VerifyError> {
    if names.len() > args.len() {
        return Err(VerifyError::InvalidCallArgNames {
            value: vid,
            args: args.len(),
            names: names.len(),
        });
    }
    for arg in args {
        check_val(fn_ir, *arg)?;
    }
    Ok(())
}

pub(crate) fn validate_intrinsic_operands(
    fn_ir: &FnIR,
    vid: ValueId,
    op: &IntrinsicOp,
    args: &[ValueId],
) -> Result<(), VerifyError> {
    let expected = intrinsic_expected_arity(op);
    if args.len() != expected {
        return Err(VerifyError::InvalidIntrinsicArity {
            value: vid,
            expected,
            got: args.len(),
        });
    }
    for arg in args {
        check_val(fn_ir, *arg)?;
    }

    Ok(())
}

pub(crate) fn intrinsic_expected_arity(op: &IntrinsicOp) -> usize {
    match op {
        IntrinsicOp::VecAddF64
        | IntrinsicOp::VecSubF64
        | IntrinsicOp::VecMulF64
        | IntrinsicOp::VecDivF64
        | IntrinsicOp::VecPmaxF64
        | IntrinsicOp::VecPminF64 => 2,
        IntrinsicOp::VecAbsF64
        | IntrinsicOp::VecLogF64
        | IntrinsicOp::VecSqrtF64
        | IntrinsicOp::VecSumF64
        | IntrinsicOp::VecMeanF64 => 1,
    }
}

pub(crate) fn validate_block_structure(fn_ir: &FnIR) -> Result<Vec<Vec<BlockId>>, VerifyError> {
    let mut preds: Vec<Vec<BlockId>> = vec![Vec::new(); fn_ir.blocks.len()];
    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        validate_block_record(fn_ir, bid, block, &mut preds)?;
    }

    if let Some(&pred) = preds[fn_ir.entry].first() {
        return Err(VerifyError::InvalidEntryPredecessor { pred });
    }

    Ok(preds)
}

pub(crate) fn validate_block_record(
    fn_ir: &FnIR,
    bid: BlockId,
    block: &Block,
    preds: &mut [Vec<BlockId>],
) -> Result<(), VerifyError> {
    if block.id != bid {
        return Err(VerifyError::BadBlock(bid));
    }
    if bid == fn_ir.entry && matches!(block.term, Terminator::Unreachable) {
        return Err(VerifyError::InvalidEntryTerminator);
    }

    validate_terminator_edges(fn_ir, bid, &block.term, preds)
}

pub(crate) fn validate_terminator_edges(
    fn_ir: &FnIR,
    bid: BlockId,
    term: &Terminator,
    preds: &mut [Vec<BlockId>],
) -> Result<(), VerifyError> {
    match term {
        Terminator::Goto(target) => {
            check_blk(fn_ir, *target)?;
            preds[*target].push(bid);
        }
        Terminator::If {
            cond,
            then_bb,
            else_bb,
        } => {
            check_val(fn_ir, *cond)?;
            check_blk(fn_ir, *then_bb)?;
            check_blk(fn_ir, *else_bb)?;
            if then_bb == else_bb {
                return Err(VerifyError::InvalidBranchTargets {
                    block: bid,
                    then_bb: *then_bb,
                    else_bb: *else_bb,
                });
            }
            preds[*then_bb].push(bid);
            preds[*else_bb].push(bid);
        }
        Terminator::Return(Some(value)) => check_val(fn_ir, *value)?,
        Terminator::Return(None) | Terminator::Unreachable => {}
    }

    Ok(())
}

pub(crate) fn validate_reachable_loop_shape(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
    preds: &[Vec<BlockId>],
) -> Result<(), VerifyError> {
    let reachable_loops: Vec<_> = LoopAnalyzer::new(fn_ir)
        .find_loops()
        .into_iter()
        .filter(|lp| reachable.contains(&lp.header) && reachable.contains(&lp.latch))
        .collect();

    for loop_info in &reachable_loops {
        validate_reachable_loop_header(fn_ir, reachable, preds, loop_info)?;
    }

    Ok(())
}

pub(crate) fn validate_reachable_loop_header(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
    preds: &[Vec<BlockId>],
    loop_info: &crate::mir::opt::loop_analysis::LoopInfo,
) -> Result<(), VerifyError> {
    let Terminator::If {
        then_bb, else_bb, ..
    } = fn_ir.blocks[loop_info.header].term
    else {
        return Ok(());
    };

    let then_in_body = loop_info.body.contains(&then_bb);
    let else_in_body = loop_info.body.contains(&else_bb);
    if then_in_body == else_in_body {
        return Err(VerifyError::InvalidLoopHeaderSplit {
            header: loop_info.header,
            then_in_body,
            else_in_body,
        });
    }

    let header_preds = reachable_loop_header_preds(reachable, preds, loop_info.header);
    let body_preds = header_preds
        .iter()
        .filter(|pred| loop_info.body.contains(pred))
        .count();
    let outer_preds =
        count_outer_loop_header_preds(fn_ir, reachable, preds, loop_info, &header_preds);
    if body_preds != 1 || outer_preds < 1 {
        return Err(VerifyError::InvalidLoopHeaderPredecessors {
            header: loop_info.header,
            body_preds,
            outer_preds,
        });
    }

    for pred in header_preds {
        if !matches!(fn_ir.blocks[pred].term, Terminator::Goto(target) if target == loop_info.header)
        {
            return Err(VerifyError::InvalidLoopHeaderPredecessors {
                header: loop_info.header,
                body_preds,
                outer_preds,
            });
        }
    }

    Ok(())
}

pub(crate) fn reachable_loop_header_preds(
    reachable: &FxHashSet<BlockId>,
    preds: &[Vec<BlockId>],
    header: BlockId,
) -> Vec<BlockId> {
    preds
        .get(header)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter(|pred| reachable.contains(pred))
        .collect()
}

pub(crate) fn count_outer_loop_header_preds(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
    preds: &[Vec<BlockId>],
    loop_info: &crate::mir::opt::loop_analysis::LoopInfo,
    header_preds: &[BlockId],
) -> usize {
    header_preds
        .iter()
        .filter(|pred| {
            !loop_info.body.contains(pred)
                && !is_loop_header_forwarder_pred(
                    fn_ir,
                    preds,
                    reachable,
                    loop_info.header,
                    &loop_info.body,
                    **pred,
                )
        })
        .count()
}

pub(crate) fn validate_phi_shapes(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
    used_values: &FxHashSet<ValueId>,
    loop_headers: &FxHashSet<BlockId>,
    preds: &[Vec<BlockId>],
) -> Result<(), VerifyError> {
    // Proof correspondence:
    // `VerifyIrStructLite` models the coarse `Phi` ownership / predecessor /
    // edge-availability obligations checked in this section, and
    // `VerifyIrValueEnvSubset` isolates the reduced predecessor-selected
    // `Phi` environment semantics behind the edge-availability step.
    for (vid, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        validate_phi_shape(PhiShapeValidation {
            fn_ir,
            vid,
            value,
            args,
            reachable,
            used_values,
            loop_headers,
            preds,
        })?;
    }

    Ok(())
}

pub(crate) struct PhiShapeValidation<'a> {
    pub(crate) fn_ir: &'a FnIR,
    pub(crate) vid: ValueId,
    pub(crate) value: &'a Value,
    pub(crate) args: &'a [(ValueId, BlockId)],
    pub(crate) reachable: &'a FxHashSet<BlockId>,
    pub(crate) used_values: &'a FxHashSet<ValueId>,
    pub(crate) loop_headers: &'a FxHashSet<BlockId>,
    pub(crate) preds: &'a [Vec<BlockId>],
}

pub(crate) fn validate_phi_shape(request: PhiShapeValidation<'_>) -> Result<(), VerifyError> {
    let inferred_phi_block = infer_phi_owner_block(request.fn_ir, request.args);
    let Some(phi_block) = request.value.phi_block.or(inferred_phi_block) else {
        if request.args.is_empty() || !request.used_values.contains(&request.vid) {
            return Ok(());
        }
        return Err(VerifyError::InvalidPhiArgs {
            phi_val: request.vid,
            expected: 0,
            got: request.args.len(),
        });
    };
    if phi_block >= request.fn_ir.blocks.len() {
        return Err(VerifyError::InvalidPhiOwnerBlock {
            value: request.vid,
            block: phi_block,
        });
    }
    if !request.reachable.contains(&phi_block) {
        return Ok(());
    }

    let expected_preds = validate_phi_placement(request.vid, phi_block, request.preds)?;
    let reachable_expected_preds = reachable_preds(expected_preds, request.reachable);
    if reachable_expected_preds.is_empty() {
        return Err(VerifyError::InvalidPhiPlacement {
            value: request.vid,
            block: phi_block,
        });
    }

    let reachable_args = reachable_phi_args(request.args, request.reachable);
    validate_phi_arg_count(
        request.vid,
        phi_block,
        request.args,
        &reachable_args,
        expected_preds,
        &reachable_expected_preds,
    )?;
    validate_phi_self_reference(request.fn_ir, request.vid, phi_block, &reachable_args)?;
    validate_phi_sources(
        request.vid,
        phi_block,
        &reachable_args,
        &reachable_expected_preds,
    )?;
    validate_phi_edge_values(
        request.fn_ir,
        request.vid,
        phi_block,
        reachable_args,
        request.loop_headers,
    )
}

pub(crate) fn validate_phi_placement(
    vid: ValueId,
    phi_block: BlockId,
    preds: &[Vec<BlockId>],
) -> Result<&[BlockId], VerifyError> {
    let expected_preds = &preds[phi_block];
    if expected_preds.is_empty() {
        return Err(VerifyError::InvalidPhiPlacement {
            value: vid,
            block: phi_block,
        });
    }
    Ok(expected_preds)
}

pub(crate) fn reachable_preds(preds: &[BlockId], reachable: &FxHashSet<BlockId>) -> Vec<BlockId> {
    preds
        .iter()
        .copied()
        .filter(|pred| reachable.contains(pred))
        .collect()
}

pub(crate) fn reachable_phi_args(
    args: &[(ValueId, BlockId)],
    reachable: &FxHashSet<BlockId>,
) -> Vec<(ValueId, BlockId)> {
    args.iter()
        .copied()
        .filter(|(_, pred)| reachable.contains(pred))
        .collect()
}

pub(crate) fn validate_phi_arg_count(
    vid: ValueId,
    phi_block: BlockId,
    args: &[(ValueId, BlockId)],
    reachable_args: &[(ValueId, BlockId)],
    expected_preds: &[BlockId],
    reachable_expected_preds: &[BlockId],
) -> Result<(), VerifyError> {
    let has_dead_only_arms = args.len() != reachable_args.len()
        || args.iter().any(|(_, pred)| !expected_preds.contains(pred));
    if reachable_expected_preds.len() < 2 && !has_dead_only_arms {
        return Err(VerifyError::InvalidPhiPredecessorAliases {
            phi_val: vid,
            block: phi_block,
        });
    }
    if reachable_args.len() != reachable_expected_preds.len() {
        return Err(VerifyError::InvalidPhiArgs {
            phi_val: vid,
            expected: reachable_expected_preds.len(),
            got: reachable_args.len(),
        });
    }
    Ok(())
}

pub(crate) fn validate_phi_self_reference(
    fn_ir: &FnIR,
    vid: ValueId,
    phi_block: BlockId,
    reachable_args: &[(ValueId, BlockId)],
) -> Result<(), VerifyError> {
    if !reachable_args.iter().any(|(arg, _)| *arg == vid) {
        return Ok(());
    }
    if !reachable_args.iter().any(|(arg, _)| *arg != vid) {
        return Err(VerifyError::SelfReferentialValue { value: vid });
    }
    if reachable_args.iter().any(|(arg, pred)| {
        *arg == vid && (*pred == phi_block || !block_reaches(fn_ir, phi_block, *pred))
    }) {
        return Err(VerifyError::SelfReferentialValue { value: vid });
    }
    Ok(())
}

pub(crate) fn validate_phi_sources(
    vid: ValueId,
    phi_block: BlockId,
    reachable_args: &[(ValueId, BlockId)],
    reachable_expected_preds: &[BlockId],
) -> Result<(), VerifyError> {
    let mut seen_phi_preds = FxHashSet::default();
    for (_, pred) in reachable_args {
        if !reachable_expected_preds.contains(pred) {
            return Err(VerifyError::InvalidPhiSource {
                phi_val: vid,
                block: *pred,
            });
        }
        if !seen_phi_preds.insert(*pred) {
            return Err(VerifyError::InvalidPhiPredecessorAliases {
                phi_val: vid,
                block: phi_block,
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_phi_edge_values(
    fn_ir: &FnIR,
    vid: ValueId,
    phi_block: BlockId,
    reachable_args: Vec<(ValueId, BlockId)>,
    loop_headers: &FxHashSet<BlockId>,
) -> Result<(), VerifyError> {
    for (arg, _) in reachable_args {
        if !loop_headers.contains(&phi_block)
            && depends_on_phi_in_block_except(fn_ir, arg, phi_block, vid)
        {
            return Err(VerifyError::InvalidPhiEdgeValue {
                phi_val: vid,
                value: arg,
            });
        }
    }
    Ok(())
}

pub(crate) fn validate_instruction_flow(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
    preds: &[Vec<BlockId>],
    in_defs: &[FxHashSet<VarId>],
) -> Result<FxHashSet<VarId>, VerifyError> {
    // Proof correspondence:
    // `VerifyIrBlockFlowSubset`, `VerifyIrBlockDefinedHereSubset`,
    // `VerifyIrCfgExecutableSubset`, `VerifyIrCfgReachabilitySubset`,
    // `VerifyIrCfgConvergenceSubset`, `VerifyIrCfgWorklistSubset`,
    // `VerifyIrCfgOrderWorklistSubset`, and `VerifyIrCfgFixedPointSubset`,
    // `VerifyIrFlowLite`, and `VerifyIrExecutableLite` approximate the
    // use-before-def and executable-shape checks that follow, with
    // `VerifyIrBlockFlowSubset` threading reduced block payloads back into
    // reduced `UseBeforeDef` obligations through `origin_var` lookup over the
    // reduced value table, and `VerifyIrBlockDefinedHereSubset` isolating the
    // sequential `defined_here` growth induced by local `Assign` instructions,
    // and the `Cfg*` layers lifting those block-local facts to reduced CFG
    // witnesses with predecessor/order and `stepInDefs` justification.
    let mut assigned_vars: FxHashSet<VarId> = fn_ir.params.iter().cloned().collect();
    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        if reachable.contains(&bid) {
            validate_reachable_block_defs(fn_ir, bid, block, &in_defs[bid], &mut assigned_vars)?;
        }
        validate_block_instr_operands(fn_ir, bid, block, reachable, &mut assigned_vars)?;
        validate_unreachable_block_shape(bid, block, preds)?;
    }
    Ok(assigned_vars)
}

pub(crate) fn validate_reachable_block_defs(
    fn_ir: &FnIR,
    bid: BlockId,
    block: &Block,
    in_defs: &FxHashSet<VarId>,
    assigned_vars: &mut FxHashSet<VarId>,
) -> Result<(), VerifyError> {
    let mut defined_here = in_defs.clone();
    for instr in &block.instrs {
        validate_instr_use_before_def(fn_ir, bid, instr, &mut defined_here, assigned_vars)?;
    }
    validate_terminator_use_before_def(fn_ir, bid, &block.term, &defined_here)
}

pub(crate) fn validate_instr_use_before_def(
    fn_ir: &FnIR,
    bid: BlockId,
    instr: &Instr,
    defined_here: &mut FxHashSet<VarId>,
    assigned_vars: &mut FxHashSet<VarId>,
) -> Result<(), VerifyError> {
    match instr {
        Instr::Assign { dst, src, .. } => {
            validate_defined_value_use(fn_ir, bid, *src, defined_here)?;
            assigned_vars.insert(dst.clone());
            defined_here.insert(dst.clone());
        }
        Instr::Eval { val, .. } => validate_defined_value_use(fn_ir, bid, *val, defined_here)?,
        Instr::StoreIndex1D { base, idx, val, .. } => {
            for root in [*base, *idx, *val] {
                validate_defined_value_use(fn_ir, bid, root, defined_here)?;
            }
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            for root in [*base, *r, *c, *val] {
                validate_defined_value_use(fn_ir, bid, root, defined_here)?;
            }
        }
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => {
            for root in [*base, *i, *j, *k, *val] {
                validate_defined_value_use(fn_ir, bid, root, defined_here)?;
            }
        }
        Instr::UnsafeRBlock { .. } => {}
    }
    Ok(())
}

pub(crate) fn validate_defined_value_use(
    fn_ir: &FnIR,
    bid: BlockId,
    root: ValueId,
    defined_here: &FxHashSet<VarId>,
) -> Result<(), VerifyError> {
    if let Some(value) = first_undefined_load_in_value(fn_ir, root, defined_here, false) {
        return Err(VerifyError::UseBeforeDef { block: bid, value });
    }
    Ok(())
}

pub(crate) fn validate_terminator_use_before_def(
    fn_ir: &FnIR,
    bid: BlockId,
    term: &Terminator,
    defined_here: &FxHashSet<VarId>,
) -> Result<(), VerifyError> {
    match term {
        Terminator::If { cond, .. } => validate_defined_value_use(fn_ir, bid, *cond, defined_here),
        Terminator::Return(Some(value)) => {
            validate_defined_value_use(fn_ir, bid, *value, defined_here)
        }
        Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => Ok(()),
    }
}

pub(crate) fn validate_block_instr_operands(
    fn_ir: &FnIR,
    bid: BlockId,
    block: &Block,
    reachable: &FxHashSet<BlockId>,
    assigned_vars: &mut FxHashSet<VarId>,
) -> Result<(), VerifyError> {
    for instr in &block.instrs {
        validate_instr_operands(fn_ir, bid, instr, reachable, assigned_vars)?;
    }
    Ok(())
}

pub(crate) fn validate_instr_operands(
    fn_ir: &FnIR,
    bid: BlockId,
    instr: &Instr,
    reachable: &FxHashSet<BlockId>,
    assigned_vars: &mut FxHashSet<VarId>,
) -> Result<(), VerifyError> {
    match instr {
        Instr::Assign { dst, src, .. } => {
            if reachable.contains(&bid) {
                assigned_vars.insert(dst.clone());
            }
            check_val(fn_ir, *src)?;
        }
        Instr::Eval { val, .. } => check_val(fn_ir, *val)?,
        Instr::StoreIndex1D { base, idx, val, .. } => {
            check_val(fn_ir, *base)?;
            check_val(fn_ir, *idx)?;
            check_val(fn_ir, *val)?;
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            check_val(fn_ir, *base)?;
            check_val(fn_ir, *r)?;
            check_val(fn_ir, *c)?;
            check_val(fn_ir, *val)?;
        }
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => {
            check_val(fn_ir, *base)?;
            check_val(fn_ir, *i)?;
            check_val(fn_ir, *j)?;
            check_val(fn_ir, *k)?;
            check_val(fn_ir, *val)?;
        }
        Instr::UnsafeRBlock { .. } => {}
    }
    Ok(())
}

pub(crate) fn validate_unreachable_block_shape(
    bid: BlockId,
    block: &Block,
    preds: &[Vec<BlockId>],
) -> Result<(), VerifyError> {
    if matches!(block.term, Terminator::Unreachable)
        && (!block.instrs.is_empty() || !preds[bid].is_empty())
    {
        return Err(VerifyError::BadTerminator(bid));
    }
    Ok(())
}

pub(crate) fn validate_phi_edge_definitions(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
    out_defs: &[FxHashSet<VarId>],
) -> Result<(), VerifyError> {
    for value in &fn_ir.values {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        let Some(phi_block) = value.phi_block else {
            continue;
        };
        if !reachable.contains(&phi_block) {
            continue;
        }
        for (arg, pred) in args {
            if !reachable.contains(pred) {
                continue;
            }
            validate_defined_value_use(fn_ir, *pred, *arg, &out_defs[*pred])?;
        }
    }
    Ok(())
}

pub(crate) fn validate_reachable_loads(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
    assigned_vars: &FxHashSet<VarId>,
) -> Result<(), VerifyError> {
    // `origin_var` is metadata and can legitimately survive after SSA rewrites
    // even when the original local assignment has been eliminated.
    let used_values = collect_used_values(fn_ir, reachable);
    for vid in used_values {
        let value = &fn_ir.values[vid];
        if let ValueKind::Load { var } = &value.kind
            && !assigned_vars.contains(var)
            && !fn_ir.params.contains(var)
            && !is_reserved_binding(var)
            && !is_namespaced_r_call(var)
        {
            return Err(VerifyError::UndefinedVar {
                var: var.clone(),
                value: vid,
            });
        }
    }

    Ok(())
}
