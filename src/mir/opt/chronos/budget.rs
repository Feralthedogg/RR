#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::mir::opt) struct ChronosBudget {
    pub(in crate::mir::opt) max_iterations: usize,
}

impl ChronosBudget {
    pub(in crate::mir::opt) const fn fixed_point(max_iterations: usize) -> Self {
        Self { max_iterations }
    }
}
