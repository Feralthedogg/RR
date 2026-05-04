pub use crate::codegen::backend::state::MapEntry;
pub use crate::codegen::backend::state::RBackend;
use crate::mir::def::{BinOp, FnIR, Instr, IntrinsicOp, Lit, UnaryOp, Value, ValueKind};
use crate::mir::structurizer::Structurizer;
use crate::typeck::TypeTerm;
use crate::utils::Span;
use regex::Captures;
use rustc_hash::FxHashSet;

#[path = "mir_emit/emitter_state.rs"]
mod emitter_state;
pub(crate) use self::emitter_state::*;
#[path = "mir_emit/function_emit.rs"]
mod function_emit;
#[path = "mir_emit/post_emit_rewrite_hooks.rs"]
mod post_emit_rewrite_hooks;
#[path = "mir_emit/test_module.rs"]
mod test_module;
