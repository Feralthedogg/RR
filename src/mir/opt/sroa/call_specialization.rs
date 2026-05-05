use super::*;
#[path = "call_specialization/entrypoints.rs"]
mod entrypoints;
pub(crate) use self::entrypoints::*;
#[path = "call_specialization/collectors.rs"]
mod collectors;
pub(crate) use self::collectors::*;
#[path = "call_specialization/return_rewrite.rs"]
mod return_rewrite;
pub(crate) use self::return_rewrite::*;
#[path = "call_specialization/purity.rs"]
mod purity;
pub(crate) use self::purity::*;
#[path = "call_specialization/arg_rewrite.rs"]
mod arg_rewrite;
pub(crate) use self::arg_rewrite::*;
#[path = "call_specialization/naming.rs"]
mod naming;
pub(crate) use self::naming::*;
