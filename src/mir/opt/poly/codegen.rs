//! Schedule-tree guided polyhedral codegen entrypoints.
//!
//! This layer decides whether a discovered poly schedule can lower through
//! specialized vectorized builders or should fall back to the generic poly MIR
//! reconstruction path.

use super::ScopRegion;
use super::codegen_generic::{
    generic_mir_effective_for_schedule, generic_schedule_supports_map,
    generic_schedule_supports_reduce, lower_fission_sequence_generic, lower_identity_map_generic,
    lower_identity_reduce_generic, lower_interchange_map_generic, lower_interchange_reduce_generic,
    lower_skew2d_map_generic, lower_skew2d_reduce_generic, lower_tile1d_map_generic,
    lower_tile1d_reduce_generic, lower_tile2d_map_generic, lower_tile2d_reduce_generic,
    lower_tile3d_map_generic, lower_tile3d_reduce_generic,
};
use super::schedule::{SchedulePlan, SchedulePlanKind};
use super::tree::{ScheduleTransform, ScheduleTree, ScheduleTreeNode};
use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::opt::v_opt::{
    Axis3D, PreparedVectorAssignment, ReduceKind, VectorPlan, build_slice_assignment_value,
    emit_same_array3_shape_or_scalar_guard, emit_same_matrix_shape_or_scalar_guard,
    finish_vector_assignments_versioned, prepare_partial_slice_value, same_length_proven,
    try_apply_vectorization_transactionally, vector_apply_site,
};
use crate::mir::{FnIR, Lit, ValueId, ValueKind};
use crate::syntax::ast::BinOp;

#[path = "codegen_lower.rs"]
mod codegen_lower;
use self::codegen_lower::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PolyCodegenPlan {
    pub emitted: bool,
}

pub fn lower_schedule_tree(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    tree: &ScheduleTree,
) -> PolyCodegenPlan {
    fn is_fission_sequence(node: &ScheduleTreeNode) -> bool {
        match node {
            ScheduleTreeNode::Sequence(nodes) if nodes.len() > 1 => nodes.iter().all(|node| {
                matches!(
                    node,
                    ScheduleTreeNode::Filter(filter) if filter.statement_ids.len() == 1
                )
            }),
            _ => false,
        }
    }

    fn scop_subset(scop: &ScopRegion, stmt_ids: &[usize]) -> ScopRegion {
        ScopRegion {
            header: scop.header,
            latch: scop.latch,
            exits: scop.exits.clone(),
            dimensions: scop.dimensions.clone(),
            iteration_space: scop.iteration_space.clone(),
            parameters: scop.parameters.clone(),
            statements: scop
                .statements
                .iter()
                .filter(|stmt| stmt_ids.contains(&stmt.id))
                .cloned()
                .collect(),
        }
    }

    fn transform_plan(
        transform: &ScheduleTransform,
        backend: super::schedule::PolyBackendUsed,
    ) -> SchedulePlan {
        SchedulePlan {
            kind: transform.plan_kind,
            relation: transform.relation.clone(),
            backend,
            tile_size: transform.tile_size,
            tile_depth: transform.tile_depth,
            tile_rows: transform.tile_rows,
            tile_cols: transform.tile_cols,
        }
    }

    if is_fission_sequence(&tree.root)
        && lower_fission_sequence_generic(fn_ir, lp, scop, &tree.to_primary_plan())
    {
        return PolyCodegenPlan { emitted: true };
    }

    // When a fission sequence splits statements into per-filter groups, the
    // per-filter `rec` path processes them one at a time.  The first successful
    // codegen mutates the preheader (Goto → If), which causes
    // `vector_apply_site` to return None for subsequent filters.
    //
    // Work around this by trying the specialized *multi-statement* codegen on
    // the full (unfissioned) SCoP before falling into per-filter processing.
    // The `build_multi_*` helpers already collect all matching statements and
    // emit them in a single `finish_vector_assignments_versioned` call.
    //
    // When the primary schedule (e.g. Skew2D) lacks specialized multi-statement
    // codegen, we also attempt Interchange which has `build_multi_*` helpers.
    if is_fission_sequence(&tree.root) {
        let plan = tree.to_primary_plan();
        let result = match plan.kind {
            SchedulePlanKind::Identity => lower_identity_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::Interchange => lower_interchange_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::Skew2D => lower_skew2d_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::Tile1D => lower_tile1d_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::Tile2D => lower_tile2d_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::Tile3D => lower_tile3d_schedule(fn_ir, lp, scop, &plan),
            SchedulePlanKind::None => PolyCodegenPlan { emitted: false },
        };
        if result.emitted {
            return result;
        }
        // Skew2D/Identity lack specialized multi-statement codegen; fall back to
        // Interchange which has `build_multi_nested_2d_*` helpers.
        if matches!(
            plan.kind,
            SchedulePlanKind::Skew2D | SchedulePlanKind::Identity
        ) && scop.dimensions.len() >= 2
        {
            let xchg_plan = SchedulePlan {
                kind: SchedulePlanKind::Interchange,
                relation: super::schedule::interchange_relation(scop),
                backend: plan.backend,
                tile_size: None,
                tile_depth: None,
                tile_rows: None,
                tile_cols: None,
            };
            let result = lower_interchange_schedule(fn_ir, lp, scop, &xchg_plan);
            if result.emitted {
                return result;
            }
        }
    }

    fn rec(
        fn_ir: &mut FnIR,
        lp: &LoopInfo,
        scop: &ScopRegion,
        tree: &ScheduleTree,
        node: &ScheduleTreeNode,
    ) -> PolyCodegenPlan {
        match node {
            ScheduleTreeNode::Sequence(nodes) => {
                let mut emitted_any = false;
                for child in nodes {
                    let next = rec(fn_ir, lp, scop, tree, child);
                    if next.emitted {
                        emitted_any = true;
                    }
                }
                PolyCodegenPlan {
                    emitted: emitted_any,
                }
            }
            ScheduleTreeNode::Filter(filter) => {
                let subset = scop_subset(scop, &filter.statement_ids);
                for child in &filter.children {
                    let next = rec(fn_ir, lp, &subset, tree, child);
                    if next.emitted {
                        return next;
                    }
                }
                PolyCodegenPlan { emitted: false }
            }
            ScheduleTreeNode::Leaf => PolyCodegenPlan { emitted: false },
            ScheduleTreeNode::Band(band) => {
                for child in &band.children {
                    let next = rec(fn_ir, lp, scop, tree, child);
                    if next.emitted {
                        return next;
                    }
                }
                PolyCodegenPlan { emitted: false }
            }
            ScheduleTreeNode::Transform(transform) => {
                let emitted = if transform.plan_kind != SchedulePlanKind::None {
                    let plan = transform_plan(transform, tree.backend);
                    match transform.plan_kind {
                        SchedulePlanKind::Identity => {
                            lower_identity_schedule(fn_ir, lp, scop, &plan)
                        }
                        SchedulePlanKind::Interchange => {
                            lower_interchange_schedule(fn_ir, lp, scop, &plan)
                        }
                        SchedulePlanKind::Skew2D => lower_skew2d_schedule(fn_ir, lp, scop, &plan),
                        SchedulePlanKind::Tile1D => lower_tile1d_schedule(fn_ir, lp, scop, &plan),
                        SchedulePlanKind::Tile2D => lower_tile2d_schedule(fn_ir, lp, scop, &plan),
                        SchedulePlanKind::Tile3D => lower_tile3d_schedule(fn_ir, lp, scop, &plan),
                        SchedulePlanKind::None => PolyCodegenPlan { emitted: false },
                    }
                } else {
                    PolyCodegenPlan { emitted: false }
                };
                if emitted.emitted {
                    return emitted;
                }
                for child in &transform.children {
                    let next = rec(fn_ir, lp, scop, tree, child);
                    if next.emitted {
                        return next;
                    }
                }
                PolyCodegenPlan { emitted: false }
            }
        }
    }

    rec(fn_ir, lp, scop, tree, &tree.root)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MatrixMapOperands {
    dest: ValueId,
    lhs_src: ValueId,
    rhs_src: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VectorMapOperands {
    dest: ValueId,
    lhs_src: ValueId,
    rhs_src: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct VectorReduceOperands {
    base: ValueId,
    start: ValueId,
    end: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MatrixRectReduceOperands {
    base: ValueId,
    r_start: ValueId,
    r_end: ValueId,
    c_start: ValueId,
    c_end: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MatrixColReduceOperands {
    base: ValueId,
    col: ValueId,
    start: ValueId,
    end: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Array3MapOperands {
    dest: ValueId,
    lhs_src: ValueId,
    rhs_src: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Array3Dim1ReduceOperands {
    base: ValueId,
    fixed_a: ValueId,
    fixed_b: ValueId,
    start: ValueId,
    end: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Array3CubeReduceOperands {
    base: ValueId,
    i_start: ValueId,
    i_end: ValueId,
    j_start: ValueId,
    j_end: ValueId,
    k_start: ValueId,
    k_end: ValueId,
}

fn is_loop_iv_subscript(scop: &ScopRegion, expr: &super::affine::AffineExpr) -> bool {
    if scop.dimensions.len() != 1 || expr.constant != 0 || expr.terms.len() != 1 {
        return false;
    }
    matches!(
        expr.terms.iter().next(),
        Some((super::affine::AffineSymbol::LoopIv(name), coeff))
            if name == &scop.dimensions[0].iv_name && *coeff == 1
    )
}

fn is_named_loop_iv_subscript(name: &str, expr: &super::affine::AffineExpr) -> bool {
    if expr.constant != 0 || expr.terms.len() != 1 {
        return false;
    }
    matches!(
        expr.terms.iter().next(),
        Some((super::affine::AffineSymbol::LoopIv(loop_name), coeff))
            if loop_name == name && *coeff == 1
    )
}

fn is_scalarish_value(fn_ir: &FnIR, value: ValueId) -> bool {
    matches!(
        fn_ir.values[value].kind,
        ValueKind::Const(Lit::Int(_))
            | ValueKind::Const(Lit::Float(_))
            | ValueKind::Const(Lit::Bool(_))
            | ValueKind::Const(Lit::Str(_))
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::Len { .. }
    )
}

fn is_same_scalar_value(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    if fn_ir.values[a].origin_var.is_some()
        && fn_ir.values[a].origin_var == fn_ir.values[b].origin_var
    {
        return true;
    }
    match (&fn_ir.values[a].kind, &fn_ir.values[b].kind) {
        (ValueKind::Load { var: va }, ValueKind::Load { var: vb }) => va == vb,
        (ValueKind::Const(Lit::Int(va)), ValueKind::Const(Lit::Int(vb))) => va == vb,
        (ValueKind::Const(Lit::Float(va)), ValueKind::Const(Lit::Float(vb))) => {
            (*va - *vb).abs() < f64::EPSILON
        }
        _ => a == b,
    }
}

fn same_base_name(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
    if fn_ir.values[a].origin_var.is_some()
        && fn_ir.values[a].origin_var == fn_ir.values[b].origin_var
    {
        return true;
    }
    match (&fn_ir.values[a].kind, &fn_ir.values[b].kind) {
        (ValueKind::Load { var: va }, ValueKind::Load { var: vb }) => va == vb,
        (ValueKind::Param { index: ia }, ValueKind::Param { index: ib }) => ia == ib,
        _ => a == b,
    }
}

fn base_symbol_name(fn_ir: &FnIR, base: ValueId) -> String {
    if let Some(origin) = fn_ir.values[base].origin_var.clone() {
        return origin;
    }
    match &fn_ir.values[base].kind {
        ValueKind::Load { var } => var.clone(),
        ValueKind::Param { index } => fn_ir
            .params
            .get(*index)
            .cloned()
            .unwrap_or_else(|| format!(".arg_{index}")),
        _ => format!("v{base}"),
    }
}

fn loop_covers_whole_vector(fn_ir: &FnIR, lp: &LoopInfo, scop: &ScopRegion, base: ValueId) -> bool {
    if scop.dimensions.len() != 1 {
        return false;
    }
    let Some(step) = lp.iv.as_ref().map(|_| scop.dimensions[0].step) else {
        return false;
    };
    let start_ok = scop.dimensions[0].lower_bound.constant == 1
        && scop.dimensions[0].lower_bound.terms.is_empty();
    if !start_ok {
        return false;
    }
    if step != 1 {
        return false;
    }

    let dest_name = base_symbol_name(fn_ir, base);
    let upper = &scop.dimensions[0].upper_bound;
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-codegen] whole-check dest={} upper={:?} lp.is_seq_along={:?} lp.is_seq_len={:?}",
            dest_name, upper, lp.is_seq_along, lp.is_seq_len
        );
    }
    if upper.constant == 0
        && upper.terms.len() == 1
        && matches!(
            upper.terms.iter().next(),
            Some((super::affine::AffineSymbol::Length(name), coeff))
                if name == &dest_name && *coeff == 1
        )
    {
        return true;
    }

    if lp
        .is_seq_along
        .is_some_and(|loop_base| same_base_name(fn_ir, loop_base, base))
    {
        return true;
    }
    if let Some(loop_base) = lp.is_seq_along
        && same_length_proven(fn_ir, base, loop_base)
    {
        return true;
    }

    if let Some(limit) = lp.is_seq_len
        && let ValueKind::Len { base: len_base } = fn_ir.values[limit].kind
    {
        return same_base_name(fn_ir, len_base, base) || same_length_proven(fn_ir, len_base, base);
    }

    false
}

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

fn index_reads_loop_vector(fn_ir: &FnIR, scop: &ScopRegion, value: ValueId) -> Option<ValueId> {
    let value = resolve_scop_local_source(fn_ir, scop, value);
    match &fn_ir.values[value].kind {
        ValueKind::Index1D { base, idx, .. } => {
            let read = scop
                .statements
                .iter()
                .flat_map(|stmt| stmt.accesses.iter())
                .find(|access| {
                    matches!(access.kind, super::access::AccessKind::Read)
                        && access.memref.base == *base
                        && access.subscripts.len() == 1
                        && is_loop_iv_subscript(scop, &access.subscripts[0])
                })?;
            let _ = idx;
            Some(read.memref.base)
        }
        _ => None,
    }
}

fn index_reads_2d_col_vector(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    value: ValueId,
    fixed_col: ValueId,
) -> Option<ValueId> {
    let value = resolve_scop_local_source(fn_ir, scop, value);
    match &fn_ir.values[value].kind {
        ValueKind::Index2D { base, r: _, c } if is_same_scalar_value(fn_ir, *c, fixed_col) => {
            let read = scop
                .statements
                .iter()
                .flat_map(|stmt| stmt.accesses.iter())
                .find(|access| {
                    matches!(access.kind, super::access::AccessKind::Read)
                        && access.memref.base == *base
                        && access.subscripts.len() == 2
                        && is_loop_iv_subscript(scop, &access.subscripts[0])
                })?;
            Some(read.memref.base)
        }
        _ => None,
    }
}

fn index_reads_3d_dim1_vector(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    value: ValueId,
    fixed_a: ValueId,
    fixed_b: ValueId,
) -> Option<ValueId> {
    let value = resolve_scop_local_source(fn_ir, scop, value);
    match &fn_ir.values[value].kind {
        ValueKind::Index3D { base, i: _, j, k }
            if is_same_scalar_value(fn_ir, *j, fixed_a)
                && is_same_scalar_value(fn_ir, *k, fixed_b) =>
        {
            let read = scop
                .statements
                .iter()
                .flat_map(|stmt| stmt.accesses.iter())
                .find(|access| {
                    matches!(access.kind, super::access::AccessKind::Read)
                        && access.memref.base == *base
                        && access.subscripts.len() == 3
                        && is_loop_iv_subscript(scop, &access.subscripts[0])
                })?;
            Some(read.memref.base)
        }
        _ => None,
    }
}

fn encode_bound(fn_ir: &mut FnIR, expr: &super::affine::AffineExpr) -> Option<ValueId> {
    if expr.terms.is_empty() {
        return Some(fn_ir.add_value(
            ValueKind::Const(Lit::Int(expr.constant)),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    if expr.terms.len() != 1 {
        return None;
    }
    let (symbol, coeff) = expr.terms.iter().next()?;
    if *coeff != 1 {
        return None;
    }
    let base = match symbol {
        super::affine::AffineSymbol::Length(name) => {
            let base = fn_ir.values.iter().position(|value| {
                value.origin_var.as_deref() == Some(name.as_str())
                    || matches!(&value.kind, ValueKind::Load { var } if var == name)
            })?;
            fn_ir.add_value(
                ValueKind::Len { base },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            )
        }
        super::affine::AffineSymbol::Param(name) => {
            if let Some(index) = fn_ir.params.iter().position(|param| param == name) {
                fn_ir.add_value(
                    ValueKind::Param { index },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                )
            } else {
                return None;
            }
        }
        super::affine::AffineSymbol::Invariant(name) => {
            if let Some(index) = fn_ir.params.iter().position(|param| param == name) {
                fn_ir.add_value(
                    ValueKind::Param { index },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    None,
                )
            } else if let Some(value) = fn_ir.values.iter().position(|value| {
                value.origin_var.as_deref() == Some(name.as_str())
                    || matches!(&value.kind, ValueKind::Load { var } if var == name)
            }) {
                value
            } else {
                fn_ir.add_value(
                    ValueKind::Load { var: name.clone() },
                    crate::utils::Span::dummy(),
                    crate::mir::def::Facts::empty(),
                    Some(name.clone()),
                )
            }
        }
        super::affine::AffineSymbol::LoopIv(_) => return None,
    };
    if expr.constant == 0 {
        Some(base)
    } else {
        let offset = fn_ir.add_value(
            ValueKind::Const(Lit::Int(expr.constant.abs())),
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        Some(fn_ir.add_value(
            ValueKind::Binary {
                op: if expr.constant >= 0 {
                    BinOp::Add
                } else {
                    BinOp::Sub
                },
                lhs: base,
                rhs: offset,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ))
    }
}

fn build_map_plan(fn_ir: &FnIR, lp: &LoopInfo, scop: &ScopRegion) -> Option<VectorPlan> {
    if scop.dimensions.len() != 1 {
        if super::poly_trace_enabled() {
            eprintln!("   [poly-codegen] map reject: expected one dimension");
        }
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) => {
                Some((stmt, *base, subscripts.clone(), expr_root))
            }
            _ => None,
        });
    let (stmt, dest, subscripts, expr_root) = stores.next()?;
    if stores.next().is_some() {
        if super::poly_trace_enabled() {
            eprintln!("   [poly-codegen] map reject: multiple stores in SCoP");
        }
        return None;
    }
    let rank = subscripts.len();
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == rank
            && is_loop_iv_subscript(scop, &access.subscripts[0])
    })?;
    let _ = write;
    if rank == 1 && !loop_covers_whole_vector(fn_ir, lp, scop, dest) {
        if super::poly_trace_enabled() {
            eprintln!("   [poly-codegen] map reject: loop does not cover whole destination");
        }
        return None;
    }

    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] map reject: rhs is not binary: {:?}",
                fn_ir.values[expr_root].kind
            );
        }
        return None;
    };
    if !matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
    ) {
        if super::poly_trace_enabled() {
            eprintln!("   [poly-codegen] map reject: unsupported op {:?}", op);
        }
        return None;
    }

    if rank == 1 {
        let lhs_vec = index_reads_loop_vector(fn_ir, scop, lhs);
        let rhs_vec = index_reads_loop_vector(fn_ir, scop, rhs);

        if let (Some(lbase), Some(rbase)) = (lhs_vec, rhs_vec) {
            return Some(VectorPlan::Map {
                dest,
                src: lbase,
                op,
                other: rbase,
                shadow_vars: Vec::new(),
            });
        }
        if let Some(lbase) = lhs_vec
            && is_scalarish_value(fn_ir, rhs)
        {
            return Some(VectorPlan::Map {
                dest,
                src: lbase,
                op,
                other: rhs,
                shadow_vars: Vec::new(),
            });
        }
        if let Some(rbase) = rhs_vec
            && is_scalarish_value(fn_ir, lhs)
        {
            return Some(VectorPlan::Map {
                dest,
                src: lhs,
                op,
                other: rbase,
                shadow_vars: Vec::new(),
            });
        }
    } else if rank == 2 && subscripts.len() == 2 && is_loop_iv_subscript(scop, &write.subscripts[0])
    {
        let fixed_col = subscripts[1];
        let lhs_vec = index_reads_2d_col_vector(fn_ir, scop, lhs, fixed_col);
        let rhs_vec = index_reads_2d_col_vector(fn_ir, scop, rhs, fixed_col);

        if let (Some(lbase), Some(rbase)) = (lhs_vec, rhs_vec) {
            return Some(VectorPlan::Map2DCol {
                dest,
                col: fixed_col,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lbase,
                rhs_src: rbase,
                op,
            });
        }
        if let Some(lbase) = lhs_vec
            && is_scalarish_value(fn_ir, rhs)
        {
            return Some(VectorPlan::Map2DCol {
                dest,
                col: fixed_col,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lbase,
                rhs_src: rhs,
                op,
            });
        }
        if let Some(rbase) = rhs_vec
            && is_scalarish_value(fn_ir, lhs)
        {
            return Some(VectorPlan::Map2DCol {
                dest,
                col: fixed_col,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lhs,
                rhs_src: rbase,
                op,
            });
        }
    } else if rank == 3 && subscripts.len() == 3 && is_loop_iv_subscript(scop, &write.subscripts[0])
    {
        let fixed_a = subscripts[1];
        let fixed_b = subscripts[2];
        let lhs_vec = index_reads_3d_dim1_vector(fn_ir, scop, lhs, fixed_a, fixed_b);
        let rhs_vec = index_reads_3d_dim1_vector(fn_ir, scop, rhs, fixed_a, fixed_b);

        if let (Some(lbase), Some(rbase)) = (lhs_vec, rhs_vec) {
            return Some(VectorPlan::Map3D {
                dest,
                axis: Axis3D::Dim1,
                fixed_a,
                fixed_b,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lbase,
                rhs_src: rbase,
                op,
            });
        }
        if let Some(lbase) = lhs_vec
            && is_scalarish_value(fn_ir, rhs)
        {
            return Some(VectorPlan::Map3D {
                dest,
                axis: Axis3D::Dim1,
                fixed_a,
                fixed_b,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lbase,
                rhs_src: rhs,
                op,
            });
        }
        if let Some(rbase) = rhs_vec
            && is_scalarish_value(fn_ir, lhs)
        {
            return Some(VectorPlan::Map3D {
                dest,
                axis: Axis3D::Dim1,
                fixed_a,
                fixed_b,
                start: lp.iv.as_ref()?.init_val,
                end: lp.limit?,
                lhs_src: lhs,
                rhs_src: rbase,
                op,
            });
        }
    }
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-codegen] map reject: rank={} lhs={:?} rhs={:?}",
            rank, fn_ir.values[lhs].kind, fn_ir.values[rhs].kind,
        );
    }
    None
}

fn build_whole_vector_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    dest: ValueId,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, VectorMapOperands)> {
    let dest_var = base_symbol_name(fn_ir, dest);
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
        return None;
    };
    if !matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
    ) {
        return None;
    }
    let lhs_vec = index_reads_loop_vector(fn_ir, scop, lhs);
    let rhs_vec = index_reads_loop_vector(fn_ir, scop, rhs);
    let (out_val, lhs_src, rhs_src) = if let (Some(lbase), Some(rbase)) = (lhs_vec, rhs_vec) {
        (
            fn_ir.add_value(
                ValueKind::Binary {
                    op,
                    lhs: lbase,
                    rhs: rbase,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            lbase,
            rbase,
        )
    } else if let Some(lbase) = lhs_vec {
        if !is_scalarish_value(fn_ir, rhs) {
            return None;
        }
        (
            fn_ir.add_value(
                ValueKind::Binary {
                    op,
                    lhs: lbase,
                    rhs,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            lbase,
            rhs,
        )
    } else if let Some(rbase) = rhs_vec {
        if !is_scalarish_value(fn_ir, lhs) {
            return None;
        }
        (
            fn_ir.add_value(
                ValueKind::Binary {
                    op,
                    lhs,
                    rhs: rbase,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            lhs,
            rbase,
        )
    } else {
        return None;
    };
    Some((
        PreparedVectorAssignment {
            dest_var,
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        VectorMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

fn build_multi_whole_vector_map_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<VectorMapOperands>)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    let mut reference_dest: Option<ValueId> = None;
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 1 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 1
                && is_loop_iv_subscript(scop, &access.subscripts[0])
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var.clone()) {
            return None;
        }
        let whole_dest = loop_covers_whole_vector(fn_ir, lp, scop, *base)
            || reference_dest.is_some_and(|reference| same_length_proven(fn_ir, *base, reference));
        if !whole_dest {
            return None;
        }
        let (assignment, operands) =
            build_whole_vector_map_assignment(fn_ir, scop, *base, expr_root)?;
        if reference_dest.is_none() {
            reference_dest = Some(*base);
        }
        assignments.push(assignment);
        guards.push(operands);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

fn build_single_whole_vector_map_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, VectorMapOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root))
                if subscripts.len() == 1 =>
            {
                Some((stmt, *base, subscripts.clone(), expr_root))
            }
            _ => None,
        });
    let (stmt, dest, _subscripts, expr_root) = stores.next()?;
    if stores.next().is_some() {
        return None;
    }
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == 1
            && is_loop_iv_subscript(scop, &access.subscripts[0])
    })?;
    let _ = write;
    if !loop_covers_whole_vector(fn_ir, lp, scop, dest) {
        return None;
    }
    build_whole_vector_map_assignment(fn_ir, scop, dest, expr_root)
}

fn build_single_range_vector_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, VectorMapOperands)> {
    if scop.dimensions.len() != 1 || scop.dimensions[0].step != 1 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root))
                if subscripts.len() == 1 =>
            {
                Some((stmt, *base, expr_root))
            }
            _ => None,
        });
    let (stmt, dest, expr_root) = stores.next()?;
    if stores.next().is_some() {
        return None;
    }
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == 1
            && is_loop_iv_subscript(scop, &access.subscripts[0])
    })?;
    let _ = write;
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
        return None;
    };
    if !matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
    ) {
        return None;
    }

    let lhs_src = if let Some(base) = index_reads_loop_vector(fn_ir, scop, lhs) {
        base
    } else if is_scalarish_value(fn_ir, lhs) {
        lhs
    } else {
        return None;
    };
    let rhs_src = if let Some(base) = index_reads_loop_vector(fn_ir, scop, rhs) {
        base
    } else if is_scalarish_value(fn_ir, rhs) {
        rhs
    } else {
        return None;
    };

    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let lhs_slice = prepare_partial_slice_value(fn_ir, dest, lhs_src, start, end);
    let rhs_slice = prepare_partial_slice_value(fn_ir, dest, rhs_src, start, end);
    let expr_vec = fn_ir.add_value(
        ValueKind::Binary {
            op,
            lhs: lhs_slice,
            rhs: rhs_slice,
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = build_slice_assignment_value(fn_ir, dest, start, end, expr_vec);

    Some((
        PreparedVectorAssignment {
            dest_var: base_symbol_name(fn_ir, dest),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        VectorMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

fn build_multi_range_vector_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<VectorMapOperands>)> {
    if scop.dimensions.len() != 1 || scop.dimensions[0].step != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 1 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 1
                && is_loop_iv_subscript(scop, &access.subscripts[0])
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var.clone()) {
            return None;
        }

        let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
        let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
            return None;
        };
        if !matches!(
            op,
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
        ) {
            return None;
        }

        let lhs_src = if let Some(base) = index_reads_loop_vector(fn_ir, scop, lhs) {
            base
        } else if is_scalarish_value(fn_ir, lhs) {
            lhs
        } else {
            return None;
        };
        let rhs_src = if let Some(base) = index_reads_loop_vector(fn_ir, scop, rhs) {
            base
        } else if is_scalarish_value(fn_ir, rhs) {
            rhs
        } else {
            return None;
        };

        let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
        let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
        let lhs_slice = prepare_partial_slice_value(fn_ir, *base, lhs_src, start, end);
        let rhs_slice = prepare_partial_slice_value(fn_ir, *base, rhs_src, start, end);
        let expr_vec = fn_ir.add_value(
            ValueKind::Binary {
                op,
                lhs: lhs_slice,
                rhs: rhs_slice,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
        let out_val = build_slice_assignment_value(fn_ir, *base, start, end, expr_vec);
        assignments.push(PreparedVectorAssignment {
            dest_var,
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        });
        guards.push(VectorMapOperands {
            dest: *base,
            lhs_src,
            rhs_src,
        });
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

fn build_whole_vector_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, VectorReduceOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let (kind, base) = match &fn_ir.values[expr_root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let kind = if *op == BinOp::Add {
                ReduceKind::Sum
            } else {
                ReduceKind::Prod
            };
            let lhs_self = is_same_named_value(fn_ir, *lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, *rhs, dst);
            let read_base = if lhs_self {
                index_reads_loop_vector(fn_ir, scop, *rhs)
            } else if rhs_self {
                index_reads_loop_vector(fn_ir, scop, *lhs)
            } else {
                None
            }?;
            (kind, read_base)
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(expr_root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let kind = if matches!(
                fn_ir.call_semantics(expr_root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                ReduceKind::Min
            } else {
                ReduceKind::Max
            };
            let lhs_self = is_same_named_value(fn_ir, args[0], dst);
            let rhs_self = is_same_named_value(fn_ir, args[1], dst);
            let read_base = if lhs_self {
                index_reads_loop_vector(fn_ir, scop, args[1])
            } else if rhs_self {
                index_reads_loop_vector(fn_ir, scop, args[0])
            } else {
                None
            }?;
            (kind, read_base)
        }
        _ => return None,
    };

    if scop.dimensions[0].step != 1 {
        return None;
    }

    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );

    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_reduce_range".to_string(),
            args: vec![base, start, end, op_lit],
            names: vec![None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );

    let out_val = if let Some(seed) = seed_assignment_outside_loop(fn_ir, lp, dst) {
        match kind {
            ReduceKind::Sum | ReduceKind::Prod => fn_ir.add_value(
                ValueKind::Binary {
                    op: if kind == ReduceKind::Sum {
                        BinOp::Add
                    } else {
                        BinOp::Mul
                    },
                    lhs: seed,
                    rhs: reduce_val,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            ReduceKind::Min | ReduceKind::Max => fn_ir.add_value(
                ValueKind::Call {
                    callee: reduction_op_symbol(kind).to_string(),
                    args: vec![seed, reduce_val],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
        }
    } else {
        reduce_val
    };

    Some((
        PreparedVectorAssignment {
            dest_var: dst.to_string(),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        VectorReduceOperands { base, start, end },
    ))
}

fn build_single_whole_vector_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, VectorReduceOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut assignments =
        scop.statements
            .iter()
            .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
                (super::PolyStmtKind::Assign { dst }, Some(expr_root)) => {
                    build_whole_vector_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
                }
                _ => None,
            });
    let assignment = assignments.next()?;
    if assignments.next().is_some() {
        return None;
    }
    Some(assignment)
}

fn build_multi_whole_vector_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<VectorReduceOperands>)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        let Some((assignment, guard)) =
            build_whole_vector_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
        else {
            continue;
        };
        if !seen_dests.insert(assignment.dest_var.clone()) {
            return None;
        }
        assignments.push(assignment);
        guards.push(guard);
    }
    (!assignments.is_empty()).then_some((assignments, guards))
}

fn build_2d_col_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    dest: ValueId,
    fixed_col: ValueId,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, MatrixMapOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
        return None;
    };
    if !matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
    ) {
        return None;
    }
    let lhs_src = if let Some(base) = index_reads_2d_col_vector(fn_ir, scop, lhs, fixed_col) {
        base
    } else if is_scalarish_value(fn_ir, lhs) {
        lhs
    } else {
        return None;
    };
    let rhs_src = if let Some(base) = index_reads_2d_col_vector(fn_ir, scop, rhs, fixed_col) {
        base
    } else if is_scalarish_value(fn_ir, rhs) {
        rhs
    } else {
        return None;
    };
    let op_sym = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%%",
        _ => return None,
    };
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(op_sym.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_col_binop_assign".to_string(),
            args: vec![dest, lhs_src, rhs_src, fixed_col, start, end, op_lit],
            names: vec![None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some((
        PreparedVectorAssignment {
            dest_var: base_symbol_name(fn_ir, dest),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        MatrixMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

fn build_multi_2d_col_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<MatrixMapOperands>)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 2 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 2
                && is_loop_iv_subscript(scop, &access.subscripts[0])
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var) {
            return None;
        }
        let (assignment, operands) =
            build_2d_col_map_assignment(fn_ir, scop, *base, subscripts[1], expr_root)?;
        assignments.push(assignment);
        guards.push(operands);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

fn build_single_2d_col_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, MatrixMapOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root))
                if subscripts.len() == 2 =>
            {
                Some((stmt, *base, subscripts.clone(), expr_root))
            }
            _ => None,
        });
    let (stmt, dest, subscripts, expr_root) = stores.next()?;
    if stores.next().is_some() {
        return None;
    }
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == 2
            && is_loop_iv_subscript(scop, &access.subscripts[0])
    })?;
    let _ = write;
    build_2d_col_map_assignment(fn_ir, scop, dest, subscripts[1], expr_root)
}

fn build_3d_dim1_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    dest: ValueId,
    fixed_a: ValueId,
    fixed_b: ValueId,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, Array3MapOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
        return None;
    };
    if !matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
    ) {
        return None;
    }
    let lhs_src = if let Some(base) = index_reads_3d_dim1_vector(fn_ir, scop, lhs, fixed_a, fixed_b)
    {
        base
    } else if is_scalarish_value(fn_ir, lhs) {
        lhs
    } else {
        return None;
    };
    let rhs_src = if let Some(base) = index_reads_3d_dim1_vector(fn_ir, scop, rhs, fixed_a, fixed_b)
    {
        base
    } else if is_scalarish_value(fn_ir, rhs) {
        rhs
    } else {
        return None;
    };
    let op_sym = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%%",
        _ => return None,
    };
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(op_sym.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_dim1_binop_assign".to_string(),
            args: vec![dest, lhs_src, rhs_src, fixed_a, fixed_b, start, end, op_lit],
            names: vec![None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some((
        PreparedVectorAssignment {
            dest_var: base_symbol_name(fn_ir, dest),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        Array3MapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

fn build_multi_3d_dim1_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<Array3MapOperands>)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 3 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 3
                && is_loop_iv_subscript(scop, &access.subscripts[0])
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var) {
            return None;
        }
        let (assignment, operands) = build_3d_dim1_map_assignment(
            fn_ir,
            scop,
            *base,
            subscripts[1],
            subscripts[2],
            expr_root,
        )?;
        assignments.push(assignment);
        guards.push(operands);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

fn build_single_3d_dim1_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, Array3MapOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root))
                if subscripts.len() == 3 =>
            {
                Some((stmt, *base, subscripts.clone(), expr_root))
            }
            _ => None,
        });
    let (stmt, dest, subscripts, expr_root) = stores.next()?;
    if stores.next().is_some() {
        return None;
    }
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == 3
            && is_loop_iv_subscript(scop, &access.subscripts[0])
    })?;
    let _ = write;
    build_3d_dim1_map_assignment(fn_ir, scop, dest, subscripts[1], subscripts[2], expr_root)
}

fn build_2d_col_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, MatrixColReduceOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let (kind, base, col) = match &fn_ir.values[expr_root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let kind = if *op == BinOp::Add {
                ReduceKind::Sum
            } else {
                ReduceKind::Prod
            };
            let lhs_self = is_same_named_value(fn_ir, *lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, *rhs, dst);
            let other = if lhs_self {
                *rhs
            } else if rhs_self {
                *lhs
            } else {
                return None;
            };
            let other_root = resolve_scop_local_source(fn_ir, scop, other);
            let ValueKind::Index2D { base, c, .. } = &fn_ir.values[other_root].kind else {
                return None;
            };
            index_reads_2d_col_vector(fn_ir, scop, other_root, *c)?;
            (kind, *base, *c)
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(expr_root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let kind = if matches!(
                fn_ir.call_semantics(expr_root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                ReduceKind::Min
            } else {
                ReduceKind::Max
            };
            let lhs_self = is_same_named_value(fn_ir, args[0], dst);
            let rhs_self = is_same_named_value(fn_ir, args[1], dst);
            let other = if lhs_self {
                args[1]
            } else if rhs_self {
                args[0]
            } else {
                return None;
            };
            let other_root = resolve_scop_local_source(fn_ir, scop, other);
            let ValueKind::Index2D { base, c, .. } = &fn_ir.values[other_root].kind else {
                return None;
            };
            index_reads_2d_col_vector(fn_ir, scop, other_root, *c)?;
            (kind, *base, *c)
        }
        _ => return None,
    };
    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_col_reduce_range".to_string(),
            args: vec![base, col, start, end, op_lit],
            names: vec![None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = if let Some(seed) = seed_assignment_outside_loop(fn_ir, lp, dst) {
        match kind {
            ReduceKind::Sum | ReduceKind::Prod => fn_ir.add_value(
                ValueKind::Binary {
                    op: if kind == ReduceKind::Sum {
                        BinOp::Add
                    } else {
                        BinOp::Mul
                    },
                    lhs: seed,
                    rhs: reduce_val,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            ReduceKind::Min | ReduceKind::Max => fn_ir.add_value(
                ValueKind::Call {
                    callee: reduction_op_symbol(kind).to_string(),
                    args: vec![seed, reduce_val],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
        }
    } else {
        reduce_val
    };
    Some((
        PreparedVectorAssignment {
            dest_var: dst.to_string(),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        MatrixColReduceOperands {
            base,
            col,
            start,
            end,
        },
    ))
}

fn build_multi_2d_col_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<MatrixColReduceOperands>)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        let Some((assignment, guard)) =
            build_2d_col_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
        else {
            continue;
        };
        if !seen_dests.insert(assignment.dest_var.clone()) {
            return None;
        }
        assignments.push(assignment);
        guards.push(guard);
    }
    (!assignments.is_empty()).then_some((assignments, guards))
}

fn build_3d_dim1_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, Array3Dim1ReduceOperands)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let (kind, base, fixed_a, fixed_b) = match &fn_ir.values[expr_root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let kind = if *op == BinOp::Add {
                ReduceKind::Sum
            } else {
                ReduceKind::Prod
            };
            let lhs_self = is_same_named_value(fn_ir, *lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, *rhs, dst);
            let other = if lhs_self {
                *rhs
            } else if rhs_self {
                *lhs
            } else {
                return None;
            };
            let other_root = resolve_scop_local_source(fn_ir, scop, other);
            let ValueKind::Index3D { base, j, k, .. } = &fn_ir.values[other_root].kind else {
                return None;
            };
            index_reads_3d_dim1_vector(fn_ir, scop, other_root, *j, *k)?;
            (kind, *base, *j, *k)
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(expr_root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let kind = if matches!(
                fn_ir.call_semantics(expr_root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                ReduceKind::Min
            } else {
                ReduceKind::Max
            };
            let lhs_self = is_same_named_value(fn_ir, args[0], dst);
            let rhs_self = is_same_named_value(fn_ir, args[1], dst);
            let other = if lhs_self {
                args[1]
            } else if rhs_self {
                args[0]
            } else {
                return None;
            };
            let other_root = resolve_scop_local_source(fn_ir, scop, other);
            let ValueKind::Index3D { base, j, k, .. } = &fn_ir.values[other_root].kind else {
                return None;
            };
            index_reads_3d_dim1_vector(fn_ir, scop, other_root, *j, *k)?;
            (kind, *base, *j, *k)
        }
        _ => return None,
    };
    let start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let reduce_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_dim1_reduce_range".to_string(),
            args: vec![base, fixed_a, fixed_b, start, end, op_lit],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = if let Some(seed) = seed_assignment_outside_loop(fn_ir, lp, dst) {
        match kind {
            ReduceKind::Sum | ReduceKind::Prod => fn_ir.add_value(
                ValueKind::Binary {
                    op: if kind == ReduceKind::Sum {
                        BinOp::Add
                    } else {
                        BinOp::Mul
                    },
                    lhs: seed,
                    rhs: reduce_val,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            ReduceKind::Min | ReduceKind::Max => fn_ir.add_value(
                ValueKind::Call {
                    callee: reduction_op_symbol(kind).to_string(),
                    args: vec![seed, reduce_val],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
        }
    } else {
        reduce_val
    };
    Some((
        PreparedVectorAssignment {
            dest_var: dst.to_string(),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        Array3Dim1ReduceOperands {
            base,
            fixed_a,
            fixed_b,
            start,
            end,
        },
    ))
}

fn build_multi_3d_dim1_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<Array3Dim1ReduceOperands>)> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        let Some((assignment, guard)) =
            build_3d_dim1_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
        else {
            continue;
        };
        if !seen_dests.insert(assignment.dest_var.clone()) {
            return None;
        }
        assignments.push(assignment);
        guards.push(guard);
    }
    (!assignments.is_empty()).then_some((assignments, guards))
}

fn build_nested_2d_full_matrix_map_value(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(ValueId, String, MatrixMapOperands)> {
    if scop.dimensions.len() != 2 {
        return None;
    }
    let mut stores = scop
        .statements
        .iter()
        .filter_map(|stmt| match (&stmt.kind, stmt.expr_root) {
            (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root))
                if subscripts.len() == 2 =>
            {
                Some((stmt, *base, subscripts.clone(), expr_root))
            }
            _ => None,
        });
    let (stmt, dest, subscripts, expr_root) = stores.next()?;
    if stores.next().is_some() {
        return None;
    }
    let outer_name = &scop.dimensions[0].iv_name;
    let inner_name = &scop.dimensions[1].iv_name;
    let write = stmt.accesses.iter().find(|access| {
        matches!(access.kind, super::access::AccessKind::Write)
            && access.memref.base == dest
            && access.subscripts.len() == 2
            && matches!(
                access.subscripts[0].terms.iter().next(),
                Some((super::affine::AffineSymbol::LoopIv(name), coeff)) if name == outer_name && *coeff == 1
            )
            && matches!(
                access.subscripts[1].terms.iter().next(),
                Some((super::affine::AffineSymbol::LoopIv(name), coeff)) if name == inner_name && *coeff == 1
            )
    })?;
    let _ = write;
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
        return None;
    };
    if !matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
    ) {
        return None;
    }
    let lhs_root = resolve_scop_local_source(fn_ir, scop, lhs);
    let rhs_root = resolve_scop_local_source(fn_ir, scop, rhs);
    let lhs_src = match &fn_ir.values[lhs_root].kind {
        ValueKind::Index2D { base, r, c }
            if is_same_scalar_value(fn_ir, *r, subscripts[0])
                && is_same_scalar_value(fn_ir, *c, subscripts[1]) =>
        {
            *base
        }
        _ if is_scalarish_value(fn_ir, lhs_root) => lhs_root,
        _ => return None,
    };
    let rhs_src = match &fn_ir.values[rhs_root].kind {
        ValueKind::Index2D { base, r, c }
            if is_same_scalar_value(fn_ir, *r, subscripts[0])
                && is_same_scalar_value(fn_ir, *c, subscripts[1]) =>
        {
            *base
        }
        _ if is_scalarish_value(fn_ir, rhs_root) => rhs_root,
        _ => return None,
    };
    let op_sym = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%%",
        _ => return None,
    };
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(op_sym.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let out_val = {
        let r_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
        let r_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
        let c_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound)?;
        let c_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound)?;
        let args = vec![
            dest, lhs_src, rhs_src, r_start, r_end, c_start, c_end, op_lit,
        ];
        fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_matrix_binop_assign".to_string(),
                args,
                names: vec![None, None, None, None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        )
    };
    Some((
        out_val,
        base_symbol_name(fn_ir, dest),
        MatrixMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

fn build_nested_2d_full_matrix_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    dest: ValueId,
    subscripts: &[ValueId],
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, MatrixMapOperands)> {
    if scop.dimensions.len() != 2 || subscripts.len() != 2 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
        return None;
    };
    if !matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
    ) {
        return None;
    }
    let lhs_root = resolve_scop_local_source(fn_ir, scop, lhs);
    let rhs_root = resolve_scop_local_source(fn_ir, scop, rhs);
    let lhs_src = match &fn_ir.values[lhs_root].kind {
        ValueKind::Index2D { base, r, c }
            if is_same_scalar_value(fn_ir, *r, subscripts[0])
                && is_same_scalar_value(fn_ir, *c, subscripts[1]) =>
        {
            *base
        }
        _ if is_scalarish_value(fn_ir, lhs_root) => lhs_root,
        _ => return None,
    };
    let rhs_src = match &fn_ir.values[rhs_root].kind {
        ValueKind::Index2D { base, r, c }
            if is_same_scalar_value(fn_ir, *r, subscripts[0])
                && is_same_scalar_value(fn_ir, *c, subscripts[1]) =>
        {
            *base
        }
        _ if is_scalarish_value(fn_ir, rhs_root) => rhs_root,
        _ => return None,
    };
    let op_sym = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%%",
        _ => return None,
    };
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(op_sym.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let r_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let r_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let c_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound)?;
    let c_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound)?;
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_matrix_binop_assign".to_string(),
            args: vec![
                dest, lhs_src, rhs_src, r_start, r_end, c_start, c_end, op_lit,
            ],
            names: vec![None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some((
        PreparedVectorAssignment {
            dest_var: base_symbol_name(fn_ir, dest),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        MatrixMapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

fn build_multi_nested_2d_full_matrix_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<MatrixMapOperands>)> {
    if scop.dimensions.len() != 2 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    let outer_name = &scop.dimensions[0].iv_name;
    let inner_name = &scop.dimensions[1].iv_name;
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 2 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 2
                && matches!(
                    access.subscripts[0].terms.iter().next(),
                    Some((super::affine::AffineSymbol::LoopIv(name), coeff))
                        if name == outer_name && *coeff == 1
                )
                && matches!(
                    access.subscripts[1].terms.iter().next(),
                    Some((super::affine::AffineSymbol::LoopIv(name), coeff))
                        if name == inner_name && *coeff == 1
                )
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var.clone()) {
            return None;
        }
        let (assignment, operands) =
            build_nested_2d_full_matrix_map_assignment(fn_ir, scop, *base, subscripts, expr_root)?;
        assignments.push(assignment);
        guards.push(operands);
    }
    (assignments.len() >= 2).then_some((assignments, guards))
}

fn emit_matrix_map_guards(
    fn_ir: &mut FnIR,
    preheader: crate::mir::BlockId,
    operands: &[MatrixMapOperands],
) {
    for operand in operands {
        emit_same_matrix_shape_or_scalar_guard(fn_ir, preheader, operand.dest, operand.lhs_src);
        emit_same_matrix_shape_or_scalar_guard(fn_ir, preheader, operand.dest, operand.rhs_src);
    }
}

fn emit_array3_map_guards(
    fn_ir: &mut FnIR,
    preheader: crate::mir::BlockId,
    operands: &[Array3MapOperands],
) {
    for operand in operands {
        emit_same_array3_shape_or_scalar_guard(fn_ir, preheader, operand.dest, operand.lhs_src);
        emit_same_array3_shape_or_scalar_guard(fn_ir, preheader, operand.dest, operand.rhs_src);
    }
}

fn build_guard_bool(fn_ir: &mut FnIR, callee: &str, lhs: ValueId, rhs: ValueId) -> ValueId {
    fn_ir.add_value(
        ValueKind::Call {
            callee: callee.to_string(),
            args: vec![lhs, rhs],
            names: vec![None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    )
}

fn combine_guard_terms(fn_ir: &mut FnIR, terms: Vec<ValueId>) -> Option<ValueId> {
    let mut iter = terms.into_iter();
    let mut acc = iter.next()?;
    for term in iter {
        acc = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::And,
                lhs: acc,
                rhs: term,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        );
    }
    Some(acc)
}

fn build_matrix_map_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[MatrixMapOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_matrix_shape_or_scalar",
            operand.dest,
            operand.lhs_src,
        ));
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_matrix_shape_or_scalar",
            operand.dest,
            operand.rhs_src,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_vector_map_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[VectorMapOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_or_scalar",
            operand.dest,
            operand.lhs_src,
        ));
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_or_scalar",
            operand.dest,
            operand.rhs_src,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_vector_reduce_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[VectorReduceOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_can_reduce_range".to_string(),
                args: vec![operand.base, operand.start, operand.end],
                names: vec![None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_array3_map_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[Array3MapOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_array3_shape_or_scalar",
            operand.dest,
            operand.lhs_src,
        ));
        terms.push(build_guard_bool(
            fn_ir,
            "rr_can_same_array3_shape_or_scalar",
            operand.dest,
            operand.rhs_src,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_matrix_rect_reduce_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[MatrixRectReduceOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_can_matrix_reduce_rect".to_string(),
                args: vec![
                    operand.base,
                    operand.r_start,
                    operand.r_end,
                    operand.c_start,
                    operand.c_end,
                ],
                names: vec![None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_matrix_col_reduce_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[MatrixColReduceOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_can_col_reduce_range".to_string(),
                args: vec![operand.base, operand.col, operand.start, operand.end],
                names: vec![None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_array3_dim1_reduce_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[Array3Dim1ReduceOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_can_dim1_reduce_range".to_string(),
                args: vec![
                    operand.base,
                    operand.fixed_a,
                    operand.fixed_b,
                    operand.start,
                    operand.end,
                ],
                names: vec![None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn build_array3_cube_reduce_guard_cond(
    fn_ir: &mut FnIR,
    operands: &[Array3CubeReduceOperands],
) -> Option<ValueId> {
    let mut terms = Vec::new();
    for operand in operands {
        terms.push(fn_ir.add_value(
            ValueKind::Call {
                callee: "rr_can_array3_reduce_cube".to_string(),
                args: vec![
                    operand.base,
                    operand.i_start,
                    operand.i_end,
                    operand.j_start,
                    operand.j_end,
                    operand.k_start,
                    operand.k_end,
                ],
                names: vec![None, None, None, None, None, None, None],
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ));
    }
    combine_guard_terms(fn_ir, terms)
}

fn reduction_op_symbol(kind: ReduceKind) -> &'static str {
    match kind {
        ReduceKind::Sum => "sum",
        ReduceKind::Prod => "prod",
        ReduceKind::Min => "min",
        ReduceKind::Max => "max",
    }
}

fn is_same_named_value(fn_ir: &FnIR, value: ValueId, name: &str) -> bool {
    fn rec(
        fn_ir: &FnIR,
        value: ValueId,
        name: &str,
        seen: &mut rustc_hash::FxHashSet<ValueId>,
    ) -> bool {
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

    rec(fn_ir, value, name, &mut rustc_hash::FxHashSet::default())
}

fn index_reads_nested_2d_rect(fn_ir: &FnIR, scop: &ScopRegion, value: ValueId) -> Option<ValueId> {
    if scop.dimensions.len() != 2 {
        return None;
    }
    let value = resolve_scop_local_source(fn_ir, scop, value);
    match &fn_ir.values[value].kind {
        ValueKind::Index2D { base, .. } => {
            let read = scop
                .statements
                .iter()
                .flat_map(|stmt| stmt.accesses.iter())
                .find(|access| {
                    matches!(access.kind, super::access::AccessKind::Read)
                        && access.memref.base == *base
                        && access.subscripts.len() == 2
                })?;
            Some(read.memref.base)
        }
        _ => None,
    }
}

fn seed_assignment_outside_loop(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> Option<ValueId> {
    let mut seed = None;
    for (bid, block) in fn_ir.blocks.iter().enumerate() {
        if lp.body.contains(&bid) {
            continue;
        }
        for instr in &block.instrs {
            let crate::mir::Instr::Assign { dst, src, .. } = instr else {
                continue;
            };
            if dst == var {
                seed = Some(*src);
            }
        }
    }
    seed
}

fn build_nested_2d_full_matrix_reduce_value(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, MatrixRectReduceOperands)> {
    let (mut assignments, mut guards) =
        build_multi_nested_2d_full_matrix_reduce_assignments(fn_ir, lp, scop)?;
    if assignments.len() != 1 {
        return None;
    }
    let assignment = assignments.pop()?;
    let guard = guards.pop()?;
    Some((assignment, guard))
}

fn build_nested_2d_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, MatrixRectReduceOperands)> {
    if scop.dimensions.len() != 2 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let (kind, base) = match &fn_ir.values[expr_root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let kind = if *op == BinOp::Add {
                ReduceKind::Sum
            } else {
                ReduceKind::Prod
            };
            let lhs_self = is_same_named_value(fn_ir, *lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, *rhs, dst);
            let read_base = if lhs_self {
                index_reads_nested_2d_rect(fn_ir, scop, *rhs)
            } else if rhs_self {
                index_reads_nested_2d_rect(fn_ir, scop, *lhs)
            } else {
                None
            }?;
            (kind, read_base)
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(expr_root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let kind = if matches!(
                fn_ir.call_semantics(expr_root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                ReduceKind::Min
            } else {
                ReduceKind::Max
            };
            let lhs_self = is_same_named_value(fn_ir, args[0], dst);
            let rhs_self = is_same_named_value(fn_ir, args[1], dst);
            let read_base = if lhs_self {
                index_reads_nested_2d_rect(fn_ir, scop, args[1])
            } else if rhs_self {
                index_reads_nested_2d_rect(fn_ir, scop, args[0])
            } else {
                None
            }?;
            (kind, read_base)
        }
        _ => return None,
    };
    let r_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let r_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let c_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound)?;
    let c_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let rect_reduce = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_matrix_reduce_rect".to_string(),
            args: vec![base, r_start, r_end, c_start, c_end, op_lit],
            names: vec![None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );

    let out_val = if let Some(seed) = seed_assignment_outside_loop(fn_ir, lp, dst) {
        match kind {
            ReduceKind::Sum | ReduceKind::Prod => fn_ir.add_value(
                ValueKind::Binary {
                    op: if kind == ReduceKind::Sum {
                        BinOp::Add
                    } else {
                        BinOp::Mul
                    },
                    lhs: seed,
                    rhs: rect_reduce,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            ReduceKind::Min | ReduceKind::Max => fn_ir.add_value(
                ValueKind::Call {
                    callee: reduction_op_symbol(kind).to_string(),
                    args: vec![seed, rect_reduce],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
        }
    } else {
        rect_reduce
    };

    Some((
        PreparedVectorAssignment {
            dest_var: dst.to_string(),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        MatrixRectReduceOperands {
            base,
            r_start,
            r_end,
            c_start,
            c_end,
        },
    ))
}

fn build_multi_nested_2d_full_matrix_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<MatrixRectReduceOperands>)> {
    if scop.dimensions.len() != 2 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        let Some((assignment, guard)) =
            build_nested_2d_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
        else {
            continue;
        };
        if !seen_dests.insert(assignment.dest_var.clone()) {
            return None;
        }
        assignments.push(assignment);
        guards.push(guard);
    }
    (!assignments.is_empty()).then_some((assignments, guards))
}

fn index_reads_nested_3d_cube(fn_ir: &FnIR, scop: &ScopRegion, value: ValueId) -> Option<ValueId> {
    if scop.dimensions.len() != 3 {
        return None;
    }
    let value = resolve_scop_local_source(fn_ir, scop, value);
    match &fn_ir.values[value].kind {
        ValueKind::Index3D { base, .. } => {
            let read = scop
                .statements
                .iter()
                .flat_map(|stmt| stmt.accesses.iter())
                .find(|access| {
                    matches!(access.kind, super::access::AccessKind::Read)
                        && access.memref.base == *base
                        && access.subscripts.len() == 3
                })?;
            Some(read.memref.base)
        }
        _ => None,
    }
}

fn build_nested_3d_full_cube_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
    dest: ValueId,
    subscripts: &[ValueId],
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, Array3MapOperands)> {
    if scop.dimensions.len() != 3 || subscripts.len() != 3 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[expr_root].kind else {
        return None;
    };
    if !matches!(
        op,
        BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod
    ) {
        return None;
    }
    let lhs_root = resolve_scop_local_source(fn_ir, scop, lhs);
    let rhs_root = resolve_scop_local_source(fn_ir, scop, rhs);
    let lhs_src = match &fn_ir.values[lhs_root].kind {
        ValueKind::Index3D { base, i, j, k }
            if is_same_scalar_value(fn_ir, *i, subscripts[0])
                && is_same_scalar_value(fn_ir, *j, subscripts[1])
                && is_same_scalar_value(fn_ir, *k, subscripts[2]) =>
        {
            *base
        }
        _ if is_scalarish_value(fn_ir, lhs_root) => lhs_root,
        _ => return None,
    };
    let rhs_src = match &fn_ir.values[rhs_root].kind {
        ValueKind::Index3D { base, i, j, k }
            if is_same_scalar_value(fn_ir, *i, subscripts[0])
                && is_same_scalar_value(fn_ir, *j, subscripts[1])
                && is_same_scalar_value(fn_ir, *k, subscripts[2]) =>
        {
            *base
        }
        _ if is_scalarish_value(fn_ir, rhs_root) => rhs_root,
        _ => return None,
    };
    let op_sym = match op {
        BinOp::Add => "+",
        BinOp::Sub => "-",
        BinOp::Mul => "*",
        BinOp::Div => "/",
        BinOp::Mod => "%%",
        _ => return None,
    };
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(op_sym.to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let i_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let i_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let j_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound)?;
    let j_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound)?;
    let k_start = encode_bound(fn_ir, &scop.dimensions[2].lower_bound)?;
    let k_end = encode_bound(fn_ir, &scop.dimensions[2].upper_bound)?;
    let out_val = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_array3_binop_cube_assign".to_string(),
            args: vec![
                dest, lhs_src, rhs_src, i_start, i_end, j_start, j_end, k_start, k_end, op_lit,
            ],
            names: vec![None, None, None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    Some((
        PreparedVectorAssignment {
            dest_var: base_symbol_name(fn_ir, dest),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        Array3MapOperands {
            dest,
            lhs_src,
            rhs_src,
        },
    ))
}

fn build_multi_nested_3d_full_cube_map_assignments(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<Array3MapOperands>)> {
    if scop.dimensions.len() != 3 {
        return None;
    }
    let i_name = &scop.dimensions[0].iv_name;
    let j_name = &scop.dimensions[1].iv_name;
    let k_name = &scop.dimensions[2].iv_name;
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Store { base, subscripts }, Some(expr_root)) =
            (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        if subscripts.len() != 3 {
            return None;
        }
        let write = stmt.accesses.iter().find(|access| {
            matches!(access.kind, super::access::AccessKind::Write)
                && access.memref.base == *base
                && access.subscripts.len() == 3
                && is_named_loop_iv_subscript(i_name, &access.subscripts[0])
                && is_named_loop_iv_subscript(j_name, &access.subscripts[1])
                && is_named_loop_iv_subscript(k_name, &access.subscripts[2])
        })?;
        let _ = write;
        let dest_var = base_symbol_name(fn_ir, *base);
        if !seen_dests.insert(dest_var) {
            return None;
        }
        let (assignment, operands) =
            build_nested_3d_full_cube_map_assignment(fn_ir, scop, *base, subscripts, expr_root)?;
        assignments.push(assignment);
        guards.push(operands);
    }
    (!assignments.is_empty()).then_some((assignments, guards))
}

fn build_single_nested_3d_full_cube_map_assignment(
    fn_ir: &mut FnIR,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, Array3MapOperands)> {
    let (mut assignments, mut guards) =
        build_multi_nested_3d_full_cube_map_assignments(fn_ir, scop)?;
    if assignments.len() != 1 {
        return None;
    }
    Some((assignments.pop()?, guards.pop()?))
}

fn build_nested_3d_full_cube_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    dst: &str,
    expr_root: ValueId,
) -> Option<(PreparedVectorAssignment, Array3CubeReduceOperands)> {
    if scop.dimensions.len() != 3 {
        return None;
    }
    let expr_root = resolve_scop_local_source(fn_ir, scop, expr_root);
    let (kind, base) = match &fn_ir.values[expr_root].kind {
        ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
            let kind = if *op == BinOp::Add {
                ReduceKind::Sum
            } else {
                ReduceKind::Prod
            };
            let lhs_self = is_same_named_value(fn_ir, *lhs, dst);
            let rhs_self = is_same_named_value(fn_ir, *rhs, dst);
            let read_base = if lhs_self {
                index_reads_nested_3d_cube(fn_ir, scop, *rhs)
            } else if rhs_self {
                index_reads_nested_3d_cube(fn_ir, scop, *lhs)
            } else {
                None
            }?;
            (kind, read_base)
        }
        ValueKind::Call { callee, args, .. }
            if args.len() == 2
                && (matches!(
                    fn_ir.call_semantics(expr_root),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                    ))
                ) || matches!(
                    callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                    "min" | "max"
                )) =>
        {
            let kind = if matches!(
                fn_ir.call_semantics(expr_root),
                Some(crate::mir::CallSemantics::Builtin(
                    crate::mir::BuiltinKind::Min
                ))
            ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
            {
                ReduceKind::Min
            } else {
                ReduceKind::Max
            };
            let lhs_self = is_same_named_value(fn_ir, args[0], dst);
            let rhs_self = is_same_named_value(fn_ir, args[1], dst);
            let read_base = if lhs_self {
                index_reads_nested_3d_cube(fn_ir, scop, args[1])
            } else if rhs_self {
                index_reads_nested_3d_cube(fn_ir, scop, args[0])
            } else {
                None
            }?;
            (kind, read_base)
        }
        _ => return None,
    };
    let i_start = encode_bound(fn_ir, &scop.dimensions[0].lower_bound)?;
    let i_end = encode_bound(fn_ir, &scop.dimensions[0].upper_bound)?;
    let j_start = encode_bound(fn_ir, &scop.dimensions[1].lower_bound)?;
    let j_end = encode_bound(fn_ir, &scop.dimensions[1].upper_bound)?;
    let k_start = encode_bound(fn_ir, &scop.dimensions[2].lower_bound)?;
    let k_end = encode_bound(fn_ir, &scop.dimensions[2].upper_bound)?;
    let op_lit = fn_ir.add_value(
        ValueKind::Const(Lit::Str(reduction_op_symbol(kind).to_string())),
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );
    let cube_reduce = fn_ir.add_value(
        ValueKind::Call {
            callee: "rr_array3_reduce_cube".to_string(),
            args: vec![base, i_start, i_end, j_start, j_end, k_start, k_end, op_lit],
            names: vec![None, None, None, None, None, None, None, None],
        },
        crate::utils::Span::dummy(),
        crate::mir::def::Facts::empty(),
        None,
    );

    let out_val = if let Some(seed) = seed_assignment_outside_loop(fn_ir, lp, dst) {
        match kind {
            ReduceKind::Sum | ReduceKind::Prod => fn_ir.add_value(
                ValueKind::Binary {
                    op: if kind == ReduceKind::Sum {
                        BinOp::Add
                    } else {
                        BinOp::Mul
                    },
                    lhs: seed,
                    rhs: cube_reduce,
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
            ReduceKind::Min | ReduceKind::Max => fn_ir.add_value(
                ValueKind::Call {
                    callee: reduction_op_symbol(kind).to_string(),
                    args: vec![seed, cube_reduce],
                    names: vec![None, None],
                },
                crate::utils::Span::dummy(),
                crate::mir::def::Facts::empty(),
                None,
            ),
        }
    } else {
        cube_reduce
    };

    Some((
        PreparedVectorAssignment {
            dest_var: dst.to_string(),
            out_val,
            shadow_vars: Vec::new(),
            shadow_idx: None,
        },
        Array3CubeReduceOperands {
            base,
            i_start,
            i_end,
            j_start,
            j_end,
            k_start,
            k_end,
        },
    ))
}

fn build_multi_nested_3d_full_cube_reduce_assignments(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(Vec<PreparedVectorAssignment>, Vec<Array3CubeReduceOperands>)> {
    if scop.dimensions.len() != 3 {
        return None;
    }
    let mut assignments = Vec::new();
    let mut guards = Vec::new();
    let mut seen_dests = std::collections::BTreeSet::new();
    for stmt in &scop.statements {
        let (super::PolyStmtKind::Assign { dst }, Some(expr_root)) = (&stmt.kind, stmt.expr_root)
        else {
            continue;
        };
        let Some((assignment, guard)) =
            build_nested_3d_full_cube_reduce_assignment(fn_ir, lp, scop, dst, expr_root)
        else {
            continue;
        };
        if !seen_dests.insert(assignment.dest_var.clone()) {
            return None;
        }
        assignments.push(assignment);
        guards.push(guard);
    }
    (!assignments.is_empty()).then_some((assignments, guards))
}

fn build_single_nested_3d_full_cube_reduce_assignment(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
) -> Option<(PreparedVectorAssignment, Array3CubeReduceOperands)> {
    let (mut assignments, mut guards) =
        build_multi_nested_3d_full_cube_reduce_assignments(fn_ir, lp, scop)?;
    if assignments.len() != 1 {
        return None;
    }
    Some((assignments.pop()?, guards.pop()?))
}

fn expr_contains_loop_read(fn_ir: &FnIR, scop: &ScopRegion, root: ValueId) -> bool {
    let root = resolve_scop_local_source(fn_ir, scop, root);
    match &fn_ir.values[root].kind {
        ValueKind::Index1D { .. } => index_reads_loop_vector(fn_ir, scop, root).is_some(),
        ValueKind::Index2D { c, .. } => index_reads_2d_col_vector(fn_ir, scop, root, *c).is_some(),
        ValueKind::Index3D { j, k, .. } => {
            index_reads_3d_dim1_vector(fn_ir, scop, root, *j, *k).is_some()
        }
        ValueKind::Binary { lhs, rhs, .. } => {
            expr_contains_loop_read(fn_ir, scop, *lhs) || expr_contains_loop_read(fn_ir, scop, *rhs)
        }
        ValueKind::Unary { rhs, .. } => expr_contains_loop_read(fn_ir, scop, *rhs),
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => args
            .iter()
            .any(|arg| expr_contains_loop_read(fn_ir, scop, *arg)),
        ValueKind::Phi { args } => args
            .iter()
            .any(|(arg, _)| expr_contains_loop_read(fn_ir, scop, *arg)),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            expr_contains_loop_read(fn_ir, scop, *base)
        }
        ValueKind::Range { start, end } => {
            expr_contains_loop_read(fn_ir, scop, *start)
                || expr_contains_loop_read(fn_ir, scop, *end)
        }
        _ => false,
    }
}

fn build_reduce_plan(fn_ir: &FnIR, lp: &LoopInfo, scop: &ScopRegion) -> Option<VectorPlan> {
    if scop.dimensions.len() != 1 {
        return None;
    }
    let iv_phi = lp.iv.as_ref()?.phi_val;

    for (id, value) in fn_ir.values.iter().enumerate() {
        let ValueKind::Phi { args } = &value.kind else {
            continue;
        };
        if value.phi_block != Some(lp.header) || args.len() != 2 {
            continue;
        }
        let Some((next_val, _)) = args.iter().find(|(_, pred)| *pred == lp.latch) else {
            continue;
        };
        let next = resolve_scop_local_source(fn_ir, scop, *next_val);

        match &fn_ir.values[next].kind {
            ValueKind::Binary { op, lhs, rhs } if matches!(op, BinOp::Add | BinOp::Mul) => {
                let other = if *lhs == id {
                    *rhs
                } else if *rhs == id {
                    *lhs
                } else {
                    continue;
                };
                let kind = if *op == BinOp::Add {
                    ReduceKind::Sum
                } else {
                    ReduceKind::Prod
                };
                let other_root = resolve_scop_local_source(fn_ir, scop, other);
                if let ValueKind::Index2D { base, c, .. } = &fn_ir.values[other_root].kind
                    && kind == ReduceKind::Sum
                    && index_reads_2d_col_vector(fn_ir, scop, other_root, *c).is_some()
                {
                    return Some(VectorPlan::Reduce2DColSum {
                        acc_phi: id,
                        base: *base,
                        col: *c,
                        start: lp.iv.as_ref()?.init_val,
                        end: lp.limit?,
                    });
                }
                if let ValueKind::Index3D { base, j, k, .. } = &fn_ir.values[other_root].kind
                    && index_reads_3d_dim1_vector(fn_ir, scop, other_root, *j, *k).is_some()
                {
                    return Some(VectorPlan::Reduce3D {
                        kind,
                        acc_phi: id,
                        base: *base,
                        axis: Axis3D::Dim1,
                        fixed_a: *j,
                        fixed_b: *k,
                        start: lp.iv.as_ref()?.init_val,
                        end: lp.limit?,
                    });
                }
                if !expr_contains_loop_read(fn_ir, scop, other) {
                    continue;
                }
                return Some(VectorPlan::Reduce {
                    kind,
                    acc_phi: id,
                    vec_expr: other,
                    iv_phi,
                });
            }
            ValueKind::Call { callee, args, .. }
                if args.len() == 2
                    && matches!(
                        fn_ir.call_semantics(next),
                        Some(crate::mir::CallSemantics::Builtin(
                            crate::mir::BuiltinKind::Min | crate::mir::BuiltinKind::Max
                        ))
                    )
                    || matches!(
                        callee.strip_prefix("base::").unwrap_or(callee.as_str()),
                        "min" | "max"
                    ) && args.len() == 2 =>
            {
                let other = if args[0] == id {
                    args[1]
                } else if args[1] == id {
                    args[0]
                } else {
                    continue;
                };
                let kind = if matches!(
                    fn_ir.call_semantics(next),
                    Some(crate::mir::CallSemantics::Builtin(
                        crate::mir::BuiltinKind::Min
                    ))
                ) || callee.strip_prefix("base::").unwrap_or(callee.as_str()) == "min"
                {
                    ReduceKind::Min
                } else {
                    ReduceKind::Max
                };
                let other_root = resolve_scop_local_source(fn_ir, scop, other);
                if let ValueKind::Index3D { base, j, k, .. } = &fn_ir.values[other_root].kind
                    && index_reads_3d_dim1_vector(fn_ir, scop, other_root, *j, *k).is_some()
                {
                    return Some(VectorPlan::Reduce3D {
                        kind,
                        acc_phi: id,
                        base: *base,
                        axis: Axis3D::Dim1,
                        fixed_a: *j,
                        fixed_b: *k,
                        start: lp.iv.as_ref()?.init_val,
                        end: lp.limit?,
                    });
                }
                if !expr_contains_loop_read(fn_ir, scop, other) {
                    continue;
                }
                return Some(VectorPlan::Reduce {
                    kind,
                    acc_phi: id,
                    vec_expr: other,
                    iv_phi,
                });
            }
            _ => {}
        }
    }
    if super::poly_trace_enabled() {
        eprintln!("   [poly-codegen] reduce reject: no matching reduction pattern");
    }
    None
}

fn build_identity_plan(fn_ir: &FnIR, lp: &LoopInfo, scop: &ScopRegion) -> Option<VectorPlan> {
    build_map_plan(fn_ir, lp, scop).or_else(|| build_reduce_plan(fn_ir, lp, scop))
}

fn replace_reduction_result_in_assignment(
    fn_ir: &mut FnIR,
    out_val: ValueId,
    new_reduce_val: ValueId,
) -> ValueId {
    match fn_ir.values[out_val].kind.clone() {
        ValueKind::Call { callee, .. } if callee == "rr_matrix_reduce_rect" => new_reduce_val,
        ValueKind::Call { callee, .. } if callee == "rr_reduce_range" => new_reduce_val,
        ValueKind::Call {
            callee,
            args,
            names,
        } if args.len() == 2 => fn_ir.add_value(
            ValueKind::Call {
                callee,
                args: vec![args[0], new_reduce_val],
                names,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ),
        ValueKind::Binary { op, lhs, .. } => fn_ir.add_value(
            ValueKind::Binary {
                op,
                lhs,
                rhs: new_reduce_val,
            },
            crate::utils::Span::dummy(),
            crate::mir::def::Facts::empty(),
            None,
        ),
        _ => new_reduce_val,
    }
}

fn combine_optional_same(lhs: Option<ValueId>, rhs: Option<ValueId>) -> Option<ValueId> {
    match (lhs, rhs) {
        (Some(a), Some(b)) if a == b => Some(a),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        _ => None,
    }
}

fn extract_reduction_op_literal(fn_ir: &FnIR, value: ValueId) -> Option<ValueId> {
    match &fn_ir.values[value].kind {
        ValueKind::Call { callee, args, .. }
            if matches!(
                callee.as_str(),
                "rr_reduce_range"
                    | "rr_col_reduce_range"
                    | "rr_matrix_reduce_rect"
                    | "rr_dim1_reduce_range"
                    | "rr_array3_reduce_cube"
                    | "rr_tile_reduce_range"
                    | "rr_tile_col_reduce_range"
                    | "rr_tile_matrix_reduce_rect"
                    | "rr_tile_dim1_reduce_range"
                    | "rr_tile_array3_reduce_cube"
            ) =>
        {
            args.last().copied()
        }
        ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
            args.iter().fold(None, |acc, arg| {
                combine_optional_same(acc, extract_reduction_op_literal(fn_ir, *arg))
            })
        }
        ValueKind::Binary { lhs, rhs, .. } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *lhs),
            extract_reduction_op_literal(fn_ir, *rhs),
        ),
        ValueKind::Unary { rhs, .. } => extract_reduction_op_literal(fn_ir, *rhs),
        ValueKind::RecordLit { fields } => fields.iter().fold(None, |acc, (_, value)| {
            combine_optional_same(acc, extract_reduction_op_literal(fn_ir, *value))
        }),
        ValueKind::FieldGet { base, .. } => extract_reduction_op_literal(fn_ir, *base),
        ValueKind::FieldSet { base, value, .. } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *base),
            extract_reduction_op_literal(fn_ir, *value),
        ),
        ValueKind::Phi { args } => args.iter().fold(None, |acc, (arg, _)| {
            combine_optional_same(acc, extract_reduction_op_literal(fn_ir, *arg))
        }),
        ValueKind::Len { base } | ValueKind::Indices { base } => {
            extract_reduction_op_literal(fn_ir, *base)
        }
        ValueKind::Range { start, end } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *start),
            extract_reduction_op_literal(fn_ir, *end),
        ),
        ValueKind::Index1D { base, idx, .. } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *base),
            extract_reduction_op_literal(fn_ir, *idx),
        ),
        ValueKind::Index2D { base, r, c } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *base),
            combine_optional_same(
                extract_reduction_op_literal(fn_ir, *r),
                extract_reduction_op_literal(fn_ir, *c),
            ),
        ),
        ValueKind::Index3D { base, i, j, k } => combine_optional_same(
            extract_reduction_op_literal(fn_ir, *base),
            combine_optional_same(
                extract_reduction_op_literal(fn_ir, *i),
                combine_optional_same(
                    extract_reduction_op_literal(fn_ir, *j),
                    extract_reduction_op_literal(fn_ir, *k),
                ),
            ),
        ),
        ValueKind::Const(_)
        | ValueKind::Param { .. }
        | ValueKind::Load { .. }
        | ValueKind::RSymbol { .. } => None,
    }
}

pub fn lower_identity_schedule(
    fn_ir: &mut FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    schedule: &SchedulePlan,
) -> PolyCodegenPlan {
    if schedule.kind != SchedulePlanKind::Identity {
        return PolyCodegenPlan { emitted: false };
    }
    let generic_covers_map = generic_schedule_supports_map(fn_ir, scop, schedule);
    let generic_covers_reduce = generic_schedule_supports_reduce(fn_ir, scop, schedule);
    if lower_identity_map_generic(fn_ir, lp, scop, schedule) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-1d-identity-map",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if lower_identity_reduce_generic(fn_ir, lp, scop, schedule) {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply=true generic-1d-identity-reduce",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: true };
    }
    if generic_mir_effective_for_schedule(scop, schedule)
        && (generic_covers_map || generic_covers_reduce)
    {
        return PolyCodegenPlan { emitted: false };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) =
            build_multi_whole_vector_map_assignments(fn_ir, lp, scop)
    {
        let Some(cond) = build_vector_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-store-map-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) = build_multi_range_vector_map_assignments(fn_ir, scop)
    {
        let Some(cond) = build_vector_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-range-map-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) = build_single_whole_vector_map_assignment(fn_ir, lp, scop)
    {
        let Some(cond) = build_vector_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-1d-map-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) = build_single_range_vector_map_assignment(fn_ir, scop)
    {
        let Some(cond) = build_vector_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-1d-range-map-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) =
            build_multi_whole_vector_reduce_assignments(fn_ir, lp, scop)
    {
        let Some(cond) = build_vector_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-1d-reduce-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) =
            build_single_whole_vector_reduce_assignment(fn_ir, lp, scop)
    {
        let Some(cond) = build_vector_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-1d-reduce-versioned",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) = build_multi_2d_col_map_assignments(fn_ir, scop)
    {
        let Some(cond) = build_matrix_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-2d-col-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) = build_single_2d_col_map_assignment(fn_ir, scop)
    {
        let Some(cond) = build_matrix_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-2d-col-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) = build_multi_2d_col_reduce_assignments(fn_ir, lp, scop)
    {
        let Some(cond) = build_matrix_col_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-2d-col-reduce",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) =
            build_multi_nested_3d_full_cube_map_assignments(fn_ir, scop)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-nested-3d-cube-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) =
            build_single_nested_3d_full_cube_map_assignment(fn_ir, scop)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-nested-3d-cube-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) =
            build_multi_nested_3d_full_cube_reduce_assignments(fn_ir, lp, scop)
    {
        let Some(cond) = build_array3_cube_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-nested-3d-cube-reduce",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) =
            build_single_nested_3d_full_cube_reduce_assignment(fn_ir, lp, scop)
    {
        let Some(cond) = build_array3_cube_reduce_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-nested-3d-cube-reduce",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) = build_multi_3d_dim1_map_assignments(fn_ir, scop)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-3d-dim1-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignment, guard)) = build_single_3d_dim1_map_assignment(fn_ir, scop)
    {
        let Some(cond) = build_array3_map_guard_cond(fn_ir, &[guard]) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, vec![assignment], cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} single-3d-dim1-map",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    if let Some(site) = vector_apply_site(fn_ir, lp)
        && let Some((assignments, guards)) = build_multi_3d_dim1_reduce_assignments(fn_ir, lp, scop)
    {
        let Some(cond) = build_array3_dim1_reduce_guard_cond(fn_ir, &guards) else {
            return PolyCodegenPlan { emitted: false };
        };
        let emitted =
            finish_vector_assignments_versioned(fn_ir, lp.header, site, assignments, cond);
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} apply={} multi-3d-dim1-reduce",
                fn_ir.name, lp.header, emitted
            );
        }
        return PolyCodegenPlan { emitted };
    }
    let Some(plan) = build_identity_plan(fn_ir, lp, scop) else {
        if super::poly_trace_enabled() {
            eprintln!(
                "   [poly-codegen] {} loop header={} plan build failed",
                fn_ir.name, lp.header
            );
        }
        return PolyCodegenPlan { emitted: false };
    };
    let emitted = try_apply_vectorization_transactionally(fn_ir, lp, plan.clone());
    if super::poly_trace_enabled() {
        eprintln!(
            "   [poly-codegen] {} loop header={} apply={} plan={:?}",
            fn_ir.name, lp.header, emitted, plan
        );
    }
    PolyCodegenPlan { emitted }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mir::flow::Facts;
    use crate::mir::opt::loop_analysis::LoopAnalyzer;
    use crate::utils::Span;

    fn build_map_loop() -> (FnIR, LoopInfo, ScopRegion) {
        let mut fn_ir = FnIR::new(
            "poly_map".to_string(),
            vec!["x".to_string(), "y".to_string()],
        );
        let entry = fn_ir.add_block();
        let header = fn_ir.add_block();
        let body = fn_ir.add_block();
        let exit = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = entry;

        let x = fn_ir.add_value(
            ValueKind::Load {
                var: "x".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("x".to_string()),
        );
        let y = fn_ir.add_value(
            ValueKind::Load {
                var: "y".to_string(),
            },
            Span::default(),
            Facts::empty(),
            Some("y".to_string()),
        );
        let one = fn_ir.add_value(
            ValueKind::Const(Lit::Int(1)),
            Span::default(),
            Facts::empty(),
            None,
        );
        let len = fn_ir.add_value(
            ValueKind::Len { base: y },
            Span::default(),
            Facts::empty(),
            None,
        );
        let phi = fn_ir.add_value(
            ValueKind::Phi { args: Vec::new() },
            Span::default(),
            Facts::empty(),
            Some("i".to_string()),
        );
        fn_ir.values[phi].phi_block = Some(header);
        let cond = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: phi,
                rhs: len,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let read = fn_ir.add_value(
            ValueKind::Index1D {
                base: x,
                idx: phi,
                is_safe: true,
                is_na_safe: true,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        let rhs = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: read,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.blocks[body]
            .instrs
            .push(crate::mir::Instr::StoreIndex1D {
                base: y,
                idx: phi,
                val: rhs,
                is_safe: true,
                is_na_safe: true,
                is_vector: false,
                span: Span::default(),
            });
        let next = fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: phi,
                rhs: one,
            },
            Span::default(),
            Facts::empty(),
            None,
        );
        fn_ir.values[phi].kind = ValueKind::Phi {
            args: vec![(one, entry), (next, body)],
        };
        fn_ir.blocks[entry].term = crate::mir::Terminator::Goto(header);
        fn_ir.blocks[header].term = crate::mir::Terminator::If {
            cond,
            then_bb: body,
            else_bb: exit,
        };
        fn_ir.blocks[body].term = crate::mir::Terminator::Goto(header);
        fn_ir.blocks[exit].term = crate::mir::Terminator::Return(Some(y));

        let loops = LoopAnalyzer::new(&fn_ir).find_loops();
        let lp = loops[0].clone();
        let scop = crate::mir::opt::poly::scop::extract_scop_region(&fn_ir, &lp, &loops)
            .expect("expected scop");
        (fn_ir, lp, scop)
    }

    #[test]
    fn identity_schedule_builds_map_plan() {
        let (fn_ir, lp, scop) = build_map_loop();
        let plan = build_identity_plan(&fn_ir, &lp, &scop).expect("expected plan");
        assert!(matches!(plan, VectorPlan::Map { .. }));
    }
}
