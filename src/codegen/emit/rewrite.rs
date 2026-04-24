use super::*;
use std::sync::OnceLock;

include!("rewrite/poly_index.rs");
include!("rewrite/literal_calls.rs");
include!("rewrite/raw_text.rs");
include!("rewrite/scalar_alias.rs");
include!("rewrite/loop_alias.rs");
include!("rewrite/duplicate_alias.rs");
include!("rewrite/temp_seed.rs");
include!("rewrite/final_cleanup.rs");
include!("rewrite/tests.rs");
