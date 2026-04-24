fn clone_value_for_generic(
    fn_ir: &mut FnIR,
    root: ValueId,
    loop_var_map: &FxHashMap<String, String>,
    memo: &mut FxHashMap<ValueId, ValueId>,
) -> Option<ValueId> {
    if let Some(mapped) = memo.get(&root) {
        return Some(*mapped);
    }
    let value = match fn_ir.values[root].kind.clone() {
        ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. } => root,
        ValueKind::Load { var } => {
            build_load(fn_ir, loop_var_map.get(&var).cloned().unwrap_or(var))
        }
        ValueKind::Phi { .. } => {
            let var = fn_ir.values[root].origin_var.clone()?;
            build_load(fn_ir, loop_var_map.get(&var).cloned().unwrap_or(var))
        }
        ValueKind::Len { base } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Len { base },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Indices { base } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Indices { base },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Range { start, end } => {
            let start = clone_value_for_generic(fn_ir, start, loop_var_map, memo)?;
            let end = clone_value_for_generic(fn_ir, end, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Range { start, end },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Binary { op, lhs, rhs } => {
            let lhs = clone_value_for_generic(fn_ir, lhs, loop_var_map, memo)?;
            let rhs = clone_value_for_generic(fn_ir, rhs, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Binary { op, lhs, rhs },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Unary { op, rhs } => {
            let rhs = clone_value_for_generic(fn_ir, rhs, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Unary { op, rhs },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Call {
            callee,
            args,
            names,
        } => {
            let args = args
                .iter()
                .map(|arg| clone_value_for_generic(fn_ir, *arg, loop_var_map, memo))
                .collect::<Option<Vec<_>>>()?;
            fn_ir.add_value(
                ValueKind::Call {
                    callee,
                    args,
                    names,
                },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Intrinsic { op, args } => {
            let args = args
                .iter()
                .map(|arg| clone_value_for_generic(fn_ir, *arg, loop_var_map, memo))
                .collect::<Option<Vec<_>>>()?;
            fn_ir.add_value(
                ValueKind::Intrinsic { op, args },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::RecordLit { fields } => {
            let fields = fields
                .iter()
                .map(|(name, value)| {
                    Some((
                        name.clone(),
                        clone_value_for_generic(fn_ir, *value, loop_var_map, memo)?,
                    ))
                })
                .collect::<Option<Vec<_>>>()?;
            fn_ir.add_value(
                ValueKind::RecordLit { fields },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::FieldGet { base, field } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::FieldGet { base, field },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::FieldSet { base, field, value } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            let value = clone_value_for_generic(fn_ir, value, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::FieldSet { base, field, value },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Index1D {
            base,
            idx,
            is_safe,
            is_na_safe,
        } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            let idx = clone_value_for_generic(fn_ir, idx, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Index1D {
                    base,
                    idx,
                    is_safe,
                    is_na_safe,
                },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Index2D { base, r, c } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            let r = clone_value_for_generic(fn_ir, r, loop_var_map, memo)?;
            let c = clone_value_for_generic(fn_ir, c, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Index2D { base, r, c },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
        ValueKind::Index3D { base, i, j, k } => {
            let base = clone_value_for_generic(fn_ir, base, loop_var_map, memo)?;
            let i = clone_value_for_generic(fn_ir, i, loop_var_map, memo)?;
            let j = clone_value_for_generic(fn_ir, j, loop_var_map, memo)?;
            let k = clone_value_for_generic(fn_ir, k, loop_var_map, memo)?;
            fn_ir.add_value(
                ValueKind::Index3D { base, i, j, k },
                fn_ir.values[root].span,
                fn_ir.values[root].facts,
                fn_ir.values[root].origin_var.clone(),
            )
        }
    };
    memo.insert(root, value);
    Some(value)
}
