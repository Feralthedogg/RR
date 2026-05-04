use super::*;

pub(crate) fn collect_referentially_pure_user_functions(program: &ProgramIR) -> FxHashSet<String> {
    fn helper_is_functionally_pure(callee: &str) -> bool {
        matches!(
            callee,
            "rr_assign_slice"
                | "rr_ifelse_strict"
                | "rr_index1_read"
                | "rr_index1_read_strict"
                | "rr_index1_read_vec"
                | "rr_index1_read_vec_floor"
                | "rr_index_vec_floor"
                | "rr_gather"
                | "rr_wrap_index_vec"
                | "rr_wrap_index_vec_i"
                | "rr_idx_cube_vec_i"
                | "rr_named_list"
                | "rr_field_get"
                | "rr_field_exists"
                | "rr_list_pattern_matchable"
        )
    }

    fn value_is_functionally_pure(
        program: &ProgramIR,
        fn_ir: &crate::mir::def::FnIR,
        vid: crate::mir::def::ValueId,
        memo: &mut FxHashMap<String, bool>,
        visiting_fns: &mut FxHashSet<String>,
        seen: &mut FxHashSet<crate::mir::def::ValueId>,
    ) -> bool {
        if !seen.insert(vid) {
            return false;
        }
        let pure = match &fn_ir.values[vid].kind {
            crate::mir::def::ValueKind::Const(_)
            | crate::mir::def::ValueKind::Param { .. }
            | crate::mir::def::ValueKind::Load { .. }
            | crate::mir::def::ValueKind::RSymbol { .. } => true,
            crate::mir::def::ValueKind::Phi { args } => args.iter().all(|(src, _)| {
                value_is_functionally_pure(program, fn_ir, *src, memo, visiting_fns, seen)
            }),
            crate::mir::def::ValueKind::Len { base }
            | crate::mir::def::ValueKind::Indices { base } => {
                value_is_functionally_pure(program, fn_ir, *base, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Range { start, end } => {
                value_is_functionally_pure(program, fn_ir, *start, memo, visiting_fns, seen)
                    && value_is_functionally_pure(program, fn_ir, *end, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Binary { lhs, rhs, .. } => {
                value_is_functionally_pure(program, fn_ir, *lhs, memo, visiting_fns, seen)
                    && value_is_functionally_pure(program, fn_ir, *rhs, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Unary { rhs, .. } => {
                value_is_functionally_pure(program, fn_ir, *rhs, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Index1D { base, idx, .. } => {
                value_is_functionally_pure(program, fn_ir, *base, memo, visiting_fns, seen)
                    && value_is_functionally_pure(program, fn_ir, *idx, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Index2D { base, r, c } => {
                value_is_functionally_pure(program, fn_ir, *base, memo, visiting_fns, seen)
                    && value_is_functionally_pure(program, fn_ir, *r, memo, visiting_fns, seen)
                    && value_is_functionally_pure(program, fn_ir, *c, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Index3D { base, i, j, k } => {
                value_is_functionally_pure(program, fn_ir, *base, memo, visiting_fns, seen)
                    && value_is_functionally_pure(program, fn_ir, *i, memo, visiting_fns, seen)
                    && value_is_functionally_pure(program, fn_ir, *j, memo, visiting_fns, seen)
                    && value_is_functionally_pure(program, fn_ir, *k, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::Intrinsic { args, .. } => args.iter().all(|arg| {
                value_is_functionally_pure(program, fn_ir, *arg, memo, visiting_fns, seen)
            }),
            crate::mir::def::ValueKind::Call { callee, args, .. } => {
                let user_pure = program.get(callee).is_some_and(|callee_ir| {
                    function_is_referentially_pure(program, callee, callee_ir, memo, visiting_fns)
                });
                (effects::call_is_pure(callee) || helper_is_functionally_pure(callee) || user_pure)
                    && args.iter().all(|arg| {
                        value_is_functionally_pure(program, fn_ir, *arg, memo, visiting_fns, seen)
                    })
            }
            crate::mir::def::ValueKind::RecordLit { fields } => fields.iter().all(|(_, value)| {
                value_is_functionally_pure(program, fn_ir, *value, memo, visiting_fns, seen)
            }),
            crate::mir::def::ValueKind::FieldGet { base, .. } => {
                value_is_functionally_pure(program, fn_ir, *base, memo, visiting_fns, seen)
            }
            crate::mir::def::ValueKind::FieldSet { base, value, .. } => {
                value_is_functionally_pure(program, fn_ir, *base, memo, visiting_fns, seen)
                    && value_is_functionally_pure(program, fn_ir, *value, memo, visiting_fns, seen)
            }
        };
        seen.remove(&vid);
        pure
    }

    fn function_is_referentially_pure(
        program: &ProgramIR,
        name: &str,
        fn_ir: &crate::mir::def::FnIR,
        memo: &mut FxHashMap<String, bool>,
        visiting_fns: &mut FxHashSet<String>,
    ) -> bool {
        if let Some(cached) = memo.get(name) {
            return *cached;
        }
        if !visiting_fns.insert(name.to_string()) {
            return false;
        }
        if fn_ir.requires_conservative_optimization() {
            memo.insert(name.to_string(), false);
            visiting_fns.remove(name);
            return false;
        }
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                match instr {
                    crate::mir::def::Instr::Assign { src, .. }
                    | crate::mir::def::Instr::Eval { val: src, .. } => {
                        if !value_is_functionally_pure(
                            program,
                            fn_ir,
                            *src,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) {
                            memo.insert(name.to_string(), false);
                            visiting_fns.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex1D { base, idx, val, .. } => {
                        if !(value_is_functionally_pure(
                            program,
                            fn_ir,
                            *base,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *idx,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *val,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        )) {
                            memo.insert(name.to_string(), false);
                            visiting_fns.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        if !(value_is_functionally_pure(
                            program,
                            fn_ir,
                            *base,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *r,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *c,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *val,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        )) {
                            memo.insert(name.to_string(), false);
                            visiting_fns.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        if !(value_is_functionally_pure(
                            program,
                            fn_ir,
                            *base,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *i,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *j,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *k,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *val,
                            memo,
                            visiting_fns,
                            &mut FxHashSet::default(),
                        )) {
                            memo.insert(name.to_string(), false);
                            visiting_fns.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::UnsafeRBlock { .. } => {
                        memo.insert(name.to_string(), false);
                        visiting_fns.remove(name);
                        return false;
                    }
                }
            }
            match &block.term {
                crate::mir::def::Terminator::If { cond, .. } => {
                    if !value_is_functionally_pure(
                        program,
                        fn_ir,
                        *cond,
                        memo,
                        visiting_fns,
                        &mut FxHashSet::default(),
                    ) {
                        memo.insert(name.to_string(), false);
                        visiting_fns.remove(name);
                        return false;
                    }
                }
                crate::mir::def::Terminator::Return(Some(v)) => {
                    if !value_is_functionally_pure(
                        program,
                        fn_ir,
                        *v,
                        memo,
                        visiting_fns,
                        &mut FxHashSet::default(),
                    ) {
                        memo.insert(name.to_string(), false);
                        visiting_fns.remove(name);
                        return false;
                    }
                }
                crate::mir::def::Terminator::Goto(_)
                | crate::mir::def::Terminator::Return(None)
                | crate::mir::def::Terminator::Unreachable => {}
            }
        }
        memo.insert(name.to_string(), true);
        visiting_fns.remove(name);
        true
    }

    let mut out = FxHashSet::default();
    let mut memo = FxHashMap::default();
    let mut names: Vec<_> = program.fns.iter().map(|unit| unit.name.clone()).collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = program.get(&name) else {
            continue;
        };
        if function_is_referentially_pure(
            program,
            &name,
            fn_ir,
            &mut memo,
            &mut FxHashSet::default(),
        ) {
            out.insert(name);
        }
    }
    out
}

pub(crate) fn collect_fresh_returning_user_functions(program: &ProgramIR) -> FxHashSet<String> {
    struct FreshAnalysisCtx {
        pub(crate) pure_memo: FxHashMap<String, bool>,
        pub(crate) fresh_memo: FxHashMap<String, bool>,
        pub(crate) visiting_pure: FxHashSet<String>,
        pub(crate) visiting_fresh: FxHashSet<String>,
    }

    fn helper_is_functionally_pure(callee: &str) -> bool {
        matches!(
            callee,
            "rr_assign_slice"
                | "rr_ifelse_strict"
                | "rr_index1_read"
                | "rr_index1_read_strict"
                | "rr_index1_read_vec"
                | "rr_index1_read_vec_floor"
                | "rr_index_vec_floor"
                | "rr_gather"
                | "rr_wrap_index_vec"
                | "rr_wrap_index_vec_i"
                | "rr_idx_cube_vec_i"
                | "rr_named_list"
                | "rr_field_get"
                | "rr_field_exists"
                | "rr_list_pattern_matchable"
        )
    }

    fn helper_is_fresh_result(callee: &str) -> bool {
        matches!(
            callee,
            "rep.int"
                | "numeric"
                | "integer"
                | "logical"
                | "character"
                | "vector"
                | "matrix"
                | "c"
                | "seq_len"
                | "seq_along"
                | "rr_named_list"
        )
    }

    fn value_is_functionally_pure(
        program: &ProgramIR,
        fn_ir: &crate::mir::def::FnIR,
        vid: crate::mir::def::ValueId,
        ctx: &mut FreshAnalysisCtx,
        seen: &mut FxHashSet<crate::mir::def::ValueId>,
    ) -> bool {
        if !seen.insert(vid) {
            return false;
        }
        let pure = match &fn_ir.values[vid].kind {
            crate::mir::def::ValueKind::Const(_)
            | crate::mir::def::ValueKind::Param { .. }
            | crate::mir::def::ValueKind::Load { .. }
            | crate::mir::def::ValueKind::RSymbol { .. } => true,
            crate::mir::def::ValueKind::Phi { args } => args
                .iter()
                .all(|(src, _)| value_is_functionally_pure(program, fn_ir, *src, ctx, seen)),
            crate::mir::def::ValueKind::Len { base }
            | crate::mir::def::ValueKind::Indices { base } => {
                value_is_functionally_pure(program, fn_ir, *base, ctx, seen)
            }
            crate::mir::def::ValueKind::Range { start, end } => {
                value_is_functionally_pure(program, fn_ir, *start, ctx, seen)
                    && value_is_functionally_pure(program, fn_ir, *end, ctx, seen)
            }
            crate::mir::def::ValueKind::Binary { lhs, rhs, .. } => {
                value_is_functionally_pure(program, fn_ir, *lhs, ctx, seen)
                    && value_is_functionally_pure(program, fn_ir, *rhs, ctx, seen)
            }
            crate::mir::def::ValueKind::Unary { rhs, .. } => {
                value_is_functionally_pure(program, fn_ir, *rhs, ctx, seen)
            }
            crate::mir::def::ValueKind::Index1D { base, idx, .. } => {
                value_is_functionally_pure(program, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(program, fn_ir, *idx, ctx, seen)
            }
            crate::mir::def::ValueKind::Index2D { base, r, c } => {
                value_is_functionally_pure(program, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(program, fn_ir, *r, ctx, seen)
                    && value_is_functionally_pure(program, fn_ir, *c, ctx, seen)
            }
            crate::mir::def::ValueKind::Index3D { base, i, j, k } => {
                value_is_functionally_pure(program, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(program, fn_ir, *i, ctx, seen)
                    && value_is_functionally_pure(program, fn_ir, *j, ctx, seen)
                    && value_is_functionally_pure(program, fn_ir, *k, ctx, seen)
            }
            crate::mir::def::ValueKind::Intrinsic { args, .. } => args
                .iter()
                .all(|arg| value_is_functionally_pure(program, fn_ir, *arg, ctx, seen)),
            crate::mir::def::ValueKind::Call { callee, args, .. } => {
                let user_pure = program.get(callee).is_some_and(|callee_ir| {
                    function_is_referentially_pure(program, callee, callee_ir, ctx)
                });
                (effects::call_is_pure(callee) || helper_is_functionally_pure(callee) || user_pure)
                    && args
                        .iter()
                        .all(|arg| value_is_functionally_pure(program, fn_ir, *arg, ctx, seen))
            }
            crate::mir::def::ValueKind::RecordLit { fields } => fields
                .iter()
                .all(|(_, value)| value_is_functionally_pure(program, fn_ir, *value, ctx, seen)),
            crate::mir::def::ValueKind::FieldGet { base, .. } => {
                value_is_functionally_pure(program, fn_ir, *base, ctx, seen)
            }
            crate::mir::def::ValueKind::FieldSet { base, value, .. } => {
                value_is_functionally_pure(program, fn_ir, *base, ctx, seen)
                    && value_is_functionally_pure(program, fn_ir, *value, ctx, seen)
            }
        };
        seen.remove(&vid);
        pure
    }

    fn value_is_fresh_result(
        program: &ProgramIR,
        fn_ir: &crate::mir::def::FnIR,
        vid: crate::mir::def::ValueId,
        ctx: &mut FreshAnalysisCtx,
        seen: &mut FxHashSet<crate::mir::def::ValueId>,
    ) -> bool {
        if !seen.insert(vid) {
            return false;
        }
        let fresh = match &fn_ir.values[vid].kind {
            crate::mir::def::ValueKind::Const(_) => true,
            crate::mir::def::ValueKind::Call { callee, args, .. } => {
                let user_fresh = program.get(callee).is_some_and(|callee_ir| {
                    function_is_fresh_returning(program, callee, callee_ir, ctx)
                });
                (helper_is_fresh_result(callee) || user_fresh)
                    && args.iter().all(|arg| {
                        value_is_functionally_pure(
                            program,
                            fn_ir,
                            *arg,
                            ctx,
                            &mut FxHashSet::default(),
                        )
                    })
            }
            _ => false,
        };
        seen.remove(&vid);
        fresh
    }

    fn function_is_referentially_pure(
        program: &ProgramIR,
        name: &str,
        fn_ir: &crate::mir::def::FnIR,
        ctx: &mut FreshAnalysisCtx,
    ) -> bool {
        if let Some(cached) = ctx.pure_memo.get(name) {
            return *cached;
        }
        if !ctx.visiting_pure.insert(name.to_string()) {
            return false;
        }
        if fn_ir.requires_conservative_optimization() {
            ctx.pure_memo.insert(name.to_string(), false);
            ctx.visiting_pure.remove(name);
            return false;
        }
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                match instr {
                    crate::mir::def::Instr::Assign { src, .. }
                    | crate::mir::def::Instr::Eval { val: src, .. } => {
                        if !value_is_functionally_pure(
                            program,
                            fn_ir,
                            *src,
                            ctx,
                            &mut FxHashSet::default(),
                        ) {
                            ctx.pure_memo.insert(name.to_string(), false);
                            ctx.visiting_pure.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex1D { base, idx, val, .. } => {
                        if !(value_is_functionally_pure(
                            program,
                            fn_ir,
                            *base,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *idx,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *val,
                            ctx,
                            &mut FxHashSet::default(),
                        )) {
                            ctx.pure_memo.insert(name.to_string(), false);
                            ctx.visiting_pure.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex2D {
                        base, r, c, val, ..
                    } => {
                        if !(value_is_functionally_pure(
                            program,
                            fn_ir,
                            *base,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *r,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *c,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *val,
                            ctx,
                            &mut FxHashSet::default(),
                        )) {
                            ctx.pure_memo.insert(name.to_string(), false);
                            ctx.visiting_pure.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::StoreIndex3D {
                        base, i, j, k, val, ..
                    } => {
                        if !(value_is_functionally_pure(
                            program,
                            fn_ir,
                            *base,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *i,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *j,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *k,
                            ctx,
                            &mut FxHashSet::default(),
                        ) && value_is_functionally_pure(
                            program,
                            fn_ir,
                            *val,
                            ctx,
                            &mut FxHashSet::default(),
                        )) {
                            ctx.pure_memo.insert(name.to_string(), false);
                            ctx.visiting_pure.remove(name);
                            return false;
                        }
                    }
                    crate::mir::def::Instr::UnsafeRBlock { .. } => {
                        ctx.pure_memo.insert(name.to_string(), false);
                        ctx.visiting_pure.remove(name);
                        return false;
                    }
                }
            }
            match &block.term {
                crate::mir::def::Terminator::If { cond, .. } => {
                    if !value_is_functionally_pure(
                        program,
                        fn_ir,
                        *cond,
                        ctx,
                        &mut FxHashSet::default(),
                    ) {
                        ctx.pure_memo.insert(name.to_string(), false);
                        ctx.visiting_pure.remove(name);
                        return false;
                    }
                }
                crate::mir::def::Terminator::Return(Some(v)) => {
                    if !value_is_functionally_pure(
                        program,
                        fn_ir,
                        *v,
                        ctx,
                        &mut FxHashSet::default(),
                    ) {
                        ctx.pure_memo.insert(name.to_string(), false);
                        ctx.visiting_pure.remove(name);
                        return false;
                    }
                }
                crate::mir::def::Terminator::Goto(_)
                | crate::mir::def::Terminator::Return(None)
                | crate::mir::def::Terminator::Unreachable => {}
            }
        }
        ctx.pure_memo.insert(name.to_string(), true);
        ctx.visiting_pure.remove(name);
        true
    }

    fn function_is_fresh_returning(
        program: &ProgramIR,
        name: &str,
        fn_ir: &crate::mir::def::FnIR,
        ctx: &mut FreshAnalysisCtx,
    ) -> bool {
        if let Some(cached) = ctx.fresh_memo.get(name) {
            return *cached;
        }
        if !ctx.visiting_fresh.insert(name.to_string()) {
            return false;
        }
        if !function_is_referentially_pure(program, name, fn_ir, ctx) {
            ctx.fresh_memo.insert(name.to_string(), false);
            ctx.visiting_fresh.remove(name);
            return false;
        }
        let mut saw_return = false;
        for block in &fn_ir.blocks {
            if let crate::mir::def::Terminator::Return(Some(v)) = &block.term {
                saw_return = true;
                if !value_is_fresh_result(program, fn_ir, *v, ctx, &mut FxHashSet::default()) {
                    ctx.fresh_memo.insert(name.to_string(), false);
                    ctx.visiting_fresh.remove(name);
                    return false;
                }
            }
        }
        ctx.fresh_memo.insert(name.to_string(), saw_return);
        ctx.visiting_fresh.remove(name);
        saw_return
    }

    let mut out = FxHashSet::default();
    let mut ctx = FreshAnalysisCtx {
        pure_memo: FxHashMap::default(),
        fresh_memo: FxHashMap::default(),
        visiting_pure: FxHashSet::default(),
        visiting_fresh: FxHashSet::default(),
    };
    let mut names: Vec<_> = program.fns.iter().map(|unit| unit.name.clone()).collect();
    names.sort();
    for name in names {
        let Some(fn_ir) = program.get(&name) else {
            continue;
        };
        if function_is_fresh_returning(program, &name, fn_ir, &mut ctx) {
            out.insert(name);
        }
    }
    out
}
