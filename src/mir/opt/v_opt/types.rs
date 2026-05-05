use crate::mir::*;
use crate::syntax::ast::BinOp;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis3D {
    Dim1,
    Dim2,
    Dim3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum MemoryStrideClass {
    Contiguous,
    Strided,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum VectorAccessOperand3D {
    Scalar(ValueId),
    Vector(ValueId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct VectorAccessPattern3D {
    pub i: VectorAccessOperand3D,
    pub j: VectorAccessOperand3D,
    pub k: VectorAccessOperand3D,
}

impl VectorAccessPattern3D {
    pub(super) fn vector_count(&self) -> usize {
        [self.i, self.j, self.k]
            .into_iter()
            .filter(|operand| matches!(operand, VectorAccessOperand3D::Vector(_)))
            .count()
    }
}

#[derive(Debug, Clone)]
pub enum VectorPlan {
    Reduce {
        kind: ReduceKind,
        acc_phi: ValueId,
        vec_expr: ValueId,
        iv_phi: ValueId,
    },
    ReduceCond {
        kind: ReduceKind,
        acc_phi: ValueId,
        cond: ValueId,
        then_val: ValueId,
        else_val: ValueId,
        iv_phi: ValueId,
    },
    MultiReduceCond {
        cond: ValueId,
        entries: Vec<ReduceCondEntry>,
        iv_phi: ValueId,
    },
    Reduce2DRowSum {
        acc_phi: ValueId,
        base: ValueId,
        row: ValueId,
        start: ValueId,
        end: ValueId,
    },
    Reduce2DColSum {
        acc_phi: ValueId,
        base: ValueId,
        col: ValueId,
        start: ValueId,
        end: ValueId,
    },
    Reduce3D {
        kind: ReduceKind,
        acc_phi: ValueId,
        base: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
    },
    Map {
        dest: ValueId,
        src: ValueId,
        op: BinOp,
        other: ValueId,
        shadow_vars: Vec<VarId>,
    },
    CondMap {
        dest: ValueId,
        cond: ValueId,
        then_val: ValueId,
        else_val: ValueId,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
        whole_dest: bool,
        shadow_vars: Vec<VarId>,
    },
    CondMap3D {
        dest: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        cond_lhs: ValueId,
        cond_rhs: ValueId,
        cmp_op: BinOp,
        then_src: ValueId,
        else_src: ValueId,
        start: ValueId,
        end: ValueId,
    },
    CondMap3DGeneral {
        dest: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        cond_lhs: ValueId,
        cond_rhs: ValueId,
        cmp_op: BinOp,
        then_val: ValueId,
        else_val: ValueId,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
    },
    RecurrenceAddConst {
        base: ValueId,
        start: ValueId,
        end: ValueId,
        delta: ValueId,
        negate_delta: bool,
    },
    RecurrenceAddConst3D {
        base: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
        delta: ValueId,
        negate_delta: bool,
    },
    ShiftedMap {
        dest: ValueId,
        src: ValueId,
        start: ValueId,
        end: ValueId,
        offset: i64,
    },
    ShiftedMap3D {
        dest: ValueId,
        src: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
        offset: i64,
    },
    CallMap {
        dest: ValueId,
        callee: String,
        args: Vec<CallMapArg>,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
        whole_dest: bool,
        shadow_vars: Vec<VarId>,
    },
    CallMap3D {
        dest: ValueId,
        callee: String,
        args: Vec<CallMapArg>,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
    },
    CallMap3DGeneral {
        dest: ValueId,
        callee: String,
        args: Vec<CallMapArg>,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
    },
    CubeSliceExprMap {
        dest: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
        face: ValueId,
        row: ValueId,
        size: ValueId,
        ctx: Option<ValueId>,
        start: ValueId,
        end: ValueId,
        shadow_vars: Vec<VarId>,
    },
    ExprMap {
        dest: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
        whole_dest: bool,
        shadow_vars: Vec<VarId>,
    },
    ExprMap3D {
        dest: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
    },
    MultiExprMap3D {
        entries: Vec<ExprMapEntry3D>,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
    },
    MultiExprMap {
        entries: Vec<ExprMapEntry>,
        iv_phi: ValueId,
        start: ValueId,
        end: ValueId,
    },
    ScatterExprMap {
        dest: ValueId,
        idx: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
    },
    ScatterExprMap3D {
        dest: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        idx: ValueId,
        expr: ValueId,
        iv_phi: ValueId,
    },
    ScatterExprMap3DGeneral {
        dest: ValueId,
        i: VectorAccessOperand3D,
        j: VectorAccessOperand3D,
        k: VectorAccessOperand3D,
        expr: ValueId,
        iv_phi: ValueId,
    },
    Map2DRow {
        dest: ValueId,
        row: ValueId,
        start: ValueId,
        end: ValueId,
        lhs_src: ValueId,
        rhs_src: ValueId,
        op: BinOp,
    },
    Map2DCol {
        dest: ValueId,
        col: ValueId,
        start: ValueId,
        end: ValueId,
        lhs_src: ValueId,
        rhs_src: ValueId,
        op: BinOp,
    },
    Map3D {
        dest: ValueId,
        axis: Axis3D,
        fixed_a: ValueId,
        fixed_b: ValueId,
        start: ValueId,
        end: ValueId,
        lhs_src: ValueId,
        rhs_src: ValueId,
        op: BinOp,
    },
}

#[derive(Debug, Clone, Copy)]
pub struct CallMapArg {
    pub(super) value: ValueId,
    pub(super) vectorized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReduceCondEntry {
    pub(super) kind: ReduceKind,
    pub(super) acc_phi: ValueId,
    pub(super) then_val: ValueId,
    pub(super) else_val: ValueId,
}

#[derive(Debug, Clone)]
pub struct ExprMapEntry {
    pub(super) dest: ValueId,
    pub(super) expr: ValueId,
    pub(super) whole_dest: bool,
    pub(super) shadow_vars: Vec<VarId>,
}

#[derive(Debug, Clone)]
pub struct ExprMapEntry3D {
    pub(super) dest: ValueId,
    pub(super) expr: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) shadow_vars: Vec<VarId>,
}

#[derive(Debug, Clone)]
pub(super) struct CallMap3DMatchCandidate {
    pub(super) dest: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) callee: String,
    pub(super) args: Vec<CallMapArg>,
    pub(super) generalized: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CallMapLoweringMode {
    DirectVector,
    RuntimeAuto { helper_cost: u32 },
}

#[derive(Debug, Clone, Copy)]
pub(super) struct BlockStore1D {
    pub(super) base: ValueId,
    pub(super) idx: ValueId,
    pub(super) val: ValueId,
    pub(super) is_vector: bool,
}

#[derive(Debug, Clone, Copy)]
pub(super) enum BlockStore1DMatch {
    None,
    One(BlockStore1D),
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct BlockStore3D {
    pub(super) base: ValueId,
    pub(super) i: ValueId,
    pub(super) j: ValueId,
    pub(super) k: ValueId,
    pub(super) val: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum BlockStore3DMatch {
    None,
    One(BlockStore3D),
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReduceKind {
    Sum,
    Prod,
    Min,
    Max,
}

#[repr(usize)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProofFallbackReason {
    Disabled,
    NotYetImplemented,
    StorelessConditionalLoop,
    StorelessReductionLoop,
    StorelessStateLoop,
    StorelessPlainLoop,
    StorefulStateLoop,
    MissingInductionVar,
    UnsupportedLoopShape,
    MissingStore,
    MultipleStores,
    NonCanonicalStore,
    UnresolvableDestination,
    NotWholeDestination,
    NotSimpleMap,
    NotSimpleCondMap,
    NotSimpleReduction,
    NotSimpleExprMap,
    NotSimpleCallMap,
    UnsupportedCondition,
    BranchLeavesLoopBody,
    BranchStoreShape,
    MismatchedBranchDestinations,
    UnsupportedConditionalValues,
    ReductionExtraState,
    UnsupportedReductionExpr,
    UnsupportedMapOperands,
    UnsupportedCallMapArgs,
    ShadowState,
}

impl ProofFallbackReason {
    pub(super) const ALL: [Self; 29] = [
        Self::Disabled,
        Self::NotYetImplemented,
        Self::StorelessConditionalLoop,
        Self::StorelessReductionLoop,
        Self::StorelessStateLoop,
        Self::StorelessPlainLoop,
        Self::StorefulStateLoop,
        Self::MissingInductionVar,
        Self::UnsupportedLoopShape,
        Self::MissingStore,
        Self::MultipleStores,
        Self::NonCanonicalStore,
        Self::UnresolvableDestination,
        Self::NotWholeDestination,
        Self::NotSimpleMap,
        Self::NotSimpleCondMap,
        Self::NotSimpleReduction,
        Self::NotSimpleExprMap,
        Self::NotSimpleCallMap,
        Self::UnsupportedCondition,
        Self::BranchLeavesLoopBody,
        Self::BranchStoreShape,
        Self::MismatchedBranchDestinations,
        Self::UnsupportedConditionalValues,
        Self::ReductionExtraState,
        Self::UnsupportedReductionExpr,
        Self::UnsupportedMapOperands,
        Self::UnsupportedCallMapArgs,
        Self::ShadowState,
    ];
    pub(super) const COUNT: usize = Self::ALL.len();

    pub(super) const fn index(self) -> usize {
        self as usize
    }

    pub(super) const fn label(self) -> &'static str {
        match self {
            Self::Disabled => "disabled",
            Self::NotYetImplemented => "not-yet-implemented",
            Self::StorelessConditionalLoop => "storeless-conditional-loop",
            Self::StorelessReductionLoop => "storeless-reduction-loop",
            Self::StorelessStateLoop => "storeless-state-loop",
            Self::StorelessPlainLoop => "storeless-plain-loop",
            Self::StorefulStateLoop => "storeful-state-loop",
            Self::MissingInductionVar => "missing-induction-var",
            Self::UnsupportedLoopShape => "unsupported-loop-shape",
            Self::MissingStore => "missing-store",
            Self::MultipleStores => "multiple-stores",
            Self::NonCanonicalStore => "non-canonical-store",
            Self::UnresolvableDestination => "unresolvable-destination",
            Self::NotWholeDestination => "not-whole-destination",
            Self::NotSimpleMap => "not-simple-map",
            Self::NotSimpleCondMap => "not-simple-cond-map",
            Self::NotSimpleReduction => "not-simple-reduction",
            Self::NotSimpleExprMap => "not-simple-expr-map",
            Self::NotSimpleCallMap => "not-simple-call-map",
            Self::UnsupportedCondition => "unsupported-condition",
            Self::BranchLeavesLoopBody => "branch-leaves-loop-body",
            Self::BranchStoreShape => "branch-store-shape",
            Self::MismatchedBranchDestinations => "mismatched-branch-destinations",
            Self::UnsupportedConditionalValues => "unsupported-conditional-values",
            Self::ReductionExtraState => "reduction-extra-state",
            Self::UnsupportedReductionExpr => "unsupported-reduction-expr",
            Self::UnsupportedMapOperands => "unsupported-map-operands",
            Self::UnsupportedCallMapArgs => "unsupported-call-map-args",
            Self::ShadowState => "shadow-state",
        }
    }
}

pub(crate) const PROOF_FALLBACK_REASON_COUNT: usize = ProofFallbackReason::COUNT;

pub(crate) fn format_proof_fallback_counts(
    counts: &[usize; PROOF_FALLBACK_REASON_COUNT],
) -> String {
    let mut entries = ProofFallbackReason::ALL
        .iter()
        .filter_map(|reason| {
            let count = counts[reason.index()];
            (count > 0).then_some((reason.label(), count))
        })
        .collect::<Vec<_>>();
    entries.sort_by(|(lhs_label, lhs_count), (rhs_label, rhs_count)| {
        rhs_count
            .cmp(lhs_count)
            .then_with(|| lhs_label.cmp(rhs_label))
    });
    entries
        .into_iter()
        .take(6)
        .map(|(label, count)| format!("{label} {count}"))
        .collect::<Vec<_>>()
        .join(" | ")
}

#[derive(Debug, Clone)]
pub(crate) struct CertifiedPlan {
    pub(crate) plan: VectorPlan,
}

#[derive(Debug, Clone)]
pub(crate) enum ProofOutcome {
    Certified(CertifiedPlan),
    NotApplicable { reason: ProofFallbackReason },
    FallbackToPattern { reason: ProofFallbackReason },
}

#[derive(Clone, Copy)]
pub(crate) struct VectorApplySite {
    pub(crate) preheader: BlockId,
    pub(crate) exit_bb: BlockId,
}

#[derive(Clone, Copy)]
pub(super) struct VectorLoopRange {
    pub(super) start: ValueId,
    pub(super) end: ValueId,
}

#[derive(Debug)]
pub(crate) struct PreparedVectorAssignment {
    pub(crate) dest_var: VarId,
    pub(crate) out_val: ValueId,
    pub(crate) shadow_vars: Vec<VarId>,
    pub(crate) shadow_idx: Option<ValueId>,
}

#[derive(Clone, Copy)]
pub(super) struct Reduce2DApplyPlan {
    pub(super) acc_phi: ValueId,
    pub(super) base: ValueId,
    pub(super) axis: ValueId,
    pub(super) range: VectorLoopRange,
}

#[derive(Clone, Copy)]
pub(super) struct Reduce3DApplyPlan {
    pub(super) kind: ReduceKind,
    pub(super) acc_phi: ValueId,
    pub(super) base: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) range: VectorLoopRange,
}

#[derive(Clone, Copy)]
pub(super) struct RecurrenceAddConstApplyPlan {
    pub(super) base: ValueId,
    pub(super) range: VectorLoopRange,
    pub(super) delta: ValueId,
    pub(super) negate_delta: bool,
}

#[derive(Clone, Copy)]
pub(super) struct RecurrenceAddConst3DApplyPlan {
    pub(super) base: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) range: VectorLoopRange,
    pub(super) delta: ValueId,
    pub(super) negate_delta: bool,
}

#[derive(Clone, Copy)]
pub(super) struct ShiftedMapApplyPlan {
    pub(super) dest: ValueId,
    pub(super) src: ValueId,
    pub(super) range: VectorLoopRange,
    pub(super) offset: i64,
}

#[derive(Clone, Copy)]
pub(super) struct ShiftedMap3DApplyPlan {
    pub(super) dest: ValueId,
    pub(super) src: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) range: VectorLoopRange,
    pub(super) offset: i64,
}

#[derive(Clone, Copy)]
pub(super) struct Map2DApplyPlan {
    pub(super) dest: ValueId,
    pub(super) axis: ValueId,
    pub(super) range: VectorLoopRange,
    pub(super) lhs_src: ValueId,
    pub(super) rhs_src: ValueId,
    pub(super) op: BinOp,
}

#[derive(Clone, Copy)]
pub(super) struct Map3DApplyPlan {
    pub(super) dest: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) range: VectorLoopRange,
    pub(super) lhs_src: ValueId,
    pub(super) rhs_src: ValueId,
    pub(super) op: BinOp,
}

pub(super) struct CallMap3DApplyPlan {
    pub(super) dest: ValueId,
    pub(super) callee: String,
    pub(super) args: Vec<CallMapArg>,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) range: VectorLoopRange,
}

pub(super) struct CallMap3DGeneralApplyPlan {
    pub(super) dest: ValueId,
    pub(super) callee: String,
    pub(super) args: Vec<CallMapArg>,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) iv_phi: ValueId,
    pub(super) range: VectorLoopRange,
}

#[derive(Clone, Copy)]
pub(super) struct ExprMap3DApplyPlan {
    pub(super) dest: ValueId,
    pub(super) expr: ValueId,
    pub(super) iv_phi: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) range: VectorLoopRange,
}

#[derive(Clone, Copy)]
pub(super) struct ScatterExprMap3DApplyPlan {
    pub(super) dest: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) idx: ValueId,
    pub(super) expr: ValueId,
    pub(super) iv_phi: ValueId,
}

#[derive(Clone, Copy)]
pub(super) struct ScatterExprMap3DGeneralApplyPlan {
    pub(super) dest: ValueId,
    pub(super) i: VectorAccessOperand3D,
    pub(super) j: VectorAccessOperand3D,
    pub(super) k: VectorAccessOperand3D,
    pub(super) expr: ValueId,
    pub(super) iv_phi: ValueId,
}

#[derive(Clone, Copy)]
pub(super) struct CondMap3DApplyPlan {
    pub(super) dest: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) cond_lhs: ValueId,
    pub(super) cond_rhs: ValueId,
    pub(super) cmp_op: BinOp,
    pub(super) then_src: ValueId,
    pub(super) else_src: ValueId,
    pub(super) range: VectorLoopRange,
}

#[derive(Clone, Copy)]
pub(super) struct CondMap3DGeneralApplyPlan {
    pub(super) dest: ValueId,
    pub(super) axis: Axis3D,
    pub(super) fixed_a: ValueId,
    pub(super) fixed_b: ValueId,
    pub(super) cond_lhs: ValueId,
    pub(super) cond_rhs: ValueId,
    pub(super) cmp_op: BinOp,
    pub(super) then_val: ValueId,
    pub(super) else_val: ValueId,
    pub(super) iv_phi: ValueId,
    pub(super) range: VectorLoopRange,
}
