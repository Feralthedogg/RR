use super::*;
pub(crate) fn infer_value_type(
    fn_ir: &FnIR,
    vid: ValueId,
    fn_ret: &FxHashMap<String, TypeState>,
    var_tys: &FxHashMap<String, TypeState>,
) -> TypeState {
    let val = &fn_ir.values[vid];
    match &val.kind {
        ValueKind::Const(l) => lit_type(l),
        ValueKind::Param { index } => fn_ir
            .param_ty_hints
            .get(*index)
            .copied()
            .map(|ty| refine_type_with_term(ty, &fn_ir.param_term_hints[*index]))
            .unwrap_or(TypeState::unknown()),
        ValueKind::Len { .. } => TypeState::scalar(PrimTy::Int, true),
        ValueKind::Indices { base } => {
            let base_ty = fn_ir.values[*base].value_ty;
            TypeState::vector(PrimTy::Int, true).with_len(base_ty.len_sym)
        }
        ValueKind::Range { .. } => TypeState::vector(PrimTy::Int, true),
        ValueKind::Unary { rhs, .. } => {
            let r = fn_ir.values[*rhs].value_ty;
            TypeState {
                prim: if matches!(r.prim, PrimTy::Int | PrimTy::Double) {
                    r.prim
                } else {
                    PrimTy::Any
                },
                shape: r.shape,
                na: r.na,
                len_sym: r.len_sym,
            }
        }
        ValueKind::Binary { op, lhs, rhs } => {
            let l = fn_ir.values[*lhs].value_ty;
            let r = fn_ir.values[*rhs].value_ty;
            use crate::syntax::ast::BinOp;
            match op {
                BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    TypeState {
                        prim: PrimTy::Logical,
                        shape: normalize_call_numeric_shape(&[l, r]),
                        na: if l.na == NaTy::Never && r.na == NaTy::Never {
                            NaTy::Never
                        } else {
                            NaTy::Maybe
                        },
                        len_sym: if l.len_sym.is_some() && l.len_sym == r.len_sym {
                            l.len_sym
                        } else {
                            None
                        },
                    }
                }
                BinOp::And | BinOp::Or => TypeState {
                    prim: PrimTy::Logical,
                    shape: normalize_call_numeric_shape(&[l, r]),
                    na: if l.na == NaTy::Never && r.na == NaTy::Never {
                        NaTy::Never
                    } else {
                        NaTy::Maybe
                    },
                    len_sym: if l.len_sym.is_some() && l.len_sym == r.len_sym {
                        l.len_sym
                    } else {
                        None
                    },
                },
                BinOp::MatMul => TypeState {
                    prim: match promoted_numeric_prim(l.prim, r.prim) {
                        PrimTy::Int | PrimTy::Double => PrimTy::Double,
                        other => other,
                    },
                    shape: if matches!(l.shape, ShapeTy::Vector | ShapeTy::Matrix)
                        && matches!(r.shape, ShapeTy::Vector | ShapeTy::Matrix)
                    {
                        ShapeTy::Matrix
                    } else {
                        ShapeTy::Unknown
                    },
                    na: if l.na == NaTy::Never && r.na == NaTy::Never {
                        NaTy::Never
                    } else {
                        NaTy::Maybe
                    },
                    len_sym: None,
                },
                _ => {
                    let prim = match op {
                        BinOp::Div => match (l.prim, r.prim) {
                            (PrimTy::Int, PrimTy::Int)
                            | (PrimTy::Int, PrimTy::Double)
                            | (PrimTy::Double, PrimTy::Int)
                            | (PrimTy::Double, PrimTy::Double) => PrimTy::Double,
                            (PrimTy::Any, other) | (other, PrimTy::Any) => other,
                            _ => PrimTy::Any,
                        },
                        BinOp::Mod => promoted_numeric_prim(l.prim, r.prim),
                        _ => promoted_numeric_prim(l.prim, r.prim),
                    };
                    TypeState {
                        prim,
                        shape: normalize_call_numeric_shape(&[l, r]),
                        na: if l.na == NaTy::Never && r.na == NaTy::Never {
                            NaTy::Never
                        } else {
                            NaTy::Maybe
                        },
                        len_sym: if l.len_sym.is_some() && l.len_sym == r.len_sym {
                            l.len_sym
                        } else {
                            None
                        },
                    }
                }
            }
        }
        ValueKind::Phi { args } => {
            let mut out = TypeState::unknown();
            for (a, _) in args {
                out = out.join(fn_ir.values[*a].value_ty);
            }
            out
        }
        ValueKind::RecordLit { fields } => type_state_from_term(&TypeTerm::NamedList(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), fn_ir.values[*value].value_term.clone()))
                .collect(),
        )),
        ValueKind::FieldGet { base, field } => type_state_from_term(
            &fn_ir.values[*base]
                .value_term
                .field_value_named(Some(field.as_str())),
        ),
        ValueKind::FieldSet { base, field, value } => {
            let updated = fn_ir.values[*base]
                .value_term
                .updated_field_value_named(field, &fn_ir.values[*value].value_term);
            refine_type_with_term(fn_ir.values[*base].value_ty, &updated)
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            if callee == "seq_along" && args.len() == 1 {
                let base_ty = fn_ir.values[args[0]].value_ty;
                return TypeState::vector(PrimTy::Int, true).with_len(base_ty.len_sym);
            }
            if callee == "seq_len" && args.len() == 1 {
                let len_sym = match &fn_ir.values[args[0]].kind {
                    ValueKind::Len { base } => fn_ir.values[*base].value_ty.len_sym,
                    _ => None,
                };
                return TypeState::vector(PrimTy::Int, true).with_len(len_sym);
            }
            if callee == "rr_field_get" && !args.is_empty() {
                let field_name = args.get(1).and_then(|arg| match &fn_ir.values[*arg].kind {
                    ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                    _ => None,
                });
                return type_state_from_term(
                    &fn_ir.values[args[0]]
                        .value_term
                        .field_value_named(field_name),
                );
            }
            if callee == "rr_field_exists" || callee == "rr_list_pattern_matchable" {
                return TypeState::scalar(PrimTy::Logical, true);
            }
            if callee == "rr_field_set" && !args.is_empty() {
                let field_name = args.get(1).and_then(|arg| match &fn_ir.values[*arg].kind {
                    ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                    _ => None,
                });
                if let (Some(name), Some(value)) = (field_name, args.get(2)) {
                    let updated = fn_ir.values[args[0]]
                        .value_term
                        .updated_field_value_named(name, &fn_ir.values[*value].value_term);
                    return refine_type_with_term(fn_ir.values[args[0]].value_ty, &updated);
                }
                return fn_ir.values[args[0]].value_ty;
            }
            let arg_tys: Vec<TypeState> = args.iter().map(|a| fn_ir.values[*a].value_ty).collect();
            let arg_terms: Vec<TypeTerm> = args
                .iter()
                .map(|a| fn_ir.values[*a].value_term.clone())
                .collect();
            let signature_indices = signature_value_arg_indices(callee, args, names);
            let signature_arg_tys: Vec<TypeState> =
                signature_indices.iter().map(|idx| arg_tys[*idx]).collect();
            let signature_arg_terms: Vec<TypeTerm> = signature_indices
                .iter()
                .map(|idx| arg_terms[*idx].clone())
                .collect();
            if callee == "compiler::getCompilerOption"
                && let Some(option_name) =
                    args.first().and_then(|arg| match &fn_ir.values[*arg].kind {
                        ValueKind::Const(crate::syntax::ast::Lit::Str(name)) => Some(name.as_str()),
                        _ => None,
                    })
            {
                let narrowed = match option_name {
                    "optimize" => TypeState::scalar(PrimTy::Int, false),
                    "suppressAll" => TypeState::scalar(PrimTy::Logical, false),
                    "suppressUndefined" => TypeState::vector(PrimTy::Char, false),
                    "suppressNoSuperAssignVar" => TypeState::scalar(PrimTy::Logical, false),
                    _ => TypeState::scalar(PrimTy::Any, false),
                };
                return refine_type_with_term(narrowed, &fn_ir.values[vid].value_term);
            }
            if let Some(b) = infer_builtin(callee, &signature_arg_tys) {
                let with_builtin_term =
                    if let Some(term) = infer_builtin_term(callee, &signature_arg_terms) {
                        refine_type_with_term(b, &term)
                    } else {
                        b
                    };
                let with_builtin_term =
                    refine_named_builtin_na(fn_ir, callee, args, names, with_builtin_term);
                return refine_type_with_term(with_builtin_term, &fn_ir.values[vid].value_term);
            }
            if let Some(pkg_ty) = infer_named_package_call_type(fn_ir, callee, args, names) {
                let with_pkg_term = if let Some(term) = infer_package_call_term(callee, &arg_terms)
                {
                    refine_type_with_term(pkg_ty, &term)
                } else {
                    pkg_ty
                };
                let with_pkg_term =
                    refine_named_builtin_na(fn_ir, callee, args, names, with_pkg_term);
                return refine_type_with_term(with_pkg_term, &fn_ir.values[vid].value_term);
            }
            if let Some(pkg_ty) = infer_package_call(callee, &signature_arg_tys) {
                let with_pkg_term =
                    if let Some(term) = infer_package_call_term(callee, &signature_arg_terms) {
                        refine_type_with_term(pkg_ty, &term)
                    } else {
                        pkg_ty
                    };
                let with_pkg_term =
                    refine_named_builtin_na(fn_ir, callee, args, names, with_pkg_term);
                return refine_type_with_term(with_pkg_term, &fn_ir.values[vid].value_term);
            }
            if callee.starts_with("Sym_") {
                return fn_ret.get(callee).copied().unwrap_or(TypeState::unknown());
            }
            TypeState::unknown()
        }
        ValueKind::Index1D { base, .. } => {
            let b = fn_ir.values[*base].value_ty;
            refine_type_with_term(
                TypeState {
                    prim: b.prim,
                    shape: ShapeTy::Scalar,
                    na: NaTy::Maybe,
                    len_sym: None,
                },
                &fn_ir.values[*base].value_term.index_element(),
            )
        }
        ValueKind::Index2D { base, .. } => {
            let b = fn_ir.values[*base].value_ty;
            refine_type_with_term(
                TypeState {
                    prim: b.prim,
                    shape: ShapeTy::Scalar,
                    na: NaTy::Maybe,
                    len_sym: None,
                },
                &fn_ir.values[*base].value_term.index_element(),
            )
        }
        ValueKind::Index3D { base, .. } => {
            let b = fn_ir.values[*base].value_ty;
            refine_type_with_term(
                TypeState {
                    prim: b.prim,
                    shape: ShapeTy::Scalar,
                    na: NaTy::Maybe,
                    len_sym: None,
                },
                &fn_ir.values[*base].value_term.index_element(),
            )
        }
        ValueKind::Load { var } => {
            let ty = var_tys
                .get(var)
                .copied()
                .or_else(|| infer_package_binding(var))
                .unwrap_or(TypeState::unknown());
            refine_type_with_term(
                ty,
                fn_ir
                    .values
                    .get(vid)
                    .map(|v| &v.value_term)
                    .unwrap_or(&TypeTerm::Any),
            )
        }
        ValueKind::RSymbol { .. } => TypeState::unknown(),
        ValueKind::Intrinsic { op, args } => {
            use crate::mir::IntrinsicOp;
            match op {
                IntrinsicOp::VecSumF64 | IntrinsicOp::VecMeanF64 => {
                    TypeState::scalar(PrimTy::Double, false)
                }
                IntrinsicOp::VecAbsF64 => {
                    let prim = args
                        .first()
                        .map(|arg| fn_ir.values[*arg].value_ty.prim)
                        .unwrap_or(PrimTy::Double);
                    TypeState::vector(
                        if matches!(prim, PrimTy::Int | PrimTy::Double) {
                            prim
                        } else {
                            PrimTy::Double
                        },
                        false,
                    )
                }
                IntrinsicOp::VecPmaxF64 | IntrinsicOp::VecPminF64 => {
                    let prim = args
                        .iter()
                        .map(|arg| fn_ir.values[*arg].value_ty.prim)
                        .fold(PrimTy::Any, promoted_numeric_prim);
                    TypeState::vector(
                        if matches!(prim, PrimTy::Int | PrimTy::Double) {
                            prim
                        } else {
                            PrimTy::Double
                        },
                        false,
                    )
                }
                _ => {
                    let mut out = TypeState::vector(PrimTy::Double, false);
                    if args.is_empty() {
                        out.shape = ShapeTy::Unknown;
                    }
                    out
                }
            }
        }
    }
}

pub(crate) fn refine_named_builtin_na(
    fn_ir: &FnIR,
    callee: &str,
    args: &[ValueId],
    names: &[Option<String>],
    mut ty: TypeState,
) -> TypeState {
    let builtin = callee.strip_prefix("base::").unwrap_or(callee);
    let value_args: Vec<ValueId> = args
        .iter()
        .copied()
        .enumerate()
        .filter_map(
            |(idx, arg)| match names.get(idx).and_then(Option::as_deref) {
                Some("na.rm") => None,
                _ => Some(arg),
            },
        )
        .collect();
    let all_values_non_na = !value_args.is_empty()
        && value_args
            .iter()
            .all(|arg| fn_ir.values[*arg].value_ty.na == NaTy::Never);

    match builtin {
        "sum" | "prod" | "min" | "max"
            if (named_bool_arg(fn_ir, args, names, "na.rm") == Some(true) || all_values_non_na) =>
        {
            ty.na = NaTy::Never;
        }
        "mean" => {
            let all_scalar_non_na = !value_args.is_empty()
                && value_args.iter().all(|arg| {
                    let value_ty = fn_ir.values[*arg].value_ty;
                    value_ty.shape == ShapeTy::Scalar && value_ty.na == NaTy::Never
                });
            if all_scalar_non_na {
                ty.na = NaTy::Never;
            }
        }
        _ => {}
    }

    ty
}

pub(crate) fn signature_value_arg_indices(
    callee: &str,
    args: &[ValueId],
    names: &[Option<String>],
) -> Vec<usize> {
    let builtin = callee.strip_prefix("base::").unwrap_or(callee);
    let drop_named_na_rm = matches!(builtin, "sum" | "prod" | "min" | "max" | "mean");
    args.iter()
        .enumerate()
        .filter_map(|(idx, _)| {
            if drop_named_na_rm && names.get(idx).and_then(Option::as_deref) == Some("na.rm") {
                None
            } else {
                Some(idx)
            }
        })
        .collect()
}

pub(crate) fn named_bool_arg(
    fn_ir: &FnIR,
    args: &[ValueId],
    names: &[Option<String>],
    target: &str,
) -> Option<bool> {
    args.iter()
        .enumerate()
        .find_map(|(idx, arg)| {
            (names.get(idx).and_then(Option::as_deref) == Some(target))
                .then(|| const_bool(fn_ir, *arg))
        })
        .flatten()
}

pub(crate) fn const_bool(fn_ir: &FnIR, value: ValueId) -> Option<bool> {
    match &fn_ir.values[value].kind {
        ValueKind::Const(crate::syntax::ast::Lit::Bool(value)) => Some(*value),
        _ => None,
    }
}
