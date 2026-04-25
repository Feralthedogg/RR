fn analyze_function_terms(fn_ir: &mut FnIR, fn_ret: &FxHashMap<String, TypeTerm>) -> TypeTerm {
    terms::analyze_function_terms(fn_ir, fn_ret)
}

fn infer_value_term(fn_ir: &FnIR, vid: ValueId, fn_ret: &FxHashMap<String, TypeTerm>) -> TypeTerm {
    terms::infer_value_term(fn_ir, vid, fn_ret)
}

fn named_call_arg(args: &[ValueId], names: &[Option<String>], target: &str) -> Option<ValueId> {
    args.iter()
        .zip(names.iter())
        .find_map(|(arg, name)| (name.as_deref() == Some(target)).then_some(*arg))
}

fn positional_call_arg(
    args: &[ValueId],
    names: &[Option<String>],
    index: usize,
) -> Option<ValueId> {
    match (args.get(index), names.get(index)) {
        (Some(arg), Some(None)) => Some(*arg),
        _ => None,
    }
}

fn infer_named_package_call_type(
    fn_ir: &FnIR,
    callee: &str,
    args: &[ValueId],
    names: &[Option<String>],
) -> Option<TypeState> {
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
                if matches!(
                    callee.as_str(),
                    "stats::update" | "stats::update.default" | "stats::step"
                ) && !args.is_empty() =>
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

    match callee {
        "base::vector" => {
            let mode = args.first().and_then(|arg| match &fn_ir.values[*arg].kind {
                ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                _ => None,
            });
            let ty = match mode {
                Some("logical") => TypeState::vector(PrimTy::Logical, false),
                Some("integer") => TypeState::vector(PrimTy::Int, false),
                Some("double") | Some("numeric") => TypeState::vector(PrimTy::Double, false),
                Some("character") => TypeState::vector(PrimTy::Char, false),
                Some("list") => TypeState::vector(PrimTy::Any, false),
                _ => TypeState::vector(PrimTy::Any, false),
            };
            Some(ty)
        }
        "stats::predict" => {
            let newdata = named_call_arg(args, names, "newdata")
                .or_else(|| positional_call_arg(args, names, 1));
            let len_sym = newdata.and_then(|arg| fn_ir.values[arg].value_ty.len_sym);
            Some(TypeState::vector(PrimTy::Double, false).with_len(len_sym))
        }
        "stats::model.frame" | "stats::model.frame.default" => {
            if let Some(data) =
                named_call_arg(args, names, "data").or_else(|| positional_call_arg(args, names, 1))
            {
                return Some(fn_ir.values[data].value_ty);
            }
            if let Some(model_arg) = positional_call_arg(args, names, 0)
                && let Some(origin) = resolve_package_call_origin(fn_ir, model_arg)
                && let ValueKind::Call {
                    callee,
                    args,
                    names,
                } = &fn_ir.values[origin].kind
                && matches!(callee.as_str(), "stats::lm" | "stats::glm")
                && let Some(data_arg) = named_call_arg(args, names, "data")
                    .or_else(|| positional_call_arg(args, names, 1))
            {
                return Some(fn_ir.values[data_arg].value_ty);
            }
            Some(TypeState::matrix(PrimTy::Any, false))
        }
        "stats::step" | "stats::update.default" => {
            positional_call_arg(args, names, 0).map(|arg| fn_ir.values[arg].value_ty)
        }
        _ => None,
    }
}
