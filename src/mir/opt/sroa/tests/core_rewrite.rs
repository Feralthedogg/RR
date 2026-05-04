use super::*;

#[path = "core_rewrite/field_get.rs"]
pub(crate) mod field_get;
#[path = "core_rewrite/field_set.rs"]
pub(crate) mod field_set;
#[path = "core_rewrite/phi.rs"]
pub(crate) mod phi;
#[path = "core_rewrite/rematerialize_alias.rs"]
pub(crate) mod rematerialize_alias;
#[path = "core_rewrite/safety_snapshots.rs"]
pub(crate) mod safety_snapshots;
