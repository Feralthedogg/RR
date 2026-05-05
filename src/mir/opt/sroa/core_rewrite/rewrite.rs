use super::*;
#[path = "rewrite/value_replacements.rs"]
mod value_replacements;
pub(crate) use self::value_replacements::*;
#[path = "rewrite/rematerialize.rs"]
mod rematerialize;
pub(crate) use self::rematerialize::*;
#[path = "rewrite/store_index.rs"]
mod store_index;
pub(crate) use self::store_index::*;
#[path = "rewrite/liveness.rs"]
mod liveness;
pub(crate) use self::liveness::*;
#[path = "rewrite/value_refs.rs"]
mod value_refs;
pub(crate) use self::value_refs::*;
