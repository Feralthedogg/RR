use super::*;
impl Lowerer {
    pub(crate) fn formula_term_text(expr: &ast::Expr) -> Option<String> {
        match &expr.kind {
            ast::ExprKind::Name(name) => Some(name.clone()),
            ast::ExprKind::Column(name) => Some(name.clone()),
            ast::ExprKind::Field { base, name } => {
                Self::formula_term_text(base).map(|prefix| format!("{prefix}.{name}"))
            }
            ast::ExprKind::Binary { op, lhs, rhs } => {
                let lhs = Self::formula_term_text(lhs)?;
                let rhs = Self::formula_term_text(rhs)?;
                let op_str = match op {
                    ast::BinOp::Add => "+",
                    ast::BinOp::Sub => "-",
                    ast::BinOp::Mul => "*",
                    ast::BinOp::Div => "/",
                    _ => return None,
                };
                Some(format!("{lhs} {op_str} {rhs}"))
            }
            ast::ExprKind::Lit(ast::Lit::Str(s)) => Some(s.clone()),
            _ => None,
        }
    }
    pub(crate) fn lower_formula_expr(
        &mut self,
        lhs: Option<ast::Expr>,
        rhs: ast::Expr,
        span: Span,
    ) -> RR<HirExpr> {
        let formula_text = if let Some(lhs) = lhs {
            let Some(lhs_text) = Self::formula_term_text(&lhs) else {
                return Err(Self::lower_formula_error(span));
            };
            let Some(rhs_text) = Self::formula_term_text(&rhs) else {
                return Err(Self::lower_formula_error(span));
            };
            format!("{lhs_text} ~ {rhs_text}")
        } else {
            let Some(rhs_text) = Self::formula_term_text(&rhs) else {
                return Err(Self::lower_formula_error(span));
            };
            format!("~{rhs_text}")
        };
        let callee = HirExpr::Global(self.intern_symbol("stats::as.formula"), span);
        Ok(HirExpr::Call(HirCall {
            callee: Box::new(callee),
            args: vec![HirArg::Pos(HirExpr::Lit(HirLit::Char(formula_text)))],
            span,
        }))
    }
    pub(crate) fn lower_formula_unary_expr(&mut self, rhs: ast::Expr, span: Span) -> RR<HirExpr> {
        self.lower_formula_expr(None, rhs, span)
    }
    pub(crate) fn lower_formula_binary_expr(
        &mut self,
        lhs: ast::Expr,
        rhs: ast::Expr,
        span: Span,
    ) -> RR<HirExpr> {
        self.lower_formula_expr(Some(lhs), rhs, span)
    }
    pub(crate) fn lower_formula_error(span: Span) -> RRException {
        RRException::new(
            "RR.TypeError",
            crate::error::RRCode::E1002,
            crate::error::Stage::Lower,
            "formula shorthand currently supports names, columns, dotted field paths, string literals, and simple infix formulas over those terms",
        )
        .at(span)
    }
}
