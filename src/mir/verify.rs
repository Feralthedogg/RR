use crate::mir::semantics::call_model::is_namespaced_r_call;
use crate::mir::opt::loop_analysis::LoopAnalyzer;
use crate::mir::*;
use rustc_hash::FxHashSet;
use std::fmt;

#[derive(Debug)]
pub enum VerifyError {
    BadValue(ValueId),
    BadBlock(BlockId),
    BadOperand(ValueId),
    BadTerminator(BlockId),
    UseBeforeDef {
        block: BlockId,
        value: ValueId,
    },
    InvalidPhiArgs {
        phi_val: ValueId,
        expected: usize,
        got: usize,
    },
    InvalidPhiSource {
        phi_val: ValueId,
        block: BlockId,
    },
    InvalidPhiOwner {
        value: ValueId,
        block: BlockId,
    },
    InvalidPhiOwnerBlock {
        value: ValueId,
        block: BlockId,
    },
    InvalidParamIndex {
        value: ValueId,
        index: usize,
        param_count: usize,
    },
    InvalidCallArgNames {
        value: ValueId,
        args: usize,
        names: usize,
    },
    SelfReferentialValue {
        value: ValueId,
    },
    NonPhiValueCycle {
        value: ValueId,
    },
    InvalidBodyHead {
        block: BlockId,
    },
    InvalidEntryPredecessor {
        pred: BlockId,
    },
    InvalidEntryTerminator,
    InvalidBranchTargets {
        block: BlockId,
        then_bb: BlockId,
        else_bb: BlockId,
    },
    InvalidBodyHeadEntryEdge {
        entry: BlockId,
        body_head: BlockId,
    },
    InvalidEntryPrologue {
        block: BlockId,
        value: ValueId,
    },
    InvalidBodyHeadTerminator {
        block: BlockId,
    },
    InvalidLoopHeaderSplit {
        header: BlockId,
        then_in_body: bool,
        else_in_body: bool,
    },
    InvalidLoopHeaderPredecessors {
        header: BlockId,
        body_preds: usize,
        outer_preds: usize,
    },
    InvalidPhiPlacement {
        value: ValueId,
        block: BlockId,
    },
    InvalidPhiPredecessorAliases {
        phi_val: ValueId,
        block: BlockId,
    },
    InvalidPhiEdgeValue {
        phi_val: ValueId,
        value: ValueId,
    },
    UndefinedVar {
        var: VarId,
        value: ValueId,
    },
    ReachablePhi {
        value: ValueId,
    },
    InvalidIntrinsicArity {
        value: ValueId,
        expected: usize,
        got: usize,
    },
}

impl fmt::Display for VerifyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            VerifyError::BadValue(v) => write!(f, "Invalid ValueId: {}", v),
            VerifyError::BadBlock(b) => write!(f, "Invalid BlockId: {}", b),
            VerifyError::BadOperand(v) => write!(f, "Invalid Operand ValueId: {}", v),
            VerifyError::BadTerminator(b) => write!(f, "Invalid Terminator in Block: {}", b),
            VerifyError::UseBeforeDef { block, value } => {
                write!(f, "Use before def in Block {}: Value {}", block, value)
            }
            VerifyError::InvalidPhiArgs {
                phi_val,
                expected,
                got,
            } => write!(
                f,
                "Phi {} has wrong arg count. Expected {}, got {}",
                phi_val, expected, got
            ),
            VerifyError::InvalidPhiSource { phi_val, block } => write!(
                f,
                "Phi {} references invalid predecessor block {}",
                phi_val, block
            ),
            VerifyError::InvalidPhiOwner { value, block } => write!(
                f,
                "Non-Phi value {} carries invalid phi owner block {}",
                value, block
            ),
            VerifyError::InvalidPhiOwnerBlock { value, block } => write!(
                f,
                "Phi value {} references invalid owner block {}",
                value, block
            ),
            VerifyError::InvalidParamIndex {
                value,
                index,
                param_count,
            } => write!(
                f,
                "Param value {} references invalid parameter index {} (param_count={})",
                value, index, param_count
            ),
            VerifyError::InvalidCallArgNames { value, args, names } => write!(
                f,
                "Call value {} has too many argument names: args={}, names={}",
                value, args, names
            ),
            VerifyError::SelfReferentialValue { value } => {
                write!(f, "Value {} directly references itself", value)
            }
            VerifyError::NonPhiValueCycle { value } => {
                write!(f, "Non-Phi value {} participates in a cyclic dependency", value)
            }
            VerifyError::InvalidBodyHead { block } => {
                write!(f, "Function body_head {} is not reachable from entry", block)
            }
            VerifyError::InvalidEntryPredecessor { pred } => {
                write!(f, "Entry block must not have predecessor {}", pred)
            }
            VerifyError::InvalidEntryTerminator => {
                write!(f, "Entry block must not terminate as unreachable")
            }
            VerifyError::InvalidBranchTargets {
                block,
                then_bb,
                else_bb,
            } => {
                write!(
                    f,
                    "Block {} has invalid If targets: then_bb={} else_bb={}",
                    block, then_bb, else_bb
                )
            }
            VerifyError::InvalidBodyHeadEntryEdge { entry, body_head } => {
                write!(
                    f,
                    "entry block {} must jump directly to body_head {} when body_head != entry",
                    entry, body_head
                )
            }
            VerifyError::InvalidEntryPrologue { block, value } => {
                write!(
                    f,
                    "entry block {} contains non-param-copy prologue value {}",
                    block, value
                )
            }
            VerifyError::InvalidBodyHeadTerminator { block } => {
                write!(f, "body_head block {} must not terminate as unreachable", block)
            }
            VerifyError::InvalidLoopHeaderSplit {
                header,
                then_in_body,
                else_in_body,
            } => {
                write!(
                    f,
                    "loop header {} must split into exactly one body successor and one exit successor (then_in_body={}, else_in_body={})",
                    header, then_in_body, else_in_body
                )
            }
            VerifyError::InvalidLoopHeaderPredecessors {
                header,
                body_preds,
                outer_preds,
            } => {
                write!(
                    f,
                    "loop header {} must have exactly one in-body predecessor, at least one outer predecessor, and all such predecessors must jump directly to the header (body_preds={}, outer_preds={})",
                    header, body_preds, outer_preds
                )
            }
            VerifyError::InvalidPhiPlacement { value, block } => {
                write!(
                    f,
                    "Phi value {} is placed in block {} which has no predecessors",
                    value, block
                )
            }
            VerifyError::InvalidPhiPredecessorAliases { phi_val, block } => {
                write!(
                    f,
                    "Phi {} in block {} aliases predecessor arms instead of merging distinct edges",
                    phi_val, block
                )
            }
            VerifyError::InvalidPhiEdgeValue { phi_val, value } => {
                write!(
                    f,
                    "Phi {} uses value {} that is not available on predecessor edges",
                    phi_val, value
                )
            }
            VerifyError::UndefinedVar { var, value } => {
                write!(f, "Value {} refers to undefined var '{}'", value, var)
            }
            VerifyError::ReachablePhi { value } => {
                write!(f, "Reachable Phi {} survived into codegen-ready MIR", value)
            }
            VerifyError::InvalidIntrinsicArity {
                value,
                expected,
                got,
            } => write!(
                f,
                "Intrinsic value {} has invalid arity: expected {}, got {}",
                value, expected, got
            ),
        }
    }
}

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
                        fn_ir,
                        &preds,
                        &reachable,
                        lp.header,
                        &lp.body,
                        **pred,
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
        let Some(phi_block) = val
            .phi_block
            .or(inferred_phi_block)
        else {
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
            || args
                .iter()
                .any(|(_, pred)| !expected_preds.contains(pred));
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
            if reachable_args
                .iter()
                .any(|(arg, pred)| {
                    *arg == vid && (*pred == phi_block || !block_reaches(fn_ir, phi_block, *pred))
                })
            {
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
            if let Some(value) =
                first_undefined_load_in_value(fn_ir, *arg, &out_defs[*pred], false)
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

fn is_loop_header_forwarder_pred(
    fn_ir: &FnIR,
    preds: &[Vec<BlockId>],
    reachable: &FxHashSet<BlockId>,
    header: BlockId,
    body: &FxHashSet<BlockId>,
    pred: BlockId,
) -> bool {
    if body.contains(&pred) {
        return false;
    }
    if !matches!(fn_ir.blocks[pred].term, Terminator::Goto(target) if target == header) {
        return false;
    }
    let incoming: Vec<BlockId> = preds
        .get(pred)
        .into_iter()
        .flatten()
        .copied()
        .filter(|pp| reachable.contains(pp))
        .collect();
    !incoming.is_empty() && incoming.iter().all(|pp| body.contains(pp))
}

pub fn verify_emittable_ir(fn_ir: &FnIR) -> Result<(), VerifyError> {
    // Proof correspondence:
    // `VerifyIrExecutableLite` / `VerifyIrRustErrorLite` approximate the final
    // executable check that no reachable `Phi` survives into codegen-ready MIR.
    verify_ir(fn_ir)?;
    let reachable = compute_reachable(fn_ir);
    let used_values = collect_used_values(fn_ir, &reachable);
    for vid in used_values {
        if matches!(fn_ir.values[vid].kind, ValueKind::Phi { .. }) {
            return Err(VerifyError::ReachablePhi { value: vid });
        }
    }
    Ok(())
}

fn check_val(fn_ir: &FnIR, vid: ValueId) -> Result<(), VerifyError> {
    if vid >= fn_ir.values.len() {
        Err(VerifyError::BadValue(vid))
    } else {
        Ok(())
    }
}

fn check_blk(fn_ir: &FnIR, bid: BlockId) -> Result<(), VerifyError> {
    if bid >= fn_ir.blocks.len() {
        Err(VerifyError::BadBlock(bid))
    } else {
        Ok(())
    }
}

fn param_runtime_var_name(fn_ir: &FnIR, index: usize) -> Option<VarId> {
    for v in &fn_ir.values {
        if let ValueKind::Param { index: i } = v.kind
            && i == index
        {
            if let Some(name) = &v.origin_var {
                return Some(name.clone());
            }
            break;
        }
    }
    fn_ir.params.get(index).cloned()
}

fn fn_is_self_recursive(fn_ir: &FnIR) -> bool {
    fn_ir.values.iter().any(|value| {
        matches!(
            &value.kind,
            ValueKind::Call { callee, .. } if callee == &fn_ir.name
        )
    })
}

fn value_has_direct_self_reference(vid: ValueId, kind: &ValueKind) -> bool {
    match kind {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } | ValueKind::RSymbol { .. } => false,
        ValueKind::Len { base }
        | ValueKind::Indices { base }
        | ValueKind::Unary { rhs: base, .. }
        | ValueKind::FieldGet { base, .. } => *base == vid,
        ValueKind::Range { start, end } => *start == vid || *end == vid,
        ValueKind::Binary { lhs, rhs, .. } => *lhs == vid || *rhs == vid,
        ValueKind::Phi { args } => args.iter().any(|(arg, _)| *arg == vid),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
            args.contains(&vid)
        }
        ValueKind::RecordLit { fields } => fields.iter().any(|(_, value)| *value == vid),
        ValueKind::FieldSet { base, value, .. } => *base == vid || *value == vid,
        ValueKind::Index1D { base, idx, .. } => *base == vid || *idx == vid,
        ValueKind::Index2D { base, r, c } => *base == vid || *r == vid || *c == vid,
        ValueKind::Index3D { base, i, j, k } => {
            *base == vid || *i == vid || *j == vid || *k == vid
        }
    }
}

fn block_reaches(fn_ir: &FnIR, start: BlockId, target: BlockId) -> bool {
    if start == target {
        return true;
    }
    let mut seen = FxHashSet::default();
    let mut stack = vec![start];
    seen.insert(start);
    while let Some(bid) = stack.pop() {
        let succs = match fn_ir.blocks[bid].term {
            Terminator::Goto(next) => [Some(next), None],
            Terminator::If {
                then_bb, else_bb, ..
            } => [Some(then_bb), Some(else_bb)],
            Terminator::Return(_) | Terminator::Unreachable => [None, None],
        };
        for succ in succs.into_iter().flatten() {
            if succ == target {
                return true;
            }
            if seen.insert(succ) {
                stack.push(succ);
            }
        }
    }
    false
}

fn detect_non_phi_value_cycle(fn_ir: &FnIR) -> Option<ValueId> {
    fn visit(fn_ir: &FnIR, vid: ValueId, colors: &mut [u8]) -> Option<ValueId> {
        if matches!(fn_ir.values[vid].kind, ValueKind::Phi { .. }) {
            colors[vid] = 2;
            return None;
        }
        match colors[vid] {
            1 => return Some(vid),
            2 => return None,
            _ => {}
        }
        colors[vid] = 1;
        for dep in non_phi_dependencies(&fn_ir.values[vid].kind) {
            if dep >= fn_ir.values.len() || matches!(fn_ir.values[dep].kind, ValueKind::Phi { .. }) {
                continue;
            }
            if let Some(cycle) = visit(fn_ir, dep, colors) {
                return Some(cycle);
            }
        }
        colors[vid] = 2;
        None
    }

    let mut colors = vec![0u8; fn_ir.values.len()];
    for vid in 0..fn_ir.values.len() {
        if matches!(fn_ir.values[vid].kind, ValueKind::Phi { .. }) || colors[vid] == 2 {
            continue;
        }
        if let Some(value) = visit(fn_ir, vid, &mut colors) {
            return Some(value);
        }
    }
    None
}

fn non_phi_dependencies(kind: &ValueKind) -> Vec<ValueId> {
    // Proof correspondence:
    // `VerifyIrChildDepsSubset` fixes the reduced non-`Phi` child-edge
    // extraction shape for unary wrappers, binary/range pairs, `Call` /
    // `Intrinsic` arg lists, `RecordLit` field values, and `Index*` nodes;
    // `VerifyIrConsumerGraphSubset` then lifts those extracted child ids into
    // a reduced seen/fuel graph closer to the recursive traversal below.
    match kind {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::Load { .. } | ValueKind::RSymbol { .. } => Vec::new(),
        ValueKind::Len { base }
        | ValueKind::Indices { base }
        | ValueKind::Unary { rhs: base, .. }
        | ValueKind::FieldGet { base, .. } => vec![*base],
        ValueKind::Range { start, end } => vec![*start, *end],
        ValueKind::Binary { lhs, rhs, .. } => vec![*lhs, *rhs],
        ValueKind::Phi { .. } => Vec::new(),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args.clone(),
        ValueKind::RecordLit { fields } => fields.iter().map(|(_, value)| *value).collect(),
        ValueKind::FieldSet { base, value, .. } => vec![*base, *value],
        ValueKind::Index1D { base, idx, .. } => vec![*base, *idx],
        ValueKind::Index2D { base, r, c } => vec![*base, *r, *c],
        ValueKind::Index3D { base, i, j, k } => vec![*base, *i, *j, *k],
    }
}

fn depends_on_phi_in_block_except(
    fn_ir: &FnIR,
    root: ValueId,
    phi_block: BlockId,
    exempt_phi: ValueId,
) -> bool {
    // Proof correspondence:
    // `VerifyIrValueDepsWalkSubset` fixes the reduced full `value_dependencies`
    // shape, including `Phi` arg lists, and lifts it into a reduced seen/fuel
    // stack walk approximating this helper's exempt-phi search;
    // `VerifyIrValueTableWalkSubset` then rephrases that walk over an explicit
    // value-table lookup with stored `phi_block` metadata closer to `FnIR.values`,
    // while `VerifyIrValueKindTableSubset` refines those rows to actual
    // `ValueKind`-named payload constructors.
    let mut seen = FxHashSet::default();
    let mut stack = vec![root];

    while let Some(vid) = stack.pop() {
        if vid >= fn_ir.values.len() || !seen.insert(vid) {
            continue;
        }
        let value = &fn_ir.values[vid];
        if matches!(value.kind, ValueKind::Phi { .. }) {
            // Check: is this a different Phi at the same join block?
            if value.phi_block == Some(phi_block) && vid != exempt_phi {
                return true;
            }
            // Stop here: do not follow through Phi args.
            // Phi args come from different control-flow paths, and
            // traversing them (especially across loop back-edges)
            // produces false positives where cross-variable loop
            // dependencies are misidentified as same-block Phi cycles.
            continue;
        }
        for dep in non_phi_dependencies(&value.kind) {
            stack.push(dep);
        }
    }
    false
}

fn infer_phi_owner_block(fn_ir: &FnIR, args: &[(ValueId, BlockId)]) -> Option<BlockId> {
    // Proof correspondence:
    // `VerifyIrStructLite` fixes the reduced owner/join discipline,
    // `VerifyIrValueEnvSubset` models the predecessor-selected value
    // environment that this inferred join block is meant to govern,
    // `VerifyIrArgEnvSubset` extends that reduced env story to arg/field-list
    // consumers under the selected predecessor,
    // `VerifyIrArgEnvTraversalSubset` adds reduced missing-use scans over
    // those selected-edge consumers, and
    // `VerifyIrEnvScanComposeSubset` packages those env-selected scan facts
    // alongside the reduced value-kind scan facts.
    fn successors(fn_ir: &FnIR, bid: BlockId) -> Vec<BlockId> {
        if bid >= fn_ir.blocks.len() {
            return Vec::new();
        }
        match fn_ir.blocks[bid].term {
            Terminator::Goto(target) => vec![target],
            Terminator::If {
                then_bb, else_bb, ..
            } => vec![then_bb, else_bb],
            Terminator::Return(_) | Terminator::Unreachable => Vec::new(),
        }
    }

    let (_, first_pred) = args.first().copied()?;
    let mut common: FxHashSet<BlockId> = successors(fn_ir, first_pred).into_iter().collect();
    for (_, pred) in args.iter().skip(1) {
        let succs: FxHashSet<BlockId> = successors(fn_ir, *pred).into_iter().collect();
        common.retain(|bid| succs.contains(bid));
        if common.is_empty() {
            return None;
        }
    }
    if common.len() == 1 {
        common.into_iter().next()
    } else {
        None
    }
}

fn compute_reachable(fn_ir: &FnIR) -> FxHashSet<BlockId> {
    let mut reachable = FxHashSet::default();
    let mut queue = vec![fn_ir.entry];
    reachable.insert(fn_ir.entry);

    let mut head = 0;
    while head < queue.len() {
        let bid = queue[head];
        head += 1;

        if let Some(blk) = fn_ir.blocks.get(bid) {
            match &blk.term {
                Terminator::Goto(target) => {
                    if reachable.insert(*target) {
                        queue.push(*target);
                    }
                }
                Terminator::If {
                    then_bb, else_bb, ..
                } => {
                    if reachable.insert(*then_bb) {
                        queue.push(*then_bb);
                    }
                    if reachable.insert(*else_bb) {
                        queue.push(*else_bb);
                    }
                }
                _ => {}
            }
        }
    }

    reachable
}

fn compute_must_defined_vars(
    fn_ir: &FnIR,
    reachable: &FxHashSet<BlockId>,
    preds: &[Vec<BlockId>],
) -> (Vec<FxHashSet<VarId>>, Vec<FxHashSet<VarId>>) {
    // Proof correspondence:
    // `VerifyIrMustDefSubset` isolates the reduced predecessor-intersection /
    // local-assign step, `VerifyIrMustDefFixedPointSubset` adds a reduced
    // reachable-pred / one-step iteration model, `VerifyIrMustDefConvergenceSubset`
    // adds reduced stable-seed preservation under iteration, and
    // `VerifyIrFlowLite` packages the resulting use-before-def obligation;
    // this helper is the concrete fixed-point computation over the real CFG
    // predecessor map.
    let universe: FxHashSet<VarId> = fn_ir
        .params
        .iter()
        .cloned()
        .chain(
            fn_ir
                .blocks
                .iter()
                .enumerate()
                .filter(|(bid, _)| reachable.contains(bid))
                .flat_map(|(_, block)| block.instrs.iter())
                .filter_map(|instr| match instr {
                    Instr::Assign { dst, .. } => Some(dst.clone()),
                    _ => None,
                }),
        )
        .collect();
    let entry_defs: FxHashSet<VarId> = fn_ir.params.iter().cloned().collect();

    let mut in_defs = vec![FxHashSet::default(); fn_ir.blocks.len()];
    let mut out_defs = vec![FxHashSet::default(); fn_ir.blocks.len()];
    for bid in 0..fn_ir.blocks.len() {
        if !reachable.contains(&bid) {
            continue;
        }
        in_defs[bid] = if bid == fn_ir.entry {
            entry_defs.clone()
        } else {
            universe.clone()
        };
        out_defs[bid] = universe.clone();
    }

    loop {
        let mut changed = false;
        for bid in 0..fn_ir.blocks.len() {
            if !reachable.contains(&bid) {
                continue;
            }
            let new_in = if bid == fn_ir.entry {
                entry_defs.clone()
            } else {
                let mut reachable_preds = preds[bid].iter().copied().filter(|pred| reachable.contains(pred));
                match reachable_preds.next() {
                    Some(first_pred) => {
                        let mut acc = out_defs[first_pred].clone();
                        for pred in reachable_preds {
                            acc.retain(|var| out_defs[pred].contains(var));
                        }
                        acc
                    }
                    None => FxHashSet::default(),
                }
            };
            if new_in != in_defs[bid] {
                in_defs[bid] = new_in.clone();
                changed = true;
            }

            let mut new_out = new_in;
            for instr in &fn_ir.blocks[bid].instrs {
                if let Instr::Assign { dst, .. } = instr {
                    new_out.insert(dst.clone());
                }
            }
            if new_out != out_defs[bid] {
                out_defs[bid] = new_out;
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }

    (in_defs, out_defs)
}

fn first_undefined_load_in_value(
    fn_ir: &FnIR,
    root: ValueId,
    defined: &FxHashSet<VarId>,
    follow_phi_args: bool,
) -> Option<ValueId> {
    // Proof correspondence:
    // `VerifyIrFlowLite` captures the coarse use-before-def side,
    // `VerifyIrUseTraversalSubset` isolates the reduced recursive wrapper/load
    // scan, `VerifyIrValueKindTraversalSubset` refines that scan to reduced
    // `ValueKind`-named wrappers, `VerifyIrArgListTraversalSubset` adds
    // reduced `Call`/`Intrinsic`/`RecordLit` list-argument scans, and
    // `VerifyIrValueEnvSubset` isolates the `Phi`-edge rewrite/evaluation
    // step when `follow_phi_args` is enabled for predecessor environments;
    // `VerifyIrEnvScanComposeSubset` then composes those reduced env-selected
    // scans with the reduced value-kind arg/field scans under generic list /
    // field clean theorems, and `VerifyIrConsumerMetaSubset` lifts that
    // composition under explicit `Call` / `Intrinsic` / `RecordLit` consumer
    // metadata closer to these concrete match arms; `VerifyIrConsumerGraphSubset`
    // then lifts those reduced consumers into a node-id / seen / fuel graph
    // closer to the recursive `ValueId` traversal and shared-child discipline.
    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        defined: &FxHashSet<VarId>,
        follow_phi_args: bool,
        seen: &mut FxHashSet<ValueId>,
    ) -> Option<ValueId> {
        if !seen.insert(root) {
            return None;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => None,
            ValueKind::Load { var } => {
                if defined.contains(var)
                    || fn_ir.params.contains(var)
                    || is_reserved_binding(var)
                    || is_namespaced_r_call(var)
                {
                    None
                } else {
                    Some(root)
                }
            }
            ValueKind::Phi { args } => {
                if !follow_phi_args {
                    return None;
                }
                for (arg, _) in args {
                    if let Some(value) = rec(fn_ir, *arg, defined, follow_phi_args, seen) {
                        return Some(value);
                    }
                }
                None
            }
            ValueKind::Len { base }
            | ValueKind::Indices { base }
            | ValueKind::FieldGet { base, .. } => rec(fn_ir, *base, defined, follow_phi_args, seen),
            ValueKind::Range { start, end } => rec(fn_ir, *start, defined, follow_phi_args, seen)
                .or_else(|| rec(fn_ir, *end, defined, follow_phi_args, seen)),
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, defined, follow_phi_args, seen),
            ValueKind::Binary { lhs, rhs, .. } => rec(fn_ir, *lhs, defined, follow_phi_args, seen)
                .or_else(|| rec(fn_ir, *rhs, defined, follow_phi_args, seen)),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    if let Some(value) = rec(fn_ir, *arg, defined, follow_phi_args, seen) {
                        return Some(value);
                    }
                }
                None
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    if let Some(offender) = rec(fn_ir, *value, defined, follow_phi_args, seen) {
                        return Some(offender);
                    }
                }
                None
            }
            ValueKind::FieldSet { base, value, .. } => {
                rec(fn_ir, *base, defined, follow_phi_args, seen)
                    .or_else(|| rec(fn_ir, *value, defined, follow_phi_args, seen))
            }
            ValueKind::Index1D { base, idx, .. } => {
                rec(fn_ir, *base, defined, follow_phi_args, seen)
                    .or_else(|| rec(fn_ir, *idx, defined, follow_phi_args, seen))
            }
            ValueKind::Index2D { base, r, c } => rec(fn_ir, *base, defined, follow_phi_args, seen)
                .or_else(|| rec(fn_ir, *r, defined, follow_phi_args, seen))
                .or_else(|| rec(fn_ir, *c, defined, follow_phi_args, seen)),
            ValueKind::Index3D { base, i, j, k } => rec(fn_ir, *base, defined, follow_phi_args, seen)
                .or_else(|| rec(fn_ir, *i, defined, follow_phi_args, seen))
                .or_else(|| rec(fn_ir, *j, defined, follow_phi_args, seen))
                .or_else(|| rec(fn_ir, *k, defined, follow_phi_args, seen)),
        }
    }

    rec(fn_ir, root, defined, follow_phi_args, &mut FxHashSet::default())
}

fn collect_used_values(fn_ir: &FnIR, reachable: &FxHashSet<BlockId>) -> FxHashSet<ValueId> {
    fn push_if_valid(
        fn_ir: &FnIR,
        used: &mut FxHashSet<ValueId>,
        worklist: &mut Vec<ValueId>,
        v: ValueId,
    ) {
        if v < fn_ir.values.len() && used.insert(v) {
            worklist.push(v);
        }
    }

    let mut used = FxHashSet::default();
    let mut worklist: Vec<ValueId> = Vec::new();

    for bid in 0..fn_ir.blocks.len() {
        if !reachable.contains(&bid) {
            continue;
        }
        let blk = &fn_ir.blocks[bid];
        for instr in &blk.instrs {
            match instr {
                Instr::Assign { src, .. } => {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *src);
                }
                Instr::Eval { val, .. } => {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *val);
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    for v in [*base, *idx, *val] {
                        push_if_valid(fn_ir, &mut used, &mut worklist, v);
                    }
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    for v in [*base, *r, *c, *val] {
                        push_if_valid(fn_ir, &mut used, &mut worklist, v);
                    }
                }
                Instr::StoreIndex3D {
                    base, i, j, k, val, ..
                } => {
                    for v in [*base, *i, *j, *k, *val] {
                        push_if_valid(fn_ir, &mut used, &mut worklist, v);
                    }
                }
            }
        }

        match &blk.term {
            Terminator::If { cond, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *cond);
            }
            Terminator::Return(Some(v)) => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *v);
            }
            _ => {}
        }
    }

    while let Some(vid) = worklist.pop() {
        if vid >= fn_ir.values.len() {
            continue;
        }
        let val = &fn_ir.values[vid];
        match &val.kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *lhs);
                push_if_valid(fn_ir, &mut used, &mut worklist, *rhs);
            }
            ValueKind::Unary { rhs, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *rhs);
            }
            ValueKind::Call { args, .. } => {
                for a in args {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *a);
                }
            }
            ValueKind::Intrinsic { args, .. } => {
                for a in args {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *a);
                }
            }
            ValueKind::Phi { args } => {
                for (a, _) in args {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *a);
                }
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    push_if_valid(fn_ir, &mut used, &mut worklist, *value);
                }
            }
            ValueKind::FieldGet { base, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
            }
            ValueKind::FieldSet { base, value, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
                push_if_valid(fn_ir, &mut used, &mut worklist, *value);
            }
            ValueKind::Index1D { base, idx, .. } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
                push_if_valid(fn_ir, &mut used, &mut worklist, *idx);
            }
            ValueKind::Index2D { base, r, c } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
                push_if_valid(fn_ir, &mut used, &mut worklist, *r);
                push_if_valid(fn_ir, &mut used, &mut worklist, *c);
            }
            ValueKind::Index3D { base, i, j, k } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
                push_if_valid(fn_ir, &mut used, &mut worklist, *i);
                push_if_valid(fn_ir, &mut used, &mut worklist, *j);
                push_if_valid(fn_ir, &mut used, &mut worklist, *k);
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *base);
            }
            ValueKind::Range { start, end } => {
                push_if_valid(fn_ir, &mut used, &mut worklist, *start);
                push_if_valid(fn_ir, &mut used, &mut worklist, *end);
            }
            _ => {}
        }
    }

    used
}

fn is_reserved_binding(name: &str) -> bool {
    name.starts_with(".phi_")
        || name.starts_with(".tachyon_")
        || name.starts_with("Sym_")
        || name.starts_with("__lambda_")
        || name.starts_with("rr_")
}

#[cfg(test)]
mod tests {
    use super::{VerifyError, verify_emittable_ir, verify_ir};
    use crate::mir::{Facts, FnIR, Instr, Terminator, ValueKind};
    use crate::syntax::ast::BinOp;
    use crate::utils::Span;

    #[test]
    fn emittable_verify_rejects_reachable_phi() {
        let mut f = FnIR::new("phi_live".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;
        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left), (two, right)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);
        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let err = verify_emittable_ir(&f).expect_err("reachable phi must be rejected");
        assert!(matches!(err, VerifyError::ReachablePhi { value } if value == phi));
    }

    #[test]
    fn verify_ir_rejects_phi_with_wrong_predecessor_count() {
        let mut f = FnIR::new("phi_bad_arity".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let err = verify_ir(&f).expect_err("phi with missing predecessor arm must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidPhiArgs {
                phi_val,
                expected: 2,
                got: 1
            } if phi_val == phi
        ));
    }

    #[test]
    fn verify_ir_rejects_phi_with_non_predecessor_source() {
        let mut f = FnIR::new("phi_bad_source".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left), (two, entry)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let err = verify_ir(&f).expect_err("phi with non-predecessor source must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidPhiSource { phi_val, block }
            if phi_val == phi && block == entry
        ));
    }

    #[test]
    fn verify_ir_rejects_non_phi_with_phi_block_metadata() {
        let mut f = FnIR::new("non_phi_phi_owner".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let value = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        f.values[value].phi_block = Some(entry);
        f.blocks[entry].term = Terminator::Return(Some(value));

        let err = verify_ir(&f).expect_err("non-phi values must not carry phi owner metadata");
        assert!(matches!(
            err,
            VerifyError::InvalidPhiOwner { value: bad_value, block }
            if bad_value == value && block == entry
        ));
    }

    #[test]
    fn verify_ir_rejects_phi_with_invalid_owner_block() {
        let mut f = FnIR::new("phi_bad_owner_block".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, entry)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(999);
        f.blocks[entry].term = Terminator::Return(Some(one));

        let err = verify_ir(&f).expect_err("phi with invalid owner block must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidPhiOwnerBlock { value: bad_value, block }
            if bad_value == phi && block == 999
        ));
    }

    #[test]
    fn verify_ir_rejects_param_with_invalid_index() {
        let mut f = FnIR::new("bad_param_index".to_string(), vec!["x".to_string()]);
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let bad_param = f.add_value(
            ValueKind::Param { index: 3 },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.blocks[entry].term = Terminator::Return(Some(bad_param));

        let err = verify_ir(&f).expect_err("param with invalid index must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidParamIndex {
                value,
                index: 3,
                param_count: 1
            } if value == bad_param
        ));
    }

    #[test]
    fn verify_ir_rejects_call_with_too_many_arg_names() {
        let mut f = FnIR::new("call_bad_names".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let call = f.add_value(
            ValueKind::Call {
                callee: "foo".to_string(),
                args: vec![one],
                names: vec![Some("x".to_string()), Some("y".to_string())],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        f.blocks[entry].term = Terminator::Return(Some(call));

        let err = verify_ir(&f).expect_err("call with too many arg names must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidCallArgNames {
                value,
                args: 1,
                names: 2
            } if value == call
        ));
    }

    #[test]
    fn verify_ir_rejects_self_referential_binary_value() {
        let mut f = FnIR::new("self_ref_binary".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let self_ref = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        f.values[self_ref].kind = ValueKind::Binary {
            op: crate::syntax::ast::BinOp::Add,
            lhs: self_ref,
            rhs: one,
        };
        f.blocks[entry].term = Terminator::Return(Some(self_ref));

        let err = verify_ir(&f).expect_err("self-referential binary value must be rejected");
        assert!(matches!(
            err,
            VerifyError::SelfReferentialValue { value } if value == self_ref
        ));
    }

    #[test]
    fn verify_ir_rejects_self_referential_phi_value() {
        let mut f = FnIR::new("self_ref_phi".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let zero = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );

        let phi = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].kind = ValueKind::Phi {
            args: vec![(zero, left), (phi, right)],
        };
        f.values[phi].phi_block = Some(merge);
        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let err = verify_ir(&f).expect_err("self-referential phi value must be rejected");
        assert!(matches!(
            err,
            VerifyError::SelfReferentialValue { value } if value == phi
        ));
    }

    #[test]
    fn verify_ir_allows_loop_header_self_passthrough_phi_arm() {
        let mut f = FnIR::new("loop_self_phi_passthrough".to_string(), Vec::new());
        let entry = f.add_block();
        let header = f.add_block();
        let next_bb = f.add_block();
        let body = f.add_block();
        let exit = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let zero = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            Some("s".to_string()),
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond_true = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let sum_next = f.add_value(
            ValueKind::Binary {
                op: crate::syntax::ast::BinOp::Add,
                lhs: phi,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            Some("s".to_string()),
        );
        f.values[phi].kind = ValueKind::Phi {
            args: vec![(zero, entry), (phi, next_bb), (sum_next, body)],
        };
        f.values[phi].phi_block = Some(header);

        f.blocks[entry].term = Terminator::Goto(header);
        f.blocks[header].term = Terminator::If {
            cond: cond_true,
            then_bb: next_bb,
            else_bb: exit,
        };
        f.blocks[next_bb].term = Terminator::Goto(header);
        f.blocks[body].term = Terminator::Goto(header);
        f.blocks[exit].term = Terminator::Return(Some(phi));

        verify_ir(&f).expect("loop-header phi may carry its previous value on a body backedge");
    }

    #[test]
    fn verify_ir_rejects_non_phi_mutual_cycle() {
        let mut f = FnIR::new("non_phi_cycle".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let a = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let b = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        f.values[a].kind = ValueKind::Binary {
            op: crate::syntax::ast::BinOp::Add,
            lhs: b,
            rhs: one,
        };
        f.values[b].kind = ValueKind::Binary {
            op: crate::syntax::ast::BinOp::Mul,
            lhs: a,
            rhs: one,
        };
        f.blocks[entry].term = Terminator::Return(Some(a));

        let err = verify_ir(&f).expect_err("non-phi mutual cycle must be rejected");
        assert!(matches!(err, VerifyError::NonPhiValueCycle { value } if value == a || value == b));
    }

    #[test]
    fn verify_ir_rejects_unreachable_body_head() {
        let mut f = FnIR::new("bad_body_head".to_string(), Vec::new());
        let entry = f.add_block();
        let dead = f.add_block();
        f.entry = entry;
        f.body_head = dead;

        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        f.blocks[entry].term = Terminator::Return(Some(one));
        f.blocks[dead].term = Terminator::Return(Some(one));

        let err = verify_ir(&f).expect_err("unreachable body_head must be rejected");
        assert!(matches!(err, VerifyError::InvalidBodyHead { block } if block == dead));
    }

    #[test]
    fn verify_ir_rejects_body_head_with_unreachable_terminator() {
        let mut f = FnIR::new("body_head_unreachable".to_string(), Vec::new());
        let entry = f.add_block();
        let body = f.add_block();
        f.entry = entry;
        f.body_head = body;

        f.blocks[entry].term = Terminator::Goto(body);
        f.blocks[body].term = Terminator::Unreachable;

        let err =
            verify_ir(&f).expect_err("body_head with unreachable terminator must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidBodyHeadTerminator { block } if block == body
        ));
    }

    #[test]
    fn verify_ir_rejects_entry_with_predecessor() {
        let mut f = FnIR::new("entry_has_pred".to_string(), Vec::new());
        let entry = f.add_block();
        let stray = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        f.blocks[entry].term = Terminator::Return(Some(one));
        f.blocks[stray].term = Terminator::Goto(entry);

        let err = verify_ir(&f).expect_err("entry predecessor must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidEntryPredecessor { pred } if pred == stray
        ));
    }

    #[test]
    fn verify_ir_rejects_unreachable_entry() {
        let mut f = FnIR::new("entry_unreachable".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;
        f.blocks[entry].term = Terminator::Unreachable;

        let err = verify_ir(&f).expect_err("unreachable entry must be rejected");
        assert!(matches!(err, VerifyError::InvalidEntryTerminator));
    }

    #[test]
    fn verify_ir_rejects_phi_in_zero_predecessor_block() {
        let mut f = FnIR::new("phi_zero_pred".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let phi = f.add_value(
            ValueKind::Phi { args: vec![] },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(entry);
        f.blocks[entry].term = Terminator::Return(Some(phi));

        let err = verify_ir(&f).expect_err("phi in zero-predecessor block must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidPhiPlacement { value, block }
            if value == phi && block == entry
        ));
    }

    #[test]
    fn verify_ir_rejects_phi_in_single_predecessor_block() {
        let mut f = FnIR::new("phi_single_pred".to_string(), Vec::new());
        let entry = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let c1 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(c1, entry)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let err = verify_ir(&f).expect_err("phi in single-predecessor block must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidPhiPredecessorAliases { phi_val, block }
            if phi_val == phi && block == merge
        ));
    }

    #[test]
    fn verify_ir_rejects_phi_with_duplicate_predecessor_arm() {
        let mut f = FnIR::new("phi_duplicate_pred_arm".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left), (two, left)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi));

        let err = verify_ir(&f).expect_err("phi with duplicate predecessor arm must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidPhiPredecessorAliases { phi_val, block }
            if phi_val == phi && block == merge
        ));
    }

    #[test]
    fn verify_ir_rejects_phi_arg_from_same_phi_block() {
        let mut f = FnIR::new("phi_same_block_arg".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi_a = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left), (two, right)],
            },
            Span::default(),
            Facts::empty(),
            Some("a".to_string()),
        );
        f.values[phi_a].phi_block = Some(merge);
        let phi_b = f.add_value(
            ValueKind::Phi {
                args: vec![(phi_a, left), (two, right)],
            },
            Span::default(),
            Facts::empty(),
            Some("b".to_string()),
        );
        f.values[phi_b].phi_block = Some(merge);

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi_b));

        let err = verify_ir(&f).expect_err("same-block phi operand must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidPhiEdgeValue { phi_val, value }
            if phi_val == phi_b && value == phi_a
        ));
    }

    #[test]
    fn verify_ir_rejects_phi_arg_from_same_phi_block_via_intrinsic_wrapper() {
        let mut f = FnIR::new("phi_same_block_wrapped_arg".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Float(1.0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Float(2.0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi_a = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left), (two, right)],
            },
            Span::default(),
            Facts::empty(),
            Some("a".to_string()),
        );
        f.values[phi_a].phi_block = Some(merge);
        let wrapped = f.add_value(
            ValueKind::Intrinsic {
                op: crate::mir::IntrinsicOp::VecAbsF64,
                args: vec![phi_a],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi_b = f.add_value(
            ValueKind::Phi {
                args: vec![(wrapped, left), (two, right)],
            },
            Span::default(),
            Facts::empty(),
            Some("b".to_string()),
        );
        f.values[phi_b].phi_block = Some(merge);

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(phi_b));

        let err = verify_ir(&f)
            .expect_err("phi operand depending on same-block phi must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidPhiEdgeValue { phi_val, value }
            if phi_val == phi_b && value == wrapped
        ));
    }

    #[test]
    fn emittable_verify_rejects_reachable_phi_nested_in_intrinsic() {
        let mut f = FnIR::new("phi_live_nested_intrinsic".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let merge = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Float(1.0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Float(2.0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left), (two, right)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[phi].phi_block = Some(merge);
        let intrinsic = f.add_value(
            ValueKind::Intrinsic {
                op: crate::mir::IntrinsicOp::VecAbsF64,
                args: vec![phi],
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(merge);
        f.blocks[right].term = Terminator::Goto(merge);
        f.blocks[merge].term = Terminator::Return(Some(intrinsic));

        let err =
            verify_emittable_ir(&f).expect_err("reachable phi nested in intrinsic must be rejected");
        assert!(matches!(err, VerifyError::ReachablePhi { value } if value == phi));
    }

    #[test]
    fn verify_ir_rejects_record_literal_with_bad_field_operand() {
        let mut f = FnIR::new("record_bad_operand".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let record = f.add_value(
            ValueKind::RecordLit {
                fields: vec![("x".to_string(), 999)],
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        f.blocks[entry].term = Terminator::Return(Some(record));

        let err = verify_ir(&f).expect_err("record literal with invalid field operand must be rejected");
        assert!(matches!(err, VerifyError::BadValue(999)));
    }

    #[test]
    fn verify_ir_rejects_same_block_use_before_def() {
        let mut f = FnIR::new("same_block_use_before_def".to_string(), Vec::new());
        let entry = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let load_x = f.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].instrs.push(Instr::Eval {
            val: load_x,
            span: Span::default(),
        });
        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: one,
            span: Span::default(),
        });
        f.blocks[entry].term = Terminator::Return(None);

        let err = verify_ir(&f).expect_err("same-block load before assignment must be rejected");
        assert!(matches!(
            err,
            VerifyError::UseBeforeDef { block, value }
            if block == entry && value == load_x
        ));
    }

    #[test]
    fn verify_ir_rejects_join_use_without_def_on_all_paths() {
        let mut f = FnIR::new("join_use_before_def".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let join = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let load_x = f.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: one,
            span: Span::default(),
        });
        f.blocks[left].term = Terminator::Goto(join);
        f.blocks[right].term = Terminator::Goto(join);
        f.blocks[join].term = Terminator::Return(Some(load_x));

        let err = verify_ir(&f).expect_err("join load without all-path definition must be rejected");
        assert!(matches!(
            err,
            VerifyError::UseBeforeDef { block, value }
            if block == join && value == load_x
        ));
    }

    #[test]
    fn verify_ir_rejects_if_with_identical_branch_targets() {
        let mut f = FnIR::new("identical_if_targets".to_string(), Vec::new());
        let entry = f.add_block();
        let join = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: join,
            else_bb: join,
        };
        f.blocks[join].term = Terminator::Return(None);

        let err = verify_ir(&f).expect_err("identical If targets must be rejected");
        assert!(matches!(
            err,
            VerifyError::InvalidBranchTargets {
                block,
                then_bb,
                else_bb
            } if block == entry && then_bb == join && else_bb == join
        ));
    }

    #[test]
    fn verify_ir_rejects_body_head_without_direct_entry_goto() {
        let mut f = FnIR::new("body_head_entry_edge".to_string(), Vec::new());
        let entry = f.add_block();
        let body = f.add_block();
        let other = f.add_block();
        f.entry = entry;
        f.body_head = body;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let self_call = f.add_value(
            ValueKind::Call {
                callee: "body_head_entry_edge".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: other,
        };
        f.blocks[body].term = Terminator::Return(Some(self_call));
        f.blocks[other].term = Terminator::Return(None);

        let err =
            verify_ir(&f).expect_err("body_head must be entered by a direct entry goto");
        assert!(matches!(
            err,
            VerifyError::InvalidBodyHeadEntryEdge { entry: e, body_head: h }
            if e == entry && h == body
        ));
    }

    #[test]
    fn verify_ir_rejects_non_param_entry_prologue_when_body_head_is_separate() {
        let mut f = FnIR::new("entry_prologue_not_param_copy".to_string(), vec!["p".to_string()]);
        let entry = f.add_block();
        let body = f.add_block();
        f.entry = entry;
        f.body_head = body;

        let c1 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let self_call = f.add_value(
            ValueKind::Call {
                callee: "entry_prologue_not_param_copy".to_string(),
                args: vec![c1],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "x".to_string(),
            src: c1,
            span: Span::default(),
        });
        f.blocks[entry].term = Terminator::Goto(body);
        f.blocks[body].term = Terminator::Return(Some(self_call));

        let err = verify_ir(&f)
            .expect_err("separate body_head entry prologue must be param-copy-only");
        assert!(matches!(
            err,
            VerifyError::InvalidEntryPrologue { block, value }
            if block == entry && value == c1
        ));
    }

    #[test]
    fn verify_ir_rejects_entry_prologue_copy_into_non_runtime_param_target() {
        let mut f =
            FnIR::new("entry_prologue_wrong_param_target".to_string(), vec!["p".to_string()]);
        let entry = f.add_block();
        let body = f.add_block();
        f.entry = entry;
        f.body_head = body;

        let p0 = f.add_value(
            ValueKind::Param { index: 0 },
            Span::default(),
            Facts::empty(),
            Some(".arg_p".to_string()),
        );
        let self_call = f.add_value(
            ValueKind::Call {
                callee: "entry_prologue_wrong_param_target".to_string(),
                args: vec![p0],
                names: vec![None],
            },
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].instrs.push(Instr::Assign {
            dst: "tmp".to_string(),
            src: p0,
            span: Span::default(),
        });
        f.blocks[entry].term = Terminator::Goto(body);
        f.blocks[body].term = Terminator::Return(Some(self_call));

        let err = verify_ir(&f)
            .expect_err("separate body_head entry prologue must copy params only into runtime param targets");
        assert!(matches!(
            err,
            VerifyError::InvalidEntryPrologue { block, value }
            if block == entry && value == p0
        ));
    }

    #[test]
    fn verify_ir_rejects_loop_header_with_both_branches_in_body() {
        let mut f = FnIR::new("loop_header_split".to_string(), Vec::new());
        let entry = f.add_block();
        let header = f.add_block();
        let body1 = f.add_block();
        let body2 = f.add_block();
        let exit = f.add_block();
        f.entry = entry;
        f.body_head = header;

        let cond1 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond2 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Goto(header);
        f.blocks[header].term = Terminator::If {
            cond: cond1,
            then_bb: body1,
            else_bb: body2,
        };
        f.blocks[body1].term = Terminator::Goto(body2);
        f.blocks[body2].term = Terminator::If {
            cond: cond2,
            then_bb: header,
            else_bb: exit,
        };
        f.blocks[exit].term = Terminator::Return(None);

        let err = verify_ir(&f)
            .expect_err("loop header must have exactly one body successor and one exit successor");
        assert!(matches!(
            err,
            VerifyError::InvalidLoopHeaderSplit {
                header: h,
                then_in_body: true,
                else_in_body: true
            } if h == header
        ));
    }

    #[test]
    fn verify_ir_ignores_unreachable_phi_shape() {
        let mut f = FnIR::new("unreachable_phi_shape".to_string(), Vec::new());
        let entry = f.add_block();
        let exit = f.add_block();
        let dead_pred = f.add_block();
        let dead_header = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let zero = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(zero, dead_pred)],
            },
            Span::default(),
            Facts::empty(),
            Some("dead".to_string()),
        );
        f.values[phi].phi_block = Some(dead_header);

        f.blocks[entry].term = Terminator::Goto(exit);
        f.blocks[exit].term = Terminator::Return(Some(zero));
        f.blocks[dead_pred].term = Terminator::Goto(dead_header);
        f.blocks[dead_header].term = Terminator::Return(Some(phi));

        verify_ir(&f).expect("unreachable phi shape should not block verifier");
    }

    #[test]
    fn verify_ir_ignores_dead_only_phi_arm_on_reachable_join() {
        let mut f = FnIR::new("dead_only_phi_arm".to_string(), Vec::new());
        let entry = f.add_block();
        let live_pred = f.add_block();
        let join = f.add_block();
        let dead_arm = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let dead_phi_seed = f.add_value(
            ValueKind::Phi { args: vec![] },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[dead_phi_seed].phi_block = Some(dead_arm);
        let join_phi = f.add_value(
            ValueKind::Phi {
                args: vec![(dead_phi_seed, dead_arm), (one, live_pred)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        f.values[join_phi].phi_block = Some(join);

        f.blocks[entry].term = Terminator::Goto(live_pred);
        f.blocks[live_pred].term = Terminator::Goto(join);
        f.blocks[join].term = Terminator::Return(Some(join_phi));
        f.blocks[dead_arm].term = Terminator::Unreachable;

        verify_ir(&f).expect("dead-only phi arm should not block reachable join verification");
    }

    #[test]
    fn verify_ir_ignores_unused_unreachable_phi_without_owner_block() {
        let mut f = FnIR::new("unused_unreachable_phi_without_owner".to_string(), Vec::new());
        let entry = f.add_block();
        let exit = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let zero = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let _dead_phi = f.add_value(
            ValueKind::Phi {
                args: vec![(zero, entry), (one, exit)],
            },
            Span::default(),
            Facts::empty(),
            Some("dead".to_string()),
        );

        f.blocks[entry].term = Terminator::Goto(exit);
        f.blocks[exit].term = Terminator::Return(Some(zero));

        verify_ir(&f).expect("unused unreachable phi without owner block should be ignored");
    }

    #[test]
    fn verify_ir_accepts_ownerless_phi_when_join_block_is_uniquely_inferred() {
        let mut f = FnIR::new("ownerless_phi_inferred_join".to_string(), Vec::new());
        let entry = f.add_block();
        let left = f.add_block();
        let right = f.add_block();
        let join = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let two = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(2)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(one, left), (two, right)],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: left,
            else_bb: right,
        };
        f.blocks[left].term = Terminator::Goto(join);
        f.blocks[right].term = Terminator::Goto(join);
        f.blocks[join].term = Terminator::Return(Some(phi));

        verify_ir(&f).expect("ownerless phi with unique join block should be accepted");
    }

    #[test]
    fn verify_ir_allows_loop_phi_backedge_value_depending_on_same_phi() {
        let mut f = FnIR::new("loop_phi_backedge_value".to_string(), Vec::new());
        let entry = f.add_block();
        let header = f.add_block();
        let body = f.add_block();
        let exit = f.add_block();
        f.entry = entry;
        f.body_head = header;

        let zero = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = f.add_value(
            ValueKind::Phi {
                args: vec![(zero, entry), (zero, body)],
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        f.values[phi].phi_block = Some(header);
        let next = f.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        if let ValueKind::Phi { args } = &mut f.values[phi].kind {
            args[1] = (next, body);
        }
        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Goto(header);
        f.blocks[header].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: exit,
        };
        f.blocks[body].term = Terminator::Goto(header);
        f.blocks[exit].term = Terminator::Return(Some(phi));

        verify_ir(&f).expect("loop-carried backedge value using the same phi should be allowed");
    }

    #[test]
    fn verify_ir_allows_loop_phi_backedge_value_depending_on_other_header_phi() {
        let mut f = FnIR::new("loop_phi_cross_header_value".to_string(), Vec::new());
        let entry = f.add_block();
        let header = f.add_block();
        let body = f.add_block();
        let exit = f.add_block();
        f.entry = entry;
        f.body_head = header;

        let zero = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let one = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let i_phi = f.add_value(
            ValueKind::Phi {
                args: vec![(zero, entry), (zero, body)],
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        f.values[i_phi].phi_block = Some(header);
        let sum_phi = f.add_value(
            ValueKind::Phi {
                args: vec![(zero, entry), (zero, body)],
            },
            Span::default(),
            Facts::empty(),
            Some("sum".to_string()),
        );
        f.values[sum_phi].phi_block = Some(header);
        let i_next = f.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: i_phi,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let sum_next = f.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: sum_phi,
                rhs: i_phi,
            },
            Span::default(),
            Facts::empty(),
            Some("sum".to_string()),
        );
        if let ValueKind::Phi { args } = &mut f.values[i_phi].kind {
            args[1] = (i_next, body);
        }
        if let ValueKind::Phi { args } = &mut f.values[sum_phi].kind {
            args[1] = (sum_next, body);
        }
        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Goto(header);
        f.blocks[header].term = Terminator::If {
            cond,
            then_bb: body,
            else_bb: exit,
        };
        f.blocks[body].term = Terminator::Goto(header);
        f.blocks[exit].term = Terminator::Return(Some(sum_phi));

        verify_ir(&f)
            .expect("loop header backedge values may depend on other header phis");
    }

    #[test]
    fn verify_ir_ignores_unreachable_loop_backedge_shape() {
        let mut f = FnIR::new("unreachable_loop_backedge".to_string(), Vec::new());
        let entry = f.add_block();
        let header = f.add_block();
        let live_exit = f.add_block();
        let live_else = f.add_block();
        let dead_latch = f.add_block();
        f.entry = entry;
        f.body_head = header;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let zero = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Int(0)),
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Goto(header);
        f.blocks[header].term = Terminator::If {
            cond,
            then_bb: live_exit,
            else_bb: live_else,
        };
        f.blocks[live_exit].term = Terminator::Return(Some(zero));
        f.blocks[live_else].term = Terminator::Return(Some(zero));
        f.blocks[dead_latch].term = Terminator::Goto(header);

        verify_ir(&f).expect("unreachable backedge must not create a fake reachable loop");
    }

    #[test]
    fn verify_ir_rejects_loop_header_with_multiple_body_predecessors() {
        let mut f = FnIR::new("loop_header_multi_latch".to_string(), Vec::new());
        let entry = f.add_block();
        let header = f.add_block();
        let branch = f.add_block();
        let body_a = f.add_block();
        let body_b = f.add_block();
        let exit = f.add_block();
        f.entry = entry;
        f.body_head = header;

        let cond1 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond2 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond3 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Goto(header);
        f.blocks[header].term = Terminator::If {
            cond: cond1,
            then_bb: branch,
            else_bb: exit,
        };
        f.blocks[branch].term = Terminator::If {
            cond: cond2,
            then_bb: body_a,
            else_bb: body_b,
        };
        // `body_a` is both a direct predecessor of the header and a predecessor
        // of the latch, so the natural loop discovered from `body_b -> header`
        // contains two distinct in-body header predecessors (`body_a`, `body_b`)
        // rather than a body-only forwarder.
        f.blocks[body_a].term = Terminator::If {
            cond: cond3,
            then_bb: header,
            else_bb: body_b,
        };
        f.blocks[body_b].term = Terminator::Goto(header);
        f.blocks[exit].term = Terminator::Return(None);

        let err = verify_ir(&f)
            .expect_err("loop header must not have multiple in-body predecessors");
        assert!(matches!(
            err,
            VerifyError::InvalidLoopHeaderPredecessors {
                header: h,
                body_preds: _,
                outer_preds: _,
            } if h == header
        ));
    }

    #[test]
    fn verify_ir_allows_loop_header_with_body_forwarder_backedge() {
        let mut f = FnIR::new("loop_header_body_forwarder".to_string(), Vec::new());
        let entry = f.add_block();
        let header = f.add_block();
        let branch = f.add_block();
        let continue_fwd = f.add_block();
        let latch = f.add_block();
        let exit = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond1 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond2 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Goto(header);
        f.blocks[header].term = Terminator::If {
            cond: cond1,
            then_bb: branch,
            else_bb: exit,
        };
        f.blocks[branch].term = Terminator::If {
            cond: cond2,
            then_bb: continue_fwd,
            else_bb: latch,
        };
        f.blocks[continue_fwd].term = Terminator::Goto(header);
        f.blocks[latch].term = Terminator::Goto(header);
        f.blocks[exit].term = Terminator::Return(None);

        verify_ir(&f).expect("body-only forwarder backedge should be accepted");
    }

    #[test]
    fn verify_ir_allows_loop_header_with_multiple_outer_predecessors() {
        let mut f = FnIR::new("loop_header_multi_outer".to_string(), Vec::new());
        let entry = f.add_block();
        let outer_left = f.add_block();
        let outer_right = f.add_block();
        let header = f.add_block();
        let body = f.add_block();
        let exit = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let loop_cond = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::If {
            cond,
            then_bb: outer_left,
            else_bb: outer_right,
        };
        f.blocks[outer_left].term = Terminator::Goto(header);
        f.blocks[outer_right].term = Terminator::Goto(header);
        f.blocks[header].term = Terminator::If {
            cond: loop_cond,
            then_bb: body,
            else_bb: exit,
        };
        f.blocks[body].term = Terminator::Goto(header);
        f.blocks[exit].term = Terminator::Return(None);

        verify_ir(&f).expect("multiple direct outer predecessors should be accepted");
    }

    #[test]
    fn verify_ir_rejects_loop_header_with_conditional_outer_predecessor() {
        let mut f = FnIR::new("loop_header_outer_if_pred".to_string(), Vec::new());
        let entry = f.add_block();
        let guard = f.add_block();
        let header = f.add_block();
        let body = f.add_block();
        let exit = f.add_block();
        f.entry = entry;
        f.body_head = entry;

        let cond1 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let cond2 = f.add_value(
            ValueKind::Const(crate::syntax::ast::Lit::Bool(true)),
            Span::default(),
            Facts::empty(),
            None,
        );

        f.blocks[entry].term = Terminator::Goto(guard);
        f.blocks[guard].term = Terminator::If {
            cond: cond1,
            then_bb: header,
            else_bb: exit,
        };
        f.blocks[header].term = Terminator::If {
            cond: cond2,
            then_bb: body,
            else_bb: exit,
        };
        f.blocks[body].term = Terminator::Goto(header);
        f.blocks[exit].term = Terminator::Return(None);

        let err = verify_ir(&f)
            .expect_err("loop header outer predecessor must jump directly to header");
        assert!(matches!(
            err,
            VerifyError::InvalidLoopHeaderPredecessors {
                header: h,
                body_preds: 1,
                outer_preds: 1,
            } if h == header
        ));
    }
}
