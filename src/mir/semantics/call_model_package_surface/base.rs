#[path = "base/core.rs"]
pub(crate) mod core;
#[path = "base/extended.rs"]
pub(crate) mod extended;

pub(crate) fn contains(name: &str) -> bool {
    core::contains(name) || extended::contains(name)
}
