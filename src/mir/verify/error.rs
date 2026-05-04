use super::*;
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
                write!(
                    f,
                    "Non-Phi value {} participates in a cyclic dependency",
                    value
                )
            }
            VerifyError::InvalidBodyHead { block } => {
                write!(
                    f,
                    "Function body_head {} is not reachable from entry",
                    block
                )
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
                write!(
                    f,
                    "body_head block {} must not terminate as unreachable",
                    block
                )
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
