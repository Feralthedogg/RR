//! RR compiler library.
//!
//! RR 2.0 keeps the stable library-facing surface intentionally small:
//! `compiler`, `error`, `pkg`, `runtime`, and `Span`.
//!
//! Frontend lowering, MIR, code generation, syntax, and type-checking internals
//! are private implementation details. Add a narrow re-export through one of the
//! stable modules when external tooling needs a durable API.
//!
//! The `fuzz-internals` feature is intentionally reserved for the in-tree fuzz
//! harness. It keeps production API shape narrow while letting fuzz targets
//! exercise compiler stages directly.

// Pass catalogs and regression-only hooks are intentionally callable even when
// a given build profile does not route through every helper.
#![expect(dead_code, reason = "compiler pass catalog keeps optional hooks")]
#[cfg(feature = "fuzz-internals")]
pub mod codegen;
#[cfg(not(feature = "fuzz-internals"))]
mod codegen;
pub mod compiler;
mod diagnostic;
pub mod error;
#[cfg(feature = "fuzz-internals")]
pub mod hir;
#[cfg(not(feature = "fuzz-internals"))]
mod hir;
#[cfg(feature = "fuzz-internals")]
pub mod mir;
#[cfg(not(feature = "fuzz-internals"))]
mod mir;
pub mod pkg;
pub mod runtime;
#[cfg(feature = "fuzz-internals")]
pub mod syntax;
#[cfg(not(feature = "fuzz-internals"))]
mod syntax;
#[cfg(feature = "fuzz-internals")]
pub mod typeck;
#[cfg(not(feature = "fuzz-internals"))]
mod typeck;
#[cfg(feature = "fuzz-internals")]
pub mod utils;
#[cfg(not(feature = "fuzz-internals"))]
mod utils;

pub use utils::Span;
