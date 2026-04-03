use crate::typeck::lattice::{PrimTy, ShapeTy, TypeState};
use crate::typeck::term::TypeTerm;

mod foundation;
mod outputs;

pub(crate) use foundation::*;
pub(crate) use outputs::*;

mod state;
mod term;

pub(crate) use state::infer_builtin;
pub(crate) use term::infer_builtin_term;
