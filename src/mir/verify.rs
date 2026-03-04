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
    UndefinedVar {
        var: VarId,
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
            VerifyError::UndefinedVar { var, value } => {
                write!(f, "Value {} refers to undefined var '{}'", value, var)
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
    // Entry points must be valid
    check_blk(fn_ir, fn_ir.entry)?;
    check_blk(fn_ir, fn_ir.body_head)?;

    // Precompute reachable blocks for reachability-aware checks
    let reachable = compute_reachable(fn_ir);

    // 1. Validate all Value definitions and operands
    for (vid, val) in fn_ir.values.iter().enumerate() {
        if val.id != vid {
            return Err(VerifyError::BadValue(vid));
        }

        match &val.kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                check_val(fn_ir, *lhs)?;
                check_val(fn_ir, *rhs)?;
            }
            ValueKind::Unary { rhs, .. } => check_val(fn_ir, *rhs)?,
            ValueKind::Phi { args } => {
                let mut seen = FxHashSet::default();
                for (v, b) in args {
                    check_val(fn_ir, *v)?;
                    check_blk(fn_ir, *b)?;
                    if !seen.insert(*b) {
                        return Err(VerifyError::InvalidPhiSource {
                            phi_val: vid,
                            block: *b,
                        });
                    }
                }
            }
            ValueKind::Call { args, .. } => {
                for a in args {
                    check_val(fn_ir, *a)?;
                }
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
            ValueKind::Len { base } | ValueKind::Indices { base } => check_val(fn_ir, *base)?,
            ValueKind::Range { start, end } => {
                check_val(fn_ir, *start)?;
                check_val(fn_ir, *end)?;
            }
            _ => {}
        }
    }

    // 2. Build predecessor map and validate block structure
    let mut preds: Vec<Vec<BlockId>> = vec![Vec::new(); fn_ir.blocks.len()];
    for (bid, blk) in fn_ir.blocks.iter().enumerate() {
        if blk.id != bid {
            return Err(VerifyError::BadBlock(bid));
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
                preds[*then_bb].push(bid);
                preds[*else_bb].push(bid);
            }
            Terminator::Return(Some(v)) => check_val(fn_ir, *v)?,
            Terminator::Return(None) => {}
            Terminator::Unreachable => {}
        }
    }

    // 3. Validate instructions and unreachable blocks
    let mut assigned_vars: FxHashSet<VarId> = FxHashSet::default();
    for (bid, blk) in fn_ir.blocks.iter().enumerate() {
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
            }
        }

        if matches!(blk.term, Terminator::Unreachable) {
            if !blk.instrs.is_empty() || !preds[bid].is_empty() {
                return Err(VerifyError::BadTerminator(bid));
            }
        }
    }

    // 4. Ensure origin_var points to an assigned variable (reachable-only)
    let used_values = collect_used_values(fn_ir, &reachable);
    for vid in used_values {
        let val = &fn_ir.values[vid];
        if let Some(name) = &val.origin_var {
            if !assigned_vars.contains(name) && !is_reserved_binding(name) {
                return Err(VerifyError::UndefinedVar {
                    var: name.clone(),
                    value: vid,
                });
            }
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

fn collect_used_values(fn_ir: &FnIR, reachable: &FxHashSet<BlockId>) -> FxHashSet<ValueId> {
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
                    if used.insert(*src) {
                        worklist.push(*src);
                    }
                }
                Instr::Eval { val, .. } => {
                    if used.insert(*val) {
                        worklist.push(*val);
                    }
                }
                Instr::StoreIndex1D { base, idx, val, .. } => {
                    for v in [*base, *idx, *val] {
                        if used.insert(v) {
                            worklist.push(v);
                        }
                    }
                }
                Instr::StoreIndex2D {
                    base, r, c, val, ..
                } => {
                    for v in [*base, *r, *c, *val] {
                        if used.insert(v) {
                            worklist.push(v);
                        }
                    }
                }
            }
        }

        match &blk.term {
            Terminator::If { cond, .. } => {
                if used.insert(*cond) {
                    worklist.push(*cond);
                }
            }
            Terminator::Return(Some(v)) => {
                if used.insert(*v) {
                    worklist.push(*v);
                }
            }
            _ => {}
        }
    }

    while let Some(vid) = worklist.pop() {
        let val = &fn_ir.values[vid];
        match &val.kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                if used.insert(*lhs) {
                    worklist.push(*lhs);
                }
                if used.insert(*rhs) {
                    worklist.push(*rhs);
                }
            }
            ValueKind::Unary { rhs, .. } => {
                if used.insert(*rhs) {
                    worklist.push(*rhs);
                }
            }
            ValueKind::Call { args, .. } => {
                for a in args {
                    if used.insert(*a) {
                        worklist.push(*a);
                    }
                }
            }
            ValueKind::Phi { args } => {
                for (a, _) in args {
                    if used.insert(*a) {
                        worklist.push(*a);
                    }
                }
            }
            ValueKind::Index1D { base, idx, .. } => {
                if used.insert(*base) {
                    worklist.push(*base);
                }
                if used.insert(*idx) {
                    worklist.push(*idx);
                }
            }
            ValueKind::Index2D { base, r, c } => {
                if used.insert(*base) {
                    worklist.push(*base);
                }
                if used.insert(*r) {
                    worklist.push(*r);
                }
                if used.insert(*c) {
                    worklist.push(*c);
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                if used.insert(*base) {
                    worklist.push(*base);
                }
            }
            ValueKind::Range { start, end } => {
                if used.insert(*start) {
                    worklist.push(*start);
                }
                if used.insert(*end) {
                    worklist.push(*end);
                }
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
