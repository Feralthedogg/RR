use super::*;
#[path = "wrapper_cleanup/scalar_clamp.rs"]
mod scalar_clamp;
pub(crate) use self::scalar_clamp::*;
#[path = "wrapper_cleanup/dot_product.rs"]
mod dot_product;
pub(crate) use self::dot_product::*;
#[path = "wrapper_cleanup/singleton_slice.rs"]
mod singleton_slice;
pub(crate) use self::singleton_slice::*;
#[path = "wrapper_cleanup/simple_expr_bundles.rs"]
mod simple_expr_bundles;
pub(crate) use self::simple_expr_bundles::*;
#[path = "wrapper_cleanup/secondary_bundles.rs"]
mod secondary_bundles;
pub(crate) use self::secondary_bundles::*;
#[path = "wrapper_cleanup/branch_tail.rs"]
mod branch_tail;
pub(crate) use self::branch_tail::*;
#[path = "wrapper_cleanup/copy_vec.rs"]
mod copy_vec;
pub(crate) use self::copy_vec::*;
#[path = "wrapper_cleanup/sym_unreachable.rs"]
mod sym_unreachable;
pub(crate) use self::sym_unreachable::*;
#[path = "wrapper_cleanup/shared.rs"]
mod shared;
pub(crate) use self::shared::*;
#[path = "wrapper_tail_cleanup.rs"]
mod wrapper_tail_cleanup;
pub(crate) use self::wrapper_tail_cleanup::*;
