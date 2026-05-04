use super::*;
pub(crate) fn finish_type_analysis(all_fns: &FxHashMap<String, FnIR>, cfg: TypeConfig) -> RR<()> {
    let mut type_errors = Vec::new();
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = all_fns.get(&name) else {
            continue;
        };
        if cfg.mode != TypeMode::Strict {
            continue;
        }

        if let Some(h) = &fn_ir.ret_term_hint {
            let inferred_term = &fn_ir.inferred_ret_term;
            if !h.is_any() && !inferred_term.is_any() && !h.compatible_with(inferred_term) {
                type_errors.push(
                    DiagnosticBuilder::new(
                        "RR.TypeError",
                        RRCode::E1010,
                        Stage::Mir,
                        format!(
                            "type hint conflict in function '{}': return hint {:?} vs inferred {:?}",
                            name, h, inferred_term
                        ),
                    )
                    .at(fn_ir.ret_hint_span.unwrap_or(fn_ir.span))
                    .constraint(
                        fn_ir.ret_hint_span.unwrap_or(fn_ir.span),
                        format!("declared return type is constrained to {:?}", h),
                    )
                    .origin(
                        first_return_origin_span(fn_ir).unwrap_or(fn_ir.span),
                        format!("inferred return flow produces {:?}", inferred_term),
                    )
                    .use_site(
                        fn_ir.span,
                        "function body must satisfy the declared return contract",
                    )
                    .note("Strict mode compares return hints against the inferred function result.")
                    .fix(format!(
                        "change the return annotation to {:?}, or return a value compatible with {:?}",
                        inferred_term, h
                    ))
                    .build(),
                );
            }
        } else if let Some(h) = fn_ir.ret_ty_hint {
            let inferred = fn_ir.inferred_ret_ty;
            if h != TypeState::unknown() && inferred != TypeState::unknown() {
                let clash = h.prim != PrimTy::Any
                    && inferred.prim != PrimTy::Any
                    && h.prim != inferred.prim;
                if clash {
                    type_errors.push(
                        DiagnosticBuilder::new(
                            "RR.TypeError",
                            RRCode::E1010,
                            Stage::Mir,
                            format!(
                                "type hint conflict in function '{}': return hint {:?} vs inferred {:?}",
                                name, h, inferred
                            ),
                        )
                        .at(fn_ir.ret_hint_span.unwrap_or(fn_ir.span))
                        .constraint(
                            fn_ir.ret_hint_span.unwrap_or(fn_ir.span),
                            format!("declared return type is constrained to {:?}", h),
                        )
                        .origin(
                            first_return_origin_span(fn_ir).unwrap_or(fn_ir.span),
                            format!("inferred return flow produces {:?}", inferred),
                        )
                        .use_site(
                            fn_ir.span,
                            "function body must satisfy the declared return contract",
                        )
                        .fix(format!(
                            "change the return annotation to {:?}, or return a {:?} value",
                            inferred, h
                        ))
                        .build(),
                    );
                }
            }
        }
    }

    if cfg.mode == TypeMode::Strict {
        type_errors.extend(validate_strict(all_fns));
    }
    finish_diagnostics(
        "RR.TypeError",
        RRCode::E1002,
        Stage::Mir,
        format!("type checking failed: {} error(s)", type_errors.len()),
        type_errors,
    )
}

pub(crate) fn validate_strict(all_fns: &FxHashMap<String, FnIR>) -> Vec<RRException> {
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    let mut errors = Vec::new();
    for fname in names {
        let Some(fn_ir) = all_fns.get(&fname) else {
            continue;
        };
        errors.extend(validate_strict_function(all_fns, &fname, fn_ir));
    }
    errors
}

pub(crate) fn validate_strict_function(
    all_fns: &FxHashMap<String, FnIR>,
    fname: &str,
    fn_ir: &FnIR,
) -> Vec<RRException> {
    let reachable = compute_reachable(fn_ir);
    let has_explicit_hints = fn_ir_has_user_type_contract(fn_ir);
    let mut errors = Vec::new();
    validate_strict_blocks(fname, fn_ir, &reachable, has_explicit_hints, &mut errors);
    validate_strict_values(all_fns, fname, fn_ir, has_explicit_hints, &mut errors);
    errors
}

pub(crate) fn validate_strict_blocks(
    fname: &str,
    fn_ir: &FnIR,
    reachable: &[bool],
    has_explicit_hints: bool,
    errors: &mut Vec<RRException>,
) {
    for (bid, bb) in fn_ir.blocks.iter().enumerate() {
        if !reachable.get(bid).copied().unwrap_or(false) {
            continue;
        }
        validate_strict_condition(fname, fn_ir, bb, has_explicit_hints, errors);
        for ins in &bb.instrs {
            validate_strict_store_instr(fname, fn_ir, ins, has_explicit_hints, errors);
        }
    }
}

pub(crate) fn validate_strict_condition(
    fname: &str,
    fn_ir: &FnIR,
    bb: &Block,
    has_explicit_hints: bool,
    errors: &mut Vec<RRException>,
) {
    if let Terminator::If { cond, .. } = bb.term {
        let cty = fn_ir.values[cond].value_ty;
        if has_explicit_hints && cty.is_unknown() {
            errors.push(
                DiagnosticBuilder::new(
                    "RR.TypeError",
                    RRCode::E1012,
                    Stage::Mir,
                    format!(
                        "strict mode unresolved condition type in function '{}' (value #{})",
                        fname, cond
                    ),
                )
                .at(fn_ir.values[cond].span)
                .origin(
                    fn_ir.values[cond].span,
                    "condition value originates here and has unresolved type facts",
                )
                .constraint(
                    fn_ir.span,
                    "strict mode requires branch conditions to be logical scalars",
                )
                .use_site(
                    fn_ir.values[cond].span,
                    "used here as an if/while condition",
                )
                .fix("add an explicit logical type hint or cast before the condition")
                .build(),
            );
        }
    }
}

pub(crate) fn validate_strict_store_instr(
    fname: &str,
    fn_ir: &FnIR,
    ins: &crate::mir::Instr,
    has_explicit_hints: bool,
    errors: &mut Vec<RRException>,
) {
    if !has_explicit_hints {
        return;
    }
    match ins {
        crate::mir::Instr::StoreIndex1D { idx, .. } => {
            validate_unknown_index_type(fname, fn_ir, *idx, UNKNOWN_ASSIGNMENT_INDEX, errors);
        }
        crate::mir::Instr::StoreIndex2D { base, r, c, .. } => {
            validate_matrix_base_type(fname, fn_ir, *base, fn_ir.values[*base].span, true, errors);
            for idx in [*r, *c] {
                validate_unknown_index_type(fname, fn_ir, idx, UNKNOWN_MATRIX_INDEX, errors);
            }
        }
        _ => {}
    }
}

pub(crate) fn validate_strict_values(
    all_fns: &FxHashMap<String, FnIR>,
    fname: &str,
    fn_ir: &FnIR,
    has_explicit_hints: bool,
    errors: &mut Vec<RRException>,
) {
    for v in &fn_ir.values {
        if has_explicit_hints {
            validate_index2d_value(fname, fn_ir, v, errors);
            validate_field_get_value(fname, fn_ir, v, errors);
            validate_field_set_value(fname, fn_ir, v, errors);
            validate_field_intrinsic_call(fname, fn_ir, v, errors);
        }
        validate_user_call_signature(all_fns, fn_ir, v, errors);
    }
}

pub(crate) fn validate_index2d_value(
    fname: &str,
    fn_ir: &FnIR,
    v: &Value,
    errors: &mut Vec<RRException>,
) {
    if let ValueKind::Index2D { base, r, c } = &v.kind {
        validate_matrix_base_type(fname, fn_ir, *base, v.span, false, errors);
        for idx in [r, c] {
            validate_unknown_index_type(fname, fn_ir, *idx, UNKNOWN_MATRIX_INDEX, errors);
        }
    }
}

pub(crate) fn validate_matrix_base_type(
    fname: &str,
    fn_ir: &FnIR,
    base: ValueId,
    use_span: crate::utils::Span,
    assignment: bool,
    errors: &mut Vec<RRException>,
) {
    let base_ty = fn_ir.values[base].value_ty;
    if base_ty.shape == ShapeTy::Unknown || base_ty.shape == ShapeTy::Matrix {
        return;
    }
    let (message, constraint_message, use_message) = if assignment {
        (
            format!(
                "strict mode 2D assignment requires matrix-typed base in function '{}' (got {:?})",
                fname, base_ty
            ),
            "2D assignment requires a matrix-typed base",
            "used here as a 2D assignment base",
        )
    } else {
        (
            format!(
                "strict mode 2D indexing requires matrix-typed base in function '{}' (got {:?})",
                fname, base_ty
            ),
            "2D indexing requires a matrix-typed base",
            "used here as a 2D indexing expression",
        )
    };
    errors.push(
        DiagnosticBuilder::new("RR.TypeError", RRCode::E1002, Stage::Mir, message)
            .at(use_span)
            .constraint(use_span, constraint_message)
            .use_site(use_span, use_message)
            .fix("change the base type hint to matrix<T>, or use 1D indexing")
            .build(),
    );
}

#[derive(Clone, Copy)]
pub(crate) enum UnknownIndexShape {
    Index1D,
    Matrix2D,
}

#[derive(Clone, Copy)]
pub(crate) enum OriginNote {
    Include,
    Omit,
}

#[derive(Clone, Copy)]
pub(crate) struct UnknownIndexDiagnosticContext {
    pub(crate) constraint_subject: &'static str,
    pub(crate) use_subject: &'static str,
    pub(crate) shape: UnknownIndexShape,
    pub(crate) origin_note: OriginNote,
}

pub(crate) const UNKNOWN_ASSIGNMENT_INDEX: UnknownIndexDiagnosticContext =
    UnknownIndexDiagnosticContext {
        constraint_subject: "assignment indices",
        use_subject: "index",
        shape: UnknownIndexShape::Index1D,
        origin_note: OriginNote::Include,
    };

pub(crate) const UNKNOWN_MATRIX_INDEX: UnknownIndexDiagnosticContext =
    UnknownIndexDiagnosticContext {
        constraint_subject: "matrix indices",
        use_subject: "matrix index",
        shape: UnknownIndexShape::Matrix2D,
        origin_note: OriginNote::Omit,
    };

pub(crate) fn validate_unknown_index_type(
    fname: &str,
    fn_ir: &FnIR,
    idx: ValueId,
    context: UnknownIndexDiagnosticContext,
    errors: &mut Vec<RRException>,
) {
    let ity = fn_ir.values[idx].value_ty;
    if !ity.is_unknown() {
        return;
    }
    let message = if matches!(context.shape, UnknownIndexShape::Matrix2D) {
        format!(
            "strict mode unresolved 2D index type in function '{}' (value #{})",
            fname, idx
        )
    } else {
        format!(
            "strict mode unresolved index type in function '{}' (value #{})",
            fname, idx
        )
    };
    let mut builder = DiagnosticBuilder::new("RR.TypeError", RRCode::E1012, Stage::Mir, message)
        .at(fn_ir.values[idx].span);
    if matches!(context.origin_note, OriginNote::Include) {
        builder = builder.origin(
            fn_ir.values[idx].span,
            "index value originates here and has unresolved type facts",
        );
    }
    errors.push(
        builder
            .constraint(
                fn_ir.span,
                format!(
                    "strict mode requires {} to be integer scalars",
                    context.constraint_subject
                ),
            )
            .use_site(
                fn_ir.values[idx].span,
                format!("used here as a {}", context.use_subject),
            )
            .fix("add an explicit integer type hint or cast before indexing")
            .build(),
    );
}

pub(crate) fn validate_field_get_value(
    fname: &str,
    fn_ir: &FnIR,
    v: &Value,
    errors: &mut Vec<RRException>,
) {
    if let ValueKind::FieldGet { base, field } = &v.kind {
        let base_term = &fn_ir.values[*base].value_term;
        if base_term.has_exact_named_fields() && base_term.exact_field_value(field).is_none() {
            push_unknown_field_error(
                fname,
                v.span,
                field.as_str(),
                "used here as a named field access",
                true,
                errors,
            );
        }
    }
}

pub(crate) fn validate_field_set_value(
    fname: &str,
    fn_ir: &FnIR,
    v: &Value,
    errors: &mut Vec<RRException>,
) {
    if let ValueKind::FieldSet { base, field, value } = &v.kind {
        let base_term = &fn_ir.values[*base].value_term;
        if base_term.has_exact_named_fields() {
            validate_field_assignment_term(
                fname,
                fn_ir,
                v.span,
                field.as_str(),
                *value,
                base_term,
                errors,
            );
        }
    }
}

pub(crate) fn validate_field_intrinsic_call(
    fname: &str,
    fn_ir: &FnIR,
    v: &Value,
    errors: &mut Vec<RRException>,
) {
    let ValueKind::Call { callee, args, .. } = &v.kind else {
        return;
    };
    if !matches!(callee.as_str(), "rr_field_get" | "rr_field_set") || args.is_empty() {
        return;
    }
    let base_term = &fn_ir.values[args[0]].value_term;
    let field_name = args.get(1).and_then(|arg| match &fn_ir.values[*arg].kind {
        ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
        _ => None,
    });
    let Some(field_name) = field_name else {
        return;
    };
    if !base_term.has_exact_named_fields() {
        return;
    }
    if callee == "rr_field_set" && args.len() >= 3 {
        validate_field_assignment_term(
            fname, fn_ir, v.span, field_name, args[2], base_term, errors,
        );
    } else if base_term.exact_field_value(field_name).is_none() {
        push_unknown_field_error(
            fname,
            v.span,
            field_name,
            "used here as a named field access",
            true,
            errors,
        );
    }
}

pub(crate) fn validate_field_assignment_term(
    fname: &str,
    fn_ir: &FnIR,
    span: crate::utils::Span,
    field: &str,
    value: ValueId,
    base_term: &TypeTerm,
    errors: &mut Vec<RRException>,
) {
    let Some(expected_field) = base_term.exact_field_value(field) else {
        push_unknown_field_error(
            fname,
            span,
            field,
            "used here as a named field assignment",
            false,
            errors,
        );
        return;
    };
    let got_term = fn_ir.values[value].value_term.clone();
    if expected_field.is_any() || got_term.is_any() || expected_field.compatible_with(&got_term) {
        return;
    }
    errors.push(
        DiagnosticBuilder::new(
            "RR.TypeError",
            RRCode::E1011,
            Stage::Mir,
            format!(
                "dataframe field '{}' expects {:?}, got {:?} in function '{}'",
                field, expected_field, got_term, fname
            ),
        )
        .at(span)
        .origin(
            fn_ir.values[value].span,
            format!("assigned field value is inferred as {:?}", got_term),
        )
        .constraint(
            span,
            format!("field '{}' is constrained to {:?}", field, expected_field),
        )
        .use_site(span, "used here as a dataframe field assignment")
        .fix("cast the assigned value or widen the record/dataframe type hint")
        .build(),
    );
}

pub(crate) fn push_unknown_field_error(
    fname: &str,
    span: crate::utils::Span,
    field: &str,
    use_site: &str,
    include_trait_note: bool,
    errors: &mut Vec<RRException>,
) {
    let mut builder = DiagnosticBuilder::new(
        "RR.TypeError",
        RRCode::E1002,
        Stage::Mir,
        format!(
            "unknown field '{}' for record/dataframe value in function '{}'",
            field, fname
        ),
    )
    .at(span)
    .constraint(
        span,
        format!("field '{}' must exist in the record/dataframe type", field),
    )
    .use_site(span, use_site);
    if include_trait_note {
        builder = builder.note(
            "If this was meant to be a trait method call, add a receiver type hint or matching `where T: Trait` bound so RR can dispatch it statically.",
        );
    }
    errors.push(
        builder
            .fix("change the field name, add the missing field to the type hint, or widen/remove the record/dataframe type hint")
            .build(),
    );
}

pub(crate) fn validate_user_call_signature(
    all_fns: &FxHashMap<String, FnIR>,
    fn_ir: &FnIR,
    v: &Value,
    errors: &mut Vec<RRException>,
) {
    let ValueKind::Call { callee, args, .. } = &v.kind else {
        return;
    };
    let Some(callee_fn) = all_fns.get(callee.as_str()) else {
        return;
    };
    let argc = args.len().min(callee_fn.param_ty_hints.len());
    for (i, arg_id) in args.iter().copied().enumerate().take(argc) {
        let arg = UserCallArgValidation {
            caller: fn_ir,
            call: v,
            callee,
            callee_fn,
            arg: arg_id,
            index: i,
        };
        validate_user_call_arg_term(arg, errors);
        validate_user_call_arg_state(arg, errors);
    }
}

#[derive(Clone, Copy)]
pub(crate) struct UserCallArgValidation<'a> {
    pub(crate) caller: &'a FnIR,
    pub(crate) call: &'a Value,
    pub(crate) callee: &'a str,
    pub(crate) callee_fn: &'a FnIR,
    pub(crate) arg: ValueId,
    pub(crate) index: usize,
}

pub(crate) fn explicit_param_hint(callee_fn: &FnIR, index: usize) -> bool {
    callee_fn
        .param_hint_spans
        .get(index)
        .copied()
        .flatten()
        .is_some()
}

pub(crate) fn validate_user_call_arg_term(
    arg: UserCallArgValidation<'_>,
    errors: &mut Vec<RRException>,
) {
    let explicit_param_hint = explicit_param_hint(arg.callee_fn, arg.index);
    let expected_term = arg
        .callee_fn
        .param_term_hints
        .get(arg.index)
        .cloned()
        .unwrap_or(TypeTerm::Any);
    let got_term = arg.caller.values[arg.arg].value_term.clone();
    if expected_term.is_any()
        || got_term.is_any()
        || call_arg_term_compatible(&expected_term, &got_term, explicit_param_hint)
    {
        return;
    }
    errors.push(call_signature_error(
        arg,
        format!("{:?}", expected_term),
        format!("{:?}", got_term),
    ));
}

pub(crate) fn validate_user_call_arg_state(
    arg: UserCallArgValidation<'_>,
    errors: &mut Vec<RRException>,
) {
    let expected = arg.callee_fn.param_ty_hints[arg.index];
    let got = arg.caller.values[arg.arg].value_ty;
    if is_arg_compatible(expected, got, explicit_param_hint(arg.callee_fn, arg.index)) {
        return;
    }
    errors.push(call_signature_error(
        arg,
        format!("{:?}", expected),
        format!("{:?}", got),
    ));
}

pub(crate) fn call_signature_error(
    arg: UserCallArgValidation<'_>,
    expected: String,
    got: String,
) -> RRException {
    DiagnosticBuilder::new(
        "RR.TypeError",
        RRCode::E1011,
        Stage::Mir,
        format!(
            "call signature type mismatch in '{}': arg {} expects {}, got {}",
            arg.callee,
            arg.index + 1,
            expected,
            got
        ),
    )
    .at(arg.call.span)
    .origin(
        arg.caller.values[arg.arg].span,
        format!(
            "argument {} originates here with inferred type {}",
            arg.index + 1,
            got
        ),
    )
    .constraint(
        arg.callee_fn
            .param_hint_spans
            .get(arg.index)
            .and_then(|s| *s)
            .or_else(|| arg.callee_fn.param_spans.get(arg.index).copied())
            .unwrap_or(arg.callee_fn.span),
        format!("callee parameter {} requires {}", arg.index + 1, expected),
    )
    .use_site(arg.call.span, "call site uses the argument here")
    .fix(format!(
        "cast argument {} or change the callee parameter annotation to a compatible type",
        arg.index + 1
    ))
    .build()
}

pub(crate) fn first_return_origin_span(fn_ir: &FnIR) -> Option<crate::utils::Span> {
    let reachable = compute_reachable(fn_ir);
    for (bid, bb) in fn_ir.blocks.iter().enumerate() {
        if !reachable.get(bid).copied().unwrap_or(false) {
            continue;
        }
        if let Terminator::Return(Some(val)) = bb.term {
            return Some(fn_ir.values[val].span);
        }
    }
    None
}

pub(crate) fn call_arg_term_compatible(
    expected: &TypeTerm,
    got: &TypeTerm,
    explicit_param_hint: bool,
) -> bool {
    if expected.compatible_with(got) {
        return true;
    }
    if !explicit_param_hint
        && matches!(
            (expected, got),
            (
                TypeTerm::Int | TypeTerm::Double,
                TypeTerm::Int | TypeTerm::Double
            )
        )
    {
        return true;
    }
    !explicit_param_hint && inferred_scalar_param_accepts_vector_arg(expected, got)
}

pub(crate) fn inferred_scalar_param_accepts_vector_arg(
    expected: &TypeTerm,
    got: &TypeTerm,
) -> bool {
    let Some(elem) = vector_term_element(got) else {
        return false;
    };
    scalar_term_accepts_vector_element(expected, elem)
}

pub(crate) fn vector_term_element(term: &TypeTerm) -> Option<&TypeTerm> {
    match term {
        TypeTerm::Vector(elem) | TypeTerm::VectorLen(elem, _) => Some(elem.as_ref()),
        _ => None,
    }
}

pub(crate) fn scalar_term_accepts_vector_element(expected: &TypeTerm, elem: &TypeTerm) -> bool {
    if expected.compatible_with(elem) {
        return true;
    }
    if elem.is_any() {
        return matches!(
            expected,
            TypeTerm::Logical | TypeTerm::Int | TypeTerm::Double | TypeTerm::Char
        );
    }
    matches!(
        (expected, elem),
        (
            TypeTerm::Int | TypeTerm::Double,
            TypeTerm::Int | TypeTerm::Double
        )
    )
}
