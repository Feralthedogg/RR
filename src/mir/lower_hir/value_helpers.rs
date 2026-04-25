impl<'a> MirLowerer<'a> {
    // Helpers

    fn add_void_val(&mut self, span: Span) -> ValueId {
        self.add_value(ValueKind::Const(Lit::Null), span)
    }

    fn add_null_val(&mut self, span: Span) -> ValueId {
        self.add_value(ValueKind::Const(Lit::Null), span)
    }

    fn add_bool_val(&mut self, b: bool, span: Span) -> ValueId {
        self.add_value(ValueKind::Const(Lit::Bool(b)), span)
    }

    fn add_int_val(&mut self, n: i64, span: Span) -> ValueId {
        self.add_value(ValueKind::Const(Lit::Int(n)), span)
    }

    fn add_bin_bool(&mut self, op: BinOp, lhs: ValueId, rhs: ValueId, span: Span) -> ValueId {
        self.add_value(ValueKind::Binary { op, lhs, rhs }, span)
    }

    fn add_call_value(&mut self, callee: &str, args: Vec<ValueId>, span: Span) -> ValueId {
        let names = vec![None; args.len()];
        self.add_value(
            ValueKind::Call {
                callee: callee.to_string(),
                args,
                names,
            },
            span,
        )
    }

    fn symbol_name(&self, sym: hir::SymbolId) -> String {
        self.symbols
            .get(&sym)
            .cloned()
            .unwrap_or_else(|| format!("field_{}", sym.0))
    }

    fn terminate_and_detach(&mut self, term: Terminator) {
        let from = self.curr_block;
        self.terminate(term);
        let dead_bb = self.fn_ir.add_block();
        if let Some(defs_here) = self.defs.get(&from).cloned() {
            self.defs.insert(dead_bb, defs_here);
        } else {
            self.defs.insert(dead_bb, FxHashMap::default());
        }
        self.curr_block = dead_bb;
    }

    fn terminate(&mut self, term: Terminator) {
        self.fn_ir.blocks[self.curr_block].term = term;
    }

    fn is_terminated(&self, b: BlockId) -> bool {
        !matches!(self.fn_ir.blocks[b].term, Terminator::Unreachable)
    }

    fn map_binop(&self, op: hir::HirBinOp) -> BinOp {
        match op {
            hir::HirBinOp::Add => BinOp::Add,
            hir::HirBinOp::Sub => BinOp::Sub,
            hir::HirBinOp::Mul => BinOp::Mul,
            hir::HirBinOp::Div => BinOp::Div,
            hir::HirBinOp::Mod => BinOp::Mod,
            hir::HirBinOp::MatMul => BinOp::MatMul,
            hir::HirBinOp::Eq => BinOp::Eq,
            hir::HirBinOp::Ne => BinOp::Ne,
            hir::HirBinOp::Lt => BinOp::Lt,
            hir::HirBinOp::Le => BinOp::Le,
            hir::HirBinOp::Gt => BinOp::Gt,
            hir::HirBinOp::Ge => BinOp::Ge,
            hir::HirBinOp::And => BinOp::And,
            hir::HirBinOp::Or => BinOp::Or,
            // HirBinOp might have more variants?
        }
    }
}
