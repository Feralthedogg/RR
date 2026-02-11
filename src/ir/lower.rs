use crate::syntax::ast::*;
use crate::ir::*;
use crate::error::{RR, RRCode, Stage, RRCtx};
use crate::bail;

pub struct Lowerer {
    pub optimize: bool,
}

impl Lowerer {
    pub fn new(optimize: bool) -> Self {
        Self { optimize }
    }

    pub fn lower_program(&self, prog: Program) -> RR<IRProgram> {
        let mut stmts = Vec::new();
        for s in prog.stmts {
            stmts.push(self.lower_stmt(s).ctx("Lowerer.lower_stmt/1", None)?);
        }
        
        if self.optimize {
            stmts = self.optimize_stmts(stmts);
        }
        
        let mut ir_prog = IRProgram { stmts };
        
        if self.optimize {
            let mut analyzer = crate::ir::analyze::Analyzer::new();
            analyzer.analyze_program(&mut ir_prog);
        }

        Ok(ir_prog)
    }

    fn lower_stmt(&self, stmt: Stmt) -> RR<IRStmt> {
        let kind = match stmt.kind {
            StmtKind::Let {
                name,
                ty_hint: _,
                init,
            } => {
                IRStmtKind::Assign { 
                    target: IRLValue::Name(name), 
                    value: self.lower_expr(init)?
                }
            }
            StmtKind::Assign { target, value } => {
                IRStmtKind::Assign { 
                    target: self.lower_lvalue(target)?, 
                    value: self.lower_expr(value)?
                }
            }
            StmtKind::ExprStmt { expr } => {
                IRStmtKind::ExprStmt { expr: self.lower_expr(expr)? }
            }
            StmtKind::FnDecl {
                name,
                params,
                ret_ty_hint: _,
                body,
            } => {
                let mut ir_body = Vec::new();
                for s in body.stmts {
                    ir_body.push(self.lower_stmt(s)?);
                }
                let param_names = params.into_iter().map(|p| p.name).collect();
                IRStmtKind::FnDecl {
                    name,
                    params: param_names,
                    body: ir_body,
                }
            }
            StmtKind::If { cond, then_blk, else_blk } => {
                let ir_cond = self.lower_expr(cond)?;
                let mut ir_then = Vec::new();
                for s in then_blk.stmts { ir_then.push(self.lower_stmt(s)?); }
                let ir_else = if let Some(blk) = else_blk {
                    let mut v = Vec::new();
                    for s in blk.stmts { v.push(self.lower_stmt(s)?); }
                    Some(v)
                } else {
                    None
                };
                IRStmtKind::If { cond: ir_cond, then_blk: ir_then, else_blk: ir_else }
            }
            StmtKind::While { cond, body } => {
                let ir_cond = self.lower_expr(cond)?;
                let mut ir_body = Vec::new();
                for s in body.stmts { ir_body.push(self.lower_stmt(s)?); }
                IRStmtKind::While { cond: ir_cond, body: ir_body }
            }
            StmtKind::For { var, iter, body } => {
                let seq = if self.optimize {
                    self.try_simplify_range_iter(&iter)?.unwrap_or_else(|| self.lower_expr_sync(&iter))
                } else {
                    self.lower_expr(iter)?
                };

                let mut ir_body = Vec::new();
                for s in body.stmts { ir_body.push(self.lower_stmt(s)?); }
                IRStmtKind::For { var, seq, body: ir_body }
            }
            StmtKind::Return { value } => {
                let ir_val = if let Some(e) = value { Some(self.lower_expr(e)?) } else { None };
                IRStmtKind::Return { value: ir_val }
            }
            StmtKind::Break | StmtKind::Next => {
                bail!(
                    "RR.FeatureError",
                    RRCode::E9999,
                    Stage::Lower,
                    "break/next are only supported in MIR pipeline"
                );
            }
        };
        Ok(IRStmt { kind, span: stmt.span })
    }

    fn try_simplify_range_iter(&self, expr: &Expr) -> RR<Option<IRExpr>> {
        if let ExprKind::Range { a, b } = &expr.kind {
            let a_is_zero = match &a.kind {
                ExprKind::Lit(Lit::Int(0)) => true,
                _ => false,
            };
            if !a_is_zero { return Ok(None); }

            if let ExprKind::Binary { op: BinOp::Sub, lhs, rhs } = &b.kind {
                let rhs_is_one = match &rhs.kind {
                    ExprKind::Lit(Lit::Int(1)) => true,
                    _ => false,
                };
                if !rhs_is_one { return Ok(None); }

                if let ExprKind::Call { callee, args } = &lhs.kind {
                    if args.len() == 1 {
                        if let ExprKind::Name(n) = &callee.kind {
                            if n == "len" || n == "length" {
                                let x_ir = self.lower_expr(args[0].clone())?;
                                return Ok(Some(IRExpr::new(IRExprKind::RrIndices { x: Box::new(x_ir) }, expr.span)));
                            }
                        }
                    }
                }
            }
        }
        Ok(None)
    }

    fn lower_expr_sync(&self, expr: &Expr) -> IRExpr {
         self.lower_expr(expr.clone()).unwrap_or_else(|_| {
             IRExpr::new(IRExprKind::Lit(Lit::Null), expr.span)
         })
    }

    fn lower_lvalue(&self, lv: LValue) -> RR<IRLValue> {
        match lv.kind {
            LValueKind::Name(n) => Ok(IRLValue::Name(n)),
            LValueKind::Index { base, idx } => {
                let ir_base = Box::new(self.lower_expr(base)?);
                let mut ir_idx = Vec::new();
                for e in idx { ir_idx.push(self.lower_expr(e)?); }
                
                if ir_idx.len() == 1 {
                    Ok(IRLValue::Index1D { base: ir_base, idx: Box::new(ir_idx.into_iter().next().unwrap()) })
                } else if ir_idx.len() == 2 {
                    let mut iter = ir_idx.into_iter();
                    let r = Box::new(iter.next().unwrap());
                    let c = Box::new(iter.next().unwrap());
                    Ok(IRLValue::Index2D { base: ir_base, r, c })
                } else {
                    bail!("RR.SemanticError", RRCode::E1002, Stage::Lower, "RR v2.0 only supports 1D and 2D indexing");
                }
            }
        }
    }

    fn lower_expr(&self, expr: Expr) -> RR<IRExpr> {
        let kind = match expr.kind {
            ExprKind::Lit(l) => IRExprKind::Lit(l),
            ExprKind::Name(n) => IRExprKind::Name(n),
            ExprKind::Unary { op, rhs } => IRExprKind::Unary { op, rhs: Box::new(self.lower_expr(*rhs)?) },
            ExprKind::Binary { op, lhs, rhs } => IRExprKind::Binary { op, lhs: Box::new(self.lower_expr(*lhs)?), rhs: Box::new(self.lower_expr(*rhs)?) },
            ExprKind::Range { a, b } => IRExprKind::RrRange { a: Box::new(self.lower_expr(*a)?), b: Box::new(self.lower_expr(*b)?) },
            ExprKind::VectorLit(v) => {
                let mut ir_elems = Vec::new();
                for e in v { ir_elems.push(self.lower_expr(e)?); }
                IRExprKind::VectorLit(ir_elems)
            }
            ExprKind::RecordLit(fields) => {
                let mut ir_fields = Vec::new();
                for (n, e) in fields { ir_fields.push((n, self.lower_expr(e)?)); }
                IRExprKind::ListLit(ir_fields)
            }
            ExprKind::Call { callee, args } => {
                let mut lowered_callee = self.lower_expr(*callee)?;
                let mut lowered_args = Vec::new();
                for e in args { lowered_args.push(self.lower_expr(e)?); }

                // Rewrite known RR aliases to canonical runtime/builtin names.
                if let IRExprKind::Name(ref mut n) = lowered_callee.kind {
                    match n.as_str() {
                        "vec_int" => *n = "integer".to_string(),
                        "vec_f64" => *n = "numeric".to_string(),
                        "vec_bool" => *n = "logical".to_string(),
                        "vec_str" => *n = "character".to_string(),
                        "len" => *n = "length".to_string(),
                        "range" => *n = "rr_range".to_string(),
                        "indices" => *n = "rr_indices".to_string(),
                        _ => {}
                    }
                }

                if let IRExprKind::Name(n) = &lowered_callee.kind {
                    match (n.as_str(), lowered_args.as_slice()) {
                        ("rr_range", [a, b]) => {
                            return Ok(IRExpr::new(IRExprKind::RrRange {
                                a: Box::new(a.clone()), b: Box::new(b.clone())
                            }, expr.span));
                        }
                        ("rr_indices", [x]) => {
                            return Ok(IRExpr::new(IRExprKind::RrIndices { x: Box::new(x.clone()) }, expr.span));
                        }
                        _ => {}
                    }
                }

                IRExprKind::Call { callee: Box::new(lowered_callee), args: lowered_args }
            }
            ExprKind::Index { base, idx } => {
                let ir_base = Box::new(self.lower_expr(*base)?);
                let ir_idx_len = idx.len();
                let mut ir_idx = Vec::new();
                for e in idx { ir_idx.push(self.lower_expr(e)?); }

                if ir_idx_len == 1 {
                    let first = ir_idx.into_iter().next().unwrap();
                    if let IRExprKind::RrRange { a, b } = first.kind {
                         IRExprKind::Slice1D { base: ir_base, a, b }
                    } else {
                         IRExprKind::Index1D { base: ir_base, idx: Box::new(first) }
                    }
                } else if ir_idx_len == 2 {
                    let mut it = ir_idx.into_iter();
                    let r = Box::new(it.next().unwrap());
                    let c = Box::new(it.next().unwrap());
                    IRExprKind::Index2D { base: ir_base, r, c }
                } else {
                    bail!("RR.SemanticError", RRCode::E1002, Stage::Lower, "RR v2.0 only supports 1D and 2D indexing");
                }
            }
            ExprKind::Pipe { lhs, rhs_call } => {
                if let ExprKind::Call { callee, mut args } = rhs_call.kind {
                   args.insert(0, *lhs);
                   let new_call = Expr {
                       kind: ExprKind::Call { callee, args },
                       span: expr.span,
                   };
                   return self.lower_expr(new_call);
                } else {
                   bail!("RR.SemanticError", RRCode::E1002, Stage::Lower, "Pipe RHS must be a Call");
                }
            }
            ExprKind::Match { .. } | ExprKind::Try { .. } | ExprKind::ColRef(_) | ExprKind::Unquote(_) => {
                bail!("RR.FeatureError", RRCode::E9999, Stage::Lower, "v6 Feature (Match/Try/Tidy) not supported in legacy IR");
            }
        };
        Ok(IRExpr::new(kind, expr.span))
    }

    fn optimize_stmts(&self, stmts: Vec<IRStmt>) -> Vec<IRStmt> {
        let optimizer = crate::ir::optimize::Optimizer::new();
        optimizer.optimize_stmts(stmts)
    }
}
