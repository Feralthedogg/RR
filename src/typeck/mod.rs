pub mod builtin_sigs;
pub mod constraints;
pub mod hm;
pub mod lattice;
pub(crate) mod sigs;
pub mod solver;
pub mod term;
pub mod trait_solver;

pub use lattice::{LenSym, NaTy, PrimTy, ShapeTy, TypeState};
pub use solver::{NativeBackend, TypeConfig, TypeMode};
pub use term::TypeTerm;
