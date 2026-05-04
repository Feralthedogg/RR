use super::*;
use std::sync::OnceLock;

#[path = "rewrite/poly_index.rs"]
mod poly_index;
pub(crate) use self::poly_index::*;
#[path = "rewrite/literal_calls.rs"]
mod literal_calls;
pub(crate) use self::literal_calls::*;
#[path = "rewrite/raw_text.rs"]
mod raw_text;
pub(crate) use self::raw_text::*;
#[path = "rewrite/scalar_alias.rs"]
mod scalar_alias;
pub(crate) use self::scalar_alias::*;
#[path = "rewrite/loop_alias.rs"]
mod loop_alias;
pub(crate) use self::loop_alias::*;
#[path = "rewrite/duplicate_alias.rs"]
mod duplicate_alias;
pub(crate) use self::duplicate_alias::*;
#[path = "rewrite/temp_seed.rs"]
mod temp_seed;
pub(crate) use self::temp_seed::*;
#[path = "rewrite/final_cleanup.rs"]
mod final_cleanup;
pub(crate) use self::final_cleanup::*;
#[cfg(test)]
#[path = "rewrite/tests.rs"]
mod tests;
