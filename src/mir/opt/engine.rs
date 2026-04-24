use super::*;

#[derive(Debug, Default, Clone)]
pub struct TachyonRunProfile {
    pub pulse_stats: TachyonPulseStats,
    pub pass_timings: TachyonPassTimings,
    pub active_pass_groups: Vec<String>,
    pub plan_summary: Vec<String>,
}

pub struct TachyonEngine {
    pub(super) phase_ordering_default_mode: types::PhaseOrderingMode,
    pub(super) compile_mode: crate::compiler::CompileMode,
}

// Backward compatibility alias for older call sites.
pub type MirOptimizer = TachyonEngine;

impl Default for TachyonEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl TachyonEngine {
    pub fn new() -> Self {
        Self {
            phase_ordering_default_mode: types::PhaseOrderingMode::Off,
            compile_mode: crate::compiler::CompileMode::Standard,
        }
    }

    pub(crate) fn with_phase_ordering_default_mode(
        phase_ordering_default_mode: types::PhaseOrderingMode,
    ) -> Self {
        Self {
            phase_ordering_default_mode,
            compile_mode: crate::compiler::CompileMode::Standard,
        }
    }

    pub(crate) fn with_phase_ordering_default_mode_and_compile_mode(
        phase_ordering_default_mode: types::PhaseOrderingMode,
        compile_mode: crate::compiler::CompileMode,
    ) -> Self {
        Self {
            phase_ordering_default_mode,
            compile_mode,
        }
    }

    pub(super) fn fast_dev_enabled(&self) -> bool {
        matches!(self.compile_mode, crate::compiler::CompileMode::FastDev)
    }

    pub(super) fn configured_max_opt_iterations(&self) -> usize {
        if self.fast_dev_enabled() {
            8
        } else {
            Self::max_opt_iterations()
        }
    }

    pub(super) fn configured_max_inline_rounds(&self) -> usize {
        if self.fast_dev_enabled() {
            1
        } else {
            Self::max_inline_rounds()
        }
    }

    pub(super) fn configured_heavy_pass_fn_ir(&self) -> usize {
        if self.fast_dev_enabled() {
            384
        } else {
            Self::heavy_pass_fn_ir()
        }
    }

    pub(super) fn configured_always_bce_fn_ir(&self) -> usize {
        self.configured_heavy_pass_fn_ir().max(64)
    }

    pub(super) fn configured_max_fn_opt_ms(&self) -> u128 {
        if self.fast_dev_enabled() {
            80
        } else {
            Self::max_fn_opt_ms()
        }
    }

    pub(super) fn configured_always_tier_max_iters(&self) -> usize {
        if self.fast_dev_enabled() {
            1
        } else {
            Self::always_tier_max_iters()
        }
    }

    pub(super) fn structural_optimizations_enabled(&self) -> bool {
        !self.fast_dev_enabled()
    }

    pub(super) fn inline_tier_enabled(&self) -> bool {
        true
    }

    pub(super) fn adjust_pass_groups_for_mode(
        &self,
        groups: &[types::PassGroup],
    ) -> Vec<types::PassGroup> {
        groups
            .iter()
            .copied()
            .filter(|group| match group {
                types::PassGroup::Required | types::PassGroup::DevCheap => true,
                types::PassGroup::ReleaseExpensive | types::PassGroup::Experimental => {
                    !self.fast_dev_enabled()
                }
            })
            .collect()
    }

    pub(super) fn active_pass_group_labels(&self) -> Vec<String> {
        let base = [
            types::PassGroup::Required,
            types::PassGroup::DevCheap,
            types::PassGroup::ReleaseExpensive,
            types::PassGroup::Experimental,
        ];
        self.adjust_pass_groups_for_mode(&base)
            .into_iter()
            .map(|group| group.label().to_string())
            .collect()
    }

    pub(super) fn plan_summary_lines(
        &self,
        ordered_names: &[String],
        plans: &FxHashMap<String, types::FunctionPhasePlan>,
    ) -> Vec<String> {
        let mut out = Vec::new();
        for name in ordered_names {
            // Proof correspondence:
            // `PhasePlanSummarySoundness` refines this ordered-summary
            // consumption boundary on top of `PhasePlanLookupSoundness`.
            // The reduced model keeps the same traversal shape:
            // ordered function ids, lookup hit/miss, and summary entries that
            // expose schedule/profile/pass-group payload from the looked-up
            // plan.
            let Some(plan) = plans.get(name) else {
                continue;
            };
            let groups = plan
                .pass_groups
                .iter()
                .map(|group| group.label())
                .collect::<Vec<_>>()
                .join(",");
            out.push(format!(
                "{} schedule={} profile={} groups={}",
                name,
                plan.schedule.label(),
                plan.profile.label(),
                groups
            ));
        }
        out
    }
}
