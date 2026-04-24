use super::*;
use crate::compiler::r_peephole::scalar_reuse_rewrites::run_secondary_exact_local_scalar_bundle;
use std::sync::OnceLock;

include!("emitted_ir/model.rs");
include!("emitted_ir/cleanup.rs");
include!("emitted_ir/passthrough.rs");
include!("emitted_ir/helper_alias.rs");
include!("emitted_ir/wrapper_cleanup.rs");
include!("emitted_ir/exact_reuse.rs");
include!("emitted_ir/tests.rs");
