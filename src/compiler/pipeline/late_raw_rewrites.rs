use super::*;

#[path = "late_raw_rewrites/buffer_swap.rs"]
pub(crate) mod buffer_swap;
#[path = "late_raw_rewrites/cg.rs"]
pub(crate) mod cg;
#[path = "late_raw_rewrites/clamp.rs"]
pub(crate) mod clamp;
#[path = "late_raw_rewrites/melt_rate.rs"]
pub(crate) mod melt_rate;
#[path = "late_raw_rewrites/prune.rs"]
pub(crate) mod prune;

pub(crate) use self::buffer_swap::restore_buffer_swaps_after_temp_copy_in_raw_emitted_r;
pub(crate) use self::cg::restore_cg_loop_carried_updates_in_raw_emitted_r;
pub(crate) use self::clamp::{
    collapse_floor_fed_particle_clamp_pair_in_raw_emitted_r,
    collapse_gray_scott_clamp_pair_in_raw_emitted_r,
};
pub(crate) use self::melt_rate::collapse_sym287_melt_rate_branch_in_raw_emitted_r;
pub(crate) use self::prune::prune_unreachable_raw_helper_definitions;
