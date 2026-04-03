#[path = "base/core.rs"]
mod core;
#[path = "base/extended.rs"]
mod extended;

pub(crate) fn contains(name: &str) -> bool {
    core::contains(name) || extended::contains(name)
}
