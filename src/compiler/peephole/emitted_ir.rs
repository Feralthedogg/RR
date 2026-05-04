use super::*;
use crate::compiler::r_peephole::scalar_reuse_rewrites::run_secondary_exact_local_scalar_bundle;
use std::sync::OnceLock;

#[path = "emitted_ir/model.rs"]
mod model;
pub(crate) use self::model::*;
#[path = "emitted_ir/cleanup.rs"]
mod cleanup;
pub(crate) use self::cleanup::*;
#[path = "emitted_ir/passthrough.rs"]
mod passthrough;
pub(crate) use self::passthrough::*;
#[path = "emitted_ir/helper_alias.rs"]
mod helper_alias;
pub(crate) use self::helper_alias::*;
#[path = "emitted_ir/wrapper_cleanup.rs"]
mod wrapper_cleanup;
pub(crate) use self::wrapper_cleanup::*;
#[path = "emitted_ir/exact_reuse.rs"]
mod exact_reuse;
pub(crate) use self::exact_reuse::*;
#[cfg(test)]
#[path = "emitted_ir/tests.rs"]
mod tests;
