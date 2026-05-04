use crate::mir::opt::loop_analysis::LoopInfo;
use crate::mir::{CallSemantics, FnIR, Instr, Lit, ValueId, ValueKind};
use rustc_hash::FxHashSet;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AffineSymbol {
    LoopIv(String),
    Param(String),
    Invariant(String),
    Length(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineExpr {
    pub constant: i64,
    pub terms: BTreeMap<AffineSymbol, i64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AffineConstraintKind {
    LowerBound,
    UpperBound,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AffineConstraint {
    pub expr: AffineExpr,
    pub kind: AffineConstraintKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PresburgerSet {
    pub dimensions: Vec<String>,
    pub constraints: Vec<AffineConstraint>,
}

impl AffineExpr {
    pub fn zero() -> Self {
        Self {
            constant: 0,
            terms: BTreeMap::new(),
        }
    }

    pub fn constant(value: i64) -> Self {
        Self {
            constant: value,
            terms: BTreeMap::new(),
        }
    }

    pub fn symbol(symbol: AffineSymbol) -> Self {
        let mut terms = BTreeMap::new();
        terms.insert(symbol, 1);
        Self { constant: 0, terms }
    }

    pub fn add_assign(&mut self, other: &Self, sign: i64) {
        self.constant += sign * other.constant;
        for (symbol, coeff) in &other.terms {
            let next = self.terms.get(symbol).copied().unwrap_or(0) + (sign * coeff);
            if next == 0 {
                self.terms.remove(symbol);
            } else {
                self.terms.insert(symbol.clone(), next);
            }
        }
    }

    pub fn scaled(&self, factor: i64) -> Self {
        let mut out = Self::constant(self.constant * factor);
        for (symbol, coeff) in &self.terms {
            out.terms.insert(symbol.clone(), coeff * factor);
        }
        out
    }
}

impl PresburgerSet {
    pub fn new(dimensions: Vec<String>, constraints: Vec<AffineConstraint>) -> Self {
        Self {
            dimensions,
            constraints,
        }
    }
}

fn normalize_builtin_name(callee: &str) -> &str {
    callee.strip_prefix("base::").unwrap_or(callee)
}

fn is_floor_like_single_positional_call(
    callee: &str,
    args: &[ValueId],
    names: &[Option<String>],
) -> bool {
    matches!(
        normalize_builtin_name(callee),
        "floor" | "ceiling" | "trunc"
    ) && args.len() == 1
        && names.len() <= 1
        && names
            .first()
            .and_then(std::option::Option::as_ref)
            .is_none()
}

fn is_affine_passthrough_call(callee: &str, args: &[ValueId], names: &[Option<String>]) -> bool {
    matches!(
        normalize_builtin_name(callee),
        "rr_index1_read" | "rr_index1_write"
    ) && !args.is_empty()
        && names
            .first()
            .and_then(std::option::Option::as_ref)
            .is_none()
}

fn var_is_written_in_loop(fn_ir: &FnIR, lp: &LoopInfo, var: &str) -> bool {
    lp.body.iter().any(|bid| {
        fn_ir.blocks[*bid]
            .instrs
            .iter()
            .any(|instr| matches!(instr, Instr::Assign { dst, .. } if dst == var))
    })
}

fn loop_iv_name(fn_ir: &FnIR, lp: &LoopInfo) -> Option<String> {
    lp.iv
        .as_ref()
        .and_then(|iv| fn_ir.values[iv.phi_val].origin_var.clone())
        .or_else(|| lp.iv.as_ref().map(|iv| format!(".poly_iv_{}", iv.phi_val)))
}

fn value_symbol_name(fn_ir: &FnIR, value: ValueId) -> String {
    if let Some(origin) = fn_ir.values[value].origin_var.clone() {
        return origin;
    }
    match &fn_ir.values[value].kind {
        ValueKind::Load { var } => var.clone(),
        ValueKind::Param { index } => fn_ir
            .params
            .get(*index)
            .cloned()
            .unwrap_or_else(|| format!(".arg_{index}")),
        _ => format!("v{value}"),
    }
}

fn canonical_param_name(fn_ir: &FnIR, name: &str) -> Option<String> {
    if fn_ir.params.iter().any(|param| param == name) {
        return Some(name.to_string());
    }
    let stripped = name.strip_prefix(".arg_")?;
    if fn_ir.params.iter().any(|param| param == stripped) {
        Some(stripped.to_string())
    } else {
        None
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

pub fn try_lift_affine_expr(fn_ir: &FnIR, root: ValueId, lp: &LoopInfo) -> Option<AffineExpr> {
    fn integral_const_value(fn_ir: &FnIR, value: ValueId) -> Option<i64> {
        match &fn_ir.values[value].kind {
            ValueKind::Const(Lit::Int(n)) => Some(*n),
            ValueKind::Const(Lit::Float(f)) if f.is_finite() && f.fract() == 0.0 => Some(*f as i64),
            _ => None,
        }
    }

    fn origin_symbol_expr(fn_ir: &FnIR, lp: &LoopInfo, origin: &str) -> Option<AffineExpr> {
        if loop_iv_name(fn_ir, lp).as_deref() == Some(origin) {
            Some(AffineExpr::symbol(AffineSymbol::LoopIv(origin.to_string())))
        } else if let Some(param) = canonical_param_name(fn_ir, origin) {
            Some(AffineExpr::symbol(AffineSymbol::Param(param)))
        } else if !var_is_written_in_loop(fn_ir, lp, origin) {
            Some(AffineExpr::symbol(AffineSymbol::Invariant(
                origin.to_string(),
            )))
        } else {
            None
        }
    }

    fn rec(
        fn_ir: &FnIR,
        root: ValueId,
        lp: &LoopInfo,
        seen: &mut FxHashSet<ValueId>,
        seen_vars: &mut FxHashSet<String>,
    ) -> Option<AffineExpr> {
        fn lift_loop_local_var(
            fn_ir: &FnIR,
            lp: &LoopInfo,
            var: &str,
            seen: &mut FxHashSet<ValueId>,
            seen_vars: &mut FxHashSet<String>,
        ) -> Option<AffineExpr> {
            if !seen_vars.insert(var.to_string()) {
                if super::poly_trace_enabled() {
                    eprintln!(
                        "   [poly-affine] {} header={} reject loop-local var={} reason=recursive",
                        fn_ir.name, lp.header, var
                    );
                }
                return None;
            }
            let mut lifted: Option<AffineExpr> = None;
            let mut accepted_sources = FxHashSet::default();
            for bid in &lp.body {
                for instr in &fn_ir.blocks[*bid].instrs {
                    let Instr::Assign { dst, src, .. } = instr else {
                        continue;
                    };
                    if dst != var {
                        continue;
                    }
                    if accepted_sources.contains(src) {
                        continue;
                    }
                    if super::poly_trace_enabled() {
                        eprintln!(
                            "   [poly-affine] {} header={} scan loop-local var={} bid={} src={} kind={:?}",
                            fn_ir.name, lp.header, var, bid, src, fn_ir.values[*src].kind
                        );
                    }
                    if matches!(&fn_ir.values[*src].kind, ValueKind::Phi { .. })
                        && fn_ir.values[*src].origin_var.as_deref() == Some(var)
                    {
                        continue;
                    }
                    let expr = rec(fn_ir, *src, lp, seen, seen_vars)?;
                    if super::poly_trace_enabled() {
                        eprintln!(
                            "   [poly-affine] {} header={} accept loop-local var={} expr={:?}",
                            fn_ir.name, lp.header, var, expr
                        );
                    }
                    accepted_sources.insert(*src);
                    match &lifted {
                        None => lifted = Some(expr),
                        Some(prev) if prev == &expr => {}
                        Some(_) => {
                            if super::poly_trace_enabled() {
                                eprintln!(
                                    "   [poly-affine] {} header={} reject loop-local var={} reason=conflicting-assigns",
                                    fn_ir.name, lp.header, var
                                );
                            }
                            seen_vars.remove(var);
                            return None;
                        }
                    }
                }
            }
            seen_vars.remove(var);
            if lifted.is_none() {
                if super::poly_trace_enabled() {
                    eprintln!(
                        "   [poly-affine] {} header={} fallback loop-local var={} -> invariant",
                        fn_ir.name, lp.header, var
                    );
                }
                return Some(AffineExpr::symbol(AffineSymbol::Invariant(var.to_string())));
            }
            lifted
        }

        let root = canonicalize_trivial_phi(fn_ir, root);
        if !seen.insert(root) {
            return None;
        }

        let out = if lp.iv.as_ref().is_some_and(|iv| iv.phi_val == root) {
            AffineExpr::symbol(AffineSymbol::LoopIv(loop_iv_name(fn_ir, lp)?))
        } else {
            match &fn_ir.values[root].kind {
                ValueKind::Const(Lit::Int(n)) => AffineExpr::constant(*n),
                ValueKind::Const(Lit::Float(f)) if f.is_finite() && f.fract() == 0.0 => {
                    AffineExpr::constant(*f as i64)
                }
                ValueKind::Param { index } => {
                    let name = fn_ir
                        .params
                        .get(*index)
                        .cloned()
                        .unwrap_or_else(|| format!(".arg_{index}"));
                    AffineExpr::symbol(AffineSymbol::Param(name))
                }
                ValueKind::Load { var } => {
                    if let Some(expr) = origin_symbol_expr(fn_ir, lp, var) {
                        expr
                    } else if let Some(expr) = lift_loop_local_var(fn_ir, lp, var, seen, seen_vars)
                    {
                        expr
                    } else {
                        if super::poly_trace_enabled() {
                            eprintln!(
                                "   [poly-affine] {} header={} reject load var={} origin={:?} kind={:?}",
                                fn_ir.name,
                                lp.header,
                                var,
                                fn_ir.values[root].origin_var,
                                fn_ir.values[root].kind
                            );
                        }
                        return None;
                    }
                }
                ValueKind::Phi { args } if args.is_empty() => {
                    let origin = fn_ir.values[root].origin_var.clone()?;
                    if let Some(expr) = origin_symbol_expr(fn_ir, lp, &origin) {
                        expr
                    } else {
                        lift_loop_local_var(fn_ir, lp, &origin, seen, seen_vars)?
                    }
                }
                ValueKind::Phi { args } => {
                    if let Some(origin) = fn_ir.values[root].origin_var.clone()
                        && let Some(param) = canonical_param_name(fn_ir, &origin)
                    {
                        return Some(AffineExpr::symbol(AffineSymbol::Param(param)));
                    }
                    if let Some(origin) = fn_ir.values[root].origin_var.clone()
                        && let Some(expr) = lift_loop_local_var(fn_ir, lp, &origin, seen, seen_vars)
                    {
                        return Some(expr);
                    }
                    let mut lifted: Option<AffineExpr> = None;
                    let mut saw_arg = false;
                    for (arg, _) in args {
                        if *arg == root {
                            continue;
                        }
                        let expr = rec(fn_ir, *arg, lp, seen, seen_vars)?;
                        saw_arg = true;
                        match &lifted {
                            None => lifted = Some(expr),
                            Some(prev) if prev == &expr => {}
                            Some(_) => return None,
                        }
                    }
                    if saw_arg {
                        lifted?
                    } else if let Some(origin) = fn_ir.values[root].origin_var.clone() {
                        origin_symbol_expr(fn_ir, lp, &origin)?
                    } else {
                        return None;
                    }
                }
                ValueKind::Len { base } => {
                    let base_name = value_symbol_name(fn_ir, *base);
                    AffineExpr::symbol(AffineSymbol::Length(base_name))
                }
                ValueKind::Unary {
                    op: crate::syntax::ast::UnaryOp::Neg,
                    rhs,
                } => rec(fn_ir, *rhs, lp, seen, seen_vars)?.scaled(-1),
                ValueKind::Binary { op, lhs, rhs } => match op {
                    crate::syntax::ast::BinOp::Add => {
                        let mut out = rec(fn_ir, *lhs, lp, seen, seen_vars)?;
                        let rhs = rec(fn_ir, *rhs, lp, seen, seen_vars)?;
                        out.add_assign(&rhs, 1);
                        out
                    }
                    crate::syntax::ast::BinOp::Sub => {
                        let mut out = rec(fn_ir, *lhs, lp, seen, seen_vars)?;
                        let rhs = rec(fn_ir, *rhs, lp, seen, seen_vars)?;
                        out.add_assign(&rhs, -1);
                        out
                    }
                    crate::syntax::ast::BinOp::Mul => {
                        if let Some(scale) = integral_const_value(fn_ir, *lhs) {
                            rec(fn_ir, *rhs, lp, seen, seen_vars)?.scaled(scale)
                        } else if let Some(scale) = integral_const_value(fn_ir, *rhs) {
                            rec(fn_ir, *lhs, lp, seen, seen_vars)?.scaled(scale)
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                },
                ValueKind::Call {
                    callee,
                    args,
                    names,
                } => {
                    if is_affine_passthrough_call(callee, args, names) {
                        rec(fn_ir, args[0], lp, seen, seen_vars)?
                    } else {
                        let floor_like =
                            matches!(
                                fn_ir.call_semantics(root),
                                Some(CallSemantics::Builtin(kind)) if kind.is_floor_like()
                            ) || is_floor_like_single_positional_call(callee, args, names);
                        if floor_like {
                            rec(fn_ir, args[0], lp, seen, seen_vars)?
                        } else {
                            return None;
                        }
                    }
                }
                _ => return None,
            }
        };

        seen.remove(&root);
        Some(out)
    }

    rec(
        fn_ir,
        root,
        lp,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}
