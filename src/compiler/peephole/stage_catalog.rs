#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PeepholeStageId {
    LinearScan,
    PrimaryFlow,
    PrimaryInline,
    PrimaryReuse,
    PrimaryLoopCleanup,
    SecondaryInline,
    SecondaryExact,
    SecondaryHelperCleanup,
    SecondaryRecordSroa,
    SecondaryFinalizeCleanup,
    Finalize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PeepholeStageMode {
    Always,
    StandardOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PeepholeStageSpec {
    pub(crate) id: PeepholeStageId,
    pub(crate) order: u8,
    pub(crate) mode: PeepholeStageMode,
    pub(crate) profile_field: &'static str,
    pub(crate) proof_key: &'static str,
}

#[derive(Debug)]
pub(crate) struct PeepholeStageRunner {
    spec: &'static PeepholeStageSpec,
    started: std::time::Instant,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct PeepholePassManager {
    standard_mode: bool,
}

pub(crate) const PEEPHOLE_STAGE_CATALOG: &[PeepholeStageSpec] = &[
    peephole_stage(
        PeepholeStageId::LinearScan,
        0,
        PeepholeStageMode::Always,
        "linear_scan_elapsed_ns",
        "PeepholePipelineSoundness.linear_scan",
    ),
    peephole_stage(
        PeepholeStageId::PrimaryFlow,
        1,
        PeepholeStageMode::Always,
        "primary_flow_elapsed_ns",
        "PeepholePipelineSoundness.primary_flow",
    ),
    peephole_stage(
        PeepholeStageId::PrimaryInline,
        2,
        PeepholeStageMode::Always,
        "primary_inline_elapsed_ns",
        "PeepholePipelineSoundness.primary_inline",
    ),
    peephole_stage(
        PeepholeStageId::PrimaryReuse,
        3,
        PeepholeStageMode::Always,
        "primary_reuse_elapsed_ns",
        "PeepholePipelineSoundness.primary_reuse",
    ),
    peephole_stage(
        PeepholeStageId::PrimaryLoopCleanup,
        4,
        PeepholeStageMode::Always,
        "primary_loop_cleanup_elapsed_ns",
        "PeepholePipelineSoundness.primary_loop_cleanup",
    ),
    peephole_stage(
        PeepholeStageId::SecondaryInline,
        5,
        PeepholeStageMode::StandardOnly,
        "secondary_inline_elapsed_ns",
        "PeepholePipelineSoundness.secondary_inline",
    ),
    peephole_stage(
        PeepholeStageId::SecondaryExact,
        6,
        PeepholeStageMode::StandardOnly,
        "secondary_exact_elapsed_ns",
        "PeepholePipelineSoundness.secondary_exact",
    ),
    peephole_stage(
        PeepholeStageId::SecondaryHelperCleanup,
        7,
        PeepholeStageMode::StandardOnly,
        "secondary_helper_cleanup_elapsed_ns",
        "PeepholePipelineSoundness.secondary_helper_cleanup",
    ),
    peephole_stage(
        PeepholeStageId::SecondaryRecordSroa,
        8,
        PeepholeStageMode::StandardOnly,
        "secondary_record_sroa_elapsed_ns",
        "PeepholePipelineSoundness.secondary_record_sroa",
    ),
    peephole_stage(
        PeepholeStageId::SecondaryFinalizeCleanup,
        9,
        PeepholeStageMode::StandardOnly,
        "secondary_finalize_cleanup_elapsed_ns",
        "PeepholePipelineSoundness.secondary_finalize_cleanup",
    ),
    peephole_stage(
        PeepholeStageId::Finalize,
        10,
        PeepholeStageMode::Always,
        "finalize_elapsed_ns",
        "PeepholePipelineSoundness.finalize",
    ),
];

const fn peephole_stage(
    id: PeepholeStageId,
    order: u8,
    mode: PeepholeStageMode,
    profile_field: &'static str,
    proof_key: &'static str,
) -> PeepholeStageSpec {
    PeepholeStageSpec {
        id,
        order,
        mode,
        profile_field,
        proof_key,
    }
}

pub(crate) fn peephole_stage_catalog_is_well_formed() -> bool {
    let mut previous_order = None;
    let mut saw_finalize = false;
    for spec in PEEPHOLE_STAGE_CATALOG {
        let _metadata = (spec.mode, spec.profile_field, spec.proof_key);
        if previous_order.is_some_and(|previous| previous >= spec.order) {
            return false;
        }
        previous_order = Some(spec.order);
        saw_finalize |= matches!(spec.id, PeepholeStageId::Finalize);
    }
    saw_finalize
}

pub(crate) fn peephole_stage_spec(id: PeepholeStageId) -> &'static PeepholeStageSpec {
    match id {
        PeepholeStageId::LinearScan => &PEEPHOLE_STAGE_CATALOG[0],
        PeepholeStageId::PrimaryFlow => &PEEPHOLE_STAGE_CATALOG[1],
        PeepholeStageId::PrimaryInline => &PEEPHOLE_STAGE_CATALOG[2],
        PeepholeStageId::PrimaryReuse => &PEEPHOLE_STAGE_CATALOG[3],
        PeepholeStageId::PrimaryLoopCleanup => &PEEPHOLE_STAGE_CATALOG[4],
        PeepholeStageId::SecondaryInline => &PEEPHOLE_STAGE_CATALOG[5],
        PeepholeStageId::SecondaryExact => &PEEPHOLE_STAGE_CATALOG[6],
        PeepholeStageId::SecondaryHelperCleanup => &PEEPHOLE_STAGE_CATALOG[7],
        PeepholeStageId::SecondaryRecordSroa => &PEEPHOLE_STAGE_CATALOG[8],
        PeepholeStageId::SecondaryFinalizeCleanup => &PEEPHOLE_STAGE_CATALOG[9],
        PeepholeStageId::Finalize => &PEEPHOLE_STAGE_CATALOG[10],
    }
}

impl PeepholeStageRunner {
    fn start_spec(spec: &'static PeepholeStageSpec) -> Self {
        Self {
            spec,
            started: std::time::Instant::now(),
        }
    }

    pub(crate) fn finish(self) -> u128 {
        let _stage_metadata = (
            self.spec.id,
            self.spec.order,
            self.spec.mode,
            self.spec.profile_field,
            self.spec.proof_key,
        );
        self.started.elapsed().as_nanos()
    }
}

impl PeepholePassManager {
    pub(crate) const fn for_fast_dev(fast_dev: bool) -> Self {
        Self {
            standard_mode: !fast_dev,
        }
    }

    pub(crate) fn stage_enabled(&self, spec: &PeepholeStageSpec) -> bool {
        matches!(spec.mode, PeepholeStageMode::Always) || self.standard_mode
    }

    pub(crate) fn validate_sequence(&self, stages: &[PeepholeStageId]) -> bool {
        let mut previous_order = None;
        for id in stages {
            let spec = peephole_stage_spec(*id);
            if !self.stage_enabled(spec) {
                return false;
            }
            if previous_order.is_some_and(|previous| previous >= spec.order) {
                return false;
            }
            previous_order = Some(spec.order);
        }
        true
    }

    pub(crate) fn run<T>(&self, id: PeepholeStageId, run: impl FnOnce() -> T) -> (T, u128) {
        let spec = peephole_stage_spec(id);
        debug_assert!(self.stage_enabled(spec));
        let stage = PeepholeStageRunner::start_spec(spec);
        let result = run();
        (result, stage.finish())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_dev_manager_rejects_standard_only_stages() {
        let manager = PeepholePassManager::for_fast_dev(true);
        let secondary = peephole_stage_spec(PeepholeStageId::SecondaryInline);

        assert!(!manager.stage_enabled(secondary));
        assert!(!manager.validate_sequence(&[PeepholeStageId::SecondaryInline]));
    }

    #[test]
    fn standard_manager_accepts_ordered_full_stage_sequence() {
        let manager = PeepholePassManager::for_fast_dev(false);

        assert!(manager.validate_sequence(&[
            PeepholeStageId::LinearScan,
            PeepholeStageId::PrimaryFlow,
            PeepholeStageId::PrimaryInline,
            PeepholeStageId::PrimaryReuse,
            PeepholeStageId::PrimaryLoopCleanup,
            PeepholeStageId::SecondaryInline,
            PeepholeStageId::SecondaryExact,
            PeepholeStageId::SecondaryHelperCleanup,
            PeepholeStageId::SecondaryRecordSroa,
            PeepholeStageId::SecondaryFinalizeCleanup,
            PeepholeStageId::Finalize,
        ]));
    }

    #[test]
    fn manager_rejects_out_of_order_stage_sequence() {
        let manager = PeepholePassManager::for_fast_dev(false);

        assert!(
            !manager
                .validate_sequence(
                    &[PeepholeStageId::PrimaryInline, PeepholeStageId::PrimaryFlow,]
                )
        );
    }
}
