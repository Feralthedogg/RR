use super::*;

pub(super) fn resolve_index1d_expr(
    this: &RBackend,
    base: usize,
    idx: usize,
    is_safe: bool,
    is_na_safe: bool,
    values: &[Value],
    params: &[String],
) -> String {
    let b = this.resolve_read_base(base, values, params);
    if let Some(end_expr) = this.known_full_end_expr_for_value(base, values, params)
        && this.value_is_one_based_full_range_alias(
            idx,
            end_expr.as_str(),
            values,
            params,
            &mut FxHashSet::default(),
        )
    {
        return b;
    }
    let i = this.resolve_preferred_plain_symbol_expr(idx, values, params);
    if (is_safe && is_na_safe) || this.can_elide_index_expr(idx, values, params) {
        format!("{}[{}]", b, i)
    } else {
        format!("rr_index1_read({}, {}, \"index\")", b, i)
    }
}

pub(super) fn resolve_index2d_expr(
    this: &RBackend,
    base: usize,
    r: usize,
    c: usize,
    values: &[Value],
    params: &[String],
) -> String {
    let b = this.resolve_read_base(base, values, params);
    let rr = this.resolve_preferred_plain_symbol_expr(r, values, params);
    let cc = this.resolve_preferred_plain_symbol_expr(c, values, params);
    let r_idx = if this.can_elide_index_expr(r, values, params) {
        rr
    } else {
        format!("rr_index1_write({}, \"row\")", rr)
    };
    let c_idx = if this.can_elide_index_expr(c, values, params) {
        cc
    } else {
        format!("rr_index1_write({}, \"col\")", cc)
    };
    format!("{}[{}, {}]", b, r_idx, c_idx)
}

pub(super) fn resolve_index3d_expr(
    this: &RBackend,
    base: usize,
    i: usize,
    j: usize,
    k: usize,
    values: &[Value],
    params: &[String],
) -> String {
    let b = this.resolve_read_base(base, values, params);
    let i_val = this.resolve_preferred_plain_symbol_expr(i, values, params);
    let j_val = this.resolve_preferred_plain_symbol_expr(j, values, params);
    let k_val = this.resolve_preferred_plain_symbol_expr(k, values, params);
    let i_idx = if this.can_elide_index_expr(i, values, params) {
        i_val
    } else {
        format!("rr_index1_write({}, \"dim1\")", i_val)
    };
    let j_idx = if this.can_elide_index_expr(j, values, params) {
        j_val
    } else {
        format!("rr_index1_write({}, \"dim2\")", j_val)
    };
    let k_idx = if this.can_elide_index_expr(k, values, params) {
        k_val
    } else {
        format!("rr_index1_write({}, \"dim3\")", k_val)
    };
    format!("{}[{}, {}, {}]", b, i_idx, j_idx, k_idx)
}

pub(super) fn resolve_cond(
    this: &RBackend,
    cond: usize,
    values: &[Value],
    params: &[String],
) -> String {
    let c = this.resolve_preferred_plain_symbol_expr(cond, values, params);
    let typed_bool_scalar = matches!(values[cond].value_term, TypeTerm::Logical)
        && values[cond].value_ty.shape == ShapeTy::Scalar;
    if values[cond].value_ty.is_logical_scalar_non_na()
        || typed_bool_scalar
        || comparison_is_scalar_non_na(this, cond, values)
    {
        c
    } else {
        format!("rr_truthy1({}, \"condition\")", c)
    }
}

pub(super) fn comparison_is_scalar_non_na(this: &RBackend, cond: usize, values: &[Value]) -> bool {
    let ValueKind::Binary { op, lhs, rhs } = values[cond].kind else {
        return false;
    };
    if !matches!(
        op,
        BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
    ) {
        return false;
    }
    value_is_scalar_non_na(this, lhs, values) && value_is_scalar_non_na(this, rhs, values)
}

pub(super) fn value_is_scalar_non_na(this: &RBackend, value_id: usize, values: &[Value]) -> bool {
    let mut seen = FxHashSet::default();
    value_is_scalar_non_na_impl(this, value_id, values, &mut seen)
}

pub(super) fn value_is_scalar_non_na_impl(
    this: &RBackend,
    value_id: usize,
    values: &[Value],
    seen: &mut FxHashSet<usize>,
) -> bool {
    if !seen.insert(value_id) {
        return false;
    }
    let value = &values[value_id];
    let scalar_shape = value.value_ty.shape == ShapeTy::Scalar
        || value.facts.has(Facts::INT_SCALAR)
        || value.facts.has(Facts::BOOL_SCALAR);
    let non_na = value.value_ty.na == crate::typeck::NaTy::Never || value.facts.has(Facts::NON_NA);
    if scalar_shape && non_na {
        return true;
    }
    match &value.kind {
        ValueKind::Const(_) => true,
        ValueKind::Load { var } => this
            .resolve_bound_value_id(var)
            .filter(|bound_id| *bound_id != value_id)
            .is_some_and(|bound_id| value_is_scalar_non_na_impl(this, bound_id, values, seen)),
        ValueKind::Unary { rhs, .. } => value_is_scalar_non_na_impl(this, *rhs, values, seen),
        ValueKind::Binary { op, lhs, rhs } => match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul => {
                value_is_scalar_non_na_impl(this, *lhs, values, seen)
                    && value_is_scalar_non_na_impl(this, *rhs, values, seen)
            }
            BinOp::Div | BinOp::Mod => {
                value_is_scalar_non_na_impl(this, *lhs, values, seen)
                    && value_is_scalar_non_na_impl(this, *rhs, values, seen)
                    && value_is_proven_non_zero(this, *rhs, values)
            }
            BinOp::Eq
            | BinOp::Ne
            | BinOp::Lt
            | BinOp::Le
            | BinOp::Gt
            | BinOp::Ge
            | BinOp::And
            | BinOp::Or => {
                value_is_scalar_non_na_impl(this, *lhs, values, seen)
                    && value_is_scalar_non_na_impl(this, *rhs, values, seen)
            }
            _ => false,
        },
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            names.iter().all(|name| name.is_none())
                && args.len() == 1
                && matches!(callee.as_str(), "floor" | "ceiling" | "trunc" | "abs")
                && value_is_scalar_non_na_impl(this, args[0], values, seen)
        }
        _ => false,
    }
}

pub(super) fn value_is_proven_non_zero(this: &RBackend, value_id: usize, values: &[Value]) -> bool {
    let mut seen = FxHashSet::default();
    value_is_proven_non_zero_impl(this, value_id, values, &mut seen)
}

pub(super) fn value_is_proven_non_zero_impl(
    this: &RBackend,
    value_id: usize,
    values: &[Value],
    seen: &mut FxHashSet<usize>,
) -> bool {
    if !seen.insert(value_id) {
        return false;
    }
    match &values[value_id].kind {
        ValueKind::Const(Lit::Int(v)) => *v != 0,
        ValueKind::Const(Lit::Float(v)) => *v != 0.0,
        ValueKind::Load { var } => this
            .resolve_bound_value_id(var)
            .filter(|bound_id| *bound_id != value_id)
            .is_some_and(|bound_id| value_is_proven_non_zero_impl(this, bound_id, values, seen)),
        ValueKind::Unary {
            op: UnaryOp::Neg,
            rhs,
        } => value_is_proven_non_zero_impl(this, *rhs, values, seen),
        ValueKind::Binary {
            op: BinOp::Mul,
            lhs,
            rhs,
        } => {
            value_is_proven_non_zero_impl(this, *lhs, values, seen)
                && value_is_proven_non_zero_impl(this, *rhs, values, seen)
        }
        _ => false,
    }
}

pub(super) fn can_elide_identity_floor_call(
    callee: &str,
    args: &[usize],
    names: &[Option<String>],
    values: &[Value],
) -> bool {
    if !matches!(callee, "floor" | "ceiling" | "trunc") {
        return false;
    }
    if args.len() != 1 || names.len() > 1 {
        return false;
    }
    if names
        .first()
        .and_then(std::option::Option::as_ref)
        .is_some()
    {
        return false;
    }
    values
        .get(args[0])
        .map(|v| v.value_ty.is_int_scalar_non_na() || v.facts.has(Facts::INT_SCALAR))
        .unwrap_or(false)
}

pub(super) fn floor_index_read_components(
    callee: &str,
    args: &[usize],
    names: &[Option<String>],
    values: &[Value],
) -> Option<(usize, usize)> {
    if !matches!(callee, "floor" | "ceiling" | "trunc") {
        return None;
    }
    if args.len() != 1 || names.len() > 1 {
        return None;
    }
    if names
        .first()
        .and_then(std::option::Option::as_ref)
        .is_some()
    {
        return None;
    }
    let inner = *args.first()?;
    match &values.get(inner)?.kind {
        ValueKind::Index1D { base, idx, .. } => Some((*base, *idx)),
        ValueKind::Call {
            callee: inner_callee,
            args: inner_args,
            names: inner_names,
        } if matches!(
            inner_callee.as_str(),
            "rr_index1_read" | "rr_index1_read_strict" | "rr_index1_read_floor"
        ) && (inner_args.len() == 2 || inner_args.len() == 3)
            && inner_names.iter().take(2).all(std::option::Option::is_none) =>
        {
            Some((inner_args[0], inner_args[1]))
        }
        _ => None,
    }
}

pub(super) fn can_elide_index_wrapper(idx: usize, values: &[Value]) -> bool {
    let Some(v) = values.get(idx) else {
        return false;
    };
    if v.facts
        .has(Facts::ONE_BASED | Facts::INT_SCALAR | Facts::NON_NA)
    {
        return true;
    }
    if v.facts.has(Facts::INT_SCALAR | Facts::NON_NA) && v.facts.interval.min >= 1 {
        return true;
    }
    match &v.kind {
        ValueKind::Const(Lit::Int(n)) => *n >= 1,
        ValueKind::Const(Lit::Float(f))
            if f.is_finite()
                && (*f - f.trunc()).abs() < f64::EPSILON
                && *f >= 1.0
                && *f <= i64::MAX as f64 =>
        {
            true
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } if callee == "rr_index1_read_idx"
            && (args.len() == 2 || args.len() == 3)
            && names.iter().take(2).all(std::option::Option::is_none) =>
        {
            true
        }
        ValueKind::Call { callee, args, .. }
            if (callee == "rr_wrap_index_vec_i" && (args.len() == 4 || args.len() == 5))
                || (callee == "rr_idx_cube_vec_i" && (args.len() == 4 || args.len() == 5)) =>
        {
            true
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } if callee == "rr_idx_cube_vec_i"
            && (args.len() == 4 || args.len() == 5)
            && names.iter().take(4).all(std::option::Option::is_none) =>
        {
            true
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } if callee == "rr_wrap_index_vec_i"
            && (args.len() == 4 || args.len() == 5)
            && names.iter().take(4).all(std::option::Option::is_none) =>
        {
            true
        }
        _ => false,
    }
}
