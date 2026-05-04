use super::*;
#[path = "return_rewrite/entrypoints.rs"]
mod entrypoints;
pub(crate) use self::entrypoints::*;
#[path = "return_rewrite/analysis.rs"]
mod analysis;
pub(crate) use self::analysis::*;
#[path = "return_rewrite/clone_value.rs"]
mod clone_value;
pub(crate) use self::clone_value::*;
#[path = "return_rewrite/apply.rs"]
mod apply;
pub(crate) use self::apply::*;
#[path = "return_rewrite/temps.rs"]
mod temps;
pub(crate) use self::temps::*;
