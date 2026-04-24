fn finish_type_analysis(all_fns: &FxHashMap<String, FnIR>, cfg: TypeConfig) -> RR<()> {
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

fn validate_strict(all_fns: &FxHashMap<String, FnIR>) -> Vec<RRException> {
    let mut names: Vec<String> = all_fns.keys().cloned().collect();
    names.sort();
    let mut errors = Vec::new();
    for fname in names {
        let Some(fn_ir) = all_fns.get(&fname) else {
            continue;
        };
        let reachable = compute_reachable(fn_ir);
        let has_explicit_hints = fn_ir.ret_ty_hint.is_some()
            || fn_ir.ret_term_hint.is_some()
            || fn_ir.param_ty_hints.iter().any(|t| !t.is_unknown())
            || fn_ir.param_term_hints.iter().any(|t| !t.is_any());
        for (bid, bb) in fn_ir.blocks.iter().enumerate() {
            if !reachable.get(bid).copied().unwrap_or(false) {
                continue;
            }
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
                        .use_site(fn_ir.values[cond].span, "used here as an if/while condition")
                        .fix("add an explicit logical type hint or cast before the condition")
                        .build(),
                    );
                }
            }
            for ins in &bb.instrs {
                if let crate::mir::Instr::StoreIndex1D { idx, .. } = ins {
                    let ity = fn_ir.values[*idx].value_ty;
                    if has_explicit_hints && ity.is_unknown() {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1012,
                                Stage::Mir,
                                format!(
                                    "strict mode unresolved index type in function '{}' (value #{})",
                                    fname, idx
                                ),
                            )
                            .at(fn_ir.values[*idx].span)
                            .origin(
                                fn_ir.values[*idx].span,
                                "index value originates here and has unresolved type facts",
                            )
                            .constraint(
                                fn_ir.span,
                                "strict mode requires assignment indices to be integer scalars",
                            )
                            .use_site(fn_ir.values[*idx].span, "used here as an index")
                            .fix("add an explicit integer type hint or cast before indexing")
                            .build(),
                        );
                    }
                }
                if let crate::mir::Instr::StoreIndex2D { base, r, c, .. } = ins
                    && has_explicit_hints
                {
                    let base_ty = fn_ir.values[*base].value_ty;
                    if base_ty.shape != ShapeTy::Unknown && base_ty.shape != ShapeTy::Matrix {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1002,
                                Stage::Mir,
                                format!(
                                    "strict mode 2D assignment requires matrix-typed base in function '{}' (got {:?})",
                                    fname, base_ty
                                ),
                            )
                            .at(fn_ir.values[*base].span)
                            .constraint(
                                fn_ir.values[*base].span,
                                "2D assignment requires a matrix-typed base",
                            )
                            .use_site(fn_ir.values[*base].span, "used here as a 2D assignment base")
                            .fix("change the base type hint to matrix<T>, or use 1D indexing")
                            .build(),
                        );
                    }
                    for idx in [r, c] {
                        let ity = fn_ir.values[*idx].value_ty;
                        if ity.is_unknown() {
                            errors.push(
                                DiagnosticBuilder::new(
                                    "RR.TypeError",
                                    RRCode::E1012,
                                    Stage::Mir,
                                    format!(
                                        "strict mode unresolved 2D index type in function '{}' (value #{})",
                                        fname, idx
                                    ),
                                )
                                .at(fn_ir.values[*idx].span)
                                .constraint(
                                    fn_ir.span,
                                    "strict mode requires matrix indices to be integer scalars",
                                )
                                .use_site(fn_ir.values[*idx].span, "used here as a matrix index")
                                .fix("add an explicit integer type hint or cast before indexing")
                                .build(),
                            );
                        }
                    }
                }
            }
        }

        for v in &fn_ir.values {
            if has_explicit_hints && let ValueKind::Index2D { base, r, c } = &v.kind {
                let base_ty = fn_ir.values[*base].value_ty;
                if base_ty.shape != ShapeTy::Unknown && base_ty.shape != ShapeTy::Matrix {
                    errors.push(
                        DiagnosticBuilder::new(
                            "RR.TypeError",
                            RRCode::E1002,
                            Stage::Mir,
                            format!(
                                "strict mode 2D indexing requires matrix-typed base in function '{}' (got {:?})",
                                fname, base_ty
                            ),
                        )
                        .at(v.span)
                        .constraint(v.span, "2D indexing requires a matrix-typed base")
                        .use_site(v.span, "used here as a 2D indexing expression")
                        .fix("change the base type hint to matrix<T>, or use 1D indexing")
                        .build(),
                    );
                }
                for idx in [r, c] {
                    let ity = fn_ir.values[*idx].value_ty;
                    if ity.is_unknown() {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1012,
                                Stage::Mir,
                                format!(
                                    "strict mode unresolved 2D index type in function '{}' (value #{})",
                                    fname, idx
                                ),
                            )
                            .at(fn_ir.values[*idx].span)
                            .constraint(
                                fn_ir.span,
                                "strict mode requires matrix indices to be integer scalars",
                            )
                            .use_site(fn_ir.values[*idx].span, "used here as a matrix index")
                            .fix("add an explicit integer type hint or cast before indexing")
                            .build(),
                        );
                    }
                }
            }
            if has_explicit_hints && let ValueKind::FieldGet { base, field } = &v.kind {
                let base_term = &fn_ir.values[*base].value_term;
                if base_term.has_exact_named_fields()
                    && base_term.exact_field_value(field).is_none()
                {
                    errors.push(
                        DiagnosticBuilder::new(
                            "RR.TypeError",
                            RRCode::E1002,
                            Stage::Mir,
                            format!(
                                "strict mode field '{}' is not present in the visible dataframe schema for function '{}'",
                                field, fname
                            ),
                        )
                        .at(v.span)
                        .constraint(
                            v.span,
                            format!("field '{}' must exist in the dataframe schema", field),
                        )
                        .use_site(v.span, "used here as a named dataframe field access")
                        .fix("change the field name or widen/remove the dataframe schema hint")
                        .build(),
                    );
                }
            }
            if has_explicit_hints && let ValueKind::FieldSet { base, field, value } = &v.kind {
                let base_term = &fn_ir.values[*base].value_term;
                if base_term.has_exact_named_fields() {
                    let expected_field = base_term.exact_field_value(field);
                    if expected_field.is_none() {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1002,
                                Stage::Mir,
                                format!(
                                    "strict mode field '{}' is not present in the visible dataframe schema for function '{}'",
                                    field, fname
                                ),
                            )
                            .at(v.span)
                            .constraint(
                                v.span,
                                format!("field '{}' must exist in the dataframe schema", field),
                            )
                            .use_site(v.span, "used here as a named dataframe field assignment")
                            .fix("change the field name or widen/remove the dataframe schema hint")
                            .build(),
                        );
                    } else {
                        let expected_field = expected_field.unwrap_or(TypeTerm::Any);
                        let got_term = fn_ir.values[*value].value_term.clone();
                        if !expected_field.is_any()
                            && !got_term.is_any()
                            && !expected_field.compatible_with(&got_term)
                        {
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
                                .at(v.span)
                                .origin(
                                    fn_ir.values[*value].span,
                                    format!("assigned field value is inferred as {:?}", got_term),
                                )
                                .constraint(
                                    v.span,
                                    format!("field '{}' is constrained to {:?}", field, expected_field),
                                )
                                .use_site(v.span, "used here as a dataframe field assignment")
                                .fix("cast the assigned value or widen the dataframe schema hint")
                                .build(),
                            );
                        }
                    }
                }
            }
            if has_explicit_hints
                && let ValueKind::Call { callee, args, .. } = &v.kind
                && matches!(callee.as_str(), "rr_field_get" | "rr_field_set")
                && !args.is_empty()
            {
                let base_term = &fn_ir.values[args[0]].value_term;
                let field_name = args.get(1).and_then(|arg| match &fn_ir.values[*arg].kind {
                    ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                    _ => None,
                });
                if let Some(field_name) = field_name
                    && base_term.has_exact_named_fields()
                {
                    let expected_field = base_term.exact_field_value(field_name);
                    if expected_field.is_none() {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1002,
                                Stage::Mir,
                                format!(
                                    "strict mode field '{}' is not present in the visible dataframe schema for function '{}'",
                                    field_name, fname
                                ),
                            )
                            .at(v.span)
                            .constraint(
                                v.span,
                                format!("field '{}' must exist in the dataframe schema", field_name),
                            )
                            .use_site(v.span, "used here as a named dataframe field access")
                            .fix("change the field name or widen/remove the dataframe schema hint")
                            .build(),
                        );
                    } else if callee == "rr_field_set" && args.len() >= 3 {
                        let expected_field = expected_field.unwrap_or(TypeTerm::Any);
                        let got_term = fn_ir.values[args[2]].value_term.clone();
                        if !expected_field.is_any()
                            && !got_term.is_any()
                            && !expected_field.compatible_with(&got_term)
                        {
                            errors.push(
                                DiagnosticBuilder::new(
                                    "RR.TypeError",
                                    RRCode::E1011,
                                    Stage::Mir,
                                    format!(
                                        "dataframe field '{}' expects {:?}, got {:?} in function '{}'",
                                        field_name, expected_field, got_term, fname
                                    ),
                                )
                                .at(v.span)
                                .origin(
                                    fn_ir.values[args[2]].span,
                                    format!("assigned field value is inferred as {:?}", got_term),
                                )
                                .constraint(
                                    v.span,
                                    format!("field '{}' is constrained to {:?}", field_name, expected_field),
                                )
                                .use_site(v.span, "used here as a dataframe field assignment")
                                .fix("cast the assigned value or widen the dataframe schema hint")
                                .build(),
                            );
                        }
                    }
                }
            }
            if let ValueKind::Call { callee, args, .. } = &v.kind
                && let Some(callee_fn) = all_fns.get(callee)
            {
                let argc = args.len().min(callee_fn.param_ty_hints.len());
                for i in 0..argc {
                    let expected_term = callee_fn
                        .param_term_hints
                        .get(i)
                        .cloned()
                        .unwrap_or(TypeTerm::Any);
                    let got_term = fn_ir.values[args[i]].value_term.clone();
                    if !expected_term.is_any()
                        && !got_term.is_any()
                        && !expected_term.compatible_with(&got_term)
                    {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1011,
                                Stage::Mir,
                                format!(
                                    "call signature type mismatch in '{}': arg {} expects {:?}, got {:?}",
                                    callee,
                                    i + 1,
                                    expected_term,
                                    got_term
                                ),
                            )
                            .at(v.span)
                            .origin(
                                fn_ir.values[args[i]].span,
                                format!("argument {} originates here with inferred type {:?}", i + 1, got_term),
                            )
                            .constraint(
                                callee_fn
                                    .param_hint_spans
                                    .get(i)
                                    .and_then(|s| *s)
                                    .or_else(|| callee_fn.param_spans.get(i).copied())
                                    .unwrap_or(callee_fn.span),
                                format!("callee parameter {} requires {:?}", i + 1, expected_term),
                            )
                            .use_site(v.span, "call site uses the argument here")
                            .fix(format!(
                                "cast argument {} or change the callee parameter annotation to a compatible type",
                                i + 1
                            ))
                            .build(),
                        );
                    }

                    let expected = callee_fn.param_ty_hints[i];
                    let got = fn_ir.values[args[i]].value_ty;
                    if !is_arg_compatible(expected, got) {
                        errors.push(
                            DiagnosticBuilder::new(
                                "RR.TypeError",
                                RRCode::E1011,
                                Stage::Mir,
                                format!(
                                    "call signature type mismatch in '{}': arg {} expects {:?}, got {:?}",
                                    callee,
                                    i + 1,
                                    expected,
                                    got
                                ),
                            )
                            .at(v.span)
                            .origin(
                                fn_ir.values[args[i]].span,
                                format!("argument {} originates here with inferred type {:?}", i + 1, got),
                            )
                            .constraint(
                                callee_fn
                                    .param_hint_spans
                                    .get(i)
                                    .and_then(|s| *s)
                                    .or_else(|| callee_fn.param_spans.get(i).copied())
                                    .unwrap_or(callee_fn.span),
                                format!("callee parameter {} requires {:?}", i + 1, expected),
                            )
                            .use_site(v.span, "call site uses the argument here")
                            .fix(format!(
                                "cast argument {} or change the callee parameter annotation to a compatible type",
                                i + 1
                            ))
                            .build(),
                        );
                    }
                }
            }
        }
    }
    errors
}

fn first_return_origin_span(fn_ir: &FnIR) -> Option<crate::utils::Span> {
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
