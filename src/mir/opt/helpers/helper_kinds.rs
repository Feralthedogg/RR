#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PeriodicIndexHelperKind {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TrivialMinMaxHelperKind {
    Min,
    Max,
}
