use crate::ir::*;
use crate::syntax::ast::Lit;

pub struct Optimizer;

impl Optimizer {
    pub fn new() -> Self { Self }

    pub fn optimize_stmts(&self, stmts: Vec<IRStmt>) -> Vec<IRStmt> {
        let mut out = Vec::new();
        for stmt in stmts {
            match stmt.kind {
                IRStmtKind::For { var, seq, body } => {
                    if let Some(opt_stmt) = self.try_loop_vectorize(&var, &seq, &body, stmt.span) {
                        out.push(opt_stmt);
                    } else if let Some(rewrite_stmt) = self.try_loop_rewrite(&var, &seq, &body, stmt.span) {
                        // Re-run optimization on rewritten loop bodies.
                        if let IRStmtKind::For { var: v, seq: s, body: b } = rewrite_stmt.kind {
                             let opt_body = self.optimize_stmts(b);
                             out.push(IRStmt { 
                                 kind: IRStmtKind::For { var: v, seq: s, body: opt_body },
                                 span: stmt.span 
                             });
                        } else {
                             out.push(rewrite_stmt);
                        }
                    } else {
                        let opt_body = self.optimize_stmts(body);
                        out.push(IRStmt { 
                            kind: IRStmtKind::For { var, seq, body: opt_body },
                            span: stmt.span 
                        });
                    }
                }
                IRStmtKind::If { cond, then_blk, else_blk } => {
                    let opt_then = self.optimize_stmts(then_blk);
                    let opt_else = else_blk.map(|b| self.optimize_stmts(b));
                    out.push(IRStmt { kind: IRStmtKind::If { cond, then_blk: opt_then, else_blk: opt_else }, span: stmt.span });
                }
                 IRStmtKind::While { cond, body } => {
                    let opt_body = self.optimize_stmts(body);
                    out.push(IRStmt { kind: IRStmtKind::While { cond, body: opt_body }, span: stmt.span });
                 }
                 IRStmtKind::FnDecl { name, params, body } => {
                     let opt_body = self.optimize_stmts(body);
                     out.push(IRStmt { kind: IRStmtKind::FnDecl { name, params, body: opt_body }, span: stmt.span });
                 }
                _ => out.push(stmt),
            }
        }
        out
    }

    fn try_loop_vectorize(&self, iter_var: &str, seq: &IRExpr, body: &Vec<IRStmt>, for_span: Span) -> Option<IRStmt> {
        if let Some(map_stmt) = self.try_loop_map(iter_var, seq, body, for_span) {
            return Some(map_stmt);
        }
        if let Some(red_stmt) = self.try_loop_reduction(iter_var, seq, body, for_span) {
            return Some(red_stmt);
        }
        None
    }

    fn try_loop_map(&self, iter_var: &str, seq: &IRExpr, body: &Vec<IRStmt>, for_span: Span) -> Option<IRStmt> {
        // Pattern: for (i in indices(x)) { y[i] = f(x[i]); }
        let x_expr = match &seq.kind {
            IRExprKind::RrIndices { x } => x,
            _ => return None, 
        };

        if body.len() != 1 { return None; }
        if let IRStmtKind::Assign { target, value } = &body[0].kind {
            let y_expr = match target {
                IRLValue::Index1D { base, idx } => {
                    if let IRExprKind::Name(n) = &idx.kind {
                        if n == iter_var { base } else { return None; }
                    } else { return None; }
                }
                _ => return None,
            };

            let x_name = match &x_expr.kind { IRExprKind::Name(n) => n, _ => return None };

            if let Some(vec_expr) = self.try_vectorize_expr(value, x_name, iter_var, x_expr) {
                let y_name = self.extract_name(y_expr)?;
                 return Some(IRStmt {
                    kind: IRStmtKind::Assign {
                        target: IRLValue::Name(y_name),
                        value: vec_expr,
                    },
                    span: for_span,
                });
            }
        }
        None
    }

    fn try_loop_reduction(&self, iter_var: &str, seq: &IRExpr, body: &Vec<IRStmt>, for_span: Span) -> Option<IRStmt> {
        // Pattern: for (i in indices(x)) { acc = acc OP x[i]; }
        let x_expr = match &seq.kind {
            IRExprKind::RrIndices { x } => x,
            _ => return None, 
        };
        let x_name = match &x_expr.kind { IRExprKind::Name(n) => n, _ => return None };

        if body.len() != 1 { return None; }
        if let IRStmtKind::Assign { target, value } = &body[0].kind {
            let acc_name = match target {
                IRLValue::Name(n) => n,
                _ => return None,
            };

            if let IRExprKind::Binary { op, lhs, rhs } = &value.kind {
                let func_name = match op {
                    crate::syntax::ast::BinOp::Add => "sum",
                    crate::syntax::ast::BinOp::Mul => "prod",
                    _ => return None,
                };
                
                let is_lhs_acc = match &lhs.kind { IRExprKind::Name(n) => n == acc_name, _ => false };
                let is_rhs_acc = match &rhs.kind { IRExprKind::Name(n) => n == acc_name, _ => false };

                let target_subexpr = if is_lhs_acc { rhs } else if is_rhs_acc { lhs } else { return None };
                
                if self.is_indexing(target_subexpr, x_name, iter_var) {
                     let vec_call = IRExpr::new(IRExprKind::Call {
                         callee: Box::new(IRExpr::new(IRExprKind::Name(func_name.to_string()), for_span)),
                         args: vec![*x_expr.clone()],
                     }, for_span);
                     
                     return Some(IRStmt {
                         kind: IRStmtKind::Assign {
                             target: IRLValue::Name(acc_name.clone()),
                             value: IRExpr::new(IRExprKind::Binary {
                                 op: *op,
                                 lhs: Box::new(IRExpr::new(IRExprKind::Name(acc_name.clone()), for_span)),
                                 rhs: Box::new(vec_call),
                             }, for_span),
                         },
                         span: for_span,
                     });
                }
            }
        }
        None
    }

    fn try_vectorize_expr(&self, expr: &IRExpr, x_name: &str, iter_var: &str, x_vec_expr: &IRExpr) -> Option<IRExpr> {
        match &expr.kind {
            IRExprKind::Binary { op, lhs, rhs } => {
                 let is_lhs_xi = self.is_indexing(lhs, x_name, iter_var);
                 let is_rhs_xi = self.is_indexing(rhs, x_name, iter_var);
                 
                 if is_lhs_xi && !self.uses_var(rhs, iter_var) {
                     return Some(IRExpr::new(IRExprKind::Binary { op: *op, lhs: Box::new(x_vec_expr.clone()), rhs: rhs.clone() }, expr.span));
                 }
                 if is_rhs_xi && !self.uses_var(lhs, iter_var) {
                     return Some(IRExpr::new(IRExprKind::Binary { op: *op, lhs: lhs.clone(), rhs: Box::new(x_vec_expr.clone()) }, expr.span));
                 }
                 if is_lhs_xi && is_rhs_xi {
                     return Some(IRExpr::new(IRExprKind::Binary { op: *op, lhs: Box::new(x_vec_expr.clone()), rhs: Box::new(x_vec_expr.clone()) }, expr.span));
                 }
            },
            IRExprKind::Unary { op, rhs } => {
                if self.is_indexing(rhs, x_name, iter_var) {
                    return Some(IRExpr::new(IRExprKind::Unary { op: *op, rhs: Box::new(x_vec_expr.clone()) }, expr.span));
                }
            },
            IRExprKind::Call { callee, args } => {
                if self.uses_var(callee, iter_var) { return None; }
                let callee_name = match &callee.kind {
                    IRExprKind::Name(n) => n,
                    _ => return None,
                };
                if !self.is_vector_safe_call(callee_name) {
                    return None;
                }
                
                let mut vec_args = Vec::new();
                let mut found_xi = false;
                
                for arg in args {
                    if self.is_indexing(arg, x_name, iter_var) {
                        vec_args.push(x_vec_expr.clone());
                        found_xi = true;
                    } else if !self.uses_var(arg, iter_var) {
                        vec_args.push(arg.clone());
                    } else {
                        return None;
                    }
                }
                
                if found_xi {
                    return Some(IRExpr::new(IRExprKind::Call { callee: callee.clone(), args: vec_args }, expr.span));
                }
            },
            _ => {}
        }
        None
    }

    fn is_vector_safe_call(&self, name: &str) -> bool {
        matches!(
            name,
            "abs"
                | "sqrt"
                | "sin"
                | "cos"
                | "tan"
                | "log"
                | "exp"
                | "floor"
                | "ceiling"
                | "round"
        )
    }

    fn is_indexing(&self, expr: &IRExpr, base_name: &str, idx_name: &str) -> bool {
        if let IRExprKind::Index1D { base, idx } = &expr.kind {
            if let IRExprKind::Name(b) = &base.kind {
                if b == base_name {
                    if let IRExprKind::Name(i) = &idx.kind {
                        return i == idx_name;
                    }
                }
            }
        }
        false
    }
    
    fn uses_var(&self, expr: &IRExpr, var: &str) -> bool {
        match &expr.kind {
            IRExprKind::Name(n) => n == var,
            IRExprKind::Lit(_) => false,
            IRExprKind::Unary { rhs, .. } => self.uses_var(rhs, var),
            IRExprKind::Binary { lhs, rhs, .. } => self.uses_var(lhs, var) || self.uses_var(rhs, var),
            IRExprKind::Call { callee, args } =>
                self.uses_var(callee, var) || args.iter().any(|a| self.uses_var(a, var)),
            IRExprKind::RrRange { a, b } => self.uses_var(a, var) || self.uses_var(b, var),
            IRExprKind::RrIndices { x } => self.uses_var(x, var),
            IRExprKind::Index1D { base, idx } => self.uses_var(base, var) || self.uses_var(idx, var),
            IRExprKind::Index2D { base, r, c } => self.uses_var(base, var) || self.uses_var(r, var) || self.uses_var(c, var),
            IRExprKind::Slice1D { base, a, b } => self.uses_var(base, var) || self.uses_var(a, var) || self.uses_var(b, var),
            IRExprKind::VectorLit(v) => v.iter().any(|e| self.uses_var(e, var)),
            IRExprKind::ListLit(fields) => fields.iter().any(|(_, e)| self.uses_var(e, var)),
        }
    }

    fn try_loop_rewrite(&self, iter_var: &str, seq: &IRExpr, body: &Vec<IRStmt>, for_span: Span) -> Option<IRStmt> {
        // Rewrite `for (i in indices(x))` into 1-based iteration for R codegen.
        let x_expr = match &seq.kind {
            IRExprKind::RrIndices { x } => x,
            _ => return None,
        };
        
        let length_call = IRExpr::new(IRExprKind::Call {
            callee: Box::new(IRExpr::new(IRExprKind::Name("length".to_string()), for_span)),
            args: vec![*x_expr.clone()],
        }, for_span);
        
        let new_seq = IRExpr::new(IRExprKind::RrRange {
            a: Box::new(IRExpr::new(IRExprKind::Lit(Lit::Int(1)), for_span)),
            b: Box::new(length_call),
        }, for_span);
        
        let replacement = IRExpr::new(IRExprKind::Binary {
            op: crate::syntax::ast::BinOp::Sub,
            lhs: Box::new(IRExpr::new(IRExprKind::Name(iter_var.to_string()), for_span)),
            rhs: Box::new(IRExpr::new(IRExprKind::Lit(Lit::Int(1)), for_span)),
        }, for_span);
        
        let new_body = self.rewrite_stmts(body, iter_var, &replacement);
        
        Some(IRStmt {
            kind: IRStmtKind::For {
                var: iter_var.to_string(),
                seq: new_seq,
                body: new_body,
            },
            span: for_span,
        })
    }

    fn rewrite_stmts(&self, stmts: &Vec<IRStmt>, target_var: &str, replacement: &IRExpr) -> Vec<IRStmt> {
        stmts.iter().map(|s| self.rewrite_stmt(s, target_var, replacement)).collect()
    }

    fn rewrite_stmt(&self, stmt: &IRStmt, target_var: &str, replacement: &IRExpr) -> IRStmt {
        let kind = match &stmt.kind {
            IRStmtKind::Assign { target, value } => IRStmtKind::Assign {
                target: self.rewrite_lvalue(target, target_var, replacement),
                value: self.rewrite_expr(value, target_var, replacement),
            },
            IRStmtKind::ExprStmt { expr } => IRStmtKind::ExprStmt {
                expr: self.rewrite_expr(expr, target_var, replacement),
            },
            IRStmtKind::If { cond, then_blk, else_blk } => IRStmtKind::If {
                cond: self.rewrite_expr(cond, target_var, replacement),
                then_blk: self.rewrite_stmts(then_blk, target_var, replacement),
                else_blk: else_blk.as_ref().map(|b| self.rewrite_stmts(b, target_var, replacement)),
            },
            IRStmtKind::While { cond, body } => IRStmtKind::While {
                cond: self.rewrite_expr(cond, target_var, replacement),
                body: self.rewrite_stmts(body, target_var, replacement),
            },
            IRStmtKind::For { var, seq, body } => {
                if var == target_var {
                    IRStmtKind::For {
                        var: var.clone(),
                        seq: self.rewrite_expr(seq, target_var, replacement),
                        body: body.clone(),
                    }
                } else {
                    IRStmtKind::For {
                        var: var.clone(),
                        seq: self.rewrite_expr(seq, target_var, replacement),
                        body: self.rewrite_stmts(body, target_var, replacement),
                    }
                }
            },
            IRStmtKind::Return { value } => IRStmtKind::Return {
                value: value.as_ref().map(|v| self.rewrite_expr(v, target_var, replacement)),
            },
            _ => stmt.kind.clone(),
        };
        IRStmt { kind, span: stmt.span }
    }

    fn rewrite_lvalue(&self, lv: &IRLValue, target_var: &str, replacement: &IRExpr) -> IRLValue {
        match lv {
            IRLValue::Index1D { base, idx } => IRLValue::Index1D {
                base: Box::new(self.rewrite_expr(base, target_var, replacement)),
                idx: Box::new(self.rewrite_expr(idx, target_var, replacement)),
            },
            IRLValue::Index2D { base, r, c } => IRLValue::Index2D {
                base: Box::new(self.rewrite_expr(base, target_var, replacement)),
                r: Box::new(self.rewrite_expr(r, target_var, replacement)),
                c: Box::new(self.rewrite_expr(c, target_var, replacement)),
            },
            _ => lv.clone(),
        }
    }

    fn rewrite_expr(&self, expr: &IRExpr, target_var: &str, replacement: &IRExpr) -> IRExpr {
        let kind = match &expr.kind {
            IRExprKind::Name(n) => {
                if n == target_var {
                    return replacement.clone();
                }
                IRExprKind::Name(n.clone())
            },
            IRExprKind::Unary { op, rhs } => IRExprKind::Unary {
                op: *op,
                rhs: Box::new(self.rewrite_expr(rhs, target_var, replacement)),
            },
            IRExprKind::Binary { op, lhs, rhs } => IRExprKind::Binary {
                op: *op,
                lhs: Box::new(self.rewrite_expr(lhs, target_var, replacement)),
                rhs: Box::new(self.rewrite_expr(rhs, target_var, replacement)),
            },
            IRExprKind::Call { callee, args } => IRExprKind::Call {
                callee: Box::new(self.rewrite_expr(callee, target_var, replacement)),
                args: args.iter().map(|a| self.rewrite_expr(a, target_var, replacement)).collect(),
            },
            IRExprKind::Index1D { base, idx } => IRExprKind::Index1D {
                base: Box::new(self.rewrite_expr(base, target_var, replacement)),
                idx: Box::new(self.rewrite_expr(idx, target_var, replacement)),
            },
             IRExprKind::RrIndices { x } => IRExprKind::RrIndices { x: Box::new(self.rewrite_expr(x, target_var, replacement)) },
             IRExprKind::RrRange { a, b } => IRExprKind::RrRange { 
                 a: Box::new(self.rewrite_expr(a, target_var, replacement)), 
                 b: Box::new(self.rewrite_expr(b, target_var, replacement)) 
             },
             IRExprKind::VectorLit(v) => IRExprKind::VectorLit(v.iter().map(|e| self.rewrite_expr(e, target_var, replacement)).collect()),
            _ => expr.kind.clone(),
        };
        IRExpr::new(kind, expr.span)
    }

    fn extract_name(&self, expr: &IRExpr) -> Option<String> {
        match &expr.kind {
            IRExprKind::Name(n) => Some(n.clone()),
            _ => None,
        }
    }
}
