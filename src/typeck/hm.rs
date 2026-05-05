use crate::hir::def::{HirBlock, HirExpr, HirFn, HirStmt, LocalId, SymbolId};
use rustc_hash::FxHashMap;

#[path = "hm/core.rs"]
mod core;
pub(crate) use self::core::*;
#[path = "hm/hints.rs"]
mod hints;
pub(crate) use self::hints::*;
