//! Routing layer for the major compile phases extracted from `pipeline.rs`.
//!
//! Source loading and emission-heavy helpers live in `source_emit.rs`, while
//! Tachyon execution and runtime injection live in `tachyon_runtime.rs`.
use super::*;

#[path = "phases/source_emit.rs"]
pub(crate) mod source_emit;
pub(crate) use source_emit::*;

#[path = "phases/tachyon_runtime.rs"]
pub(crate) mod tachyon_runtime;
pub(crate) use tachyon_runtime::*;
