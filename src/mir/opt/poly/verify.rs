use super::cost::{describe_schedule_decision, estimate_fission_benefit, estimate_schedule_cost};
use super::dependence_backend::{
    DependenceEdge, DependenceRelation, DependenceResult, DependenceSummary,
};
use super::schedule::SchedulePlan;
use super::tree::{ScheduleTransformKind, ScheduleTree};
use super::{ScopExtractionFailure, ScopRegion};
use crate::mir::{BlockId, FnIR};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct PolyCertificate {
    pub function: String,
    pub header: BlockId,
    pub latch: BlockId,
    pub summary: String,
    pub dependence: DependenceSummary,
    pub dependence_relation: DependenceRelation,
    pub dependence_edges: usize,
    pub dependence_edge_snapshot: Vec<DependenceEdge>,
    pub schedule_tree_primary: SchedulePlan,
    pub schedule_tree_nodes: usize,
    pub schedule_tree_band_depth: usize,
    pub schedule_tree_fuse_nodes: usize,
    pub schedule_tree_split_nodes: usize,
    pub schedule_tree_skew_nodes: usize,
    pub schedule_tree_backend_artifact: Option<String>,
    pub schedule_estimated_cost: u32,
    pub schedule_fission_benefit: i32,
    pub schedule_decision_reason: String,
    pub schedule: SchedulePlan,
    pub schedule_tree: ScheduleTree,
}

pub fn format_reject_reason(reason: ScopExtractionFailure) -> &'static str {
    match reason {
        ScopExtractionFailure::UnsupportedCfgShape => "unsupported CFG shape",
        ScopExtractionFailure::MissingInductionVar => "missing induction variable",
        ScopExtractionFailure::NonAffineLoopBound => "non-affine loop bound",
        ScopExtractionFailure::NonAffineAccess => "non-affine access",
        ScopExtractionFailure::EffectfulStatement => "effectful statement in region",
        ScopExtractionFailure::UnsupportedNestedLoop => "nested loop not yet supported",
    }
}

pub fn format_scop_summary(scop: &ScopRegion) -> String {
    let access_count = scop
        .statements
        .iter()
        .map(|stmt| stmt.accesses.len())
        .sum::<usize>();
    format!(
        "dims={} stmts={} accesses={} params={} constraints={}",
        scop.dimensions.len(),
        scop.statements.len(),
        access_count,
        scop.parameters.len(),
        scop.iteration_space.constraints.len(),
    )
}

pub fn build_certificate(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    dependence: DependenceResult,
    schedule: SchedulePlan,
    schedule_tree: ScheduleTree,
) -> PolyCertificate {
    PolyCertificate {
        function: fn_ir.name.clone(),
        header: scop.header,
        latch: scop.latch,
        summary: format_scop_summary(scop),
        dependence: dependence.summary,
        dependence_relation: dependence.relation.clone(),
        dependence_edges: dependence.edges.len(),
        dependence_edge_snapshot: dependence.edges,
        schedule_tree_primary: schedule_tree.to_primary_plan(),
        schedule_tree_nodes: schedule_tree.node_count(),
        schedule_tree_band_depth: schedule_tree.band_depth(),
        schedule_tree_fuse_nodes: schedule_tree.transform_count(ScheduleTransformKind::Fuse),
        schedule_tree_split_nodes: schedule_tree.transform_count(ScheduleTransformKind::Split),
        schedule_tree_skew_nodes: schedule_tree.transform_count(ScheduleTransformKind::Skew),
        schedule_tree_backend_artifact: schedule_tree.backend_artifact.clone(),
        schedule_estimated_cost: estimate_schedule_cost(scop, &schedule),
        schedule_fission_benefit: estimate_fission_benefit(scop, &schedule),
        schedule_decision_reason: describe_schedule_decision(scop, &schedule),
        schedule,
        schedule_tree,
    }
}

fn sanitize(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch);
        } else {
            out.push('_');
        }
    }
    if out.is_empty() {
        "unnamed".to_string()
    } else {
        out
    }
}

fn write_text(path: &Path, body: String) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(path, body);
}

pub fn dump_certificate(root: &Path, certificate: &PolyCertificate, scop: &ScopRegion) {
    let file = format!(
        "{}__header{}_latch{}.poly.txt",
        sanitize(&certificate.function),
        certificate.header,
        certificate.latch
    );
    let path = root.join(file);
    let body = format!(
        "# poly certificate\nfunction: {}\nheader: {}\nlatch: {}\nsummary: {}\ndependence: {:?}\ndependence_relation: {:#?}\ndependence_edges: {}\ndependence_edge_snapshot: {:#?}\nschedule_tree_primary: {:?}\nschedule_tree_nodes: {}\nschedule_tree_band_depth: {}\nschedule_tree_fuse_nodes: {}\nschedule_tree_split_nodes: {}\nschedule_tree_skew_nodes: {}\nschedule_tree_backend_artifact: {:?}\nschedule_estimated_cost: {}\nschedule_fission_benefit: {}\nschedule_decision_reason: {}\nschedule_tree: {:#?}\nprimary_schedule: {:?}\n\niteration_space: {:#?}\n\nscop: {:#?}\n",
        certificate.function,
        certificate.header,
        certificate.latch,
        certificate.summary,
        certificate.dependence,
        certificate.dependence_relation,
        certificate.dependence_edges,
        certificate.dependence_edge_snapshot,
        certificate.schedule_tree_primary,
        certificate.schedule_tree_nodes,
        certificate.schedule_tree_band_depth,
        certificate.schedule_tree_fuse_nodes,
        certificate.schedule_tree_split_nodes,
        certificate.schedule_tree_skew_nodes,
        certificate.schedule_tree_backend_artifact,
        certificate.schedule_estimated_cost,
        certificate.schedule_fission_benefit,
        certificate.schedule_decision_reason,
        certificate.schedule_tree,
        certificate.schedule,
        scop.iteration_space,
        scop
    );
    write_text(&path, body);
}

pub fn dump_rejection(
    root: &Path,
    fn_ir: &FnIR,
    header: BlockId,
    latch: BlockId,
    reason: ScopExtractionFailure,
) {
    let file = format!(
        "{}__header{}_latch{}.reject.txt",
        sanitize(&fn_ir.name),
        header,
        latch
    );
    let path = root.join(file);
    let body = format!(
        "# poly rejection\nfunction: {}\nheader: {}\nlatch: {}\nreason: {}\n",
        fn_ir.name,
        header,
        latch,
        format_reject_reason(reason)
    );
    write_text(&path, body);
}

pub fn dump_root_from_env() -> Option<PathBuf> {
    std::env::var("RR_POLY_DUMP_DIR").ok().and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(PathBuf::from(trimmed))
        }
    })
}
