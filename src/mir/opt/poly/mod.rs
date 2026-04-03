mod access;
mod affine;
mod backend;
mod codegen;
mod codegen_generic;
mod cost;
mod dependence_backend;
mod isl;
mod schedule;
mod scop;
mod tree;
mod verify;

use crate::mir::FnIR;
use crate::mir::opt::loop_analysis::LoopAnalyzer;
use std::cmp::Reverse;

use backend::make_backend_from_env;
use codegen::lower_schedule_tree;
pub(crate) use codegen_generic::is_generated_loop_iv_name as is_generated_poly_loop_var_name;
use cost::estimate_schedule_cost;
use dependence_backend::{DependenceState, make_dependence_backend};
use schedule::SchedulePlanKind;
pub use scop::{LoopDimension, PolyStmt, PolyStmtKind, ScopExtractionFailure, ScopRegion};
use verify::{
    build_certificate, dump_certificate, dump_rejection, dump_root_from_env, format_reject_reason,
    format_scop_summary,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct PolyStats {
    pub loops_seen: usize,
    pub scops_detected: usize,
    pub rejected_cfg_shape: usize,
    pub rejected_non_affine: usize,
    pub rejected_effects: usize,
    pub affine_stmt_count: usize,
    pub access_relation_count: usize,
    pub dependence_solved: usize,
    pub schedule_attempted: usize,
    pub schedule_applied: usize,
    pub schedule_attempted_identity: usize,
    pub schedule_attempted_interchange: usize,
    pub schedule_attempted_skew2d: usize,
    pub schedule_attempted_tile1d: usize,
    pub schedule_attempted_tile2d: usize,
    pub schedule_attempted_tile3d: usize,
    pub schedule_applied_identity: usize,
    pub schedule_applied_interchange: usize,
    pub schedule_applied_skew2d: usize,
    pub schedule_applied_tile1d: usize,
    pub schedule_applied_tile2d: usize,
    pub schedule_applied_tile3d: usize,
    pub schedule_auto_fuse_selected: usize,
    pub schedule_auto_fission_selected: usize,
    pub schedule_auto_skew2d_selected: usize,
    pub schedule_backend_hint_selected: usize,
}

impl PolyStats {
    fn record_reject(&mut self, reason: ScopExtractionFailure) {
        match reason {
            ScopExtractionFailure::UnsupportedCfgShape
            | ScopExtractionFailure::MissingInductionVar
            | ScopExtractionFailure::UnsupportedNestedLoop => self.rejected_cfg_shape += 1,
            ScopExtractionFailure::NonAffineLoopBound | ScopExtractionFailure::NonAffineAccess => {
                self.rejected_non_affine += 1
            }
            ScopExtractionFailure::EffectfulStatement => self.rejected_effects += 1,
        }
    }

    fn record_schedule_attempt(&mut self, kind: SchedulePlanKind) {
        match kind {
            SchedulePlanKind::Identity => self.schedule_attempted_identity += 1,
            SchedulePlanKind::Interchange => self.schedule_attempted_interchange += 1,
            SchedulePlanKind::Skew2D => self.schedule_attempted_skew2d += 1,
            SchedulePlanKind::Tile1D => self.schedule_attempted_tile1d += 1,
            SchedulePlanKind::Tile2D => self.schedule_attempted_tile2d += 1,
            SchedulePlanKind::Tile3D => self.schedule_attempted_tile3d += 1,
            SchedulePlanKind::None => {}
        }
    }

    fn record_schedule_applied(&mut self, kind: SchedulePlanKind) {
        match kind {
            SchedulePlanKind::Identity => self.schedule_applied_identity += 1,
            SchedulePlanKind::Interchange => self.schedule_applied_interchange += 1,
            SchedulePlanKind::Skew2D => self.schedule_applied_skew2d += 1,
            SchedulePlanKind::Tile1D => self.schedule_applied_tile1d += 1,
            SchedulePlanKind::Tile2D => self.schedule_applied_tile2d += 1,
            SchedulePlanKind::Tile3D => self.schedule_applied_tile3d += 1,
            SchedulePlanKind::None => {}
        }
    }
}

fn env_flag_enabled(key: &str) -> bool {
    std::env::var(key).ok().is_some_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

pub fn poly_enabled() -> bool {
    env_flag_enabled("RR_POLY_ENABLE")
}

pub fn poly_trace_enabled() -> bool {
    poly_enabled() && env_flag_enabled("RR_POLY_TRACE")
}

fn env_auto_mode(key: &str) -> bool {
    match std::env::var(key)
        .ok()
        .map(|value| value.trim().to_ascii_lowercase())
        .as_deref()
    {
        None | Some("auto") => true,
        Some(_) => false,
    }
}

pub fn analyze_with_stats(fn_ir: &FnIR) -> PolyStats {
    if !poly_enabled() {
        return PolyStats::default();
    }
    analyze_loops(fn_ir).0
}

pub fn run_hidden_poly_cli(args: &[String]) -> Option<i32> {
    isl::run_materialize_helper_from_cli(args)
}

pub fn optimize_with_stats(fn_ir: &mut FnIR) -> PolyStats {
    if !poly_enabled() {
        return PolyStats::default();
    }
    let mut stats = PolyStats::default();
    let mut changed = true;
    let mut iterations = 0usize;
    while changed && iterations < 8 {
        changed = false;
        iterations += 1;
        let (iter_stats, loops) = analyze_loops(fn_ir);
        stats.loops_seen += iter_stats.loops_seen;
        stats.scops_detected += iter_stats.scops_detected;
        stats.rejected_cfg_shape += iter_stats.rejected_cfg_shape;
        stats.rejected_non_affine += iter_stats.rejected_non_affine;
        stats.rejected_effects += iter_stats.rejected_effects;
        stats.affine_stmt_count += iter_stats.affine_stmt_count;
        stats.access_relation_count += iter_stats.access_relation_count;
        stats.dependence_solved += iter_stats.dependence_solved;
        stats.schedule_attempted += iter_stats.schedule_attempted;
        stats.schedule_attempted_identity += iter_stats.schedule_attempted_identity;
        stats.schedule_attempted_interchange += iter_stats.schedule_attempted_interchange;
        stats.schedule_attempted_skew2d += iter_stats.schedule_attempted_skew2d;
        stats.schedule_attempted_tile1d += iter_stats.schedule_attempted_tile1d;
        stats.schedule_attempted_tile2d += iter_stats.schedule_attempted_tile2d;
        stats.schedule_attempted_tile3d += iter_stats.schedule_attempted_tile3d;
        stats.schedule_applied_identity += iter_stats.schedule_applied_identity;
        stats.schedule_applied_interchange += iter_stats.schedule_applied_interchange;
        stats.schedule_applied_skew2d += iter_stats.schedule_applied_skew2d;
        stats.schedule_applied_tile1d += iter_stats.schedule_applied_tile1d;
        stats.schedule_applied_tile2d += iter_stats.schedule_applied_tile2d;
        stats.schedule_applied_tile3d += iter_stats.schedule_applied_tile3d;
        stats.schedule_auto_fuse_selected += iter_stats.schedule_auto_fuse_selected;
        stats.schedule_auto_fission_selected += iter_stats.schedule_auto_fission_selected;
        stats.schedule_auto_skew2d_selected += iter_stats.schedule_auto_skew2d_selected;
        stats.schedule_backend_hint_selected += iter_stats.schedule_backend_hint_selected;

        for lp in &loops {
            let Ok(scop) = scop::extract_scop_region(fn_ir, lp, &loops) else {
                continue;
            };
            let backend = make_backend_from_env();
            let dep_backend = make_dependence_backend(backend.as_ref());
            let deps = dep_backend.analyze(fn_ir, &scop);
            let schedule_tree = backend.build_schedule_tree(&scop, &deps);
            let schedule = schedule_tree.to_primary_plan();
            if let Some(root) = dump_root_from_env() {
                let cert = build_certificate(
                    fn_ir,
                    &scop,
                    deps.clone(),
                    schedule.clone(),
                    schedule_tree.clone(),
                );
                dump_certificate(&root, &cert, &scop);
            }
            let emitted = lower_schedule_tree(fn_ir, lp, &scop, &schedule_tree).emitted;
            if emitted {
                stats.schedule_applied += 1;
                stats.record_schedule_applied(schedule.kind);
                changed = true;
                break;
            }
        }
    }
    stats
}

fn analyze_loops(fn_ir: &FnIR) -> (PolyStats, Vec<crate::mir::opt::loop_analysis::LoopInfo>) {
    let mut stats = PolyStats::default();
    let mut loops = LoopAnalyzer::new(fn_ir).find_loops();
    loops.sort_by_key(|lp| (Reverse(lp.body.len()), lp.header));
    stats.loops_seen = loops.len();

    for lp in &loops {
        match scop::extract_scop_region(fn_ir, lp, &loops) {
            Ok(scop) => {
                stats.scops_detected += 1;
                stats.affine_stmt_count += scop.statements.len();
                stats.access_relation_count += scop
                    .statements
                    .iter()
                    .map(|stmt| stmt.accesses.len())
                    .sum::<usize>();

                let backend = make_backend_from_env();
                let dep_backend = make_dependence_backend(backend.as_ref());
                let deps = dep_backend.analyze(fn_ir, &scop);
                if deps.has_explicit_relations() || deps.derived_state() != DependenceState::Unknown
                {
                    stats.dependence_solved += 1;
                }
                let schedule_tree = backend.build_schedule_tree(&scop, &deps);
                let schedule = schedule_tree.to_primary_plan();
                if schedule_tree.primary_kind() != SchedulePlanKind::None {
                    stats.schedule_attempted += 1;
                    stats.record_schedule_attempt(schedule_tree.primary_kind());
                }
                if schedule_tree.contains_annotation_note("auto-fused-statements") {
                    stats.schedule_auto_fuse_selected += 1;
                }
                if schedule_tree.contains_annotation_note("auto-fission-split") {
                    stats.schedule_auto_fission_selected += 1;
                }
                if schedule.kind == SchedulePlanKind::Skew2D && env_auto_mode("RR_POLY_SKEW_2D") {
                    stats.schedule_auto_skew2d_selected += 1;
                }
                if schedule_tree
                    .backend_artifact
                    .as_deref()
                    .is_some_and(|artifact| artifact.contains("hint_selected=1"))
                {
                    stats.schedule_backend_hint_selected += 1;
                }
                if let Some(root) = dump_root_from_env() {
                    let cert = build_certificate(
                        fn_ir,
                        &scop,
                        deps.clone(),
                        schedule.clone(),
                        schedule_tree.clone(),
                    );
                    dump_certificate(&root, &cert, &scop);
                }

                if poly_trace_enabled() {
                    eprintln!(
                        "   [poly] {} loop header={} latch={} :: {} | dep={:?} | schedule={:?} | cost={}",
                        fn_ir.name,
                        lp.header,
                        lp.latch,
                        format_scop_summary(&scop),
                        deps.derived_state(),
                        schedule.kind,
                        estimate_schedule_cost(&scop, &schedule),
                    );
                }
            }
            Err(reason) => {
                stats.record_reject(reason);
                if let Some(root) = dump_root_from_env() {
                    dump_rejection(&root, fn_ir, lp.header, lp.latch, reason);
                }
                if poly_trace_enabled() {
                    eprintln!(
                        "   [poly] {} loop header={} latch={} reject: {}",
                        fn_ir.name,
                        lp.header,
                        lp.latch,
                        format_reject_reason(reason),
                    );
                }
            }
        }
    }

    (stats, loops)
}
