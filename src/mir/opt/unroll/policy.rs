use super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct UnrollPolicy {
    pub(crate) enabled: bool,
    pub(crate) max_trip: usize,
    pub(crate) partial_enabled: bool,
    pub(crate) max_partial_trip: usize,
    pub(crate) max_partial_factor: usize,
    pub(crate) max_growth_ir: usize,
}

impl UnrollPolicy {
    pub(crate) fn for_engine(engine: &TachyonEngine) -> Self {
        let default_enabled = !engine.fast_dev_enabled()
            && !engine.size_opt_enabled()
            && !matches!(
                engine.opt_level,
                crate::compiler::OptLevel::O0 | crate::compiler::OptLevel::O1
            );
        let enabled = TachyonEngine::env_bool("RR_UNROLL_ENABLE", default_enabled);
        let default_trip = if engine.aggressive_opt_enabled() {
            16
        } else {
            8
        };
        let partial_enabled = TachyonEngine::env_bool("RR_UNROLL_PARTIAL_ENABLE", enabled);
        let default_partial_trip = if engine.aggressive_opt_enabled() {
            256
        } else {
            128
        };
        let default_partial_factor = if engine.aggressive_opt_enabled() {
            8
        } else {
            4
        };
        let default_growth_ir = if engine.aggressive_opt_enabled() {
            360
        } else {
            180
        };
        Self {
            enabled,
            max_trip: TachyonEngine::env_usize("RR_UNROLL_MAX_TRIP", default_trip),
            partial_enabled,
            max_partial_trip: TachyonEngine::env_usize(
                "RR_UNROLL_MAX_PARTIAL_TRIP",
                default_partial_trip,
            ),
            max_partial_factor: TachyonEngine::env_usize(
                "RR_UNROLL_MAX_FACTOR",
                default_partial_factor,
            ),
            max_growth_ir: TachyonEngine::env_usize("RR_UNROLL_MAX_GROWTH_IR", default_growth_ir),
        }
    }
}
