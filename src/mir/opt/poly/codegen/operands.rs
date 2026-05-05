use super::*;
pub(crate) struct MatrixMapOperands {
    pub(crate) dest: ValueId,
    pub(crate) lhs_src: ValueId,
    pub(crate) rhs_src: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VectorMapOperands {
    pub(crate) dest: ValueId,
    pub(crate) lhs_src: ValueId,
    pub(crate) rhs_src: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct VectorReduceOperands {
    pub(crate) base: ValueId,
    pub(crate) start: ValueId,
    pub(crate) end: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MatrixRectReduceOperands {
    pub(crate) base: ValueId,
    pub(crate) r_start: ValueId,
    pub(crate) r_end: ValueId,
    pub(crate) c_start: ValueId,
    pub(crate) c_end: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MatrixColReduceOperands {
    pub(crate) base: ValueId,
    pub(crate) col: ValueId,
    pub(crate) start: ValueId,
    pub(crate) end: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Array3MapOperands {
    pub(crate) dest: ValueId,
    pub(crate) lhs_src: ValueId,
    pub(crate) rhs_src: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Array3Dim1ReduceOperands {
    pub(crate) base: ValueId,
    pub(crate) fixed_a: ValueId,
    pub(crate) fixed_b: ValueId,
    pub(crate) start: ValueId,
    pub(crate) end: ValueId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Array3CubeReduceOperands {
    pub(crate) base: ValueId,
    pub(crate) i_start: ValueId,
    pub(crate) i_end: ValueId,
    pub(crate) j_start: ValueId,
    pub(crate) j_end: ValueId,
    pub(crate) k_start: ValueId,
    pub(crate) k_end: ValueId,
}

pub(crate) fn is_loop_iv_subscript(scop: &ScopRegion, expr: &super::affine::AffineExpr) -> bool {
    if scop.dimensions.len() != 1 || expr.constant != 0 || expr.terms.len() != 1 {
        return false;
    }
    matches!(
        expr.terms.iter().next(),
        Some((super::affine::AffineSymbol::LoopIv(name), coeff))
            if name == &scop.dimensions[0].iv_name && *coeff == 1
    )
}

pub(crate) fn is_named_loop_iv_subscript(name: &str, expr: &super::affine::AffineExpr) -> bool {
    if expr.constant != 0 || expr.terms.len() != 1 {
        return false;
    }
    matches!(
        expr.terms.iter().next(),
        Some((super::affine::AffineSymbol::LoopIv(loop_name), coeff))
            if loop_name == name && *coeff == 1
    )
}

pub(crate) fn is_scalarish_value(fn_ir: &FnIR, value: ValueId) -> bool {
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

pub(crate) fn is_same_scalar_value(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
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

pub(crate) fn same_base_name(fn_ir: &FnIR, a: ValueId, b: ValueId) -> bool {
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

pub(crate) fn base_symbol_name(fn_ir: &FnIR, base: ValueId) -> String {
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

pub(crate) fn loop_covers_whole_vector(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    scop: &ScopRegion,
    base: ValueId,
) -> bool {
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

pub(crate) fn resolve_scop_local_source(fn_ir: &FnIR, scop: &ScopRegion, root: ValueId) -> ValueId {
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

pub(crate) fn index_reads_loop_vector(
    fn_ir: &FnIR,
    scop: &ScopRegion,
    value: ValueId,
) -> Option<ValueId> {
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

pub(crate) fn index_reads_2d_col_vector(
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

pub(crate) fn index_reads_3d_dim1_vector(
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

pub(crate) fn encode_bound(fn_ir: &mut FnIR, expr: &super::affine::AffineExpr) -> Option<ValueId> {
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
