use std::collections::{BTreeMap, BTreeSet, HashMap};

use crate::bail;
use crate::error::{InternalCompilerError, RR, RRCode, Stage};
use crate::ir::*;
use crate::mir::*;
use crate::syntax::ast::{BinOp, Lit};
use crate::utils::Span;

pub struct MirBuilder {
    curr_fn: FnIR,
    curr_block: BlockId,
    env: HashMap<String, ValueId>,
}

impl MirBuilder {
    pub fn new(name: String) -> Self {
        let mut fn_ir = FnIR::new(name, Vec::new()); // types?
        let entry = fn_ir.add_block();
        let body_head = fn_ir.add_block();
        fn_ir.entry = entry;
        fn_ir.body_head = body_head;
        Self {
            curr_fn: fn_ir,
            curr_block: entry,
            env: HashMap::new(),
        }
    }

    pub fn build_stmts(&mut self, stmts: Vec<IRStmt>) -> RR<()> {
        for stmt in stmts {
            self.lower_stmt(stmt)?;
        }
        Ok(())
    }

    fn lower_stmt(&mut self, stmt: IRStmt) -> RR<()> {
        match stmt.kind {
            IRStmtKind::Assign { target, value } => {
                let val_id = self.lower_expr(value)?;
                self.lower_assign(target, val_id, stmt.span)?;
            }
            IRStmtKind::ExprStmt { expr } => {
                self.lower_expr(expr)?; // For side effects
            }
            IRStmtKind::If {
                cond,
                then_blk,
                else_blk,
            } => {
                self.lower_if(cond, then_blk, else_blk, stmt.span)?;
            }
            IRStmtKind::While { cond, body } => {
                self.lower_while(cond, body, stmt.span)?;
            }
            IRStmtKind::For { var, seq, body } => {
                self.lower_for(var, seq, body, stmt.span)?;
            }
            IRStmtKind::Return { value } => {
                let val = if let Some(e) = value {
                    Some(self.lower_expr(e)?)
                } else {
                    None
                };
                self.terminate(Terminator::Return(val));
                // Start new unreachable block for subsequent stmts
                self.curr_block = self.curr_fn.add_block();
            }
            _ => {}
        }
        Ok(())
    }

    fn lower_assign(&mut self, target: IRLValue, val_id: ValueId, span: Span) -> RR<()> {
        match target {
            IRLValue::Name(n) => {
                // SSA: Update environment, no instruction needed.
                self.env.insert(n, val_id);
                Ok(())
            }
            IRLValue::Index1D { base, idx } => {
                let base_id = self.lower_expr(*base)?;
                let idx_id = self.lower_expr(*idx)?;
                // StoreIndex1D is a side-effect instruction, still needs to be emitted
                let kind = Instr::StoreIndex1D {
                    base: base_id,
                    idx: idx_id,
                    val: val_id,
                    is_safe: false,
                    is_na_safe: false,
                    is_vector: false,
                    span,
                };
                self.push_instr(kind);
                Ok(())
            }
            IRLValue::Index2D { base, r, c } => {
                let base_id = self.lower_expr(*base)?;
                let r_id = self.lower_expr(*r)?;
                let c_id = self.lower_expr(*c)?;
                let kind = Instr::StoreIndex2D {
                    base: base_id,
                    r: r_id,
                    c: c_id,
                    val: val_id,
                    span,
                };
                self.push_instr(kind);
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn lower_expr(&mut self, expr: IRExpr) -> RR<ValueId> {
        let span = expr.span;
        let kind = match expr.kind {
            IRExprKind::Lit(l) => ValueKind::Const(l),
            IRExprKind::Name(n) => {
                // SSA Lookup
                if let Some(&id) = self.env.get(&n) {
                    return Ok(id);
                } else {
                    bail!(
                        "RR.SemanticError",
                        RRCode::E1001,
                        Stage::Mir,
                        "Undefined variable '{}'",
                        n
                    );
                }
            }
            IRExprKind::Binary { op, lhs, rhs } => {
                let l = self.lower_expr(*lhs)?;
                let r = self.lower_expr(*rhs)?;
                ValueKind::Binary { op, lhs: l, rhs: r }
            }
            IRExprKind::Unary { op, rhs } => {
                let r = self.lower_expr(*rhs)?;
                ValueKind::Unary { op, rhs: r }
            }

            // PRIMITIVE LOWERING
            IRExprKind::Call { callee, args } => {
                let c_id = self.lower_expr(*callee)?;
                let mut args_id = Vec::new();
                for a in args {
                    args_id.push(self.lower_expr(a)?);
                }

                // Canonicalize known primitives to dedicated MIR nodes.
                let c_name = self.extract_name_sync(c_id);
                if c_name == "length" && args_id.len() == 1 {
                    ValueKind::Len { base: args_id[0] }
                } else if c_name == "rr_indices" && args_id.len() == 1 {
                    ValueKind::Indices { base: args_id[0] }
                } else if c_name == "rr_range" && args_id.len() == 2 {
                    ValueKind::Range {
                        start: args_id[0],
                        end: args_id[1],
                    }
                } else {
                    ValueKind::Call {
                        callee: c_name,
                        names: vec![None; args_id.len()],
                        args: args_id,
                    }
                }
            }

            IRExprKind::Index1D { base, idx } => {
                let b_id = self.lower_expr(*base)?;
                let i_id = self.lower_expr(*idx)?;
                ValueKind::Index1D {
                    base: b_id,
                    idx: i_id,
                    is_safe: false,
                    is_na_safe: false,
                }
            }
            IRExprKind::Index2D { base, r, c } => {
                let b_id = self.lower_expr(*base)?;
                let r_id = self.lower_expr(*r)?;
                let c_id = self.lower_expr(*c)?;
                ValueKind::Index2D {
                    base: b_id,
                    r: r_id,
                    c: c_id,
                }
            }

            IRExprKind::RrIndices { x } => {
                let x_id = self.lower_expr(*x)?;
                ValueKind::Indices { base: x_id }
            }
            IRExprKind::RrRange { a, b } => {
                let s = self.lower_expr(*a)?;
                let e = self.lower_expr(*b)?;
                ValueKind::Range { start: s, end: e }
            }
            _ => bail!(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Mir,
                "Unsupported IR expr in MIR: {:?}",
                expr
            ),
        };
        Ok(self.curr_fn.add_value(kind, span, expr.facts, None))
    }

    fn extract_name_sync(&self, vid: ValueId) -> String {
        let val = &self.curr_fn.values[vid];
        if let ValueKind::Const(Lit::Str(n)) = &val.kind {
            n.clone()
        } else {
            "unknown".into()
        }
    }

    fn lower_if(
        &mut self,
        cond: IRExpr,
        then_blk: Vec<IRStmt>,
        else_blk: Option<Vec<IRStmt>>,
        _span: Span,
    ) -> RR<()> {
        let cond_id = self.lower_expr(cond)?;

        let start_env = self.env.clone();

        let then_bb = self.curr_fn.add_block();
        let else_bb = self.curr_fn.add_block();
        let merge_bb = self.curr_fn.add_block();

        self.terminate(Terminator::If {
            cond: cond_id,
            then_bb,
            else_bb,
        });

        // THEN Branch
        self.curr_block = then_bb;
        self.build_stmts(then_blk)?;
        let end_then_bb = self.curr_block;
        let then_env = self.env.clone();
        if !self.is_terminated(self.curr_block) {
            self.terminate(Terminator::Goto(merge_bb));
        }

        // ELSE Branch
        self.env = start_env;
        self.curr_block = else_bb;
        if let Some(eb) = else_blk {
            self.build_stmts(eb)?;
        }
        let end_else_bb = self.curr_block;
        let else_env = self.env.clone();
        if !self.is_terminated(self.curr_block) {
            self.terminate(Terminator::Goto(merge_bb));
        }

        // MERGE Branch
        self.curr_block = merge_bb;
        self.env = self.merge_envs(then_env, else_env, end_then_bb, end_else_bb, _span)?;
        Ok(())
    }

    fn merge_envs(
        &mut self,
        env1: HashMap<String, ValueId>,
        env2: HashMap<String, ValueId>,
        bb1: BlockId,
        bb2: BlockId,
        span: Span,
    ) -> RR<HashMap<String, ValueId>> {
        let mut merged = HashMap::new();
        let mut keys = BTreeSet::new();
        keys.extend(env1.keys().cloned());

        // Definite assignment: only merge keys present in both branches.
        for key in keys {
            let Some(&val1) = env1.get(&key) else {
                continue;
            };
            let Some(&val2) = env2.get(&key) else {
                continue;
            };
            if val1 == val2 {
                merged.insert(key, val1);
                continue;
            }

            let phi_id = self.curr_fn.add_value(
                ValueKind::Phi {
                    args: vec![(val1, bb1), (val2, bb2)],
                },
                span,
                Facts::empty(),
                None,
            );
            if let Some(v) = self.curr_fn.values.get_mut(phi_id) {
                v.phi_block = Some(self.curr_block);
            }
            merged.insert(key, phi_id);
        }
        Ok(merged)
    }

    fn lower_while(&mut self, cond: IRExpr, body: Vec<IRStmt>, _span: Span) -> RR<()> {
        let mutated_vars = self.collect_assigned_vars(&body);

        let preheader_bb = self.curr_block;
        let header_bb = self.curr_fn.add_block();
        let body_bb = self.curr_fn.add_block();
        let exit_bb = self.curr_fn.add_block();

        self.terminate(Terminator::Goto(header_bb));

        // Header: Setup Phis
        self.curr_block = header_bb;
        let mut phis: BTreeMap<String, ValueId> = BTreeMap::new();
        for var in mutated_vars {
            if let Some(&pre_val) = self.env.get(&var) {
                let phi_id = self.curr_fn.add_value(
                    ValueKind::Phi {
                        args: vec![(pre_val, preheader_bb)],
                    },
                    _span,
                    Facts::empty(),
                    None,
                );
                if let Some(v) = self.curr_fn.values.get_mut(phi_id) {
                    v.phi_block = Some(header_bb);
                }
                phis.insert(var.clone(), phi_id);
                self.env.insert(var, phi_id);
            }
        }

        let cond_id = self.lower_expr(cond)?;
        self.terminate(Terminator::If {
            cond: cond_id,
            then_bb: body_bb,
            else_bb: exit_bb,
        });

        // Body
        self.curr_block = body_bb;
        self.build_stmts(body)?;
        let latch_bb = self.curr_block;
        if !self.is_terminated(latch_bb) {
            self.terminate(Terminator::Goto(header_bb));
        }

        // Backpatch Phis
        for (var, phi_id) in phis {
            let latch_val = self.env.get(&var).copied().ok_or_else(|| {
                InternalCompilerError::new(
                    Stage::Mir,
                    format!("Var '{}' vanished in while body", var),
                )
                .into_exception()
            })?;
            self.append_phi_arg(phi_id, latch_val, latch_bb)?;
        }

        self.curr_block = exit_bb;
        Ok(())
    }

    fn lower_for(&mut self, var: String, seq: IRExpr, body: Vec<IRStmt>, span: Span) -> RR<()> {
        // 1. Identify Loop Bounds (Start, End) from Sequence
        let (start_id, end_id) = match seq.kind {
            IRExprKind::RrRange { a, b } => (self.lower_expr(*a)?, self.lower_expr(*b)?),
            IRExprKind::RrIndices { x } => {
                let x_id = self.lower_expr(*x)?;
                // start = 0
                let zero = self.curr_fn.add_value(
                    ValueKind::Const(Lit::Int(0)),
                    span,
                    Facts::empty(),
                    None,
                );
                // end = len(x) - 1
                let len = self.curr_fn.add_value(
                    ValueKind::Len { base: x_id },
                    span,
                    Facts::empty(),
                    None,
                );
                let one = self.curr_fn.add_value(
                    ValueKind::Const(Lit::Int(1)),
                    span,
                    Facts::empty(),
                    None,
                );
                let end = self.curr_fn.add_value(
                    ValueKind::Binary {
                        op: BinOp::Sub,
                        lhs: len,
                        rhs: one,
                    },
                    span,
                    Facts::empty(),
                    None,
                );
                (zero, end)
            }
            _ => bail!(
                "RR.SemanticError",
                RRCode::E1002,
                Stage::Mir,
                "Unsupported loop sequence: {:?}",
                seq
            ),
        };

        let preheader_bb = self.curr_block;
        let preheader_env = self.env.clone();

        let header_bb = self.curr_fn.add_block();
        let body_bb = self.curr_fn.add_block();
        let exit_bb = self.curr_fn.add_block();

        self.terminate(Terminator::Goto(header_bb));

        // 2. Header: Setup Phis
        self.curr_block = header_bb;

        let mutated = self.collect_assigned_vars(&body);
        let mut phi_map: BTreeMap<String, ValueId> = BTreeMap::new(); // Var -> PhiValueId

        // Induction Variable Phi
        let iv_phi =
            self.curr_fn
                .add_value(ValueKind::Phi { args: vec![] }, span, Facts::empty(), None);
        if let Some(v) = self.curr_fn.values.get_mut(iv_phi) {
            v.phi_block = Some(header_bb);
        }
        self.env.insert(var.clone(), iv_phi);
        phi_map.insert(var.clone(), iv_phi);

        // Other Mutated Vars Phi
        for m in &mutated {
            if m == &var {
                continue;
            }
            // If defined in preheader, use it. If not, maybe Undef?
            if let Some(&_pre_val) = preheader_env.get(m) {
                let p = self.curr_fn.add_value(
                    ValueKind::Phi { args: vec![] },
                    span,
                    Facts::empty(),
                    None,
                );
                if let Some(v) = self.curr_fn.values.get_mut(p) {
                    v.phi_block = Some(header_bb);
                }
                self.env.insert(m.clone(), p);
                phi_map.insert(m.clone(), p);
            }
        }

        // Populate Preheader args for Phis
        for (v, phi_id) in &phi_map {
            let val = if v == &var {
                start_id
            } else {
                *preheader_env.get(v).unwrap()
            }; // Safe unwrap due to check above
            self.append_phi_arg(*phi_id, val, preheader_bb)?;
        }

        // Condition: var <= end
        let curr_iv = *self.env.get(&var).unwrap();
        let cond = self.curr_fn.add_value(
            ValueKind::Binary {
                op: BinOp::Le,
                lhs: curr_iv,
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

        // 3. Body
        self.curr_block = body_bb;
        self.build_stmts(body)?;
        let latch_bb = self.curr_block;

        // Step: next_iv = curr_iv + 1
        let one = self
            .curr_fn
            .add_value(ValueKind::Const(Lit::Int(1)), span, Facts::empty(), None);
        let next_iv = self.curr_fn.add_value(
            ValueKind::Binary {
                op: BinOp::Add,
                lhs: iv_phi,
                rhs: one,
            },
            span,
            Facts::empty(),
            None,
        );

        // Latch: Goto Header
        if !self.is_terminated(self.curr_block) {
            self.terminate(Terminator::Goto(header_bb));
        }

        // 4. Backpatch Phis
        // For IV: comes from next_iv
        self.append_phi_arg(iv_phi, next_iv, latch_bb)?;

        // For others: comes from self.env (end of body)
        for (v, phi_id) in &phi_map {
            if v == &var {
                continue;
            } // Already handled IV
            if let Some(&latch_val) = self.env.get(v) {
                self.append_phi_arg(*phi_id, latch_val, latch_bb)?;
            } else {
                // If var was killed (unlikely in this model), use preheader val?
                // Or undefined.
            }
        }

        // 5. Exit
        self.curr_block = exit_bb;

        // Preserve loop-variable visibility at loop exit.
        let ran_at_least_once = self.curr_fn.add_value(
            ValueKind::Phi {
                args: vec![(iv_phi, header_bb)],
            },
            span,
            Facts::empty(),
            None,
        );
        if let Some(v) = self.curr_fn.values.get_mut(ran_at_least_once) {
            v.phi_block = Some(exit_bb);
        }
        if let Some(&pre_val) = preheader_env.get(&var) {
            let final_val = self.curr_fn.add_value(
                ValueKind::Phi {
                    args: vec![(pre_val, header_bb)], // Header is the only pred
                },
                span,
                Facts::empty(),
                None,
            );
            if let Some(v) = self.curr_fn.values.get_mut(final_val) {
                v.phi_block = Some(exit_bb);
            }
            self.env.insert(var.clone(), iv_phi);
        }
        Ok(())
    }

    fn append_phi_arg(&mut self, phi_id: ValueId, val: ValueId, bb: BlockId) -> RR<()> {
        if let ValueKind::Phi { args } = &mut self.curr_fn.values[phi_id].kind {
            args.push((val, bb));
        }
        Ok(())
    }

    fn terminate(&mut self, term: Terminator) {
        self.curr_fn.blocks[self.curr_block].term = term;
    }

    fn is_terminated(&self, block: BlockId) -> bool {
        match self.curr_fn.blocks[block].term {
            Terminator::Unreachable => false, // Default state before a terminator is set.
            _ => true,
        }
    }

    fn push_instr(&mut self, instr: Instr) {
        self.curr_fn.blocks[self.curr_block].instrs.push(instr);
    }

    fn extract_var(&self, expr: Box<IRExpr>) -> String {
        match expr.kind {
            IRExprKind::Name(n) => n,
            _ => "tmp".to_string(), // Error handling?
        }
    }

    fn extract_name(&self, expr: IRExpr) -> Option<String> {
        match expr.kind {
            IRExprKind::Name(n) => Some(n),
            _ => None,
        }
    }

    fn collect_assigned_vars(&self, stmts: &[IRStmt]) -> Vec<String> {
        let mut vars = BTreeSet::new();
        self.collect_assigned_vars_into(stmts, &mut vars);
        vars.into_iter().collect()
    }

    fn collect_assigned_vars_into(&self, stmts: &[IRStmt], out: &mut BTreeSet<String>) {
        for stmt in stmts {
            match &stmt.kind {
                IRStmtKind::Assign {
                    target: IRLValue::Name(n),
                    ..
                } => {
                    out.insert(n.clone());
                }
                IRStmtKind::If {
                    then_blk, else_blk, ..
                } => {
                    self.collect_assigned_vars_into(then_blk, out);
                    if let Some(else_branch) = else_blk {
                        self.collect_assigned_vars_into(else_branch, out);
                    }
                }
                IRStmtKind::While { body, .. } | IRStmtKind::For { body, .. } => {
                    self.collect_assigned_vars_into(body, out);
                }
                _ => {}
            }
        }
    }

    pub fn finish(self) -> FnIR {
        self.curr_fn
    }
}
