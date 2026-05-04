use super::*;
pub(crate) fn infer_value_term(
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
                    logical_binary_term(&l, &r)
                }
                BinOp::And | BinOp::Or => logical_binary_term(&l, &r),
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
            if callee == "rr_field_exists" || callee == "rr_list_pattern_matchable" {
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
        ValueKind::RecordLit { fields } => TypeTerm::NamedList(
            fields
                .iter()
                .map(|(name, value)| (name.clone(), fn_ir.values[*value].value_term.clone()))
                .collect(),
        ),
        ValueKind::FieldGet { base, field } => fn_ir.values[*base]
            .value_term
            .field_value_named(Some(field.as_str())),
        ValueKind::FieldSet { base, field, value } => fn_ir.values[*base]
            .value_term
            .updated_field_value_named(field, &fn_ir.values[*value].value_term),
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
