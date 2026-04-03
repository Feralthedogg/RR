//! Generic polyhedral MIR reconstruction helpers.
//!
//! These routines rebuild structured loop nests directly from affine schedule
//! information when the specialized poly codegen path is unavailable or not a
//! good fit for the selected schedule shape.

use super::ScopRegion;
use super::access::MemoryLayout;
use super::affine::{AffineExpr, AffineSymbol};
use super::schedule::{SchedulePlan, SchedulePlanKind};
use super::scop::{PolyStmt, PolyStmtKind};
use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::opt::v_opt::ReduceKind;
use crate::mir::opt::v_opt::vector_apply_site;
use crate::mir::{Facts, FnIR, Instr, Lit, Terminator, ValueId, ValueKind};
use crate::syntax::ast::BinOp;
use crate::utils::Span;
use rustc_hash::{FxHashMap, FxHashSet};

const GENERATED_LOOP_IV_PREFIX: &str = ".__poly_gen_iv_";

pub fn generic_mir_enabled() -> bool {
    std::env::var("RR_POLY_GENERIC_MIR")
        .ok()
        .is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
}

pub fn generic_mir_effective_for_schedule(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    generic_mir_enabled() || default_generic_schedule(scop, schedule)
}

fn generic_mir_effective(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    generic_mir_effective_for_schedule(scop, schedule)
}

fn default_generic_schedule(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    default_generic_single_dim(scop, schedule) || default_generic_full_rank_nested(scop, schedule)
}

fn default_generic_single_dim(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    if scop.dimensions.len() != 1 {
        return false;
    }
    let dim = &scop.dimensions[0];
    if dim.step != 1 {
        return false;
    }
    matches!(
        schedule.kind,
        SchedulePlanKind::Identity | SchedulePlanKind::Tile1D
    )
}

fn default_generic_full_rank_nested(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    if scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    match schedule.kind {
        SchedulePlanKind::Identity => matches!(scop.dimensions.len(), 2 | 3),
        SchedulePlanKind::Interchange => matches!(scop.dimensions.len(), 2 | 3),
        SchedulePlanKind::Skew2D => scop.dimensions.len() == 2,
        SchedulePlanKind::Tile2D => scop.dimensions.len() == 2,
        SchedulePlanKind::Tile3D => scop.dimensions.len() == 3,
        _ => false,
    }
}

pub fn generic_fission_enabled() -> bool {
    let enabled = |key: &str| {
        std::env::var(key).ok().is_some_and(|value| {
            matches!(
                value.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
    };
    enabled("RR_POLY_GENERIC_FISSION") || enabled("RR_POLY_FISSION")
}

fn generic_fission_effective(scop: &ScopRegion, schedule: &SchedulePlan) -> bool {
    generic_fission_enabled() || default_generic_schedule(scop, schedule)
}

pub fn is_generated_loop_iv_name(name: &str) -> bool {
    name.starts_with(GENERATED_LOOP_IV_PREFIX)
}

pub fn lower_identity_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Identity {
        return false;
    }
    if scop.dimensions.len() == 1
        && scop
            .statements
            .iter()
            .flat_map(|stmt| stmt.accesses.iter())
            .any(|access| !access_is_generic_single_dim_contiguous(access, scop))
    {
        return false;
    }
    if !scop_is_generic_map_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_map_schedule(fn_ir, lp, scop, schedule)
}

pub fn generic_schedule_supports_map(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    match schedule.kind {
        SchedulePlanKind::Identity => {
            if scop.dimensions.len() == 1
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .any(|access| !access_is_generic_single_dim_contiguous(access, scop))
            {
                return false;
            }
            scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Interchange => {
            (scop.dimensions.len() == 2 || scop.dimensions.len() == 3)
                && scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Skew2D => {
            scop.dimensions.len() == 2 && scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile1D => {
            scop.dimensions.len() == 1
                && scop.dimensions[0].step == 1
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .all(|access| access_is_generic_single_dim_contiguous(access, scop))
                && scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile2D => {
            scop.dimensions.len() == 2
                && scop.dimensions.iter().all(|dim| dim.step == 1)
                && scop.statements.iter().all(|stmt| {
                    matches!(
                        &stmt.kind,
                        PolyStmtKind::Store { subscripts, .. } if subscripts.len() == 2
                    ) || !matches!(&stmt.kind, PolyStmtKind::Store { .. })
                })
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .all(|access| {
                        access.memref.layout == MemoryLayout::ColumnMajor2D
                            && access.subscripts.len() == 2
                    })
                && scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile3D => {
            scop.dimensions.len() == 3
                && scop.dimensions.iter().all(|dim| dim.step == 1)
                && scop.statements.iter().all(|stmt| {
                    matches!(
                        &stmt.kind,
                        PolyStmtKind::Store { subscripts, .. } if subscripts.len() == 3
                    ) || !matches!(&stmt.kind, PolyStmtKind::Store { .. })
                })
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .all(|access| {
                        access.memref.layout == MemoryLayout::ColumnMajor3D
                            && access.subscripts.len() == 3
                    })
                && scop_is_generic_map_compatible(fn_ir, scop)
        }
        SchedulePlanKind::None => false,
    }
}

pub fn lower_tile1d_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile1D || scop.dimensions.len() != 1 {
        return false;
    }
    if scop.dimensions[0].step != 1 {
        return false;
    }
    if scop
        .statements
        .iter()
        .flat_map(|stmt| stmt.accesses.iter())
        .any(|access| !access_is_generic_single_dim_contiguous(access, scop))
    {
        return false;
    }
    if !scop_is_generic_map_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_1d(fn_ir, lp, scop, schedule, false)
}

pub fn lower_tile2d_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile2D || scop.dimensions.len() != 2 {
        return false;
    }
    if scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    if scop.statements.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            PolyStmtKind::Store { subscripts, .. } if subscripts.len() != 2
        )
    }) {
        return false;
    }
    if scop
        .statements
        .iter()
        .flat_map(|stmt| stmt.accesses.iter())
        .any(|access| {
            access.memref.layout != MemoryLayout::ColumnMajor2D || access.subscripts.len() != 2
        })
    {
        return false;
    }
    if !scop_is_generic_map_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_2d(fn_ir, lp, scop, schedule, false)
}

pub fn lower_tile3d_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile3D || scop.dimensions.len() != 3 {
        return false;
    }
    if scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    if scop.statements.iter().any(|stmt| {
        matches!(
            &stmt.kind,
            PolyStmtKind::Store { subscripts, .. } if subscripts.len() != 3
        )
    }) {
        return false;
    }
    if scop
        .statements
        .iter()
        .flat_map(|stmt| stmt.accesses.iter())
        .any(|access| {
            access.memref.layout != MemoryLayout::ColumnMajor3D || access.subscripts.len() != 3
        })
    {
        return false;
    }
    if !scop_is_generic_map_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_3d(fn_ir, lp, scop, schedule, false)
}

pub fn lower_identity_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Identity {
        return false;
    }
    if scop.dimensions.len() == 1 {
        if !scop_is_generic_reduce_compatible(fn_ir, scop) {
            return false;
        }
        return lower_generic_reduce_schedule(fn_ir, lp, scop, schedule);
    }
    if !scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_reduce_schedule(fn_ir, lp, scop, schedule)
}

pub fn generic_schedule_supports_reduce(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    match schedule.kind {
        SchedulePlanKind::Identity => {
            if scop.dimensions.len() == 1 {
                scop_is_generic_reduce_compatible(fn_ir, scop)
            } else {
                scop_is_generic_nested_reduce_compatible(fn_ir, scop)
            }
        }
        SchedulePlanKind::Interchange => {
            (scop.dimensions.len() == 2 || scop.dimensions.len() == 3)
                && scop_is_generic_nested_reduce_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Skew2D => {
            scop.dimensions.len() == 2 && scop_is_generic_nested_reduce_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile1D => {
            scop.dimensions.len() == 1
                && scop.dimensions[0].step == 1
                && scop_is_generic_reduce_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile2D => {
            scop.dimensions.len() == 2
                && scop.dimensions.iter().all(|dim| dim.step == 1)
                && scop_is_generic_nested_reduce_compatible(fn_ir, scop)
        }
        SchedulePlanKind::Tile3D => {
            scop.dimensions.len() == 3
                && scop.dimensions.iter().all(|dim| dim.step == 1)
                && scop_is_generic_nested_reduce_compatible(fn_ir, scop)
        }
        SchedulePlanKind::None => false,
    }
}

pub fn lower_tile1d_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile1D || scop.dimensions.len() != 1 {
        return false;
    }
    if scop.dimensions[0].step != 1 {
        return false;
    }
    if !scop_is_generic_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_1d(fn_ir, lp, scop, schedule, true)
}

pub fn lower_tile2d_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile2D || scop.dimensions.len() != 2 {
        return false;
    }
    if scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    if !scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_2d(fn_ir, lp, scop, schedule, true)
}

pub fn lower_tile3d_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Tile3D || scop.dimensions.len() != 3 {
        return false;
    }
    if scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    if !scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_tiled_3d(fn_ir, lp, scop, schedule, true)
}

pub fn lower_interchange_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Interchange
        || !(scop.dimensions.len() == 2 || scop.dimensions.len() == 3)
    {
        return false;
    }
    lower_generic_map_schedule(fn_ir, lp, scop, schedule)
}

pub fn lower_interchange_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Interchange
        || !(scop.dimensions.len() == 2 || scop.dimensions.len() == 3)
    {
        return false;
    }
    if !scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_reduce_schedule(fn_ir, lp, scop, schedule)
}

pub fn lower_skew2d_map_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Skew2D || scop.dimensions.len() != 2 {
        return false;
    }
    lower_generic_skew2d(fn_ir, lp, scop, schedule, false)
}

pub fn lower_skew2d_reduce_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if schedule.kind != SchedulePlanKind::Skew2D || scop.dimensions.len() != 2 {
        return false;
    }
    if !scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
        return false;
    }
    lower_generic_skew2d(fn_ir, lp, scop, schedule, true)
}

pub(super) fn lower_fission_sequence_generic(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if !generic_fission_effective(scop, schedule) || !has_multiple_data_statements(scop) {
        return false;
    }

    match schedule.kind {
        SchedulePlanKind::Identity => {
            if scop.dimensions.len() == 1
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .all(|access| access_is_generic_single_dim_contiguous(access, scop))
                && scop_is_generic_map_compatible(fn_ir, scop)
            {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_loop_nest,
                );
            }
            if scop.dimensions.len() == 1 && scop_is_generic_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::Interchange => {
            if scop_is_generic_map_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_loop_nest,
                );
            }
            if scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::Skew2D => {
            if scop_is_generic_map_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_skewed_2d_loop_nest,
                );
            }
            if scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_skewed_2d_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::Tile1D => {
            if scop.dimensions.len() == 1
                && scop
                    .statements
                    .iter()
                    .flat_map(|stmt| stmt.accesses.iter())
                    .all(|access| access_is_generic_single_dim_contiguous(access, scop))
                && scop_is_generic_map_compatible(fn_ir, scop)
            {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_tiled_1d_loop_nest,
                );
            }
            if scop.dimensions.len() == 1 && scop_is_generic_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_tiled_1d_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::Tile2D => {
            if scop_is_generic_map_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_tiled_2d_loop_nest,
                );
            }
            if scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_tiled_2d_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::Tile3D => {
            if scop_is_generic_map_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    false,
                    rebuild_generic_tiled_3d_loop_nest,
                );
            }
            if scop_is_generic_nested_reduce_compatible(fn_ir, scop) {
                return lower_generic_fission_sequence_with_builder(
                    fn_ir,
                    lp,
                    scop,
                    schedule,
                    true,
                    rebuild_generic_tiled_3d_loop_nest,
                );
            }
            false
        }
        SchedulePlanKind::None => false,
    }
}

fn lower_generic_map_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            lp,
            scop,
            schedule,
            site.preheader,
            site.exit_bb,
            false,
            rebuild_generic_loop_nest,
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-map-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    if !scop_is_generic_map_compatible(fn_ir, scop) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: scop not map-compatible",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: missing apply site",
                fn_ir.name, lp.header
            );
        }
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        false,
    ) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: rebuild failed",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

fn lower_generic_fission_sequence_with_builder(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    skip_accessless_assigns: bool,
    lower_one: fn(&mut FnIR, &LoopInfo, &ScopRegion, &SchedulePlan, usize, usize, bool) -> bool,
) -> bool {
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_fissioned_sequence(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        skip_accessless_assigns,
        lower_one,
    ) {
        return false;
    }
    *fn_ir = trial;
    true
}

fn lower_generic_tiled_1d(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    skip_accessless_assigns: bool,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            lp,
            scop,
            schedule,
            site.preheader,
            site.exit_bb,
            skip_accessless_assigns,
            rebuild_generic_tiled_1d_loop_nest,
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-tile1d-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: missing apply site",
                fn_ir.name, lp.header
            );
        }
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_tiled_1d_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        skip_accessless_assigns,
    ) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: tiled rebuild failed",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-tile1d-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

fn lower_generic_skew2d(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    skip_accessless_assigns: bool,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            lp,
            scop,
            schedule,
            site.preheader,
            site.exit_bb,
            skip_accessless_assigns,
            rebuild_generic_skewed_2d_loop_nest,
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-skew2d-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_skewed_2d_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        skip_accessless_assigns,
    ) {
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-skew2d-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

fn lower_generic_tiled_2d(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    skip_accessless_assigns: bool,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            lp,
            scop,
            schedule,
            site.preheader,
            site.exit_bb,
            skip_accessless_assigns,
            rebuild_generic_tiled_2d_loop_nest,
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-tile2d-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: missing apply site",
                fn_ir.name, lp.header
            );
        }
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_tiled_2d_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        skip_accessless_assigns,
    ) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: tiled2d rebuild failed",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-tile2d-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

fn lower_generic_tiled_3d(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    skip_accessless_assigns: bool,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            lp,
            scop,
            schedule,
            site.preheader,
            site.exit_bb,
            skip_accessless_assigns,
            rebuild_generic_tiled_3d_loop_nest,
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-tile3d-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: missing apply site",
                fn_ir.name, lp.header
            );
        }
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_tiled_3d_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        skip_accessless_assigns,
    ) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: tiled3d rebuild failed",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-tile3d-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

fn lower_generic_reduce_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> bool {
    if !generic_mir_effective(scop, schedule) {
        return false;
    }
    if generic_fission_effective(scop, schedule) && has_multiple_data_statements(scop) {
        let Some(site) = vector_apply_site(fn_ir, lp) else {
            return false;
        };
        let mut trial = fn_ir.clone();
        if !rebuild_generic_fissioned_sequence(
            &mut trial,
            lp,
            scop,
            schedule,
            site.preheader,
            site.exit_bb,
            true,
            rebuild_generic_loop_nest,
        ) {
            return false;
        }
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} apply=true generic-fission-reduce-loop-sequence",
                fn_ir.name, lp.header
            );
        }
        *fn_ir = trial;
        return true;
    }
    let Some(site) = vector_apply_site(fn_ir, lp) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: missing apply site",
                fn_ir.name, lp.header
            );
        }
        return false;
    };
    let mut trial = fn_ir.clone();
    if !rebuild_generic_loop_nest(
        &mut trial,
        lp,
        scop,
        schedule,
        site.preheader,
        site.exit_bb,
        true,
    ) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-generic] {} loop header={} reject: rebuild failed",
                fn_ir.name, lp.header
            );
        }
        return false;
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-generic] {} loop header={} apply=true generic-reduction-loop-nest",
            fn_ir.name, lp.header
        );
    }
    *fn_ir = trial;
    true
}

fn scop_is_generic_map_compatible(fn_ir: &FnIR, scop: &ScopRegion) -> bool {
    let loop_iv_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<FxHashSet<_>>();
    scop.statements
        .iter()
        .all(|stmt| match (&stmt.kind, stmt.expr_root) {
            (PolyStmtKind::Assign { dst }, _) if loop_iv_names.contains(dst.as_str()) => true,
            (PolyStmtKind::Assign { .. }, _) if stmt_is_progress_assign(fn_ir, scop, stmt) => true,
            (PolyStmtKind::Assign { dst }, Some(expr_root)) => {
                !expr_mentions_var(fn_ir, expr_root, dst, &mut FxHashSet::default())
            }
            (PolyStmtKind::Store { .. }, Some(_)) => true,
            (PolyStmtKind::Eval, Some(_)) => true,
            _ => false,
        })
}

fn scop_is_generic_reduce_compatible(fn_ir: &FnIR, scop: &ScopRegion) -> bool {
    let loop_iv_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<FxHashSet<_>>();
    let mut reduction_stmt_count = 0usize;
    for stmt in &scop.statements {
        if stmt.accesses.is_empty() {
            continue;
        }
        match (&stmt.kind, stmt.expr_root) {
            (PolyStmtKind::Assign { dst }, _) if loop_iv_names.contains(dst.as_str()) => continue,
            (PolyStmtKind::Assign { .. }, _) if stmt_is_progress_assign(fn_ir, scop, stmt) => {
                continue;
            }
            (PolyStmtKind::Assign { dst }, Some(expr_root)) => {
                if classify_generic_reduce_kind(fn_ir, scop, stmt, dst, expr_root).is_none() {
                    return false;
                }
                reduction_stmt_count += 1;
            }
            _ => return false,
        }
    }
    reduction_stmt_count >= 1
}

fn scop_is_generic_nested_reduce_compatible(fn_ir: &FnIR, scop: &ScopRegion) -> bool {
    let loop_iv_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<FxHashSet<_>>();
    let mut reduction_stmt_count = 0usize;
    for stmt in &scop.statements {
        if stmt.accesses.is_empty() {
            continue;
        }
        match (&stmt.kind, stmt.expr_root) {
            (PolyStmtKind::Assign { dst }, _) if loop_iv_names.contains(dst.as_str()) => continue,
            (PolyStmtKind::Assign { .. }, _) if stmt_is_progress_assign(fn_ir, scop, stmt) => {
                continue;
            }
            (PolyStmtKind::Assign { dst }, Some(expr_root)) => {
                if classify_generic_nested_reduce_kind(fn_ir, scop, stmt, dst, expr_root).is_none()
                {
                    return false;
                }
                reduction_stmt_count += 1;
            }
            _ => return false,
        }
    }
    reduction_stmt_count >= 1
}

fn classify_generic_reduce_kind(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    stmt: &PolyStmt,
    dst: &str,
    expr_root: ValueId,
) -> Option<ReduceKind> {
    if !stmt_reads_dense_1d_loop_vector(stmt, scop) {
        return None;
    }
    let root = resolve_scop_local_source(fn_ir, scop, expr_root);
    match &fn_ir.values[root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let lhs = resolve_scop_local_source(fn_ir, scop, *lhs);
            let rhs = resolve_scop_local_source(fn_ir, scop, *rhs);
            let lhs_self = is_same_named_value(fn_ir, lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, rhs, dst);
            if lhs_self ^ rhs_self {
                Some(if *op == BinOp::Add {
                    ReduceKind::Sum
                } else {
                    ReduceKind::Prod
                })
            } else {
                None
            }
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let lhs = resolve_scop_local_source(fn_ir, scop, args[0]);
            let rhs = resolve_scop_local_source(fn_ir, scop, args[1]);
            let lhs_self = is_same_named_value(fn_ir, lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, rhs, dst);
            if !(lhs_self ^ rhs_self) {
                return None;
            }
            if matches!(
                fn_ir.call_semantics(root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                Some(ReduceKind::Min)
            } else {
                Some(ReduceKind::Max)
            }
        }
        _ => None,
    }
}

fn classify_generic_nested_reduce_kind(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    _stmt: &PolyStmt,
    dst: &str,
    expr_root: ValueId,
) -> Option<ReduceKind> {
    let root = resolve_scop_local_source(fn_ir, scop, expr_root);
    match &fn_ir.values[root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let lhs = resolve_scop_local_source(fn_ir, scop, *lhs);
            let rhs = resolve_scop_local_source(fn_ir, scop, *rhs);
            let lhs_self = is_same_named_value(fn_ir, lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, rhs, dst);
            if lhs_self ^ rhs_self {
                Some(if *op == BinOp::Add {
                    ReduceKind::Sum
                } else {
                    ReduceKind::Prod
                })
            } else {
                None
            }
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let lhs = resolve_scop_local_source(fn_ir, scop, args[0]);
            let rhs = resolve_scop_local_source(fn_ir, scop, args[1]);
            let lhs_self = is_same_named_value(fn_ir, lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, rhs, dst);
            if !(lhs_self ^ rhs_self) {
                return None;
            }
            if matches!(
                fn_ir.call_semantics(root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                Some(ReduceKind::Min)
            } else {
                Some(ReduceKind::Max)
            }
        }
        _ => None,
    }
}

fn stmt_reads_dense_1d_loop_vector(stmt: &PolyStmt, scop: &ScopRegion) -> bool {
    stmt.accesses.iter().any(|access| {
        matches!(access.kind, super::access::AccessKind::Read)
            && access_is_generic_single_dim_contiguous(access, scop)
    })
}

fn has_multiple_data_statements(scop: &ScopRegion) -> bool {
    scop.statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .count()
        > 1
}

fn expr_mentions_any_loop_iv(scop: &ScopRegion, expr: &AffineExpr) -> bool {
    expr.terms.iter().any(|(symbol, coeff)| {
        *coeff != 0
            && matches!(
                symbol,
                AffineSymbol::LoopIv(name) if scop.dimensions.iter().any(|dim| dim.iv_name == *name)
            )
    })
}

fn access_is_generic_single_dim_contiguous(
    access: &super::access::AccessRelation,
    scop: &ScopRegion,
) -> bool {
    fn expr_is_unit_stride_for_loop(expr: &AffineExpr, scop: &ScopRegion) -> bool {
        let mut found = false;
        for (symbol, coeff) in &expr.terms {
            match symbol {
                AffineSymbol::LoopIv(name)
                    if scop.dimensions.iter().any(|dim| dim.iv_name == *name) =>
                {
                    if found || *coeff != 1 {
                        return false;
                    }
                    found = true;
                }
                AffineSymbol::LoopIv(_) => return false,
                _ => {}
            }
        }
        found
    }

    if scop.dimensions.len() != 1 {
        return false;
    }
    match access.memref.layout {
        MemoryLayout::Dense1D => {
            access.subscripts.len() == 1
                && expr_is_unit_stride_for_loop(&access.subscripts[0], scop)
        }
        MemoryLayout::ColumnMajor2D => {
            access.subscripts.len() == 2
                && expr_is_unit_stride_for_loop(&access.subscripts[0], scop)
                && !expr_mentions_any_loop_iv(scop, &access.subscripts[1])
        }
        MemoryLayout::ColumnMajor3D => {
            access.subscripts.len() == 3
                && expr_is_unit_stride_for_loop(&access.subscripts[0], scop)
                && !expr_mentions_any_loop_iv(scop, &access.subscripts[1])
                && !expr_mentions_any_loop_iv(scop, &access.subscripts[2])
        }
    }
}

fn is_loop_iv_subscript(scop: &ScopRegion, expr: &AffineExpr) -> bool {
    expr.constant == 0
        && matches!(
            expr.terms.iter().next(),
            Some((AffineSymbol::LoopIv(name), coeff))
                if expr.terms.len() == 1
                    && *coeff == 1
                    && scop.dimensions.iter().any(|dim| dim.iv_name == *name)
        )
}

fn resolve_scop_local_source(fn_ir: &FnIR, scop: &ScopRegion, root: ValueId) -> ValueId {
    let mut current = root;
    let mut seen = FxHashSet::default();
    loop {
        if !seen.insert(current) {
            return current;
        }
        let ValueKind::Load { var } = &fn_ir.values[current].kind else {
            return current;
        };
        let mut matches =
            scop.statements
                .iter()
                .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
                    (PolyStmtKind::Assign { dst }, Some(expr_root)) if dst == var => {
                        Some(expr_root)
                    }
                    _ => None,
                });
        let Some(next) = matches.next() else {
            return current;
        };
        if matches.next().is_some() {
            return current;
        }
        current = next;
    }
}

fn is_same_named_value(fn_ir: &FnIR, value: ValueId, name: &str) -> bool {
    fn rec(fn_ir: &FnIR, value: ValueId, name: &str, seen: &mut FxHashSet<ValueId>) -> bool {
        if !seen.insert(value) {
            return false;
        }
        if fn_ir.values[value].origin_var.as_deref() == Some(name) {
            return true;
        }
        match &fn_ir.values[value].kind {
            ValueKind::Load { var } => var == name,
            ValueKind::Phi { args } => args.iter().any(|(arg, _)| rec(fn_ir, *arg, name, seen)),
            _ => false,
        }
    }
    rec(fn_ir, value, name, &mut FxHashSet::default())
}

fn expr_mentions_var(
    fn_ir: &FnIR,
    root: ValueId,
    var: &str,
    seen: &mut FxHashSet<ValueId>,
) -> bool {
    if !seen.insert(root) {
        return false;
    }
    if fn_ir.values[root].origin_var.as_deref() == Some(var) {
        return true;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Load { var: load_var } => load_var == var,
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_mentions_var(fn_ir, *lhs, var, seen) || expr_mentions_var(fn_ir, *rhs, var, seen)
        }
        ValueKind::Unary { rhs, .. } => expr_mentions_var(fn_ir, *rhs, var, seen),
        ValueKind::RecordLit { fields } => fields
            .iter()
            .any(|(_, value)| expr_mentions_var(fn_ir, *value, var, seen)),
        ValueKind::FieldGet { base, .. } => expr_mentions_var(fn_ir, *base, var, seen),
        ValueKind::FieldSet { base, value, .. } => {
            expr_mentions_var(fn_ir, *base, var, seen)
                || expr_mentions_var(fn_ir, *value, var, seen)
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| expr_mentions_var(fn_ir, *arg, var, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(arg, _)| expr_mentions_var(fn_ir, *arg, var, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_mentions_var(fn_ir, *base, var, seen)
        }
        ValueKind::Range { start, end } => {
            expr_mentions_var(fn_ir, *start, var, seen) || expr_mentions_var(fn_ir, *end, var, seen)
        }
        ValueKind::Index1D { base, idx, .. } => {
            expr_mentions_var(fn_ir, *base, var, seen) || expr_mentions_var(fn_ir, *idx, var, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            expr_mentions_var(fn_ir, *base, var, seen)
                || expr_mentions_var(fn_ir, *r, var, seen)
                || expr_mentions_var(fn_ir, *c, var, seen)
        }
        ValueKind::Index3D { base, i, j, k } => {
            expr_mentions_var(fn_ir, *base, var, seen)
                || expr_mentions_var(fn_ir, *i, var, seen)
                || expr_mentions_var(fn_ir, *j, var, seen)
                || expr_mentions_var(fn_ir, *k, var, seen)
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
    }
}

fn ordered_dimension_names(schedule: &SchedulePlan, scop: &ScopRegion) -> Vec<String> {
    let mut dims = Vec::with_capacity(schedule.relation.output_expressions.len());
    for expr in &schedule.relation.output_expressions {
        let Some((AffineSymbol::LoopIv(name), coeff)) = expr.terms.iter().next() else {
            return scop
                .dimensions
                .iter()
                .map(|dim| dim.iv_name.clone())
                .collect();
        };
        if *coeff != 1 || expr.terms.len() != 1 || expr.constant != 0 {
            return scop
                .dimensions
                .iter()
                .map(|dim| dim.iv_name.clone())
                .collect();
        }
        dims.push(name.clone());
    }
    if dims.is_empty() {
        scop.dimensions
            .iter()
            .map(|dim| dim.iv_name.clone())
            .collect()
    } else {
        dims
    }
}

fn generated_iv_name(header: usize, dim_name: &str) -> String {
    format!("{GENERATED_LOOP_IV_PREFIX}{header}_{dim_name}")
}

fn generated_tile_iv_name(header: usize, dim_name: &str) -> String {
    format!("{GENERATED_LOOP_IV_PREFIX}tile_{header}_{dim_name}")
}

fn rebuild_generic_loop_nest(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
) -> bool {
    let ordered_names = ordered_dimension_names(schedule, scop);
    let mut dims = Vec::with_capacity(ordered_names.len());
    for name in ordered_names {
        let Some(dim) = scop.dimensions.iter().find(|dim| dim.iv_name == name) else {
            return false;
        };
        dims.push(dim.clone());
    }

    let mut loop_var_map = FxHashMap::default();
    for dim in &scop.dimensions {
        loop_var_map.insert(
            dim.iv_name.clone(),
            generated_iv_name(lp.header, &dim.iv_name),
        );
    }
    for dim in &dims {
        let Some(dst) = loop_var_map.get(&dim.iv_name).cloned() else {
            return false;
        };
        let Some(init_val) = materialize_affine_expr(fn_ir, &dim.lower_bound, &loop_var_map) else {
            return false;
        };
        fn_ir.blocks[preheader].instrs.push(Instr::Assign {
            dst,
            src: init_val,
            span: Span::dummy(),
        });
    }

    let Some(entry_init) = build_loop_level(
        fn_ir,
        &dims,
        0,
        scop,
        &loop_var_map,
        exit_bb,
        skip_accessless_assigns,
    ) else {
        return false;
    };
    fn_ir.blocks[preheader].term = Terminator::Goto(entry_init);

    for bid in &lp.body {
        fn_ir.blocks[*bid].instrs.clear();
        fn_ir.blocks[*bid].term = Terminator::Goto(exit_bb);
    }

    true
}

fn rebuild_generic_tiled_1d_loop_nest(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
) -> bool {
    let Some(tile_size) = schedule.tile_size.filter(|size| *size > 0) else {
        return false;
    };
    let dim = &scop.dimensions[0];
    let loop_var = generated_iv_name(lp.header, &dim.iv_name);
    let tile_var = generated_tile_iv_name(lp.header, &dim.iv_name);
    let mut loop_var_map = FxHashMap::default();
    loop_var_map.insert(dim.iv_name.clone(), loop_var.clone());

    let Some(lower) = materialize_affine_expr(fn_ir, &dim.lower_bound, &loop_var_map) else {
        return false;
    };
    let Some(upper) = materialize_affine_expr(fn_ir, &dim.upper_bound, &loop_var_map) else {
        return false;
    };
    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: tile_var.clone(),
        src: lower,
        span: Span::dummy(),
    });
    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: loop_var.clone(),
        src: lower,
        span: Span::dummy(),
    });

    let outer_header = fn_ir.add_block();
    let inner_init = fn_ir.add_block();
    let inner_header = fn_ir.add_block();
    let inner_step = fn_ir.add_block();
    let outer_step = fn_ir.add_block();
    let body_bb = fn_ir.add_block();
    fn_ir.blocks[preheader].term = Terminator::Goto(outer_header);

    let tile_load = build_load(fn_ir, tile_var.clone());
    let outer_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: tile_load,
            rhs: upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_header].term = Terminator::If {
        cond: outer_cond,
        then_bb: inner_init,
        else_bb: exit_bb,
    };

    let tile_load_for_init = build_load(fn_ir, tile_var.clone());
    fn_ir.blocks[inner_init].instrs.push(Instr::Assign {
        dst: loop_var.clone(),
        src: tile_load_for_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_init].term = Terminator::Goto(inner_header);

    let loop_load_a = build_load(fn_ir, loop_var.clone());
    let inner_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: loop_load_a,
            rhs: upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile_load_for_limit = build_load(fn_ir, tile_var.clone());
    let tile_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_size - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_load_for_limit,
            rhs: tile_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let loop_load_b = build_load(fn_ir, loop_var.clone());
    let inner_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: loop_load_b,
            rhs: tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let inner_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: inner_cond_a,
            rhs: inner_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[inner_header].term = Terminator::If {
        cond: inner_cond,
        then_bb: body_bb,
        else_bb: outer_step,
    };

    if emit_loop_iv_aliases(fn_ir, body_bb, scop, &loop_var_map).is_none()
        || emit_generic_body(fn_ir, body_bb, scop, &loop_var_map, skip_accessless_assigns).is_none()
    {
        return false;
    }
    fn_ir.blocks[body_bb].term = Terminator::Goto(inner_step);

    let loop_load_for_step = build_load(fn_ir, loop_var.clone());
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_loop = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: loop_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[inner_step].instrs.push(Instr::Assign {
        dst: loop_var,
        src: next_loop,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_step].term = Terminator::Goto(inner_header);

    let tile_load_for_step = build_load(fn_ir, tile_var.clone());
    let tile_step = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_size as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_load_for_step,
            rhs: tile_step,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_step].instrs.push(Instr::Assign {
        dst: tile_var,
        src: next_tile,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer_step].term = Terminator::Goto(outer_header);

    for bid in &lp.body {
        fn_ir.blocks[*bid].instrs.clear();
        fn_ir.blocks[*bid].term = Terminator::Goto(exit_bb);
    }

    true
}

fn generated_skew_iv_name(header: usize, dim_name: &str) -> String {
    format!("{GENERATED_LOOP_IV_PREFIX}skew_{header}_{dim_name}")
}

fn rebuild_generic_skewed_2d_loop_nest(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    _schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
) -> bool {
    if scop.dimensions.len() != 2 || scop.dimensions.iter().any(|dim| dim.step != 1) {
        return false;
    }
    let outer_dim = &scop.dimensions[0];
    let inner_dim = &scop.dimensions[1];
    let outer_var = generated_iv_name(lp.header, &outer_dim.iv_name);
    let inner_var = generated_iv_name(lp.header, &inner_dim.iv_name);
    let skew_var = generated_skew_iv_name(lp.header, &inner_dim.iv_name);

    let mut loop_var_map = FxHashMap::default();
    loop_var_map.insert(outer_dim.iv_name.clone(), outer_var.clone());
    loop_var_map.insert(inner_dim.iv_name.clone(), inner_var.clone());

    let Some(outer_lower) = materialize_affine_expr(fn_ir, &outer_dim.lower_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(outer_upper) = materialize_affine_expr(fn_ir, &outer_dim.upper_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(inner_lower) = materialize_affine_expr(fn_ir, &inner_dim.lower_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(inner_upper) = materialize_affine_expr(fn_ir, &inner_dim.upper_bound, &loop_var_map)
    else {
        return false;
    };

    fn_ir.blocks[preheader].instrs.push(Instr::Assign {
        dst: outer_var.clone(),
        src: outer_lower,
        span: Span::dummy(),
    });

    let outer_header = fn_ir.add_block();
    let inner_init = fn_ir.add_block();
    let inner_header = fn_ir.add_block();
    let body_bb = fn_ir.add_block();
    let inner_step = fn_ir.add_block();
    let outer_step = fn_ir.add_block();

    fn_ir.blocks[preheader].term = Terminator::Goto(outer_header);

    let outer_load = build_load(fn_ir, outer_var.clone());
    let outer_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: outer_load,
            rhs: outer_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_header].term = Terminator::If {
        cond: outer_cond,
        then_bb: inner_init,
        else_bb: exit_bb,
    };

    let outer_for_init = build_load(fn_ir, outer_var.clone());
    let skew_init = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: outer_for_init,
            rhs: inner_lower,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[inner_init].instrs.push(Instr::Assign {
        dst: skew_var.clone(),
        src: skew_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_init].instrs.push(Instr::Assign {
        dst: inner_var.clone(),
        src: inner_lower,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_init].term = Terminator::Goto(inner_header);

    let outer_for_limit = build_load(fn_ir, outer_var.clone());
    let skew_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: outer_for_limit,
            rhs: inner_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let skew_load = build_load(fn_ir, skew_var.clone());
    let inner_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: skew_load,
            rhs: skew_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[inner_header].term = Terminator::If {
        cond: inner_cond,
        then_bb: body_bb,
        else_bb: outer_step,
    };

    if emit_loop_iv_aliases(fn_ir, body_bb, scop, &loop_var_map).is_none()
        || emit_generic_body(fn_ir, body_bb, scop, &loop_var_map, skip_accessless_assigns).is_none()
    {
        return false;
    }
    fn_ir.blocks[body_bb].term = Terminator::Goto(inner_step);

    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let skew_load_for_step = build_load(fn_ir, skew_var.clone());
    let next_skew = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: skew_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let inner_load_for_step = build_load(fn_ir, inner_var.clone());
    let next_inner = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: inner_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[inner_step].instrs.push(Instr::Assign {
        dst: skew_var,
        src: next_skew,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_step].instrs.push(Instr::Assign {
        dst: inner_var,
        src: next_inner,
        span: Span::dummy(),
    });
    fn_ir.blocks[inner_step].term = Terminator::Goto(inner_header);

    let outer_load_for_step = build_load(fn_ir, outer_var.clone());
    let next_outer = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: outer_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_step].instrs.push(Instr::Assign {
        dst: outer_var,
        src: next_outer,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer_step].term = Terminator::Goto(outer_header);

    for bid in &lp.body {
        fn_ir.blocks[*bid].instrs.clear();
        fn_ir.blocks[*bid].term = Terminator::Goto(exit_bb);
    }

    true
}

fn rebuild_generic_tiled_2d_loop_nest(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
) -> bool {
    let (Some(tile_rows), Some(tile_cols)) = (
        schedule.tile_rows.filter(|size| *size > 0),
        schedule.tile_cols.filter(|size| *size > 0),
    ) else {
        return false;
    };

    let ordered_names = ordered_dimension_names(schedule, scop);
    let mut dims = Vec::with_capacity(ordered_names.len());
    for name in ordered_names {
        let Some(dim) = scop.dimensions.iter().find(|dim| dim.iv_name == name) else {
            return false;
        };
        dims.push(dim.clone());
    }
    if dims.len() != 2 {
        return false;
    }
    let row_dim = &dims[0];
    let col_dim = &dims[1];

    let row_var = generated_iv_name(lp.header, &row_dim.iv_name);
    let col_var = generated_iv_name(lp.header, &col_dim.iv_name);
    let tile_row_var = generated_tile_iv_name(lp.header, &row_dim.iv_name);
    let tile_col_var = generated_tile_iv_name(lp.header, &col_dim.iv_name);

    let mut loop_var_map = FxHashMap::default();
    loop_var_map.insert(row_dim.iv_name.clone(), row_var.clone());
    loop_var_map.insert(col_dim.iv_name.clone(), col_var.clone());

    let Some(row_lower) = materialize_affine_expr(fn_ir, &row_dim.lower_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(row_upper) = materialize_affine_expr(fn_ir, &row_dim.upper_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(col_lower) = materialize_affine_expr(fn_ir, &col_dim.lower_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(col_upper) = materialize_affine_expr(fn_ir, &col_dim.upper_bound, &loop_var_map)
    else {
        return false;
    };

    for (dst, src) in [
        (tile_row_var.clone(), row_lower),
        (tile_col_var.clone(), col_lower),
        (row_var.clone(), row_lower),
        (col_var.clone(), col_lower),
    ] {
        fn_ir.blocks[preheader].instrs.push(Instr::Assign {
            dst,
            src,
            span: Span::dummy(),
        });
    }

    let outer_row_header = fn_ir.add_block();
    let outer_col_init = fn_ir.add_block();
    let outer_col_header = fn_ir.add_block();
    let row_init = fn_ir.add_block();
    let row_header = fn_ir.add_block();
    let col_init = fn_ir.add_block();
    let col_header = fn_ir.add_block();
    let body_bb = fn_ir.add_block();
    let col_step = fn_ir.add_block();
    let row_step = fn_ir.add_block();
    let outer_col_step = fn_ir.add_block();
    let outer_row_step = fn_ir.add_block();

    fn_ir.blocks[preheader].term = Terminator::Goto(outer_row_header);

    let tile_row_load = build_load(fn_ir, tile_row_var.clone());
    let outer_row_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: tile_row_load,
            rhs: row_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_row_header].term = Terminator::If {
        cond: outer_row_cond,
        then_bb: outer_col_init,
        else_bb: exit_bb,
    };

    let Some(col_lower_reload) =
        materialize_affine_expr(fn_ir, &col_dim.lower_bound, &loop_var_map)
    else {
        return false;
    };
    fn_ir.blocks[outer_col_init].instrs.push(Instr::Assign {
        dst: tile_col_var.clone(),
        src: col_lower_reload,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer_col_init].term = Terminator::Goto(outer_col_header);

    let tile_col_load = build_load(fn_ir, tile_col_var.clone());
    let outer_col_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: tile_col_load,
            rhs: col_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_col_header].term = Terminator::If {
        cond: outer_col_cond,
        then_bb: row_init,
        else_bb: outer_row_step,
    };

    let tile_row_for_row_init = build_load(fn_ir, tile_row_var.clone());
    fn_ir.blocks[row_init].instrs.push(Instr::Assign {
        dst: row_var.clone(),
        src: tile_row_for_row_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[row_init].term = Terminator::Goto(row_header);

    let row_load_a = build_load(fn_ir, row_var.clone());
    let row_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: row_load_a,
            rhs: row_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile_row_for_limit = build_load(fn_ir, tile_row_var.clone());
    let tile_row_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_rows - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_row_for_limit,
            rhs: tile_row_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_load_b = build_load(fn_ir, row_var.clone());
    let row_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: row_load_b,
            rhs: row_tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let row_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: row_cond_a,
            rhs: row_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[row_header].term = Terminator::If {
        cond: row_cond,
        then_bb: col_init,
        else_bb: outer_col_step,
    };

    let tile_col_for_col_init = build_load(fn_ir, tile_col_var.clone());
    fn_ir.blocks[col_init].instrs.push(Instr::Assign {
        dst: col_var.clone(),
        src: tile_col_for_col_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[col_init].term = Terminator::Goto(col_header);

    let col_load_a = build_load(fn_ir, col_var.clone());
    let col_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: col_load_a,
            rhs: col_upper,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile_col_for_limit = build_load(fn_ir, tile_col_var.clone());
    let tile_col_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_cols - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_col_for_limit,
            rhs: tile_col_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_load_b = build_load(fn_ir, col_var.clone());
    let col_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: col_load_b,
            rhs: col_tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let col_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: col_cond_a,
            rhs: col_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[col_header].term = Terminator::If {
        cond: col_cond,
        then_bb: body_bb,
        else_bb: row_step,
    };

    if emit_loop_iv_aliases(fn_ir, body_bb, scop, &loop_var_map).is_none()
        || emit_generic_body(fn_ir, body_bb, scop, &loop_var_map, skip_accessless_assigns).is_none()
    {
        return false;
    }
    fn_ir.blocks[body_bb].term = Terminator::Goto(col_step);

    let col_load_for_step = build_load(fn_ir, col_var.clone());
    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_col = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: col_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[col_step].instrs.push(Instr::Assign {
        dst: col_var,
        src: next_col,
        span: Span::dummy(),
    });
    fn_ir.blocks[col_step].term = Terminator::Goto(col_header);

    let row_load_for_step = build_load(fn_ir, row_var.clone());
    let next_row = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: row_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[row_step].instrs.push(Instr::Assign {
        dst: row_var,
        src: next_row,
        span: Span::dummy(),
    });
    fn_ir.blocks[row_step].term = Terminator::Goto(row_header);

    let tile_col_load_for_step = build_load(fn_ir, tile_col_var.clone());
    let tile_col_step_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_cols as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile_col = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_col_load_for_step,
            rhs: tile_col_step_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_col_step].instrs.push(Instr::Assign {
        dst: tile_col_var,
        src: next_tile_col,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer_col_step].term = Terminator::Goto(outer_col_header);

    let tile_row_load_for_step = build_load(fn_ir, tile_row_var.clone());
    let tile_row_step_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_rows as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile_row = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile_row_load_for_step,
            rhs: tile_row_step_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer_row_step].instrs.push(Instr::Assign {
        dst: tile_row_var,
        src: next_tile_row,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer_row_step].term = Terminator::Goto(outer_row_header);

    for bid in &lp.body {
        fn_ir.blocks[*bid].instrs.clear();
        fn_ir.blocks[*bid].term = Terminator::Goto(exit_bb);
    }

    true
}

fn rebuild_generic_tiled_3d_loop_nest(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
) -> bool {
    let (Some(tile_depth), Some(tile_rows), Some(tile_cols)) = (
        schedule.tile_depth.filter(|size| *size > 0),
        schedule.tile_rows.filter(|size| *size > 0),
        schedule.tile_cols.filter(|size| *size > 0),
    ) else {
        return false;
    };

    let ordered_names = ordered_dimension_names(schedule, scop);
    let mut dims = Vec::with_capacity(ordered_names.len());
    for name in ordered_names {
        let Some(dim) = scop.dimensions.iter().find(|dim| dim.iv_name == name) else {
            return false;
        };
        dims.push(dim.clone());
    }
    if dims.len() != 3 {
        return false;
    }

    let dim0 = &dims[0];
    let dim1 = &dims[1];
    let dim2 = &dims[2];

    let var0 = generated_iv_name(lp.header, &dim0.iv_name);
    let var1 = generated_iv_name(lp.header, &dim1.iv_name);
    let var2 = generated_iv_name(lp.header, &dim2.iv_name);
    let tile_var0 = generated_tile_iv_name(lp.header, &dim0.iv_name);
    let tile_var1 = generated_tile_iv_name(lp.header, &dim1.iv_name);
    let tile_var2 = generated_tile_iv_name(lp.header, &dim2.iv_name);

    let mut loop_var_map = FxHashMap::default();
    loop_var_map.insert(dim0.iv_name.clone(), var0.clone());
    loop_var_map.insert(dim1.iv_name.clone(), var1.clone());
    loop_var_map.insert(dim2.iv_name.clone(), var2.clone());

    let Some(lower0) = materialize_affine_expr(fn_ir, &dim0.lower_bound, &loop_var_map) else {
        return false;
    };
    let Some(upper0) = materialize_affine_expr(fn_ir, &dim0.upper_bound, &loop_var_map) else {
        return false;
    };
    let Some(lower1) = materialize_affine_expr(fn_ir, &dim1.lower_bound, &loop_var_map) else {
        return false;
    };
    let Some(upper1) = materialize_affine_expr(fn_ir, &dim1.upper_bound, &loop_var_map) else {
        return false;
    };
    let Some(lower2) = materialize_affine_expr(fn_ir, &dim2.lower_bound, &loop_var_map) else {
        return false;
    };
    let Some(upper2) = materialize_affine_expr(fn_ir, &dim2.upper_bound, &loop_var_map) else {
        return false;
    };

    for (dst, src) in [
        (tile_var0.clone(), lower0),
        (tile_var1.clone(), lower1),
        (tile_var2.clone(), lower2),
        (var0.clone(), lower0),
        (var1.clone(), lower1),
        (var2.clone(), lower2),
    ] {
        fn_ir.blocks[preheader].instrs.push(Instr::Assign {
            dst,
            src,
            span: Span::dummy(),
        });
    }

    let outer0_header = fn_ir.add_block();
    let outer1_init = fn_ir.add_block();
    let outer1_header = fn_ir.add_block();
    let outer2_init = fn_ir.add_block();
    let outer2_header = fn_ir.add_block();
    let dim0_init = fn_ir.add_block();
    let dim0_header = fn_ir.add_block();
    let dim1_init = fn_ir.add_block();
    let dim1_header = fn_ir.add_block();
    let dim2_init = fn_ir.add_block();
    let dim2_header = fn_ir.add_block();
    let body_bb = fn_ir.add_block();
    let dim2_step = fn_ir.add_block();
    let dim1_step = fn_ir.add_block();
    let dim0_step = fn_ir.add_block();
    let outer2_step = fn_ir.add_block();
    let outer1_step = fn_ir.add_block();
    let outer0_step = fn_ir.add_block();

    fn_ir.blocks[preheader].term = Terminator::Goto(outer0_header);

    let outer0_load = build_load(fn_ir, tile_var0.clone());
    let outer0_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: outer0_load,
            rhs: upper0,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer0_header].term = Terminator::If {
        cond: outer0_cond,
        then_bb: outer1_init,
        else_bb: exit_bb,
    };

    let Some(lower1_reload) = materialize_affine_expr(fn_ir, &dim1.lower_bound, &loop_var_map)
    else {
        return false;
    };
    let Some(lower2_reload) = materialize_affine_expr(fn_ir, &dim2.lower_bound, &loop_var_map)
    else {
        return false;
    };
    fn_ir.blocks[outer1_init].instrs.push(Instr::Assign {
        dst: tile_var1.clone(),
        src: lower1_reload,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer1_init].instrs.push(Instr::Assign {
        dst: tile_var2.clone(),
        src: lower2_reload,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer1_init].term = Terminator::Goto(outer1_header);

    let outer1_load = build_load(fn_ir, tile_var1.clone());
    let outer1_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: outer1_load,
            rhs: upper1,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer1_header].term = Terminator::If {
        cond: outer1_cond,
        then_bb: outer2_init,
        else_bb: outer0_step,
    };

    let Some(lower2_reload_b) = materialize_affine_expr(fn_ir, &dim2.lower_bound, &loop_var_map)
    else {
        return false;
    };
    fn_ir.blocks[outer2_init].instrs.push(Instr::Assign {
        dst: tile_var2.clone(),
        src: lower2_reload_b,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer2_init].term = Terminator::Goto(outer2_header);

    let outer2_load = build_load(fn_ir, tile_var2.clone());
    let outer2_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: outer2_load,
            rhs: upper2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer2_header].term = Terminator::If {
        cond: outer2_cond,
        then_bb: dim0_init,
        else_bb: outer1_step,
    };

    let tile0_for_init = build_load(fn_ir, tile_var0.clone());
    fn_ir.blocks[dim0_init].instrs.push(Instr::Assign {
        dst: var0.clone(),
        src: tile0_for_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim0_init].term = Terminator::Goto(dim0_header);

    let dim0_load_a = build_load(fn_ir, var0.clone());
    let dim0_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim0_load_a,
            rhs: upper0,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile0_for_limit = build_load(fn_ir, tile_var0.clone());
    let tile0_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_depth - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim0_tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile0_for_limit,
            rhs: tile0_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim0_load_b = build_load(fn_ir, var0.clone());
    let dim0_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim0_load_b,
            rhs: dim0_tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim0_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: dim0_cond_a,
            rhs: dim0_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim0_header].term = Terminator::If {
        cond: dim0_cond,
        then_bb: dim1_init,
        else_bb: outer2_step,
    };

    let tile1_for_init = build_load(fn_ir, tile_var1.clone());
    fn_ir.blocks[dim1_init].instrs.push(Instr::Assign {
        dst: var1.clone(),
        src: tile1_for_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim1_init].term = Terminator::Goto(dim1_header);

    let dim1_load_a = build_load(fn_ir, var1.clone());
    let dim1_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim1_load_a,
            rhs: upper1,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile1_for_limit = build_load(fn_ir, tile_var1.clone());
    let tile1_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_rows - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim1_tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile1_for_limit,
            rhs: tile1_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim1_load_b = build_load(fn_ir, var1.clone());
    let dim1_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim1_load_b,
            rhs: dim1_tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim1_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: dim1_cond_a,
            rhs: dim1_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim1_header].term = Terminator::If {
        cond: dim1_cond,
        then_bb: dim2_init,
        else_bb: dim0_step,
    };

    let tile2_for_init = build_load(fn_ir, tile_var2.clone());
    fn_ir.blocks[dim2_init].instrs.push(Instr::Assign {
        dst: var2.clone(),
        src: tile2_for_init,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim2_init].term = Terminator::Goto(dim2_header);

    let dim2_load_a = build_load(fn_ir, var2.clone());
    let dim2_cond_a = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim2_load_a,
            rhs: upper2,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let tile2_for_limit = build_load(fn_ir, tile_var2.clone());
    let tile2_span = fn_ir.add_value(
        ValueKind::Const(Lit::Int((tile_cols - 1) as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim2_tile_limit = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile2_for_limit,
            rhs: tile2_span,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim2_load_b = build_load(fn_ir, var2.clone());
    let dim2_cond_b = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Le,
            lhs: dim2_load_b,
            rhs: dim2_tile_limit,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let dim2_cond = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::And,
            lhs: dim2_cond_a,
            rhs: dim2_cond_b,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim2_header].term = Terminator::If {
        cond: dim2_cond,
        then_bb: body_bb,
        else_bb: dim1_step,
    };

    if emit_loop_iv_aliases(fn_ir, body_bb, scop, &loop_var_map).is_none()
        || emit_generic_body(fn_ir, body_bb, scop, &loop_var_map, skip_accessless_assigns).is_none()
    {
        return false;
    }
    fn_ir.blocks[body_bb].term = Terminator::Goto(dim2_step);

    let one = fn_ir.add_value(
        ValueKind::Const(Lit::Int(1)),
        Span::dummy(),
        Facts::empty(),
        None,
    );

    let dim2_load_for_step = build_load(fn_ir, var2.clone());
    let next_dim2 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: dim2_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim2_step].instrs.push(Instr::Assign {
        dst: var2,
        src: next_dim2,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim2_step].term = Terminator::Goto(dim2_header);

    let dim1_load_for_step = build_load(fn_ir, var1.clone());
    let next_dim1 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: dim1_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim1_step].instrs.push(Instr::Assign {
        dst: var1,
        src: next_dim1,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim1_step].term = Terminator::Goto(dim1_header);

    let dim0_load_for_step = build_load(fn_ir, var0.clone());
    let next_dim0 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: dim0_load_for_step,
            rhs: one,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[dim0_step].instrs.push(Instr::Assign {
        dst: var0,
        src: next_dim0,
        span: Span::dummy(),
    });
    fn_ir.blocks[dim0_step].term = Terminator::Goto(dim0_header);

    let tile2_load_for_step = build_load(fn_ir, tile_var2.clone());
    let tile2_step_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_cols as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile2 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile2_load_for_step,
            rhs: tile2_step_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer2_step].instrs.push(Instr::Assign {
        dst: tile_var2,
        src: next_tile2,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer2_step].term = Terminator::Goto(outer2_header);

    let tile1_load_for_step = build_load(fn_ir, tile_var1.clone());
    let tile1_step_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_rows as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile1 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile1_load_for_step,
            rhs: tile1_step_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer1_step].instrs.push(Instr::Assign {
        dst: tile_var1,
        src: next_tile1,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer1_step].term = Terminator::Goto(outer1_header);

    let tile0_load_for_step = build_load(fn_ir, tile_var0.clone());
    let tile0_step_val = fn_ir.add_value(
        ValueKind::Const(Lit::Int(tile_depth as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next_tile0 = fn_ir.add_value(
        ValueKind::Binary {
            op: BinOp::Add,
            lhs: tile0_load_for_step,
            rhs: tile0_step_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[outer0_step].instrs.push(Instr::Assign {
        dst: tile_var0,
        src: next_tile0,
        span: Span::dummy(),
    });
    fn_ir.blocks[outer0_step].term = Terminator::Goto(outer0_header);

    for bid in &lp.body {
        fn_ir.blocks[*bid].instrs.clear();
        fn_ir.blocks[*bid].term = Terminator::Goto(exit_bb);
    }

    true
}

fn scop_subset_with_stmt(scop: &ScopRegion, stmt: PolyStmt) -> ScopRegion {
    ScopRegion {
        header: scop.header,
        latch: scop.latch,
        exits: scop.exits.clone(),
        dimensions: scop.dimensions.clone(),
        iteration_space: scop.iteration_space.clone(),
        parameters: scop.parameters.clone(),
        statements: vec![stmt],
    }
}

#[allow(clippy::too_many_arguments)]
fn rebuild_generic_fissioned_sequence(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
    preheader: usize,
    exit_bb: usize,
    skip_accessless_assigns: bool,
    lower_one: fn(&mut FnIR, &LoopInfo, &ScopRegion, &SchedulePlan, usize, usize, bool) -> bool,
) -> bool {
    let stmts = scop
        .statements
        .iter()
        .filter(|stmt| !stmt.accesses.is_empty())
        .cloned()
        .collect::<Vec<_>>();
    if stmts.len() <= 1 {
        return false;
    }
    let mut next_preheader = preheader;
    for (idx, stmt) in stmts.into_iter().enumerate() {
        let next_exit = if idx + 1
            == scop
                .statements
                .iter()
                .filter(|stmt| !stmt.accesses.is_empty())
                .count()
        {
            exit_bb
        } else {
            fn_ir.add_block()
        };
        let subset = scop_subset_with_stmt(scop, stmt);
        if !lower_one(
            fn_ir,
            lp,
            &subset,
            schedule,
            next_preheader,
            next_exit,
            skip_accessless_assigns,
        ) {
            return false;
        }
        next_preheader = next_exit;
    }
    true
}

fn build_loop_level(
    fn_ir: &mut FnIR,
    dims: &[super::LoopDimension],
    level: usize,
    scop: &ScopRegion,
    loop_var_map: &FxHashMap<String, String>,
    loop_exit_bb: usize,
    skip_accessless_assigns: bool,
) -> Option<usize> {
    let dim = dims.get(level)?;
    let init_bb = fn_ir.add_block();
    let header_bb = fn_ir.add_block();
    let step_bb = fn_ir.add_block();

    let after_body_bb = if level + 1 == dims.len() {
        let body_bb = fn_ir.add_block();
        emit_loop_iv_aliases(fn_ir, body_bb, scop, loop_var_map)?;
        emit_generic_body(fn_ir, body_bb, scop, loop_var_map, skip_accessless_assigns)?;
        fn_ir.blocks[body_bb].term = Terminator::Goto(step_bb);
        body_bb
    } else {
        build_loop_level(
            fn_ir,
            dims,
            level + 1,
            scop,
            loop_var_map,
            step_bb,
            skip_accessless_assigns,
        )?
    };

    let init_val = materialize_affine_expr(fn_ir, &dim.lower_bound, loop_var_map)?;
    fn_ir.blocks[init_bb].instrs.push(Instr::Assign {
        dst: loop_var_map.get(&dim.iv_name)?.clone(),
        src: init_val,
        span: Span::dummy(),
    });
    fn_ir.blocks[init_bb].term = Terminator::Goto(header_bb);

    let iv_load = build_load(fn_ir, loop_var_map.get(&dim.iv_name)?.clone());
    let bound_val = materialize_affine_expr(fn_ir, &dim.upper_bound, loop_var_map)?;
    let cond = fn_ir.add_value(
        ValueKind::Binary {
            op: if dim.step >= 0 { BinOp::Le } else { BinOp::Ge },
            lhs: iv_load,
            rhs: bound_val,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[header_bb].term = Terminator::If {
        cond,
        then_bb: after_body_bb,
        else_bb: loop_exit_bb,
    };

    let step_load = build_load(fn_ir, loop_var_map.get(&dim.iv_name)?.clone());
    let step_mag = fn_ir.add_value(
        ValueKind::Const(Lit::Int(dim.step.unsigned_abs() as i64)),
        Span::dummy(),
        Facts::empty(),
        None,
    );
    let next = fn_ir.add_value(
        ValueKind::Binary {
            op: if dim.step >= 0 {
                BinOp::Add
            } else {
                BinOp::Sub
            },
            lhs: step_load,
            rhs: step_mag,
        },
        Span::dummy(),
        Facts::empty(),
        None,
    );
    fn_ir.blocks[step_bb].instrs.push(Instr::Assign {
        dst: loop_var_map.get(&dim.iv_name)?.clone(),
        src: next,
        span: Span::dummy(),
    });
    fn_ir.blocks[step_bb].term = Terminator::Goto(header_bb);

    Some(init_bb)
}

fn emit_loop_iv_aliases(
    fn_ir: &mut FnIR,
    body_bb: usize,
    scop: &ScopRegion,
    loop_var_map: &FxHashMap<String, String>,
) -> Option<()> {
    for dim in &scop.dimensions {
        let generated = loop_var_map.get(&dim.iv_name)?;
        if generated == &dim.iv_name {
            continue;
        }
        let src = build_load(fn_ir, generated.clone());
        fn_ir.blocks[body_bb].instrs.push(Instr::Assign {
            dst: dim.iv_name.clone(),
            src,
            span: Span::dummy(),
        });
    }
    Some(())
}

fn emit_generic_body(
    fn_ir: &mut FnIR,
    body_bb: usize,
    scop: &ScopRegion,
    loop_var_map: &FxHashMap<String, String>,
    skip_accessless_assigns: bool,
) -> Option<()> {
    let loop_iv_names = scop
        .dimensions
        .iter()
        .map(|dim| dim.iv_name.as_str())
        .collect::<FxHashSet<_>>();
    let mut memo = FxHashMap::default();
    for (idx, stmt) in scop.statements.iter().enumerate() {
        if let PolyStmtKind::Assign { dst } = &stmt.kind
            && loop_iv_names.contains(dst.as_str())
        {
            continue;
        }
        if stmt_is_progress_assign(fn_ir, scop, stmt) {
            continue;
        }
        if skip_accessless_assigns
            && matches!(&stmt.kind, PolyStmtKind::Assign { .. })
            && stmt.accesses.is_empty()
        {
            let PolyStmtKind::Assign { dst } = &stmt.kind else {
                unreachable!();
            };
            let needed_later = scop.statements[idx + 1..]
                .iter()
                .any(|later| stmt_mentions_var(fn_ir, later, dst));
            if !needed_later {
                continue;
            }
        }
        emit_generic_stmt(fn_ir, body_bb, stmt, loop_var_map, &mut memo)?;
    }
    Some(())
}

fn stmt_is_progress_assign(fn_ir: &FnIR, scop: &ScopRegion, stmt: &PolyStmt) -> bool {
    let (PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root) else {
        return false;
    };
    if !stmt.accesses.is_empty() {
        return false;
    }
    let root = resolve_scop_local_source(fn_ir, scop, expr_root);
    match &fn_ir.values[root].kind {
        ValueKind::Binary {
            op: BinOp::Add | BinOp::Sub,
            lhs,
            rhs,
        } => {
            let lhs = resolve_scop_local_source(fn_ir, scop, *lhs);
            let rhs = resolve_scop_local_source(fn_ir, scop, *rhs);
            let lhs_self = is_same_named_value(fn_ir, lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, rhs, dst);
            let lhs_const = matches!(fn_ir.values[lhs].kind, ValueKind::Const(Lit::Int(_)));
            let rhs_const = matches!(fn_ir.values[rhs].kind, ValueKind::Const(Lit::Int(_)));
            (lhs_self && rhs_const) || (rhs_self && lhs_const)
        }
        _ => false,
    }
}

fn stmt_mentions_var(fn_ir: &FnIR, stmt: &PolyStmt, var: &str) -> bool {
    let mut seen = FxHashSet::default();
    if stmt
        .expr_root
        .is_some_and(|root| expr_mentions_var(fn_ir, root, var, &mut seen))
    {
        return true;
    }
    match &stmt.kind {
        PolyStmtKind::Assign { .. } | PolyStmtKind::Eval => false,
        PolyStmtKind::Store { base, subscripts } => {
            expr_mentions_var(fn_ir, *base, var, &mut seen)
                || subscripts
                    .iter()
                    .any(|sub| expr_mentions_var(fn_ir, *sub, var, &mut seen))
        }
    }
}

fn emit_generic_stmt(
    fn_ir: &mut FnIR,
    body_bb: usize,
    stmt: &PolyStmt,
    loop_var_map: &FxHashMap<String, String>,
    memo: &mut FxHashMap<ValueId, ValueId>,
) -> Option<()> {
    let span = stmt
        .expr_root
        .map(|root| fn_ir.values[root].span)
        .unwrap_or_else(Span::dummy);
    match (&stmt.kind, stmt.expr_root) {
        (PolyStmtKind::Assign { dst }, Some(root)) => {
            let src = clone_value_for_generic(fn_ir, root, loop_var_map, memo)?;
            fn_ir.blocks[body_bb].instrs.push(Instr::Assign {
                dst: dst.clone(),
                src,
                span,
            });
            Some(())
        }
        (PolyStmtKind::Eval, Some(root)) => {
            let val = clone_value_for_generic(fn_ir, root, loop_var_map, memo)?;
            fn_ir.blocks[body_bb].instrs.push(Instr::Eval { val, span });
            Some(())
        }
        (PolyStmtKind::Store { base, subscripts }, Some(root)) => {
            let base = clone_value_for_generic(fn_ir, *base, loop_var_map, memo)?;
            let value = clone_value_for_generic(fn_ir, root, loop_var_map, memo)?;
            let subscripts = subscripts
                .iter()
                .map(|sub| clone_value_for_generic(fn_ir, *sub, loop_var_map, memo))
                .collect::<Option<Vec<_>>>()?;
            match subscripts.as_slice() {
                [idx] => fn_ir.blocks[body_bb].instrs.push(Instr::StoreIndex1D {
                    base,
                    idx: *idx,
                    val: value,
                    is_safe: false,
                    is_na_safe: false,
                    is_vector: false,
                    span,
                }),
                [r, c] => fn_ir.blocks[body_bb].instrs.push(Instr::StoreIndex2D {
                    base,
                    r: *r,
                    c: *c,
                    val: value,
                    span,
                }),
                [i, j, k] => fn_ir.blocks[body_bb].instrs.push(Instr::StoreIndex3D {
                    base,
                    i: *i,
                    j: *j,
                    k: *k,
                    val: value,
                    span,
                }),
                _ => return None,
            }
            Some(())
        }
        _ => None,
    }
}

fn build_load(fn_ir: &mut FnIR, var: String) -> ValueId {
    fn_ir.add_value(
        ValueKind::Load { var: var.clone() },
        Span::dummy(),
        Facts::empty(),
        Some(var),
    )
}

fn materialize_symbol_value(
    fn_ir: &mut FnIR,
    symbol: &AffineSymbol,
    loop_var_map: &FxHashMap<String, String>,
) -> ValueId {
    match symbol {
        AffineSymbol::LoopIv(name) => build_load(
            fn_ir,
            loop_var_map
                .get(name)
                .cloned()
                .unwrap_or_else(|| name.clone()),
        ),
        AffineSymbol::Param(name) | AffineSymbol::Invariant(name) => {
            if let Some(index) = fn_ir.params.iter().position(|param| param == name) {
                fn_ir.add_value(
                    ValueKind::Param { index },
                    Span::dummy(),
                    Facts::empty(),
                    Some(name.clone()),
                )
            } else {
                build_load(fn_ir, name.clone())
            }
        }
        AffineSymbol::Length(name) => {
            let base = if let Some(index) = fn_ir.params.iter().position(|param| param == name) {
                fn_ir.add_value(
                    ValueKind::Param { index },
                    Span::dummy(),
                    Facts::empty(),
                    Some(name.clone()),
                )
            } else {
                build_load(fn_ir, name.clone())
            };
            fn_ir.add_value(ValueKind::Len { base }, Span::dummy(), Facts::empty(), None)
        }
    }
}

fn materialize_affine_expr(
    fn_ir: &mut FnIR,
    expr: &AffineExpr,
    loop_var_map: &FxHashMap<String, String>,
) -> Option<ValueId> {
    let mut acc: Option<ValueId> = None;
    if expr.constant != 0 || expr.terms.is_empty() {
        acc = Some(fn_ir.add_value(
            ValueKind::Const(Lit::Int(expr.constant)),
            Span::dummy(),
            Facts::empty(),
            None,
        ));
    }
    for (symbol, coeff) in &expr.terms {
        let base = materialize_symbol_value(fn_ir, symbol, loop_var_map);
        let term = if *coeff == 1 {
            base
        } else {
            let coeff_val = fn_ir.add_value(
                ValueKind::Const(Lit::Int(*coeff)),
                Span::dummy(),
                Facts::empty(),
                None,
            );
            fn_ir.add_value(
                ValueKind::Binary {
                    op: BinOp::Mul,
                    lhs: base,
                    rhs: coeff_val,
                },
                Span::dummy(),
                Facts::empty(),
                None,
            )
        };
        acc = Some(match acc {
            None => term,
            Some(lhs) => fn_ir.add_value(
                ValueKind::Binary {
                    op: BinOp::Add,
                    lhs,
                    rhs: term,
                },
                Span::dummy(),
                Facts::empty(),
                None,
            ),
        });
    }
    acc
}

fn clone_value_for_generic(
    fn_ir: &mut FnIR,
    root: ValueId,
    loop_var_map: &FxHashMap<String, String>,
    memo: &mut FxHashMap<ValueId, ValueId>,
) -> Option<ValueId> {
    if let Some(mapped) = memo.get(&root) {
        return Some(*mapped);
    }
    let value = match fn_ir.values[root].kind.clone() {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => root,
        ValueKind::Load { var } => {
            build_load(fn_ir, loop_var_map.get(&var).cloned().unwrap_or(var))
        }
        ValueKind::Phi { .. } => {
            let var = fn_ir.values[root].origin_var.clone()?;
            build_load(fn_ir, loop_var_map.get(&var).cloned().unwrap_or(var))
        }
        ValueKind::Len { base } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Len { base },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Indices { base } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Indices { base },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Range { start, end } => {
            let start = clone_value_for_generic(fn_ir, start, loop_var_map, memo)?;
            let end = clone_value_for_generic(fn_ir, end, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Range { start, end },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Binary { op, lhs, rhs } => {
            let lhs = clone_value_for_generic(fn_ir, lhs, loop_var_map, memo)?;
            let rhs = clone_value_for_generic(fn_ir, rhs, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Binary { op, lhs, rhs },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Unary { op, rhs } => {
            let rhs = clone_value_for_generic(fn_ir, rhs, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Unary { op, rhs },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let args = args
                .iter()
                .map(|arg| clone_value_for_generic(fn_ir, *arg, loop_var_map, memo))
                .collect::<Option<Vec<_>>>()?;
            fn_ir.add_value(
                ValueKind::Call {
                    callee,
                    args,
                    names,
                },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Intrinsic { op, args } => {
            let args = args
                .iter()
                .map(|arg| clone_value_for_generic(fn_ir, *arg, loop_var_map, memo))
                .collect::<Option<Vec<_>>>()?;
            fn_ir.add_value(
                ValueKind::Intrinsic { op, args },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::RecordLit { fields } => {
            let fields = fields
                .iter()
                .map(|(name, value)| {
                    Some((
                        name.clone(),
                        clone_value_for_generic(fn_ir, *value, loop_var_map, memo)?,
                    ))
                })
                .collect::<Option<Vec<_>>>()?;
            fn_ir.add_value(
                ValueKind::RecordLit { fields },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::FieldGet { base, field } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::FieldGet { base, field },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::FieldSet { base, field, value } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            let value = clone_value_for_generic(fn_ir, value, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::FieldSet { base, field, value },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            let idx = clone_value_for_generic(fn_ir, idx, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Index1D {
                    base,
                    idx,
                    is_safe,
                    is_na_safe,
                },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Index2D { base, r, c } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            let r = clone_value_for_generic(fn_ir, r, loop_var_map, memo)?;
            let c = clone_value_for_generic(fn_ir, c, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Index2D { base, r, c },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Index3D { base, i, j, k } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            let i = clone_value_for_generic(fn_ir, i, loop_var_map, memo)?;
            let j = clone_value_for_generic(fn_ir, j, loop_var_map, memo)?;
            let k = clone_value_for_generic(fn_ir, k, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Index3D { base, i, j, k },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
    };
    memo.insert(root, value);
    Some(value)
}
