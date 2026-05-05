use super::*;

#[path = "analysis_vectorization/loop_shape.rs"]
mod loop_shape;
pub(crate) use self::loop_shape::*;
#[path = "analysis_vectorization/call_safety.rs"]
mod call_safety;
pub(crate) use self::call_safety::*;
#[path = "analysis_vectorization/indexing.rs"]
mod indexing;
pub(crate) use self::indexing::*;
#[path = "analysis_vectorization/expr_vectorizable.rs"]
mod expr_vectorizable;
pub(crate) use self::expr_vectorizable::*;
#[path = "analysis_vectorization/lengths.rs"]
mod lengths;
pub(crate) use self::lengths::*;
#[path = "analysis_vectorization/induction.rs"]
mod induction;
pub(crate) use self::induction::*;
#[path = "analysis_vectorization/hoist_alias.rs"]
mod hoist_alias;
pub(crate) use self::hoist_alias::*;
#[cfg(test)]
#[path = "analysis_vectorization/tests.rs"]
mod tests;
