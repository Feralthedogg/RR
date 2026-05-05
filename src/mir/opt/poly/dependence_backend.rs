use super::ScopRegion;
use super::access::{AccessKind, AccessRelation, MemRef};
use super::affine::{AffineExpr, AffineSymbol};
use super::backend::PolySolverBackend;
use super::isl::{map_roundtrip_if_non_empty, union_maps_roundtrip};
use crate::mir::{FnIR, ValueId, ValueKind};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependenceState {
    NotNeeded,
    IdentityProven,
    ReductionProven,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependenceSummary {
    pub state: DependenceState,
    pub write_count: usize,
    pub access_count: usize,
    pub reduction_count: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DependenceKind {
    Raw,
    War,
    Waw,
    Reduction,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DependenceEdge {
    pub kind: DependenceKind,
    pub statement_id: usize,
    pub memref_base: ValueId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependenceRelation {
    pub iteration_dimensions: Vec<String>,
    pub edge_count: usize,
    pub raw_relation: Option<String>,
    pub war_relation: Option<String>,
    pub waw_relation: Option<String>,
    pub reduction_relation: Option<String>,
    pub validity_relation: Option<String>,
    pub proximity_relation: Option<String>,
    pub symbolic_guard_candidate: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DependenceResult {
    pub summary: DependenceSummary,
    pub relation: DependenceRelation,
    pub edges: Vec<DependenceEdge>,
}

impl DependenceResult {
    pub fn has_explicit_relations(&self) -> bool {
        self.relation.raw_relation.is_some()
            || self.relation.war_relation.is_some()
            || self.relation.waw_relation.is_some()
            || self.relation.reduction_relation.is_some()
            || self.relation.validity_relation.is_some()
            || self.relation.proximity_relation.is_some()
    }

    pub fn derived_state(&self) -> DependenceState {
        if self.relation.validity_relation.is_some() {
            DependenceState::Unknown
        } else if self.relation.reduction_relation.is_some() {
            DependenceState::ReductionProven
        } else if self.summary.write_count == 0 {
            DependenceState::NotNeeded
        } else {
            DependenceState::IdentityProven
        }
    }

    pub fn legality_is_proven(&self) -> bool {
        self.derived_state() != DependenceState::Unknown
    }
}

fn derived_state_from_relation(
    relation: &DependenceRelation,
    write_count: usize,
) -> DependenceState {
    if relation.validity_relation.is_some() {
        DependenceState::Unknown
    } else if relation.reduction_relation.is_some() {
        DependenceState::ReductionProven
    } else if write_count == 0 {
        DependenceState::NotNeeded
    } else {
        DependenceState::IdentityProven
    }
}

fn memref_origin_key(fn_ir: &FnIR, memref: &MemRef) -> Option<String> {
    let value = fn_ir.values.get(memref.base)?;
    if let Some(origin) = value.origin_var.as_deref()
        && !origin.is_empty()
    {
        return Some(format!("origin:{origin}"));
    }
    match &value.kind {
        ValueKind::Load { var } => Some(format!("origin:{var}")),
        ValueKind::Param { index } => fn_ir
            .params
            .get(*index)
            .map(|name| format!("origin:{name}"))
            .or_else(|| Some(format!("param:{index}"))),
        _ => None,
    }
}

fn memref_identity_key(fn_ir: &FnIR, access: &AccessRelation) -> String {
    memref_origin_key(fn_ir, &access.memref)
        .unwrap_or_else(|| format!("value:{}", access.memref.base))
}

fn memrefs_may_alias(fn_ir: &FnIR, lhs: &AccessRelation, rhs: &AccessRelation) -> bool {
    lhs.memref.base == rhs.memref.base
        || memref_origin_key(fn_ir, &lhs.memref)
            .zip(memref_origin_key(fn_ir, &rhs.memref))
            .is_some_and(|(lhs, rhs)| lhs == rhs)
}

pub trait PolyDependenceBackend {
    fn analyze(&self, fn_ir: &FnIR, scop: &ScopRegion) -> DependenceResult;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct HeuristicDependenceBackend;

#[derive(Debug, Default, Clone, Copy)]
pub struct IslDependenceBackend;

fn resolve_scop_local_source(fn_ir: &FnIR, scop: &ScopRegion, root: ValueId) -> ValueId {
    let mut current = root;
    let mut seen = rustc_hash::FxHashSet::default();
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
                    (super::PolyStmtKind::Assign { dst }, Some(expr_root)) if dst == var => {
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

fn expr_mentions_var(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    root: ValueId,
    var: &str,
    seen: &mut rustc_hash::FxHashSet<ValueId>,
) -> bool {
    let root = resolve_scop_local_source(fn_ir, scop, root);
    if !seen.insert(root) {
        return false;
    }
    if fn_ir.values[root].origin_var.as_deref() == Some(var) {
        return true;
    }
    match &fn_ir.values[root].kind {
        ValueKind::Load { var: load_var } => load_var == var,
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_mentions_var(fn_ir, scop, *lhs, var, seen)
                || expr_mentions_var(fn_ir, scop, *rhs, var, seen)
        }
        ValueKind::Unary { rhs, .. } => expr_mentions_var(fn_ir, scop, *rhs, var, seen),
        ValueKind::RecordLit { fields } => fields
            .iter()
            .any(|(_, value)| expr_mentions_var(fn_ir, scop, *value, var, seen)),
        ValueKind::FieldGet { base, .. } => expr_mentions_var(fn_ir, scop, *base, var, seen),
        ValueKind::FieldSet { base, value, .. } => {
            expr_mentions_var(fn_ir, scop, *base, var, seen)
                || expr_mentions_var(fn_ir, scop, *value, var, seen)
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| expr_mentions_var(fn_ir, scop, *arg, var, seen)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(arg, _)| expr_mentions_var(fn_ir, scop, *arg, var, seen)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_mentions_var(fn_ir, scop, *base, var, seen)
        }
        ValueKind::Range { start, end } => {
            expr_mentions_var(fn_ir, scop, *start, var, seen)
                || expr_mentions_var(fn_ir, scop, *end, var, seen)
        }
        ValueKind::Index1D { base, idx, .. } => {
            expr_mentions_var(fn_ir, scop, *base, var, seen)
                || expr_mentions_var(fn_ir, scop, *idx, var, seen)
        }
        ValueKind::Index2D { base, r, c } => {
            expr_mentions_var(fn_ir, scop, *base, var, seen)
                || expr_mentions_var(fn_ir, scop, *r, var, seen)
                || expr_mentions_var(fn_ir, scop, *c, var, seen)
        }
        ValueKind::Index3D { base, i, j, k } => {
            expr_mentions_var(fn_ir, scop, *base, var, seen)
                || expr_mentions_var(fn_ir, scop, *i, var, seen)
                || expr_mentions_var(fn_ir, scop, *j, var, seen)
                || expr_mentions_var(fn_ir, scop, *k, var, seen)
        }
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => false,
    }
}

fn statement_is_reduction(fn_ir: &FnIR, scop: &ScopRegion, stmt: &super::PolyStmt) -> bool {
    let (super::PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root)
    else {
        return false;
    };
    if stmt.accesses.is_empty() {
        return false;
    }
    expr_mentions_var(
        fn_ir,
        scop,
        expr_root,
        dst,
        &mut rustc_hash::FxHashSet::default(),
    )
}

fn heuristic_dependence_result(fn_ir: &FnIR, scop: &ScopRegion) -> DependenceResult {
    let accesses = scop
        .statements
        .iter()
        .flat_map(|stmt| stmt.accesses.iter().map(move |access| (stmt.id, access)))
        .collect::<Vec<_>>();
    let write_accesses = accesses
        .iter()
        .filter(|(_, access)| matches!(access.kind, AccessKind::Write))
        .collect::<Vec<_>>();
    let write_count = write_accesses.len();
    let access_count = accesses.len();
    let reduction_count = scop
        .statements
        .iter()
        .filter(|stmt| statement_is_reduction(fn_ir, scop, stmt))
        .count();
    let distinct_write_bases = write_accesses
        .iter()
        .map(|(_, access)| memref_identity_key(fn_ir, access))
        .collect::<std::collections::BTreeSet<_>>()
        .len();

    let mut edges = Vec::new();
    for (stmt_id, access) in &accesses {
        let kind = match access.kind {
            AccessKind::Read => {
                if reduction_count > 0 {
                    DependenceKind::Reduction
                } else {
                    DependenceKind::Raw
                }
            }
            AccessKind::Write => {
                if distinct_write_bases == write_count {
                    DependenceKind::Waw
                } else {
                    DependenceKind::War
                }
            }
        };
        edges.push(DependenceEdge {
            kind,
            statement_id: *stmt_id,
            memref_base: access.memref.base,
        });
    }

    let relation = DependenceRelation {
        iteration_dimensions: scop
            .dimensions
            .iter()
            .map(|dim| dim.iv_name.clone())
            .collect(),
        edge_count: edges.len(),
        raw_relation: None,
        war_relation: None,
        waw_relation: None,
        reduction_relation: if reduction_count > 0 && write_count == 0 {
            Some("heuristic-reduction".to_string())
        } else {
            None
        },
        validity_relation: if write_count > 1 && distinct_write_bases != write_count {
            Some("heuristic-unknown".to_string())
        } else {
            None
        },
        proximity_relation: None,
        symbolic_guard_candidate: symbolic_guard_candidate(scop),
    };
    let state = derived_state_from_relation(&relation, write_count);
    DependenceResult {
        summary: DependenceSummary {
            state,
            write_count,
            access_count,
            reduction_count,
        },
        relation,
        edges,
    }
}

fn sanitize(raw: &str) -> String {
    raw.chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect()
}

fn symbolic_guard_candidate(scop: &ScopRegion) -> Option<String> {
    if scop.parameters.is_empty() {
        return None;
    }
    let dims = scop
        .dimensions
        .iter()
        .map(|dim| sanitize(&dim.iv_name))
        .collect::<Vec<_>>();
    let params = scop
        .parameters
        .iter()
        .map(|name| sanitize(name))
        .collect::<Vec<_>>();
    let prefix = if params.is_empty() {
        String::new()
    } else {
        format!("[{}] -> ", params.join(", "))
    };
    let constraints = scop
        .iteration_space
        .constraints
        .iter()
        .zip(
            scop.dimensions
                .iter()
                .flat_map(|dim| [dim.iv_name.clone(), dim.iv_name.clone()]),
        )
        .map(|(constraint, iv_name)| {
            let iv_name = sanitize(&iv_name);
            match constraint.kind {
                super::affine::AffineConstraintKind::LowerBound => {
                    format!("{} <= {}", expr_to_isl(&constraint.expr, sanitize), iv_name)
                }
                super::affine::AffineConstraintKind::UpperBound => {
                    format!("{} <= {}", iv_name, expr_to_isl(&constraint.expr, sanitize))
                }
            }
        })
        .collect::<Vec<_>>();
    Some(format!(
        "{prefix}{{ [{}] : {} }}",
        dims.join(", "),
        constraints.join(" and ")
    ))
}

fn symbol_name(symbol: &AffineSymbol, loop_renamer: impl Fn(&str) -> String) -> String {
    match symbol {
        AffineSymbol::LoopIv(name) => loop_renamer(name),
        AffineSymbol::Param(name) => format!("p_{}", sanitize(name)),
        AffineSymbol::Invariant(name) => format!("inv_{}", sanitize(name)),
        AffineSymbol::Length(name) => format!("len_{}", sanitize(name)),
    }
}

fn expr_to_isl(expr: &AffineExpr, loop_renamer: impl Fn(&str) -> String + Copy) -> String {
    let mut parts = Vec::new();
    for (symbol, coeff) in &expr.terms {
        let name = symbol_name(symbol, loop_renamer);
        let term = match *coeff {
            1 => name,
            -1 => format!("-{name}"),
            coeff => format!("{coeff}*{name}"),
        };
        parts.push(term);
    }
    if expr.constant != 0 || parts.is_empty() {
        parts.push(expr.constant.to_string());
    }
    parts.join(" + ").replace("+ -", "- ")
}

fn collect_params_from_expr(expr: &AffineExpr, params: &mut BTreeSet<String>) {
    for symbol in expr.terms.keys() {
        match symbol {
            AffineSymbol::LoopIv(_) => {}
            AffineSymbol::Param(name) => {
                params.insert(format!("p_{}", sanitize(name)));
            }
            AffineSymbol::Invariant(name) => {
                params.insert(format!("inv_{}", sanitize(name)));
            }
            AffineSymbol::Length(name) => {
                params.insert(format!("len_{}", sanitize(name)));
            }
        }
    }
}

fn scop_param_prefix(scop: &ScopRegion) -> String {
    let mut params = BTreeSet::new();
    for dim in &scop.dimensions {
        collect_params_from_expr(&dim.lower_bound, &mut params);
        collect_params_from_expr(&dim.upper_bound, &mut params);
    }
    for stmt in &scop.statements {
        for access in &stmt.accesses {
            for subscript in &access.subscripts {
                collect_params_from_expr(subscript, &mut params);
            }
        }
    }
    if params.is_empty() {
        String::new()
    } else {
        format!(
            "[{}] -> ",
            params.into_iter().collect::<Vec<_>>().join(", ")
        )
    }
}

fn lex_lt_constraint(src_dims: &[String], dst_dims: &[String]) -> String {
    let mut clauses = Vec::new();
    for idx in 0..src_dims.len().min(dst_dims.len()) {
        let mut parts = Vec::new();
        for prefix in 0..idx {
            parts.push(format!("{} = {}", src_dims[prefix], dst_dims[prefix]));
        }
        parts.push(format!("{} < {}", src_dims[idx], dst_dims[idx]));
        clauses.push(format!("({})", parts.join(" and ")));
    }
    clauses.join(" or ")
}

fn domain_constraints(scop: &ScopRegion, suffix: &str) -> Vec<String> {
    scop.dimensions
        .iter()
        .flat_map(|dim| {
            let iv_name = format!("{}_{}", sanitize(&dim.iv_name), suffix);
            let lower = expr_to_isl(&dim.lower_bound, |name| {
                format!("{}_{}", sanitize(name), suffix)
            });
            let upper = expr_to_isl(&dim.upper_bound, |name| {
                format!("{}_{}", sanitize(name), suffix)
            });
            vec![
                format!("{lower} <= {iv_name}"),
                format!("{iv_name} <= {upper}"),
            ]
        })
        .collect()
}

fn dependence_map_string(
    scop: &ScopRegion,
    source_stmt_id: usize,
    sink_stmt_id: usize,
    source_subscripts: &[AffineExpr],
    sink_subscripts: &[AffineExpr],
) -> Option<String> {
    if source_subscripts.len() != sink_subscripts.len() {
        return None;
    }
    let param_prefix = scop_param_prefix(scop);
    let src_dims = scop
        .dimensions
        .iter()
        .map(|dim| format!("{}_src", sanitize(&dim.iv_name)))
        .collect::<Vec<_>>();
    let dst_dims = scop
        .dimensions
        .iter()
        .map(|dim| format!("{}_dst", sanitize(&dim.iv_name)))
        .collect::<Vec<_>>();
    let mut constraints = Vec::new();
    constraints.extend(domain_constraints(scop, "src"));
    constraints.extend(domain_constraints(scop, "dst"));
    let lex = lex_lt_constraint(&src_dims, &dst_dims);
    if !lex.is_empty() {
        constraints.push(lex);
    }
    for (src, dst) in source_subscripts.iter().zip(sink_subscripts.iter()) {
        let src_expr = expr_to_isl(src, |name| format!("{}_src", sanitize(name)));
        let dst_expr = expr_to_isl(dst, |name| format!("{}_dst", sanitize(name)));
        constraints.push(format!("{src_expr} = {dst_expr}"));
    }
    Some(format!(
        "{param_prefix}{{ S{source_stmt_id}[{}] -> S{sink_stmt_id}[{}] : {} }}",
        src_dims.join(", "),
        dst_dims.join(", "),
        constraints.join(" and ")
    ))
}

fn reduction_map_string(scop: &ScopRegion, stmt_id: usize) -> Option<String> {
    let param_prefix = scop_param_prefix(scop);
    let src_dims = scop
        .dimensions
        .iter()
        .map(|dim| format!("{}_src", sanitize(&dim.iv_name)))
        .collect::<Vec<_>>();
    let dst_dims = scop
        .dimensions
        .iter()
        .map(|dim| format!("{}_dst", sanitize(&dim.iv_name)))
        .collect::<Vec<_>>();
    let mut constraints = Vec::new();
    constraints.extend(domain_constraints(scop, "src"));
    constraints.extend(domain_constraints(scop, "dst"));
    let lex = lex_lt_constraint(&src_dims, &dst_dims);
    if !lex.is_empty() {
        constraints.push(lex);
    }
    Some(format!(
        "{param_prefix}{{ S{stmt_id}[{}] -> S{stmt_id}[{}] : {} }}",
        src_dims.join(", "),
        dst_dims.join(", "),
        constraints.join(" and ")
    ))
}

fn access_is_full_rank_iteration_identity(
    access: &super::access::AccessRelation,
    scop: &ScopRegion,
) -> bool {
    access.subscripts.len() == scop.dimensions.len()
        && access
            .subscripts
            .iter()
            .zip(scop.dimensions.iter())
            .all(|(expr, dim)| {
                expr.constant == 0
                    && expr.terms.len() == 1
                    && matches!(
                        expr.terms.iter().next(),
                        Some((AffineSymbol::LoopIv(name), coeff)) if name == &dim.iv_name && *coeff == 1
                    )
            })
}

fn isl_dependence_result(fn_ir: &FnIR, scop: &ScopRegion) -> DependenceResult {
    let accesses = scop
        .statements
        .iter()
        .flat_map(|stmt| stmt.accesses.iter().map(move |access| (stmt.id, access)))
        .collect::<Vec<_>>();
    let write_accesses = accesses
        .iter()
        .filter(|(_, access)| matches!(access.kind, AccessKind::Write))
        .collect::<Vec<_>>();
    let write_count = write_accesses.len();
    let access_count = accesses.len();
    let reduction_stmt_ids = scop
        .statements
        .iter()
        .filter(|stmt| statement_is_reduction(fn_ir, scop, stmt))
        .map(|stmt| stmt.id)
        .collect::<BTreeSet<_>>();
    let reduction_count = reduction_stmt_ids.len();

    let mut raw_maps = Vec::new();
    let mut war_maps = Vec::new();
    let mut waw_maps = Vec::new();
    let mut reduction_maps = Vec::new();
    let mut edges = Vec::new();

    for (source_stmt_id, source_access) in &accesses {
        for (sink_stmt_id, sink_access) in &accesses {
            if !memrefs_may_alias(fn_ir, source_access, sink_access)
                || source_access.subscripts.len() != sink_access.subscripts.len()
            {
                continue;
            }
            let kind = match (source_access.kind, sink_access.kind) {
                (AccessKind::Write, AccessKind::Read) => DependenceKind::Raw,
                (AccessKind::Read, AccessKind::Write) => DependenceKind::War,
                (AccessKind::Write, AccessKind::Write) => DependenceKind::Waw,
                (AccessKind::Read, AccessKind::Read) => continue,
            };
            if matches!(kind, DependenceKind::Waw)
                && source_stmt_id == sink_stmt_id
                && source_access.subscripts == sink_access.subscripts
                && access_is_full_rank_iteration_identity(source_access, scop)
            {
                continue;
            }
            let Some(raw_map) = dependence_map_string(
                scop,
                *source_stmt_id,
                *sink_stmt_id,
                &source_access.subscripts,
                &sink_access.subscripts,
            ) else {
                continue;
            };
            let Some(roundtrip) = map_roundtrip_if_non_empty(&raw_map) else {
                continue;
            };
            match kind {
                DependenceKind::Raw => raw_maps.push(roundtrip),
                DependenceKind::War => war_maps.push(roundtrip),
                DependenceKind::Waw => waw_maps.push(roundtrip),
                DependenceKind::Reduction => {}
            }
            edges.push(DependenceEdge {
                kind,
                statement_id: *sink_stmt_id,
                memref_base: source_access.memref.base,
            });
        }
    }

    for stmt_id in &reduction_stmt_ids {
        let Some(raw_map) = reduction_map_string(scop, *stmt_id) else {
            continue;
        };
        let Some(roundtrip) = map_roundtrip_if_non_empty(&raw_map) else {
            continue;
        };
        reduction_maps.push(roundtrip);
        if let Some(stmt) = scop.statements.iter().find(|stmt| stmt.id == *stmt_id) {
            let memref_base = stmt
                .accesses
                .first()
                .map(|access| access.memref.base)
                .unwrap_or(0);
            edges.push(DependenceEdge {
                kind: DependenceKind::Reduction,
                statement_id: *stmt_id,
                memref_base,
            });
        }
    }

    let raw_relation = union_maps_roundtrip(&raw_maps);
    let war_relation = union_maps_roundtrip(&war_maps);
    let waw_relation = union_maps_roundtrip(&waw_maps);
    let reduction_relation = union_maps_roundtrip(&reduction_maps);
    let validity_inputs = raw_relation
        .iter()
        .chain(war_relation.iter())
        .chain(waw_relation.iter())
        .cloned()
        .collect::<Vec<_>>();
    let validity_relation = union_maps_roundtrip(&validity_inputs);
    let proximity_relation = reduction_relation
        .clone()
        .or_else(|| validity_relation.clone());

    let relation = DependenceRelation {
        iteration_dimensions: scop
            .dimensions
            .iter()
            .map(|dim| dim.iv_name.clone())
            .collect(),
        edge_count: edges.len(),
        raw_relation,
        war_relation,
        waw_relation,
        reduction_relation,
        validity_relation,
        proximity_relation,
        symbolic_guard_candidate: symbolic_guard_candidate(scop),
    };
    let state = derived_state_from_relation(&relation, write_count);
    DependenceResult {
        summary: DependenceSummary {
            state,
            write_count,
            access_count,
            reduction_count,
        },
        relation,
        edges,
    }
}

impl PolyDependenceBackend for HeuristicDependenceBackend {
    fn analyze(&self, fn_ir: &FnIR, scop: &ScopRegion) -> DependenceResult {
        heuristic_dependence_result(fn_ir, scop)
    }
}

impl PolyDependenceBackend for IslDependenceBackend {
    fn analyze(&self, fn_ir: &FnIR, scop: &ScopRegion) -> DependenceResult {
        isl_dependence_result(fn_ir, scop)
    }
}

pub fn make_dependence_backend(
    solver_backend: &dyn PolySolverBackend,
) -> Box<dyn PolyDependenceBackend> {
    match solver_backend.used_backend() {
        super::schedule::PolyBackendUsed::Heuristic => Box::new(HeuristicDependenceBackend),
        super::schedule::PolyBackendUsed::Isl => Box::new(IslDependenceBackend),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::flow::Facts;
    use crate::mir::opt::poly::access::{AccessKind, AccessRelation, MemRef, MemoryLayout};
    use crate::mir::opt::poly::affine::{AffineExpr, AffineSymbol, PresburgerSet};
    use crate::mir::opt::poly::{LoopDimension, PolyStmt, PolyStmtKind, ScopRegion};
    use crate::utils::Span;

    #[test]
    fn heuristic_backend_produces_edges_and_relation() {
        let mut fn_ir = FnIR::new("dep_edges".to_string(), vec![]);
        let base = fn_ir.add_value(
            ValueKind::Load {
                var: "a".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("a".to_string()),
        );
        let idx = fn_ir.add_value(
            ValueKind::Load {
                var: "i".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        let scop = ScopRegion {
            header: 0,
            latch: 1,
            exits: vec![2],
            dimensions: vec![LoopDimension {
                iv_name: "i".to_string(),
                lower_bound: AffineExpr::constant(1),
                upper_bound: AffineExpr::symbol(AffineSymbol::Length("n".to_string())),
                step: 1,
            }],
            iteration_space: PresburgerSet::new(vec!["i".to_string()], vec![]),
            parameters: Default::default(),
            statements: vec![PolyStmt {
                id: 0,
                block: 0,
                kind: PolyStmtKind::Store {
                    base,
                    subscripts: vec![idx],
                },
                expr_root: None,
                accesses: vec![AccessRelation {
                    statement_id: 0,
                    kind: AccessKind::Write,
                    memref: MemRef {
                        base,
                        name: "a".to_string(),
                        rank: 1,
                        layout: MemoryLayout::Dense1D,
                    },
                    subscripts: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
                }],
            }],
        };
        let result = HeuristicDependenceBackend.analyze(&fn_ir, &scop);
        assert_eq!(result.relation.edge_count, 1);
        assert_eq!(result.edges.len(), 1);
        assert_eq!(result.summary.write_count, 1);
    }

    #[test]
    fn memref_aliases_by_origin_var_for_mutable_local_base() {
        let mut fn_ir = FnIR::new("dep_origin_alias".to_string(), vec![]);
        let x_alloc = fn_ir.add_value(
            ValueKind::Call {
                callee: "seq_len".to_string(),
                args: vec![],
                names: vec![],
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let x_load = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let out_load = fn_ir.add_value(
            ValueKind::Load {
                var: "out".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("out".to_string()),
        );
        let read = AccessRelation {
            statement_id: 0,
            kind: AccessKind::Read,
            memref: MemRef {
                base: x_alloc,
                name: "v0".to_string(),
                rank: 1,
                layout: MemoryLayout::Dense1D,
            },
            subscripts: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
        };
        let write_same_var = AccessRelation {
            statement_id: 0,
            kind: AccessKind::Write,
            memref: MemRef {
                base: x_load,
                name: "x".to_string(),
                rank: 1,
                layout: MemoryLayout::Dense1D,
            },
            subscripts: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
        };
        let write_other_var = AccessRelation {
            statement_id: 0,
            kind: AccessKind::Write,
            memref: MemRef {
                base: out_load,
                name: "out".to_string(),
                rank: 1,
                layout: MemoryLayout::Dense1D,
            },
            subscripts: vec![AffineExpr::symbol(AffineSymbol::LoopIv("i".to_string()))],
        };

        assert!(memrefs_may_alias(&fn_ir, &read, &write_same_var));
        assert!(!memrefs_may_alias(&fn_ir, &read, &write_other_var));
    }
}
