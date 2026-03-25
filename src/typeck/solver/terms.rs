use super::*;

fn const_integral_value(fn_ir: &FnIR, vid: ValueId) -> Option<i64> {
    match &fn_ir.values[vid].kind {
        ValueKind::Const(crate::syntax::ast::Lit::Int(i)) => Some(*i),
        ValueKind::Const(crate::syntax::ast::Lit::Float(f))
            if f.is_finite() && (*f - f.trunc()).abs() < f64::EPSILON =>
        {
            Some(*f as i64)
        }
        _ => None,
    }
}

fn matrix_call_term(fn_ir: &FnIR, args: &[ValueId]) -> TypeTerm {
    let elem = args
        .first()
        .map(|arg| match &fn_ir.values[*arg].value_term {
            TypeTerm::Vector(inner)
            | TypeTerm::Matrix(inner)
            | TypeTerm::MatrixDim(inner, _, _) => inner.as_ref().clone(),
            other => other.clone(),
        })
        .unwrap_or(TypeTerm::Double);
    let rows = args
        .get(1)
        .and_then(|arg| const_integral_value(fn_ir, *arg));
    let cols = args
        .get(2)
        .and_then(|arg| const_integral_value(fn_ir, *arg));
    if rows.is_some() || cols.is_some() {
        TypeTerm::MatrixDim(Box::new(elem), rows, cols)
    } else {
        TypeTerm::Matrix(Box::new(elem))
    }
}

pub(super) fn analyze_function_terms(
    fn_ir: &mut FnIR,
    fn_ret: &FxHashMap<String, TypeTerm>,
) -> TypeTerm {
    let mut changed = true;
    let mut guard = 0usize;

    while changed && guard < 32 {
        guard += 1;
        changed = false;
        for vid in 0..fn_ir.values.len() {
            let old = fn_ir.values[vid].value_term.clone();
            let new = infer_value_term(fn_ir, vid, fn_ret);
            let joined = old.join(&new);
            if joined != old {
                fn_ir.values[vid].value_term = joined;
                changed = true;
            }
        }
    }

    let mut cs = ConstraintSet::default();
    let vars: Vec<_> = (0..fn_ir.values.len()).map(|_| cs.fresh_var()).collect();
    for (vid, v) in fn_ir.values.iter().enumerate() {
        cs.add(TypeConstraint::Bind(vars[vid], v.value_term.clone()));
        match &v.kind {
            ValueKind::Phi { args } => {
                for (arg, _) in args {
                    cs.add(TypeConstraint::Eq(vars[vid], vars[*arg]));
                }
            }
            ValueKind::Index1D { base, .. } => {
                cs.add(TypeConstraint::ElementOf {
                    container: vars[*base],
                    element: vars[vid],
                });
            }
            ValueKind::Call { callee, args, .. } if callee == "unbox" && !args.is_empty() => {
                cs.add(TypeConstraint::Unbox {
                    boxed: vars[args[0]],
                    value: vars[vid],
                });
            }
            _ => {}
        }
    }
    cs.solve();
    for (vid, slot) in fn_ir.values.iter_mut().enumerate() {
        let resolved = cs.resolve(vars[vid]);
        slot.value_term = slot.value_term.join(&resolved);
    }

    let mut ret_term = TypeTerm::Any;
    let reachable = compute_reachable(fn_ir);
    for (bid, bb) in fn_ir.blocks.iter().enumerate() {
        if !reachable.get(bid).copied().unwrap_or(false) {
            continue;
        }
        if let Terminator::Return(Some(v)) = bb.term {
            ret_term = ret_term.join(&fn_ir.values[v].value_term);
        }
    }

    if ret_term.is_any()
        && let Some(h) = &fn_ir.ret_term_hint
    {
        ret_term = h.clone();
    }

    ret_term
}

pub(super) fn infer_value_term(
    fn_ir: &FnIR,
    vid: ValueId,
    fn_ret: &FxHashMap<String, TypeTerm>,
) -> TypeTerm {
    let val = &fn_ir.values[vid];
    match &val.kind {
        ValueKind::Const(l) => lit_term(l),
        ValueKind::Param { index } => fn_ir
            .param_term_hints
            .get(*index)
            .cloned()
            .unwrap_or(TypeTerm::Any),
        ValueKind::Len { .. } => TypeTerm::Int,
        ValueKind::Indices { .. } | ValueKind::Range { .. } => {
            TypeTerm::Vector(Box::new(TypeTerm::Int))
        }
        ValueKind::Unary { rhs, .. } => {
            let r = fn_ir.values[*rhs].value_term.clone();
            match r {
                TypeTerm::Int | TypeTerm::Double => r,
                TypeTerm::Vector(inner) => TypeTerm::Vector(inner),
                _ => TypeTerm::Any,
            }
        }
        ValueKind::Binary { op, lhs, rhs } => {
            use crate::syntax::ast::BinOp;
            let l = fn_ir.values[*lhs].value_term.clone();
            let r = fn_ir.values[*rhs].value_term.clone();
            match op {
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    TypeTerm::Logical
                }
                BinOp::And | BinOp::Or => TypeTerm::Logical,
                BinOp::MatMul => {
                    let l_parts = l.matrix_parts().or_else(|| match &l {
                        TypeTerm::Vector(inner) => Some((inner.as_ref(), Some(1), None)),
                        _ => None,
                    });
                    let r_parts = r.matrix_parts().or_else(|| match &r {
                        TypeTerm::Vector(inner) => Some((inner.as_ref(), None, Some(1))),
                        _ => None,
                    });
                    match (l_parts, r_parts) {
                        (Some((le, lrows, _lcols)), Some((re, _rrows, rcols))) => {
                            let elem = le.join(re);
                            TypeTerm::MatrixDim(Box::new(elem), lrows, rcols)
                        }
                        _ => TypeTerm::Matrix(Box::new(TypeTerm::Double)),
                    }
                }
                _ => match (l, r) {
                    (TypeTerm::Double, TypeTerm::Int)
                    | (TypeTerm::Int, TypeTerm::Double)
                    | (TypeTerm::Double, TypeTerm::Double) => TypeTerm::Double,
                    (TypeTerm::Int, TypeTerm::Int) => TypeTerm::Int,
                    (TypeTerm::Vector(a), TypeTerm::Vector(b)) => {
                        TypeTerm::Vector(Box::new(a.join(&b)))
                    }
                    (TypeTerm::Vector(a), b) | (b, TypeTerm::Vector(a)) => {
                        TypeTerm::Vector(Box::new(a.join(&b)))
                    }
                    (TypeTerm::Matrix(a), TypeTerm::Matrix(b)) => {
                        TypeTerm::Matrix(Box::new(a.join(&b)))
                    }
                    (TypeTerm::MatrixDim(a, ar, ac), TypeTerm::MatrixDim(b, br, bc)) => {
                        TypeTerm::MatrixDim(Box::new(a.join(&b)), ar.or(br), ac.or(bc))
                    }
                    (TypeTerm::Matrix(a), TypeTerm::MatrixDim(b, _, _))
                    | (TypeTerm::MatrixDim(b, _, _), TypeTerm::Matrix(a)) => {
                        TypeTerm::Matrix(Box::new(a.join(&b)))
                    }
                    _ => TypeTerm::Any,
                },
            }
        }
        ValueKind::Phi { args } => {
            let mut out = TypeTerm::Any;
            for (a, _) in args {
                out = out.join(&fn_ir.values[*a].value_term);
            }
            out
        }
        ValueKind::Call { callee, args, .. } => {
            if callee == "matrix" {
                return matrix_call_term(fn_ir, args);
            }
            if callee == "rr_field_get" && !args.is_empty() {
                let field_name = args.get(1).and_then(|arg| match &fn_ir.values[*arg].kind {
                    ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                    _ => None,
                });
                return fn_ir.values[args[0]]
                    .value_term
                    .field_value_named(field_name);
            }
            if callee == "rr_field_exists" {
                return TypeTerm::Logical;
            }
            if callee == "rr_field_set" && !args.is_empty() {
                let field_name = args.get(1).and_then(|arg| match &fn_ir.values[*arg].kind {
                    ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                    _ => None,
                });
                if let (Some(name), Some(value)) = (field_name, args.get(2)) {
                    return fn_ir.values[args[0]]
                        .value_term
                        .updated_field_value_named(name, &fn_ir.values[*value].value_term);
                }
                return fn_ir.values[args[0]].value_term.clone();
            }
            let arg_terms: Vec<TypeTerm> = args
                .iter()
                .map(|a| fn_ir.values[*a].value_term.clone())
                .collect();
            if let Some(t) = infer_builtin_term(callee, &arg_terms) {
                return t;
            }
            if callee.starts_with("Sym_") {
                return fn_ret.get(callee).cloned().unwrap_or(TypeTerm::Any);
            }
            TypeTerm::Any
        }
        ValueKind::Index1D { base, .. }
        | ValueKind::Index2D { base, .. }
        | ValueKind::Index3D { base, .. } => fn_ir.values[*base].value_term.index_element(),
        ValueKind::Load { .. } | ValueKind::RSymbol { .. } => TypeTerm::Any,
        ValueKind::Intrinsic { op, args } => {
            use crate::mir::IntrinsicOp;
            match op {
                IntrinsicOp::VecSumF64 | IntrinsicOp::VecMeanF64 => TypeTerm::Double,
                _ => {
                    if args.is_empty() {
                        TypeTerm::Any
                    } else {
                        TypeTerm::Vector(Box::new(TypeTerm::Double))
                    }
                }
            }
        }
    }
}
