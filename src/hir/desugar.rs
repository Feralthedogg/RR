use crate::error::RR;
use crate::hir::def::*;
use crate::utils::Span;

pub struct Desugarer {
    next_local_temp: u32,
}

impl Default for Desugarer {
    fn default() -> Self {
        Self::new()
    }
}

impl Desugarer {
    pub fn new() -> Self {
        Self {
            next_local_temp: 10000,
        }
    }

    fn temp_local(&mut self) -> LocalId {
        let id = LocalId(self.next_local_temp);
        self.next_local_temp += 1;
        id
    }

    pub fn desugar_program(&mut self, prog: HirProgram) -> RR<HirProgram> {
        let mut modules = Vec::new();
        for m in prog.modules {
            modules.push(self.desugar_module(m)?);
        }
        Ok(HirProgram { modules })
    }

    fn desugar_module(&mut self, mut module: HirModule) -> RR<HirModule> {
        let mut items = Vec::new();
        for item in module.items {
            items.push(self.desugar_item(item)?);
        }
        module.items = items;
        Ok(module)
    }

    fn desugar_item(&mut self, item: HirItem) -> RR<HirItem> {
        match item {
            HirItem::Fn(mut f) => {
                f.body = self.desugar_block(f.body)?;
                Ok(HirItem::Fn(f))
            }
            HirItem::Stmt(s) => Ok(HirItem::Stmt(self.desugar_stmt(s)?)),
            _ => Ok(item),
        }
    }

    fn desugar_block(&mut self, block: HirBlock) -> RR<HirBlock> {
        let mut stmts = Vec::new();
        for s in block.stmts {
            stmts.push(self.desugar_stmt(s)?);
        }
        Ok(HirBlock {
            stmts,
            span: block.span,
        })
    }

    fn desugar_stmt(&mut self, stmt: HirStmt) -> RR<HirStmt> {
        match stmt {
            HirStmt::Let {
                local,
                name,
                ty,
                init,
                span,
            } => {
                let init = if let Some(e) = init {
                    Some(self.desugar_expr(e)?)
                } else {
                    None
                };
                Ok(HirStmt::Let {
                    local,
                    name,
                    ty,
                    init,
                    span,
                })
            }
            HirStmt::Assign {
                target,
                value,
                span,
            } => Ok(HirStmt::Assign {
                target: self.desugar_lvalue(target)?,
                value: self.desugar_expr(value)?,
                span,
            }),
            HirStmt::If {
                cond,
                then_blk,
                else_blk,
                span,
            } => Ok(HirStmt::If {
                cond: self.desugar_expr(cond)?,
                then_blk: self.desugar_block(then_blk)?,
                else_blk: if let Some(e) = else_blk {
                    Some(self.desugar_block(e)?)
                } else {
                    None
                },
                span,
            }),
            HirStmt::While { cond, body, span } => Ok(HirStmt::While {
                cond: self.desugar_expr(cond)?,
                body: self.desugar_block(body)?,
                span,
            }),
            HirStmt::Expr { expr, span } => Ok(HirStmt::Expr {
                expr: self.desugar_expr(expr)?,
                span,
            }),
            HirStmt::For { iter, body, span } => {
                let iter = match iter {
                    HirForIter::Range {
                        var,
                        start,
                        end,
                        inclusive,
                    } => HirForIter::Range {
                        var,
                        start: self.desugar_expr(start)?,
                        end: self.desugar_expr(end)?,
                        inclusive,
                    },
                    HirForIter::SeqLen { var, len } => HirForIter::SeqLen {
                        var,
                        len: self.desugar_expr(len)?,
                    },
                    HirForIter::SeqAlong { var, xs } => HirForIter::SeqAlong {
                        var,
                        xs: self.desugar_expr(xs)?,
                    },
                };
                Ok(HirStmt::For {
                    iter,
                    body: self.desugar_block(body)?,
                    span,
                })
            }
            HirStmt::Return { value, span } => {
                let v = if let Some(e) = value {
                    Some(self.desugar_expr(e)?)
                } else {
                    None
                };
                Ok(HirStmt::Return { value: v, span })
            }
            HirStmt::Break { span } => Ok(HirStmt::Break { span }),
            HirStmt::Next { span } => Ok(HirStmt::Next { span }),
            HirStmt::UnsafeRBlock {
                code,
                read_only,
                span,
            } => Ok(HirStmt::UnsafeRBlock {
                code,
                read_only,
                span,
            }),
        }
    }

    fn desugar_lvalue(&mut self, lv: HirLValue) -> RR<HirLValue> {
        match lv {
            HirLValue::Index { base, index } => {
                let mut new_index = Vec::with_capacity(index.len());
                for idx in index {
                    new_index.push(self.desugar_expr(idx)?);
                }
                Ok(HirLValue::Index {
                    base: self.desugar_expr(base)?,
                    index: new_index,
                })
            }
            HirLValue::Field { base, name } => Ok(HirLValue::Field {
                base: self.desugar_expr(base)?,
                name,
            }),
            _ => Ok(lv),
        }
    }

    fn desugar_expr(&mut self, expr: HirExpr) -> RR<HirExpr> {
        match expr {
            HirExpr::Match { scrut, arms } => {
                // Keep Match structure intact for MIR lowering.
                let scrut = Box::new(self.desugar_expr(*scrut)?);
                let mut new_arms = Vec::with_capacity(arms.len());
                for arm in arms {
                    let guard = if let Some(g) = arm.guard {
                        Some(self.desugar_expr(g)?)
                    } else {
                        None
                    };
                    new_arms.push(HirMatchArm {
                        pat: arm.pat,
                        guard,
                        body: self.desugar_expr(arm.body)?,
                        span: arm.span,
                    });
                }
                Ok(HirExpr::Match {
                    scrut,
                    arms: new_arms,
                })
            }
            HirExpr::Binary { op, lhs, rhs } => Ok(HirExpr::Binary {
                op,
                lhs: Box::new(self.desugar_expr(*lhs)?),
                rhs: Box::new(self.desugar_expr(*rhs)?),
            }),
            HirExpr::Range { start, end } => Ok(HirExpr::Range {
                start: Box::new(self.desugar_expr(*start)?),
                end: Box::new(self.desugar_expr(*end)?),
            }),
            HirExpr::Index { base, index } => {
                let mut new_index = Vec::with_capacity(index.len());
                for idx in index {
                    new_index.push(self.desugar_expr(idx)?);
                }
                Ok(HirExpr::Index {
                    base: Box::new(self.desugar_expr(*base)?),
                    index: new_index,
                })
            }
            HirExpr::Call(mut c) => {
                c.callee = Box::new(self.desugar_expr(*c.callee)?);
                for arg in &mut c.args {
                    match arg {
                        HirArg::Pos(e) => *e = self.desugar_expr(e.clone())?,
                        HirArg::Named { value, .. } => *value = self.desugar_expr(value.clone())?,
                    }
                }
                Ok(HirExpr::Call(c))
            }
            HirExpr::Try(e) => {
                // `Try` is preserved until variant-pattern support is introduced.
                let e = self.desugar_expr(*e)?;
                let _ok_sym = SymbolId(1);
                let _ok_lid = self.temp_local();
                let _err_sym = SymbolId(2);
                let _err_lid = self.temp_local();
                let _pat_ok = HirPat::Record { fields: vec![] };
                let _arms: Vec<HirMatchArm> = vec![];
                Ok(HirExpr::Try(Box::new(e)))
            }
            HirExpr::Block(blk) => Ok(HirExpr::Block(self.desugar_block(blk)?)),
            HirExpr::ListLit(items) => {
                let mut new_items = Vec::new();
                for (n, e) in items {
                    new_items.push((n, self.desugar_expr(e)?));
                }
                Ok(HirExpr::ListLit(new_items))
            }
            HirExpr::Some(e) => Ok(HirExpr::Some(Box::new(self.desugar_expr(*e)?))),
            HirExpr::Ok(e) => Ok(HirExpr::Ok(Box::new(self.desugar_expr(*e)?))),
            HirExpr::Err(e) => Ok(HirExpr::Err(Box::new(self.desugar_expr(*e)?))),
            _ => Ok(expr),
        }
    }

    fn desugar_match(&mut self, scrut: HirExpr, arms: Vec<HirMatchArm>) -> RR<HirExpr> {
        let span = Span {
            start_byte: 0,
            end_byte: 0,
            start_line: 0,
            end_line: 0,
            start_col: 0,
            end_col: 0,
        };

        let temp_id = self.temp_local();
        let temp_sym = SymbolId(0);

        let let_stmt = HirStmt::Let {
            local: temp_id,
            name: temp_sym,
            ty: None,
            init: Some(scrut),
            span,
        };

        let mut current_expr = HirExpr::Lit(HirLit::Null);

        for arm in arms.into_iter().rev() {
            let pat_check = self.pattern_match_check(temp_id, &arm.pat, span)?;

            let cond = if let Some(g) = arm.guard {
                HirExpr::Binary {
                    op: HirBinOp::And,
                    lhs: Box::new(pat_check),
                    rhs: Box::new(g),
                }
            } else {
                pat_check
            };

            let mut bind_stmts = Vec::new();
            self.pattern_bindings(temp_id, &arm.pat, &mut bind_stmts, span)?;

            let desugared_body = self.desugar_expr(arm.body)?;

            let branch_expr = if bind_stmts.is_empty() {
                desugared_body
            } else {
                bind_stmts.push(HirStmt::Expr {
                    expr: desugared_body,
                    span,
                });
                HirExpr::Block(HirBlock {
                    stmts: bind_stmts,
                    span,
                })
            };

            current_expr = HirExpr::IfExpr {
                cond: Box::new(cond),
                then_expr: Box::new(branch_expr),
                else_expr: Box::new(current_expr),
            };
        }

        Ok(HirExpr::Block(HirBlock {
            stmts: vec![
                let_stmt,
                HirStmt::Expr {
                    expr: current_expr,
                    span,
                },
            ],
            span,
        }))
    }

    fn pattern_match_check(&mut self, root: LocalId, pat: &HirPat, _span: Span) -> RR<HirExpr> {
        match pat {
            HirPat::Wild => Ok(HirExpr::Lit(HirLit::Bool(true))),
            HirPat::Bind { .. } => Ok(HirExpr::Lit(HirLit::Bool(true))),
            HirPat::Lit(l) => Ok(HirExpr::Binary {
                op: HirBinOp::Eq,
                lhs: Box::new(HirExpr::Local(root)),
                rhs: Box::new(HirExpr::Lit(l.clone())),
            }),
            _ => Ok(HirExpr::Lit(HirLit::Bool(false))),
        }
    }

    fn pattern_bindings(
        &mut self,
        root: LocalId,
        pat: &HirPat,
        stmts: &mut Vec<HirStmt>,
        span: Span,
    ) -> RR<()> {
        if let HirPat::Bind { name, local } = pat {
            stmts.push(HirStmt::Let {
                local: *local,
                name: *name,
                ty: None,
                init: Some(HirExpr::Local(root)),
                span,
            });
        }
        Ok(())
    }
}
