use super::*;
impl<'a> MirLowerer<'a> {
    pub(crate) fn lower_expr(&mut self, expr: hir::HirExpr) -> RR<ValueId> {
        // println!("DEBUG: Lowering Expr: {:?}", expr);
        match expr {
            hir::HirExpr::Lit(l) => Ok(self.lower_literal(l)),
            hir::HirExpr::Local(l) => self.read_var(l, self.curr_block),
            hir::HirExpr::Global(sym, span) => Ok(self.lower_global(sym, span)),
            hir::HirExpr::Unary { op, expr } => self.lower_unary_expr(op, *expr),
            hir::HirExpr::Binary { op, lhs, rhs } => self.lower_binary_expr(op, *lhs, *rhs),
            hir::HirExpr::Field { base, name } => self.lower_field_expr(*base, name),
            hir::HirExpr::Index { base, index } => self.lower_index_expr(*base, index),
            hir::HirExpr::Block(blk) => self.lower_block(blk),

            hir::HirExpr::Call(hir::HirCall { callee, args, span }) => {
                self.lower_call_expr(callee, args, span)
            }
            hir::HirExpr::IfExpr {
                cond,
                then_expr,
                else_expr,
            } => self.lower_if_expr(*cond, *then_expr, *else_expr),
            hir::HirExpr::VectorLit(elems) => self.lower_vector_literal(elems),
            hir::HirExpr::ListLit(fields) => self.lower_list_literal(fields),
            hir::HirExpr::Range { start, end } => self.lower_range_expr(*start, *end),
            hir::HirExpr::Try(inner) => {
                // RR v1: try-postfix lowers to value propagation in MIR.
                // Runtime error propagation is still handled by R semantics.
                self.lower_expr(*inner)
            }
            hir::HirExpr::Match { scrut, arms } => self.lower_match_expr(*scrut, arms),
            hir::HirExpr::Column(name) => {
                Ok(self.add_value(ValueKind::RSymbol { name }, Span::default()))
            }
            hir::HirExpr::Unquote(e) => self.lower_expr(*e),
            _ => Err(Self::unsupported_expr_error(expr)),
        }
    }

    pub(crate) fn lower_literal(&mut self, lit: hir::HirLit) -> ValueId {
        let lit = match lit {
            hir::HirLit::Int(i) => Lit::Int(i),
            hir::HirLit::Double(f) => Lit::Float(f),
            hir::HirLit::Char(s) => Lit::Str(s),
            hir::HirLit::Bool(b) => Lit::Bool(b),
            hir::HirLit::NA => Lit::Na,
            hir::HirLit::Null => Lit::Null,
        };
        self.add_value(ValueKind::Const(lit), Span::default())
    }

    pub(crate) fn lower_global(&mut self, sym: hir::SymbolId, span: Span) -> ValueId {
        let raw_name = self
            .symbols
            .get(&sym)
            .cloned()
            .unwrap_or_else(|| format!("Sym_{}", sym.0));
        if self.in_tidy_mask() && Self::should_lower_as_tidy_symbol(&raw_name) {
            return self.add_value(
                ValueKind::RSymbol {
                    name: raw_name.clone(),
                },
                span,
            );
        }
        let name = if self.known_functions.contains_key(&raw_name) {
            format!("Sym_{}", sym.0)
        } else {
            raw_name
        };
        self.add_value_with_name(ValueKind::Load { var: name.clone() }, span, Some(name))
    }

    pub(crate) fn lower_unary_expr(&mut self, op: hir::HirUnOp, expr: hir::HirExpr) -> RR<ValueId> {
        let rhs = self.lower_expr(expr)?;
        let op = match op {
            hir::HirUnOp::Not => crate::syntax::ast::UnaryOp::Not,
            hir::HirUnOp::Neg => crate::syntax::ast::UnaryOp::Neg,
        };
        Ok(self.add_value(ValueKind::Unary { op, rhs }, Span::default()))
    }

    pub(crate) fn lower_binary_expr(
        &mut self,
        op: hir::HirBinOp,
        lhs: hir::HirExpr,
        rhs: hir::HirExpr,
    ) -> RR<ValueId> {
        let lhs = self.lower_expr(lhs)?;
        let rhs = self.lower_expr(rhs)?;
        let op = self.map_binop(op);
        Ok(self.add_value(ValueKind::Binary { op, lhs, rhs }, Span::default()))
    }

    pub(crate) fn lower_field_expr(
        &mut self,
        base: hir::HirExpr,
        name: hir::SymbolId,
    ) -> RR<ValueId> {
        let base = self.lower_expr(base)?;
        let field = self
            .symbols
            .get(&name)
            .cloned()
            .unwrap_or_else(|| format!("field_{}", name.0));
        Ok(self.add_value(ValueKind::FieldGet { base, field }, Span::default()))
    }

    pub(crate) fn lower_index_expr(
        &mut self,
        base: hir::HirExpr,
        index: Vec<hir::HirExpr>,
    ) -> RR<ValueId> {
        let span = Span::default();
        let base = self.lower_expr(base)?;
        let mut ids = Vec::with_capacity(index.len());
        for idx_expr in index {
            ids.push(self.lower_expr(idx_expr)?);
        }
        match ids.as_slice() {
            [idx] => Ok(self.add_value(
                ValueKind::Index1D {
                    base,
                    idx: *idx,
                    is_safe: false,
                    is_na_safe: false,
                },
                span,
            )),
            [r, c] => Ok(self.add_value(ValueKind::Index2D { base, r: *r, c: *c }, span)),
            [i, j, k] => Ok(self.add_value(
                ValueKind::Index3D {
                    base,
                    i: *i,
                    j: *j,
                    k: *k,
                },
                span,
            )),
            _ => Err(crate::error::RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E1002,
                crate::error::Stage::Mir,
                "Only 1D/2D/3D indexing is supported",
            )),
        }
    }

    pub(crate) fn lower_call_expr(
        &mut self,
        callee: Box<hir::HirExpr>,
        args: Vec<hir::HirArg>,
        span: Span,
    ) -> RR<ValueId> {
        let tidy_mask_args = self.call_uses_tidy_mask(callee.as_ref());
        let (v_args, arg_names) = self.lower_call_args(args, tidy_mask_args)?;

        match callee.as_ref() {
            hir::HirExpr::Global(sym, _) => self.lower_global_call(*sym, v_args, arg_names, span),
            _ => self.lower_dynamic_call(callee.as_ref().clone(), v_args, arg_names, span),
        }
    }

    pub(crate) fn call_uses_tidy_mask(&self, callee: &hir::HirExpr) -> bool {
        match callee {
            hir::HirExpr::Global(sym, _) => self
                .symbols
                .get(sym)
                .is_some_and(|name| Self::is_tidy_data_mask_call(name)),
            _ => false,
        }
    }

    pub(crate) fn lower_call_args(
        &mut self,
        args: Vec<hir::HirArg>,
        tidy_mask_args: bool,
    ) -> RR<(Vec<ValueId>, Vec<Option<String>>)> {
        let mut v_args = Vec::new();
        let mut arg_names = Vec::new();
        for arg in args {
            match arg {
                hir::HirArg::Pos(expr) => {
                    v_args.push(self.lower_call_arg_value(expr, tidy_mask_args)?);
                    arg_names.push(None);
                }
                hir::HirArg::Named { name, value } => {
                    v_args.push(self.lower_call_arg_value(value, tidy_mask_args)?);
                    arg_names.push(Some(self.call_arg_name(name)));
                }
            }
        }
        Ok((v_args, arg_names))
    }

    pub(crate) fn lower_call_arg_value(
        &mut self,
        expr: hir::HirExpr,
        tidy_mask_args: bool,
    ) -> RR<ValueId> {
        if tidy_mask_args {
            self.with_tidy_mask(|lowerer| lowerer.lower_expr(expr))
        } else {
            self.lower_expr(expr)
        }
    }

    pub(crate) fn call_arg_name(&self, name: hir::SymbolId) -> String {
        self.symbols
            .get(&name)
            .cloned()
            .unwrap_or_else(|| format!("arg_{}", name.0))
    }

    pub(crate) fn lower_global_call(
        &mut self,
        sym: hir::SymbolId,
        args: Vec<ValueId>,
        names: Vec<Option<String>>,
        span: Span,
    ) -> RR<ValueId> {
        let Some(name) = self.symbols.get(&sym).cloned() else {
            return Err(crate::error::RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E1001,
                crate::error::Stage::Mir,
                "invalid unresolved callee symbol".to_string(),
            )
            .at(span));
        };

        if Self::allow_user_builtin_shadowing(&name) && self.known_functions.contains_key(&name) {
            return Ok(self.add_user_function_call_value(sym, args, names, span));
        }
        if name.starts_with("rr_") || Self::is_known_r_builtin_call(&name) {
            return Ok(self.add_named_call_value(name, args, names, span));
        }
        if name == "length" {
            return self.lower_length_call(args, span);
        }
        if Self::is_dynamic_fallback_builtin(&name) {
            self.fn_ir
                .mark_hybrid_interop(Self::hybrid_interop_reason(&name));
            return Ok(self.add_named_call_value(name, args, names, span));
        }
        if Self::is_namespaced_r_call(&name) {
            if !Self::is_supported_package_call(&name) {
                self.fn_ir
                    .mark_opaque_interop_reason(Self::opaque_package_reason(&name));
            }
            return Ok(self.add_named_call_value(name, args, names, span));
        }
        if self.in_tidy_mask() && Self::is_tidy_helper_call(&name) {
            if !Self::is_supported_tidy_helper_call(&name) {
                self.fn_ir
                    .mark_opaque_interop_reason(Self::opaque_tidy_helper_reason(&name));
            }
            return Ok(self.add_named_call_value(name, args, names, span));
        }
        if self.known_functions.contains_key(&name) {
            return Ok(self.add_user_function_call_value(sym, args, names, span));
        }
        Err(self.undefined_function_error(&name, span))
    }

    pub(crate) fn lower_length_call(&mut self, args: Vec<ValueId>, span: Span) -> RR<ValueId> {
        if args.len() != 1 {
            return Err(crate::error::RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E1002,
                crate::error::Stage::Mir,
                format!("builtin 'length' expects 1 argument, got {}", args.len()),
            )
            .at(span));
        }
        Ok(self.add_value(ValueKind::Len { base: args[0] }, span))
    }

    pub(crate) fn lower_dynamic_call(
        &mut self,
        callee: hir::HirExpr,
        args: Vec<ValueId>,
        names: Vec<Option<String>>,
        span: Span,
    ) -> RR<ValueId> {
        let callee_val = self.lower_expr(callee)?;
        let mut dyn_args = Vec::with_capacity(args.len() + 1);
        dyn_args.push(callee_val);
        dyn_args.extend(args);
        let mut dyn_names = Vec::with_capacity(names.len() + 1);
        dyn_names.push(None);
        dyn_names.extend(names);
        Ok(self.add_named_call_value("rr_call_closure".to_string(), dyn_args, dyn_names, span))
    }

    pub(crate) fn add_named_call_value(
        &mut self,
        callee: String,
        args: Vec<ValueId>,
        names: Vec<Option<String>>,
        span: Span,
    ) -> ValueId {
        self.add_value(
            ValueKind::Call {
                callee,
                args,
                names,
            },
            span,
        )
    }

    pub(crate) fn add_user_function_call_value(
        &mut self,
        sym: hir::SymbolId,
        args: Vec<ValueId>,
        names: Vec<Option<String>>,
        span: Span,
    ) -> ValueId {
        self.add_named_call_value(format!("Sym_{}", sym.0), args, names, span)
    }

    pub(crate) fn undefined_function_error(
        &self,
        name: &str,
        span: Span,
    ) -> crate::error::RRException {
        let mut err = crate::error::RRException::new(
            "RR.SemanticError",
            crate::error::RRCode::E1001,
            crate::error::Stage::Mir,
            format!("undefined function '{}'", name),
        )
        .at(span)
        .note("Define or import the function before calling it.");
        if let Some(suggestion) = self.suggest_function_name(name) {
            err = err.help(suggestion);
        }
        err
    }

    pub(crate) fn is_known_r_builtin_call(name: &str) -> bool {
        matches!(
            name,
            "seq_along"
                | "seq_len"
                | "c"
                | "list"
                | "sum"
                | "mean"
                | "var"
                | "prod"
                | "min"
                | "max"
                | "abs"
                | "sqrt"
                | "sin"
                | "cos"
                | "tan"
                | "asin"
                | "acos"
                | "atan"
                | "atan2"
                | "sinh"
                | "cosh"
                | "tanh"
                | "log"
                | "log10"
                | "log2"
                | "exp"
                | "sign"
                | "gamma"
                | "lgamma"
                | "floor"
                | "ceiling"
                | "trunc"
                | "round"
                | "pmax"
                | "pmin"
                | "print"
                | "paste"
                | "paste0"
                | "sprintf"
                | "cat"
                | "names"
                | "rownames"
                | "colnames"
                | "sort"
                | "order"
                | "match"
                | "unique"
                | "duplicated"
                | "anyDuplicated"
                | "any"
                | "all"
                | "which"
                | "is.na"
                | "is.finite"
                | "numeric"
                | "character"
                | "logical"
                | "integer"
                | "double"
                | "rep"
                | "rep.int"
                | "vector"
                | "matrix"
                | "dim"
                | "dimnames"
                | "nrow"
                | "ncol"
                | "colSums"
                | "rowSums"
                | "crossprod"
                | "tcrossprod"
                | "t"
                | "diag"
                | "rbind"
                | "cbind"
        )
    }

    pub(crate) fn lower_if_expr(
        &mut self,
        cond: hir::HirExpr,
        then_expr: hir::HirExpr,
        else_expr: hir::HirExpr,
    ) -> RR<ValueId> {
        let cond_val = self.lower_expr(cond)?;
        let then_bb = self.fn_ir.add_block();
        let else_bb = self.fn_ir.add_block();
        let merge_bb = self.fn_ir.add_block();

        self.add_pred(then_bb, self.curr_block);
        self.add_pred(else_bb, self.curr_block);
        self.terminate(Terminator::If {
            cond: cond_val,
            then_bb,
            else_bb,
        });

        let (then_val, then_end_bb) = self.lower_if_branch(then_bb, then_expr, merge_bb)?;
        let (else_val, else_end_bb) = self.lower_if_branch(else_bb, else_expr, merge_bb)?;

        self.curr_block = merge_bb;
        self.seal_block(merge_bb)?;
        let phi_val = self.add_value(
            ValueKind::Phi {
                args: vec![(then_val, then_end_bb), (else_val, else_end_bb)],
            },
            Span::default(),
        );
        if let Some(v) = self.fn_ir.values.get_mut(phi_val) {
            v.phi_block = Some(merge_bb);
        }

        Ok(phi_val)
    }

    pub(crate) fn lower_if_branch(
        &mut self,
        branch_bb: BlockId,
        expr: hir::HirExpr,
        merge_bb: BlockId,
    ) -> RR<(ValueId, BlockId)> {
        self.curr_block = branch_bb;
        self.seal_block(branch_bb)?;
        let value = self.lower_expr(expr)?;
        if !self.is_terminated(branch_bb) {
            self.add_pred(merge_bb, self.curr_block);
            self.terminate(Terminator::Goto(merge_bb));
        }
        Ok((value, self.curr_block))
    }

    pub(crate) fn lower_vector_literal(&mut self, elems: Vec<hir::HirExpr>) -> RR<ValueId> {
        let mut vals = Vec::new();
        for expr in elems {
            vals.push(self.lower_expr(expr)?);
        }
        let names = vec![None; vals.len()];
        Ok(self.add_value(
            ValueKind::Call {
                callee: "c".to_string(),
                args: vals,
                names,
            },
            Span::default(),
        ))
    }

    pub(crate) fn lower_list_literal(
        &mut self,
        fields: Vec<(hir::SymbolId, hir::HirExpr)>,
    ) -> RR<ValueId> {
        let mut vals = Vec::new();
        for (sym, expr) in fields {
            let field = self
                .symbols
                .get(&sym)
                .cloned()
                .unwrap_or_else(|| format!("field_{}", sym.0));
            vals.push((field, self.lower_expr(expr)?));
        }
        Ok(self.add_value(ValueKind::RecordLit { fields: vals }, Span::default()))
    }

    pub(crate) fn lower_range_expr(
        &mut self,
        start: hir::HirExpr,
        end: hir::HirExpr,
    ) -> RR<ValueId> {
        let start = self.lower_expr(start)?;
        let end = self.lower_expr(end)?;
        Ok(self.add_value(ValueKind::Range { start, end }, Span::default()))
    }

    pub(crate) fn unsupported_expr_error(expr: hir::HirExpr) -> crate::error::RRException {
        crate::error::RRException::new(
            "RR.SemanticError",
            crate::error::RRCode::E1002,
            crate::error::Stage::Mir,
            format!("unsupported expression in MIR lowering: {:?}", expr),
        )
        .at(Span::default())
        .push_frame("mir::lower_hir::lower_expr/1", Some(Span::default()))
    }
}
