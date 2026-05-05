use super::*;
pub(crate) fn function_is_effect_free(
    name: &str,
    all_fns: &FxHashMap<String, FnIR>,
    cache: &mut FxHashMap<String, bool>,
) -> bool {
    if effects::call_is_pure(name) {
        return true;
    }
    if let Some(cached) = cache.get(name).copied() {
        return cached;
    }
    let mut visiting = FxHashSet::default();
    let pure = function_is_effect_free_inner(name, all_fns, cache, &mut visiting);
    cache.insert(name.to_string(), pure);
    pure
}

pub(crate) fn function_is_effect_free_inner(
    name: &str,
    all_fns: &FxHashMap<String, FnIR>,
    cache: &mut FxHashMap<String, bool>,
    visiting: &mut FxHashSet<String>,
) -> bool {
    if effects::call_is_pure(name) {
        return true;
    }
    if let Some(cached) = cache.get(name).copied() {
        return cached;
    }
    if !visiting.insert(name.to_string()) {
        return false;
    }
    let Some(fn_ir) = all_fns.get(name) else {
        visiting.remove(name);
        return false;
    };
    if fn_ir.requires_conservative_optimization() {
        visiting.remove(name);
        return false;
    }

    for block in &fn_ir.blocks {
        for instr in &block.instrs {
            match instr {
                Instr::Assign { src, .. } => {
                    if !value_is_effect_free_in_program(*src, fn_ir, all_fns, cache, visiting) {
                        visiting.remove(name);
                        return false;
                    }
                }
                Instr::Eval { .. }
                | Instr::StoreIndex1D { .. }
                | Instr::StoreIndex2D { .. }
                | Instr::StoreIndex3D { .. }
                | Instr::UnsafeRBlock { .. } => {
                    visiting.remove(name);
                    return false;
                }
            }
        }
        match &block.term {
            Terminator::If { cond, .. } | Terminator::Return(Some(cond)) => {
                if !value_is_effect_free_in_program(*cond, fn_ir, all_fns, cache, visiting) {
                    visiting.remove(name);
                    return false;
                }
            }
            Terminator::Goto(_) | Terminator::Return(None) | Terminator::Unreachable => {}
        }
    }

    visiting.remove(name);
    cache.insert(name.to_string(), true);
    true
}

pub(crate) fn value_is_effect_free_in_program(
    value: ValueId,
    fn_ir: &FnIR,
    all_fns: &FxHashMap<String, FnIR>,
    cache: &mut FxHashMap<String, bool>,
    visiting_fns: &mut FxHashSet<String>,
) -> bool {
    fn rec(
        value: ValueId,
        fn_ir: &FnIR,
        all_fns: &FxHashMap<String, FnIR>,
        cache: &mut FxHashMap<String, bool>,
        visiting_fns: &mut FxHashSet<String>,
        visiting_values: &mut FxHashSet<ValueId>,
    ) -> bool {
        if !visiting_values.insert(value) {
            return false;
        }
        let pure = match &fn_ir.values[value].kind {
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => true,
            ValueKind::Call { callee, args, .. } => {
                args.iter()
                    .all(|arg| rec(*arg, fn_ir, all_fns, cache, visiting_fns, visiting_values))
                    && (effects::call_is_pure(callee)
                        || function_is_effect_free_inner(callee, all_fns, cache, visiting_fns))
            }
            _ => value_dependencies(&fn_ir.values[value].kind)
                .into_iter()
                .all(|dep| rec(dep, fn_ir, all_fns, cache, visiting_fns, visiting_values)),
        };
        visiting_values.remove(&value);
        pure
    }

    rec(
        value,
        fn_ir,
        all_fns,
        cache,
        visiting_fns,
        &mut FxHashSet::default(),
    )
}
