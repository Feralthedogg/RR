use super::super::*;

#[derive(Debug, Clone, Copy)]
pub(crate) struct OutlinePolicy {
    pub(crate) enabled: bool,
    pub(crate) min_parent_ir: usize,
    pub(crate) min_region_ir: usize,
    pub(crate) branch_min_region_ir: usize,
    pub(crate) loop_min_region_ir: usize,
    pub(crate) max_live_in: usize,
    pub(crate) max_live_out: usize,
}

impl OutlinePolicy {
    pub(crate) fn for_engine(engine: &TachyonEngine) -> Self {
        let o3 = matches!(engine.opt_level, crate::compiler::OptLevel::O3);
        let default_enabled = matches!(
            engine.opt_level,
            crate::compiler::OptLevel::O2 | crate::compiler::OptLevel::O3
        ) && !engine.fast_dev_enabled()
            && !engine.size_opt_enabled();

        let min_parent_ir = if o3 { 900 } else { 1_200 };
        let min_region_ir = if o3 { 90 } else { 140 };

        Self {
            enabled: TachyonEngine::env_bool("RR_OUTLINE_ENABLE", default_enabled),
            min_parent_ir: env_usize("RR_OUTLINE_MIN_PARENT_IR").unwrap_or(min_parent_ir),
            min_region_ir: env_usize("RR_OUTLINE_MIN_REGION_IR").unwrap_or(min_region_ir),
            branch_min_region_ir: env_usize("RR_OUTLINE_BRANCH_MIN_REGION_IR")
                .unwrap_or(min_region_ir.saturating_mul(2) / 3),
            loop_min_region_ir: env_usize("RR_OUTLINE_LOOP_MIN_REGION_IR")
                .unwrap_or(min_region_ir.saturating_mul(3) / 2),
            max_live_in: if o3 { 12 } else { 8 },
            max_live_out: 4,
        }
    }
}

fn env_usize(key: &str) -> Option<usize> {
    std::env::var(key).ok()?.parse().ok()
}
