#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ChronosPassOutcome {
    pub(in crate::mir::opt) changed_count: usize,
    pub(in crate::mir::opt) changed_passes: usize,
    pub(in crate::mir::opt) structural_progress: bool,
}

impl ChronosPassOutcome {
    pub(in crate::mir::opt) const fn unchanged() -> Self {
        Self {
            changed_count: 0,
            changed_passes: 0,
            structural_progress: false,
        }
    }

    pub(in crate::mir::opt) const fn changed(changed: bool) -> Self {
        Self {
            changed_count: changed as usize,
            changed_passes: changed as usize,
            structural_progress: false,
        }
    }

    pub(in crate::mir::opt) const fn counted(changed_count: usize) -> Self {
        Self {
            changed_count,
            changed_passes: (changed_count > 0) as usize,
            structural_progress: changed_count > 0,
        }
    }

    pub(in crate::mir::opt) const fn combined(changed_count: usize, changed_passes: usize) -> Self {
        Self {
            changed_count,
            changed_passes,
            structural_progress: changed_count > 0,
        }
    }

    pub(in crate::mir::opt) const fn is_changed(self) -> bool {
        self.changed_count > 0
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub(in crate::mir::opt) struct ChronosFixedPointOutcome {
    pub(in crate::mir::opt) changed: bool,
    pub(in crate::mir::opt) iterations: usize,
}
