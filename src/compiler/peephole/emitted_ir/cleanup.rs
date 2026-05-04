use super::*;
#[path = "cleanup/terminal_repeat.rs"]
mod terminal_repeat;
pub(crate) use self::terminal_repeat::*;
#[path = "cleanup/basic.rs"]
mod basic;
pub(crate) use self::basic::*;
#[path = "cleanup/match_else.rs"]
mod match_else;
pub(crate) use self::match_else::*;
#[path = "cleanup/unreachable_sym.rs"]
mod unreachable_sym;
pub(crate) use self::unreachable_sym::*;
#[path = "cleanup/finalize_bundles.rs"]
mod finalize_bundles;
pub(crate) use self::finalize_bundles::*;
