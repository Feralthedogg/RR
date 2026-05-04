use super::*;

pub(crate) struct HoistedSubexprState<'a> {
    pub(crate) values: &'a [Value],
    pub(crate) params: &'a [String],
    pub(crate) counts: &'a FxHashMap<usize, usize>,
    pub(crate) emitted_ids: &'a mut FxHashSet<usize>,
    pub(crate) path: &'a mut FxHashSet<usize>,
    pub(crate) temps: &'a mut Vec<String>,
}

impl RBackend {
    pub(crate) fn function_frame_vars(fn_ir: &FnIR) -> FxHashSet<String> {
        let mut vars = fn_ir.params.iter().cloned().collect::<FxHashSet<_>>();
        for block in &fn_ir.blocks {
            for instr in &block.instrs {
                match instr {
                    Instr::Assign { dst, .. } => {
                        vars.insert(dst.clone());
                    }
                    Instr::StoreIndex1D { base, .. }
                    | Instr::StoreIndex2D { base, .. }
                    | Instr::StoreIndex3D { base, .. } => {
                        if let Some(var) = Self::named_written_base(*base, &fn_ir.values) {
                            vars.insert(var);
                        }
                    }
                    Instr::Eval { .. } | Instr::UnsafeRBlock { .. } => {}
                }
            }
        }
        for value in &fn_ir.values {
            if let Some(var) = &value.origin_var {
                vars.insert(var.clone());
            }
            if let ValueKind::Load { var } = &value.kind {
                vars.insert(var.clone());
            }
        }
        vars
    }

    pub(crate) fn named_written_base(base: usize, values: &[Value]) -> Option<String> {
        if let Some(var) = values[base].origin_var.as_ref() {
            return Some(var.clone());
        }
        match &values[base].kind {
            ValueKind::Load { var } => Some(var.clone()),
            _ => None,
        }
    }

    pub(crate) fn named_written_base_in_context(
        &self,
        base: usize,
        fn_ir: &FnIR,
    ) -> Option<String> {
        self.resolve_named_mutable_base_var(base, &fn_ir.values, &fn_ir.params)
            .or_else(|| Self::named_written_base(base, &fn_ir.values))
    }

    pub(crate) fn collect_mutated_vars(
        &self,
        node: &StructuredBlock,
        fn_ir: &FnIR,
        out: &mut FxHashSet<String>,
    ) {
        match node {
            StructuredBlock::Sequence(items) => {
                for item in items {
                    self.collect_mutated_vars(item, fn_ir, out);
                }
            }
            StructuredBlock::BasicBlock(bid) => {
                for instr in &fn_ir.blocks[*bid].instrs {
                    match instr {
                        Instr::Assign { dst, .. } => {
                            out.insert(dst.clone());
                        }
                        Instr::StoreIndex1D { base, .. }
                        | Instr::StoreIndex2D { base, .. }
                        | Instr::StoreIndex3D { base, .. } => {
                            if let Some(var) = self.named_written_base_in_context(*base, fn_ir) {
                                out.insert(var);
                            }
                        }
                        Instr::UnsafeRBlock { .. } => {
                            out.extend(Self::function_frame_vars(fn_ir));
                        }
                        Instr::Eval { .. } => {}
                    }
                }
            }
            StructuredBlock::If {
                then_body,
                else_body,
                ..
            } => {
                self.collect_mutated_vars(then_body, fn_ir, out);
                if let Some(else_body) = else_body {
                    self.collect_mutated_vars(else_body, fn_ir, out);
                }
            }
            StructuredBlock::Loop { header, body, .. } => {
                for instr in &fn_ir.blocks[*header].instrs {
                    match instr {
                        Instr::Assign { dst, .. } => {
                            out.insert(dst.clone());
                        }
                        Instr::StoreIndex1D { base, .. }
                        | Instr::StoreIndex2D { base, .. }
                        | Instr::StoreIndex3D { base, .. } => {
                            if let Some(var) = self.named_written_base_in_context(*base, fn_ir) {
                                out.insert(var);
                            }
                        }
                        Instr::UnsafeRBlock { .. } => {
                            out.extend(Self::function_frame_vars(fn_ir));
                        }
                        Instr::Eval { .. } => {}
                    }
                }
                self.collect_mutated_vars(body, fn_ir, out);
            }
            StructuredBlock::Break | StructuredBlock::Next | StructuredBlock::Return(_) => {}
        }
    }

    pub(crate) fn collect_loop_invariant_scalar_candidates_from_instrs(
        &self,
        instrs: &[Instr],
        values: &[Value],
        loop_mutated_vars: &FxHashSet<String>,
        visited: &mut FxHashSet<usize>,
        out: &mut Vec<usize>,
    ) {
        for instr in instrs {
            match instr {
                Instr::Assign { src, .. } | Instr::Eval { val: src, .. } => {
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *src,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
                Instr::StoreIndex1D { idx, val, .. } => {
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *idx,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *val,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
                Instr::StoreIndex2D { r, c, val, .. } => {
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *r,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *c,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *val,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
                Instr::StoreIndex3D { i, j, k, val, .. } => {
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *i,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *j,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *k,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        *val,
                        values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
                Instr::UnsafeRBlock { .. } => {}
            }
        }
    }

    pub(crate) fn collect_loop_invariant_scalar_candidates_from_block(
        &self,
        node: &StructuredBlock,
        fn_ir: &FnIR,
        loop_mutated_vars: &FxHashSet<String>,
        visited: &mut FxHashSet<usize>,
        out: &mut Vec<usize>,
    ) {
        match node {
            StructuredBlock::Sequence(items) => {
                for item in items {
                    self.collect_loop_invariant_scalar_candidates_from_block(
                        item,
                        fn_ir,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
            }
            StructuredBlock::BasicBlock(bid) => {
                self.collect_loop_invariant_scalar_candidates_from_instrs(
                    &fn_ir.blocks[*bid].instrs,
                    &fn_ir.values,
                    loop_mutated_vars,
                    visited,
                    out,
                );
                if let Terminator::Return(Some(val)) = fn_ir.blocks[*bid].term {
                    self.collect_loop_invariant_scalar_candidates_from_value(
                        val,
                        &fn_ir.values,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
            }
            StructuredBlock::If {
                cond,
                then_body,
                else_body,
            } => {
                self.collect_loop_invariant_scalar_candidates_from_value(
                    *cond,
                    &fn_ir.values,
                    loop_mutated_vars,
                    visited,
                    out,
                );
                self.collect_loop_invariant_scalar_candidates_from_block(
                    then_body,
                    fn_ir,
                    loop_mutated_vars,
                    visited,
                    out,
                );
                if let Some(else_body) = else_body {
                    self.collect_loop_invariant_scalar_candidates_from_block(
                        else_body,
                        fn_ir,
                        loop_mutated_vars,
                        visited,
                        out,
                    );
                }
            }
            StructuredBlock::Loop { .. } => {
                // Nested loops get their own LICM pass when emitted. Reaching into them from an
                // outer loop can hoist inner induction updates like `j + 1L` past the loop that
                // defines `j`, which is wrong-code.
            }
            StructuredBlock::Break | StructuredBlock::Next | StructuredBlock::Return(_) => {}
        }
    }

    pub(crate) fn collect_loop_invariant_scalar_candidates_from_value(
        &self,
        val_id: usize,
        values: &[Value],
        loop_mutated_vars: &FxHashSet<String>,
        visited: &mut FxHashSet<usize>,
        out: &mut Vec<usize>,
    ) {
        if !visited.insert(val_id) {
            return;
        }
        Self::for_each_expr_child(val_id, values, |child| {
            self.collect_loop_invariant_scalar_candidates_from_value(
                child,
                values,
                loop_mutated_vars,
                visited,
                out,
            );
        });
        if self.is_loop_invariant_scalar_expr_candidate(val_id, values, loop_mutated_vars) {
            out.push(val_id);
        }
    }

    pub(crate) fn is_loop_invariant_scalar_expr_candidate(
        &self,
        val_id: usize,
        values: &[Value],
        loop_mutated_vars: &FxHashSet<String>,
    ) -> bool {
        if !self.value_is_scalar_shape(val_id, values) {
            return false;
        }
        match values.get(val_id).map(|value| &value.kind) {
            Some(ValueKind::Unary { op, rhs }) => {
                !matches!(op, UnaryOp::Formula)
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *rhs,
                        values,
                        loop_mutated_vars,
                        &mut FxHashSet::default(),
                    )
            }
            Some(ValueKind::Binary { op, lhs, rhs }) => {
                !matches!(op, BinOp::MatMul)
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *lhs,
                        values,
                        loop_mutated_vars,
                        &mut FxHashSet::default(),
                    )
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *rhs,
                        values,
                        loop_mutated_vars,
                        &mut FxHashSet::default(),
                    )
            }
            _ => false,
        }
    }

    pub(crate) fn value_depends_only_on_loop_invariant_inputs(
        &self,
        val_id: usize,
        values: &[Value],
        loop_mutated_vars: &FxHashSet<String>,
        seen: &mut FxHashSet<usize>,
    ) -> bool {
        if !seen.insert(val_id) {
            return true;
        }
        match values.get(val_id).map(|value| &value.kind) {
            Some(ValueKind::Const(_) | ValueKind::Param { .. } | ValueKind::RSymbol { .. }) => true,
            Some(ValueKind::Load { var }) => !loop_mutated_vars.contains(var),
            Some(ValueKind::Unary { op, rhs }) => {
                !matches!(op, UnaryOp::Formula)
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *rhs,
                        values,
                        loop_mutated_vars,
                        seen,
                    )
            }
            Some(ValueKind::Binary { op, lhs, rhs }) => {
                !matches!(op, BinOp::MatMul)
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *lhs,
                        values,
                        loop_mutated_vars,
                        seen,
                    )
                    && self.value_depends_only_on_loop_invariant_inputs(
                        *rhs,
                        values,
                        loop_mutated_vars,
                        seen,
                    )
            }
            _ => false,
        }
    }

    pub(crate) fn emit_loop_invariant_scalar_hoists(
        &mut self,
        header: BlockId,
        cond: usize,
        body: &StructuredBlock,
        fn_ir: &FnIR,
        loop_mutated_vars: &FxHashSet<String>,
        current_loop_idx_var: Option<&str>,
    ) {
        if fn_ir.requires_conservative_optimization() {
            return;
        }
        let mut candidates = Vec::new();
        let mut visited = FxHashSet::default();
        self.collect_loop_invariant_scalar_candidates_from_instrs(
            &fn_ir.blocks[header].instrs,
            &fn_ir.values,
            loop_mutated_vars,
            &mut visited,
            &mut candidates,
        );
        self.collect_loop_invariant_scalar_candidates_from_value(
            cond,
            &fn_ir.values,
            loop_mutated_vars,
            &mut visited,
            &mut candidates,
        );
        self.collect_loop_invariant_scalar_candidates_from_block(
            body,
            fn_ir,
            loop_mutated_vars,
            &mut visited,
            &mut candidates,
        );

        for val_id in candidates {
            if self.resolve_bound_value(val_id).is_some() {
                continue;
            }
            let expr = self.resolve_val(val_id, &fn_ir.values, &fn_ir.params, false);
            let expr_idents = crate::compiler::pipeline::raw_expr_idents(expr.as_str());
            if expr_idents
                .iter()
                .any(|ident| loop_mutated_vars.contains(ident))
            {
                continue;
            }
            if current_loop_idx_var
                .is_some_and(|idx_var| expr_idents.iter().any(|ident| ident == idx_var))
            {
                continue;
            }
            let temp_name = format!("licm_{val_id}");
            self.write_stmt(&format!("{temp_name} <- {expr}"));
            self.note_var_write(&temp_name);
            self.bind_value_to_var(val_id, &temp_name);
            self.bind_var_to_value(&temp_name, val_id);
        }
    }

    pub(crate) fn emit_common_subexpr_temps(
        &mut self,
        root: usize,
        values: &[Value],
        params: &[String],
    ) {
        let mut counts = std::mem::take(&mut self.emit_scratch.expr_use_counts);
        let mut path = std::mem::take(&mut self.emit_scratch.expr_path);
        let mut emitted_ids = std::mem::take(&mut self.emit_scratch.emitted_ids);
        let mut temps = std::mem::take(&mut self.emit_scratch.emitted_temp_names);
        counts.clear();
        path.clear();
        emitted_ids.clear();
        temps.clear();

        Self::collect_expr_use_counts(root, values, &mut counts, &mut path);
        if !counts.values().any(|c| *c > 1) {
            self.emit_scratch.expr_use_counts = counts;
            self.emit_scratch.expr_path = path;
            self.emit_scratch.emitted_ids = emitted_ids;
            self.emit_scratch.emitted_temp_names = temps;
            return;
        }

        path.clear();
        self.emit_hoisted_subexprs_dfs(
            root,
            root,
            &mut HoistedSubexprState {
                values,
                params,
                counts: &counts,
                emitted_ids: &mut emitted_ids,
                path: &mut path,
                temps: &mut temps,
            },
        );
        self.emit_scratch.expr_use_counts = counts;
        self.emit_scratch.expr_path = path;
        self.emit_scratch.emitted_ids = emitted_ids;
        self.emit_scratch.emitted_temp_names = temps;
    }

    pub(crate) fn emit_hoisted_subexprs_dfs(
        &mut self,
        vid: usize,
        root: usize,
        state: &mut HoistedSubexprState<'_>,
    ) {
        if !state.path.insert(vid) {
            return;
        }
        Self::for_each_expr_child(vid, state.values, |child| {
            self.emit_hoisted_subexprs_dfs(child, root, state);
        });
        state.path.remove(&vid);

        if vid == root {
            return;
        }
        let uses = state.counts.get(&vid).copied().unwrap_or(0);
        if !Self::should_hoist_common_subexpr(vid, uses, state.values) {
            return;
        }
        if !state.emitted_ids.insert(vid) {
            return;
        }
        if self.resolve_bound_value(vid).is_some() {
            return;
        }
        if Self::should_prefer_stale_var_over_expr(&state.values[vid])
            && (self
                .resolve_stale_origin_var(vid, &state.values[vid], state.values)
                .is_some()
                || self
                    .resolve_stale_fresh_clone_var(vid, &state.values[vid], state.values)
                    .is_some())
        {
            return;
        }

        let temp = format!(".__rr_cse_{}", vid);
        let expr = self.rewrite_known_one_based_full_range_alias_reads(
            &self.resolve_val(vid, state.values, state.params, true),
            state.values,
            state.params,
        );
        self.write_stmt(&format!("{} <- {}", temp, expr));
        self.note_var_write(&temp);
        self.bind_value_to_var(vid, &temp);
        self.bind_var_to_value(&temp, vid);
        self.remember_known_full_end_expr(&temp, vid, state.values, state.params);
        state.temps.push(temp);
    }

    pub(crate) fn collect_expr_use_counts(
        root: usize,
        values: &[Value],
        counts: &mut FxHashMap<usize, usize>,
        path: &mut FxHashSet<usize>,
    ) {
        *counts.entry(root).or_insert(0) += 1;
        if !path.insert(root) {
            return;
        }
        Self::for_each_expr_child(root, values, |child| {
            Self::collect_expr_use_counts(child, values, counts, path);
        });
        path.remove(&root);
    }

    pub(crate) fn for_each_expr_child<F>(vid: usize, values: &[Value], mut visit: F)
    where
        F: FnMut(usize),
    {
        match &values[vid].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                visit(*lhs);
                visit(*rhs);
            }
            ValueKind::Unary { rhs, .. } => visit(*rhs),
            ValueKind::Call { args, .. } | ValueKind::Intrinsic { args, .. } => {
                for arg in args {
                    visit(*arg);
                }
            }
            ValueKind::RecordLit { fields } => {
                for (_, value) in fields {
                    visit(*value);
                }
            }
            ValueKind::FieldGet { base, .. } => visit(*base),
            ValueKind::FieldSet { base, value, .. } => {
                visit(*base);
                visit(*value);
            }
            ValueKind::Len { base } | ValueKind::Indices { base } => visit(*base),
            ValueKind::Range { start, end } => {
                visit(*start);
                visit(*end);
            }
            ValueKind::Index1D { base, idx, .. } => {
                visit(*base);
                visit(*idx);
            }
            ValueKind::Index2D { base, r, c } => {
                visit(*base);
                visit(*r);
                visit(*c);
            }
            ValueKind::Index3D { base, i, j, k } => {
                visit(*base);
                visit(*i);
                visit(*j);
                visit(*k);
            }
            ValueKind::Phi { args } => {
                for (value, _) in args {
                    visit(*value);
                }
            }
            ValueKind::Const(_)
            | ValueKind::Param { .. }
            | ValueKind::Load { .. }
            | ValueKind::RSymbol { .. } => {}
        }
    }

    pub(crate) fn invalidate_emitted_cse_temps(&mut self) {
        let mut temps = std::mem::take(&mut self.emit_scratch.emitted_temp_names);
        for temp in temps.drain(..) {
            self.note_var_write(&temp);
        }
        self.emit_scratch.emitted_temp_names = temps;
    }

    pub(crate) fn should_hoist_common_subexpr(vid: usize, uses: usize, values: &[Value]) -> bool {
        if uses <= 1 || values[vid].origin_var.is_some() {
            return false;
        }
        matches!(
            values[vid].kind,
            ValueKind::Call { .. }
                | ValueKind::Intrinsic { .. }
                | ValueKind::Index1D { .. }
                | ValueKind::Index2D { .. }
                | ValueKind::Index3D { .. }
                | ValueKind::Range { .. }
                | ValueKind::Len { .. }
                | ValueKind::Indices { .. }
        ) || match &values[vid].kind {
            ValueKind::Binary { lhs, rhs, .. } => {
                !Self::is_const_like_leaf(*lhs, values) || !Self::is_const_like_leaf(*rhs, values)
            }
            ValueKind::Unary { rhs, .. } => !Self::is_const_like_leaf(*rhs, values),
            _ => false,
        }
    }

    pub(crate) fn is_const_like_leaf(vid: usize, values: &[Value]) -> bool {
        matches!(values[vid].kind, ValueKind::Const(_))
    }
}
