#![allow(dead_code)]

#[path = "expr_reuse/forward.rs"]
mod forward;
#[path = "expr_reuse/temp_tail.rs"]
mod temp_tail;

pub(super) use self::forward::*;
pub(super) use self::temp_tail::*;
