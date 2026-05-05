use super::affine::{AffineExpr, try_lift_affine_expr};
use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::{FnIR, MemoryLayoutHint, ValueId, ValueKind};
use rustc_hash::FxHashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryLayout {
    Dense1D,
    ColumnMajor2D,
    ColumnMajor3D,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MemRef {
    pub base: ValueId,
    pub name: String,
    pub rank: usize,
    pub layout: MemoryLayout,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessKind {
    Read,
    Write,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AccessRelation {
    pub statement_id: usize,
    pub kind: AccessKind,
    pub memref: MemRef,
    pub subscripts: Vec<AffineExpr>,
}

fn base_name(fn_ir: &FnIR, base: ValueId) -> String {
    if let Some(origin) = fn_ir.values[base].origin_var.as_deref()
        && !origin.is_empty()
    {
        return origin.to_string();
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

fn resolve_memory_layout_hint(
    fn_ir: &FnIR,
    root: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> Option<MemoryLayoutHint> {
    if !seen_vals.insert(root) {
        return None;
    }
    if let Some(layout) = fn_ir.memory_layout_hint(root) {
        seen_vals.remove(&root);
        return Some(layout);
    }
    let out = match &fn_ir.values[root].kind {
        ValueKind::Load { var } => {
            if !seen_vars.insert(var.clone()) {
                None
            } else {
                let mut layout: Option<MemoryLayoutHint> = None;
                for block in &fn_ir.blocks {
                    for instr in &block.instrs {
                        let crate::mir::Instr::Assign { dst, src, .. } = instr else {
                            continue;
                        };
                        if dst != var {
                            continue;
                        }
                        let next = resolve_memory_layout_hint(fn_ir, *src, seen_vals, seen_vars)?;
                        match layout {
                            None => layout = Some(next),
                            Some(prev) if prev == next => {}
                            Some(_) => return None,
                        }
                    }
                }
                seen_vars.remove(var);
                layout
            }
        }
        ValueKind::Phi { args } => {
            let mut layout: Option<MemoryLayoutHint> = None;
            for (arg, _) in args {
                let next = resolve_memory_layout_hint(fn_ir, *arg, seen_vals, seen_vars)?;
                match layout {
                    None => layout = Some(next),
                    Some(prev) if prev == next => {}
                    Some(_) => return None,
                }
            }
            layout
        }
        _ => None,
    };
    seen_vals.remove(&root);
    out
}

fn build_memref(fn_ir: &FnIR, base: ValueId, rank: usize) -> MemRef {
    let base = canonicalize_trivial_phi(fn_ir, base);
    let layout = match resolve_memory_layout_hint(
        fn_ir,
        base,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    ) {
        Some(MemoryLayoutHint::Dense1D) => MemoryLayout::Dense1D,
        Some(MemoryLayoutHint::ColumnMajor2D) => MemoryLayout::ColumnMajor2D,
        Some(MemoryLayoutHint::ColumnMajorND) => {
            if rank <= 2 {
                MemoryLayout::ColumnMajor2D
            } else {
                MemoryLayout::ColumnMajor3D
            }
        }
        None => match rank {
            1 => MemoryLayout::Dense1D,
            2 => MemoryLayout::ColumnMajor2D,
            _ => MemoryLayout::ColumnMajor3D,
        },
    };
    MemRef {
        base,
        name: base_name(fn_ir, base),
        rank,
        layout,
    }
}

fn canonicalize_trivial_phi(fn_ir: &FnIR, mut value: ValueId) -> ValueId {
    let mut seen = FxHashSet::default();
    loop {
        if !seen.insert(value) {
            return value;
        }
        let ValueKind::Phi { args } = &fn_ir.values[value].kind else {
            return value;
        };
        if args.is_empty() {
            if let Some(origin) = fn_ir.values[value].origin_var.as_deref()
                && let Some(candidate) = fn_ir.values.iter().find(|candidate| {
                    candidate.id != value
                        && matches!(
                            &candidate.kind,
                            ValueKind::Load { var } if var == origin
                        )
                })
            {
                value = candidate.id;
                continue;
            }
            return value;
        }
        let first = args[0].0;
        if args.iter().all(|(arg, _)| *arg == first) {
            value = first;
            continue;
        }
        let mut unique_non_self = args
            .iter()
            .map(|(arg, _)| *arg)
            .filter(|arg| *arg != value)
            .collect::<FxHashSet<_>>();
        if unique_non_self.len() == 1 {
            value = unique_non_self.drain().next().unwrap_or(value);
            continue;
        }
        return value;
    }
}

fn lift_subscript_with_trace(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    value: ValueId,
    context: &str,
) -> Option<AffineExpr> {
    let canonical = canonicalize_trivial_phi(fn_ir, value);
    let lifted = try_lift_affine_expr(fn_ir, canonical, lp);
    if lifted.is_none() && super::poly_trace_enabled() {
        eprintln!(
            "   [poly-access] {} header={} reject {} value={} canonical={} kind={:?}",
            fn_ir.name, lp.header, context, value, canonical, fn_ir.values[canonical].kind
        );
    }
    lifted
}

pub fn build_write_access(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    statement_id: usize,
    base: ValueId,
    subscripts: &[ValueId],
) -> Option<AccessRelation> {
    let lifted = subscripts
        .iter()
        .map(|subscript| lift_subscript_with_trace(fn_ir, lp, *subscript, "write-subscript"))
        .collect::<Option<Vec<_>>>()?;
    Some(AccessRelation {
        statement_id,
        kind: AccessKind::Write,
        memref: build_memref(fn_ir, base, subscripts.len()),
        subscripts: lifted,
    })
}

pub fn extract_read_accesses(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    statement_id: usize,
    root: ValueId,
) -> Option<Vec<AccessRelation>> {
    fn rec(
        fn_ir: &FnIR,
        lp: &LoopInfo,
        statement_id: usize,
        root: ValueId,
        out: &mut Vec<AccessRelation>,
        seen: &mut FxHashSet<ValueId>,
    ) -> Option<()> {
        if !seen.insert(root) {
            return Some(());
        }
        let root = canonicalize_trivial_phi(fn_ir, root);
        match &fn_ir.values[root].kind {
            ValueKind::Index1D { base, idx, .. } => {
                out.push(AccessRelation {
                    statement_id,
                    kind: AccessKind::Read,
                    memref: build_memref(fn_ir, canonicalize_trivial_phi(fn_ir, *base), 1),
                    subscripts: vec![lift_subscript_with_trace(
                        fn_ir,
                        lp,
                        *idx,
                        "read-1d-subscript",
                    )?],
                });
                rec(fn_ir, lp, statement_id, *idx, out, seen)?;
            }
            ValueKind::Index2D { base, r, c } => {
                out.push(AccessRelation {
                    statement_id,
                    kind: AccessKind::Read,
                    memref: build_memref(fn_ir, canonicalize_trivial_phi(fn_ir, *base), 2),
                    subscripts: vec![
                        lift_subscript_with_trace(fn_ir, lp, *r, "read-2d-r-subscript")?,
                        lift_subscript_with_trace(fn_ir, lp, *c, "read-2d-c-subscript")?,
                    ],
                });
                rec(fn_ir, lp, statement_id, *r, out, seen)?;
                rec(fn_ir, lp, statement_id, *c, out, seen)?;
            }
            ValueKind::Index3D { base, i, j, k } => {
                out.push(AccessRelation {
                    statement_id,
                    kind: AccessKind::Read,
                    memref: build_memref(fn_ir, canonicalize_trivial_phi(fn_ir, *base), 3),
                    subscripts: vec![
                        lift_subscript_with_trace(fn_ir, lp, *i, "read-3d-i-subscript")?,
                        lift_subscript_with_trace(fn_ir, lp, *j, "read-3d-j-subscript")?,
                        lift_subscript_with_trace(fn_ir, lp, *k, "read-3d-k-subscript")?,
                    ],
                });
                rec(fn_ir, lp, statement_id, *i, out, seen)?;
                rec(fn_ir, lp, statement_id, *j, out, seen)?;
                rec(fn_ir, lp, statement_id, *k, out, seen)?;
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    rec(fn_ir, lp, statement_id, *value, out, seen)?;
                }
            }
            ValueKind::FieldGet { base, .. } => {
                rec(fn_ir, lp, statement_id, *base, out, seen)?;
            }
            ValueKind::FieldSet { base, value, .. } => {
                rec(fn_ir, lp, statement_id, *base, out, seen)?;
                rec(fn_ir, lp, statement_id, *value, out, seen)?;
            }
            ValueKind::Binary { lhs, rhs, .. } => {
                rec(fn_ir, lp, statement_id, *lhs, out, seen)?;
                rec(fn_ir, lp, statement_id, *rhs, out, seen)?;
            }
            ValueKind::Unary { rhs, .. } => {
                rec(fn_ir, lp, statement_id, *rhs, out, seen)?;
            }
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    rec(fn_ir, lp, statement_id, *arg, out, seen)?;
                }
            }
            ValueKind::Phi { args } => {
                for (arg, _) in args {
                    rec(fn_ir, lp, statement_id, *arg, out, seen)?;
                }
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => {
                rec(fn_ir, lp, statement_id, *base, out, seen)?;
            }
            ValueKind::Range { start, end } => {
                rec(fn_ir, lp, statement_id, *start, out, seen)?;
                rec(fn_ir, lp, statement_id, *end, out, seen)?;
            }
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => {}
        }
        Some(())
    }

    let mut out = Vec::new();
    rec(
        fn_ir,
        lp,
        statement_id,
        root,
        &mut out,
        &mut FxHashSet::default(),
    )?;
    Some(out)
}
