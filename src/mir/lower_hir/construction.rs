use super::*;
impl<'a> MirLowerer<'a> {
    pub fn new(
        name: String,
        params: Vec<String>,
        var_names: FxHashMap<hir::LocalId, String>,
        symbols: &'a FxHashMap<hir::SymbolId, String>,
        known_functions: &'a FxHashMap<String, usize>,
    ) -> Self {
        let mut fn_ir = FnIR::new(name, params.clone());
        let entry = fn_ir.add_block();
        let body_head = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = body_head;

        // Init defs map for entry
        let mut defs = FxHashMap::default();
        defs.insert(entry, FxHashMap::default());
        defs.insert(body_head, FxHashMap::default());

        Self {
            fn_ir,
            curr_block: entry,
            defs,
            incomplete_phis: FxHashMap::default(),
            sealed_blocks: FxHashSet::default(),
            preds: FxHashMap::default(),
            var_names,
            symbols,
            known_functions,
            loop_stack: Vec::new(),
            tidy_mask_depth: 0,
            unsafe_r_seen: false,
        }
    }

    pub(crate) fn with_tidy_mask<T>(&mut self, f: impl FnOnce(&mut Self) -> RR<T>) -> RR<T> {
        self.tidy_mask_depth += 1;
        let out = f(self);
        self.tidy_mask_depth -= 1;
        out
    }

    pub(crate) fn in_tidy_mask(&self) -> bool {
        self.tidy_mask_depth > 0
    }

    // Core Helpers
    pub(crate) fn add_pred(&mut self, target: BlockId, pred: BlockId) {
        self.preds.entry(target).or_default().push(pred);
    }

    // Standardize Value Addition
    pub(crate) fn add_value(&mut self, kind: ValueKind, span: Span) -> ValueId {
        let vid = self.fn_ir.add_value(kind, span, Facts::empty(), None);
        self.annotate_new_value(vid);
        vid
    }

    pub(crate) fn add_value_with_name(
        &mut self,
        kind: ValueKind,
        span: Span,
        var_name: Option<String>,
    ) -> ValueId {
        let vid = self.fn_ir.add_value(kind, span, Facts::empty(), var_name);
        self.annotate_new_value(vid);
        vid
    }

    pub(crate) fn annotate_new_value(&mut self, vid: ValueId) {
        match &self.fn_ir.values[vid].kind {
            ValueKind::Call { callee, .. } => {
                if let Some(kind) = builtin_kind_for_name(callee) {
                    self.fn_ir
                        .set_call_semantics(vid, CallSemantics::Builtin(kind));
                    match kind {
                        BuiltinKind::SeqAlong
                        | BuiltinKind::SeqLen
                        | BuiltinKind::C
                        | BuiltinKind::Numeric
                        | BuiltinKind::Character
                        | BuiltinKind::Logical
                        | BuiltinKind::Integer
                        | BuiltinKind::Double
                        | BuiltinKind::Rep
                        | BuiltinKind::RepInt
                        | BuiltinKind::Vector => {
                            self.fn_ir
                                .set_memory_layout_hint(vid, MemoryLayoutHint::Dense1D);
                        }
                        BuiltinKind::Matrix
                        | BuiltinKind::Transpose
                        | BuiltinKind::Diag
                        | BuiltinKind::Rbind
                        | BuiltinKind::Cbind
                        | BuiltinKind::Crossprod
                        | BuiltinKind::Tcrossprod => {
                            self.fn_ir
                                .set_memory_layout_hint(vid, MemoryLayoutHint::ColumnMajor2D);
                        }
                        BuiltinKind::Array => {
                            self.fn_ir
                                .set_memory_layout_hint(vid, MemoryLayoutHint::ColumnMajorND);
                        }
                        _ => {}
                    }
                } else if callee == "rr_call_closure" {
                    self.fn_ir
                        .set_call_semantics(vid, CallSemantics::ClosureDispatch);
                } else if callee.starts_with("rr_") {
                    self.fn_ir
                        .set_call_semantics(vid, CallSemantics::RuntimeHelper);
                } else {
                    self.fn_ir
                        .set_call_semantics(vid, CallSemantics::UserDefined);
                }
            }
            ValueKind::Len { .. } | ValueKind::Range { .. } | ValueKind::Indices { .. } => {
                self.fn_ir
                    .set_memory_layout_hint(vid, MemoryLayoutHint::Dense1D);
            }
            _ => {}
        }
    }

    // Core Helpers
    pub(crate) fn define_var_at(
        &mut self,
        block: BlockId,
        var: hir::LocalId,
        val: ValueId,
        emit_assign: bool,
    ) {
        let name = self.var_names.get(&var).cloned();
        if let Some(n) = name {
            if emit_assign {
                self.fn_ir.blocks[block].instrs.push(Instr::Assign {
                    dst: n.clone(),
                    src: val,
                    span: Span::default(),
                });
                let mismatched_origin = self
                    .fn_ir
                    .values
                    .get(val)
                    .and_then(|v| v.origin_var.as_ref())
                    .map(|orig| orig != &n)
                    .unwrap_or(false);
                let is_phi_value = self
                    .fn_ir
                    .values
                    .get(val)
                    .map(|v| matches!(v.kind, ValueKind::Phi { .. }))
                    .unwrap_or(false);

                let def_val = if mismatched_origin || is_phi_value {
                    self.add_value_with_name(
                        ValueKind::Load { var: n.clone() },
                        Span::default(),
                        Some(n),
                    )
                } else {
                    if let Some(v) = self.fn_ir.values.get_mut(val)
                        && v.origin_var.is_none()
                    {
                        v.origin_var = Some(n);
                    }
                    val
                };
                self.defs.entry(block).or_default().insert(var, def_val);
                return;
            }

            if let Some(v) = self.fn_ir.values.get_mut(val)
                && v.origin_var.is_none()
            {
                v.origin_var = Some(n);
            }
        }

        self.defs.entry(block).or_default().insert(var, val);
    }

    pub(crate) fn write_var(&mut self, var: hir::LocalId, val: ValueId) {
        self.define_var_at(self.curr_block, var, val, true);
    }

    pub(crate) fn visible_def_locals(
        &self,
        block: BlockId,
        seen: &mut FxHashSet<BlockId>,
    ) -> FxHashSet<hir::LocalId> {
        let mut locals: FxHashSet<hir::LocalId> = self
            .defs
            .get(&block)
            .map(|defs| defs.keys().copied().collect())
            .unwrap_or_default();
        if !seen.insert(block) {
            return locals;
        }

        let preds = self.preds.get(&block).cloned().unwrap_or_default();
        if preds.is_empty() {
            return locals;
        }

        let mut pred_sets = preds.into_iter().map(|pred| {
            let mut pred_seen = seen.clone();
            self.visible_def_locals(pred, &mut pred_seen)
        });
        let Some(mut common) = pred_sets.next() else {
            return locals;
        };
        for pred_set in pred_sets {
            common.retain(|local| pred_set.contains(local));
        }
        locals.extend(common);
        locals
    }

    pub(crate) fn invalidate_defs_after_unsafe_r(&mut self, span: Span) {
        let mut affected_locals = self
            .visible_def_locals(self.curr_block, &mut FxHashSet::default())
            .into_iter()
            .collect::<FxHashSet<_>>();
        // Raw R can read or assign any symbol in the function's R frame. Keep
        // every RR local conservatively backed by a post-block Load so branch
        // joins cannot fold back to a stale SSA constant.
        //
        // NOTE: This currently models the generated function frame, not nested
        // R environments. If RR grows deeper closure/capture lowering, revisit
        // this invalidation boundary so captured locals are neither missed nor
        // over-promised as direct frame bindings.
        affected_locals.extend(self.var_names.keys().copied());
        for local in affected_locals {
            let Some(var) = self.var_names.get(&local).cloned() else {
                continue;
            };
            let load =
                self.add_value_with_name(ValueKind::Load { var: var.clone() }, span, Some(var));
            self.defs
                .entry(self.curr_block)
                .or_default()
                .insert(local, load);
        }
    }

    pub(crate) fn unsafe_r_load_for_var(
        &mut self,
        var: hir::LocalId,
        resolved: ValueId,
    ) -> Option<ValueId> {
        if !self.unsafe_r_seen {
            return None;
        }
        let name = self.var_names.get(&var)?.clone();
        if matches!(
            self.fn_ir.values.get(resolved).map(|value| &value.kind),
            Some(ValueKind::Load { var }) if var == &name
        ) {
            return Some(resolved);
        }
        let span = self
            .fn_ir
            .values
            .get(resolved)
            .map(|value| value.span)
            .unwrap_or_default();
        let load =
            self.add_value_with_name(ValueKind::Load { var: name.clone() }, span, Some(name));
        if let Some(src) = self.fn_ir.values.get(resolved).cloned()
            && let Some(dst) = self.fn_ir.values.get_mut(load)
        {
            dst.facts = src.facts;
            dst.value_ty = src.value_ty;
            dst.value_term = src.value_term;
            dst.escape = src.escape;
        }
        Some(load)
    }

    pub(crate) fn mutable_local_base_for_lvalue(
        &mut self,
        var: hir::LocalId,
        span: Span,
    ) -> RR<ValueId> {
        let resolved = self.read_var(var, self.curr_block)?;
        let Some(name) = self.var_names.get(&var).cloned() else {
            return Ok(resolved);
        };
        if matches!(
            self.fn_ir.values.get(resolved).map(|value| &value.kind),
            Some(ValueKind::Load { var }) if var == &name
        ) {
            return Ok(resolved);
        }

        let load =
            self.add_value_with_name(ValueKind::Load { var: name.clone() }, span, Some(name));
        if let Some(src) = self.fn_ir.values.get(resolved).cloned()
            && let Some(dst) = self.fn_ir.values.get_mut(load)
        {
            dst.facts = src.facts;
            dst.value_ty = src.value_ty;
            dst.value_term = src.value_term;
            dst.escape = src.escape;
        }
        Ok(load)
    }

    pub(crate) fn mark_local_mutated_after_index_store(
        &mut self,
        var: hir::LocalId,
        base: ValueId,
        span: Span,
    ) {
        let Some(name) = self.var_names.get(&var).cloned() else {
            return;
        };
        let mutated = if matches!(
            self.fn_ir.values.get(base).map(|value| &value.kind),
            Some(ValueKind::Load { var }) if var == &name
        ) {
            base
        } else {
            let load = self.add_value_with_name(
                ValueKind::Load { var: name.clone() },
                span,
                Some(name.clone()),
            );
            if let Some(src) = self.fn_ir.values.get(base).cloned()
                && let Some(dst) = self.fn_ir.values.get_mut(load)
            {
                dst.facts = src.facts;
                dst.value_ty = src.value_ty;
                dst.value_term = src.value_term;
                dst.escape = src.escape;
            }
            load
        };
        self.defs
            .entry(self.curr_block)
            .or_default()
            .insert(var, mutated);
    }

    pub(crate) fn read_var(&mut self, var: hir::LocalId, block: BlockId) -> RR<ValueId> {
        let resolved = if let Some(m) = self.defs.get(&block)
            && let Some(&v) = m.get(&var)
        {
            v
        } else {
            // Not found in local, look in predecessors.
            self.read_var_recursive(var, block)?
        };
        Ok(self
            .unsafe_r_load_for_var(var, resolved)
            .unwrap_or(resolved))
    }

    // Sealed Block SSA Construction (Braun et al.)

    pub(crate) fn seal_block(&mut self, block: BlockId) -> RR<()> {
        if self.sealed_blocks.contains(&block) {
            return Ok(());
        }

        // Resolve incomplete Phis
        if let Some(incomplete) = self.incomplete_phis.remove(&block) {
            for (var, phi_val) in incomplete {
                self.add_phi_operands(block, var, phi_val)?;
            }
        }

        self.sealed_blocks.insert(block);
        Ok(())
    }

    pub(crate) fn read_var_recursive(&mut self, var: hir::LocalId, block: BlockId) -> RR<ValueId> {
        if !self.sealed_blocks.contains(&block) {
            // Create a placeholder phi and resolve operands when the block is sealed.
            let phi = self.add_phi_placeholder(block, Span::default());
            self.incomplete_phis
                .entry(block)
                .or_default()
                .push((var, phi));
            // Define the SSA name for this block without emitting an assignment.
            self.define_var_at(block, var, phi, false);
            return Ok(phi);
        }

        let preds = self.preds.get(&block).cloned().unwrap_or_default();
        if preds.is_empty() {
            let var_name = self
                .var_names
                .get(&var)
                .cloned()
                .unwrap_or_else(|| format!("local#{}", var.0));
            Err(crate::error::RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E1001,
                crate::error::Stage::Mir,
                format!("undefined variable '{}'", var_name),
            )
            .at(Span::default())
            .push_frame(
                "mir::lower_hir::read_var_recursive/2",
                Some(Span::default()),
            )
            .note("Declare the variable with let before use."))
        } else if preds.len() == 1 {
            // Optimize: No phi needed, just look in pred
            self.read_var(var, preds[0])
        } else {
            // Multiple predecessors require a phi.
            let phi = self.add_phi_placeholder(block, Span::default());
            // Break cycles with a Phi placeholder, but don't emit an assignment yet.
            self.define_var_at(block, var, phi, false);
            self.add_phi_operands(block, var, phi)?;
            Ok(phi)
        }
    }

    pub(crate) fn add_phi_operands(
        &mut self,
        block: BlockId,
        var: hir::LocalId,
        phi_val: ValueId,
    ) -> RR<()> {
        // Collect operands from all preds
        let preds = self.preds.get(&block).cloned().unwrap_or_default();
        let mut new_args = Vec::new();
        for pred in preds {
            let val = self.read_var(var, pred)?;
            new_args.push((val, pred));
        }

        if let Some(src) = self.trivial_phi_source(phi_val, &new_args, &mut FxHashSet::default()) {
            self.defs.entry(block).or_default().insert(var, src);
            let src_val = self.fn_ir.values[src].clone();
            if let Some(dst) = self.fn_ir.values.get_mut(phi_val) {
                dst.kind = src_val.kind;
                dst.facts = src_val.facts;
                dst.value_ty = src_val.value_ty;
                dst.value_term = src_val.value_term;
                if dst.origin_var.is_none() {
                    dst.origin_var = src_val.origin_var;
                }
                dst.phi_block = None;
                dst.escape = src_val.escape;
            }
            return Ok(());
        }

        // Update Phi instruction
        if let Some(val) = self.fn_ir.values.get_mut(phi_val) {
            if let ValueKind::Phi { ref mut args } = val.kind {
                *args = new_args;
            } else {
                return Err(InternalCompilerError::new(
                    Stage::Mir,
                    format!("Value {} is not a Phi during SSA sealing", phi_val),
                )
                .into_exception());
            }
        } else {
            return Err(InternalCompilerError::new(
                Stage::Mir,
                format!("Value {} not found during SSA sealing", phi_val),
            )
            .into_exception());
        }

        Ok(())
    }

    pub(crate) fn trivial_phi_source(
        &self,
        phi_val: ValueId,
        args: &[(ValueId, BlockId)],
        seen: &mut FxHashSet<ValueId>,
    ) -> Option<ValueId> {
        if !seen.insert(phi_val) {
            return None;
        }
        let mut candidate = None;
        for (arg, pred) in args {
            if *arg == phi_val {
                continue;
            }
            let resolved = match &self.fn_ir.values[*arg].kind {
                ValueKind::Phi { args: nested } => {
                    self.trivial_phi_source(*arg, nested, seen).unwrap_or(*arg)
                }
                _ => *arg,
            };
            let resolved = self.canonicalize_phi_arg_for_pred(*pred, resolved);
            match candidate {
                None => candidate = Some(resolved),
                Some(prev) if prev == resolved => {}
                Some(_) => return None,
            }
        }
        candidate
    }

    pub(crate) fn canonicalize_phi_arg_for_pred(
        &self,
        pred: BlockId,
        mut value: ValueId,
    ) -> ValueId {
        let mut seen = FxHashSet::default();
        while seen.insert(value) {
            let ValueKind::Load { var } = &self.fn_ir.values[value].kind else {
                break;
            };
            let Some(next) = self.fn_ir.blocks[pred]
                .instrs
                .iter()
                .rev()
                .find_map(|instr| match instr {
                    Instr::Assign { dst, src, .. } if dst == var => Some(*src),
                    _ => None,
                })
            else {
                break;
            };
            value = next;
        }
        value
    }

    pub(crate) fn add_phi_placeholder(&mut self, _block: BlockId, span: Span) -> ValueId {
        let id = self.add_value(ValueKind::Phi { args: vec![] }, span);
        if let Some(v) = self.fn_ir.values.get_mut(id) {
            v.phi_block = Some(_block);
        }
        id
    }
}
