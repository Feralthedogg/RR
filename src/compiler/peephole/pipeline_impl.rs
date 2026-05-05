use super::*;
use crate::compiler::peephole::stage_catalog::{PeepholePassManager, PeepholeStageId};
use std::time::Instant;

#[path = "pipeline_impl/outcomes.rs"]
mod outcomes;
pub(crate) use self::outcomes::*;
#[path = "pipeline_impl/stages.rs"]
mod stages;
pub(crate) use self::stages::*;
#[path = "pipeline_impl/driver.rs"]
mod driver;
pub(crate) use self::driver::*;
