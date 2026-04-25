pub fn verify(fn_ir: &FnIR) -> Result<(), VerifyError> {
    verify_ir(fn_ir)
}

pub fn verify_ir(fn_ir: &FnIR) -> Result<(), VerifyError> {
    // Proof correspondence:
    // `proof/lean/RRProofs/VerifyIrStructLite.lean` and its Coq companion
    // package the reduced structural obligations approximated by this verifier.

    // Entry points must be valid
    check_blk(fn_ir, fn_ir.entry)?;
    check_blk(fn_ir, fn_ir.body_head)?;

    // Precompute reachable blocks for reachability-aware checks
    let reachable = compute_reachable(fn_ir);
    let used_values = collect_used_values(fn_ir, &reachable);
    let loop_headers: FxHashSet<BlockId> = LoopAnalyzer::new(fn_ir)
        .find_loops()
        .into_iter()
        .map(|lp| lp.header)
        .filter(|header| reachable.contains(header))
        .collect();
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
        match fn_ir.blocks[fn_ir.entry].term {
            Terminator::Goto(target) if target == fn_ir.body_head => {}
            _ => {
                return Err(VerifyError::InvalidBodyHeadEntryEdge {
                    entry: fn_ir.entry,
                    body_head: fn_ir.body_head,
                });
            }
        }
        for instr in &fn_ir.blocks[fn_ir.entry].instrs {
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
                }
                Instr::Eval { val: src, .. } => {
                    return Err(VerifyError::InvalidEntryPrologue {
                        block: fn_ir.entry,
                        value: *src,
                    });
                }
                Instr::StoreIndex1D { base, .. }
                | Instr::StoreIndex2D { base, .. }
                | Instr::StoreIndex3D { base, .. } => {
                    return Err(VerifyError::InvalidEntryPrologue {
                        block: fn_ir.entry,
                        value: *base,
                    });
                }
            }
        }
    }

    // 1. Validate all Value definitions and operands
    for (vid, val) in fn_ir.values.iter().enumerate() {
        if val.id != vid {
            return Err(VerifyError::BadValue(vid));
        }

        if !matches!(val.kind, ValueKind::Phi { .. })
            && let Some(block) = val.phi_block
        {
            return Err(VerifyError::InvalidPhiOwner { value: vid, block });
        }
        if !matches!(val.kind, ValueKind::Phi { .. })
            && value_has_direct_self_reference(vid, &val.kind)
        {
            return Err(VerifyError::SelfReferentialValue { value: vid });
        }

        match &val.kind {
            ValueKind::Param { index } => {
                if *index >= fn_ir.params.len() {
                    return Err(VerifyError::InvalidParamIndex {
                        value: vid,
                        index: *index,
                        param_count: fn_ir.params.len(),
                    });
                }
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                check_val(fn_ir, *lhs)?;
                check_val(fn_ir, *rhs)?;
            }
            ValueKind::Unary { rhs, .. } => check_val(fn_ir, *rhs)?,
            ValueKind::Phi { args } => {
                for (v, b) in args {
                    check_val(fn_ir, *v)?;
                    check_blk(fn_ir, *b)?;
                }
            }
            ValueKind::Call { args, names, .. } => {
                if names.len() > args.len() {
                    return Err(VerifyError::InvalidCallArgNames {
                        value: vid,
                        args: args.len(),
                        names: names.len(),
                    });
                }
                for a in args {
                    check_val(fn_ir, *a)?;
                }
            }
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
            ValueKind::Intrinsic { op, args } => {
                let expected = match op {
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
                };
                if args.len() != expected {
                    return Err(VerifyError::InvalidIntrinsicArity {
                        value: vid,
                        expected,
                        got: args.len(),
                    });
                }
                for a in args {
                    check_val(fn_ir, *a)?;
                }
            }
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
            ValueKind::RSymbol { .. } => {}
            ValueKind::Len { base } | ValueKind::Indices { base } => check_val(fn_ir, *base)?,
            ValueKind::Range { start, end } => {
                check_val(fn_ir, *start)?;
                check_val(fn_ir, *end)?;
            }
            _ => {}
        }
    }
    if let Some(value) = detect_non_phi_value_cycle(fn_ir) {
        return Err(VerifyError::NonPhiValueCycle { value });
    }

    // 2. Build predecessor map and validate block structure
    let mut preds: Vec<Vec<BlockId>> = vec![Vec::new(); fn_ir.blocks.len()];
    for (bid, blk) in fn_ir.blocks.iter().enumerate() {
        if blk.id != bid {
            return Err(VerifyError::BadBlock(bid));
        }
        if bid == fn_ir.entry && matches!(blk.term, Terminator::Unreachable) {
            return Err(VerifyError::InvalidEntryTerminator);
        }

        match &blk.term {
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
            Terminator::Return(Some(v)) => check_val(fn_ir, *v)?,
            Terminator::Return(None) => {}
            Terminator::Unreachable => {}
        }
    }
    if let Some(&pred) = preds[fn_ir.entry].first() {
        return Err(VerifyError::InvalidEntryPredecessor { pred });
    }

    let reachable_loops: Vec<_> = LoopAnalyzer::new(fn_ir)
        .find_loops()
        .into_iter()
        .filter(|lp| reachable.contains(&lp.header) && reachable.contains(&lp.latch))
        .collect();

    for lp in &reachable_loops {
        let Terminator::If {
            then_bb, else_bb, ..
        } = fn_ir.blocks[lp.header].term
        else {
            continue;
        };
        let then_in_body = lp.body.contains(&then_bb);
        let else_in_body = lp.body.contains(&else_bb);
        if then_in_body == else_in_body {
            return Err(VerifyError::InvalidLoopHeaderSplit {
                header: lp.header,
                then_in_body,
                else_in_body,
            });
        }
        let header_preds: Vec<BlockId> = preds
            .get(lp.header)
            .cloned()
            .unwrap_or_default()
            .into_iter()
            .filter(|pred| reachable.contains(pred))
            .collect();
        let body_preds = header_preds
            .iter()
            .filter(|pred| lp.body.contains(pred))
            .count();
        let outer_preds = header_preds
            .iter()
            .filter(|pred| {
                !lp.body.contains(pred)
                    && !is_loop_header_forwarder_pred(
                        fn_ir, &preds, &reachable, lp.header, &lp.body, **pred,
                    )
            })
            .count();
        if body_preds != 1 || outer_preds < 1 {
            return Err(VerifyError::InvalidLoopHeaderPredecessors {
                header: lp.header,
                body_preds,
                outer_preds,
            });
        }
        for pred in header_preds {
            if !matches!(fn_ir.blocks[pred].term, Terminator::Goto(target) if target == lp.header) {
                return Err(VerifyError::InvalidLoopHeaderPredecessors {
                    header: lp.header,
                    body_preds,
                    outer_preds,
                });
            }
        }
    }

    // Proof correspondence:
    // `VerifyIrStructLite` models the coarse `Phi` ownership / predecessor /
    // edge-availability obligations checked in this section, and
    // `VerifyIrValueEnvSubset` isolates the reduced predecessor-selected
    // `Phi` environment semantics behind the edge-availability step.
    // 3. Validate Phi shape against CFG predecessors
    for (vid, val) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        let inferred_phi_block = infer_phi_owner_block(fn_ir, args);
        let Some(phi_block) = val.phi_block.or(inferred_phi_block) else {
            if args.is_empty() || !used_values.contains(&vid) {
                continue;
            }
            return Err(VerifyError::InvalidPhiArgs {
                phi_val: vid,
                expected: 0,
                got: args.len(),
            });
        };
        if phi_block >= fn_ir.blocks.len() {
            return Err(VerifyError::InvalidPhiOwnerBlock {
                value: vid,
                block: phi_block,
            });
        }
        if !reachable.contains(&phi_block) {
            continue;
        }
        let expected_preds = &preds[phi_block];
        if expected_preds.is_empty() {
            return Err(VerifyError::InvalidPhiPlacement {
                value: vid,
                block: phi_block,
            });
        }
        let reachable_expected_preds: Vec<BlockId> = expected_preds
            .iter()
            .copied()
            .filter(|pred| reachable.contains(pred))
            .collect();
        if reachable_expected_preds.is_empty() {
            return Err(VerifyError::InvalidPhiPlacement {
                value: vid,
                block: phi_block,
            });
        }
        let reachable_args: Vec<(ValueId, BlockId)> = args
            .iter()
            .copied()
            .filter(|(_, pred)| reachable.contains(pred))
            .collect();
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
        if reachable_args.iter().any(|(arg, _)| *arg == vid) {
            if !reachable_args.iter().any(|(arg, _)| *arg != vid) {
                return Err(VerifyError::SelfReferentialValue { value: vid });
            }
            if reachable_args.iter().any(|(arg, pred)| {
                *arg == vid && (*pred == phi_block || !block_reaches(fn_ir, phi_block, *pred))
            }) {
                return Err(VerifyError::SelfReferentialValue { value: vid });
            }
        }
        let mut seen_phi_preds = FxHashSet::default();
        for (_, pred) in &reachable_args {
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
    }

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
    let (in_defs, out_defs) = compute_must_defined_vars(fn_ir, &reachable, &preds);

    // 4. Validate instructions, use-before-def, and unreachable blocks
    let mut assigned_vars: FxHashSet<VarId> = fn_ir.params.iter().cloned().collect();
    for (bid, blk) in fn_ir.blocks.iter().enumerate() {
        if reachable.contains(&bid) {
            let mut defined_here = in_defs[bid].clone();
            for instr in &blk.instrs {
                match instr {
                    Instr::Assign { dst, src, .. } => {
                        if let Some(value) =
                            first_undefined_load_in_value(fn_ir, *src, &defined_here, false)
                        {
                            return Err(VerifyError::UseBeforeDef { block: bid, value });
                        }
                        assigned_vars.insert(dst.clone());
                        defined_here.insert(dst.clone());
                    }
                    Instr::Eval { val, .. } => {
                        if let Some(value) =
                            first_undefined_load_in_value(fn_ir, *val, &defined_here, false)
                        {
                            return Err(VerifyError::UseBeforeDef { block: bid, value });
                        }
                    }
                    Instr::StoreIndex1D { base, idx, val, .. } => {
                        for root in [*base, *idx, *val] {
                            if let Some(value) =
                                first_undefined_load_in_value(fn_ir, root, &defined_here, false)
                            {
                                return Err(VerifyError::UseBeforeDef { block: bid, value });
                            }
                        }
                    }
                    Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        for root in [*base, *r, *c, *val] {
                            if let Some(value) =
                                first_undefined_load_in_value(fn_ir, root, &defined_here, false)
                            {
                                return Err(VerifyError::UseBeforeDef { block: bid, value });
                            }
                        }
                    }
                    Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        for root in [*base, *i, *j, *k, *val] {
                            if let Some(value) =
                                first_undefined_load_in_value(fn_ir, root, &defined_here, false)
                            {
                                return Err(VerifyError::UseBeforeDef { block: bid, value });
                            }
                        }
                    }
                }
            }

            match &blk.term {
                Terminator::If { cond, .. } => {
                    if let Some(value) =
                        first_undefined_load_in_value(fn_ir, *cond, &defined_here, false)
                    {
                        return Err(VerifyError::UseBeforeDef { block: bid, value });
                    }
                }
                Terminator::Return(Some(value)) => {
                    if let Some(offender) =
                        first_undefined_load_in_value(fn_ir, *value, &defined_here, false)
                    {
                        return Err(VerifyError::UseBeforeDef {
                            block: bid,
                            value: offender,
                        });
                    }
                }
                Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
            }
        }

        for instr in &blk.instrs {
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
            }
        }

        if matches!(blk.term, Terminator::Unreachable)
            && (!blk.instrs.is_empty() || !preds[bid].is_empty())
        {
            return Err(VerifyError::BadTerminator(bid));
        }
    }

    // 5. Phi inputs are evaluated from predecessor contexts and must also be defined there.
    for val in &fn_ir.values {
        let ValueKind::Phi { args } = &val.kind else {
            continue;
        };
        let Some(phi_block) = val.phi_block else {
            continue;
        };
        if !reachable.contains(&phi_block) {
            continue;
        }
        for (arg, pred) in args {
            if !reachable.contains(pred) {
                continue;
            }
            if let Some(value) = first_undefined_load_in_value(fn_ir, *arg, &out_defs[*pred], false)
            {
                return Err(VerifyError::UseBeforeDef {
                    block: *pred,
                    value,
                });
            }
        }
    }

    // 6. Ensure reachable explicit loads point to an assigned variable.
    // `origin_var` is metadata and can legitimately survive after SSA rewrites
    // even when the original local assignment has been eliminated.
    let used_values = collect_used_values(fn_ir, &reachable);
    for vid in used_values {
        let val = &fn_ir.values[vid];
        if let ValueKind::Load { var } = &val.kind
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
