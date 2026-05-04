//! RR compiler library.
//!
//! RR 2.0 keeps the stable library-facing surface intentionally small:
//! `compiler`, `error`, `pkg`, `runtime`, and `Span`.
//!
//! Frontend lowering, MIR, code generation, syntax, and type-checking internals
//! are private implementation details. Add a narrow re-export through one of the
//! stable modules when external tooling needs a durable API.

// Pass catalogs and regression-only hooks are intentionally callable even when
// a given build profile does not route through every helper.
#![expect(dead_code, reason = "compiler pass catalog keeps optional hooks")]
mod codegen;
pub mod compiler;
mod diagnostic;
pub mod error;
mod hir;
mod mir;
pub mod pkg;
pub mod runtime;
mod syntax;
mod typeck;
mod utils;

pub use utils::Span;
