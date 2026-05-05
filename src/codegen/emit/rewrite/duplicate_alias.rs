use super::*;
#[path = "duplicate_alias/duplicate_assignments.rs"]
mod duplicate_assignments;
pub(crate) use self::duplicate_assignments::*;
#[path = "duplicate_alias/structural_cleanup.rs"]
mod structural_cleanup;
pub(crate) use self::structural_cleanup::*;
#[path = "duplicate_alias/blank_cleanup.rs"]
mod blank_cleanup;
pub(crate) use self::blank_cleanup::*;
#[path = "duplicate_alias/repeat_tail.rs"]
mod repeat_tail;
pub(crate) use self::repeat_tail::*;
#[path = "duplicate_alias/temp_copy.rs"]
mod temp_copy;
pub(crate) use self::temp_copy::*;
#[path = "duplicate_alias/dead_scalar.rs"]
mod dead_scalar;
pub(crate) use self::dead_scalar::*;
