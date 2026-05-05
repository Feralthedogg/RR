pub(crate) use super::*;
pub(crate) use crate::mir::opt::poly::access::{AccessKind, AccessRelation, MemRef, MemoryLayout};
pub(crate) use crate::mir::opt::poly::affine::{
    AffineConstraint, AffineConstraintKind, AffineExpr, AffineSymbol, PresburgerSet,
};
pub(crate) use crate::mir::opt::poly::dependence_backend::{
    DependenceRelation, DependenceResult, DependenceState, DependenceSummary,
};
pub(crate) use crate::mir::opt::poly::{LoopDimension, PolyStmt, PolyStmtKind, ScopRegion};

pub(crate) fn loop_iv(name: &str) -> AffineExpr {
    AffineExpr::symbol(AffineSymbol::LoopIv(name.to_string()))
}

#[path = "core_cases.rs"]
mod core_cases;
#[path = "extended_cases.rs"]
mod extended_cases;
