use crate::compiler::pipeline::line_contains_symbol;
use rustc_hash::{FxHashMap, FxHashSet};

use super::patterns;
use super::patterns::{
    assign_re, expr_idents, indexed_store_base_re, next_generated_cse_index, plain_ident_re,
    split_top_level_args,
};

#[path = "vector/helper_rewrites.rs"]
mod helper_rewrites;
pub(crate) use self::helper_rewrites::*;
#[path = "vector/exact_gather_temps.rs"]
mod exact_gather_temps;
pub(crate) use self::exact_gather_temps::*;
#[path = "vector/semantic_index_cse.rs"]
mod semantic_index_cse;
pub(crate) use self::semantic_index_cse::*;
