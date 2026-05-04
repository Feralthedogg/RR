use super::super::access::{AccessRelation, build_write_access, extract_read_accesses};
use super::super::affine::{
    AffineConstraint, AffineConstraintKind, AffineExpr, AffineSymbol, PresburgerSet,
    try_lift_affine_expr,
};
use super::*;
use crate::mir::analyze::effects;
use crate::mir::opt::loop_analysis::{LoopInfo, build_pred_map};
use crate::mir::{BlockId, CallSemantics, FnIR, Instr, Terminator, ValueId, ValueKind};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScopExtractionFailure {
    UnsupportedCfgShape,
    MissingInductionVar,
    NonAffineLoopBound,
    NonAffineAccess,
    EffectfulStatement,
    UnsupportedNestedLoop,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopDimension {
    pub iv_name: String,
    pub lower_bound: AffineExpr,
    pub upper_bound: AffineExpr,
    pub step: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PolyStmtKind {
    Assign {
        dst: String,
    },
    Eval,
    Store {
        base: ValueId,
        subscripts: Vec<ValueId>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolyStmt {
    pub id: usize,
    pub block: BlockId,
    pub kind: PolyStmtKind,
    pub expr_root: Option<ValueId>,
    pub accesses: Vec<AccessRelation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScopRegion {
    pub header: BlockId,
    pub latch: BlockId,
    pub exits: Vec<BlockId>,
    pub dimensions: Vec<LoopDimension>,
    pub iteration_space: PresburgerSet,
    pub parameters: BTreeSet<String>,
    pub statements: Vec<PolyStmt>,
}

pub(crate) fn signed_step(lp: &LoopInfo) -> Option<i64> {
    let iv = lp.iv.as_ref()?;
    let magnitude = iv.step;
    Some(if iv.step_op == crate::syntax::ast::BinOp::Sub {
        -magnitude
    } else {
        magnitude
    })
}

pub(crate) fn loop_iv_name(fn_ir: &FnIR, lp: &LoopInfo) -> Option<String> {
    let name = lp
        .iv
        .as_ref()
        .and_then(|iv| fn_ir.values[iv.phi_val].origin_var.clone())
        .or_else(|| lp.iv.as_ref().map(|iv| format!(".poly_iv_{}", iv.phi_val)));
    name.filter(|name| !super::codegen_generic::is_generated_loop_iv_name(name))
}

pub(crate) fn collect_affine_symbols(expr: &AffineExpr, params: &mut BTreeSet<String>) {
    for symbol in expr.terms.keys() {
        match symbol {
            AffineSymbol::LoopIv(_) => {}
            AffineSymbol::Param(name)
            | AffineSymbol::Invariant(name)
            | AffineSymbol::Length(name) => {
                params.insert(name.clone());
            }
        }
    }
}

pub(crate) fn normalize_callee(callee: &str) -> &str {
    callee.strip_prefix("base::").unwrap_or(callee)
}

pub(crate) fn affine_mentions_loop_iv(expr: &AffineExpr, iv_name: &str) -> bool {
    expr.terms.iter().any(|(symbol, coeff)| {
        matches!(symbol, AffineSymbol::LoopIv(name) if name == iv_name) && *coeff != 0
    })
}

pub(crate) fn extract_loop_upper_bound(fn_ir: &FnIR, lp: &LoopInfo) -> Option<ValueId> {
    let Terminator::If { cond, .. } = fn_ir.blocks[lp.header].term else {
        return lp.limit;
    };
    let ValueKind::Binary { op, lhs, rhs } = fn_ir.values[cond].kind else {
        return lp.limit;
    };
    if !matches!(
        op,
        crate::syntax::ast::BinOp::Le
            | crate::syntax::ast::BinOp::Lt
            | crate::syntax::ast::BinOp::Ge
            | crate::syntax::ast::BinOp::Gt
    ) {
        return lp.limit;
    }
    let iv_side = |value: ValueId| {
        let Some(iv) = lp.iv.as_ref() else {
            return false;
        };
        if value == iv.phi_val {
            return true;
        }
        let iv_name = loop_iv_name(fn_ir, lp);
        if fn_ir.values[value].origin_var == iv_name {
            return true;
        }
        match &fn_ir.values[value].kind {
            ValueKind::Load { var } => iv_name.as_deref() == Some(var.as_str()),
            ValueKind::Phi { args } => args.iter().any(|(arg, _)| *arg == iv.phi_val),
            _ => false,
        }
    };
    if iv_side(lhs) != iv_side(rhs) {
        return Some(if iv_side(lhs) { rhs } else { lhs });
    }
    if let Some(iv_name) = loop_iv_name(fn_ir, lp) {
        let lhs_affine = try_lift_affine_expr(fn_ir, lhs, lp);
        let rhs_affine = try_lift_affine_expr(fn_ir, rhs, lp);
        let lhs_is_iv = lhs_affine
            .as_ref()
            .is_some_and(|expr| affine_mentions_loop_iv(expr, &iv_name));
        let rhs_is_iv = rhs_affine
            .as_ref()
            .is_some_and(|expr| affine_mentions_loop_iv(expr, &iv_name));
        if lhs_is_iv != rhs_is_iv {
            return Some(if lhs_is_iv { rhs } else { lhs });
        }
    }
    let mut written_vars = std::collections::BTreeSet::new();
    for bid in &lp.body {
        for instr in &fn_ir.blocks[*bid].instrs {
            if let Instr::Assign { dst, .. } = instr {
                written_vars.insert(dst.clone());
            }
        }
    }
    let side_is_iv = |value: ValueId| {
        matches!(&fn_ir.values[value].kind, ValueKind::Load { var } if written_vars.contains(var))
            || fn_ir.values[value]
                .origin_var
                .as_ref()
                .is_some_and(|var| written_vars.contains(var))
    };
    if side_is_iv(lhs) {
        Some(rhs)
    } else if side_is_iv(rhs) {
        Some(lhs)
    } else {
        lp.limit
    }
}

pub(crate) fn choose_affine_loop_limit(fn_ir: &FnIR, lp: &LoopInfo) -> Option<ValueId> {
    if let Some(limit) = lp.limit
        && try_lift_affine_expr(fn_ir, limit, lp).is_some()
    {
        return Some(limit);
    }
    extract_loop_upper_bound(fn_ir, lp)
        .filter(|limit| try_lift_affine_expr(fn_ir, *limit, lp).is_some())
}

pub(crate) fn remap_nested_iv_symbols(expr: &mut AffineExpr, loop_names: &[&str]) {
    let terms = expr.terms.clone();
    expr.terms.clear();
    for (symbol, coeff) in terms {
        let symbol = match symbol {
            AffineSymbol::Invariant(name)
                if loop_names.iter().any(|candidate| *candidate == name) =>
            {
                AffineSymbol::LoopIv(name)
            }
            other => other,
        };
        expr.terms.insert(symbol, coeff);
    }
}

pub(crate) fn normalize_nested_accesses(statements: &mut [PolyStmt], loop_names: &[String]) {
    let refs = loop_names.iter().map(String::as_str).collect::<Vec<_>>();
    for stmt in statements {
        for access in &mut stmt.accesses {
            for expr in &mut access.subscripts {
                remap_nested_iv_symbols(expr, &refs);
            }
        }
    }
}

pub(crate) fn direct_nested_loops<'a>(
    lp: &LoopInfo,
    all_loops: &'a [LoopInfo],
) -> Vec<&'a LoopInfo> {
    let nested = all_loops
        .iter()
        .filter(|other| {
            other.header != lp.header
                && other.body.len() < lp.body.len()
                && lp.body.contains(&other.header)
        })
        .collect::<Vec<_>>();
    nested
        .iter()
        .copied()
        .filter(|candidate| {
            !nested.iter().any(|other| {
                other.header != candidate.header
                    && other.body.len() > candidate.body.len()
                    && other.body.contains(&candidate.header)
            })
        })
        .collect()
}

pub(crate) fn expr_is_scop_safe(fn_ir: &FnIR, _lp: &LoopInfo, root: ValueId) -> bool {
    fn rec(fn_ir: &FnIR, root: ValueId, seen: &mut rustc_hash::FxHashSet<ValueId>) -> bool {
        if !seen.insert(root) {
            return true;
        }
        match &fn_ir.values[root].kind {
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => true,
            ValueKind::Binary { lhs, rhs, .. } => rec(fn_ir, *lhs, seen) && rec(fn_ir, *rhs, seen),
            ValueKind::Unary { rhs, .. } => rec(fn_ir, *rhs, seen),
            ValueKind::Call { callee, args, .. } => {
                callee != "rr_call_closure"
                    && (matches!(fn_ir.call_semantics(root), Some(CallSemantics::Builtin(_)))
                        || effects::call_is_pure(callee)
                        || effects::call_is_pure(normalize_callee(callee)))
                    && args.iter().all(|arg| rec(fn_ir, *arg, seen))
            }
            ValueKind::RecordLit { .. }
            | ValueKind::FieldGet { .. }
            | ValueKind::FieldSet { .. } => false,
            ValueKind::Intrinsic { args, .. } => args.iter().all(|arg| rec(fn_ir, *arg, seen)),
            ValueKind::Phi { args } => args.iter().all(|(arg, _)| rec(fn_ir, *arg, seen)),
            ValueKind::Len { base } | ValueKind::Indices { base } => rec(fn_ir, *base, seen),
            ValueKind::Range { start, end } => rec(fn_ir, *start, seen) && rec(fn_ir, *end, seen),
            ValueKind::Index1D { idx, .. } => rec(fn_ir, *idx, seen),
            ValueKind::Index2D { r, c, .. } => rec(fn_ir, *r, seen) && rec(fn_ir, *c, seen),
            ValueKind::Index3D { i, j, k, .. } => {
                rec(fn_ir, *i, seen) && rec(fn_ir, *j, seen) && rec(fn_ir, *k, seen)
            }
        }
    }

    rec(fn_ir, root, &mut rustc_hash::FxHashSet::default())
}

pub(crate) fn extract_stmt(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    statement_id: usize,
    block: BlockId,
    instr: &Instr,
) -> Result<PolyStmt, ScopExtractionFailure> {
    match instr {
        Instr::Assign { dst, src, .. } => {
            if !expr_is_scop_safe(fn_ir, lp, *src) {
                return Err(ScopExtractionFailure::EffectfulStatement);
            }
            let accesses = extract_read_accesses(fn_ir, lp, statement_id, *src).unwrap_or_else(|| {
                if super::poly_trace_enabled() {
                    eprintln!(
                        "   [poly-scop] {} block={} stmt={} accessless/non-affine assign src={:?}",
                        fn_ir.name, block, statement_id, fn_ir.values[*src].kind
                    );
                }
                Vec::new()
            });
            Ok(PolyStmt {
                id: statement_id,
                block,
                kind: PolyStmtKind::Assign { dst: dst.clone() },
                expr_root: Some(*src),
                accesses,
            })
        }
        Instr::Eval { val, .. } => {
            if !expr_is_scop_safe(fn_ir, lp, *val) {
                return Err(ScopExtractionFailure::EffectfulStatement);
            }
            let accesses = extract_read_accesses(fn_ir, lp, statement_id, *val).unwrap_or_else(|| {
                if super::poly_trace_enabled() {
                    eprintln!(
                        "   [poly-scop] {} block={} stmt={} accessless/non-affine eval val={:?}",
                        fn_ir.name, block, statement_id, fn_ir.values[*val].kind
                    );
                }
                Vec::new()
            });
            Ok(PolyStmt {
                id: statement_id,
                block,
                kind: PolyStmtKind::Eval,
                expr_root: Some(*val),
                accesses,
            })
        }
        Instr::StoreIndex1D {
            base,
            idx,
            val,
            is_vector,
            ..
        } => {
            if *is_vector || !expr_is_scop_safe(fn_ir, lp, *val) {
                return Err(ScopExtractionFailure::EffectfulStatement);
            }
            let mut accesses = extract_read_accesses(fn_ir, lp, statement_id, *val)
                .ok_or_else(|| {
                    if super::poly_trace_enabled() {
                        eprintln!(
                            "   [poly-scop] {} block={} stmt={} reject 1d store read accesses val={:?}",
                            fn_ir.name, block, statement_id, fn_ir.values[*val].kind
                        );
                    }
                    ScopExtractionFailure::NonAffineAccess
                })?;
            accesses.push(
                build_write_access(fn_ir, lp, statement_id, *base, &[*idx]).ok_or_else(|| {
                    if super::poly_trace_enabled() {
                        eprintln!(
                            "   [poly-scop] {} block={} stmt={} reject 1d store subscript idx={:?}",
                            fn_ir.name, block, statement_id, fn_ir.values[*idx].kind
                        );
                    }
                    ScopExtractionFailure::NonAffineAccess
                })?,
            );
            Ok(PolyStmt {
                id: statement_id,
                block,
                kind: PolyStmtKind::Store {
                    base: *base,
                    subscripts: vec![*idx],
                },
                expr_root: Some(*val),
                accesses,
            })
        }
        Instr::StoreIndex2D {
            base, r, c, val, ..
        } => {
            if !expr_is_scop_safe(fn_ir, lp, *val) {
                return Err(ScopExtractionFailure::EffectfulStatement);
            }
            let mut accesses = extract_read_accesses(fn_ir, lp, statement_id, *val)
                .ok_or_else(|| {
                    if super::poly_trace_enabled() {
                        eprintln!(
                            "   [poly-scop] {} block={} stmt={} reject 2d store read accesses val={:?}",
                            fn_ir.name, block, statement_id, fn_ir.values[*val].kind
                        );
                    }
                    ScopExtractionFailure::NonAffineAccess
                })?;
            accesses.push(
                build_write_access(fn_ir, lp, statement_id, *base, &[*r, *c])
                    .ok_or_else(|| {
                        if super::poly_trace_enabled() {
                            eprintln!(
                                "   [poly-scop] {} block={} stmt={} reject 2d store subscripts r={:?} c={:?}",
                                fn_ir.name,
                                block,
                                statement_id,
                                fn_ir.values[*r].kind,
                                fn_ir.values[*c].kind
                            );
                        }
                        ScopExtractionFailure::NonAffineAccess
                    })?,
            );
            Ok(PolyStmt {
                id: statement_id,
                block,
                kind: PolyStmtKind::Store {
                    base: *base,
                    subscripts: vec![*r, *c],
                },
                expr_root: Some(*val),
                accesses,
            })
        }
        Instr::StoreIndex3D {
            base, i, j, k, val, ..
        } => {
            if !expr_is_scop_safe(fn_ir, lp, *val) {
                return Err(ScopExtractionFailure::EffectfulStatement);
            }
            let mut accesses = extract_read_accesses(fn_ir, lp, statement_id, *val)
                .ok_or_else(|| {
                    if super::poly_trace_enabled() {
                        eprintln!(
                            "   [poly-scop] {} block={} stmt={} reject 3d store read accesses val={:?}",
                            fn_ir.name, block, statement_id, fn_ir.values[*val].kind
                        );
                    }
                    ScopExtractionFailure::NonAffineAccess
                })?;
            accesses.push(
                build_write_access(fn_ir, lp, statement_id, *base, &[*i, *j, *k])
                    .ok_or_else(|| {
                        if super::poly_trace_enabled() {
                            eprintln!(
                                "   [poly-scop] {} block={} stmt={} reject 3d store subscripts i={:?} j={:?} k={:?}",
                                fn_ir.name,
                                block,
                                statement_id,
                                fn_ir.values[*i].kind,
                                fn_ir.values[*j].kind,
                                fn_ir.values[*k].kind
                            );
                        }
                        ScopExtractionFailure::NonAffineAccess
                    })?,
            );
            Ok(PolyStmt {
                id: statement_id,
                block,
                kind: PolyStmtKind::Store {
                    base: *base,
                    subscripts: vec![*i, *j, *k],
                },
                expr_root: Some(*val),
                accesses,
            })
        }
        Instr::UnsafeRBlock { .. } => Err(ScopExtractionFailure::EffectfulStatement),
    }
}

pub(crate) fn passthrough_loop_successor(
    fn_ir: &FnIR,
    bid: BlockId,
    loop_body: &rustc_hash::FxHashSet<BlockId>,
) -> Option<BlockId> {
    let block = fn_ir.blocks.get(bid)?;
    if !block.instrs.is_empty() {
        return None;
    }
    match block.term {
        Terminator::Goto(next) if loop_body.contains(&next) => Some(next),
        _ => None,
    }
}

pub(crate) fn branch_is_affine_passthrough(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    bid: BlockId,
    loop_body: &rustc_hash::FxHashSet<BlockId>,
) -> Option<(BlockId, Vec<(String, AffineExpr)>)> {
    let block = fn_ir.blocks.get(bid)?;
    let assigns = block_affine_bookkeeping_assigns(fn_ir, lp, &block.instrs)?;
    let succ = match block.term {
        Terminator::Goto(next) if loop_body.contains(&next) => next,
        _ => return None,
    };
    Some((succ, assigns))
}

pub(crate) fn block_affine_bookkeeping_assigns(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    instrs: &[Instr],
) -> Option<Vec<(String, AffineExpr)>> {
    let mut assigns = Vec::new();
    for instr in instrs {
        let Instr::Assign { dst, src, .. } = instr else {
            return None;
        };
        if matches!(&fn_ir.values[*src].kind, ValueKind::Phi { .. })
            && fn_ir.values[*src].origin_var.as_deref() == Some(dst.as_str())
        {
            continue;
        }
        if matches!(&fn_ir.values[*src].kind, ValueKind::Phi { .. }) {
            continue;
        }
        let Some(expr) = try_lift_affine_expr(fn_ir, *src, lp) else {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} header={} non-affine bookkeeping dst={} src={} kind={:?}",
                    fn_ir.name, lp.header, dst, src, fn_ir.values[*src].kind
                );
            }
            return None;
        };
        assigns.push((dst.clone(), expr));
    }
    Some(assigns)
}

pub(crate) fn is_ignorable_loop_if_block(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    bid: BlockId,
    loop_body: &rustc_hash::FxHashSet<BlockId>,
) -> bool {
    let Some(block) = fn_ir.blocks.get(bid) else {
        return false;
    };
    if !block.instrs.is_empty()
        && block_affine_bookkeeping_assigns(fn_ir, lp, &block.instrs).is_none()
    {
        return false;
    }
    let Terminator::If {
        then_bb, else_bb, ..
    } = block.term
    else {
        return false;
    };
    if then_bb == else_bb {
        return loop_body.contains(&then_bb)
            || passthrough_loop_successor(fn_ir, then_bb, loop_body).is_some()
            || branch_is_affine_passthrough(fn_ir, lp, then_bb, loop_body).is_some();
    }
    let passthrough = (
        passthrough_loop_successor(fn_ir, then_bb, loop_body),
        passthrough_loop_successor(fn_ir, else_bb, loop_body),
    );
    match passthrough {
        (Some(lhs), Some(rhs)) => lhs == rhs,
        _ => {
            let affine = (
                branch_is_affine_passthrough(fn_ir, lp, then_bb, loop_body),
                branch_is_affine_passthrough(fn_ir, lp, else_bb, loop_body),
            );
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} header={} if-block={} then={} else={} passthrough={:?} affine={:?}",
                    fn_ir.name, lp.header, bid, then_bb, else_bb, passthrough, affine
                );
            }
            match affine {
                (Some((lhs_succ, lhs_assigns)), Some((rhs_succ, rhs_assigns))) => {
                    lhs_succ == rhs_succ && lhs_assigns == rhs_assigns
                }
                _ => false,
            }
        }
    }
}

pub fn extract_scop_region(
    fn_ir: &FnIR,
    lp: &LoopInfo,
    all_loops: &[LoopInfo],
) -> Result<ScopRegion, ScopExtractionFailure> {
    if lp.exits.len() != 1 {
        return Err(ScopExtractionFailure::UnsupportedCfgShape);
    }
    let nested_loops = direct_nested_loops(lp, all_loops);
    if !nested_loops.is_empty() {
        if nested_loops.len() == 1 {
            let middle = nested_loops[0];
            let middle_nested = direct_nested_loops(middle, all_loops);
            if middle_nested.is_empty() {
                return extract_nested_scop_region(fn_ir, lp, middle, all_loops);
            }
            if middle_nested.len() == 1
                && direct_nested_loops(middle_nested[0], all_loops).is_empty()
            {
                return extract_triply_nested_scop_region(
                    fn_ir,
                    lp,
                    middle,
                    middle_nested[0],
                    all_loops,
                );
            }
        }
        return Err(ScopExtractionFailure::UnsupportedNestedLoop);
    }

    let iv = lp
        .iv
        .as_ref()
        .ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let step = signed_step(lp).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    if step == 0 {
        return Err(ScopExtractionFailure::MissingInductionVar);
    }

    let lower_bound = match try_lift_affine_expr(fn_ir, iv.init_val, lp) {
        Some(expr) => expr,
        None => {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} header={} reject lower bound: {:?}",
                    fn_ir.name, lp.header, fn_ir.values[iv.init_val].kind
                );
            }
            return Err(ScopExtractionFailure::NonAffineLoopBound);
        }
    };
    let limit =
        choose_affine_loop_limit(fn_ir, lp).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let mut upper_bound = match try_lift_affine_expr(fn_ir, limit, lp) {
        Some(expr) => expr,
        None => {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} header={} reject upper bound: {:?}",
                    fn_ir.name, lp.header, fn_ir.values[limit].kind
                );
            }
            return Err(ScopExtractionFailure::NonAffineLoopBound);
        }
    };
    upper_bound.constant += lp.limit_adjust;

    let mut statements = Vec::new();
    let mut body_blocks: Vec<BlockId> = lp.body.iter().copied().collect();
    body_blocks.sort_unstable();
    let preds = build_pred_map(fn_ir);
    for bid in body_blocks {
        if bid != lp.header
            && matches!(fn_ir.blocks[bid].term, Terminator::If { .. })
            && !is_ignorable_loop_if_block(fn_ir, lp, bid, &lp.body)
        {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} header={} reject cfg block={} term={:?} instrs={:#?}",
                    fn_ir.name, lp.header, bid, fn_ir.blocks[bid].term, fn_ir.blocks[bid].instrs
                );
            }
            return Err(ScopExtractionFailure::UnsupportedCfgShape);
        }
        if bid == lp.header && preds.get(&bid).is_some_and(|incoming| incoming.len() > 2) {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} header={} reject preds={:?}",
                    fn_ir.name,
                    lp.header,
                    preds.get(&bid)
                );
            }
            return Err(ScopExtractionFailure::UnsupportedCfgShape);
        }
        for instr in &fn_ir.blocks[bid].instrs {
            let stmt = extract_stmt(fn_ir, lp, statements.len(), bid, instr)?;
            statements.push(stmt);
        }
    }

    let mut parameters = BTreeSet::new();
    collect_affine_symbols(&lower_bound, &mut parameters);
    collect_affine_symbols(&upper_bound, &mut parameters);
    for stmt in &statements {
        for access in &stmt.accesses {
            for expr in &access.subscripts {
                collect_affine_symbols(expr, &mut parameters);
            }
        }
    }

    let iv_name = loop_iv_name(fn_ir, lp).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let constraints = vec![
        AffineConstraint {
            expr: lower_bound.clone(),
            kind: AffineConstraintKind::LowerBound,
        },
        AffineConstraint {
            expr: upper_bound.clone(),
            kind: AffineConstraintKind::UpperBound,
        },
    ];

    Ok(ScopRegion {
        header: lp.header,
        latch: lp.latch,
        exits: lp.exits.clone(),
        dimensions: vec![LoopDimension {
            iv_name: iv_name.clone(),
            lower_bound: lower_bound.clone(),
            upper_bound: upper_bound.clone(),
            step,
        }],
        iteration_space: PresburgerSet::new(vec![iv_name], constraints),
        parameters,
        statements,
    })
}

pub(crate) fn extract_nested_scop_region(
    fn_ir: &FnIR,
    outer: &LoopInfo,
    inner: &LoopInfo,
    all_loops: &[LoopInfo],
) -> Result<ScopRegion, ScopExtractionFailure> {
    if !direct_nested_loops(inner, all_loops).is_empty() {
        return Err(ScopExtractionFailure::UnsupportedNestedLoop);
    }
    let outer_iv = outer
        .iv
        .as_ref()
        .ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let inner_iv = inner
        .iv
        .as_ref()
        .ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let outer_step = signed_step(outer).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let inner_step = signed_step(inner).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    if outer_step == 0 || inner_step == 0 {
        return Err(ScopExtractionFailure::MissingInductionVar);
    }

    for bid in outer
        .body
        .iter()
        .copied()
        .filter(|bid| !inner.body.contains(bid))
    {
        let block = &fn_ir.blocks[bid];
        for instr in &block.instrs {
            match instr {
                Instr::Assign { src, .. } => {
                    if try_lift_affine_expr(fn_ir, *src, outer).is_none()
                        && try_lift_affine_expr(fn_ir, *src, inner).is_none()
                    {
                        return Err(ScopExtractionFailure::UnsupportedNestedLoop);
                    }
                }
                Instr::Eval { .. }
                | Instr::StoreIndex1D { .. }
                | Instr::StoreIndex2D { .. }
                | Instr::StoreIndex3D { .. }
                | Instr::UnsafeRBlock { .. } => {
                    return Err(ScopExtractionFailure::UnsupportedNestedLoop);
                }
            }
        }
    }

    let outer_lower = match try_lift_affine_expr(fn_ir, outer_iv.init_val, outer) {
        Some(expr) => expr,
        None => {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} outer header={} reject lower bound: {:?}",
                    fn_ir.name, outer.header, fn_ir.values[outer_iv.init_val].kind
                );
            }
            return Err(ScopExtractionFailure::NonAffineLoopBound);
        }
    };
    let outer_limit =
        choose_affine_loop_limit(fn_ir, outer).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let mut outer_upper = match try_lift_affine_expr(fn_ir, outer_limit, outer) {
        Some(expr) => expr,
        None => {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} outer header={} reject upper bound: {:?}",
                    fn_ir.name, outer.header, fn_ir.values[outer_limit].kind
                );
            }
            return Err(ScopExtractionFailure::NonAffineLoopBound);
        }
    };
    outer_upper.constant += outer.limit_adjust;

    let inner_lower = match try_lift_affine_expr(fn_ir, inner_iv.init_val, inner) {
        Some(expr) => expr,
        None => {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} inner header={} reject lower bound: {:?}",
                    fn_ir.name, inner.header, fn_ir.values[inner_iv.init_val].kind
                );
            }
            return Err(ScopExtractionFailure::NonAffineLoopBound);
        }
    };
    let inner_limit =
        choose_affine_loop_limit(fn_ir, inner).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let mut inner_upper = match try_lift_affine_expr(fn_ir, inner_limit, inner) {
        Some(expr) => expr,
        None => {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} inner header={} reject upper bound: {:?}",
                    fn_ir.name, inner.header, fn_ir.values[inner_limit].kind
                );
            }
            return Err(ScopExtractionFailure::NonAffineLoopBound);
        }
    };
    inner_upper.constant += inner.limit_adjust;

    let mut statements = Vec::new();
    let mut body_blocks: Vec<BlockId> = inner.body.iter().copied().collect();
    body_blocks.sort_unstable();
    let preds = build_pred_map(fn_ir);
    for bid in body_blocks {
        if bid != inner.header
            && matches!(fn_ir.blocks[bid].term, Terminator::If { .. })
            && !is_ignorable_loop_if_block(fn_ir, inner, bid, &inner.body)
        {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} nested header={} reject cfg block={} term={:?} instrs={:#?}",
                    fn_ir.name, inner.header, bid, fn_ir.blocks[bid].term, fn_ir.blocks[bid].instrs
                );
            }
            return Err(ScopExtractionFailure::UnsupportedCfgShape);
        }
        if bid == inner.header && preds.get(&bid).is_some_and(|incoming| incoming.len() > 2) {
            if super::poly_trace_enabled() {
                eprintln!(
                    "   [poly-scop] {} nested header={} reject preds={:?}",
                    fn_ir.name,
                    inner.header,
                    preds.get(&bid)
                );
            }
            return Err(ScopExtractionFailure::UnsupportedCfgShape);
        }
        for instr in &fn_ir.blocks[bid].instrs {
            let stmt = extract_stmt(fn_ir, inner, statements.len(), bid, instr)?;
            statements.push(stmt);
        }
    }

    let outer_name =
        loop_iv_name(fn_ir, outer).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    let inner_name =
        loop_iv_name(fn_ir, inner).ok_or(ScopExtractionFailure::MissingInductionVar)?;
    normalize_nested_accesses(&mut statements, &[outer_name.clone(), inner_name.clone()]);

    let mut parameters = BTreeSet::new();
    for expr in [&outer_lower, &outer_upper, &inner_lower, &inner_upper] {
        collect_affine_symbols(expr, &mut parameters);
    }
    for stmt in &statements {
        for access in &stmt.accesses {
            for expr in &access.subscripts {
                collect_affine_symbols(expr, &mut parameters);
            }
        }
    }

    let constraints = vec![
        AffineConstraint {
            expr: outer_lower.clone(),
            kind: AffineConstraintKind::LowerBound,
        },
        AffineConstraint {
            expr: outer_upper.clone(),
            kind: AffineConstraintKind::UpperBound,
        },
        AffineConstraint {
            expr: inner_lower.clone(),
            kind: AffineConstraintKind::LowerBound,
        },
        AffineConstraint {
            expr: inner_upper.clone(),
            kind: AffineConstraintKind::UpperBound,
        },
    ];

    Ok(ScopRegion {
        header: outer.header,
        latch: outer.latch,
        exits: outer.exits.clone(),
        dimensions: vec![
            LoopDimension {
                iv_name: outer_name.clone(),
                lower_bound: outer_lower.clone(),
                upper_bound: outer_upper.clone(),
                step: outer_step,
            },
            LoopDimension {
                iv_name: inner_name.clone(),
                lower_bound: inner_lower.clone(),
                upper_bound: inner_upper.clone(),
                step: inner_step,
            },
        ],
        iteration_space: PresburgerSet::new(vec![outer_name, inner_name], constraints),
        parameters,
        statements,
    })
}
