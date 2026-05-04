use super::*;
impl<'a> MirLowerer<'a> {
    pub(crate) fn block_contains_write_unsafe_r(blk: &hir::HirBlock) -> bool {
        blk.stmts.iter().any(|stmt| match stmt {
            hir::HirStmt::UnsafeRBlock { read_only, .. } => !read_only,
            hir::HirStmt::If {
                then_blk, else_blk, ..
            } => {
                Self::block_contains_write_unsafe_r(then_blk)
                    || else_blk
                        .as_ref()
                        .is_some_and(Self::block_contains_write_unsafe_r)
            }
            hir::HirStmt::While { body, .. } | hir::HirStmt::For { body, .. } => {
                Self::block_contains_write_unsafe_r(body)
            }
            _ => false,
        })
    }

    // Call update: terminate must track preds

    // Proof correspondence:
    // `proof/lean/RRProofs/LoweringSubset.lean`,
    // `proof/lean/RRProofs/LoweringIfPhiSubset.lean`,
    // `proof/lean/RRProofs/PipelineBlockEnvSubset.lean`,
    // `proof/lean/RRProofs/PipelineFnEnvSubset.lean`,
    // `proof/lean/RRProofs/PipelineFnCfgSubset.lean`,
    // and the Coq `Lowering*` / `Pipeline*Subset` companions model reduced
    // slices of this source-to-MIR lowering entry point.
    pub fn lower_fn(mut self, f: hir::HirFn) -> RR<FnIR> {
        self.unsafe_r_seen = Self::block_contains_write_unsafe_r(&f.body);
        self.fn_ir.span = f.span;
        self.fn_ir.user_name = self.symbols.get(&f.name).cloned();
        self.fn_ir.param_default_r_exprs = f
            .params
            .iter()
            .map(|p| {
                p.default
                    .as_ref()
                    .map(|expr| self.render_default_expr(expr))
                    .transpose()
            })
            .collect::<RR<Vec<_>>>()?;
        self.fn_ir.param_spans = f.params.iter().map(|p| p.span).collect();
        self.fn_ir.param_ty_hints = f
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(hir_ty_to_type_state)
                    .unwrap_or(crate::typeck::TypeState::unknown())
            })
            .collect();
        self.fn_ir.param_term_hints = f
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .map(|ty| hir_ty_to_type_term_with_symbols(ty, self.symbols))
                    .unwrap_or(crate::typeck::TypeTerm::Any)
            })
            .collect();
        self.fn_ir.param_hint_spans = f
            .params
            .iter()
            .map(|p| {
                p.ty.as_ref()
                    .and_then(|_| (!p.ty_inferred).then_some(p.span))
            })
            .collect();
        self.fn_ir.ret_ty_hint = f.ret_ty.as_ref().map(hir_ty_to_type_state);
        self.fn_ir.ret_term_hint = f
            .ret_ty
            .as_ref()
            .map(|ty| hir_ty_to_type_term_with_symbols(ty, self.symbols));
        self.fn_ir.ret_hint_span = f
            .ret_ty
            .as_ref()
            .and_then(|_| (!f.ret_ty_inferred).then_some(f.span));

        // 1. Bind parameters in the entry block
        for (i, param) in f.params.iter().enumerate() {
            let param_name = self.symbols.get(&param.name).cloned().ok_or_else(|| {
                InternalCompilerError::new(
                    Stage::Mir,
                    format!(
                        "missing parameter symbol during MIR lowering: {:?}",
                        param.name
                    ),
                )
                .into_exception()
            })?; // Clone early to avoid borrow conflict
            if let Some((&local_id, _)) =
                self.var_names.iter().find(|(_, name)| **name == param_name)
            {
                let local_param_name = self.unique_param_local_name(&param_name, local_id);
                self.var_names.insert(local_id, local_param_name.clone());
                // Initialize parameter Value
                let param_val = self.add_value(ValueKind::Param { index: i }, param.span);
                // Parameter writes always target an internal local copy to avoid accidental
                // mutation/aliasing of the visible argument symbol in generated R.
                if let Some(v) = self.fn_ir.values.get_mut(param_val) {
                    v.origin_var = Some(local_param_name);
                }
                // Write to the variable (this also emits Instr::Assign in entry block)
                self.write_var(local_id, param_val);
            }
        }

        // 2. Transition from Entry to Body Head
        let entry_bb = self.fn_ir.entry;
        let head_bb = self.fn_ir.body_head;
        self.add_pred(head_bb, entry_bb);
        self.terminate(Terminator::Goto(head_bb));
        self.curr_block = head_bb;
        self.seal_block(head_bb)?;

        // 3. Lower Body
        let ret_val = self.lower_block(f.body)?;

        // Implicit return if not terminated
        if !self.is_terminated(self.curr_block) {
            self.fn_ir.blocks[self.curr_block].term = Terminator::Return(Some(ret_val));
        }

        Ok(self.fn_ir)
    }

    pub(crate) fn lower_block(&mut self, blk: hir::HirBlock) -> RR<ValueId> {
        let mut last_val = self.add_void_val(blk.span);
        let len = blk.stmts.len();

        for (i, stmt) in blk.stmts.into_iter().enumerate() {
            if let hir::HirStmt::Expr { expr, span } = stmt {
                let val = self.lower_expr(expr)?;
                if i < len - 1 {
                    // Non-tail expression statements are evaluated for effects.
                    self.fn_ir.blocks[self.curr_block]
                        .instrs
                        .push(Instr::Eval { val, span });
                    last_val = self.add_void_val(span);
                } else {
                    last_val = val;
                }
            } else {
                self.lower_stmt(stmt)?;
                last_val = self.add_void_val(blk.span);
            }
        }
        Ok(last_val)
    }

    pub(crate) fn lower_block_effects(&mut self, blk: hir::HirBlock) -> RR<()> {
        for stmt in blk.stmts {
            match stmt {
                hir::HirStmt::Expr { expr, span } => {
                    let val = self.lower_expr(expr)?;
                    self.fn_ir.blocks[self.curr_block]
                        .instrs
                        .push(Instr::Eval { val, span });
                }
                other => {
                    self.lower_stmt(other)?;
                }
            }
        }
        Ok(())
    }
}
