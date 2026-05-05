use super::*;
use crate::syntax::ast::Lit;
use rustc_hash::{FxHashMap, FxHashSet};

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Lattice {
    Top, // Undefined
    Constant(Lit),
    Bottom, // Overdefined
}

pub(crate) type ExecutableEdge = (BlockId, BlockId);
pub(crate) type SolveResult = (
    FxHashMap<ValueId, Lattice>,
    FxHashSet<BlockId>,
    FxHashSet<ExecutableEdge>,
);

pub struct MirSCCP;

impl Default for MirSCCP {
    fn default() -> Self {
        Self::new()
    }
}
