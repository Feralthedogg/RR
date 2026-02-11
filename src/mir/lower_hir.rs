use crate::error::RR;
use crate::hir::def as hir;
use crate::mir::flow::Facts;
use crate::mir::*;
use crate::syntax::ast::{BinOp, Lit};
use crate::utils::Span;
use std::collections::HashMap;

#[derive(Clone, Copy)]
struct LoopTargets {
    break_bb: BlockId,
    continue_bb: BlockId,
    continue_step: Option<(hir::LocalId, ValueId)>,
}

pub struct MirLowerer<'a> {
    fn_ir: FnIR,

    // SSA construction state.
    curr_block: BlockId,

    // Current definitions per block (sealed SSA construction).
    defs: HashMap<BlockId, HashMap<hir::LocalId, ValueId>>,

    // Deferred phi operands for unsealed blocks.
    incomplete_phis: HashMap<BlockId, Vec<(hir::LocalId, ValueId)>>,
    sealed_blocks: std::collections::HashSet<BlockId>,
    // Predecessor map for SSA reads.
    preds: HashMap<BlockId, Vec<BlockId>>,

    // Name mapping for codegen.
    var_names: HashMap<hir::LocalId, String>,

    // Symbol table snapshot.
    symbols: &'a HashMap<hir::SymbolId, String>,
    known_functions: &'a HashMap<String, usize>,
    loop_stack: Vec<LoopTargets>,
}

impl<'a> MirLowerer<'a> {
    pub fn new(
        name: String,
        params: Vec<String>,
        var_names: HashMap<hir::LocalId, String>,
        symbols: &'a HashMap<hir::SymbolId, String>,
        known_functions: &'a HashMap<String, usize>,
    ) -> Self {
        let mut fn_ir = FnIR::new(name, params.clone());
        let entry = fn_ir.add_block();
        let body_head = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = body_head;

        // Init defs map for entry
        let mut defs = HashMap::new();
        defs.insert(entry, HashMap::new());
        defs.insert(body_head, HashMap::new());

        Self {
            fn_ir,
            curr_block: entry,
            defs,
            incomplete_phis: HashMap::new(),
            sealed_blocks: std::collections::HashSet::new(),
            preds: HashMap::new(),
            var_names,
            symbols,
            known_functions,
            loop_stack: Vec::new(),
        }
    }

    // Core Helpers
    fn add_pred(&mut self, target: BlockId, pred: BlockId) {
        self.preds.entry(target).or_insert_with(Vec::new).push(pred);
    }

    // Standardize Value Addition
    fn add_value(&mut self, kind: ValueKind, span: Span) -> ValueId {
        self.fn_ir.add_value(kind, span, Facts::empty(), None)
    }

    fn add_value_with_name(
        &mut self,
        kind: ValueKind,
        span: Span,
        var_name: Option<String>,
    ) -> ValueId {
        self.fn_ir.add_value(kind, span, Facts::empty(), var_name)
    }

    // Core Helpers
    fn define_var_at(
        &mut self,
        block: BlockId,
        var: hir::LocalId,
        val: ValueId,
        emit_assign: bool,
    ) {
        // Update origin_var of the value to the name of this local
        let name = self.var_names.get(&var).cloned();
        if let Some(n) = &name {
            if let Some(v) = self.fn_ir.values.get_mut(val) {
                if v.origin_var.is_none() {
                    v.origin_var = Some(n.clone());
                }
            }
            if emit_assign {
                // Emit assignment instruction so it exists in R code
                self.fn_ir.blocks[block].instrs.push(Instr::Assign {
                    dst: n.clone(),
                    src: val,
                    span: Span::default(),
                });
            }
        }

        self.defs.entry(block).or_default().insert(var, val);
    }

    fn write_var(&mut self, var: hir::LocalId, val: ValueId) {
        self.define_var_at(self.curr_block, var, val, true);
    }

    fn read_var(&mut self, var: hir::LocalId, block: BlockId) -> RR<ValueId> {
        if let Some(m) = self.defs.get(&block) {
            if let Some(&v) = m.get(&var) {
                return Ok(v);
            }
        }
        // Not found in local, look in predecessors
        self.read_var_recursive(var, block)
    }

    // Sealed Block SSA Construction (Braun et al.)

    fn seal_block(&mut self, block: BlockId) -> RR<()> {
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

    fn read_var_recursive(&mut self, var: hir::LocalId, block: BlockId) -> RR<ValueId> {
        if !self.sealed_blocks.contains(&block) {
            // Create a placeholder phi and resolve operands when the block is sealed.
            let phi = self.add_phi_placeholder(block, Span::default());
            self.incomplete_phis
                .entry(block)
                .or_default()
                .push((var.clone(), phi));
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
            return Err(crate::error::RRException::new(
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
            .note("Declare the variable with let before use."));
        } else if preds.len() == 1 {
            // Optimize: No phi needed, just look in pred
            self.read_var(var.clone(), preds[0])
        } else {
            // Multiple predecessors require a phi.
            let phi = self.add_phi_placeholder(block, Span::default());
            // Break cycles with a Phi placeholder, but don't emit an assignment yet.
            self.define_var_at(block, var.clone(), phi, false);
            self.add_phi_operands(block, var, phi)?;
            Ok(phi)
        }
    }

    fn add_phi_operands(&mut self, block: BlockId, var: hir::LocalId, phi_val: ValueId) -> RR<()> {
        // Collect operands from all preds
        let preds = self.preds.get(&block).cloned().unwrap_or_default();
        let mut new_args = Vec::new();
        for pred in preds {
            let val = self.read_var(var.clone(), pred)?;
            new_args.push((val, pred));
        }

        // Update Phi instruction
        if let Some(val) = self.fn_ir.values.get_mut(phi_val) {
            if let ValueKind::Phi { ref mut args } = val.kind {
                *args = new_args;
            } else {
                panic!("Value {:?} is not a Phi", phi_val);
            }
        } else {
            panic!("Value {:?} not found", phi_val);
        }

        // Trivial Phi elimination could be done here (if all args same)
        Ok(())
    }

    fn add_phi_placeholder(&mut self, _block: BlockId, span: Span) -> ValueId {
        let id = self.add_value(ValueKind::Phi { args: vec![] }, span);
        if let Some(v) = self.fn_ir.values.get_mut(id) {
            v.phi_block = Some(_block);
        }
        id
    }

    // Call update: terminate must track preds

    pub fn lower_fn(mut self, f: hir::HirFn) -> RR<FnIR> {
        // 1. Bind parameters in the entry block
        for (i, param) in f.params.iter().enumerate() {
            let param_name = self.symbols[&param.name].clone(); // Clone early to avoid borrow conflict
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
            } else {
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

    fn lower_block(&mut self, blk: hir::HirBlock) -> RR<ValueId> {
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

    fn lower_block_effects(&mut self, blk: hir::HirBlock) -> RR<()> {
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

    fn lower_stmt(&mut self, stmt: hir::HirStmt) -> RR<()> {
        match stmt {
            hir::HirStmt::Let {
                local, init, span, ..
            } => {
                let val = if let Some(e) = init {
                    self.lower_expr(e)?
                } else {
                    self.add_null_val(span) // Default init
                };
                self.write_var(local, val);
            }
            hir::HirStmt::Assign {
                target,
                value,
                span,
            } => {
                let v = self.lower_expr(value)?;
                match target {
                    hir::HirLValue::Local(l) => self.write_var(l, v),
                    hir::HirLValue::Index { base, index } => {
                        let base_id = self.lower_expr(base)?;
                        let mut ids = Vec::with_capacity(index.len());
                        for idx_expr in index {
                            ids.push(self.lower_expr(idx_expr)?);
                        }
                        match ids.as_slice() {
                            [idx] => {
                                self.fn_ir.blocks[self.curr_block].instrs.push(
                                    Instr::StoreIndex1D {
                                        base: base_id,
                                        idx: *idx,
                                        val: v,
                                        is_safe: false,
                                        is_na_safe: false,
                                        is_vector: false,
                                        span,
                                    },
                                );
                            }
                            [r, c] => {
                                self.fn_ir.blocks[self.curr_block].instrs.push(
                                    Instr::StoreIndex2D {
                                        base: base_id,
                                        r: *r,
                                        c: *c,
                                        val: v,
                                        span,
                                    },
                                );
                            }
                            _ => {
                                return Err(crate::error::RRException::new(
                                    "RR.SemanticError",
                                    crate::error::RRCode::E1002,
                                    crate::error::Stage::Mir,
                                    "Only 1D/2D indexing is supported",
                                ));
                            }
                        }
                    }
                    hir::HirLValue::Field { base, name } => {
                        let field_name = self
                            .symbols
                            .get(&name)
                            .cloned()
                            .unwrap_or_else(|| format!("field_{}", name.0));
                        let field_lit =
                            self.add_value(ValueKind::Const(Lit::Str(field_name)), span);
                        let base_clone = base.clone();
                        let base_id = self.lower_expr(base)?;
                        let set_id = self.add_value(
                            ValueKind::Call {
                                callee: "rr_field_set".to_string(),
                                args: vec![base_id, field_lit, v],
                                names: vec![None, None, None],
                            },
                            span,
                        );
                        match base_clone {
                            hir::HirExpr::Local(lid) => {
                                self.write_var(lid, set_id);
                            }
                            hir::HirExpr::Global(sym) => {
                                if let Some(dst_name) = self.symbols.get(&sym).cloned() {
                                    self.fn_ir.blocks[self.curr_block]
                                        .instrs
                                        .push(Instr::Assign {
                                            dst: dst_name,
                                            src: set_id,
                                            span,
                                        });
                                } else {
                                    self.fn_ir.blocks[self.curr_block]
                                        .instrs
                                        .push(Instr::Eval { val: set_id, span });
                                }
                            }
                            _ => {
                                // Fallback: preserve side effect when base isn't a writable symbol.
                                self.fn_ir.blocks[self.curr_block]
                                    .instrs
                                    .push(Instr::Eval { val: set_id, span });
                            }
                        }
                    }
                }
            }
            hir::HirStmt::Expr { expr, .. } => {
                self.lower_expr(expr)?;
            }
            hir::HirStmt::Return { value, span: _span } => {
                let v = if let Some(e) = value {
                    Some(self.lower_expr(e)?)
                } else {
                    None
                };
                self.terminate_and_detach(Terminator::Return(v));
            }
            hir::HirStmt::If {
                cond,
                then_blk,
                else_blk,
                span: _span,
            } => {
                let cond_val = self.lower_expr(cond)?;
                let pre_if_bb = self.curr_block;

                let then_bb = self.fn_ir.add_block();
                let else_bb = self.fn_ir.add_block();
                let join_bb = self.fn_ir.add_block();

                self.terminate(Terminator::If {
                    cond: cond_val,
                    then_bb,
                    else_bb,
                });

                // Then branch
                self.add_pred(then_bb, pre_if_bb);
                self.curr_block = then_bb;
                self.seal_block(then_bb)?;
                self.lower_block_effects(then_blk)?;
                if !self.is_terminated(self.curr_block) {
                    self.add_pred(join_bb, self.curr_block);
                    self.terminate(Terminator::Goto(join_bb));
                }

                // Else branch
                self.add_pred(else_bb, pre_if_bb);
                self.curr_block = else_bb;
                self.seal_block(else_bb)?;
                if let Some(eb) = else_blk {
                    self.lower_block_effects(eb)?;
                }
                if !self.is_terminated(self.curr_block) {
                    self.add_pred(join_bb, self.curr_block);
                    self.terminate(Terminator::Goto(join_bb));
                }

                self.curr_block = join_bb;
                self.seal_block(join_bb)?;
            }
            hir::HirStmt::While {
                cond,
                body,
                span: _span,
            } => {
                let header_bb = self.fn_ir.add_block();
                let body_bb = self.fn_ir.add_block();
                let exit_bb = self.fn_ir.add_block();

                self.add_pred(header_bb, self.curr_block);
                self.terminate(Terminator::Goto(header_bb));

                self.curr_block = header_bb;
                let cond_val = self.lower_expr(cond)?;
                self.terminate(Terminator::If {
                    cond: cond_val,
                    then_bb: body_bb,
                    else_bb: exit_bb,
                });

                self.add_pred(body_bb, header_bb);
                self.curr_block = body_bb;
                self.seal_block(body_bb)?;
                self.loop_stack.push(LoopTargets {
                    break_bb: exit_bb,
                    continue_bb: header_bb,
                    continue_step: None,
                });
                self.lower_block_effects(body)?;
                self.loop_stack.pop();
                let curr_reachable = self
                    .preds
                    .get(&self.curr_block)
                    .map(|ps| !ps.is_empty())
                    .unwrap_or(false);
                if !self.is_terminated(self.curr_block) && curr_reachable {
                    self.add_pred(header_bb, self.curr_block);
                    self.terminate(Terminator::Goto(header_bb));
                }

                self.seal_block(header_bb)?;
                self.add_pred(exit_bb, header_bb);
                self.curr_block = exit_bb;
                self.seal_block(exit_bb)?;
            }
            hir::HirStmt::For { iter, body, span } => {
                self.lower_for(iter, body, span)?;
            }
            hir::HirStmt::Break { span } => {
                if let Some(targets) = self.loop_stack.last().copied() {
                    self.add_pred(targets.break_bb, self.curr_block);
                    self.terminate_and_detach(Terminator::Goto(targets.break_bb));
                } else {
                    return Err(crate::error::RRException::new(
                        "RR.SemanticError",
                        crate::error::RRCode::E1002,
                        crate::error::Stage::Mir,
                        "break used outside of a loop".to_string(),
                    )
                    .at(span));
                }
            }
            hir::HirStmt::Next { span } => {
                if let Some(targets) = self.loop_stack.last().copied() {
                    if let Some((var, iv)) = targets.continue_step {
                        let one = self.add_int_val(1, span);
                        let next_iv = self.add_value(
                            ValueKind::Binary {
                                op: BinOp::Add,
                                lhs: iv,
                                rhs: one,
                            },
                            span,
                        );
                        self.write_var(var, next_iv);
                    }
                    self.add_pred(targets.continue_bb, self.curr_block);
                    self.terminate_and_detach(Terminator::Goto(targets.continue_bb));
                } else {
                    return Err(crate::error::RRException::new(
                        "RR.SemanticError",
                        crate::error::RRCode::E1002,
                        crate::error::Stage::Mir,
                        "next used outside of a loop".to_string(),
                    )
                    .at(span));
                }
            }
        }
        Ok(())
    }

    fn lower_expr(&mut self, expr: hir::HirExpr) -> RR<ValueId> {
        // println!("DEBUG: Lowering Expr: {:?}", expr);
        match expr {
            hir::HirExpr::Lit(l) => {
                let al = match l {
                    hir::HirLit::Int(i) => Lit::Int(i),
                    hir::HirLit::Double(f) => Lit::Float(f),
                    hir::HirLit::Char(s) => Lit::Str(s),
                    hir::HirLit::Bool(b) => Lit::Bool(b),
                    hir::HirLit::NA => Lit::Na,
                    hir::HirLit::Null => Lit::Null,
                };
                Ok(self.add_value(ValueKind::Const(al), Span::default()))
            }
            hir::HirExpr::Local(l) => self.read_var(l, self.curr_block),
            hir::HirExpr::Global(sym) => {
                let raw_name = self
                    .symbols
                    .get(&sym)
                    .cloned()
                    .unwrap_or_else(|| format!("Sym_{}", sym.0));
                let name = if self.known_functions.contains_key(&raw_name) {
                    format!("Sym_{}", sym.0)
                } else {
                    raw_name
                };
                Ok(self.add_value_with_name(
                    ValueKind::Load { var: name.clone() },
                    Span::default(),
                    Some(name),
                ))
            }
            hir::HirExpr::Unary { op, expr } => {
                let rhs = self.lower_expr(*expr)?;
                let op = match op {
                    hir::HirUnOp::Not => crate::syntax::ast::UnaryOp::Not,
                    hir::HirUnOp::Neg => crate::syntax::ast::UnaryOp::Neg,
                };
                Ok(self.add_value(ValueKind::Unary { op, rhs }, Span::default()))
            }
            hir::HirExpr::Binary { op, lhs, rhs } => {
                let l = self.lower_expr(*lhs)?;
                let r = self.lower_expr(*rhs)?;
                let op = self.map_binop(op);
                Ok(self.add_value(ValueKind::Binary { op, lhs: l, rhs: r }, Span::default()))
            }
            hir::HirExpr::Field { base, name } => {
                let b = self.lower_expr(*base)?;
                let field_name = self
                    .symbols
                    .get(&name)
                    .cloned()
                    .unwrap_or_else(|| format!("field_{}", name.0));
                let field_lit =
                    self.add_value(ValueKind::Const(Lit::Str(field_name)), Span::default());
                Ok(self.add_value(
                    ValueKind::Call {
                        callee: "rr_field_get".to_string(),
                        args: vec![b, field_lit],
                        names: vec![None, None],
                    },
                    Span::default(),
                ))
            }
            hir::HirExpr::Index { base, index } => {
                let span = Span::default();
                let base_id = self.lower_expr(*base)?;
                let mut ids = Vec::with_capacity(index.len());
                for idx_expr in index {
                    ids.push(self.lower_expr(idx_expr)?);
                }
                match ids.as_slice() {
                    [idx] => Ok(self.fn_ir.add_value(
                        ValueKind::Index1D {
                            base: base_id,
                            idx: *idx,
                            is_safe: false,
                            is_na_safe: false,
                        },
                        span,
                        Facts::empty(),
                        None,
                    )),
                    [r, c] => Ok(self.fn_ir.add_value(
                        ValueKind::Index2D {
                            base: base_id,
                            r: *r,
                            c: *c,
                        },
                        span,
                        Facts::empty(),
                        None,
                    )),
                    _ => Err(crate::error::RRException::new(
                        "RR.SemanticError",
                        crate::error::RRCode::E1002,
                        crate::error::Stage::Mir,
                        "Only 1D/2D indexing is supported",
                    )),
                }
            }
            hir::HirExpr::Block(blk) => self.lower_block(blk),

            hir::HirExpr::Call(hir::HirCall { callee, args, span }) => {
                let mut v_args = Vec::new();
                let mut arg_names: Vec<Option<String>> = Vec::new();
                for arg in args {
                    match arg {
                        hir::HirArg::Pos(e) => {
                            v_args.push(self.lower_expr(e)?);
                            arg_names.push(None);
                        }
                        hir::HirArg::Named { name, value } => {
                            v_args.push(self.lower_expr(value)?);
                            let n = self
                                .symbols
                                .get(&name)
                                .cloned()
                                .unwrap_or_else(|| format!("arg_{}", name.0));
                            arg_names.push(Some(n));
                        }
                    }
                }

                match callee.as_ref() {
                    hir::HirExpr::Global(sym) => {
                        if let Some(name) = self.symbols.get(sym) {
                            if name == "length" {
                                if v_args.len() != 1 {
                                    return Err(crate::error::RRException::new(
                                        "RR.SemanticError",
                                        crate::error::RRCode::E1002,
                                        crate::error::Stage::Mir,
                                        format!(
                                            "builtin '{}' expects 1 argument, got {}",
                                            name,
                                            v_args.len()
                                        ),
                                    )
                                    .at(span));
                                }
                                return Ok(self.add_value(ValueKind::Len { base: v_args[0] }, span));
                            }
                            if name.starts_with("rr_") {
                                return Ok(self.fn_ir.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                    Facts::empty(),
                                    None,
                                ));
                            }
                            // Known builtins should keep their original names.
                            if matches!(
                                name.as_str(),
                                "seq_along"
                                    | "seq_len"
                                    | "c"
                                    | "list"
                                    | "sum"
                                    | "mean"
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
                                    | "numeric"
                                    | "rep.int"
                                    | "vector"
                                    | "matrix"
                                    | "colSums"
                                    | "rowSums"
                                    | "crossprod"
                                    | "tcrossprod"
                            ) {
                                return Ok(self.fn_ir.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                    Facts::empty(),
                                    None,
                                ));
                            }
                            if Self::is_dynamic_fallback_builtin(name) {
                                self.fn_ir
                                    .mark_unsupported_dynamic(format!("dynamic builtin: {}", name));
                                return Ok(self.fn_ir.add_value(
                                    ValueKind::Call {
                                        callee: name.clone(),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                    Facts::empty(),
                                    None,
                                ));
                            }
                            if let Some(expected) = self.known_functions.get(name) {
                                if *expected != v_args.len() {
                                    return Err(crate::error::RRException::new(
                                        "RR.SemanticError",
                                        crate::error::RRCode::E1002,
                                        crate::error::Stage::Mir,
                                        format!(
                                            "function '{}' expects {} argument(s), got {}",
                                            name,
                                            expected,
                                            v_args.len()
                                        ),
                                    )
                                    .at(span));
                                }
                                return Ok(self.fn_ir.add_value(
                                    ValueKind::Call {
                                        callee: format!("Sym_{}", sym.0),
                                        args: v_args,
                                        names: arg_names,
                                    },
                                    span,
                                    Facts::empty(),
                                    None,
                                ));
                            }
                            return Err(crate::error::RRException::new(
                                "RR.SemanticError",
                                crate::error::RRCode::E1001,
                                crate::error::Stage::Mir,
                                format!("undefined function '{}'", name),
                            )
                            .at(span)
                            .note("Define or import the function before calling it."));
                        }
                        return Err(crate::error::RRException::new(
                            "RR.SemanticError",
                            crate::error::RRCode::E1001,
                            crate::error::Stage::Mir,
                            "invalid unresolved callee symbol".to_string(),
                        )
                        .at(span));
                    }
                    _ => {
                        let callee_val = self.lower_expr(callee.as_ref().clone())?;
                        let mut dyn_args = Vec::with_capacity(v_args.len() + 1);
                        dyn_args.push(callee_val);
                        dyn_args.extend(v_args);
                        let mut dyn_names = Vec::with_capacity(arg_names.len() + 1);
                        dyn_names.push(None);
                        dyn_names.extend(arg_names);
                        return Ok(self.fn_ir.add_value(
                            ValueKind::Call {
                                callee: "rr_call_closure".to_string(),
                                args: dyn_args,
                                names: dyn_names,
                            },
                            span,
                            Facts::empty(),
                            None,
                        ));
                    }
                }
            }
            hir::HirExpr::IfExpr {
                cond,
                then_expr,
                else_expr,
            } => {
                let cond_val = self.lower_expr(*cond)?;

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

                // Then Branch
                self.curr_block = then_bb;
                // Seal Then? Only 1 pred.
                self.seal_block(then_bb)?;
                let then_val = self.lower_expr(*then_expr)?;
                if !self.is_terminated(then_bb) {
                    self.add_pred(merge_bb, self.curr_block);
                    self.terminate(Terminator::Goto(merge_bb));
                }
                let then_end_bb = self.curr_block;

                // Else Branch
                self.curr_block = else_bb;
                self.seal_block(else_bb)?;
                let else_val = self.lower_expr(*else_expr)?;
                if !self.is_terminated(else_bb) {
                    self.add_pred(merge_bb, self.curr_block);
                    self.terminate(Terminator::Goto(merge_bb));
                }
                let else_end_bb = self.curr_block;

                // Merge Branch
                self.curr_block = merge_bb;
                self.seal_block(merge_bb)?;

                // Phi for result value
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
            hir::HirExpr::VectorLit(elems) => {
                let mut vals = Vec::new();
                for e in elems {
                    vals.push(self.lower_expr(e)?);
                }
                // Lower vector literals via R's `c(...)` constructor.
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
            hir::HirExpr::ListLit(fields) => {
                // Preserve names via a runtime helper:
                // rr_named_list("a", v1, "b", v2, ...)
                let mut vals = Vec::new();
                for (sym, e) in fields {
                    let key = self
                        .symbols
                        .get(&sym)
                        .cloned()
                        .unwrap_or_else(|| format!("field_{}", sym.0));
                    let k = self.add_value(ValueKind::Const(Lit::Str(key)), Span::default());
                    vals.push(k);
                    vals.push(self.lower_expr(e)?);
                }
                let names = vec![None; vals.len()];
                Ok(self.add_value(
                    ValueKind::Call {
                        callee: "rr_named_list".to_string(),
                        args: vals,
                        names,
                    },
                    Span::default(),
                ))
            }
            hir::HirExpr::Range { start, end } => {
                let s = self.lower_expr(*start)?;
                let e = self.lower_expr(*end)?;
                Ok(self.add_value(ValueKind::Range { start: s, end: e }, Span::default()))
            }
            hir::HirExpr::Try(inner) => {
                // RR v1: try-postfix lowers to value propagation in MIR.
                // Runtime error propagation is still handled by R semantics.
                self.lower_expr(*inner)
            }
            hir::HirExpr::Match { scrut, arms } => self.lower_match_expr(*scrut, arms),
            hir::HirExpr::Column(name) => {
                // Tidy Column -> Constant String
                // We rely on runtime to distinguish column ref vs string if needed.
                // But typically @col evaluates to the string "col".
                Ok(self.add_value(ValueKind::Const(Lit::Str(name)), Span::default()))
            }
            hir::HirExpr::Unquote(e) => self.lower_expr(*e),
            _ => Err(crate::error::RRException::new(
                "RR.SemanticError",
                crate::error::RRCode::E1002,
                crate::error::Stage::Mir,
                format!("unsupported expression in MIR lowering: {:?}", expr),
            )
            .at(Span::default())
            .push_frame("mir::lower_hir::lower_expr/1", Some(Span::default()))),
        }
    }

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
            self.defs.insert(dead_bb, HashMap::new());
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

    fn is_dynamic_fallback_builtin(name: &str) -> bool {
        matches!(
            name,
            "eval"
                | "parse"
                | "get"
                | "assign"
                | "exists"
                | "mget"
                | "rm"
                | "ls"
                | "parent.frame"
                | "environment"
                | "sys.frame"
                | "sys.call"
                | "do.call"
        )
    }

    fn lower_match_expr(
        &mut self,
        scrut: hir::HirExpr,
        arms: Vec<hir::HirMatchArm>,
    ) -> RR<ValueId> {
        let span = Span::default();
        let scrut_val = self.lower_expr(scrut)?;
        if arms.is_empty() {
            return Ok(self.add_value(ValueKind::Const(Lit::Null), span));
        }

        let merge_bb = self.fn_ir.add_block();
        let mut arm_results: Vec<(ValueId, BlockId)> = Vec::new();
        let mut test_bb = self.curr_block;
        let arm_len = arms.len();

        for (i, arm) in arms.into_iter().enumerate() {
            self.curr_block = test_bb;

            let cond = self.lower_match_pat_cond(scrut_val, &arm.pat, arm.span)?;
            let arm_bb = self.fn_ir.add_block();
            let fail_bb = self.fn_ir.add_block();

            if let Some(guard_expr) = arm.guard {
                let guard_bb = self.fn_ir.add_block();
                self.terminate(Terminator::If {
                    cond,
                    then_bb: guard_bb,
                    else_bb: fail_bb,
                });
                self.add_pred(guard_bb, test_bb);
                self.add_pred(fail_bb, test_bb);

                self.curr_block = guard_bb;
                self.seal_block(guard_bb)?;
                self.bind_match_pattern(scrut_val, &arm.pat, arm.span)?;
                let guard_val = self.lower_expr(guard_expr)?;
                self.terminate(Terminator::If {
                    cond: guard_val,
                    then_bb: arm_bb,
                    else_bb: fail_bb,
                });
                self.add_pred(arm_bb, guard_bb);
                self.add_pred(fail_bb, guard_bb);
            } else {
                self.terminate(Terminator::If {
                    cond,
                    then_bb: arm_bb,
                    else_bb: fail_bb,
                });
                self.add_pred(arm_bb, test_bb);
                self.add_pred(fail_bb, test_bb);
            }

            self.curr_block = arm_bb;
            self.seal_block(arm_bb)?;
            self.bind_match_pattern(scrut_val, &arm.pat, arm.span)?;
            let arm_val = self.lower_expr(arm.body)?;
            let arm_end_bb = self.curr_block;
            if !self.is_terminated(self.curr_block) {
                self.add_pred(merge_bb, self.curr_block);
                self.terminate(Terminator::Goto(merge_bb));
            }
            arm_results.push((arm_val, arm_end_bb));

            test_bb = fail_bb;
            self.seal_block(fail_bb)?;

            // If this is the last arm, keep flowing to default no-match path.
            if i + 1 == arm_len {
                self.curr_block = test_bb;
            }
        }

        // No arm matched: evaluate to NULL by default.
        self.curr_block = test_bb;
        let default_val = self.add_value(ValueKind::Const(Lit::Null), span);
        let default_end_bb = self.curr_block;
        if !self.is_terminated(self.curr_block) {
            self.add_pred(merge_bb, self.curr_block);
            self.terminate(Terminator::Goto(merge_bb));
        }
        arm_results.push((default_val, default_end_bb));

        self.curr_block = merge_bb;
        self.seal_block(merge_bb)?;
        let phi = self.add_value(ValueKind::Phi { args: arm_results }, span);
        if let Some(v) = self.fn_ir.values.get_mut(phi) {
            v.phi_block = Some(merge_bb);
        }
        Ok(phi)
    }

    fn lower_match_pat_cond(
        &mut self,
        scrut: ValueId,
        pat: &hir::HirPat,
        span: Span,
    ) -> RR<ValueId> {
        match pat {
            hir::HirPat::Wild | hir::HirPat::Bind { .. } => Ok(self.add_bool_val(true, span)),
            hir::HirPat::Lit(l) => {
                let rhs = self.add_value(ValueKind::Const(Self::hir_lit_to_lit(l)), span);
                Ok(self.add_value(
                    ValueKind::Binary {
                        op: BinOp::Eq,
                        lhs: scrut,
                        rhs,
                    },
                    span,
                ))
            }
            hir::HirPat::Or(pats) => {
                if pats.is_empty() {
                    return Ok(self.add_value(ValueKind::Const(Lit::Bool(false)), span));
                }
                let mut cond = self.lower_match_pat_cond(scrut, &pats[0], span)?;
                for p in pats.iter().skip(1) {
                    let rhs = self.lower_match_pat_cond(scrut, p, span)?;
                    cond = self.add_bin_bool(BinOp::Or, cond, rhs, span);
                }
                Ok(cond)
            }
            hir::HirPat::List { items, rest } => {
                let len = self.add_value(ValueKind::Len { base: scrut }, span);
                let expected = self.add_int_val(items.len() as i64, span);
                let mut cond = if rest.is_some() {
                    self.add_bin_bool(BinOp::Ge, len, expected, span)
                } else {
                    self.add_bin_bool(BinOp::Eq, len, expected, span)
                };

                for (i, item_pat) in items.iter().enumerate() {
                    let idx = self.add_int_val((i + 1) as i64, span);
                    let elem = self.add_value(
                        ValueKind::Index1D {
                            base: scrut,
                            idx,
                            is_safe: true,
                            is_na_safe: true,
                        },
                        span,
                    );
                    let elem_cond = self.lower_match_pat_cond(elem, item_pat, span)?;
                    cond = self.add_bin_bool(BinOp::And, cond, elem_cond, span);
                }
                Ok(cond)
            }
            hir::HirPat::Record { fields } => {
                let mut cond = self.add_bool_val(true, span);
                for (field, subpat) in fields {
                    let field_name = self.symbol_name(*field);
                    let field_name_val =
                        self.add_value(ValueKind::Const(Lit::Str(field_name)), span);
                    let exists =
                        self.add_call_value("rr_field_exists", vec![scrut, field_name_val], span);
                    cond = self.add_bin_bool(BinOp::And, cond, exists, span);

                    let field_val =
                        self.add_call_value("rr_field_get", vec![scrut, field_name_val], span);
                    let field_cond = self.lower_match_pat_cond(field_val, subpat, span)?;
                    cond = self.add_bin_bool(BinOp::And, cond, field_cond, span);
                }
                Ok(cond)
            }
        }
    }

    fn bind_match_pattern(&mut self, scrut: ValueId, pat: &hir::HirPat, span: Span) -> RR<()> {
        match pat {
            hir::HirPat::Bind { local, .. } => {
                self.write_var(*local, scrut);
                Ok(())
            }
            hir::HirPat::Or(_) | hir::HirPat::Wild | hir::HirPat::Lit(_) => Ok(()),
            hir::HirPat::List { items, rest } => {
                for (i, item_pat) in items.iter().enumerate() {
                    let idx = self.add_int_val((i + 1) as i64, span);
                    let elem = self.add_value(
                        ValueKind::Index1D {
                            base: scrut,
                            idx,
                            is_safe: true,
                            is_na_safe: true,
                        },
                        span,
                    );
                    self.bind_match_pattern(elem, item_pat, span)?;
                }
                if let Some((_, rest_local)) = rest {
                    let start_idx = self.add_int_val((items.len() + 1) as i64, span);
                    let tail = self.add_call_value("rr_list_rest", vec![scrut, start_idx], span);
                    self.write_var(*rest_local, tail);
                }
                Ok(())
            }
            hir::HirPat::Record { fields } => {
                for (field, subpat) in fields {
                    let field_name = self.symbol_name(*field);
                    let field_name_val =
                        self.add_value(ValueKind::Const(Lit::Str(field_name)), span);
                    let field_val =
                        self.add_call_value("rr_field_get", vec![scrut, field_name_val], span);
                    self.bind_match_pattern(field_val, subpat, span)?;
                }
                Ok(())
            }
        }
    }

    fn hir_lit_to_lit(l: &hir::HirLit) -> Lit {
        match l {
            hir::HirLit::Int(i) => Lit::Int(*i),
            hir::HirLit::Double(f) => Lit::Float(*f),
            hir::HirLit::Char(s) => Lit::Str(s.clone()),
            hir::HirLit::Bool(b) => Lit::Bool(*b),
            hir::HirLit::NA => Lit::Na,
            hir::HirLit::Null => Lit::Null,
        }
    }

    fn unique_param_local_name(&self, param_name: &str, local_id: hir::LocalId) -> String {
        let mut candidate = format!(".arg_{}", param_name);
        if self.var_names.values().any(|n| n == &candidate) {
            candidate = format!(".arg_{}_{}", param_name, local_id.0);
        }
        candidate
    }

    fn lower_for(&mut self, iter: hir::HirForIter, body: hir::HirBlock, span: Span) -> RR<()> {
        let (var, start_id, end_id) = match iter {
            hir::HirForIter::Range {
                var, start, end, ..
            } => (var, self.lower_expr(start)?, self.lower_expr(end)?),
            hir::HirForIter::SeqAlong { var, xs } => {
                // If xs is a Range, extract start/end
                if let hir::HirExpr::Range { start, end } = xs {
                    (var, self.lower_expr(*start)?, self.lower_expr(*end)?)
                } else {
                    let xs_id = self.lower_expr(xs)?;
                    let start = self.add_value(ValueKind::Const(Lit::Int(1)), span);
                    let end = self.add_value(ValueKind::Len { base: xs_id }, span);
                    (var, start, end)
                }
            }
            hir::HirForIter::SeqLen { var, len } => {
                let start = self.add_value(ValueKind::Const(Lit::Int(1)), span);
                let end = self.lower_expr(len)?;
                (var, start, end)
            }
        };
        let pre_bb = self.curr_block;

        let header_bb = self.fn_ir.add_block();
        let body_bb = self.fn_ir.add_block();
        let exit_bb = self.fn_ir.add_block();

        self.write_var(var, start_id);
        self.add_pred(header_bb, pre_bb);
        self.terminate(Terminator::Goto(header_bb));

        self.curr_block = header_bb;
        let iv = self.read_var(var, header_bb)?;
        let cond = self.fn_ir.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: iv,
                rhs: end_id,
            },
            span,
            Facts::empty(),
            None,
        );
        self.terminate(Terminator::If {
            cond,
            then_bb: body_bb,
            else_bb: exit_bb,
        });

        self.add_pred(body_bb, header_bb);
        self.seal_block(body_bb)?; // Body is dominated by header, so we can seal immediately
        self.curr_block = body_bb;
        self.loop_stack.push(LoopTargets {
            break_bb: exit_bb,
            continue_bb: header_bb,
            continue_step: Some((var, iv)),
        });
        self.lower_block_effects(body)?;
        self.loop_stack.pop();

        let curr_reachable = self
            .preds
            .get(&self.curr_block)
            .map(|ps| !ps.is_empty())
            .unwrap_or(false);
        if !self.is_terminated(self.curr_block) && curr_reachable {
            let one =
                self.fn_ir
                    .add_value(ValueKind::Const(Lit::Int(1)), span, Facts::empty(), None);
            let next_iv = self.fn_ir.add_value(
                ValueKind::Binary {
                    op: BinOp::Add,
                    lhs: iv,
                    rhs: one,
                },
                span,
                Facts::empty(),
                None,
            );
            self.write_var(var, next_iv);
            self.add_pred(header_bb, self.curr_block);
            self.terminate(Terminator::Goto(header_bb));
        }

        self.seal_block(header_bb)?;
        self.add_pred(exit_bb, header_bb);
        self.curr_block = exit_bb;
        self.seal_block(exit_bb)?;

        Ok(())
    }
}
