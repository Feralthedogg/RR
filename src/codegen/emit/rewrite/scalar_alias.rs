use super::*;
#[path = "scalar_alias/single_use_index.rs"]
mod single_use_index;
pub(crate) use self::single_use_index::*;
#[path = "scalar_alias/branch_helpers.rs"]
mod branch_helpers;
pub(crate) use self::branch_helpers::*;
#[path = "scalar_alias/branch_rebind.rs"]
mod branch_rebind;
pub(crate) use self::branch_rebind::*;
#[path = "scalar_alias/named_expr.rs"]
mod named_expr;
pub(crate) use self::named_expr::*;
#[path = "scalar_alias/index_alias.rs"]
mod index_alias;
pub(crate) use self::index_alias::*;
