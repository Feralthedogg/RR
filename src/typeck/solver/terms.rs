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
            | TypeTerm::VectorLen(inner, _)
            | TypeTerm::Matrix(inner)
            | TypeTerm::MatrixDim(inner, _, _)
            | TypeTerm::ArrayDim(inner, _) => inner.as_ref().clone(),
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

fn data_frame_column_term(term: &TypeTerm) -> TypeTerm {
    match term {
        TypeTerm::Vector(inner)
        | TypeTerm::VectorLen(inner, _)
        | TypeTerm::Matrix(inner)
        | TypeTerm::MatrixDim(inner, _, _)
        | TypeTerm::ArrayDim(inner, _) => TypeTerm::Vector(inner.clone()),
        other => TypeTerm::Vector(Box::new(other.clone())),
    }
}

fn dataframe_first_column_term(term: &TypeTerm) -> TypeTerm {
    match term {
        TypeTerm::DataFrameNamed(cols) => cols
            .first()
            .map(|(_, term)| term.clone())
            .unwrap_or_else(|| TypeTerm::Vector(Box::new(TypeTerm::Any))),
        TypeTerm::DataFrame(cols) => cols
            .first()
            .cloned()
            .unwrap_or_else(|| TypeTerm::Vector(Box::new(TypeTerm::Any))),
        _ => TypeTerm::Vector(Box::new(TypeTerm::Any)),
    }
}

fn infer_named_package_call_term(
    callee: &str,
    names: &[Option<String>],
    arg_terms: &[TypeTerm],
) -> Option<TypeTerm> {
    match callee {
        "base::data.frame" => {
            if !arg_terms.is_empty() && names.iter().all(Option::is_some) {
                let cols = names
                    .iter()
                    .zip(arg_terms.iter())
                    .filter_map(|(name, term)| {
                        name.as_ref()
                            .map(|name| (name.clone(), data_frame_column_term(term)))
                    })
                    .collect();
                Some(TypeTerm::DataFrameNamed(cols))
            } else {
                Some(TypeTerm::DataFrame(
                    arg_terms.iter().map(data_frame_column_term).collect(),
                ))
            }
        }
        "compiler::setCompilerOptions" => {
            if !arg_terms.is_empty() && names.iter().all(Option::is_some) {
                let fields = names
                    .iter()
                    .zip(arg_terms.iter())
                    .filter_map(|(name, term)| {
                        name.as_ref().map(|name| {
                            let normalized = match name.as_str() {
                                "optimize" => TypeTerm::Double,
                                "suppressAll" => TypeTerm::Logical,
                                "suppressUndefined" => TypeTerm::Vector(Box::new(TypeTerm::Char)),
                                "suppressNoSuperAssignVar" => TypeTerm::Logical,
                                _ => term.clone(),
                            };
                            (name.clone(), normalized)
                        })
                    })
                    .collect();
                Some(TypeTerm::NamedList(fields))
            } else {
                Some(TypeTerm::List(Box::new(TypeTerm::Any)))
            }
        }
        "base::vector" => Some(TypeTerm::Vector(Box::new(TypeTerm::Any))),
        "stats::model.frame" | "stats::model.frame.default" => {
            let data_arg = names
                .iter()
                .position(|name| name.as_deref() == Some("data"))
                .and_then(|idx| arg_terms.get(idx).cloned())
                .or_else(|| arg_terms.get(1).cloned());
            match data_arg {
                Some(TypeTerm::DataFrameNamed(cols)) => Some(TypeTerm::DataFrameNamed(cols)),
                Some(TypeTerm::DataFrame(cols)) => Some(TypeTerm::DataFrame(cols)),
                _ => Some(TypeTerm::DataFrame(Vec::new())),
            }
        }
        "stats::terms.formula" | "stats::delete.response" | "stats::drop.terms" => {
            Some(terms_model_term())
        }
        "stats::get_all_vars" => {
            let data_arg = names
                .iter()
                .position(|name| name.as_deref() == Some("data"))
                .and_then(|idx| arg_terms.get(idx).cloned());
            match data_arg {
                Some(TypeTerm::DataFrameNamed(cols)) => Some(TypeTerm::DataFrameNamed(cols)),
                Some(TypeTerm::DataFrame(cols)) => Some(TypeTerm::DataFrame(cols)),
                _ => Some(TypeTerm::DataFrame(Vec::new())),
            }
        }
        "stats::model.response" => arg_terms.first().map(dataframe_first_column_term),
        "stats::model.extract" => {
            let want_response = names
                .iter()
                .position(|name| name.is_none())
                .and_then(|idx| arg_terms.get(idx + 1))
                .is_some_and(|term| matches!(term, TypeTerm::Char));
            if want_response {
                arg_terms.first().map(dataframe_first_column_term)
            } else {
                Some(TypeTerm::Any)
            }
        }
        "stats::step" | "stats::update.default" | "stats::update.formula" => {
            arg_terms.first().cloned()
        }
        _ => None,
    }
}

fn unique_assign_source_for_var(fn_ir: &FnIR, var: &str) -> Option<ValueId> {
    let mut found = None;
    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            if let Instr::Assign { dst, src, .. } = instr
                && dst == var
            {
                if found.is_some() {
                    return None;
                }
                found = Some(*src);
            }
        }
    }
    found
}

fn resolve_package_call_source_inner(
    fn_ir: &FnIR,
    vid: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> Option<String> {
    if !seen_vals.insert(vid) {
        return None;
    }
    match &fn_ir.values.get(vid)?.kind {
        ValueKind::Call { callee, args, .. }
            if matches!(
                callee.as_str(),
                "stats::update" | "stats::update.default" | "stats::step"
            ) && !args.is_empty() =>
        {
            resolve_package_call_source_inner(fn_ir, args[0], seen_vals, seen_vars)
        }
        ValueKind::Call { callee, .. } if callee.contains("::") => Some(callee.clone()),
        ValueKind::Load { var } => {
            if var.contains("::") {
                return Some(var.clone());
            }
            if !seen_vars.insert(var.clone()) {
                return None;
            }
            let src = unique_assign_source_for_var(fn_ir, var)?;
            resolve_package_call_source_inner(fn_ir, src, seen_vals, seen_vars)
        }
        ValueKind::Phi { args } => {
            let mut out: Option<String> = None;
            for (src, _) in args {
                let resolved =
                    resolve_package_call_source_inner(fn_ir, *src, seen_vals, seen_vars)?;
                match &out {
                    None => out = Some(resolved),
                    Some(prev) if prev == &resolved => {}
                    Some(_) => return None,
                }
            }
            out
        }
        _ => None,
    }
}

fn resolve_package_call_source(fn_ir: &FnIR, vid: ValueId) -> Option<String> {
    resolve_package_call_source_inner(
        fn_ir,
        vid,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

fn resolve_package_call_origin_inner(
    fn_ir: &FnIR,
    vid: ValueId,
    seen_vals: &mut FxHashSet<ValueId>,
    seen_vars: &mut FxHashSet<String>,
) -> Option<ValueId> {
    if !seen_vals.insert(vid) {
        return None;
    }
    match &fn_ir.values.get(vid)?.kind {
        ValueKind::Call { callee, args, .. }
            if matches!(callee.as_str(), "stats::update" | "stats::step") && !args.is_empty() =>
        {
            resolve_package_call_origin_inner(fn_ir, args[0], seen_vals, seen_vars)
        }
        ValueKind::Call { callee, .. } if callee.contains("::") => Some(vid),
        ValueKind::Load { var } => {
            if var.contains("::") {
                return Some(vid);
            }
            if !seen_vars.insert(var.clone()) {
                return None;
            }
            let src = unique_assign_source_for_var(fn_ir, var)?;
            resolve_package_call_origin_inner(fn_ir, src, seen_vals, seen_vars)
        }
        ValueKind::Phi { args } => {
            let mut out: Option<ValueId> = None;
            for (src, _) in args {
                let resolved =
                    resolve_package_call_origin_inner(fn_ir, *src, seen_vals, seen_vars)?;
                match out {
                    None => out = Some(resolved),
                    Some(prev) if prev == resolved => {}
                    Some(_) => return None,
                }
            }
            out
        }
        _ => None,
    }
}

fn resolve_package_call_origin(fn_ir: &FnIR, vid: ValueId) -> Option<ValueId> {
    resolve_package_call_origin_inner(
        fn_ir,
        vid,
        &mut FxHashSet::default(),
        &mut FxHashSet::default(),
    )
}

fn named_call_arg_value(args: &[ValueId], names: &[Option<String>], name: &str) -> Option<ValueId> {
    args.iter()
        .zip(names.iter())
        .find_map(|(arg, field)| (field.as_deref() == Some(name)).then_some(*arg))
}

fn positional_call_arg_value(args: &[ValueId], idx: usize) -> Option<ValueId> {
    args.get(idx).copied()
}

fn visible_model_data_term(fn_ir: &FnIR, model_vid: ValueId) -> Option<TypeTerm> {
    let origin = resolve_package_call_origin(fn_ir, model_vid)?;
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &fn_ir.values.get(origin)?.kind
    else {
        return None;
    };
    if !matches!(callee.as_str(), "stats::lm" | "stats::glm") {
        return None;
    }
    let data_arg =
        named_call_arg_value(args, names, "data").or_else(|| positional_call_arg_value(args, 1))?;
    match fn_ir.values[data_arg].value_term.clone() {
        TypeTerm::DataFrameNamed(cols) => Some(TypeTerm::DataFrameNamed(cols)),
        TypeTerm::DataFrame(cols) => Some(TypeTerm::DataFrame(cols)),
        _ => None,
    }
}

fn known_vector_len(fn_ir: &FnIR, vid: ValueId) -> Option<i64> {
    match &fn_ir.values.get(vid)?.kind {
        ValueKind::Const(_) => Some(1),
        ValueKind::Call { callee, args, .. } if callee == "c" => {
            let mut total = 0_i64;
            for arg in args {
                total += known_vector_len(fn_ir, *arg)?;
            }
            Some(total)
        }
        ValueKind::Load { var } => {
            let src = unique_assign_source_for_var(fn_ir, var)?;
            known_vector_len(fn_ir, src)
        }
        ValueKind::Phi { args } => {
            let first = known_vector_len(fn_ir, args.first()?.0)?;
            for (src, _) in &args[1..] {
                if known_vector_len(fn_ir, *src)? != first {
                    return None;
                }
            }
            Some(first)
        }
        _ => None,
    }
}

fn known_dataframe_nrow(fn_ir: &FnIR, vid: ValueId) -> Option<i64> {
    match &fn_ir.values.get(vid)?.kind {
        ValueKind::Call { callee, args, .. } if callee == "base::data.frame" => {
            let first = known_vector_len(fn_ir, *args.first()?)?;
            for arg in &args[1..] {
                if known_vector_len(fn_ir, *arg)? != first {
                    return None;
                }
            }
            Some(first)
        }
        ValueKind::Load { var } => {
            let src = unique_assign_source_for_var(fn_ir, var)?;
            known_dataframe_nrow(fn_ir, src)
        }
        ValueKind::Phi { args } => {
            let first = known_dataframe_nrow(fn_ir, args.first()?.0)?;
            for (src, _) in &args[1..] {
                if known_dataframe_nrow(fn_ir, *src)? != first {
                    return None;
                }
            }
            Some(first)
        }
        _ => None,
    }
}

fn visible_model_data_nrow(fn_ir: &FnIR, model_vid: ValueId) -> Option<i64> {
    let origin = resolve_package_call_origin(fn_ir, model_vid)?;
    let ValueKind::Call {
        callee,
        args,
        names,
    } = &fn_ir.values.get(origin)?.kind
    else {
        return None;
    };
    if !matches!(callee.as_str(), "stats::lm" | "stats::glm") {
        return None;
    }
    let data_arg =
        named_call_arg_value(args, names, "data").or_else(|| positional_call_arg_value(args, 1))?;
    known_dataframe_nrow(fn_ir, data_arg)
}

fn simple_formula_design_cols(src: &str) -> Option<i64> {
    let (_, rhs) = src.split_once('~')?;
    let compact: String = rhs.chars().filter(|c| !c.is_whitespace()).collect();
    if compact.is_empty() {
        return None;
    }
    if compact.contains(':')
        || compact.contains('*')
        || compact.contains('(')
        || compact.contains(')')
    {
        return None;
    }

    let mut intercept = true;
    if compact == "0" || compact == "-1" {
        return Some(0);
    }

    let mut sign = '+';
    let mut current = String::new();
    let mut term_count = 0_i64;

    let mut flush = |sign: char, token: &str| -> Option<()> {
        if token.is_empty() {
            return None;
        }
        match token {
            "0" => {
                intercept = sign == '-';
                Some(())
            }
            "1" => {
                intercept = sign != '-';
                Some(())
            }
            _ => {
                if sign == '-' {
                    return None;
                }
                term_count += 1;
                Some(())
            }
        }
    };

    for ch in compact.chars() {
        if ch == '+' || ch == '-' {
            if current.is_empty() {
                sign = ch;
                continue;
            }
            flush(sign, &current)?;
            current.clear();
            sign = ch;
            continue;
        }
        current.push(ch);
    }
    flush(sign, &current)?;

    Some(term_count + i64::from(intercept))
}

fn known_formula_design_cols(fn_ir: &FnIR, vid: ValueId) -> Option<i64> {
    match &fn_ir.values.get(vid)?.kind {
        ValueKind::Const(crate::syntax::ast::Lit::Str(src)) => simple_formula_design_cols(src),
        ValueKind::Call { callee, args, .. } if callee == "stats::as.formula" => {
            known_formula_design_cols(fn_ir, *args.first()?)
        }
        ValueKind::Load { var } => {
            let src = unique_assign_source_for_var(fn_ir, var)?;
            known_formula_design_cols(fn_ir, src)
        }
        ValueKind::Phi { args } => {
            let first = known_formula_design_cols(fn_ir, args.first()?.0)?;
            for (src, _) in &args[1..] {
                if known_formula_design_cols(fn_ir, *src)? != first {
                    return None;
                }
            }
            Some(first)
        }
        _ => None,
    }
}

fn visible_model_formula_cols(fn_ir: &FnIR, model_vid: ValueId) -> Option<i64> {
    if let Some(value) = fn_ir.values.get(model_vid)
        && let ValueKind::Call { callee, args, .. } = &value.kind
        && matches!(callee.as_str(), "stats::update" | "stats::step")
    {
        if let Some(formula_arg) = args.get(1).copied()
            && let Some(cols) = known_formula_design_cols(fn_ir, formula_arg)
        {
            return Some(cols);
        }
        if let Some(base_model) = args.first().copied() {
            return visible_model_formula_cols(fn_ir, base_model);
        }
    }

    let origin = resolve_package_call_origin(fn_ir, model_vid)?;
    let ValueKind::Call { callee, args, .. } = &fn_ir.values.get(origin)?.kind else {
        return None;
    };
    if !matches!(callee.as_str(), "stats::lm" | "stats::glm") {
        return None;
    }
    known_formula_design_cols(fn_ir, *args.first()?)
}

fn summary_lm_term() -> TypeTerm {
    TypeTerm::NamedList(vec![
        ("call".to_string(), TypeTerm::Any),
        ("terms".to_string(), terms_model_term()),
        (
            "residuals".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Double)),
        ),
        (
            "coefficients".to_string(),
            TypeTerm::Matrix(Box::new(TypeTerm::Double)),
        ),
        (
            "aliased".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Logical)),
        ),
        ("sigma".to_string(), TypeTerm::Double),
        ("df".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
        ("r.squared".to_string(), TypeTerm::Double),
        ("adj.r.squared".to_string(), TypeTerm::Double),
        (
            "fstatistic".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Double)),
        ),
        (
            "cov.unscaled".to_string(),
            TypeTerm::Matrix(Box::new(TypeTerm::Double)),
        ),
    ])
}

fn glm_family_term() -> TypeTerm {
    TypeTerm::NamedList(vec![
        ("family".to_string(), TypeTerm::Char),
        ("link".to_string(), TypeTerm::Char),
    ])
}

fn summary_glm_term() -> TypeTerm {
    TypeTerm::NamedList(vec![
        ("call".to_string(), TypeTerm::Any),
        ("terms".to_string(), terms_model_term()),
        ("family".to_string(), glm_family_term()),
        ("deviance".to_string(), TypeTerm::Double),
        ("aic".to_string(), TypeTerm::Double),
        ("contrasts".to_string(), TypeTerm::Any),
        ("df.residual".to_string(), TypeTerm::Int),
        ("null.deviance".to_string(), TypeTerm::Double),
        ("df.null".to_string(), TypeTerm::Int),
        ("iter".to_string(), TypeTerm::Int),
        (
            "deviance.resid".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Double)),
        ),
        (
            "coefficients".to_string(),
            TypeTerm::Matrix(Box::new(TypeTerm::Double)),
        ),
        (
            "aliased".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Logical)),
        ),
        ("dispersion".to_string(), TypeTerm::Double),
        ("df".to_string(), TypeTerm::Vector(Box::new(TypeTerm::Int))),
        (
            "cov.unscaled".to_string(),
            TypeTerm::Matrix(Box::new(TypeTerm::Double)),
        ),
        (
            "cov.scaled".to_string(),
            TypeTerm::Matrix(Box::new(TypeTerm::Double)),
        ),
    ])
}

fn terms_model_term() -> TypeTerm {
    TypeTerm::NamedList(vec![
        (
            "variables".to_string(),
            TypeTerm::List(Box::new(TypeTerm::Any)),
        ),
        (
            "factors".to_string(),
            TypeTerm::Matrix(Box::new(TypeTerm::Int)),
        ),
        (
            "term.labels".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Char)),
        ),
        (
            "order".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Int)),
        ),
        ("intercept".to_string(), TypeTerm::Int),
        ("response".to_string(), TypeTerm::Int),
        (
            "predvars".to_string(),
            TypeTerm::List(Box::new(TypeTerm::Any)),
        ),
        (
            "dataClasses".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Char)),
        ),
        (
            "class".to_string(),
            TypeTerm::Vector(Box::new(TypeTerm::Char)),
        ),
        (".Environment".to_string(), TypeTerm::Any),
    ])
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
                TypeTerm::VectorLen(inner, len) => TypeTerm::VectorLen(inner, len),
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
                        TypeTerm::Vector(inner) | TypeTerm::VectorLen(inner, _) => {
                            Some((inner.as_ref(), Some(1), None))
                        }
                        _ => None,
                    });
                    let r_parts = r.matrix_parts().or_else(|| match &r {
                        TypeTerm::Vector(inner) | TypeTerm::VectorLen(inner, _) => {
                            Some((inner.as_ref(), None, Some(1)))
                        }
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
                    (TypeTerm::VectorLen(a, alen), TypeTerm::VectorLen(b, blen)) => {
                        TypeTerm::VectorLen(Box::new(a.join(&b)), alen.or(blen))
                    }
                    (TypeTerm::Vector(a), TypeTerm::VectorLen(b, _))
                    | (TypeTerm::VectorLen(a, _), TypeTerm::Vector(b)) => {
                        TypeTerm::Vector(Box::new(a.join(&b)))
                    }
                    (TypeTerm::Vector(a), b) | (b, TypeTerm::Vector(a)) => {
                        TypeTerm::Vector(Box::new(a.join(&b)))
                    }
                    (TypeTerm::VectorLen(a, _), b) | (b, TypeTerm::VectorLen(a, _)) => {
                        TypeTerm::Vector(Box::new(a.join(&b)))
                    }
                    (TypeTerm::Matrix(a), TypeTerm::Matrix(b)) => {
                        TypeTerm::Matrix(Box::new(a.join(&b)))
                    }
                    (TypeTerm::MatrixDim(a, ar, ac), TypeTerm::MatrixDim(b, br, bc)) => {
                        TypeTerm::MatrixDim(Box::new(a.join(&b)), ar.or(br), ac.or(bc))
                    }
                    (TypeTerm::ArrayDim(a, adims), TypeTerm::ArrayDim(b, bdims))
                        if adims.len() == bdims.len() =>
                    {
                        TypeTerm::ArrayDim(
                            Box::new(a.join(&b)),
                            adims
                                .iter()
                                .zip(bdims.iter())
                                .map(|(a, b)| (*a).or(*b))
                                .collect(),
                        )
                    }
                    (TypeTerm::Matrix(a), TypeTerm::MatrixDim(b, _, _))
                    | (TypeTerm::MatrixDim(b, _, _), TypeTerm::Matrix(a))
                    | (TypeTerm::Matrix(a), TypeTerm::ArrayDim(b, _))
                    | (TypeTerm::ArrayDim(b, _), TypeTerm::Matrix(a))
                    | (TypeTerm::MatrixDim(a, _, _), TypeTerm::ArrayDim(b, _))
                    | (TypeTerm::ArrayDim(b, _), TypeTerm::MatrixDim(a, _, _)) => {
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
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
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
            if callee == "base::summary"
                && let Some(arg) = args.first()
                && let Some(source) = resolve_package_call_source(fn_ir, *arg)
            {
                return match source.as_str() {
                    "stats::lm" => summary_lm_term(),
                    "stats::glm" => summary_glm_term(),
                    _ => TypeTerm::List(Box::new(TypeTerm::Any)),
                };
            }
            if matches!(
                callee.as_str(),
                "stats::model.frame" | "stats::model.frame.default"
            ) {
                if let Some(data_arg) = named_call_arg_value(args, names, "data")
                    .or_else(|| positional_call_arg_value(args, 1))
                {
                    return match &fn_ir.values[data_arg].value_term {
                        TypeTerm::DataFrameNamed(cols) => TypeTerm::DataFrameNamed(cols.clone()),
                        TypeTerm::DataFrame(cols) => TypeTerm::DataFrame(cols.clone()),
                        _ => TypeTerm::DataFrame(Vec::new()),
                    };
                }
                if let Some(model_arg) = args.first().copied()
                    && let Some(term) = visible_model_data_term(fn_ir, model_arg)
                {
                    return term;
                }
                return TypeTerm::DataFrame(Vec::new());
            }
            if matches!(
                callee.as_str(),
                "stats::model.matrix" | "stats::model.matrix.default" | "stats::model.matrix.lm"
            ) {
                if let Some(data_arg) = named_call_arg_value(args, names, "data")
                    .or_else(|| positional_call_arg_value(args, 1))
                {
                    let cols = args
                        .first()
                        .and_then(|formula_arg| known_formula_design_cols(fn_ir, *formula_arg));
                    return TypeTerm::MatrixDim(
                        Box::new(TypeTerm::Double),
                        known_dataframe_nrow(fn_ir, data_arg),
                        cols,
                    );
                }
                if let Some(model_arg) = args.first().copied() {
                    return TypeTerm::MatrixDim(
                        Box::new(TypeTerm::Double),
                        visible_model_data_nrow(fn_ir, model_arg),
                        visible_model_formula_cols(fn_ir, model_arg),
                    );
                }
                return TypeTerm::Matrix(Box::new(TypeTerm::Double));
            }
            if matches!(
                callee.as_str(),
                "stats::update" | "stats::update.default" | "stats::step"
            ) && !args.is_empty()
            {
                return fn_ir.values[args[0]].value_term.clone();
            }
            if callee == "stats::terms"
                && let Some(arg) = args.first()
                && let Some(source) = resolve_package_call_source(fn_ir, *arg)
                && matches!(source.as_str(), "stats::lm" | "stats::glm")
            {
                return terms_model_term();
            }
            if callee == "base::vector"
                && let Some(mode_name) =
                    args.first().and_then(|arg| match &fn_ir.values[*arg].kind {
                        ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                        _ => None,
                    })
            {
                return match mode_name {
                    "logical" => TypeTerm::Vector(Box::new(TypeTerm::Logical)),
                    "integer" => TypeTerm::Vector(Box::new(TypeTerm::Int)),
                    "double" | "numeric" => TypeTerm::Vector(Box::new(TypeTerm::Double)),
                    "character" => TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    "list" => TypeTerm::List(Box::new(TypeTerm::Any)),
                    _ => TypeTerm::Vector(Box::new(TypeTerm::Any)),
                };
            }
            if callee == "compiler::getCompilerOption"
                && let Some(option_name) =
                    args.first().and_then(|arg| match &fn_ir.values[*arg].kind {
                        ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                        _ => None,
                    })
            {
                return match option_name {
                    "optimize" => TypeTerm::Int,
                    "suppressAll" => TypeTerm::Logical,
                    "suppressUndefined" => TypeTerm::Vector(Box::new(TypeTerm::Char)),
                    "suppressNoSuperAssignVar" => TypeTerm::Logical,
                    _ => TypeTerm::Any,
                };
            }
            let arg_terms: Vec<TypeTerm> = args
                .iter()
                .map(|a| fn_ir.values[*a].value_term.clone())
                .collect();
            if let Some(t) = infer_builtin_term(callee, &arg_terms) {
                return t;
            }
            if let Some(t) = infer_named_package_call_term(callee, names, &arg_terms) {
                return t;
            }
            if let Some(t) = infer_package_call_term(callee, &arg_terms) {
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
        ValueKind::Load { var } => infer_package_binding_term(var).unwrap_or(TypeTerm::Any),
        ValueKind::RSymbol { .. } => TypeTerm::Any,
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
